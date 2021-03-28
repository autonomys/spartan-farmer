#![feature(try_blocks)]

mod commands;
mod crypto;
mod plot;
mod plotter;
mod utils;

use async_std::task;
use clap::{Clap, ValueHint};
use std::path::PathBuf;

const PRIME_SIZE_BYTES: usize = 8;
const PIECE_SIZE: usize = 4096;
const ENCODE_ROUNDS: usize = 1;

type Piece = [u8; PIECE_SIZE];
type Tag = [u8; PRIME_SIZE_BYTES];

#[derive(Debug, Clap)]
#[clap(about, version)]
enum Command {
    /// Create initial plot
    Plot {
        /// Use custom path for data storage instead of platform-specific default
        #[clap(long, value_hint = ValueHint::FilePath)]
        custom_path: Option<PathBuf>,
    },
    /// Start a farmer using previously created plot
    Farm {
        /// Use custom path for data storage instead of platform-specific default
        #[clap(long, value_hint = ValueHint::FilePath)]
        custom_path: Option<PathBuf>,
    },
}

fn main() {
    env_logger::init();

    let command: Command = Command::parse();

    match command {
        Command::Plot { custom_path } => {
            let path = utils::get_path(custom_path);
            task::block_on(commands::plot::plot(path, [0u8; PIECE_SIZE], 256)).unwrap();
        }
        Command::Farm { .. } => {
            // TODO: Implement correctly
            task::block_on(commands::farm::farm()).unwrap();
        }
    }
}
