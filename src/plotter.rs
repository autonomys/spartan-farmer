use crate::plot::Plot;
use crate::{crypto, utils, Piece, ENCODE_ROUNDS, PIECE_SIZE, PRIME_SIZE_BYTES};
use async_std::path::PathBuf;
use async_std::task;
use futures::channel::oneshot;
use indicatif::ProgressBar;
use log::{info, warn};
use rayon::prelude::*;
use schnorrkel::PublicKey;
use spartan::Spartan;
use std::time::Instant;

// TODO: Plotter consumes a lot of RAM if disk write is slower than CPU encoding
pub async fn plot(path: PathBuf, genesis_piece: Piece, piece_count: u64, public_key: PublicKey) {
    let plot = Plot::open_or_create(&path).await.unwrap();
    let public_key_hash = crypto::hash_public_key(&public_key);
    let spartan: Spartan<PRIME_SIZE_BYTES, PIECE_SIZE> =
        Spartan::<PRIME_SIZE_BYTES, PIECE_SIZE>::new(genesis_piece);

    if plot.is_empty().await {
        let plotting_fut = utils::spawn_blocking({
            let plot = plot.clone();

            move || {
                let bar = ProgressBar::new(piece_count);

                (0..piece_count).into_par_iter().for_each(|index| {
                    let encoding = spartan.encode(public_key_hash, index, ENCODE_ROUNDS);

                    task::spawn({
                        let plot = plot.clone();

                        async move {
                            let result = plot.write(encoding, index).await;

                            if let Err(error) = result {
                                warn!("{}", error);
                            }
                        }
                    });
                    bar.inc(1);
                });

                bar.finish();
            }
        });

        let plot_time = Instant::now();

        info!("Slowly plotting {} pieces...", piece_count);

        info!(
            r#"
          `""==,,__
            `"==..__"=..__ _    _..-==""_
                 .-,`"=/ /\ \""/_)==""``
                ( (    | | | \/ |
                 \ '.  |  \;  \ /
                  |  \ |   |   ||
             ,-._.'  |_|   |   ||
            .\_/\     -'   ;   Y
           |  `  |        /    |-.
           '. __/_    _.-'     /'
                  `'-.._____.-'
        "#
        );

        plotting_fut.await;

        let (tx, rx) = oneshot::channel();

        let _handler = plot.on_close(move || {
            let _ = tx.send(());
        });

        drop(plot);

        rx.await.unwrap();

        let total_plot_time = plot_time.elapsed();
        let average_plot_time =
            (total_plot_time.as_nanos() / piece_count as u128) as f32 / (1000f32 * 1000f32);

        info!("Average plot time is {:.3} ms per piece", average_plot_time);

        info!(
            "Total plot time is {:.3} minutes",
            total_plot_time.as_secs_f32() / 60f32
        );

        info!(
            "Plotting throughput is {} mb/sec\n",
            ((piece_count as u64 * PIECE_SIZE as u64) / (1000 * 1000)) as f32
                / (total_plot_time.as_secs_f32())
        );
    } else {
        info!("Using existing plot...");

        let (tx, rx) = oneshot::channel();

        let _handler = plot.on_close(move || {
            let _ = tx.send(());
        });

        drop(plot);

        rx.await.unwrap();
    }
}
