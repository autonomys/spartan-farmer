#![feature(try_blocks)]

mod commands;
mod crypto;
mod plot;
mod utils;

use async_std::task;
use clap::{Clap, ValueHint};
use log::info;
use std::fs;
use std::path::PathBuf;

type Piece = [u8; PIECE_SIZE];
type Tag = [u8; PRIME_SIZE_BYTES];
type Salt = [u8; 32];

const PRIME_SIZE_BYTES: usize = 8;
const PIECE_SIZE: usize = 4096;
const ENCODE_ROUNDS: usize = 1;
// TODO: Replace fixed salt with something
const SALT: Salt = [1u8; 32];
const SIGNING_CONTEXT: &[u8] = b"FARMER";

#[derive(Debug, Clap)]
#[clap(about, version)]
enum Command {
    /// Create initial plot
    Plot {
        /// Use custom path for data storage instead of platform-specific default
        #[clap(long, value_hint = ValueHint::FilePath)]
        custom_path: Option<PathBuf>,
        /// Number of 4096 bytes pieces to plot
        plot_pieces: u64,
        /// Seed used for generating genesis piece
        seed: String,
    },
    /// Erase existing
    ErasePlot {
        /// Use custom path for data storage instead of platform-specific default
        #[clap(long, value_hint = ValueHint::FilePath)]
        custom_path: Option<PathBuf>,
    },
    /// Start a farmer using previously created plot
    Farm {
        /// Use custom path for data storage instead of platform-specific default
        #[clap(long, value_hint = ValueHint::FilePath)]
        custom_path: Option<PathBuf>,
        #[clap(long, default_value = "ws://127.0.0.1:9944")]
        ws_server: String,
    },
}

fn main() {
    env_logger::init();

    let command: Command = Command::parse();

    match command {
        Command::Plot {
            custom_path,
            plot_pieces,
            seed,
        } => {
            let path = utils::get_path(custom_path);
            task::block_on(commands::plot::plot(
                path,
                crypto::genesis_piece_from_seed(&seed),
                plot_pieces,
                SALT,
            ))
            .unwrap();
        }
        Command::ErasePlot { custom_path } => {
            let path = utils::get_path(custom_path);
            info!("Erasing the plot");
            fs::remove_file(path.join("plot.bin")).unwrap();
            info!("Erasing plot metadata");
            fs::remove_dir_all(path.join("plot-tags")).unwrap();
            info!("Done");
        }
        Command::Farm {
            custom_path,
            ws_server,
        } => {
            let path = utils::get_path(custom_path);
            task::block_on(commands::farm::farm(path, &ws_server)).unwrap();
        }
    }
}
