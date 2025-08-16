use crate::{
    downloader::{encryption::Decrypter, mux::Stream},
    merger::Merger,
    playlist::{KeyMethod, MediaPlaylist, MediaType},
    utils,
};
use anyhow::{Result, bail};
use kdam::{BarExt, Column, RichProgress, term::Colorizer};
use rayon::{ThreadPool, ThreadPoolBuilder};
use reqwest::{
    StatusCode, Url,
    blocking::{Client, RequestBuilder},
    header,
};
use std::{
    collections::{HashMap, VecDeque},
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
    mut pb: RichProgress,
    query: &HashMap<String, String>,
    retries: u8,
    streams: Vec<MediaPlaylist>,
    threads: u8,
    temp_files: &mut Vec<Stream>,
) -> Result<()> {
    let mut streams = streams
        .into_iter()
        .filter(|x| x.media_type != MediaType::Subtitles)
        .collect::<Vec<_>>();

    let mut downloaded_bytes = 0;
    let mut estimated_bytes = VecDeque::new();

    for stream in &mut streams {
        estimated_bytes.push_back(stream.estimate_size(base_url, client, query)?);
    }

    pb.columns.extend_from_slice(&[
        Column::Text("•".to_owned()),
        Column::Text("[yellow]?".to_owned()), // download speed
    ]);
    let pb = Arc::new(Mutex::new(pb));
    let pool = ThreadPoolBuilder::new()
        .num_threads(threads as usize)
        .build()
        .unwrap();

    for stream in streams {
        pb.lock().unwrap().write(format!(
            " {} [{:>5}] {}",
            "Processing".colorize("cyan"),
            stream.media_type.to_string(),
            stream.display_stream(),
        ))?;

        if stream.segments.is_empty() {
            pb.lock().unwrap().write(format!(
                "    {} skipping stream (no segments)",
                "Warning".colorize("yellow"),
            ))?;
            continue;
        }

        let temp_file = stream.path(directory, stream.extension());

        temp_files.push(Stream {
            language: stream.language.clone(),
            media_type: stream.media_type.clone(),
            path: temp_file.clone(),
        });

        let _ = estimated_bytes.pop_front();

        pb.lock().unwrap().write(format!(
            "{} {}",
            "Downloading".colorize("bold green"),
            temp_file.to_string_lossy(),
        ))?;
        download_stream(
            base_url,
            client,
            &mut downloaded_bytes,
            decrypter.clone(),
            estimated_bytes.iter().sum(),
            no_decrypt,
            no_merge,
            pb.clone(),
            &pool,
            query,
            retries,
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
    decrypter: Decrypter,
    estimated_bytes: usize,
    no_decrypt: bool,
    no_merge: bool,
    pb: Arc<Mutex<RichProgress>>,
    pool: &ThreadPool,
    query: &HashMap<String, String>,
    retries: u8,
    stream: MediaPlaylist,
    temp_file: &PathBuf,
) -> Result<()> {
    let mut init_seg = None;
    let merger = Arc::new(Mutex::new(if no_merge {
        Merger::new_directory(stream.segments.len(), temp_file)?
    } else {
        Merger::new_file(stream.segments.len(), temp_file)?
    }));
    let mut increment_iv = false;
    let base_url = base_url
        .clone()
        .unwrap_or(stream.uri.parse::<Url>().unwrap());
    let mut threads = Vec::with_capacity(stream.segments.len());
    let timer = Arc::new(Instant::now());

    let mut default_kid = None;
    let mut stream_decrypter = decrypter.clone();

    for (i, segment) in stream.segments.iter().enumerate() {
        if let Some(map) = &segment.map {
            let url = base_url.join(&map.uri)?;
            let mut request = client.get(url).query(query);

            if let Some(range) = &map.range {
                request = request.header(header::RANGE, range.as_header_value());
            }

            let response = request.send()?;
            let bytes = response.bytes()?;

            //             Pssh {
            //     key_ids: [
            //         KeyId {
            //             system_type: PlayReady,
            //             value: "000000003c42c8c86331202020202020",
            //         },
            //         KeyId {
            //             system_type: WideVine,
            //             value: "00000000423cc8c86331202020202020",
            //         },
            //     ],
            //     system_ids: [
            //         "9a04f07998404286ab92e65be0885f95",
            //         "edef8ba979d64acea3c827dcd51d21ed",
            //     ],
            // }
            // println!("{:#?}", vsd_mp4::pssh::Pssh::new(&bytes).unwrap());

            default_kid = vsd_mp4::pssh::default_kid(&bytes)?.or(stream.default_kid());
            init_seg = Some(bytes.to_vec())
        }

        if !no_decrypt {
            if increment_iv {
                stream_decrypter.increment_iv();
            }

            if let Some(key) = &segment.key {
                match key.method {
                    KeyMethod::Aes128 | KeyMethod::SampleAes => {
                        let url = base_url.join(key.uri.as_ref().unwrap())?;
                        let request = client.get(url).query(query);
                        let response = request.send()?;
                        let bytes = response.bytes()?;

                        stream_decrypter = Decrypter::new_hls_aes(
                            key.key(&bytes)?,
                            key.iv(stream.media_sequence)?,
                            &key.method,
                        );

                        if key.method == KeyMethod::SampleAes && key.iv.is_none() {
                            increment_iv = true;
                        }
                    }
                    KeyMethod::Mp4Decrypt => {
                        if let Decrypter::Mp4Decrypt(kid_key_pairs) = &stream_decrypter {
                            if let Some(default_kid) = &default_kid {
                                // We already checked this before hand
                                let key = kid_key_pairs.get(default_kid).unwrap();

                                pb.lock().unwrap().write(format!(
                                    "        {} {}:{}",
                                    "Key".colorize("bold red"),
                                    default_kid,
                                    key,
                                ))?;

                                stream_decrypter = Decrypter::Mp4Decrypt(HashMap::from([(
                                    default_kid.to_owned(),
                                    key.to_owned(),
                                )]));
                            }
                        } else {
                            bail!("custom keys (KID:KEY;...) are required to continue further.",);
                        }
                    }
                    _ => (),
                }
            }
        }

        let url = base_url.join(&segment.uri)?;
        let mut request = client.get(url).query(query);

        if let Some(range) = &segment.range {
            request = request.header(header::RANGE, range.as_header_value());
        }

        threads.push(Thread {
            decrypter: stream_decrypter.clone(),
            downloaded_bytes: *downloaded_bytes,
            estimated_bytes,
            index: i,
            init_seg: init_seg.clone(),
            merger: merger.clone(),
            pb: pb.clone(),
            request,
            retries,
            timer: timer.clone(),
        });

        if stream_decrypter.is_none() {
            init_seg = None;
        }
    }

    pool.scope_fifo(|s| {
        for mut thread in threads {
            s.spawn_fifo(move |_| {
                if let Err(e) = thread.execute() {
                    let _lock = thread.pb.lock().unwrap();
                    println!("\n{}: {}", "error".colorize("bold red"), e);
                    std::process::exit(1);
                }
            });
        }
    });

    let mut merger = merger.lock().unwrap();
    merger.flush()?;

    if !merger.buffered() {
        bail!("failed to download stream.",);
    }

    *downloaded_bytes += merger.stored();

    pb.lock().unwrap().write(format!(
        " {} stream successfully",
        "Downloaded".colorize("bold green"),
    ))?;

    Ok(())
}

struct Thread {
    decrypter: Decrypter,
    downloaded_bytes: usize,
    estimated_bytes: usize,
    index: usize,
    init_seg: Option<Vec<u8>>,
    merger: Arc<Mutex<Merger>>,
    pb: Arc<Mutex<RichProgress>>,
    request: RequestBuilder,
    retries: u8,
    timer: Arc<Instant>,
}

impl Thread {
    fn execute(&mut self) -> Result<()> {
        let mut segment = Vec::new();

        if let Some(init_segment) = &mut self.init_seg {
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

        bail!("reached max retries to download a segment.");
    }
}

fn check_reqwest_error(error: &reqwest::Error) -> Result<String> {
    let request = "Request".colorize("yellow");
    let url = error.url().unwrap();

    if error.is_connect() {
        return Ok(format!("    {request} {url} (connection error)"));
    } else if error.is_timeout() {
        return Ok(format!("    {request} {url} (timeout)"));
    }

    if let Some(status) = error.status() {
        match status {
            StatusCode::GATEWAY_TIMEOUT => Ok(format!("    {request} {url} (gateway timeout)")),
            StatusCode::REQUEST_TIMEOUT => Ok(format!("    {request} {url} (timeout)")),
            StatusCode::SERVICE_UNAVAILABLE => {
                Ok(format!("    {request} {url} (service unavailable)"))
            }
            StatusCode::TOO_MANY_REQUESTS => Ok(format!("    {request} {url} (too many requests)")),
            _ => bail!("download failed {} (HTTP {})", url, status),
        }
    } else {
        bail!("download failed {}", url)
    }
}
