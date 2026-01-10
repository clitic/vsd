use crate::{
    downloader::{MAX_RETRIES, MAX_THREADS, SKIP_MERGE, encryption::Decrypter, fix, mux::Stream},
    playlist::{KeyMethod, MediaPlaylist, MediaType},
    progress::Progress,
};
use anyhow::{Result, bail};
use colored::Colorize;
use log::{debug, error, info, warn};
use reqwest::{Client, RequestBuilder, StatusCode, Url, header};
use std::{collections::HashMap, path::PathBuf, sync::atomic::Ordering};
use tokio::{
    fs::{self, File},
    io::{self, AsyncWriteExt},
    task::JoinSet,
};

#[allow(clippy::too_many_arguments)]
pub async fn download_streams(
    base_url: &Option<Url>,
    client: &Client,
    decrypter: Decrypter,
    directory: Option<&PathBuf>,
    no_decrypt: bool,
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
            stream.display_stream().bold(),
        );

        if stream.segments.is_empty() {
            warn!("Skipping stream (no segments)",);
            continue;
        }

        let temp_file = stream.path(directory);

        temp_files.push(Stream {
            language: stream.language.clone(),
            media_type: stream.media_type.clone(),
            path: temp_file.clone(),
        });

        info!(
            "Downloading segments: {}",
            temp_file.with_extension("").to_string_lossy()
        );
        download_stream(
            base_url,
            client,
            decrypter.clone(),
            no_decrypt,
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
    pb: Progress,
    query: &HashMap<String, String>,
    stream: MediaPlaylist,
    temp_file: &PathBuf,
) -> Result<()> {
    let mut init_seg = None;
    let mut increment_iv = false;
    let base_url = base_url
        .clone()
        .unwrap_or(stream.uri.parse::<Url>().unwrap());
    let mut threads = Vec::with_capacity(stream.segments.len());

    let mut default_kid = None;
    let mut widevine_kid = None;
    let mut stream_decrypter = decrypter.clone();
    let temp_dir = temp_file.with_extension("");
    let extension = stream.extension();
    let total = stream.segments.len();

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

                                info!("Using key: {}:{}", default_kid, key);
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
            init_seg: init_seg.clone(),
            pb: pb.clone(),
            request,
            temp_file: temp_dir.join(format!("{}.{}.part", i, extension)),
        });

        if stream_decrypter.is_none() {
            init_seg = None;
        }
    }

    fs::create_dir_all(&temp_dir).await?;

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

    eprintln!();

    if SKIP_MERGE.load(Ordering::SeqCst) {
        info!("Merging skipped {}", temp_file.to_string_lossy());
    } else {
        info!("Merging segments {}", temp_file.to_string_lossy());

        let mut outfile = File::create(temp_file).await?;

        for i in 0..total {
            let path = temp_dir.join(format!("{}.{}", i, extension));

            if path.exists() {
                io::copy(&mut File::open(&path).await?, &mut outfile).await?;
            }
        }

        info!("Deleting {}", temp_dir.to_string_lossy());
        fs::remove_dir_all(&temp_dir).await?;
    }

    info!("Downloaded stream successfully");
    Ok(())
}

struct Thread {
    decrypter: Decrypter,
    init_seg: Option<Vec<u8>>,
    pb: Progress,
    request: RequestBuilder,
    temp_file: PathBuf,
}

impl Thread {
    async fn execute(&mut self) -> Result<()> {
        let mut segment = Vec::new();

        if let Some(init_segment) = &mut self.init_seg {
            segment.append(init_segment);
        }

        let data = self.segment().await?;
        let chunk_bytes = data.len();

        segment.extend_from_slice(fix::fake_png_header(&data));
        segment = self.decrypter.decrypt(segment)?;

        let mut file = File::create(&self.temp_file).await?;
        file.write_all(&segment).await?;
        file.flush().await?;
        fs::rename(&self.temp_file, self.temp_file.with_extension("")).await?;

        self.pb.update(chunk_bytes);
        Ok(())
    }

    async fn segment(&self) -> Result<Vec<u8>> {
        for _ in 0..MAX_RETRIES.load(Ordering::SeqCst) {
            let response = match self.request.try_clone().unwrap().send().await {
                Ok(response) => response,
                Err(error) => {
                    debug!("{}", check_reqwest_error(&error)?);
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
