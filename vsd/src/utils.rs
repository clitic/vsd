use anyhow::Result;
use base64::Engine;
use std::{env, path::PathBuf};

pub fn decode_base64<T: AsRef<[u8]>>(input: T) -> Result<Vec<u8>> {
    base64::engine::general_purpose::STANDARD
        .decode(input)
        .map_err(|x| x.into())
}

pub fn find_ffmpeg() -> Option<PathBuf> {
    let bin = if cfg!(target_os = "windows") {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    };

    // Search in current directory
    let exe = PathBuf::from(bin);

    if exe.exists() {
        return Some(exe);
    }

    // Search in executable directory
    if let Some(exe) = env::current_exe()
        .ok()
        .and_then(|x| x.parent().map(|y| y.join(bin)))
        && exe.exists()
    {
        return Some(exe);
    }

    // Search in PATH
    env::var("PATH")
        .ok()?
        .split(if cfg!(target_os = "windows") {
            ';'
        } else {
            ':'
        })
        .find_map(|s| {
            let exe = PathBuf::from(s).join(bin);
            if exe.exists() { Some(exe) } else { None }
        })
}
