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
    pub alternative_media_type: Option<m3u8_rs::AlternativeMediaType>,
    pub args: commands::Save,
    pub client: Arc<Client>,
    pub progress: Progress,
}

impl DownloadState {
    pub fn perform(&mut self) -> Result<()> {
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

        self.download()?;
        self.progress
            .mux(&self.args.output, &self.alternative_media_type)?;
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
                dash::to_m3u8_as_media(mpd, &self.args.input, &uri)
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

    pub fn download(&mut self) -> Result<()> {
        self.progress.set_progress_file();

        let mut pb = RichProgress::new(
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
        );

        if matches!(
            self.alternative_media_type,
            Some(m3u8_rs::AlternativeMediaType::Subtitles)
                | Some(m3u8_rs::AlternativeMediaType::ClosedCaptions)
        ) {
            self.progress.video =
                self.download_subtitles(self.progress.video.to_owned(), &mut pb)?;
            return Ok(());
        }

        if let Some(subtitles) = &self.progress.subtitles {
            self.progress.subtitles = Some(self.download_subtitles(subtitles.to_owned(), &mut pb)?);
        }

        self.check_segments()?;
        pb.pb.reset(Some(
            self.progress.video.total + self.progress.audio.clone().map(|x| x.total).unwrap_or(0),
        ));
        pb.replace(3, Column::Percentage(2));
        pb.columns
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

        pb.write(format!(
            "{} {} stream to {}",
            "Downloading".colorize("bold green"),
            if self.args.alternative {
                "audio/alternative"
            } else {
                "video"
            },
            self.progress.video.file.colorize("cyan")
        ));
        let pb = Arc::new(Mutex::new(pb));

        let pool = threadpool::ThreadPool::new(self.args.threads as usize);

        stored_bytes = self.download_segments(
            self.progress.video.to_playlist().segments,
            &self.progress.video.file,
            &pool,
            &pb,
            stored_bytes,
            relative_size,
        )?;

        pb.lock().unwrap().write(format!(
            " {} {} stream successfully",
            "Downloaded".colorize("bold green"),
            if self.args.alternative {
                "audio/alternative"
            } else {
                "video"
            }
        ));

        if let Some(audio) = &self.progress.audio {
            pb.lock().unwrap().write(format!(
                "{} audio stream to {}",
                "Downloading".colorize("bold green"),
                audio.file.colorize("cyan")
            ));

            let _ = self.download_segments(
                audio.to_playlist().segments,
                &audio.file,
                &pool,
                &pb,
                stored_bytes,
                None,
            )?;

            pb.lock().unwrap().write(format!(
                " {} audio stream successfully",
                "Downloaded".colorize("bold green"),
            ));
        }

        println!();
        Ok(())
    }

    fn download_subtitles(
        &mut self,
        subtitles: StreamData,
        pb: &mut RichProgress,
    ) -> Result<StreamData> {
        let mut subtitles = subtitles;

        let playlist = subtitles.to_playlist();
        let segments = playlist.segments;
        pb.pb.set_total(segments.len());

        let mut total_bytes = 0;

        let mut subtitles_data = self
            .client
            .get(&self.args.get_url(&segments[0].uri)?)
            .send()?
            .bytes()?
            .to_vec();

        total_bytes += subtitles_data.len();

        pb.replace(
            1,
            Column::Text(format!(
                "[bold blue]{}",
                utils::format_bytes(total_bytes, 2).2
            )),
        );
        pb.update(1);

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

        pb.write(format!(
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

            pb.replace(
                1,
                Column::Text(format!(
                    "[bold blue]{}",
                    utils::format_bytes(total_bytes, 2).2
                )),
            );
            pb.update(1);
        }

        if mp4subtitles {
            pb.write(format!(
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
        subtitles.downloaded = pb.pb.get_total();
        pb.write(format!(
            " {} subtitle stream successfully",
            "Downloaded".colorize("bold green"),
        ));

        Ok(subtitles.to_owned())
    }

    fn check_segments(&self) -> Result<()> {
        let mut segments = self.progress.video.to_playlist().segments;

        if let Some(audio) = &self.progress.audio {
            segments.extend_from_slice(&audio.to_playlist().segments);
        }

        self.args.get_url(&segments[0].uri)?;

        let mut encryption_type = None;

        for segment in segments {
            let segment_tags = dash::SegmentTag::from(&segment.unknown_tags);
            if segment_tags.single {
                bail!("single file dash streams are not supported")
            }

            match &segment.key {
                Some(m3u8_rs::Key {
                    method: m3u8_rs::KeyMethod::AES128,
                    ..
                }) => encryption_type = Some("AES-128"),
                Some(m3u8_rs::Key {
                    method: m3u8_rs::KeyMethod::SampleAES,
                    ..
                }) => {
                    println!("{} SAMPLE-AES streams found in playlist", "Encrypted".colorize("bold yellow"));
                    bail!("SAMPLE-AES encrypted playlists are not supported.")
                },
                Some(m3u8_rs::Key {
                    method: m3u8_rs::KeyMethod::Other(x),
                    ..
                }) => {
                    if x == "CENC" {
                        if self.args.key.is_empty() {
                            println!("{} CENC streams found in playlist", "Encrypted".colorize("bold yellow"));
                            bail!("CENC encrypted playlists requires --key")
                        }

                        encryption_type = Some("CENC");
                    } else {
                        println!("{} {} streams found in playlist", "Encrypted".colorize("bold yellow"), x);
                        bail!("{} encrypted playlists are not supported.", x)
                    }
                }
                _ => (),
            }
        }

        if let Some(encryption_type) = encryption_type {
            println!("  {} {} streams found in playlist", "Encrypted".colorize("bold yellow"), encryption_type);
        }

        Ok(())
    }

    fn download_segments(
        &self,
        segments: Vec<m3u8_rs::MediaSegment>,
        tempfile: &str,
        pool: &threadpool::ThreadPool,
        pb: &Arc<Mutex<RichProgress>>,
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

                let mut gaurded_pb = pb.lock().unwrap();
                gaurded_pb.replace(
                    1,
                    Column::Text(format!(
                        "[bold blue]{}",
                        utils::format_download_bytes(
                            stored_bytes + merger.stored(),
                            if let Some(size) = relative_size {
                                stored_bytes + size + merger.estimate()
                            } else {
                                stored_bytes + merger.estimate()
                            }
                        ),
                    )),
                );
                gaurded_pb.update_to(pos);
            }

            let sync_data = SyncData {
                byte_range: segment.byte_range.clone(),
                client: self.client.clone(),
                key_info: match &segment.key {
                    Some(m3u8_rs::Key {
                        uri: Some(link), ..
                    }) => Some((self.args.get_url(link)?, segment.key.clone().unwrap())),
                    _ => None,
                },
                merger: merger.clone(),
                pb: pb.clone(),
                relative_size,
                segment_index: i,
                segment_url: self.args.get_url(&segment.uri)?,
                stored_bytes,
                timer: timer.clone(),
                total_retries: self.args.retry_count,
            };

            pool.execute(move || {
                if let Err(e) = download_segment(&sync_data) {
                    let _ = sync_data.pb.lock().unwrap();
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

struct SyncData {
    byte_range: Option<m3u8_rs::ByteRange>,
    client: Arc<Client>,
    key_info: Option<(String, m3u8_rs::Key)>,
    merger: Arc<Mutex<BinaryMerger>>,
    pb: Arc<Mutex<RichProgress>>,
    relative_size: Option<usize>,
    segment_index: usize,
    segment_url: String,
    stored_bytes: usize,
    timer: Arc<std::time::Instant>,
    total_retries: u8,
}

fn download_segment(sync_data: &SyncData) -> Result<()> {
    let fetch_segment = || -> Result<Vec<u8>, reqwest::Error> {
        match sync_data.byte_range {
            Some(m3u8_rs::ByteRange {
                length: start,
                offset: Some(end),
            }) => Ok(sync_data
                .client
                .get(&sync_data.segment_url)
                .header(
                    header::RANGE,
                    format!("bytes={}-{}", start, start + end - 1),
                )
                .send()?
                .bytes()?
                .to_vec()),
            _ => Ok(sync_data
                .client
                .get(&sync_data.segment_url)
                .send()?
                .bytes()?
                .to_vec()),
        }
    };

    let mut retries = 0;
    let mut data = loop {
        match fetch_segment() {
            Ok(bytes) => {
                let elapsed_time = sync_data.timer.elapsed().as_secs() as usize;
                if elapsed_time != 0 {
                    let stored = sync_data.merger.lock().unwrap().stored() + bytes.len();
                    sync_data.pb.lock().unwrap().replace(
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
                if sync_data.total_retries > retries {
                    sync_data
                        .pb
                        .lock()
                        .unwrap()
                        .write(utils::check_reqwest_error(&e, &sync_data.segment_url)?);
                    retries += 1;
                    continue;
                } else {
                    bail!(
                        "Reached maximum number of retries for segment at index {}.",
                        sync_data.segment_index
                    )
                }
            }
        }
    };

    // Decrypt
    let fetch_key = |key_url| -> Result<Vec<u8>, reqwest::Error> {
        Ok(sync_data.client.get(key_url).send()?.bytes()?.to_vec())
    };

    retries = 0;
    if let Some((key_url, key)) = &sync_data.key_info {
        let key_content = loop {
            match fetch_key(key_url) {
                Ok(bytes) => break bytes,
                Err(e) => {
                    if sync_data.total_retries > retries {
                        sync_data
                            .pb
                            .lock()
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

    let mut guarded_merger = sync_data.merger.lock().unwrap();
    guarded_merger.write(sync_data.segment_index, &data)?;
    guarded_merger.flush()?;
    let stored = guarded_merger.stored();
    let estimate = guarded_merger.estimate();

    let mut gaurded_pb = sync_data.pb.lock().unwrap();
    gaurded_pb.replace(
        1,
        Column::Text(format!(
            "[bold blue]{}",
            utils::format_download_bytes(
                sync_data.stored_bytes + stored,
                if let Some(size) = sync_data.relative_size {
                    sync_data.stored_bytes + size + estimate
                } else {
                    sync_data.stored_bytes + estimate
                }
            ),
        )),
    );

    gaurded_pb.update(1);
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
