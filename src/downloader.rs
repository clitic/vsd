use crate::{
    merger::BinaryMerger,
    playlist::{KeyMethod, MediaPlaylist, PlaylistType, Segment},
    progress::Stream,
    utils,
};
use anyhow::{anyhow, bail, Result};
use kdam::{term::Colorizer, tqdm, BarExt, Column, RichProgress};
use reqwest::{
    header::{CONTENT_TYPE, RANGE},
    Url,
};
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::Write,
    path::Path,
    sync::{Arc, Mutex},
};

enum SubtitleType {
    Mp4Vtt,
    Mp4Ttml,
    SrtText,
    TtmlText,
    VttText,
}

pub(crate) fn download(
    all_keys: bool,
    baseurl: Option<Url>,
    client: reqwest::blocking::Client,
    directory: Option<std::path::PathBuf>,
    input: &str,
    key: Vec<(Option<String>, String)>,
    prefer_audio_lang: Option<String>,
    prefer_subs_lang: Option<String>,
    quality: crate::commands::Quality,
) -> Result<()> {
    let mut playlist_url = "https://example.com".parse::<Url>().unwrap();

    // -----------------------------------------------------------------------------------------
    // Fetch Playlist
    // -----------------------------------------------------------------------------------------

    let mut playlist_type = None;
    let path = std::path::Path::new(input);

    let playlist = if path.exists() {
        if baseurl.is_none() {
            println!(
                "    {} base url is not set",
                "Warning".colorize("bold yellow")
            );
        }

        if let Some(ext) = path.extension() {
            let ext = ext.to_string_lossy();
            if ext == "mpd" {
                playlist_type = Some(PlaylistType::Dash);
            } else if ext == "m3u" || ext == "m3u8" {
                playlist_type = Some(PlaylistType::Hls);
            }
        }

        if playlist_type.is_none() {
            std::fs::read(path)?
        } else {
            let text = std::fs::read_to_string(path)?;
            if text.contains("<MPD") {
                playlist_type = Some(PlaylistType::Dash);
            } else if text.contains("#EXTM3U") {
                playlist_type = Some(PlaylistType::Hls);
            }
            text.as_bytes().to_vec()
        }
    } else {
        let response = client.get(input).send()?;
        playlist_url = response.url().to_owned();

        if let Some(content_type) = response.headers().get(CONTENT_TYPE) {
            match content_type.as_bytes() {
                b"application/dash+xml" | b"video/vnd.mpeg.dash.mpd" => {
                    playlist_type = Some(PlaylistType::Dash)
                }
                b"application/x-mpegurl" | b"application/vnd.apple.mpegurl" => {
                    playlist_type = Some(PlaylistType::Hls)
                }
                _ => (),
            }
        }

        if playlist_type.is_none() {
            let text = response.text()?;
            if text.contains("<MPD") {
                playlist_type = Some(PlaylistType::Dash);
            } else if text.contains("#EXTM3U") {
                playlist_type = Some(PlaylistType::Hls);
            }
            text.as_bytes().to_vec()
        } else {
            response.bytes()?.to_vec()
        }
    };

    // -----------------------------------------------------------------------------------------
    // Parse Playlist & Select Streams & Push Segments
    // -----------------------------------------------------------------------------------------

    let (video_audio_streams, subtitle_streams) = match playlist_type {
        Some(PlaylistType::Dash) => {
            let mpd = crate::dash::parse(&playlist).map_err(|x| {
                anyhow!(
                    "couldn't parse xml string as mpd content (failed with {}).\n\n{}",
                    x,
                    String::from_utf8(playlist).unwrap()
                )
            })?;
            let (mut video_audio_streams, mut subtitle_streams) =
                crate::dash::parse_as_master(&mpd, playlist_url.as_str())
                    .sort_streams(prefer_audio_lang, prefer_subs_lang)
                    .select_streams(quality)?;

            for stream in video_audio_streams
                .iter_mut()
                .chain(subtitle_streams.iter_mut())
            {
                crate::dash::push_segments(
                    &mpd,
                    stream,
                    baseurl.as_ref().unwrap_or(&playlist_url).as_str(),
                );
                stream.uri = playlist_url.as_str().to_owned();
            }

            (video_audio_streams, subtitle_streams)
        }
        Some(PlaylistType::Hls) => match m3u8_rs::parse_playlist_res(&playlist) {
            Ok(m3u8_rs::Playlist::MasterPlaylist(m3u8)) => {
                let (mut video_audio_streams, mut subtitle_streams) =
                    crate::hls::parse_as_master(&m3u8, playlist_url.as_str())
                        .sort_streams(prefer_audio_lang, prefer_subs_lang)
                        .select_streams(quality)?;

                for stream in video_audio_streams
                    .iter_mut()
                    .chain(subtitle_streams.iter_mut())
                {
                    stream.uri = stream
                        .url(baseurl.as_ref().unwrap_or(&playlist_url))?
                        .as_str()
                        .to_owned();
                    let response = client.get(&stream.uri).send()?;
                    let media_playlist =
                        m3u8_rs::parse_media_playlist_res(&response.bytes()?).unwrap(); // TODO - Add better message for error
                    crate::hls::push_segments(&media_playlist, stream);
                }

                (video_audio_streams, subtitle_streams)
            }
            Ok(m3u8_rs::Playlist::MediaPlaylist(m3u8)) => {
                let mut media_playlist = crate::playlist::MediaPlaylist::default();
                media_playlist.uri = playlist_url.as_str().to_owned();
                crate::hls::push_segments(&m3u8, &mut media_playlist);
                (vec![media_playlist], vec![])
            }
            Err(_) => bail!("couldn't parse {} as HLS playlist.", playlist_url),
        },
        _ => bail!("only DASH (.mpd) and HLS (.m3u8) playlists are supported."),
    };

    // -----------------------------------------------------------------------------------------
    // Parse Key Ids & Return Error Related To Decryption
    // -----------------------------------------------------------------------------------------

    let mut default_kids = HashSet::new();

    for stream in &video_audio_streams {
        if let Some(segment) = stream.segments.get(0) {
            if let Some(key) = &segment.key {
                match &key.method {
                    KeyMethod::Other(x) => bail!("{} decryption is not supported.", x),
                    KeyMethod::SampleAes => {
                        if stream.is_hls() {
                            bail!("sample-aes (HLS) decryption is not supported.");
                        }
                    }
                    _ => (),
                }

                if let Some(default_kid) = &key.default_kid {
                    default_kids.insert(default_kid.replace('-', ""));
                }
            }
        }
    }

    let mut kids = HashSet::new();

    for stream in &video_audio_streams {
        if let Some(segment) = stream.segments.get(0) {
            if segment.map.is_some() {
                let mut request = client.get(
                    segment
                        .map_url(
                            baseurl
                                .as_ref()
                                .unwrap_or(&stream.uri.parse::<Url>().unwrap()),
                        )?
                        .unwrap(),
                );

                if let Some((range, _)) = segment.map_range(0) {
                    request = request.header(RANGE, range);
                }

                let response = request.send()?;
                let pssh = crate::mp4parser::Pssh::new(&response.bytes()?).unwrap(); // TODO - Add better message for error

                for key_id in pssh.key_ids {
                    if !kids.contains(&key_id.value) {
                        kids.insert(key_id.value.clone());
                        println!(
                            "      {} {} {} ({})",
                            "KeyId".colorize("bold green"),
                            if default_kids.contains(&key_id.value) {
                                "*"
                            } else {
                                " "
                            },
                            key_id.uuid(),
                            key_id.system_type,
                        );
                    }
                }
            }
        }
    }

    for default_kid in &default_kids {
        if !key
            .iter()
            .flat_map(|x| x.0.as_ref())
            .any(|x| x == default_kid)
        {
            bail!(
                "use {} flag to specify CENC content decryption keys for atleast * (star) prefixed key ids.",
                "--key".colorize("bold green")
            );
        }
    }

    // -----------------------------------------------------------------------------------------
    // Prepare Progress Bar
    // -----------------------------------------------------------------------------------------

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

    // -----------------------------------------------------------------------------------------
    // Create Temporary Directory
    // -----------------------------------------------------------------------------------------

    if let Some(directory) = &directory {
        if !directory.exists() {
            std::fs::create_dir_all(directory)?;
        }
    }

    // -----------------------------------------------------------------------------------------
    // Download Subtitle Streams
    // -----------------------------------------------------------------------------------------

    for stream in subtitle_streams {
        pb.write(format!(
            " {} subtitle stream {}",
            "Processing".colorize("bold green"),
            stream.display_subtitle_stream().colorize("cyan"),
        ));

        let length = stream.segments.len();

        if length == 0 {
            pb.write(format!(
                "    {} skipping subtitle stream (no segments)",
                "Warning".colorize("bold yellow")
            ));
            continue;
        }

        pb.pb.set_total(length);

        let mut extension = stream.extension();
        let mut codec = None;

        if let Some(codecs) = &stream.codecs {
            match codecs.as_str() {
                "vtt" => {
                    extension = "vtt".to_owned();
                    codec = Some(SubtitleType::VttText);
                }
                "wvtt" => {
                    extension = "vtt".to_owned();
                    codec = Some(SubtitleType::Mp4Vtt);
                }
                "stpp" | "stpp.ttml" | "stpp.ttml.im1t" | "stpp.TTML.im1t" => {
                    extension = "srt".to_owned();
                    codec = Some(SubtitleType::Mp4Ttml);
                }
                _ => (),
            }
        }

        let mut total_bytes = 0;
        let mut subtitles_data = vec![];
        let mut previous_byterange_end = 0;
        let mut tempfile = String::new();

        let seg_baseurl = baseurl
            .clone()
            .unwrap_or(stream.uri.parse::<Url>().unwrap());

        let mut first_run = true;

        for segment in &stream.segments {
            if segment.map.is_some() {
                let mut request = client.get(segment.map_url(&seg_baseurl)?.unwrap());

                if let Some((range, _)) = segment.map_range(0) {
                    request = request.header(RANGE, range);
                }

                let response = request.send()?;
                let bytes = response.bytes()?;
                total_bytes += bytes.len();
                subtitles_data.extend_from_slice(&bytes);
            }

            let mut request = client.get(segment.seg_url(&seg_baseurl)?);

            if let Some((range, previous)) = segment.seg_range(previous_byterange_end) {
                previous_byterange_end = previous;
                request = request.header(RANGE, range);
            }

            let response = request.send()?;
            let bytes = response.bytes()?;
            total_bytes += bytes.len();
            subtitles_data.extend_from_slice(&bytes);

            if first_run {
                first_run = false;

                if subtitles_data.starts_with(b"WEBVTT") {
                    extension = "vtt".to_owned();
                    codec = Some(SubtitleType::VttText);
                } else if subtitles_data.starts_with(b"1") {
                    extension = "srt".to_owned();
                    codec = Some(SubtitleType::SrtText);
                } else if subtitles_data.starts_with(b"<?xml") || subtitles_data.starts_with(b"<tt")
                {
                    // application/ttml+xml
                    extension = "srt".to_owned();
                    codec = Some(SubtitleType::TtmlText);
                } else if codec.is_none() {
                    bail!("cannot determine subtitle codec.");
                }

                // tempfile = subtitles.path(&self.args.directory);
                tempfile = "1.vtt".to_owned();
                pb.write(format!(
                    "{} subtitle stream to {}",
                    "Downloading".colorize("bold green"),
                    tempfile.colorize("cyan")
                ));
            }

            pb.replace(
                1,
                Column::Text(format!(
                    "[bold blue]{}",
                    utils::format_bytes(total_bytes, 2).2
                )),
            );
            pb.update(1);
        }

        match codec {
            Some(SubtitleType::Mp4Vtt) => {
                pb.write(format!(
                    " {} wvtt subtitles",
                    "Extracting".colorize("bold cyan"),
                ));

                let vtt = crate::mp4parser::Mp4VttParser::parse_init(&subtitles_data)
                    .map_err(|x| anyhow!(x))?;
                let cues = vtt
                    .parse_media(&subtitles_data, None)
                    .map_err(|x| anyhow!(x))?;
                let subtitles = crate::mp4parser::Subtitles::new(cues);
                File::create(&tempfile)?.write_all(subtitles.to_vtt().as_bytes())?;
            }
            Some(SubtitleType::Mp4Ttml) => {
                pb.write(format!(
                    " {} stpp subtitles",
                    "Extracting".colorize("bold cyan"),
                ));

                let ttml = crate::mp4parser::Mp4TtmlParser::parse_init(&subtitles_data)
                    .map_err(|x| anyhow!(x))?;
                let cues = ttml.parse_media(&subtitles_data).map_err(|x| anyhow!(x))?;
                let subtitles = crate::mp4parser::Subtitles::new(cues);
                File::create(&tempfile)?.write_all(subtitles.to_srt().as_bytes())?;
            }
            Some(SubtitleType::TtmlText) => {
                pb.write(format!(
                    " {} ttml+xml subtitles",
                    "Extracting".colorize("bold cyan"),
                ));

                let xml = String::from_utf8(subtitles_data)
                    .map_err(|_| anyhow!("cannot decode subtitles as valid utf8 string."))?;
                let ttml = crate::mp4parser::ttml_text_parser::parse(&xml).map_err(|x| {
                    anyhow!(
                        "couldn't parse xml string as ttml content (failed with {}).\n\n{}",
                        x,
                        xml
                    )
                })?;
                let cues = ttml.to_cues();
                let subtitles = crate::mp4parser::Subtitles::new(cues);
                File::create(&tempfile)?.write_all(subtitles.to_srt().as_bytes())?;
            }
            _ => File::create(&tempfile)?.write_all(&subtitles_data)?,
        };

        pb.write(format!(
            " {} subtitle stream successfully",
            "Downloaded".colorize("bold green"),
        ));
        println!();
        pb.reset(Some(0));
    }

    Ok(())
}

// fn dash_decrypt(
//     &self,
//     segment: &m3u8_rs::MediaSegment,
//     default_kid: Option<String>,
//     pb: &Arc<Mutex<RichProgress>>,
// ) -> Result<(Vec<u8>, HashMap<String, String>)> {
//     if dash::SegmentTag::from(&segment.unknown_tags).init {
//         let init_segment = self
//             .client
//             .get(self.args.get_url(&segment.uri)?)
//             .send()?
//             .bytes()?
//             .to_vec();
//         let mut keys = HashMap::new();

//         for key in &self.args.key {
//             if let Some(default_kid) = &default_kid {
//                 if let Some(kid) = &key.0 {
//                     if default_kid == kid {
//                         keys.insert(default_kid.to_owned(), key.1.to_owned());
//                     }
//                 } else {
//                     keys.insert(default_kid.to_owned(), key.1.to_owned());
//                 }
//             }
//         }

//         if keys.len() != 1 {
//             bail!(
//                 "specify decryption key using {} syntax",
//                 "--key KID:(base64:)KEY".colorize("bold green")
//             )
//         }

//         if let Some(default_kid) = &default_kid {
//             pb.lock().unwrap().write(format!(
//                 "        {} {}:{}",
//                 "Key".colorize("bold green"),
//                 default_kid,
//                 keys.get(default_kid).unwrap()
//             ));
//         }

//         Ok((init_segment, keys))
//     } else {
//         bail!("stream is CENC encrypted without a init segment?")
//     }
// }

//     fn download(&mut self) -> Result<()> {
//         if matches!(
//             self.alternative_media_type,
//             Some(m3u8_rs::AlternativeMediaType::Subtitles)
//                 | Some(m3u8_rs::AlternativeMediaType::ClosedCaptions)
//         ) {
//             self.progress.video =
//                 self.download_subtitles(self.progress.video.to_owned(), &mut pb)?;
//             return Ok(());
//         }

//         if let Some(subtitles) = &self.progress.subtitles {
//             self.progress.subtitles = Some(self.download_subtitles(subtitles.to_owned(), &mut pb)?);
//         }

//         let mut segment_factor = 0;

//         if self.dash && self.cenc_encrypted_video {
//             segment_factor += 1;
//         }

//         if self.dash && self.cenc_encrypted_audio {
//             segment_factor += 1;
//         }

//         pb.pb.reset(Some(
//             self.progress.video.total + self.progress.audio.clone().map(|x| x.total).unwrap_or(0)
//                 - segment_factor,
//         ));
//         pb.replace(3, Column::Percentage(2));
//         pb.columns
//             .extend_from_slice(&[Column::text("•"), Column::text("[yellow]?")]);

//         let mut stored_bytes = 0;
//         let relative_size = if let Some(audio) = &self.progress.audio {
//             let segments = audio.to_playlist().segments;

//             Some(
//                 (segments.len() - if self.cenc_encrypted_audio { 1 } else { 0 })
//                     * self
//                         .client
//                         .get(&self.args.get_url(&segments[1].uri)?)
//                         .send()?
//                         .content_length()
//                         .unwrap_or(0) as usize,
//             )
//         } else {
//             None
//         };

//         self.progress.video.set_suffix(if self.args.alternative {
//             "audio-alternative"
//         } else {
//             "video"
//         });

//         let pb = Arc::new(Mutex::new(pb));
//         let pool = threadpool::ThreadPool::new(self.args.threads as usize);
//         let video_segments = self.progress.video.to_playlist().segments;

//         let tempfile = self.progress.video.path(&self.args.directory);

//         pb.lock().unwrap().write(format!(
//             "{} {} stream to {}",
//             "Downloading".colorize("bold green"),
//             if self.args.alternative {
//                 "audio/alternative"
//             } else {
//                 "video"
//             },
//             tempfile.colorize("cyan")
//         ));

//         let dash_decrypt = if self.dash && self.cenc_encrypted_video {
//             Some(
//                 self.dash_decrypt(
//                     &video_segments[0],
//                     dash::SegmentTag::from(&video_segments[1].unknown_tags)
//                         .kid
//                         .map(|x| x.replace('-', "").to_lowercase()),
//                     &pb,
//                 )?,
//             )
//         } else {
//             None
//         };

//         stored_bytes = self.download_segments(
//             if dash_decrypt.is_some() {
//                 video_segments[1..].to_vec()
//             } else {
//                 video_segments
//             },
//             &tempfile,
//             &pool,
//             &pb,
//             stored_bytes,
//             relative_size,
//             dash_decrypt,
//         )?;

//         pb.lock().unwrap().write(format!(
//             " {} {} stream successfully",
//             "Downloaded".colorize("bold green"),
//             if self.args.alternative {
//                 "audio/alternative"
//             } else {
//                 "video"
//             }
//         ));

//         if let Some(audio) = &self.progress.audio {
//             let audio_segments = audio.to_playlist().segments;
//             let tempfile = audio.path(&self.args.directory);

//             pb.lock().unwrap().write(format!(
//                 "{} audio stream to {}",
//                 "Downloading".colorize("bold green"),
//                 tempfile.colorize("cyan")
//             ));

//             let dash_decrypt = if self.dash && self.cenc_encrypted_audio {
//                 Some(
//                     self.dash_decrypt(
//                         &audio_segments[0],
//                         dash::SegmentTag::from(&audio_segments[1].unknown_tags)
//                             .kid
//                             .map(|x| x.replace('-', "").to_lowercase()),
//                         &pb,
//                     )?,
//                 )
//             } else {
//                 None
//             };

//             let _ = self.download_segments(
//                 if dash_decrypt.is_some() {
//                     audio_segments[1..].to_vec()
//                 } else {
//                     audio_segments
//                 },
//                 &tempfile,
//                 &pool,
//                 &pb,
//                 stored_bytes,
//                 None,
//                 dash_decrypt,
//             )?;

//             pb.lock().unwrap().write(format!(
//                 " {} audio stream successfully",
//                 "Downloaded".colorize("bold green"),
//             ));
//         }

//         println!();
//         Ok(())
//     }

//     #[allow(clippy::too_many_arguments)]
//     fn download_segments(
//         &self,
//         segments: Vec<m3u8_rs::MediaSegment>,
//         tempfile: &str,
//         pool: &threadpool::ThreadPool,
//         pb: &Arc<Mutex<RichProgress>>,
//         stored_bytes: usize,
//         relative_size: Option<usize>,
//         dash_decrypt: Option<(Vec<u8>, HashMap<String, String>)>,
//     ) -> Result<usize> {
//         let merger = Arc::new(Mutex::new(BinaryMerger::new(segments.len(), tempfile)?));

//         // TODO support resume
//         // Arc::new(Mutex::new(BinaryMerger::try_from_json(
//         //     segments.len(),
//         //     tempfile,
//         //     self.progress.file.clone(),
//         // )?))
//         // merger.lock().unwrap().update()?;

//         let timer = Arc::new(std::time::Instant::now());

//         let mut previous_byterange_end = 0;

//         for (i, segment) in segments.iter().enumerate() {
//             // if self.args.resume {
//             //     let merger = merger.lock().unwrap();
//             //     let pos = merger.position();

//             //     if pos != 0 && pos > i {
//             //         continue;
//             //     }

//             //     let mut gaurded_pb = pb.lock().unwrap();
//             //     gaurded_pb.replace(
//             //         1,
//             //         Column::Text(format!(
//             //             "[bold blue]{}",
//             //             utils::format_download_bytes(
//             //                 stored_bytes + merger.stored(),
//             //                 if let Some(size) = relative_size {
//             //                     stored_bytes + size + merger.estimate()
//             //                 } else {
//             //                     stored_bytes + merger.estimate()
//             //                 }
//             //             ),
//             //         )),
//             //     );
//             //     gaurded_pb.update_to(pos);
//             // }

//             let thread_data = ThreadData {
//                 // Request
//                 client: client.clone(),

//                 // Segment
//                 byte_range: if let Some(byte_range) = &segment.byte_range {
//                     let offset = byte_range.offset.unwrap_or(0);

//                     let (start, end) = if offset == 0 {
//                         (
//                             previous_byterange_end,
//                             previous_byterange_end + byte_range.length - 1,
//                         )
//                     } else {
//                         (byte_range.length, byte_range.length + offset - 1)
//                     };

//                     previous_byterange_end = end;
//                     Some(format!("bytes={}-{}", start, end))
//                 } else {
//                     None
//                 },
//                 map: if let Some(map) = &segment.map {
//                     let uri = if map.uri.starts_with("http") {
//                         map.uri.clone()
//                     } else {
//                         self.args.get_url(&map.uri)?
//                     };

//                     client.get(uri).send()?.bytes()?.to_vec()
//                 } else {
//                     vec![]
//                 },
//                 segment_url: self.args.get_url(&segment.uri)?,
//                 total_retries: self.args.retry_count,

//                 // Decryption
//                 dash_decrypt: dash_decrypt.clone(),
//                 key: if let Some(key) = &segment.key {
//                     let mut key = key.clone();

//                     if let Some(uri) = &key.uri {
//                         if uri.starts_with("skd://") {
//                             let uri = uri.trim_start_matches("skd://");

//                             if uri.contains(':') {
//                                 key.uri = Some(uri.to_owned());
//                             } else {
//                                 bail!("SAMPLE-AES (com.apple.streamingkeydelivery) skd://{} uri is not supported", uri)
//                             }
//                         } else {
//                             if !uri.starts_with("http") {
//                                 key.uri = Some(self.args.get_url(uri)?);
//                             }
//                         }
//                     }

//                     Some(key)
//                 } else {
//                     None
//                 },

//                 // File
//                 index: i,
//                 merger: merger.clone(),

//                 // Progress Bar
//                 pb: pb.clone(),
//                 relative_size,
//                 stored_bytes,
//                 timer: timer.clone(),
//             };

//             pool.execute(move || {
//                 if let Err(e) = thread_data.perform() {
//                     let _lock = thread_data.pb.lock().unwrap();
//                     println!("\n{}: {}", "error".colorize("bold red"), e);
//                     std::process::exit(1);
//                 }
//             });
//         }

//         pool.join();
//         let mut merger = merger.lock().unwrap();
//         merger.flush()?;

//         if !merger.buffered() {
//             bail!(
//                 "File {} not downloaded successfully.",
//                 tempfile.colorize("bold red")
//             );
//         }

//         Ok(merger.stored())
//     }
// }

// struct ThreadData {
//     // Request
//     client: Client,

//     // Segment
//     byte_range: Option<String>,
//     map: Vec<u8>,
//     segment_url: String,
//     total_retries: u8,

//     // Decryption
//     dash_decrypt: Option<(Vec<u8>, HashMap<String, String>)>,
//     key: Option<m3u8_rs::Key>,

//     // File
//     index: usize,
//     merger: Arc<Mutex<BinaryMerger>>,

//     // Progress Bar
//     pb: Arc<Mutex<RichProgress>>,
//     relative_size: Option<usize>,
//     stored_bytes: usize,
//     timer: Arc<std::time::Instant>,
// }

// impl ThreadData {
//     fn perform(&self) -> Result<()> {
//         let mut segment = self.map.clone();
//         segment.append(&mut self.download_segment()?);
//         let segment = self.decrypt(&segment)?;

//         let mut merger = self.merger.lock().unwrap();
//         merger.write(self.index, &segment)?;
//         merger.flush()?;

//         self.notify(merger.stored(), merger.estimate())?;
//         Ok(())
//     }

//     fn download_segment(&self) -> Result<Vec<u8>> {
//         let fetch_segment = || -> Result<Vec<u8>, reqwest::Error> {
//             if let Some(range) = &self.byte_range {
//                 Ok(self
//                     .client
//                     .get(&self.segment_url)
//                     .header(header::RANGE, range)
//                     .send()?
//                     .bytes()?
//                     .to_vec())
//             } else {
//                 Ok(client.get(&self.segment_url).send()?.bytes()?.to_vec())
//             }
//         };

//         let mut retries = 0;
//         let data = loop {
//             match fetch_segment() {
//                 Ok(bytes) => {
//                     // Download Speed
//                     let elapsed_time = self.timer.elapsed().as_secs() as usize;
//                     if elapsed_time != 0 {
//                         let stored = self.merger.lock().unwrap().stored() + bytes.len();
//                         self.pb.lock().unwrap().replace(
//                             13,
//                             Column::Text(format!(
//                                 "[yellow]{}/s",
//                                 utils::format_bytes(stored / elapsed_time, 2).2
//                             )),
//                         );
//                     }

//                     break bytes;
//                 }
//                 Err(e) => {
//                     if self.total_retries > retries {
//                         self.pb
//                             .lock()
//                             .unwrap()
//                             .write(utils::check_reqwest_error(&e, &self.segment_url)?);
//                         retries += 1;
//                         continue;
//                     } else {
//                         bail!(
//                             "Reached maximum number of retries for segment at index {}.",
//                             self.index
//                         )
//                     }
//                 }
//             }
//         };

//         Ok(data)
//     }

//     fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
//         match &self.key {
//             Some(m3u8_rs::Key {
//                 iv,
//                 method: m3u8_rs::KeyMethod::AES128,
//                 uri: Some(uri),
//                 ..
//             }) => {
//                 // let fetch_key = |key_url| -> Result<Vec<u8>, reqwest::Error> {
//                 //     Ok(client.get(key_url).send()?.bytes()?.to_vec())
//                 // };

//                 let mut retries = 0;

//                 let key = loop {
//                     match Ok(client.get(uri).send()?.bytes()?.to_vec()) {
//                         Ok(bytes) => break bytes,
//                         Err(e) => {
//                             if self.total_retries > retries {
//                                 self.pb
//                                     .lock()
//                                     .unwrap()
//                                     .write(utils::check_reqwest_error(&e, uri)?);
//                                 retries += 1;
//                                 continue;
//                             } else {
//                                 bail!(
//                                     "Reached maximum number of retries to download decryption key."
//                                 )
//                             }
//                         }
//                     }
//                 };

//                 Ok(openssl::symm::decrypt(
//                     openssl::symm::Cipher::aes_128_cbc(),
//                     &key,
//                     iv.as_ref().map(|x| x.as_bytes()),
//                     data,
//                 )?)
//             }
//             Some(m3u8_rs::Key {
//                 method: m3u8_rs::KeyMethod::SampleAES,
//                 keyformat: Some(keyformat),
//                 uri: Some(uri),
//                 ..
//             }) if keyformat == "com.apple.streamingkeydelivery" => Ok(mp4decrypt::mp4decrypt(
//                 data,
//                 HashMap::from([(
//                     uri.split(':').nth(0).unwrap().replace('-', ""),
//                     uri.split(':').nth(1).unwrap().to_lowercase(),
//                 )]),
//                 None,
//             )
//             .map_err(|x| anyhow!(x))?),
//             Some(m3u8_rs::Key {
//                 method: m3u8_rs::KeyMethod::Other(method),
//                 ..
//             }) if method == "CENC" => {
//                 let (mut init_segment, keys) = self.dash_decrypt.clone().unwrap();
//                 init_segment.extend_from_slice(data);
//                 Ok(mp4decrypt::mp4decrypt(&init_segment, keys, None).map_err(|x| anyhow!(x))?)
//             }
//             _ => Ok(data.to_vec()),
//         }
//     }

//     fn notify(&self, stored: usize, estimate: usize) -> Result<()> {
//         let mut pb = self.pb.lock().unwrap();
//         pb.replace(
//             1,
//             Column::Text(format!(
//                 "[bold blue]{}",
//                 utils::format_download_bytes(
//                     self.stored_bytes + stored,
//                     if let Some(size) = self.relative_size {
//                         self.stored_bytes + size + estimate
//                     } else {
//                         self.stored_bytes + estimate
//                     }
//                 ),
//             )),
//         );
//         pb.update(1);
//         Ok(())
//     }
// }

// // fn hls_live(&mut self) -> Result<()> {
// //     let live_playlist = hls::LivePlaylist::new(
// //         &self.args.input,
// //         client.clone(),
// //         self.args.record_duration,
// //     );
// //     let mut file = std::fs::File::create(&self.args.tempfile())?;
// //     let mut pb = tqdm!(
// //         // total = total,
// //         unit = "ts".to_owned(),
// //         dynamic_ncols = true
// //     );
// //     pb.refresh();
// //     let mut total_bytes = 0;

// //     for media in live_playlist {
// //         for seg in media.map_err(|x| anyhow!(x))?.segments {
// //             let bytes = self
// //                 .client
// //                 .get(&self.args.get_url(&seg.uri)?)
// //                 .send()?
// //                 .bytes()?
// //                 .to_vec();
// //             total_bytes += bytes.len();
// //             file.write_all(&bytes)?;
// //             pb.set_description(utils::format_bytes(total_bytes, 2).2);
// //             pb.update(1);
// //         }
// //     }

// //     Ok(())
// // }
