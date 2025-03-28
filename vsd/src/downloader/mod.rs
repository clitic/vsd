mod encryption;
mod fetch;
mod parse;
mod subtitle;

use encryption::Decrypter;
pub use fetch::{fetch_playlist, InputMetadata};
pub use parse::{parse_all_streams, parse_selected_streams};
pub use subtitle::download_subtitle_streams;

use crate::{
    merger::Merger,
    playlist::{KeyMethod, MediaPlaylist, MediaType, Range, Segment},
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
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Arc, Mutex},
    time::Instant,
};

pub type SelectedPlaylists = (Vec<MediaPlaylist>, Vec<MediaPlaylist>);

pub struct Prompts {
    pub skip: bool,
    pub raw: bool,
}

pub struct Stream {
    pub file_path: String,
    pub language: Option<String>,
    pub media_type: MediaType,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn download(
    all_keys: bool,
    base_url: Option<Url>,
    client: Client,
    directory: Option<PathBuf>,
    keys: Vec<(Option<String>, String)>,
    no_decrypt: bool,
    no_merge: bool,
    output: Option<String>,
    selected_playlists: SelectedPlaylists,
    retry_count: u8,
    threads: u8,
) -> Result<()> {
    let (mut video_audio_streams, subtitle_streams) = selected_playlists;

    let one_stream = (video_audio_streams.len() == 1) && subtitle_streams.is_empty();
    let mut should_mux = !no_decrypt && !no_merge;

    if let Some(output) = &output {
        if one_stream
            && output.ends_with(&format!(
                ".{}",
                video_audio_streams.first().unwrap().extension()
            ))
        {
            should_mux = false;
        }
    }

    let video_streams_count = video_audio_streams
        .iter()
        .filter(|x| x.media_type == MediaType::Video)
        .count();

    if video_streams_count > 1 {
        should_mux = false;
    }

    if should_mux && utils::find_ffmpeg().is_none() {
        bail!("ffmpeg couldn't be found, it is required to continue further.");
    }

    // -----------------------------------------------------------------------------------------
    // Parse Key Ids
    // -----------------------------------------------------------------------------------------

    if !no_decrypt {
        encryption::check_unsupported_encryptions(&video_audio_streams)?;
        let default_kids =
            encryption::extract_default_kids(&base_url, &client, &video_audio_streams)?;
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

    if output.is_some() {
        let video_streams_count = video_audio_streams
            .iter()
            .filter(|x| x.media_type == MediaType::Video)
            .count();
        let audio_streams_count = video_audio_streams
            .iter()
            .filter(|x| x.media_type == MediaType::Audio)
            .count();

        if video_streams_count > 1 {
            println!(
                "    {} --output is ignored when multiple video streams are selected",
                "Warning".colorize("bold yellow")
            );
        }

        if video_streams_count == 0
            && (audio_streams_count > 1
                || subtitle_streams.len() > 1
                || (audio_streams_count != 0 && !subtitle_streams.is_empty()))
        {
            println!(
                "    {} --output is ignored when no video streams is selected but multiple audio/subtitle streams are selected",
                "Warning".colorize("bold yellow")
            );
        }

        if no_decrypt {
            println!(
                "    {} --output is ignored when --no-decrypt is used",
                "Warning".colorize("bold yellow")
            );
        }

        if no_merge {
            println!(
                "    {} --output is ignored when --no-merge is used",
                "Warning".colorize("bold yellow")
            );
        }
    }

    if !subtitle_streams.is_empty() && no_merge {
        println!(
            "    {} subtitle streams are always merged even if --no-merge is used",
            "Warning".colorize("bold yellow")
        );
    }

    let mut temp_files = vec![];

    // -----------------------------------------------------------------------------------------
    // Download Subtitle Streams
    // -----------------------------------------------------------------------------------------

    download_subtitle_streams(
        base_url.clone(),
        &client,
        &directory,
        &subtitle_streams,
        &mut pb,
        &mut temp_files,
    )?;

    // -----------------------------------------------------------------------------------------
    // Estimation
    // -----------------------------------------------------------------------------------------

    let mut downloaded_bytes = 0;
    let mut relative_sizes = VecDeque::new();

    for stream in video_audio_streams.iter_mut() {
        let stream_base_url = base_url
            .clone()
            .unwrap_or(stream.uri.parse::<Url>().unwrap());

        let total_segments = stream.segments.len();
        let buffer_size = 1024 * 1024 * 2; // 2 MiB
        let mut ranges = None;

        if let Some(segment) = stream.segments.first() {
            let url = stream_base_url.join(&segment.uri)?;
            let mut request = client.head(url.clone());

            if total_segments == 1 {
                let response = request.send()?;
                let content_length = response
                    .headers()
                    .get(header::CONTENT_LENGTH)
                    .map(|x| x.to_str().unwrap().parse::<usize>().unwrap())
                    .unwrap_or(0);

                if content_length == 0 {
                    bail!(
                        "cannot download a single segment ({}) of unknown content length.",
                        url
                    );
                } else {
                    ranges = Some(PartialRangeIter {
                        start: 0,
                        end: content_length as u64 - 1,
                        buffer_size,
                    });
                    relative_sizes.push_back(content_length);
                }
            } else {
                if let Some(range) = &segment.range {
                    request = request.header(header::RANGE, range.as_header_value());
                }

                let response = request.send()?;
                let content_length = response
                    .headers()
                    .get(header::CONTENT_LENGTH)
                    .map(|x| x.to_str().unwrap().parse::<usize>().unwrap())
                    .unwrap_or(0);

                relative_sizes.push_back(total_segments * content_length);
            }
        }

        if let Some(ranges) = ranges {
            let segment = stream.segments.remove(0);

            for (i, range) in ranges.enumerate() {
                if i == 0 {
                    let mut segment_copy = segment.clone();
                    segment_copy.range = Some(range);
                    stream.segments.push(segment_copy);
                } else {
                    stream.segments.push(Segment {
                        range: Some(range),
                        duration: segment.duration,
                        uri: segment.uri.clone(),
                        ..Default::default()
                    });
                }
            }
        }
    }

    // -----------------------------------------------------------------------------------------
    // Prepare Progress Bar
    // -----------------------------------------------------------------------------------------

    pb.replace(2, Column::Percentage(2));
    pb.columns.extend_from_slice(&[
        Column::Text("•".to_owned()),
        Column::Text("[yellow]?".to_owned()),
    ]);
    pb.pb.reset(Some(
        video_audio_streams.iter().map(|x| x.segments.len()).sum(),
    ));
    let pb = Arc::new(Mutex::new(pb));

    // -----------------------------------------------------------------------------------------
    // Download Video & Audio Streams
    // -----------------------------------------------------------------------------------------

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads as usize)
        .build()
        .unwrap();

    for stream in video_audio_streams {
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

        let mut temp_file = stream
            .file_path(&directory, &stream.extension())
            .to_string_lossy()
            .to_string();

        if let Some(output) = &output {
            if one_stream && output.ends_with(&format!(".{}", stream.extension())) {
                temp_file = output.to_owned();
            }
        }

        temp_files.push(Stream {
            file_path: temp_file.clone(),
            language: stream.language.clone(),
            media_type: stream.media_type.clone(),
        });
        pb.lock().unwrap().write(format!(
            "{} stream to {}",
            "Downloading".colorize("bold green"),
            temp_file.colorize("cyan"),
        ))?;

        let merger = Arc::new(Mutex::new(if no_merge {
            Merger::with_directory(stream.segments.len(), &temp_file)?
        } else {
            Merger::new(stream.segments.len(), &temp_file)?
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
                                decrypter = Decrypter::new_aes(
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

                            decrypter = Decrypter::ClearKey(decryption_keys);
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
                temp_file
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

    let video_temp_files = temp_files
        .iter()
        .filter(|x| (x.media_type == MediaType::Video) || (x.media_type == MediaType::Undefined))
        .collect::<Vec<_>>();
    let video_streams_count = video_temp_files.len();
    let audio_streams_count = temp_files
        .iter()
        .filter(|x| x.media_type == MediaType::Audio)
        .count();
    let subtitle_streams_count = temp_files
        .iter()
        .filter(|x| x.media_type == MediaType::Subtitles)
        .count();

    if should_mux
        && (video_streams_count == 1 || audio_streams_count == 1 || subtitle_streams_count == 1)
    {
        if let Some(output) = &output {
            let all_temp_files = temp_files
                .iter()
                .filter(|x| {
                    (x.media_type == MediaType::Video) || (x.media_type == MediaType::Undefined)
                })
                .chain(
                    temp_files
                        .iter()
                        .filter(|x| x.media_type == MediaType::Audio),
                )
                .chain(
                    temp_files
                        .iter()
                        .filter(|x| x.media_type == MediaType::Subtitles),
                )
                .collect::<Vec<_>>();

            let mut args = vec![];

            for temp_file in &all_temp_files {
                args.extend_from_slice(&["-i".to_owned(), temp_file.file_path.clone()]);
            }

            if (video_streams_count == 1)
                || (audio_streams_count == 1)
                || (subtitle_streams_count == 1)
            {
                // TODO - Re-consider this copy
                args.extend_from_slice(&["-c".to_owned(), "copy".to_owned()]);
            } else {
                args.extend_from_slice(&["-c".to_owned(), "copy".to_owned()]);

                if subtitle_streams_count > 0 && output.ends_with(".mp4") {
                    args.extend_from_slice(&["-c:s".to_owned(), "mov_text".to_owned()]);
                }

                for i in 0..all_temp_files.len() {
                    args.extend_from_slice(&["-map".to_owned(), i.to_string()]);
                }

                let mut audio_index = 0;
                let mut subtitle_index = 0;

                for temp_file in &all_temp_files {
                    match temp_file.media_type {
                        MediaType::Audio => {
                            if let Some(language) = &temp_file.language {
                                args.extend_from_slice(&[
                                    format!("-metadata:s:a:{}", audio_index),
                                    format!("language={}", language),
                                ]);
                            }

                            audio_index += 1;
                        }
                        MediaType::Subtitles => {
                            if let Some(language) = &temp_file.language {
                                args.extend_from_slice(&[
                                    format!("-metadata:s:s:{}", subtitle_index),
                                    format!("language={}", language),
                                ]);
                            }

                            subtitle_index += 1;
                        }
                        _ => (),
                    }
                }

                if subtitle_streams_count > 0 {
                    args.extend_from_slice(&["-disposition:s:0".to_owned(), "default".to_owned()]);
                }
            }

            args.push(output.to_owned());

            println!(
                "  {} ffmpeg {}",
                "Executing".colorize("bold cyan"),
                args.iter()
                    .map(|x| if x.contains(' ') {
                        format!("\"{}\"", x)
                    } else {
                        x.to_owned()
                    })
                    .collect::<Vec<_>>()
                    .join(" ")
            );

            if Path::new(output).exists() {
                println!("   {} {}", "Deleting".colorize("bold red"), output);
                std::fs::remove_file(output)?;
            }

            let code = Command::new(utils::find_ffmpeg().unwrap())
                .args(args)
                .stderr(Stdio::null())
                .spawn()?
                .wait()?;

            if !code.success() {
                bail!("ffmpeg exited with code {}", code.code().unwrap_or(1))
            }

            for temp_file in &all_temp_files {
                println!(
                    "   {} {}",
                    "Deleting".colorize("bold red"),
                    temp_file.file_path
                );
                std::fs::remove_file(&temp_file.file_path)?;
            }

            if let Some(directory) = &directory {
                if std::fs::read_dir(directory)?.next().is_none() {
                    println!(
                        "   {} {}",
                        "Deleting".colorize("bold red"),
                        directory.to_string_lossy()
                    );
                    std::fs::remove_dir(directory)?;
                }
            }
        }
    }

    Ok(())
}

// https://rust-lang-nursery.github.io/rust-cookbook/web/clients/download.html#make-a-partial-download-with-http-range-headers
struct PartialRangeIter {
    start: u64,
    end: u64,
    buffer_size: u32,
}

impl Iterator for PartialRangeIter {
    type Item = Range;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start > self.end {
            None
        } else {
            let prev_start = self.start;
            self.start += std::cmp::min(self.buffer_size as u64, self.end - self.start + 1);
            Some(Range {
                start: prev_start,
                end: self.start - 1,
            })
        }
    }
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
