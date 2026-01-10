use crate::{
    downloader::{MAX_RETRIES, MAX_THREADS, encryption::Decrypter, mux::Stream},
    merger::Merger,
    playlist::{KeyMethod, MediaPlaylist, MediaType},
    progress::Progress,
};
use anyhow::{Result, bail};
use log::{error, info, trace, warn};
use reqwest::{Client, RequestBuilder, StatusCode, Url, header};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex, atomic::Ordering},
};
use tokio::task::JoinSet;

#[allow(clippy::too_many_arguments)]
pub async fn download_streams(
    base_url: &Option<Url>,
    client: &Client,
    decrypter: Decrypter,
    directory: Option<&PathBuf>,
    no_decrypt: bool,
    no_merge: bool,
    query: &HashMap<String, String>,
    streams: Vec<MediaPlaylist>,
    temp_files: &mut Vec<Stream>,
) -> Result<()> {
    let streams = streams
        .into_iter()
        .filter(|x| x.media_type != MediaType::Subtitles)
        .collect::<Vec<_>>();

    for stream in streams {
        info!(
            "Processing {:>5} stream: {}",
            stream.media_type.to_string(),
            stream.display_stream(),
        );

        if stream.segments.is_empty() {
            warn!("Skipping stream (no segments)",);
            continue;
        }

        let temp_file = stream.path(directory, stream.extension());

        temp_files.push(Stream {
            language: stream.language.clone(),
            media_type: stream.media_type.clone(),
            path: temp_file.clone(),
        });

        info!("Downloading {}", temp_file.to_string_lossy());
        download_stream(
            base_url,
            client,
            decrypter.clone(),
            no_decrypt,
            no_merge,
            Progress::new("0", stream.segments.len()),
            query,
            stream,
            &temp_file,
        )
        .await?;
    }

    // eprintln!();
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn download_stream(
    base_url: &Option<Url>,
    client: &Client,
    decrypter: Decrypter,
    no_decrypt: bool,
    no_merge: bool,
    pb: Progress,
    query: &HashMap<String, String>,
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

    let mut default_kid = None;
    let mut widevine_kid = None;
    let mut stream_decrypter = decrypter.clone();

    for (i, segment) in stream.segments.iter().enumerate() {
        if let Some(map) = &segment.map {
            let url = base_url.join(&map.uri)?;
            let mut request = client.get(url).query(query);

            if let Some(range) = &map.range {
                request = request.header(header::RANGE, range.as_header_value());
            }

            let response = request.send().await?;
            let bytes = response.bytes().await?;

            default_kid = vsd_mp4::pssh::default_kid(&bytes)?.or(stream.default_kid());
            widevine_kid = vsd_mp4::pssh::Pssh::new(&bytes)?
                .key_ids
                .into_iter()
                .find_map(|x| match x.system_type {
                    vsd_mp4::pssh::KeyIdSystemType::WideVine => Some(x.value),
                    _ => None,
                });

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
                        let response = request.send().await?;
                        let bytes = response.bytes().await?;

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
                        if let Decrypter::Mp4Decrypt(kid_key_pairs) = &decrypter {
                            // We already checked this before hand so unwraping is safe.
                            if let Some(default_kid) = &default_kid {
                                let key = if default_kid == "00000000000000000000000000000000" {
                                    if widevine_kid.is_none() {
                                        bail!(
                                            "couldn't determine which widevine key to be mapped for this stream's zero kid."
                                        );
                                    }

                                    kid_key_pairs.get(widevine_kid.as_ref().unwrap()).unwrap()
                                } else {
                                    kid_key_pairs.get(default_kid).unwrap()
                                };

                                stream_decrypter = Decrypter::Mp4Decrypt(HashMap::from([(
                                    default_kid.to_owned(),
                                    key.to_owned(),
                                )]));

                                info!("Using key: {}:{}", default_kid, key,);
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
            index: i,
            init_seg: init_seg.clone(),
            merger: merger.clone(),
            pb: pb.clone(),
            request,
        });

        if stream_decrypter.is_none() {
            init_seg = None;
        }
    }

    let mut set = JoinSet::new();
    let max_threads = MAX_THREADS.load(Ordering::SeqCst) as usize;

    for mut thread in threads {
        while set.len() >= max_threads {
            set.join_next().await;
        }
        set.spawn(async move {
            if let Err(e) = thread.execute().await {
                error!("{}", e);
                std::process::exit(1);
            }
        });
    }

    while let Some(_res) = set.join_next().await {}

    let mut merger = merger.lock().unwrap();
    merger.flush()?;

    if !merger.buffered() {
        bail!("failed to download stream.",);
    }

    eprintln!();
    info!("Downloaded stream successfully");
    Ok(())
}

struct Thread {
    decrypter: Decrypter,
    index: usize,
    init_seg: Option<Vec<u8>>,
    merger: Arc<Mutex<Merger>>,
    pb: Progress,
    request: RequestBuilder,
}

impl Thread {
    async fn execute(&mut self) -> Result<()> {
        let mut segment = Vec::new();

        if let Some(init_segment) = &mut self.init_seg {
            segment.append(init_segment);
        }

        let mut data = self.segment().await?;
        let chunk_bytes = data.len();
        // data = fix::fake_png_header(&data);
        segment.append(&mut data);
        segment = self.decrypter.decrypt(segment)?;

        let mut merger = self.merger.lock().unwrap();
        merger.write(self.index, &segment)?;
        merger.flush()?;

        self.pb.update(chunk_bytes);
        Ok(())
    }

    async fn segment(&self) -> Result<Vec<u8>> {
        for _ in 0..MAX_RETRIES.load(Ordering::SeqCst) {
            let response = match self.request.try_clone().unwrap().send().await {
                Ok(response) => response,
                Err(error) => {
                    trace!("{}", check_reqwest_error(&error)?);
                    continue;
                }
            };

            let status = response.status();

            if status.is_client_error() || status.is_server_error() {
                bail!("failed to fetch segments.");
            }

            let data = response.bytes().await?.to_vec();
            return Ok(data);
        }

        bail!("reached max retries to download a segment.");
    }
}

fn check_reqwest_error(error: &reqwest::Error) -> Result<String> {
    let url = error.url().unwrap();

    if error.is_connect() {
        return Ok(format!("{url} (connection error)"));
    } else if error.is_timeout() {
        return Ok(format!("{url} (timeout)"));
    }

    if let Some(status) = error.status() {
        match status {
            StatusCode::GATEWAY_TIMEOUT => Ok(format!("{url} (gateway timeout)")),
            StatusCode::REQUEST_TIMEOUT => Ok(format!("{url} (timeout)")),
            StatusCode::SERVICE_UNAVAILABLE => Ok(format!("{url} (service unavailable)")),
            StatusCode::TOO_MANY_REQUESTS => Ok(format!("{url} (too many requests)")),
            _ => bail!("download failed {} (HTTP {})", url, status),
        }
    } else {
        bail!("download failed {}", url)
    }
}
