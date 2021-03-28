use log::debug;
use schnorrkel::Keypair;
use std::fs;
use std::path::PathBuf;

pub fn plot(path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
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
    // TODO
    Ok(())
}
