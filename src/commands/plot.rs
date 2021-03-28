use crate::{plotter, Piece};
use log::debug;
use schnorrkel::Keypair;
use std::fs;
use std::path::PathBuf;

pub async fn plot(
    path: PathBuf,
    genesis_piece: Piece,
    piece_count: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let identity_file = path.join("identity.bin");
    let keypair = if identity_file.exists() {
        debug!("Opening existing keypair");
        Keypair::from_bytes(&fs::read(identity_file)?).map_err(|error| error.to_string())?
    } else {
        debug!("Generating new keypair");
        let keypair = Keypair::generate();
        fs::write(identity_file, keypair.to_bytes())?;
        keypair
    };

    plotter::plot(path.into(), genesis_piece, piece_count, keypair.public).await;
    // TODO
    Ok(())
}
