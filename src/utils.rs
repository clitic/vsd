use anyhow::{bail, Result};
use kdam::term::Colorizer;
use reqwest::StatusCode;
use std::io::Write;

pub fn format_bytes(bytesval: usize, precision: usize) -> (String, String, String) {
    let mut val = bytesval as f32;

    for unit in ["bytes", "KB", "MB", "GB", "TB"] {
        if val < 1024.0 {
            return (
                format!("{:.precision$}", val, precision = precision),
                unit.to_owned(),
                format!("{:.precision$} {}", val, unit, precision = precision),
            );
        }

        val /= 1024.0;
    }

    return (
        format!("{:.precision$}", bytesval, precision = precision),
        "".to_owned(),
        format!("{:.precision$}", bytesval, precision = precision),
    );
}

pub fn find_hls_dash_links(text: &str) -> Vec<String> {
    let re = regex::Regex::new(r"(https|ftp|http)://([\w_-]+(?:(?:\.[\w_-]+)+))([\w.,@?^=%&:/~+#-]*[\w@?^=%&/~+#-]\.(m3u8|m3u|mpd))").unwrap();
    let links = re
        .captures_iter(text)
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

pub fn select(prompt: String, choices: &Vec<String>, raw: bool) -> Result<usize> {
    if raw {
        println!("{}", prompt);

        for choice in choices {
            println!("{}", choice);
        }

        print!("{} (1, 2, etc.): ", prompt.trim_end_matches(':'));
        std::io::stdout().flush()?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        return Ok(input.trim().parse::<usize>()? - 1);
    }

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

fn find_ffmpeg_with_path() -> Option<String> {
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

pub fn check_ffmpeg(text: &str) -> Result<()> {
    if find_ffmpeg_with_path().is_none() {
        bail!(
            "FFMPEG couldn't be located in PATH. \
            It is required because {}. \
            Visit https://www.ffmpeg.org/download.html to install it.",
            text
        );
    }

    Ok(())
}

pub fn replace_ext(pth: &str, ext: &str) -> String {
    let mut tpth = std::path::PathBuf::from(pth);
    tpth.set_extension(ext);
    tpth.to_str().unwrap().to_owned()
}

pub fn get_columns() -> u16 {
    kdam::term::get_columns_or(10)
}

pub fn check_reqwest_error(error: &reqwest::Error) -> Result<String> {
    let url = error
        .url()
        .map(|x| {
            format!(
                "{}://{} ({})",
                x.scheme(),
                x.domain().unwrap(),
                x.to_string()
                    .split("?")
                    .next()
                    .unwrap()
                    .split("/")
                    .last()
                    .unwrap()
                    .colorize("cyan")
            )
        })
        .unwrap_or("".to_owned());

    if error.is_timeout() {
        return Ok(format!(
            "{} {}",
            "REQUEST TIMEOUT".colorize("bold red"),
            url
        ));
    } else if error.is_connect() {
        return Ok(format!(
            "{} {}",
            "CONNECTION ERROR".colorize("bold red"),
            url
        ));
    }
    if let Some(status) = error.status() {
        match status {
            StatusCode::REQUEST_TIMEOUT => Ok(format!(
                "{} {}",
                "REQUEST TIMEOUT".colorize("bold red"),
                url
            )),
            StatusCode::TOO_MANY_REQUESTS => Ok(format!(
                "{ }{}",
                "TOO MANY REQUESTS".colorize("bold red"),
                url
            )),
            StatusCode::SERVICE_UNAVAILABLE => Ok(format!(
                "{} {}",
                "SERVICE UNAVAILABLE".colorize("bold red"),
                url
            )),
            StatusCode::GATEWAY_TIMEOUT => Ok(format!(
                "{} {}",
                "GATEWAY TIMEOUT".colorize("bold red"),
                url
            )),
            _ => bail!(
                "{} failed with HTTP {} ({})",
                "Download".colorize("bold red"),
                status,
                error.url().map(|x| x.as_str()).unwrap().colorize("cyan")
            ),
        }
    } else {
        bail!(
            "{} failed ({})",
            "Download".colorize("bold red"),
            error.url().map(|x| x.as_str()).unwrap().colorize("cyan")
        )
    }
}
