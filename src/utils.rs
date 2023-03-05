use anyhow::{bail, Result};
use kdam::term::Colorizer;
use reqwest::StatusCode;
use std::io::Write;

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

pub(super) fn find_hls_dash_links(text: &str) -> Vec<String> {
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

pub(super) fn select(prompt: String, choices: &[String], raw: bool) -> Result<usize> {
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

                if choice.text.contains("AUDIO")
                    || choice.text.contains("SUBTITLES")
                    || choice.text.contains("http")
                {
                    write!(backend, "{}", text.colorize("cyan"))
                } else {
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
                }
            })
            .build(),
    )?
    .as_list_item()
    .unwrap()
    .index)
}

#[cfg(feature = "chrome")]
pub(super) fn get_columns() -> u16 {
    kdam::term::get_columns_or(10)
}

// TODO: update message if #[cfg(feature = "chrome")]
pub(super) fn scrape_website_message(url: &str) -> String {
    format!(
        "No links found on website source.\n\n\
        {} Consider using {} subcommand and then \
        run the {} subcommand with same arguments by replacing the {} with captured url.\n\n\
        Suppose first command captures https://streaming.site/video_001/master.m3u8\n\
        $ vsd capture {}\n\
        $ vsd save https://streaming.site/video_001/master.m3u8 \n\n\
        {} Consider using {} subcommand \
        and then run {} subcommand with saved playlist file as {}. \n\n\
        Suppose first command saves master.m3u8\n\
        $ vsd collect --build {}\n\
        $ vsd save master.m3u8",
        "TRY THIS:".colorize("yellow"),
        "capture".colorize("bold green"),
        "save".colorize("bold green"),
        "INPUT".colorize("bold green"),
        url,
        "OR THIS:".colorize("yellow"),
        "collect".colorize("bold green"),
        "save".colorize("bold green"),
        "INPUT".colorize("bold green"),
        url,
    )
}

pub(super) fn check_reqwest_error(error: &reqwest::Error, url: &str) -> Result<String> {
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

// pub(super) fn duration(duration: &str) -> Result<f32> {
//     let duration = duration.replace('s', "").replace(',', ".");
//     let is_frame = duration.split(':').count() >= 4;
//     let mut duration = duration.split(':').rev();
//     let mut total_seconds = 0.0;

//     if is_frame {
//         if let Some(seconds) = duration.next() {
//             total_seconds += seconds.parse::<f32>()? / 1000.0;
//         }
//     }

//     if let Some(seconds) = duration.next() {
//         total_seconds += seconds.parse::<f32>()?;
//     }

//     if let Some(minutes) = duration.next() {
//         total_seconds += minutes.parse::<f32>()? * 60.0;
//     }

//     if let Some(hours) = duration.next() {
//         total_seconds += hours.parse::<f32>()? * 3600.0;
//     }

//     Ok(total_seconds)
// }

pub(super) fn pathbuf_from_url(url: &str) -> std::path::PathBuf {
    let output = url.split('?').next().unwrap().split('/').last().unwrap();

    if output.ends_with(".m3u") || output.ends_with(".m3u8") {
        if output.ends_with(".ts.m3u8") {
            std::path::PathBuf::from(output.trim_end_matches(".m3u8").to_owned())
        } else {
            let mut path = std::path::PathBuf::from(&output);
            path.set_extension("ts");
            path
        }
    } else if output.ends_with(".mpd") || output.ends_with(".xml") {
        let mut path = std::path::PathBuf::from(&output);
        path.set_extension("m4s");
        path
    } else {
        let mut path = std::path::PathBuf::from(
            output
                .replace('<', "-")
                .replace('>', "-")
                .replace(':', "-")
                .replace('\"', "-")
                .replace('/', "-")
                .replace('\\', "-")
                .replace('|', "-")
                .replace('?', "-"),
        );
        path.set_extension("mp4");
        path
    }
}

// if !self.input.starts_with("http") {
//     bail!(
//         "Non HTTP input should have {} set explicitly.",
//         "--baseurl".colorize("bold green")
//     )
// }
pub(super) fn build_absolute_url(baseurl: &str, uri: &str) -> Result<reqwest::Url> {
    if uri.starts_with("http") {
        Ok(reqwest::Url::parse(uri)?)
    } else {
        Ok(reqwest::Url::parse(baseurl)?.join(uri)?)
    }
}

// use reqwest::header::HeaderValue;
// use reqwest::header;

// struct PartialRangeIter {
//     start: u64,
//     end: u64,
//     buffer_size: u32,
//   }

//   impl PartialRangeIter {
//     pub(super) fn new(start: u64, end: u64, buffer_size: u32) -> Self {
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
