use crate::{crypto, utils, Piece, Salt, Tag, PIECE_SIZE};
use async_std::fs::OpenOptions;
use async_std::path::PathBuf;
use async_std::task;
use event_listener_primitives::{BagOnce, HandlerId};
use futures::channel::mpsc as async_mpsc;
use futures::channel::oneshot;
use futures::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt, SinkExt, StreamExt};
use log::*;
use rocksdb::IteratorMode;
use rocksdb::DB;
use std::convert::TryInto;
use std::io;
use std::io::SeekFrom;
use std::ops::Deref;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PlotCreationError {
    #[error("Plot open error: {0}")]
    PlotOpen(io::Error),
    #[error("Plot tags open error: {0}")]
    PlotTagsOpen(rocksdb::Error),
}

#[derive(Debug)]
enum ReadRequests {
    IsEmpty {
        result_sender: oneshot::Sender<bool>,
    },
    ReadEncoding {
        index: u64,
        result_sender: oneshot::Sender<io::Result<Piece>>,
    },
    FindByRange {
        target: Tag,
        range: u64,
        result_sender: oneshot::Sender<io::Result<Option<(Tag, u64)>>>,
    },
}

#[derive(Debug)]
enum WriteRequests {
    WriteEncodings {
        encodings: Vec<Piece>,
        first_index: u64,
        salt: Salt,
        result_sender: oneshot::Sender<io::Result<()>>,
    },
}

#[derive(Default)]
struct Handlers {
    close: BagOnce<Box<dyn FnOnce() + Send>>,
}

pub struct Inner {
    handlers: Arc<Handlers>,
    any_requests_sender: async_mpsc::Sender<()>,
    read_requests_sender: async_mpsc::Sender<ReadRequests>,
    write_requests_sender: async_mpsc::Sender<WriteRequests>,
}

#[derive(Clone)]
pub struct Plot {
    inner: Arc<Inner>,
}

impl Plot {
    /// Creates a new plot for persisting encoded pieces to disk
    pub async fn open_or_create(path: &PathBuf) -> Result<Plot, PlotCreationError> {
        let mut plot_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path.join("plot.bin"))
            .await
            .map_err(PlotCreationError::PlotOpen)?;

        let tags_db = Arc::new(
            DB::open_default(path.join("plot-tags")).map_err(PlotCreationError::PlotTagsOpen)?,
        );

        // Channel with at most single element to throttle loop below if there are no updates
        let (any_requests_sender, mut any_requests_receiver) = async_mpsc::channel::<()>(1);
        let (read_requests_sender, mut read_requests_receiver) =
            async_mpsc::channel::<ReadRequests>(100);
        let (write_requests_sender, mut write_requests_receiver) =
            async_mpsc::channel::<WriteRequests>(100);

        let handlers = Arc::new(Handlers::default());

        task::spawn({
            let handlers = Arc::clone(&handlers);

            async move {
                let mut did_nothing = true;
                'outer: loop {
                    if did_nothing {
                        // Wait for stuff to come in
                        if any_requests_receiver.next().await.is_none() {
                            break;
                        }
                    }

                    did_nothing = true;

                    // Process as many read requests as there is
                    while let Ok(read_request) = read_requests_receiver.try_next() {
                        did_nothing = false;

                        match read_request {
                            Some(ReadRequests::IsEmpty { result_sender }) => {
                                let _ = result_sender.send(
                                    utils::spawn_blocking({
                                        let tags_db = Arc::clone(&tags_db);
                                        move || {
                                            tags_db.iterator(IteratorMode::Start).next().is_none()
                                        }
                                    })
                                    .await,
                                );
                            }
                            Some(ReadRequests::ReadEncoding {
                                index,
                                result_sender,
                            }) => {
                                let _ = result_sender.send(
                                    try {
                                        plot_file
                                            .seek(SeekFrom::Start(index * PIECE_SIZE as u64))
                                            .await?;
                                        let mut buffer = [0u8; PIECE_SIZE];
                                        plot_file.read_exact(&mut buffer).await?;
                                        buffer
                                    },
                                );
                            }
                            None => {
                                break 'outer;
                            }
                            Some(ReadRequests::FindByRange {
                                target,
                                range,
                                result_sender,
                            }) => {
                                // TODO: Remove unwrap
                                let solutions = utils::spawn_blocking({
                                    let tags_db = Arc::clone(&tags_db);
                                    move || {
                                        let mut iter = tags_db.raw_iterator();

                                        let mut solutions: Vec<(Tag, u64)> = Vec::new();

                                        let (lower, is_lower_overflowed) =
                                            u64::from_be_bytes(target).overflowing_sub(range / 2);
                                        let (upper, is_upper_overflowed) =
                                            u64::from_be_bytes(target).overflowing_add(range / 2);

                                        trace!(
                                            "{} Lower overflow: {} -- Upper overflow: {}",
                                            u64::from_be_bytes(target),
                                            is_lower_overflowed,
                                            is_upper_overflowed
                                        );

                                        if is_lower_overflowed || is_upper_overflowed {
                                            iter.seek_to_first();
                                            while let Some(tag) = iter.key() {
                                                let tag = tag.try_into().unwrap();
                                                let index = iter.value().unwrap();
                                                if u64::from_be_bytes(tag) <= upper {
                                                    solutions.push((
                                                        tag,
                                                        u64::from_le_bytes(
                                                            index.try_into().unwrap(),
                                                        ),
                                                    ));
                                                    iter.next();
                                                } else {
                                                    break;
                                                }
                                            }
                                            iter.seek(lower.to_be_bytes());
                                            while let Some(tag) = iter.key() {
                                                let tag = tag.try_into().unwrap();
                                                let index = iter.value().unwrap();

                                                solutions.push((
                                                    tag,
                                                    u64::from_le_bytes(index.try_into().unwrap()),
                                                ));
                                                iter.next();
                                            }
                                        } else {
                                            iter.seek(lower.to_be_bytes());
                                            while let Some(tag) = iter.key() {
                                                let tag = tag.try_into().unwrap();
                                                let index = iter.value().unwrap();
                                                if u64::from_be_bytes(tag) <= upper {
                                                    solutions.push((
                                                        tag,
                                                        u64::from_le_bytes(
                                                            index.try_into().unwrap(),
                                                        ),
                                                    ));
                                                    iter.next();
                                                } else {
                                                    break;
                                                }
                                            }
                                        }

                                        solutions
                                    }
                                })
                                .await;

                                let _ = result_sender.send(Ok(solutions.into_iter().next()));
                            }
                        }
                    }

                    let write_request = write_requests_receiver.try_next();
                    if write_request.is_ok() {
                        did_nothing = false;
                    }
                    // Process at most write request since reading is higher priority
                    match write_request {
                        Ok(Some(WriteRequests::WriteEncodings {
                            encodings,
                            first_index,
                            salt,
                            result_sender,
                        })) => {
                            let _ = result_sender.send(
                                try {
                                    plot_file
                                        .seek(SeekFrom::Start(first_index * PIECE_SIZE as u64))
                                        .await?;
                                    {
                                        let mut whole_encoding = Vec::with_capacity(
                                            encodings[0].len() * encodings.len(),
                                        );
                                        for encoding in &encodings {
                                            whole_encoding.extend_from_slice(encoding);
                                        }
                                        plot_file.write_all(&whole_encoding).await?;
                                    }

                                    // TODO: remove unwrap
                                    utils::spawn_blocking({
                                        let tags_db = Arc::clone(&tags_db);
                                        move || {
                                            for (encoding, index) in
                                                encodings.iter().zip(first_index..)
                                            {
                                                let tag = crypto::create_hmac(encoding, &salt);
                                                tags_db.put(&tag[0..8], index.to_le_bytes())?;
                                            }

                                            Ok::<(), rocksdb::Error>(())
                                        }
                                    })
                                    .await
                                    .unwrap();
                                },
                            );
                        }
                        Ok(None) => {
                            break 'outer;
                        }
                        Err(_) => {
                            // Ignore
                        }
                    }
                }

                std::thread::spawn({
                    let handlers = Arc::clone(&handlers);

                    move || {
                        drop(tags_db);

                        handlers.close.call_simple();
                    }
                });
            }
        });

        let inner = Inner {
            handlers,
            any_requests_sender,
            read_requests_sender,
            write_requests_sender,
        };

        Ok(Plot {
            inner: Arc::new(inner),
        })
    }

    pub async fn is_empty(&self) -> bool {
        let (result_sender, result_receiver) = oneshot::channel();

        self.read_requests_sender
            .clone()
            .send(ReadRequests::IsEmpty { result_sender })
            .await
            .expect("Failed sending read request");

        // If fails - it is either full or disconnected, we don't care either way, so ignore result
        let _ = self.any_requests_sender.clone().try_send(());

        result_receiver
            .await
            .expect("Read result sender was dropped")
    }

    /// Reads a piece from plot by index
    pub async fn read(&self, index: u64) -> io::Result<Piece> {
        let (result_sender, result_receiver) = oneshot::channel();

        self.read_requests_sender
            .clone()
            .send(ReadRequests::ReadEncoding {
                index,
                result_sender,
            })
            .await
            .expect("Failed sending read encoding request");

        // If fails - it is either full or disconnected, we don't care either way, so ignore result
        let _ = self.any_requests_sender.clone().try_send(());

        result_receiver
            .await
            .expect("Read encoding result sender was dropped")
    }

    pub async fn find_by_range(
        &self,
        target: [u8; 8],
        range: u64,
    ) -> io::Result<Option<(Tag, u64)>> {
        let (result_sender, result_receiver) = oneshot::channel();

        self.read_requests_sender
            .clone()
            .send(ReadRequests::FindByRange {
                target,
                range,
                result_sender,
            })
            .await
            .expect("Failed sending get by range request");

        // If fails - it is either full or disconnected, we don't care either way, so ignore result
        let _ = self.any_requests_sender.clone().try_send(());

        result_receiver
            .await
            .expect("Get by range result sender was dropped")
    }

    /// Writes a piece to the plot by index, will overwrite if piece exists (updates)
    pub async fn write_many(
        &self,
        encodings: Vec<Piece>,
        first_index: u64,
        salt: Salt,
    ) -> io::Result<()> {
        if encodings.is_empty() {
            return Ok(());
        }
        let (result_sender, result_receiver) = oneshot::channel();

        self.write_requests_sender
            .clone()
            .send(WriteRequests::WriteEncodings {
                encodings,
                first_index,
                salt,
                result_sender,
            })
            .await
            .expect("Failed sending write encoding request");

        // If fails - it is either full or disconnected, we don't care either way, so ignore result
        let _ = self.any_requests_sender.clone().try_send(());

        result_receiver
            .await
            .expect("Write encoding result sender was dropped")
    }

    pub fn on_close<F: FnOnce() + Send + 'static>(&self, callback: F) -> HandlerId {
        self.inner.handlers.close.add(Box::new(callback))
    }
}

impl Deref for Plot {
    type Target = Inner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_std::path::PathBuf;
    use rand::prelude::*;
    use std::fs;
    use std::time::Duration;

    struct TargetDirectory {
        path: PathBuf,
    }

    impl Drop for TargetDirectory {
        fn drop(&mut self) {
            drop(fs::remove_dir_all(&self.path));
        }
    }

    impl Deref for TargetDirectory {
        type Target = PathBuf;

        fn deref(&self) -> &Self::Target {
            &self.path
        }
    }

    impl TargetDirectory {
        fn new(test_name: &str) -> Self {
            let path = PathBuf::from("target").join(test_name);

            fs::create_dir_all(&path).unwrap();

            Self { path }
        }
    }

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    fn generate_random_piece() -> Piece {
        let mut bytes = [0u8; crate::PIECE_SIZE];
        rand::thread_rng().fill(&mut bytes[..]);
        bytes
    }

    #[async_std::test]
    async fn test_read_write() {
        init();
        let path = TargetDirectory::new("read_write");

        let piece = generate_random_piece();
        let salt: Salt = [1u8; 32];
        let index = 0;

        let plot = Plot::open_or_create(&path).await.unwrap();
        assert_eq!(true, plot.is_empty().await);
        plot.write_many(vec![piece], index, salt).await.unwrap();
        assert_eq!(false, plot.is_empty().await);
        let extracted_piece = plot.read(index).await.unwrap();

        assert_eq!(piece[..], extracted_piece[..]);

        drop(plot);

        async_std::task::sleep(Duration::from_millis(100)).await;

        // Make sure it is still not empty on reopen
        let plot = Plot::open_or_create(&path).await.unwrap();
        assert_eq!(false, plot.is_empty().await);
        drop(plot);

        // Let plot to destroy gracefully, otherwise may get "pure virtual method called
        // terminate called without an active exception" message
        async_std::task::sleep(Duration::from_millis(100)).await;
    }

    #[async_std::test]
    async fn test_find_by_tag() {
        init();
        let path = TargetDirectory::new("find_by_tag");
        let salt: Salt = [1u8; 32];

        let plot = Plot::open_or_create(&path).await.unwrap();

        plot.write_many(
            (0..1024_usize).map(|_| generate_random_piece()).collect(),
            0,
            salt,
        )
        .await
        .unwrap();

        {
            let target = [0u8, 0, 0, 0, 0, 0, 0, 1];
            let solution_range =
                u64::from_be_bytes([0u8, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
            // This is probabilistic, but should be fine most of the time
            let (solution, _) = plot
                .find_by_range(target, solution_range)
                .await
                .unwrap()
                .unwrap();
            // Wraps around
            let lower = u64::from_be_bytes(target).wrapping_sub(solution_range / 2);
            let upper = u64::from_be_bytes(target) + solution_range / 2;
            let solution = u64::from_be_bytes(solution);
            assert!(
                solution >= lower || solution <= upper,
                "Solution {:?} must be over wrapped lower edge {:?} or under upper edge {:?}",
                solution.to_be_bytes(),
                lower.to_be_bytes(),
                upper.to_be_bytes(),
            );
        }

        {
            let target = [0xff_u8, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xfe];
            let solution_range =
                u64::from_be_bytes([0u8, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
            // This is probabilistic, but should be fine most of the time
            let (solution, _) = plot
                .find_by_range(target, solution_range)
                .await
                .unwrap()
                .unwrap();
            // Wraps around
            let lower = u64::from_be_bytes(target) - solution_range / 2;
            let upper = u64::from_be_bytes(target).wrapping_add(solution_range / 2);
            let solution = u64::from_be_bytes(solution);
            assert!(
                solution >= lower || solution <= upper,
                "Solution {:?} must be over lower edge {:?} or under wrapped upper edge {:?}",
                solution.to_be_bytes(),
                lower.to_be_bytes(),
                upper.to_be_bytes(),
            );
        }

        {
            let target = [0xef_u8, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff];
            let solution_range =
                u64::from_be_bytes([0u8, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
            // This is probabilistic, but should be fine most of the time
            let (solution, _) = plot
                .find_by_range(target, solution_range)
                .await
                .unwrap()
                .unwrap();
            let lower = u64::from_be_bytes(target) - solution_range / 2;
            let upper = u64::from_be_bytes(target) + solution_range / 2;
            let solution = u64::from_be_bytes(solution);
            assert!(
                solution >= lower && solution <= upper,
                "Solution {:?} must be over lower edge {:?} and under upper edge {:?}",
                solution.to_be_bytes(),
                lower.to_be_bytes(),
                upper.to_be_bytes(),
            );
        }

        drop(plot);

        // Let plot to destroy gracefully, otherwise may get "pure virtual method called
        // terminate called without an active exception" message
        async_std::task::sleep(Duration::from_millis(100)).await;
    }
}
