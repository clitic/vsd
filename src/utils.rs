// use kdam::term::Colorizer;
// use std::io::Write;

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

// pub(super) fn find_hls_dash_links(text: &str) -> Vec<String> {
//     let re = regex::Regex::new(r"(https|ftp|http)://([\w_-]+(?:(?:\.[\w_-]+)+))([\w.,@?^=%&:/~+#-]*[\w@?^=%&/~+#-]\.(m3u8|m3u|mpd))").unwrap();
//     let links = re
//         .captures_iter(text)
//         .map(|caps| caps.get(0).unwrap().as_str().to_string())
//         .collect::<Vec<String>>();

//     let mut unique_links = vec![];
//     for link in links {
//         if !unique_links.contains(&link) {
//             unique_links.push(link);
//         }
//     }
//     unique_links
// }


// TODO: update message if #[cfg(feature = "chrome")]
// pub(super) fn scrape_website_message(url: &str) -> String {
//     format!(
//         "No links found on website source.\n\n\
//         {} Consider using {} subcommand and then \
//         run the {} subcommand with same arguments by replacing the {} with captured url.\n\n\
//         Suppose first command captures https://streaming.site/video_001/master.m3u8\n\
//         $ vsd capture {}\n\
//         $ vsd save https://streaming.site/video_001/master.m3u8 \n\n\
//         {} Consider using {} subcommand \
//         and then run {} subcommand with saved playlist file as {}. \n\n\
//         Suppose first command saves master.m3u8\n\
//         $ vsd collect --build {}\n\
//         $ vsd save master.m3u8",
//         "TRY THIS:".colorize("yellow"),
//         "capture".colorize("bold green"),
//         "save".colorize("bold green"),
//         "INPUT".colorize("bold green"),
//         url,
//         "OR THIS:".colorize("yellow"),
//         "collect".colorize("bold green"),
//         "save".colorize("bold green"),
//         "INPUT".colorize("bold green"),
//         url,
//     )
// }



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
