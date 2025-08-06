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
    {
        if exe.exists() {
            return Some(exe);
        }
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

pub fn format_bytes(bytesval: usize, precision: usize) -> (String, String, String) {
    let mut val = bytesval as f32;

    for unit in ["bytes", "KiB", "MiB", "GiB", "TiB"] {
        if val < 1024.0 {
            return (
                format!("{val:.precision$}"),
                unit.to_owned(),
                format!("{val:.precision$} {unit}"),
            );
        }

        val /= 1024.0;
    }

    (
        format!("{bytesval:.precision$}"),
        "".to_owned(),
        format!("{bytesval:.precision$}"),
    )
}

pub fn format_download_bytes(downloaded: usize, total: usize) -> String {
    let downloaded = format_bytes(downloaded, 2);
    let mut total = format_bytes(total, 2);

    if total.1 == "MiB" {
        total.0 = total.0.split('.').next().unwrap().to_owned();
    }

    if downloaded.1 == total.1 {
        format!("{} / {} {}", downloaded.0, total.0, downloaded.1)
    } else {
        format!("{} / {}", downloaded.2, total.2)
    }
}
