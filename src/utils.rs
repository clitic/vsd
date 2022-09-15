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

pub fn select(prompt: String, choices: &[String], raw: bool) -> Result<usize> {
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
            .transform(|choice, _, backend| {
                let text = choice
                    .text
                    .trim()
                    .trim_start_matches(&format!("{})", choice.index + 1))
                    .trim();
                let resolution = text.split('.').next().unwrap().split(' ').next().unwrap();
                let bandwidth = text
                    .trim_start_matches(resolution)
                    .trim()
                    .split('s')
                    .next()
                    .unwrap();

                write!(
                    backend,
                    "{}",
                    format!("{} ({}s)", resolution, bandwidth).colorize("cyan")
                )
            })
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

pub fn get_columns() -> u16 {
    kdam::term::get_columns_or(10)
}

pub fn scrape_website_message(url: &str) -> String {
    format!(
        "No links found on website source.\n\n\
        {} Consider using {} flag and then \
        run the command with same arguments by replacing the {} with captured m3u8 url.\n\n\
        Suppose first command captures https://streaming.site/video_001/master.m3u8\n\
        $ vsd --capture {}\n\
        $ vsd https://streaming.site/video_001/master.m3u8 \n\n\
        {} Consider using {} flag \
        and then run the command with saved .m3u8 file as {}. \n\n\
        Suppose first command saves master.m3u8\n\
        $ vsd --collect --build {}\n\
        $ vsd master.m3u8",
        "TRY THIS:".colorize("yellow"),
        "--capture".colorize("bold green"),
        "INPUT".colorize("bold green"),
        url,
        "OR THIS:".colorize("yellow"),
        "--collect --build".colorize("bold green"),
        "INPUT".colorize("bold green"),
        url,
    )
}

pub fn check_reqwest_error(error: &reqwest::Error, url: &str) -> Result<String> {
    let request = "Request".colorize("bold yellow");

    if error.is_timeout() {
        return Ok(format!("    {} {} (timeout)", request, url));
    } else if error.is_connect() {
        return Ok(format!("    {} {} (connection error)", request, url));
    }

    if let Some(status) = error.status() {
        match status {
            StatusCode::REQUEST_TIMEOUT => Ok(format!("    {} {} (timeout)", request, url)),
            StatusCode::TOO_MANY_REQUESTS => {
                Ok(format!("    {} {} (too many requests)", request, url))
            }
            StatusCode::SERVICE_UNAVAILABLE => {
                Ok(format!("    {} {} (service unavailable)", request, url))
            }
            StatusCode::GATEWAY_TIMEOUT => Ok(format!("    {} {} (gateway timeout)", request, url)),
            _ => bail!("download failed {} (HTTP {})", url, status),
        }
    } else {
        bail!("download failed {}", url)
    }
}

pub fn duration(duration: &str) -> Result<f32> {
    let duration = duration.replace('s', "").replace(',', ".");
    let mut duration = duration.split(':').rev();
    let mut total_seconds = 0.0;

    if let Some(seconds) = duration.next() {
        total_seconds += seconds.parse::<f32>()?;
    }

    if let Some(minutes) = duration.next() {
        total_seconds += minutes.parse::<f32>()? * 60.0;
    }

    if let Some(hours) = duration.next() {
        total_seconds += hours.parse::<f32>()? * 3600.0;
    }

    Ok(total_seconds)
}

// use reqwest::header::HeaderValue;
// use reqwest::header;

// struct PartialRangeIter {
//     start: u64,
//     end: u64,
//     buffer_size: u32,
//   }

//   impl PartialRangeIter {
//     pub fn new(start: u64, end: u64, buffer_size: u32) -> Self {
//       if buffer_size == 0 {
//         panic!("invalid buffer_size, give a value greater than zero.");
//       }

//       PartialRangeIter {
//         start,
//         end,
//         buffer_size,
//       }
//     }
//   }

//   impl Iterator for PartialRangeIter {
//     type Item = HeaderValue;
//     fn next(&mut self) -> Option<Self::Item> {
//       if self.start > self.end {
//         None
//       } else {
//         let prev_start = self.start;
//         self.start += std::cmp::min(self.buffer_size as u64, self.end - self.start + 1);
//         Some(HeaderValue::from_str(&format!("bytes={}-{}", prev_start, self.start - 1)).expect("string provided by format!"))
//       }
//     }
//   }

//   let response = client.head(url).send()?;
//   let length = response
//     .headers()
//     .get(CONTENT_LENGTH)
//     .ok_or("response doesn't include the content length")?;
//   let length = u64::from_str(length.to_str()?).map_err(|_| "invalid Content-Length header")?;

//   for range in PartialRangeIter::new(0, length - 1, CHUNK_SIZE)? {
//     let mut response = client.get(url).header(RANGE, range).send()?;
//     status == StatusCode::PARTIAL_CONTENT
