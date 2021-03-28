#![feature(try_blocks)]

mod commands;
mod crypto;
mod plot;
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
        /// Number of 4096 bytes pieces to plot
        plot_pieces: u64,
        /// Seed used for generating genesis piece
        seed: String,
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
            ))
            .unwrap();
        }
        Command::Farm { .. } => {
            // TODO: Implement correctly
            task::block_on(commands::farm::farm()).unwrap();
        }
    }
}
