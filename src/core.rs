use crate::utils;
use crate::{dash, hls};
use crate::{Args, BinaryMerger, Decrypter, Estimater, InputType, Progress, StreamData};
use anyhow::{anyhow, bail, Result};
use kdam::prelude::*;
use reqwest::blocking::Client;
use reqwest::header;
use reqwest::header::HeaderValue;
use std::io::Write;
use std::sync::{Arc, Mutex};

pub struct DownloadState {
    pub args: Args,
    client: Arc<Client>,
    progress: Progress,
}

impl DownloadState {
    pub fn new(args: Args) -> Result<Self> {
        let client = args.client()?;

        if let Some(output) = &args.output {
            if !output.ends_with(".ts") {
                utils::check_ffmpeg("the given output doesn't have .ts file extension")?
            }
        }

        Ok(Self {
            args,
            client,
            progress: Progress::new_empty(),
        })
    }

    fn scrape_website(&mut self) -> Result<()> {
        println!("Scraping website for HLS and Dash links.");
        let links =
            crate::utils::find_hls_dash_links(&self.client.get(&self.args.input).send()?.text()?);

        match links.len() {
            0 => bail!(utils::scrape_website_message(&self.args.input)),
            1 => {
                self.args.input = links[0].clone();
                println!("Found one link {}", &links[0]);
            }
            _ => {
                let mut elinks = vec![];
                for (i, link) in links.iter().enumerate() {
                    elinks.push(format!("{:2}) {}", i + 1, link));
                }
                let index = utils::select(
                    "Select one link:".to_string(),
                    &elinks,
                    self.args.raw_prompts,
                )?;
                self.args.input = links[index].clone();
            }
        }

        Ok(())
    }

    fn fetch_alternative_streams(&mut self, master: &m3u8_rs::MasterPlaylist) -> Result<()> {
        for alternative in &master.alternatives {
            match alternative.media_type {
                m3u8_rs::AlternativeMediaType::Audio => {
                    if alternative.autoselect {
                        if let Some(uri) = &alternative.uri {
                            if self.progress.audio.is_none() {
                                let uri = self.args.get_url(uri)?;
                                self.progress.audio = Some(StreamData::new(
                                    &uri,
                                    &format!(
                                        "{}_audio.ts",
                                        self.progress.video.file.trim_end_matches(".ts")
                                    ),
                                    &self.client.get(&uri).send()?.text()?,
                                )?);
                            }
                        }
                    }
                }
                m3u8_rs::AlternativeMediaType::Subtitles
                | m3u8_rs::AlternativeMediaType::ClosedCaptions => {
                    if alternative.autoselect {
                        if let Some(uri) = &alternative.uri {
                            if self.progress.subtitles.is_none() {
                                self.progress.subtitles =
                                    Some(download_subtitles(&self.args, &self.client, &uri)?);
                            }
                        }
                    }
                }
                _ => (),
            }
        }

        Ok(())
    }

    fn hls_vod(&mut self, content: &[u8]) -> Result<()> {
        match m3u8_rs::parse_playlist_res(content)
            .map_err(|_| anyhow!("Couldn't parse {} playlist.", self.args.input))?
        {
            m3u8_rs::Playlist::MasterPlaylist(master) => {
                self.args.input = if self.args.alternative {
                    self.args
                        .get_url(&hls::alternative(&master, self.args.raw_prompts)?)?
                } else {
                    self.args.get_url(&hls::master(
                        &master,
                        &self.args.quality,
                        self.args.raw_prompts,
                    )?)?
                };

                if !self.args.alternative && !self.args.skip {
                    // self.download_alternative(&master)?;
                }

                let playlist = self.client.get(&self.args.input).send()?.text()?;

                match m3u8_rs::parse_playlist_res(playlist.as_bytes())
                    .map_err(|_| anyhow!("Couldn't parse {} playlist.", self.args.input))?
                {
                    m3u8_rs::Playlist::MediaPlaylist(_) => {
                        self.progress.video =
                            StreamData::new(&self.args.input, &self.args.tempfile(), &playlist)?;
                    }
                    _ => bail!("Media playlist not found."),
                }
            }
            m3u8_rs::Playlist::MediaPlaylist(_) => {
                println!("{} video stream.", "Downloading".colorize("bold green"));
                self.progress.video = StreamData::new(
                    &self.args.input,
                    &self.args.tempfile(),
                    std::str::from_utf8(content)?,
                )?;
            }
        }

        Ok(())
    }

    fn dash_vod(&mut self, content: &[u8]) -> Result<m3u8_rs::MediaPlaylist> {
        let mpd = dash::parse(content)?;
        let master = dash::to_m3u8_as_master(&mpd);

        let uri = if self.args.alternative {
            hls::alternative(&master, self.args.raw_prompts)?
        } else {
            hls::master(&master, &self.args.quality, self.args.raw_prompts)?
        };

        if !self.args.alternative && !self.args.skip {
            // self.download_alternative(&master)?;
        }

        let media = dash::to_m3u8_as_media(&mpd, &self.args.input, &uri).unwrap();

        // println!(
        //     "{} {} stream.",
        //     "Downloading".colorize("bold green"),
        //     if self.args.alternative {
        //         "alternative"
        //     } else {
        //         "video"
        //     }
        // );

        return Ok(media);
    }

    fn hls_live(&mut self) -> Result<()> {
        let live_playlist = hls::LivePlaylist::new(
            &self.args.input,
            self.client.clone(),
            self.args.record_duration,
        );
        let mut file = std::fs::File::create(&self.args.tempfile())?;
        let mut pb = tqdm!(
            // total = total,
            unit = "ts".to_owned(),
            dynamic_ncols = true
        );
        pb.refresh();
        let mut total_bytes = 0;

        for media in live_playlist {
            for seg in media.map_err(|x| anyhow!(x))?.segments {
                let bytes = self
                    .client
                    .get(&self.args.get_url(&seg.uri)?)
                    .send()?
                    .bytes()?
                    .to_vec();
                total_bytes += bytes.len();
                file.write_all(&bytes)?;
                pb.set_description(utils::format_bytes(total_bytes, 2).2);
                pb.update(1);
            }
        }

        Ok(())
    }

    pub fn playlist(&mut self) -> Result<()> {
        if self.args.input_type().is_website() {
            self.scrape_website()?;
        }

        let input_type = self.args.input_type();

        let content = match input_type {
            InputType::HlsUrl | InputType::DashUrl => {
                self.client.get(&self.args.input).send()?.bytes()?.to_vec()
            }
            InputType::HlsLocalFile | InputType::DashLocalFile => {
                std::fs::read_to_string(&self.args.input)?
                    .as_bytes()
                    .to_vec()
            }
            InputType::LocalFile | InputType::Website => {
                bail!("Unsupported input file {}.", self.args.input)
            }
        };

        if input_type.is_hls() {
            self.hls_vod(&content)?;
        } else if input_type.is_dash() {
        } else {
            bail!("Only HLS and DASH streams are supported.")
        }

        Ok(())
    }

    fn check_segments(&self) -> Result<()> {
        let mut segments = self.progress.video.to_playlist().segments;

        if let Some(audio) = &self.progress.audio {
            segments.extend_from_slice(&audio.to_playlist().segments);
        }

        self.args.get_url(&segments[0].uri)?;

        for segment in segments {
            match &segment.key {
                Some(m3u8_rs::Key {
                    method: m3u8_rs::KeyMethod::SampleAES,
                    ..
                }) => bail!("SAMPLE-AES encrypted playlists are not supported."),
                Some(m3u8_rs::Key {
                    method: m3u8_rs::KeyMethod::Other(x),
                    ..
                }) => {
                    if x != "CENC" {
                        bail!("{} encrypted playlists are not supported.", x)
                    }
                }
                _ => (),
            }
        }

        Ok(())
    }

    pub fn download(&mut self) -> Result<()> {
        // download_subtitles()
        self.check_segments()?;

        let size =
            self.progress.video.total + self.progress.audio.clone().map(|x| x.total).unwrap_or(0);
        let pool = threadpool::ThreadPool::new(self.args.threads as usize);
        let pb = Arc::new(Mutex::new(tqdm!(
            total = size,
            unit = "ts".to_owned(),
            dynamic_ncols = true
        )));

        self.progress
            .json_file(&utils::replace_ext(&self.progress.video.file, "json"));
        let tempfile = self.progress.video.file.clone();
        let segments = self.progress.video.to_playlist().segments;

        let merger = if self.args.resume {
            Arc::new(Mutex::new(BinaryMerger::try_from_json(
                self.progress.video.total,
                tempfile.clone(),
                self.progress.json_file.clone(),
            )?))
        } else {
            Arc::new(Mutex::new(BinaryMerger::new(
                self.progress.video.total,
                tempfile.clone(),
                self.progress.clone(),
            )?))
        };
        merger.lock().unwrap().update()?;

        println!(
            "{} {} stream to {}.",
            "Downloading".colorize("bold green"),
            if self.args.alternative {
                "alternative"
            } else {
                "video"
            },
            tempfile.colorize("cyan")
        );

        for (i, segment) in segments.iter().enumerate() {
            if self.args.resume {
                let merger = merger.lock().unwrap();
                let pos = merger.position();

                if pos != 0 && pos > i {
                    continue;
                }

                let mut pb = pb.lock().unwrap();
                pb.set_description(format!(
                    "{} / {}",
                    utils::format_bytes(merger.stored(), 2).2,
                    utils::format_bytes(merger.estimate(), 0).2
                ));
                pb.update_to(pos);
            }

            let client = self.client.clone();
            let merger = merger.clone();
            let size = size.clone();
            let pb = pb.clone();
            let segment_url = self.args.get_url(&segment.uri)?;
            let byte_range = segment.byte_range.clone();
            let total_retries = self.args.retry_count;
            let key_info = match &segment.key {
                Some(m3u8_rs::Key {
                    uri: Some(link), ..
                }) => Some((self.args.get_url(link)?, segment.key.clone().unwrap())),
                _ => None,
            };

            pool.execute(move || {
                download_segments(
                    client,
                    merger.clone(),
                    pb.clone(),
                    i,
                    segment_url,
                    byte_range,
                    total_retries,
                    key_info,
                )
                .unwrap();

                let merger = merger.lock().unwrap();
                let mut pb = pb.lock().unwrap();
                pb.set_description(format!(
                    "{} / {}",
                    utils::format_bytes(merger.stored(), 2).2,
                    utils::format_bytes(merger.relative_estimate(size), 0).2
                ));
                pb.update(1);
            });
        }

        pool.join();
        let mut gaurded_merger = merger.lock().unwrap();
        gaurded_merger.flush()?;

        if gaurded_merger.buffered() {
            pb.lock().unwrap().write(format!(
                "File {} downloaded successfully.",
                tempfile.colorize("bold green")
            ));
        } else {
            bail!(
                "File {} not downloaded successfully.",
                tempfile.colorize("bold red")
            );
        }

        if let Some(audio) = &self.progress.audio {
            let stored_bytes = gaurded_merger.stored();
            let tempfile = audio.file.clone();
            let playlist = audio.to_playlist();
            let segments = playlist.segments;
            gaurded_merger.reset(audio.total, tempfile.clone())?;

            println!(
                "{} audio stream to {}.",
                "Downloading".colorize("bold green"),
                tempfile.colorize("cyan")
            );

            for (i, segment) in segments.iter().enumerate() {
                if self.args.resume {
                    let merger = merger.lock().unwrap();
                    let pos = merger.position();

                    if pos != 0 && pos > i {
                        continue;
                    }

                    let mut pb = pb.lock().unwrap();
                    pb.set_description(format!(
                        "{} / {}",
                        utils::format_bytes(merger.stored(), 2).2,
                        utils::format_bytes(merger.estimate(), 0).2
                    ));
                    pb.update_to(pos);
                }

                let client = self.client.clone();
                let merger = merger.clone();
                let pb = pb.clone();
                let segment_url = self.args.get_url(&segment.uri)?;
                let byte_range = segment.byte_range.clone();
                let total_retries = self.args.retry_count;
                let key_info = match &segment.key {
                    Some(m3u8_rs::Key {
                        uri: Some(link), ..
                    }) => Some((self.args.get_url(link)?, segment.key.clone().unwrap())),
                    _ => None,
                };

                let stored_bytes = stored_bytes.clone();

                pool.execute(move || {
                    download_segments(
                        client,
                        merger.clone(),
                        pb.clone(),
                        i,
                        segment_url,
                        byte_range,
                        total_retries,
                        key_info,
                    )
                    .unwrap();

                    let merger = merger.lock().unwrap();
                    let mut pb = pb.lock().unwrap();
                    pb.set_description(format!(
                        "{} / {}",
                        utils::format_bytes(merger.stored() + stored_bytes, 2).2,
                        utils::format_bytes(merger.estimate() + stored_bytes, 0).2
                    ));
                    pb.update(1);
                });
            }

            pool.join();
            let mut gaurded_merger = merger.lock().unwrap();
            gaurded_merger.flush()?;
            let mut pb = pb.lock().unwrap();

            if gaurded_merger.buffered() {
                pb.write(format!(
                    "File {} downloaded successfully.",
                    tempfile.colorize("bold green")
                ));
            } else {
                bail!(
                    "File {} not downloaded successfully.",
                    tempfile.colorize("bold red")
                );
            }
        }

        Ok(())
    }

    pub fn transmux_trancode(&mut self) -> Result<()> {
        if let Some(output) = &self.args.output {
            if output.ends_with(".ts") {
                return Ok(());
            }

            let mut args = vec!["-i", &self.progress.video.file];

            if let Some(audio) = &self.progress.audio {
                args.push("-i");
                args.push(&audio.file);
            }

            // if let Some(subtitle) = &self.progress.subtitles {
            //     args.push("-i");
            //     args.push(&subtitle.file);
            // }

            if std::path::Path::new(output).exists() {
                std::fs::remove_file(output)?;
            }

            if !(output.ends_with(".srt") || output.ends_with(".vtt")) {
                args.push("-c");
                args.push("copy");
            }

            args.push(output);

            println!(
                "Executing {} {}",
                "ffmpeg".colorize("cyan"),
                args.join(" ").colorize("cyan")
            );

            let code = std::process::Command::new("ffmpeg")
                .args(args)
                .stderr(std::process::Stdio::null())
                .spawn()?
                .wait()?;

            if !code.success() {
                bail!("FFMPEG exited with code {}.", code.code().unwrap_or(1))
            }

            if let Some(audio) = &self.progress.audio {
                std::fs::remove_file(&audio.file)?;
            }

            // if let Some(subtitle) = &self.progress.subtitles {
            //     std::fs::remove_file(&subtitle.file)?;
            // }

            std::fs::remove_file(&self.progress.video.file)?;
        }

        if std::path::Path::new(&self.progress.json_file).exists() {
            std::fs::remove_file(&self.progress.json_file)?;
        }
        Ok(())
    }
}

fn download_segments(
    client: Arc<Client>,
    merger: Arc<Mutex<BinaryMerger>>,
    pb: Arc<Mutex<kdam::Bar>>,
    segment_index: usize,
    segment_url: String,
    byte_range: Option<m3u8_rs::ByteRange>,
    total_retries: u8,
    key_info: Option<(String, m3u8_rs::Key)>,
) -> Result<()> {
    let fetch_segment = || -> Result<Vec<u8>, reqwest::Error> {
        match byte_range {
            Some(m3u8_rs::ByteRange {
                length: start,
                offset: Some(end),
            }) => Ok(client
                .get(&segment_url)
                .header(
                    header::RANGE,
                    format!("bytes={}-{}", start, start + end - 1),
                )
                .send()?
                .bytes()?
                .to_vec()),
            _ => Ok(client.get(&segment_url).send()?.bytes()?.to_vec()),
        }
    };

    let mut retries = 0;
    let mut data = loop {
        match fetch_segment() {
            Ok(bytes) => break bytes,
            Err(e) => {
                if total_retries > retries {
                    pb.lock().unwrap().write(utils::check_reqwest_error(&e)?);
                    retries += 1;
                    continue;
                } else {
                    bail!(
                        "Reached maximum number of retries for segment at index {}.",
                        segment_index
                    )
                }
            }
        }
    };

    // Decrypt
    let fetch_key = |key_url| -> Result<Vec<u8>, reqwest::Error> {
        Ok(client.get(key_url).send()?.bytes()?.to_vec())
    };

    retries = 0;
    if let Some((key_url, key)) = &key_info {
        let key_content = loop {
            match fetch_key(key_url) {
                Ok(bytes) => break bytes,
                Err(e) => {
                    if total_retries > retries {
                        pb.lock().unwrap().write(utils::check_reqwest_error(&e)?);
                        retries += 1;
                        continue;
                    } else {
                        bail!("Reached maximum number of retries to download decryption key.")
                    }
                }
            }
        };

        data = Decrypter::from_key(key, &key_content)
            .unwrap()
            .decrypt(&data, None)?;
    }

    let mut merger = merger.lock().unwrap();
    merger.write(segment_index, &data)?;
    merger.flush()?;
    Ok(())
}

fn download_subtitles(args: &Args, client: &Arc<Client>, uri: &str) -> Result<String> {
    println!("{} subtitles stream.", "Downloading".colorize("bold green"));

    let uri = args.get_url(uri)?;
    let mut subtitles = vec![];

    for segment in m3u8_rs::parse_media_playlist_res(&client.get(&uri).send()?.bytes()?.to_vec())
        .map_err(|_| anyhow!("Couldn't parse {} as media playlist.", uri))?
        .segments
    {
        subtitles.extend_from_slice(
            &client
                .get(&args.get_url(&segment.uri)?)
                .send()?
                .bytes()?
                .to_vec(),
        );
    }

    Ok(String::from_utf8(subtitles)?)
}
