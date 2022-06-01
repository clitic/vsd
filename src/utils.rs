use std::collections::HashMap;

use anyhow::{bail, Result};

pub fn quality_selector(
    quality: &str,
    res_band: HashMap<&str, (usize, usize)>,
    master: &m3u8_rs::MasterPlaylist,
) -> Result<String> {
    if let Some(index) = res_band.get(quality) {
        Ok(master.variants[index.1].uri.clone())
    } else {
        bail!(
            "Master playlist doesn't contain {} quality variant stream.",
            quality
        );
    }
}

pub fn find_hls_dash_links(text: &str) -> Vec<String> {
    let re = regex::Regex::new(r"(https|ftp|http)://([\w_-]+(?:(?:\.[\w_-]+)+))([\w.,@?^=%&:/~+#-]*[\w@?^=%&/~+#-]\.(m3u8|m3u|mpd))").unwrap();
    let links = re
        .captures_iter(&text)
        .map(|caps| caps.get(0).unwrap().as_str().to_string())
        .collect::<Vec<String>>();

    let mut unique_links = vec![];
    for link in links {
        if !unique_links.contains(&link) {
            unique_links.push(link);
        }
    }
    unique_links
}

pub fn find_ffmpeg_with_path() -> Option<String> {
    Some(
        std::env::var("PATH")
            .ok()?
            .split(if cfg!(target_os = "windows") {
                ';'
            } else {
                ':'
            })
            .find(|s| {
                std::path::Path::new(s)
                    .join(if cfg!(target_os = "windows") {
                        "ffmpeg.exe"
                    } else {
                        "ffmpeg"
                    })
                    .exists()
            })?
            .to_owned(),
    )
}

pub fn select_str(prompt: String, choices: Vec<String>) -> Result<String> {
    Ok(requestty::prompt_one(
        requestty::Question::select("theme")
            .message(prompt)
            .choices(choices)
            .build(),
    )?
    .as_list_item()
    .unwrap()
    .text
    .to_string())
}

pub fn select(prompt: String, choices: Vec<String>) -> Result<usize> {
    // println!("{}", prompt);
    // for choice in choices.clone() {
    //     println!("{}", choice);
    // }
    // let mut input = String::new();
    // println!("\n{} (1, 2, etc.): \r", prompt);
    // std::io::stdin().read_line(&mut input)?;
    // println!("{}", input.trim());

    Ok(requestty::prompt_one(
        requestty::Question::select("theme")
            .message(prompt)
            .choices(choices)
            .build(),
    )?
    .as_list_item()
    .unwrap()
    .index)
}

pub fn format_bytes(bytesval: usize) -> (String, String, String) {
    let mut val = bytesval as f32;

    for unit in ["bytes", "KB", "MB", "GB", "TB"] {
        if val < 1024.0 {
            return (format!("{:.2}", val), unit.to_owned(), format!("{:.2} {}", val, unit));
        }

        val /= 1024.0;
    }

    return (format!("{:.2}", bytesval), "".to_owned(), format!("{:.2}", bytesval));
}