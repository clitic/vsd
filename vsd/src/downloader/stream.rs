use crate::{
    downloader::{
        MAX_RETRIES, MAX_THREADS, SKIP_DECRYPT, SKIP_MERGE, encryption::Decrypter, fix, mux::Stream,
    },
    playlist::{KeyMethod, MediaPlaylist, MediaType},
    progress::Progress,
};
use anyhow::{Result, anyhow, bail};
use colored::Colorize;
use log::{debug, error, info, warn};
use reqwest::{Client, RequestBuilder, StatusCode, Url, header};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, atomic::Ordering},
};
use tokio::{
    fs::{self, File},
    io::{self, AsyncWriteExt},
    task::JoinSet,
};
use vsd_mp4::{
    boxes::TencBox,
    decrypt::{CencDecryptingProcessor, HlsAes128Decrypter, HlsSampleAesDecrypter},
    pssh::PsshBox,
};

#[allow(clippy::too_many_arguments)]
pub async fn download_streams(
    base_url: &Option<Url>,
    client: &Client,
    directory: Option<&PathBuf>,
    keys: &HashMap<String, String>,
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
            "Stream [{}] {}",
            stream.media_type.to_string().green(),
            stream.display().cyan(),
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
            keys,
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
    keys: &HashMap<String, String>,
    pb: Progress,
    query: &HashMap<String, String>,
    stream: MediaPlaylist,
    temp_file: &PathBuf,
) -> Result<()> {
    let base_url = base_url
        .clone()
        .unwrap_or(stream.uri.parse::<Url>().unwrap());
    let mut tasks = Vec::with_capacity(stream.segments.len());

    let mut decrypter = Decrypter::None;
    let temp_dir = temp_file.with_extension("");
    let extension = stream.extension();
    let total = stream.segments.len();
    let should_decrypt = !SKIP_DECRYPT.load(Ordering::SeqCst);
    let mut increment_media_sequence = false;
    let mut media_sequence = stream.media_sequence;

    let init_seg = stream.fetch_init_seg(&base_url, client, query).await?;

    let default_kid = if let Some(init_seg) = &init_seg {
        TencBox::from_init(init_seg)?.map(|x| x.default_kid_hex())
    } else {
        stream.default_kid()
    };

    for (i, segment) in stream.segments.iter().enumerate() {
        if should_decrypt {
            if decrypter.is_hls() && segment.key.is_none() && increment_media_sequence {
                decrypter.increment_iv();
                media_sequence += 1;
            }

            if let Some(key) = &segment.key {
                match key.method {
                    KeyMethod::Aes128 | KeyMethod::SampleAes => {
                        match key.method {
                            KeyMethod::Aes128 => {
                                decrypter = Decrypter::Aes128(HlsAes128Decrypter::new(
                                    &key.key(&base_url, client, query).await?,
                                    &key.iv(media_sequence)?,
                                ));
                            }
                            KeyMethod::SampleAes => {
                                decrypter = Decrypter::SampleAes(HlsSampleAesDecrypter::new(
                                    &key.key(&base_url, client, query).await?,
                                    &key.iv(media_sequence)?,
                                ));
                            }
                            _ => (),
                        }

                        if key.iv.is_none() {
                            increment_media_sequence = true;
                            media_sequence += 1;
                        } else {
                            increment_media_sequence = false;
                        }
                    }
                    KeyMethod::Cenc => {
                        if keys.is_empty() {
                            bail!("Custom keys are required to proceed further.");
                        }

                        let default_kid = default_kid.as_ref().ok_or_else(|| {
                            anyhow!("Unable to determine the default KID for this stream.")
                        })?;

                        let mut key = None;

                        if keys.contains_key(default_kid) {
                            key = Some(keys.get(default_kid).unwrap().to_owned())
                        } else {
                            warn!(
                                "No key provided for ({}:?); checking PSSH data to identify other mappable KIDs.",
                                default_kid
                            );

                            if let Some(init_seg) = &init_seg {
                                for kid in PsshBox::from_init(init_seg)?
                                    .data
                                    .into_iter()
                                    .map(|x| x.key_ids)
                                    .flatten()
                                {
                                    if keys.contains_key(&kid.0) {
                                        key = Some(keys.get(&kid.0).unwrap().to_owned());
                                    }
                                }
                            }
                        }

                        let key = key.ok_or_else(|| {
                            anyhow!("Unable to determine the key for this stream.")
                        })?;

                        decrypter = Decrypter::Cenc(Arc::new(
                            CencDecryptingProcessor::builder()
                                .key(default_kid, &key)?
                                .build()?,
                        ));

                        info!("Using key: {}:{}", default_kid, key);
                    }
                    _ => (),
                }
            }
        }

        let url = base_url.join(&segment.uri)?;
        let mut request = client.get(url).query(query);

        if let Some(range) = &segment.range {
            request = request.header(header::RANGE, range);
        }

        tasks.push(Task {
            decrypter: decrypter.clone(),
            init_seg: init_seg.clone(),
            pb: pb.clone(),
            request,
            temp_file: temp_dir.join(format!("{}.{}.part", i, extension)),
        });
    }

    fs::create_dir_all(&temp_dir).await?;

    let mut set = JoinSet::new();
    let max_threads = MAX_THREADS.load(Ordering::SeqCst) as usize;

    for task in tasks {
        while set.len() >= max_threads {
            set.join_next().await;
        }
        set.spawn(async move {
            if let Err(e) = task.execute().await {
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

struct Task {
    decrypter: Decrypter,
    init_seg: Option<Arc<Vec<u8>>>,
    pb: Progress,
    request: RequestBuilder,
    temp_file: PathBuf,
}

impl Task {
    async fn execute(self) -> Result<()> {
        let segment = self.segment().await?;
        let segment_bytes = segment.len();
        let segment = fix::fake_png_header(segment);
        let segment = self
            .decrypter
            .decrypt(segment, self.init_seg.as_ref().map(|x| x.as_ref().to_vec()))?;

        let mut f = File::create(&self.temp_file).await?;

        if let Some(init_seg) = self.init_seg {
            f.write_all(&init_seg).await?;
        }

        f.write_all(&segment).await?;
        f.flush().await?;
        fs::rename(&self.temp_file, self.temp_file.with_extension("")).await?;

        self.pb.update(segment_bytes);
        Ok(())
    }

    async fn segment(&self) -> Result<Vec<u8>> {
        for _ in 0..MAX_RETRIES.load(Ordering::SeqCst) {
            let response = match self.request.try_clone().unwrap().send().await {
                Ok(response) => response,
                Err(error) => {
                    debug!("{}", check(&error)?);
                    continue;
                }
            };

            let status = response.status();
            if status.is_client_error() || status.is_server_error() {
                bail!("{} (http {})", response.url(), status);
            }

            let bytes = response.bytes().await?;
            return Ok(bytes.to_vec());
        }

        bail!("Exceeded the maximum retry limit while downloading a segment.");
    }
}

fn check(error: &reqwest::Error) -> Result<String> {
    let url = error.url().unwrap();

    if error.is_connect() {
        return Ok(format!("{url} (connection error)"));
    } else if error.is_timeout() {
        return Ok(format!("{url} (timeout)"));
    }

    if let Some(status) = error.status() {
        match status {
            StatusCode::GATEWAY_TIMEOUT => Ok(format!("{url} (gateway timeout)")),
            StatusCode::REQUEST_TIMEOUT => Ok(format!("{url} (request timeout)")),
            StatusCode::SERVICE_UNAVAILABLE => Ok(format!("{url} (service unavailable)")),
            StatusCode::TOO_MANY_REQUESTS => Ok(format!("{url} (too many requests)")),
            _ => bail!("{} (http {})", url, status),
        }
    } else {
        bail!("{} (http error)", url)
    }
}
