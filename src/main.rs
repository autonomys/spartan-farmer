mod commands;

use async_std::task;
use clap::{Clap, ValueHint};
use commands::{farm, plot};
use std::fs;
use std::path::PathBuf;

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

fn get_path(custom_path: Option<PathBuf>) -> PathBuf {
    // set storage path
    let path = custom_path
        .or_else(|| std::env::var("SUBSPACE_DIR").map(PathBuf::from).ok())
        .unwrap_or_else(|| {
            dirs::data_local_dir()
                .expect("Can't find local data directory, needs to be specified explicitly")
                .join("subspace")
        });

    if !path.exists() {
        fs::create_dir_all(&path).unwrap_or_else(|error| {
            panic!("Failed to create data directory {:?}: {:?}", path, error)
        });
    }

    path
}

fn main() {
    env_logger::init();

    let command: Command = Command::parse();

    match command {
        Command::Plot { custom_path } => {
            let path = get_path(custom_path);
            plot::plot(path).unwrap();
        }
        Command::Farm { .. } => {
            // TODO: Implement correctly
            task::block_on(farm::farm()).unwrap();
        }
    }
}
