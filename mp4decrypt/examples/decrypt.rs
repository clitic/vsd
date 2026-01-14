use mp4decrypt::{Error, Mp4Decrypter};
use std::path::Path;

fn main() -> Result<(), Error> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));

    Mp4Decrypter::new()
        .key(
            "eb676abbcb345e96bbcf616630f1a3da",
            "100b6c20940f779a4589152b57d2dacb",
        )?
        .init_file(root.join("examples/sample/init.mp4"))?
        .input_file(root.join("examples/sample/segment_0.m4s"))?
        .decrypt_to_file(root.join("../target/mp4decrypt-example.mp4"))?;

    Ok(())
}
