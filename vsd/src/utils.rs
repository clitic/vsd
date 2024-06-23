use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, KeyIvInit};
use anyhow::{anyhow, bail, Result};
use base64::Engine;
use regex::Regex;
use std::collections::HashSet;

type Aes128CbcDec = cbc::Decryptor<aes::Aes128>;

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

pub(super) fn scrape_playlist_links(text: &str) -> Vec<String> {
    // let re = Regex::new(r"(https|ftp|http)://([\w_-]+(?:(?:\.[\w_-]+)+))([\w.,@?^=%&:/~+#-]*[\w@?^=%&/~+#-]\.(m3u8|m3u|mpd))").unwrap();
    let re = Regex::new(r#"([\"\'])(https?:\/\/[^\"\']*\.(m3u8|m3u|mpd)[^\"\']*)([\"\'])"#).unwrap();
    let links = re
        .captures_iter(text)
        .map(|caps| caps.get(2).unwrap().as_str().to_string())
        .collect::<HashSet<String>>();

    // in case of amalgated urls
    links
        .into_iter()
        .map(|x| {
            if x.starts_with("http") {
                if let Some(y) = x.split("http").last() {
                    return format!("http{}", y);
                }
            }
            x
        })
        .collect()
}

pub(super) fn decode_base64<T: AsRef<[u8]>>(input: T) -> Result<Vec<u8>> {
    base64::engine::general_purpose::STANDARD
        .decode(input)
        .map_err(|x| x.into())
}

// pub(super) fn encode_base64<T: AsRef<[u8]>>(input: T) -> String {
//     base64::engine::general_purpose::STANDARD.encode(input)
// }

pub(super) fn decrypt_aes_128_cbc(
    input: &mut [u8],
    key: &[u8],
    iv: Option<&Vec<u8>>,
) -> Result<Vec<u8>> {
    let key_length = key.len();

    if key_length != 16 {
        bail!("invalid key size i.e. {} but expected size 16.", key_length);
    }

    let mut key_c = [0_u8; 16];
    key_c.copy_from_slice(key);

    let mut iv_c = [0_u8; 16];

    if let Some(iv) = iv {
        let iv_length = key.len();

        if iv_length != 16 {
            bail!("invalid iv size i.e. {} but expected size 16.", iv_length);
        }

        iv_c.copy_from_slice(iv);
    }

    Aes128CbcDec::new(&key_c.into(), &iv_c.into())
        .decrypt_padded_mut::<Pkcs7>(input)
        .map(|x| x.to_vec())
        .map_err(|x| anyhow!("{}", x))
}

// fn find_ffmpeg() -> Option<String> {
//     Some(
//         std::env::var("PATH")
//             .ok()?
//             .split(if cfg!(target_os = "windows") {
//                 ';'
//             } else {
//                 ':'
//             })
//             .find(|s| {
//                 std::path::Path::new(s)
//                     .join(if cfg!(target_os = "windows") {
//                         "ffmpeg.exe"
//                     } else {
//                         "ffmpeg"
//                     })
//                     .exists()
//             })?
//             .to_owned(),
//     )
// }

// fn output_parser(s: &str) -> Result<String, String> {
//     if find_ffmpeg().is_some() {
//         Ok(s.to_owned())
//     } else {
//         Err(
//             "could'nt locate ffmpeg binary in PATH (https://www.ffmpeg.org/download.html)"
//                 .to_owned(),
//         )
//     }
// }
