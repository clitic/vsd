use anyhow::Result;
use base64::Engine;
use std::{env, path::Path};

pub(super) fn format_bytes(bytesval: usize, precision: usize) -> (String, String, String) {
    let mut val = bytesval as f32;

    for unit in ["bytes", "KiB", "MiB", "GiB", "TiB"] {
        if val < 1024.0 {
            return (
                format!("{:.precision$}", val, precision = precision),
                unit.to_owned(),
                format!("{:.precision$} {}", val, unit, precision = precision),
            );
        }

        val /= 1024.0;
    }

    (
        format!("{:.precision$}", bytesval, precision = precision),
        "".to_owned(),
        format!("{:.precision$}", bytesval, precision = precision),
    )
}

pub(super) fn format_download_bytes(downloaded: usize, total: usize) -> String {
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

pub(super) fn decode_base64<T: AsRef<[u8]>>(input: T) -> Result<Vec<u8>> {
    base64::engine::general_purpose::STANDARD
        .decode(input)
        .map_err(|x| x.into())
}

// pub(super) fn encode_base64<T: AsRef<[u8]>>(input: T) -> String {
//     base64::engine::general_purpose::STANDARD.encode(input)
// }

pub(super) fn find_ffmpeg() -> Option<String> {
    let bin = if cfg!(target_os = "windows") {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    };

    if Path::new(bin).exists() {
        return Some(bin.to_owned());
    }

    env::var("PATH")
        .ok()?
        .split(if cfg!(target_os = "windows") {
            ';'
        } else {
            ':'
        })
        .find_map(|s| {
            let x = Path::new(s).join(bin);

            if x.exists() {
                Some(x.to_str().unwrap().to_owned())
            } else {
                None
            }
        })
}
