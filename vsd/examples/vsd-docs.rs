use std::{env, error::Error, fs, path::Path};

fn main() -> Result<(), Box<dyn Error>> {
    let markdown = clap_markdown::help_markdown::<vsd::Args>();
    fs::write(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("CLI.md"),
        markdown,
    )?;
    Ok(())
}
