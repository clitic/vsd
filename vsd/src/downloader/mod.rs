mod encryption;
mod fetch;
mod mux;
mod parse;
mod subtitle;

use encryption::Decrypter;
pub use fetch::{fetch_playlist, InputMetadata};
use mux::Stream;
pub use parse::{parse_all_streams, parse_selected_streams};
pub use subtitle::download_subtitle_streams;

use crate::{
    merger::Merger,
    playlist::{KeyMethod, MediaPlaylist, MediaType},
    utils,
};
use anyhow::{bail, Result};
use kdam::{term::Colorizer, tqdm, BarExt, Column, RichProgress};
use reqwest::{
    blocking::{Client, RequestBuilder},
    header, StatusCode, Url,
};
use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Instant,
};

pub struct Prompts {
    pub skip: bool,
    pub raw: bool,
}

#[allow(clippy::too_many_arguments)]
pub fn download(
    all_keys: bool,
    base_url: Option<Url>,
    client: Client,
    directory: Option<PathBuf>,
    keys: Vec<(Option<String>, String)>,
    no_decrypt: bool,
    no_merge: bool,
    output: Option<PathBuf>,
    streams: Vec<MediaPlaylist>,
    retry_count: u8,
    threads: u8,
) -> Result<()> {
    let should_mux = mux::should_mux(no_decrypt, no_merge, &streams, output.as_ref());

    if should_mux && utils::find_ffmpeg().is_none() {
        bail!("ffmpeg couldn't be found, it is required to continue further.");
    }

    // -----------------------------------------------------------------------------------------
    // Parse Key Ids
    // -----------------------------------------------------------------------------------------

    if !no_decrypt {
        encryption::check_unsupported_encryptions(&streams)?;
        let default_kids = encryption::extract_default_kids(&base_url, &client, &streams)?;
        encryption::check_key_exists_for_kid(&keys, &default_kids)?;
    }

    // -----------------------------------------------------------------------------------------
    // Prepare Progress Bar
    // -----------------------------------------------------------------------------------------

    let mut pb = RichProgress::new(
        tqdm!(unit = " SEG".to_owned(), dynamic_ncols = true),
        vec![
            Column::Text("[bold blue]?".to_owned()),
            Column::Animation,
            Column::Percentage(0),
            Column::Text("•".to_owned()),
            Column::CountTotal,
            Column::Text("•".to_owned()),
            Column::ElapsedTime,
            Column::Text("[cyan]>".to_owned()),
            Column::RemainingTime,
            Column::Text("•".to_owned()),
            Column::Rate,
        ],
    );

    // -----------------------------------------------------------------------------------------
    // Prepare Directory & Store Streams Metadata
    // -----------------------------------------------------------------------------------------

    if let Some(directory) = &directory {
        if !directory.exists() {
            std::fs::create_dir_all(directory)?;
        }
    }

    let mut temp_files = vec![];

    // -----------------------------------------------------------------------------------------
    // Download Subtitle Streams
    // -----------------------------------------------------------------------------------------

    download_subtitle_streams(
        &base_url,
        &client,
        &directory,
        &streams,
        &mut pb,
        &mut temp_files,
    )?;

    let mut streams = streams
        .into_iter()
        .filter(|x| x.media_type != MediaType::Subtitles)
        .collect::<Vec<_>>();

    // -----------------------------------------------------------------------------------------
    // Estimation
    // -----------------------------------------------------------------------------------------

    let mut downloaded_bytes = 0;
    let mut relative_sizes = VecDeque::new();

    for stream in &mut streams {
        relative_sizes.push_back(stream.estimate_size(&base_url, &client)?);
        stream.split_segment(&base_url, &client)?;
    }

    // -----------------------------------------------------------------------------------------
    // Prepare Progress Bar
    // -----------------------------------------------------------------------------------------

    pb.replace(2, Column::Percentage(2));
    pb.columns.extend_from_slice(&[
        Column::Text("•".to_owned()),
        Column::Text("[yellow]?".to_owned()),
    ]);
    pb.pb
        .reset(Some(streams.iter().map(|x| x.segments.len()).sum()));
    let pb = Arc::new(Mutex::new(pb));

    // -----------------------------------------------------------------------------------------
    // Download Video & Audio Streams
    // -----------------------------------------------------------------------------------------

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads as usize)
        .build()
        .unwrap();

    let one_stream = streams.len() == 1;

    for stream in streams {
        pb.lock().unwrap().write(format!(
            " {} {} stream {}",
            "Processing".colorize("bold green"),
            stream.media_type,
            stream.display_stream().colorize("cyan"),
        ))?;

        let length = stream.segments.len();

        if length == 0 {
            pb.lock().unwrap().write(format!(
                "    {} skipping stream (no segments)",
                "Warning".colorize("bold yellow"),
            ))?;
            continue;
        }

        let mut temp_file = stream.file_path(&directory, &stream.extension());

        if let Some(output) = &output {
            if one_stream && output.extension() == Some(stream.extension()) {
                temp_file = output.to_owned();
            }
        }

        temp_files.push(Stream {
            language: stream.language.clone(),
            media_type: stream.media_type.clone(),
            path: temp_file.clone(),
        });
        pb.lock().unwrap().write(format!(
            "{} stream to {}",
            "Downloading".colorize("bold green"),
            temp_file.to_string_lossy().colorize("cyan"),
        ))?;

        let merger = Arc::new(Mutex::new(if no_merge {
            Merger::new_directory(stream.segments.len(), &temp_file)?
        } else {
            Merger::new_file(stream.segments.len(), &temp_file)?
        }));
        let timer = Arc::new(Instant::now());

        let _ = relative_sizes.pop_front();
        let relative_size = relative_sizes.iter().sum();
        let mut iv_present = false;
        let mut decrypter = Decrypter::None;
        let mut previous_map = None;

        let stream_base_url = base_url
            .clone()
            .unwrap_or(stream.uri.parse::<Url>().unwrap());

        let mut thread_datas = Vec::with_capacity(stream.segments.len());

        for (i, segment) in stream.segments.iter().enumerate() {
            if let Some(map) = &segment.map {
                let url = stream_base_url.join(&map.uri)?;
                let mut request = client.get(url);

                if let Some(range) = &map.range {
                    request = request.header(header::RANGE, range.as_header_value());
                }

                let response = request.send()?;
                let bytes = response.bytes()?;
                previous_map = Some(bytes.to_vec())
            }

            if !no_decrypt {
                if !decrypter.is_none() && !iv_present {
                    decrypter.increment_iv();
                }

                if let Some(key) = &segment.key {
                    match key.method {
                        KeyMethod::Aes128 | KeyMethod::SampleAes => {
                            if !keys.is_empty() {
                                bail!("custom keys with AES-128 encryption is not supported");
                            }

                            iv_present = key.iv.is_some();

                            if let Some(uri) = &key.uri {
                                let bytes =
                                    client.get(stream_base_url.join(uri)?).send()?.bytes()?;
                                decrypter = Decrypter::new_hls_aes(
                                    key.key(&bytes)?,
                                    key.iv(stream.media_sequence + (i as u64))?,
                                    &key.method,
                                )?;
                            } else {
                                bail!("uri cannot be none when key method is AES-128/SAMPLE-AES");
                            }
                        }
                        KeyMethod::Mp4Decrypt => {
                            let default_kid = stream.default_kid();
                            let mut decryption_keys = HashMap::new();

                            if all_keys {
                                for key in &keys {
                                    if let Some(kid) = &key.0 {
                                        decryption_keys.insert(kid.to_owned(), key.1.to_owned());
                                    } else if let Some(default_kid) = &default_kid {
                                        decryption_keys
                                            .insert(default_kid.to_owned(), key.1.to_owned());
                                    }
                                }
                            } else {
                                for key in &keys {
                                    if let Some(default_kid) = &default_kid {
                                        if let Some(kid) = &key.0 {
                                            if default_kid == kid {
                                                decryption_keys
                                                    .insert(kid.to_owned(), key.1.to_owned());
                                            }
                                        } else {
                                            decryption_keys
                                                .insert(default_kid.to_owned(), key.1.to_owned());
                                        }
                                    }
                                }
                            }

                            if decryption_keys.is_empty() {
                                bail!(
                                    "cannot determine keys to use, bypass this error using {} flag.",
                                    "--all-keys".colorize("bold green")
                                );
                            }

                            for key in &decryption_keys {
                                pb.lock().unwrap().write(format!(
                                    "        {} {}:{}",
                                    "Key".colorize("bold green"),
                                    key.0,
                                    key.1
                                ))?;
                            }

                            decrypter = Decrypter::Mp4Decrypt(decryption_keys);
                        }
                        _ => decrypter = Decrypter::None,
                    }
                }
            }

            let url = stream_base_url.join(&segment.uri)?;
            let mut request = client.get(url);

            if let Some(range) = &segment.range {
                request = request.header(header::RANGE, range.as_header_value());
            }

            let thread_data = ThreadData {
                downloaded_bytes,
                index: i,
                decrypter: decrypter.clone(),
                map: previous_map.clone(),
                merger: merger.clone(),
                pb: pb.clone(),
                relative_size,
                request,
                timer: timer.clone(),
                total_retries: retry_count,
            };

            if decrypter.is_none() {
                previous_map = None;
            }

            thread_datas.push(thread_data);
        }

        pool.scope_fifo(|s| {
            for thread_data in thread_datas {
                s.spawn_fifo(move |_| {
                    if let Err(e) = thread_data.execute() {
                        let _lock = thread_data.pb.lock().unwrap();
                        println!("\n{}: {}", "error".colorize("bold red"), e);
                        std::process::exit(1);
                    }
                });
            }
        });

        let mut merger = merger.lock().unwrap();
        merger.flush()?;

        if !merger.buffered() {
            bail!(
                "failed to download {} stream to {}",
                stream.display_stream().colorize("cyan"),
                temp_file.to_string_lossy()
            );
        }

        downloaded_bytes += merger.stored();

        pb.lock().unwrap().write(format!(
            " {} stream successfully",
            "Downloaded".colorize("bold green"),
        ))?;
    }

    eprintln!();

    // -----------------------------------------------------------------------------------------
    // Mux Downloaded Streams
    // -----------------------------------------------------------------------------------------

    if should_mux {
        mux::ffmpeg(output.as_ref(), &temp_files)?;
        mux::delete_temp_files(directory.as_ref(), &temp_files)?;
    }

    Ok(())
}

struct ThreadData {
    downloaded_bytes: usize,
    index: usize,
    decrypter: Decrypter,
    map: Option<Vec<u8>>,
    merger: Arc<Mutex<Merger>>,
    pb: Arc<Mutex<RichProgress>>,
    relative_size: usize,
    request: RequestBuilder,
    timer: Arc<Instant>,
    total_retries: u8,
}

impl ThreadData {
    fn execute(&self) -> Result<()> {
        let mut segment = self.map.clone().unwrap_or_default();
        segment.append(&mut self.download_segment()?);

        segment = self.decrypter.decrypt(segment)?;

        let mut merger = self.merger.lock().unwrap();
        merger.write(self.index, &segment)?;
        merger.flush()?;

        self.notify(merger.stored(), merger.estimate())?;
        Ok(())
    }

    fn download_segment(&self) -> Result<Vec<u8>> {
        for _ in 0..self.total_retries {
            let response = match self.request.try_clone().unwrap().send() {
                Ok(response) => response,
                Err(error) => {
                    self.pb
                        .lock()
                        .unwrap()
                        .write(check_reqwest_error(&error)?)?;
                    continue;
                }
            };

            let status = response.status();

            if status.is_client_error() || status.is_server_error() {
                bail!("failed to fetch segments");
            }

            let data = response.bytes()?.to_vec();
            let elapsed_time = self.timer.elapsed().as_secs() as usize;

            if elapsed_time != 0 {
                let stored = self.merger.lock().unwrap().stored() + data.len();
                self.pb.lock().unwrap().replace(
                    12,
                    Column::Text(format!(
                        "[yellow]{}/s",
                        utils::format_bytes(stored / elapsed_time, 2).2
                    )),
                );
            }

            return Ok(data);
        }

        bail!("reached maximum number of retries to download a segment");
    }

    fn notify(&self, stored: usize, estimate: usize) -> Result<()> {
        let mut pb = self.pb.lock().unwrap();
        pb.replace(
            0,
            Column::Text(format!(
                "[bold blue]{}",
                utils::format_download_bytes(
                    self.downloaded_bytes + stored,
                    self.downloaded_bytes + estimate + self.relative_size,
                ),
            )),
        );
        pb.update(1).unwrap();
        Ok(())
    }
}

fn check_reqwest_error(error: &reqwest::Error) -> Result<String> {
    let request = "Request".colorize("bold yellow");
    let url = error.url().unwrap();

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
