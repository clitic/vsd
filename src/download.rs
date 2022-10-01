use crate::commands::InputType;
use crate::decrypter::Decrypter;
use crate::merger::BinaryMerger;
use crate::progress::{Progress, StreamData};
use crate::subtitles::MP4Subtitles;
use crate::{commands, dash, hls, utils};
use anyhow::{anyhow, bail, Result};
use kdam::prelude::*;
use kdam::{Column, RichProgress};
use reqwest::blocking::Client;
use reqwest::header;
use std::io::Write;
use std::sync::{Arc, Mutex};

pub struct DownloadState {
    pub args: commands::Save,
    pub client: Arc<Client>,
    pub progress: Progress,
    pub pb: Arc<Mutex<RichProgress>>,
    pub alternative_media_type: Option<m3u8_rs::AlternativeMediaType>,
}

impl DownloadState {
    pub fn new(args: commands::Save) -> Result<Self> {
        let client = args.client()?;

        if let Some(output) = &args.output {
            if !output.ends_with(".ts") {
                utils::check_ffmpeg("the given output doesn't have .ts file extension")?
            }
        }

        Ok(Self {
            alternative_media_type: None,
            args,
            client,
            progress: Progress::new_empty(),
            pb: Arc::new(Mutex::new(RichProgress::new(
                tqdm!(unit = " SEG".to_owned(), dynamic_ncols = true),
                vec![
                    Column::Spinner(
                        "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"
                            .chars()
                            .map(|x| x.to_string())
                            .collect::<Vec<String>>(),
                        80.0,
                        1.0,
                    ),
                    Column::text("[bold blue]?"),
                    Column::Bar,
                    Column::Percentage(0),
                    Column::text("•"),
                    Column::CountTotal,
                    Column::text("•"),
                    Column::ElapsedTime,
                    Column::text("[cyan]>"),
                    Column::RemainingTime,
                    Column::text("•"),
                    Column::Rate,
                ],
            ))),
        })
    }

    fn scrape_website(&mut self) -> Result<()> {
        println!(
            "{} website for HLS and DASH stream links.",
            "Scraping".colorize("bold green"),
        );
        let links =
            crate::utils::find_hls_dash_links(&self.client.get(&self.args.input).send()?.text()?);

        match links.len() {
            0 => bail!(utils::scrape_website_message(&self.args.input)),
            1 => {
                self.args.input = links[0].clone();
                println!("{} {}", "Found".colorize("bold green"), &links[0]);
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

    pub fn fetch_playlists(&mut self) -> Result<()> {
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
            self.dash_vod(&content)?
        } else {
            bail!("Only HLS and DASH streams are supported.")
        }

        Ok(())
    }

    fn fetch_alternative_streams(
        &mut self,
        master: &m3u8_rs::MasterPlaylist,
        mpd: Option<dash::MPD>,
    ) -> Result<()> {
        let fetch_playlist = |uri: &str| -> Result<(String, String, Option<&str>)> {
            let uri = if uri.starts_with("dash://") {
                uri.to_owned()
            } else {
                self.args.get_url(uri)?
            };

            let playlist = if let Some(mpd) = &mpd {
                let mut playlist = vec![];
                dash::to_m3u8_as_media(&mpd, &self.args.input, &uri)
                    .unwrap()
                    .write_to(&mut playlist)?;

                String::from_utf8(playlist)?
            } else {
                self.client.get(&uri).send()?.text()?
            };

            Ok((
                uri,
                playlist,
                if mpd.is_some() {
                    Some("m4s")
                } else {
                    Some("ts")
                },
            ))
        };

        for alternative in &master.alternatives {
            match alternative.media_type {
                m3u8_rs::AlternativeMediaType::Audio => {
                    if alternative.autoselect {
                        if let Some(uri) = &alternative.uri {
                            if self.progress.audio.is_none() {
                                let (uri, playlist, ext) = fetch_playlist(uri)?;
                                self.progress.audio = Some(StreamData::new(
                                    &uri,
                                    alternative.language.clone(),
                                    &self.progress.video.filename("audio", ext),
                                    &playlist,
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
                                let (uri, playlist, _) = fetch_playlist(uri)?;
                                self.progress.subtitles = Some(StreamData::new(
                                    &uri,
                                    alternative.language.clone(),
                                    &self.progress.video.filename("subtitles", Some("txt")),
                                    &playlist,
                                )?);
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
                    let alternative = hls::alternative(&master, self.args.raw_prompts)?;
                    self.alternative_media_type = Some(alternative.media_type);
                    self.args.get_url(&alternative.uri.unwrap())?
                } else {
                    self.args.get_url(&hls::master(
                        &master,
                        &self.args.quality,
                        self.args.raw_prompts,
                    )?)?
                };

                let playlist = self.client.get(&self.args.input).send()?.text()?;

                match m3u8_rs::parse_playlist_res(playlist.as_bytes())
                    .map_err(|_| anyhow!("Couldn't parse {} playlist.", self.args.input))?
                {
                    m3u8_rs::Playlist::MediaPlaylist(_) => {
                        self.progress.video = StreamData::new(
                            &self.args.input,
                            None,
                            &self.args.tempfile(),
                            &playlist,
                        )?;

                        if !self.args.alternative && !self.args.skip {
                            self.fetch_alternative_streams(&master, None)?;
                        }
                    }
                    _ => bail!("Media playlist not found."),
                }
            }
            m3u8_rs::Playlist::MediaPlaylist(_) => {
                if self.args.alternative {
                    bail!("Alternative streams can only be downloaded from master playlist.")
                }

                self.progress.video = StreamData::new(
                    &self.args.input,
                    None,
                    &self.args.tempfile(),
                    std::str::from_utf8(content)?,
                )?;
            }
        }

        Ok(())
    }

    fn dash_vod(&mut self, content: &[u8]) -> Result<()> {
        let mpd = dash::parse(content)?;
        let mut master = dash::to_m3u8_as_master(&mpd);
        hls::autoselect(&mut master, None, None);

        let uri = if self.args.alternative {
            let alternative = hls::alternative(&master, self.args.raw_prompts)?;
            self.alternative_media_type = Some(alternative.media_type);
            alternative.uri.unwrap()
        } else {
            hls::master(&master, &self.args.quality, self.args.raw_prompts)?
        };

        let mut playlist = vec![];
        dash::to_m3u8_as_media(&mpd, &self.args.input, &uri)
            .unwrap()
            .write_to(&mut playlist)?;

        self.progress.video = StreamData::new(
            &self.args.input,
            None,
            &self.args.tempfile(),
            &String::from_utf8(playlist)?,
        )?;

        if !self.args.alternative && !self.args.skip {
            self.fetch_alternative_streams(&master, Some(mpd))?;
        }

        Ok(())
    }

    // fn hls_live(&mut self) -> Result<()> {
    //     let live_playlist = hls::LivePlaylist::new(
    //         &self.args.input,
    //         self.client.clone(),
    //         self.args.record_duration,
    //     );
    //     let mut file = std::fs::File::create(&self.args.tempfile())?;
    //     let mut pb = tqdm!(
    //         // total = total,
    //         unit = "ts".to_owned(),
    //         dynamic_ncols = true
    //     );
    //     pb.refresh();
    //     let mut total_bytes = 0;

    //     for media in live_playlist {
    //         for seg in media.map_err(|x| anyhow!(x))?.segments {
    //             let bytes = self
    //                 .client
    //                 .get(&self.args.get_url(&seg.uri)?)
    //                 .send()?
    //                 .bytes()?
    //                 .to_vec();
    //             total_bytes += bytes.len();
    //             file.write_all(&bytes)?;
    //             pb.set_description(utils::format_bytes(total_bytes, 2).2);
    //             pb.update(1);
    //         }
    //     }

    //     Ok(())
    // }

    fn check_segments(&self) -> Result<()> {
        let mut segments = self.progress.video.to_playlist().segments;

        if let Some(audio) = &self.progress.audio {
            segments.extend_from_slice(&audio.to_playlist().segments);
        }

        self.args.get_url(&segments[0].uri)?;

        for segment in segments {
            let segment_tags = dash::SegmentTag::from(&segment.unknown_tags);
            if segment_tags.single {
                bail!("single file dash streams are not supported")
            }

            match &segment.key {
                Some(m3u8_rs::Key {
                    method: m3u8_rs::KeyMethod::SampleAES,
                    ..
                }) => bail!("SAMPLE-AES encrypted playlists are not supported."),
                Some(m3u8_rs::Key {
                    method: m3u8_rs::KeyMethod::Other(x),
                    ..
                }) => {
                    if x == "CENC" && self.args.key.is_empty() {
                        bail!("CENC encrypted playlists requires --key")
                    } else {
                        bail!("{} encrypted playlists are not supported.", x)
                    }
                }
                _ => (),
            }
        }

        Ok(())
    }

    fn download_subtitles(&mut self, subtitles: StreamData) -> Result<StreamData> {
        let mut subtitles = subtitles;
        let mut gaurded_pb = self.pb.lock().unwrap();

        let playlist = subtitles.to_playlist();
        let segments = playlist.segments;
        gaurded_pb.pb.set_total(segments.len());

        let mut total_bytes = 0;

        let mut subtitles_data = self
            .client
            .get(&self.args.get_url(&segments[0].uri)?)
            .send()?
            .bytes()?
            .to_vec();

        total_bytes += subtitles_data.len();

        gaurded_pb.replace(
            1,
            Column::Text(format!(
                "[bold blue]{}",
                utils::format_bytes(total_bytes, 2).2
            )),
        );
        gaurded_pb.update(1);

        let mut mp4subtitles = false;

        if &subtitles_data[..6] == "WEBVTT".as_bytes() {
            subtitles.set_extension_mut("vtt");
        } else if subtitles_data[0] == "1".as_bytes()[0] {
            subtitles.set_extension_mut("srt");
        } else if &subtitles_data[..3] == "<tt".as_bytes() {
            bail!("raw ttml subtitles are not supported")
        } else {
            let playlist_tags = dash::PlaylistTag::from(&playlist.unknown_tags);
            let segment_tags = dash::SegmentTag::from(&segments[0].unknown_tags);
            let uri = segments[0].uri.split('?').next().unwrap();

            if segment_tags.init {
                if playlist_tags.vtt
                    || playlist_tags.ttml
                    || uri.ends_with(".mp4")
                    || uri.ends_with(".cmft")
                    || uri.ends_with(".ismt")
                {
                    subtitles.set_extension_mut("vtt");
                    mp4subtitles = true;
                } else {
                    bail!("unknown embedded subtitles are not supported")
                }
            }
        }

        gaurded_pb.write(format!(
            "{} subtitle stream to {}",
            "Downloading".colorize("bold green"),
            subtitles.file.colorize("cyan")
        ));

        for segment in &segments[1..] {
            let bytes = self
                .client
                .get(&self.args.get_url(&segment.uri)?)
                .send()?
                .bytes()?
                .to_vec();

            total_bytes += bytes.len();
            subtitles_data.extend_from_slice(&bytes);

            gaurded_pb.replace(
                1,
                Column::Text(format!(
                    "[bold blue]{}",
                    utils::format_bytes(total_bytes, 2).2
                )),
            );
            gaurded_pb.update(1);
        }

        if mp4subtitles {
            gaurded_pb.write(format!(
                " {} embedded subtitles",
                "Extracting".colorize("bold cyan"),
            ));

            let split_data = mp4decrypt::mp4split(&subtitles_data).map_err(|x| anyhow!(x))?;

            subtitles_data = MP4Subtitles::new(&split_data[0], None)
                .map_err(|x| anyhow!(x))?
                .add_cues(&split_data[1..])
                .map_err(|x| anyhow!(x))?
                .to_subtitles()
                .to_vtt()
                .as_bytes()
                .to_vec();
        }

        std::fs::File::create(&subtitles.file)?.write_all(&subtitles_data)?;
        subtitles.downloaded = gaurded_pb.pb.get_total();
        gaurded_pb.write(format!(
            " {} subtitle stream successfully",
            "Downloaded".colorize("bold green"),
        ));

        Ok(subtitles.to_owned())
    }

    pub fn download(&mut self) -> Result<()> {
        self.progress.set_progress_file();

        if let Some(m3u8_rs::AlternativeMediaType::Subtitles)
        | Some(m3u8_rs::AlternativeMediaType::ClosedCaptions) = self.alternative_media_type
        {
            self.progress.video = self.download_subtitles(self.progress.video.to_owned())?;
            return Ok(());
        }

        if let Some(subtitles) = &self.progress.subtitles {
            self.progress.subtitles = Some(self.download_subtitles(subtitles.to_owned())?);
        }

        self.check_segments()?;
        let mut gaurded_pb = self.pb.lock().unwrap();
        gaurded_pb.pb.reset(Some(
            self.progress.video.total + self.progress.audio.clone().map(|x| x.total).unwrap_or(0),
        ));
        gaurded_pb.replace(3, Column::Percentage(2));
        gaurded_pb
            .columns
            .extend_from_slice(&[Column::text("•"), Column::text("[yellow]?")]);

        let mut stored_bytes = 0;
        let relative_size = if let Some(audio) = &self.progress.audio {
            let segments = audio.to_playlist().segments;

            Some(
                segments.len()
                    * self
                        .client
                        .get(&self.args.get_url(&segments[1].uri)?)
                        .send()?
                        .content_length()
                        .unwrap_or(0) as usize,
            )
        } else {
            None
        };

        self.progress.video.set_suffix(if self.args.alternative {
            "audio-alternative"
        } else {
            "video"
        });

        gaurded_pb.write(format!(
            "{} {} stream to {}",
            "Downloading".colorize("bold green"),
            if self.args.alternative {
                "audio/alternative"
            } else {
                "video"
            },
            self.progress.video.file.colorize("cyan")
        ));

        drop(gaurded_pb);

        let pool = threadpool::ThreadPool::new(self.args.threads as usize);

        stored_bytes = self.download_segments_in_threads(
            self.progress.video.to_playlist().segments,
            &self.progress.video.file,
            &pool,
            stored_bytes,
            relative_size,
        )?;

        self.pb.lock().unwrap().write(format!(
            " {} {} stream successfully",
            "Downloaded".colorize("bold green"),
            if self.args.alternative {
                "audio/alternative"
            } else {
                "video"
            }
        ));

        if let Some(audio) = &self.progress.audio {
            self.pb.lock().unwrap().write(format!(
                "{} audio stream to {}",
                "Downloading".colorize("bold green"),
                audio.file.colorize("cyan")
            ));

            let _ = self.download_segments_in_threads(
                audio.to_playlist().segments,
                &audio.file,
                &pool,
                stored_bytes,
                None,
            )?;

            self.pb.lock().unwrap().write(format!(
                " {} audio stream successfully",
                "Downloaded".colorize("bold green"),
            ));
        }

        println!();
        Ok(())
    }

    fn download_segments_in_threads(
        &self,
        segments: Vec<m3u8_rs::MediaSegment>,
        tempfile: &str,
        pool: &threadpool::ThreadPool,
        stored_bytes: usize,
        relative_size: Option<usize>,
    ) -> Result<usize> {
        let merger = if self.args.resume {
            Arc::new(Mutex::new(BinaryMerger::try_from_json(
                segments.len(),
                tempfile,
                self.progress.file.clone(),
            )?))
        } else {
            Arc::new(Mutex::new(BinaryMerger::new(
                segments.len(),
                tempfile,
                self.progress.clone(),
            )?))
        };
        merger.lock().unwrap().update()?;

        let timer = Arc::new(std::time::Instant::now());

        for (i, segment) in segments.iter().enumerate() {
            if self.args.resume {
                let merger = merger.lock().unwrap();
                let pos = merger.position();

                if pos != 0 && pos > i {
                    continue;
                }

                let mut pb = self.pb.lock().unwrap();
                pb.replace(
                    1,
                    Column::Text(format!(
                        "[bold blue]{} / {}",
                        utils::format_bytes(stored_bytes + merger.stored(), 2).2,
                        if let Some(size) = relative_size {
                            utils::format_bytes(stored_bytes + size + merger.estimate(), 0).2
                        } else {
                            utils::format_bytes(stored_bytes + merger.estimate(), 0).2
                        }
                    )),
                );
                pb.update_to(pos);
            }

            let client = self.client.clone();
            let merger = merger.clone();
            let stored_bytes = stored_bytes.clone();
            let relative_size = relative_size.clone();
            let pb = self.pb.clone();
            let segment_url = self.args.get_url(&segment.uri)?;
            let byte_range = segment.byte_range.clone();
            let total_retries = self.args.retry_count;
            let key_info = match &segment.key {
                Some(m3u8_rs::Key {
                    uri: Some(link), ..
                }) => Some((self.args.get_url(link)?, segment.key.clone().unwrap())),
                _ => None,
            };
            let timer = timer.clone();

            pool.execute(move || {
                if let Err(e) = download_segments(
                    client,
                    merger.clone(),
                    pb.clone(),
                    i,
                    segment_url,
                    byte_range,
                    total_retries,
                    key_info,
                    stored_bytes,
                    relative_size,
                    timer,
                ) {
                    let _ = pb.lock().unwrap();
                    println!("\n{}: {}", "error".colorize("bold red"), e);
                    std::process::exit(1);
                }
            });
        }

        pool.join();
        let mut merger = merger.lock().unwrap();
        merger.flush()?;

        if !merger.buffered() {
            bail!(
                "File {} not downloaded successfully.",
                tempfile.colorize("bold red")
            );
        }

        Ok(merger.stored())
    }
}

fn download_segments(
    client: Arc<Client>,
    merger: Arc<Mutex<BinaryMerger>>,
    pb: Arc<Mutex<RichProgress>>,
    segment_index: usize,
    segment_url: String,
    byte_range: Option<m3u8_rs::ByteRange>,
    total_retries: u8,
    key_info: Option<(String, m3u8_rs::Key)>,
    stored_bytes: usize,
    relative_size: Option<usize>,
    timer: Arc<std::time::Instant>,
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
            Ok(bytes) => {
                let elapsed_time = timer.elapsed().as_secs() as usize;
                if elapsed_time != 0 {
                    let stored = merger.lock().unwrap().stored() + bytes.len();
                    pb.lock().unwrap().replace(
                        13,
                        Column::Text(format!(
                            "[yellow]{}/s",
                            utils::format_bytes(stored / elapsed_time, 2).2
                        )),
                    );
                }

                break bytes;
            }
            Err(e) => {
                if total_retries > retries {
                    pb.lock()
                        .unwrap()
                        .write(utils::check_reqwest_error(&e, &segment_url)?);
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
                        pb.lock()
                            .unwrap()
                            .write(utils::check_reqwest_error(&e, key_url)?);
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

    let mut guarded_merger = merger.lock().unwrap();
    guarded_merger.write(segment_index, &data)?;
    guarded_merger.flush()?;
    let stored = guarded_merger.stored();
    let estimate = guarded_merger.estimate();

    let mut gaurded_pb = pb.lock().unwrap();
    gaurded_pb.replace(
        1,
        Column::Text(format!(
            "[bold blue]{} / {}",
            utils::format_bytes(stored_bytes + stored, 2).2,
            if let Some(size) = relative_size {
                utils::format_bytes(stored_bytes + size + estimate, 0).2
            } else {
                utils::format_bytes(stored_bytes + estimate, 0).2
            },
        )),
    );

    gaurded_pb.update(1);

    Ok(())
}
