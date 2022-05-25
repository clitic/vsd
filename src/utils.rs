use anyhow::Result;

pub fn join_url(url1: &str, url2: &str) -> Result<String> {
    Ok(reqwest::Url::parse(url1)?.join(url2)?.to_string())
}

pub fn find_hls_dash_links(text: String) -> Vec<String> {
    // let re = regex::Regex::new(r"https?://[a-zA-Z0-9-._%/]*\.m3u8")
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

pub fn select(prompt: String, choices: Vec<String>) -> Result<String> {
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

pub fn select_index(prompt: String, choices: Vec<String>) -> Result<usize> {
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

pub fn format_bytes(bytesval: usize) -> (String, String) {
    let mut val = bytesval as f32;

    for unit in ["bytes", "KB", "MB", "GB", "TB"] {
        if val < 1024.0 {
            return (format!("{:.2}", val), unit.to_owned());
        }

        val /= 1024.0;
    }

    return (format!("{:.2}", bytesval), "".to_owned());
}

pub fn format_bytes_joined(bytesval: usize) -> String {
    let mut val = bytesval as f32;

    for unit in ["bytes", "KB", "MB", "GB", "TB"] {
        if val < 1024.0 {
            return format!("{:.2} {}", val, unit);
        }

        val /= 1024.0;
    }

    return format!("{:.2} bytes", val);
}