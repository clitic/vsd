use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

pub fn join(pth1: &str, pth2: &str) -> Result<String> {
    Ok(Path::new(pth1)
        .join(pth2)
        .to_str()
        .context(format!("couldn't join {} path with {}", pth1, pth2))?
        .to_string())
}

pub fn replace_ext(pth: &str, ext: &str) -> String {
    let mut tpth = PathBuf::from(pth);
    tpth.set_extension(ext);
    tpth.to_str().unwrap().to_string()
}
