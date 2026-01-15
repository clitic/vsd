use mp4decrypt::Ap4CencDecryptingProcessor;
use std::{error::Error, fs, path::Path};

fn main() -> Result<(), Box<dyn Error>> {
    let root_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let samples_dir = root_dir.join("tests/samples/cenc-multi");

    let decrypted = Ap4CencDecryptingProcessor::new()
        .key(
            "eb676abbcb345e96bbcf616630f1a3da",
            "100b6c20940f779a4589152b57d2dacb",
        )?
        .key(
            "63cb5f7184dd4b689a5c5ff11ee6a328",
            "3bda3329158a4789880816a70e7e436d",
        )?
        .build()?
        .decrypt(
            fs::read(samples_dir.join("video_1.m4s"))?,
            Some(fs::read(samples_dir.join("video_init.mp4"))?),
        )?;

    fs::write(root_dir.join("examples/output.mp4"), decrypted)?;
    Ok(())
}
