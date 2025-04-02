use crate::{
    downloader::{encryption::Decrypter, mux::Stream},
    merger::Merger,
    playlist::{KeyMethod, MediaPlaylist},
    utils,
};
use anyhow::{bail, Result};
use kdam::{term::Colorizer, BarExt, Column, RichProgress};
use rayon::{ThreadPool, ThreadPoolBuilder};
use reqwest::{
    blocking::{Client, RequestBuilder},
    header, StatusCode, Url,
};
use std::{
    collections::VecDeque,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Instant,
};

#[allow(clippy::too_many_arguments)]
pub fn download_streams(
    base_url: &Option<Url>,
    client: &Client,
    decrypter: Decrypter,
    directory: Option<&PathBuf>,
    no_decrypt: bool,
    no_merge: bool,
    output: Option<&PathBuf>,
    pb: RichProgress,
    retry_count: u8,
    streams: Vec<MediaPlaylist>,
    threads: u8,
    temp_files: &mut Vec<Stream>,
) -> Result<()> {
    let mut estimated_bytes = VecDeque::new();

    for stream in &streams {
        estimated_bytes.push_back(stream.estimate_size(base_url, client)?);
    }

    let mut temp_file = None;

    if streams.len() == 1 {
        if let Some(output) = output {
            if output.extension() == Some(streams.first().unwrap().extension()) {
                temp_file = Some(output.to_owned());
            }
        }
    }

    let mut downloaded_bytes = 0;
    let pb = Arc::new(Mutex::new(pb));
    let pool = ThreadPoolBuilder::new()
        .num_threads(threads as usize)
        .build()
        .unwrap();

    for stream in streams {
        let temp_file = temp_file
            .clone()
            .unwrap_or(stream.file_path(directory, stream.extension()));

        temp_files.push(Stream {
            language: stream.language.clone(),
            media_type: stream.media_type.clone(),
            path: temp_file.clone(),
        });

        let _ = estimated_bytes.pop_front();

        download_stream(
            base_url,
            client,
            &mut downloaded_bytes,
            decrypter.clone(),
            estimated_bytes.iter().sum::<usize>(),
            no_decrypt,
            no_merge,
            pb.clone(),
            &pool,
            retry_count,
            stream,
            &temp_file,
        )?;
    }

    eprintln!();
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn download_stream(
    base_url: &Option<Url>,
    client: &Client,
    downloaded_bytes: &mut usize,
    mut decrypter: Decrypter,
    estimated_bytes: usize,
    no_decrypt: bool,
    no_merge: bool,
    pb: Arc<Mutex<RichProgress>>,
    pool: &ThreadPool,
    retry_count: u8,
    stream: MediaPlaylist,
    temp_file: &PathBuf,
) -> Result<()> {
    pb.lock().unwrap().write(format!(
        " {} {} stream {}",
        "Processing".colorize("bold green"),
        stream.media_type,
        stream.display_stream().colorize("cyan"),
    ))?;
    pb.lock().unwrap().write(format!(
        "{} stream to {}",
        "Downloading".colorize("bold green"),
        temp_file.to_string_lossy().colorize("cyan"),
    ))?;

    let mut download_threads = Vec::with_capacity(stream.segments.len());
    let mut init_segment = None;
    let hls_custom_key = decrypter.is_hls_aes_and_not_defined();
    let merger = Arc::new(Mutex::new(if no_merge {
        Merger::new_directory(stream.segments.len(), temp_file)?
    } else {
        Merger::new_file(stream.segments.len(), temp_file)?
    }));
    let mut no_iv = false;
    let base_url = base_url
        .clone()
        .unwrap_or(stream.uri.parse::<Url>().unwrap());
    let timer = Arc::new(Instant::now());

    for (i, segment) in stream.segments.iter().enumerate() {
        if let Some(map) = &segment.map {
            let url = base_url.join(&map.uri)?;
            let mut request = client.get(url);

            if let Some(range) = &map.range {
                request = request.header(header::RANGE, range.as_header_value());
            }

            let response = request.send()?;
            let bytes = response.bytes()?;
            init_segment = Some(bytes.to_vec())
        }

        if !no_decrypt {
            if i == 0 && hls_custom_key {
                decrypter.update_enc_type(&KeyMethod::Aes128);
                decrypter.update_iv(stream.media_sequence);
                no_iv = true;
            }

            if no_iv {
                decrypter.increment_iv();
            }

            if let Some(key) = &segment.key {
                match key.method {
                    KeyMethod::Aes128 | KeyMethod::SampleAes => {
                        if hls_custom_key {
                            decrypter.update_enc_type(&key.method);
                        } else {
                            let url = base_url.join(key.uri.as_ref().unwrap())?;
                            let bytes = client.get(url).send()?.bytes()?;

                            decrypter = Decrypter::new_hls_aes(
                                key.key(&bytes)?,
                                key.iv(stream.media_sequence)?,
                                &key.method,
                            );

                            no_iv = key.iv.is_none();
                        }
                    }
                    KeyMethod::Mp4Decrypt => {
                        if let Decrypter::Mp4Decrypt(kid_key_pairs) = &decrypter {
                            if let Some(default_kid) = stream.default_kid() {
                                pb.lock().unwrap().write(format!(
                                    "        {} {}:{}",
                                    "Key".colorize("bold green"),
                                    default_kid,
                                    kid_key_pairs.get(&default_kid).unwrap(), // We already checked this before hand
                                ))?;
                            }
                        } else {
                            bail!(
                                "custom keys (KID:KEY;KID:KEY...) are required to continue further.",
                            );
                        }
                    }
                    _ => (),
                }
            }
        }

        let url = base_url.join(&segment.uri)?;
        let mut request = client.get(url);

        if let Some(range) = &segment.range {
            request = request.header(header::RANGE, range.as_header_value());
        }

        download_threads.push(ThreadData {
            decrypter: decrypter.clone(),
            downloaded_bytes: *downloaded_bytes,
            estimated_bytes,
            index: i,
            init_segment: init_segment.clone(),
            merger: merger.clone(),
            pb: pb.clone(),
            request,
            retries: retry_count,
            timer: timer.clone(),
        });

        if decrypter.is_none() {
            init_segment = None;
        }
    }

    pool.scope_fifo(|s| {
        for mut thread_data in download_threads {
            s.spawn_fifo(move |_| {
                if let Err(e) = thread_data.execute() {
                    let _lock = thread_data.pb.lock().unwrap();
                    println!("\n{}: {}", "error".colorize("bold red"), e);
                    // TODO - Add resume support
                    std::process::exit(1);
                }
            });
        }
    });

    let mut merger = merger.lock().unwrap();
    merger.flush()?;

    if !merger.buffered() {
        bail!(
            "failed to download stream to {}",
            temp_file.to_string_lossy()
        );
    }

    *downloaded_bytes += merger.stored();

    pb.lock().unwrap().write(format!(
        " {} stream successfully",
        "Downloaded".colorize("bold green"),
    ))?;

    Ok(())
}

struct ThreadData {
    decrypter: Decrypter,
    downloaded_bytes: usize,
    estimated_bytes: usize,
    index: usize,
    init_segment: Option<Vec<u8>>,
    merger: Arc<Mutex<Merger>>,
    pb: Arc<Mutex<RichProgress>>,
    request: RequestBuilder,
    retries: u8,
    timer: Arc<Instant>,
}

impl ThreadData {
    fn execute(&mut self) -> Result<()> {
        let mut segment = Vec::new();

        if let Some(init_segment) = &mut self.init_segment {
            segment.append(init_segment);
        }

        segment.append(&mut self.segment()?);
        segment = self.decrypter.decrypt(segment)?;

        let mut merger = self.merger.lock().unwrap();
        merger.write(self.index, &segment)?;
        merger.flush()?;

        self.notify(merger.stored(), merger.estimate())?;
        Ok(())
    }

    fn notify(&self, stored: usize, estimate: usize) -> Result<()> {
        let mut pb = self.pb.lock().unwrap();
        pb.replace(
            0,
            Column::Text(format!(
                "[bold blue]{}",
                utils::format_download_bytes(
                    self.downloaded_bytes + stored,
                    self.downloaded_bytes + estimate + self.estimated_bytes,
                ),
            )),
        );
        pb.update(1)?;
        Ok(())
    }

    fn segment(&self) -> Result<Vec<u8>> {
        for _ in 0..self.retries {
            let response = match self.request.try_clone().unwrap().send() {
                Ok(response) => response,
                Err(error) => {
                    // TODO - Only print this info on verbose logging
                    self.pb
                        .lock()
                        .unwrap()
                        .write(check_reqwest_error(&error)?)?;
                    continue;
                }
            };

            let status = response.status();

            if status.is_client_error() || status.is_server_error() {
                bail!("failed to fetch segments.");
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

        bail!("reached maximum number of retries to download a segment.");
    }
}

fn check_reqwest_error(error: &reqwest::Error) -> Result<String> {
    let request = "Request".colorize("bold yellow");
    let url = error.url().unwrap();

    if error.is_connect() {
        return Ok(format!("    {} {} (connection error)", request, url));
    } else if error.is_timeout() {
        return Ok(format!("    {} {} (timeout)", request, url));
    }

    if let Some(status) = error.status() {
        match status {
            StatusCode::GATEWAY_TIMEOUT => Ok(format!("    {} {} (gateway timeout)", request, url)),
            StatusCode::REQUEST_TIMEOUT => Ok(format!("    {} {} (timeout)", request, url)),
            StatusCode::SERVICE_UNAVAILABLE => {
                Ok(format!("    {} {} (service unavailable)", request, url))
            }
            StatusCode::TOO_MANY_REQUESTS => {
                Ok(format!("    {} {} (too many requests)", request, url))
            }
            _ => bail!("download failed {} (HTTP {})", url, status),
        }
    } else {
        bail!("download failed {}", url)
    }
}
