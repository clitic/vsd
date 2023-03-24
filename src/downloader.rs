use crate::{
    merger::Merger,
    playlist::{KeyMethod, MediaType, PlaylistType},
    utils,
};
use anyhow::{anyhow, bail, Result};
use kdam::{term::Colorizer, tqdm, BarExt, Column, RichProgress};
use reqwest::{
    blocking::{Client, RequestBuilder},
    header::{CONTENT_TYPE, RANGE},
    StatusCode, Url,
};
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::Write,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Instant,
};

enum SubtitleType {
    Mp4Vtt,
    Mp4Ttml,
    SrtText,
    TtmlText,
    VttText,
}

struct Stream {
    file_path: String,
    language: Option<String>,
    media_type: MediaType,
}

#[derive(Clone)]
struct Keys {
    bytes: Vec<u8>,
    iv: Option<String>,
    method: KeyMethod,
}

impl Keys {
    fn from_hex_keys(keys: HashMap<String, String>) -> Self {
        let mut bytes = String::new();

        for (kid, key) in keys {
            bytes += &(kid + ":" + &key + ";");
        }

        Self {
            bytes: bytes.get(..(bytes.len() - 1)).unwrap().as_bytes().to_vec(),
            iv: None,
            method: KeyMethod::Cenc,
        }
    }

    fn as_hex_keys(&self) -> HashMap<String, String> {
        String::from_utf8(self.bytes.clone())
            .unwrap()
            .split(';')
            .map(|x| {
                let kid_key = x.split(':').collect::<Vec<_>>();
                (
                    kid_key.get(0).unwrap().to_string(),
                    kid_key.get(1).unwrap().to_string(),
                )
            })
            .collect()
    }

    fn decrypt(&self, data: Vec<u8>) -> Result<Vec<u8>> {
        Ok(match self.method {
            KeyMethod::Aes128 => openssl::symm::decrypt(
                openssl::symm::Cipher::aes_128_cbc(),
                &self.bytes,
                self.iv.as_ref().map(|x| x.as_bytes()),
                &data,
            )?,
            KeyMethod::Cenc | KeyMethod::SampleAes => {
                mp4decrypt::mp4decrypt(&data, self.as_hex_keys(), None).map_err(|x| anyhow!(x))?
            }
            _ => data,
        })
    }
}

pub(crate) fn download(
    all_keys: bool,
    baseurl: Option<Url>,
    client: Client,
    directory: Option<PathBuf>,
    input: &str,
    keys: Vec<(Option<String>, String)>,
    no_decrypt: bool,
    output: Option<String>,
    prefer_audio_lang: Option<String>,
    prefer_subs_lang: Option<String>,
    quality: crate::commands::Quality,
    retry_count: u8,
    threads: u8,
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
                )?;
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
    // Parse Key Ids
    // -----------------------------------------------------------------------------------------

    let mut default_kids = HashSet::new();

    for stream in &video_audio_streams {
        if let Some(segment) = stream.segments.get(0) {
            if let Some(key) = &segment.key {
                match &key.method {
                    KeyMethod::Other(x) => bail!("{} decryption is not supported. Use {} flag to download encrypted streams.", x, "--no-decrypt".colorize("bold green")),
                    KeyMethod::SampleAes => {
                        if stream.is_hls() {
                            // TODO - Only if "keyformat=identity" 
                            bail!("sample-aes (HLS) decryption is not supported. Use {} flag to download encrypted streams.", "--no-decrypt".colorize("bold green"));
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
        if !keys
            .iter()
            .flat_map(|x| x.0.as_ref())
            .any(|x| x == default_kid)
            && !no_decrypt
        {
            bail!(
                "use {} flag to specify CENC content decryption keys for at least * (star) prefixed key ids.",
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
    // Prepare Directory & Store Streams Metadata
    // -----------------------------------------------------------------------------------------

    if let Some(directory) = &directory {
        if !directory.exists() {
            std::fs::create_dir_all(directory)?;
        }
    }

    if output.is_some() && no_decrypt {
        println!(
            "    {} --output is ignored when --no-decrypt is used",
            "Warning".colorize("bold yellow")
        );
    }

    let mut temp_files = vec![];

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

        let mut ext = stream.extension();
        let mut codec = None;

        if let Some(codecs) = &stream.codecs {
            match codecs.as_str() {
                "vtt" => {
                    ext = "vtt".to_owned();
                    codec = Some(SubtitleType::VttText);
                }
                "wvtt" => {
                    ext = "vtt".to_owned();
                    codec = Some(SubtitleType::Mp4Vtt);
                }
                "stpp" | "stpp.ttml" | "stpp.ttml.im1t" | "stpp.TTML.im1t" => {
                    ext = "srt".to_owned();
                    codec = Some(SubtitleType::Mp4Ttml);
                }
                _ => (),
            }
        }

        let seg_baseurl = baseurl
            .clone()
            .unwrap_or(stream.uri.parse::<Url>().unwrap());

        let mut temp_file = String::new();
        let mut previous_byterange_end = 0;

        let mut first_run = true;
        let mut subtitles_data = vec![];

        for segment in &stream.segments {
            if segment.map.is_some() {
                let mut request = client.get(segment.map_url(&seg_baseurl)?.unwrap());

                if let Some((range, _)) = segment.map_range(0) {
                    request = request.header(RANGE, range);
                }

                let response = request.send()?;
                let bytes = response.bytes()?;
                subtitles_data.extend_from_slice(&bytes);
            }

            let mut request = client.get(segment.seg_url(&seg_baseurl)?);

            if let Some((range, previous)) = segment.seg_range(previous_byterange_end) {
                previous_byterange_end = previous;
                request = request.header(RANGE, range);
            }

            let response = request.send()?;
            let bytes = response.bytes()?;
            subtitles_data.extend_from_slice(&bytes);

            if first_run {
                first_run = false;

                if subtitles_data.starts_with(b"WEBVTT") {
                    ext = "vtt".to_owned();
                    codec = Some(SubtitleType::VttText);
                } else if subtitles_data.starts_with(b"1") {
                    ext = "srt".to_owned();
                    codec = Some(SubtitleType::SrtText);
                } else if subtitles_data.starts_with(b"<?xml") || subtitles_data.starts_with(b"<tt")
                {
                    // TODO - Match using Representation node @mimeType (DASH)
                    // application/ttml+xml
                    ext = "srt".to_owned();
                    codec = Some(SubtitleType::TtmlText);
                } else if codec.is_none() {
                    bail!("cannot determine subtitle codec.");
                }

                temp_file = stream
                    .file_path(&directory, &ext)
                    .to_string_lossy()
                    .to_string();
                temp_files.push(Stream {
                    file_path: temp_file.clone(),
                    language: stream.language.clone(),
                    media_type: stream.media_type.clone(),
                });
                pb.write(format!(
                    "{} subtitle stream to {}",
                    "Downloading".colorize("bold green"),
                    temp_file.colorize("cyan")
                ));
            }

            pb.replace(
                0,
                Column::Text(format!(
                    "[bold blue]{}",
                    utils::format_bytes(subtitles_data.len(), 2).2
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
                File::create(&temp_file)?.write_all(subtitles.to_vtt().as_bytes())?;
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
                File::create(&temp_file)?.write_all(subtitles.to_srt().as_bytes())?;
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
                File::create(&temp_file)?.write_all(ttml.to_srt().as_bytes())?;
            }
            _ => File::create(&temp_file)?.write_all(&subtitles_data)?,
        };

        pb.write(format!(
            " {} subtitle stream successfully",
            "Downloaded".colorize("bold green"),
        ));
        println!(); // TODO - See Later
        pb.reset(Some(0));
    }

    // -----------------------------------------------------------------------------------------
    // Prepare Progress Bar
    // -----------------------------------------------------------------------------------------

    pb.replace(2, Column::Percentage(2));
    pb.columns
        .extend_from_slice(&[Column::text("•"), Column::text("[yellow]?")]);
    pb.pb.reset(Some(
        video_audio_streams.iter().map(|x| x.segments.len()).sum(),
    ));
    let pb = Arc::new(Mutex::new(pb));

    // -----------------------------------------------------------------------------------------
    // Separate Video & Audio Streams
    // -----------------------------------------------------------------------------------------

    let mut video_stream = None;
    let mut audio_streams = vec![];

    for stream in video_audio_streams {
        match &stream.media_type {
            MediaType::Audio => audio_streams.push(stream),
            MediaType::Video => video_stream = Some(stream),
            _ => (),
        }
    }

    // -----------------------------------------------------------------------------------------
    // Estimation
    // -----------------------------------------------------------------------------------------

    let mut downloaded_bytes = 0;
    let mut relative_size = vec![];

    for (i, stream) in audio_streams.iter().enumerate() {
        if let Some(segment) = stream.segments.get(0) {
            let mut request = client.head(
                segment.seg_url(
                    baseurl
                        .as_ref()
                        .unwrap_or(&stream.uri.parse::<Url>().unwrap()),
                )?,
            );

            if let Some((range, _)) = segment.map_range(0) {
                request = request.header(RANGE, range);
            }

            let response = request.send()?;
            relative_size.insert(
                i,
                stream.segments.len() * (response.content_length().unwrap_or(0) as usize),
            );
        }
    }

    // -----------------------------------------------------------------------------------------
    // Prepare Thread Pool
    // -----------------------------------------------------------------------------------------

    // TODO - Create a custom thread pool module.
    let pool = threadpool::ThreadPool::new(threads as usize);

    // -----------------------------------------------------------------------------------------
    // Download Video Stream
    // -----------------------------------------------------------------------------------------

    if let Some(mut stream) = video_stream {
        let temp_file = stream
            .file_path(&directory, &stream.extension())
            .to_string_lossy()
            .to_string();
        temp_files.push(Stream {
            file_path: temp_file.clone(),
            language: stream.language.clone(),
            media_type: stream.media_type.clone(),
        });
        pb.lock().unwrap().write(format!(
            "{} video stream to {}",
            "Downloading".colorize("bold green"),
            temp_file.colorize("cyan")
        ));

        let merger = Arc::new(Mutex::new(Merger::new(stream.segments.len(), &temp_file)?));

        // TODO - Add resume support
        // Arc::new(Mutex::new(BinaryMerger::try_from_json(
        //     segments.len(),
        //     tempfile,
        //     self.progress.file.clone(),
        // )?))
        // merger.lock().unwrap().update()?;

        let timer = Arc::new(Instant::now());
        let mut previous_map = None;
        let mut previous_key = None;
        let mut previous_byterange_end = 0;

        for (i, segment) in stream.segments.iter().enumerate() {
            // if resume {
            //     let merger = merger.lock().unwrap();
            //     let pos = merger.position();

            //     if pos != 0 && pos > i {
            //         continue;
            //     }

            //     let mut gaurded_pb = pb.lock().unwrap();
            //     gaurded_pb.replace(
            //         0,
            //         Column::Text(format!(
            //             "[bold blue]{}",
            //             utils::format_download_bytes(
            //                 stored_bytes + merger.stored(),
            //                 if let Some(size) = relative_size {
            //                     stored_bytes + size + merger.estimate()
            //                 } else {
            //                     stored_bytes + merger.estimate()
            //                 }
            //             ),
            //         )),
            //     );
            //     gaurded_pb.update_to(pos);
            // }

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
                let bytes = response.bytes()?;
                previous_map = Some(bytes.to_vec())
            }

            if !no_decrypt {
                if let Some(key) = &segment.key {
                    match key.method {
                        KeyMethod::Aes128 => {
                            // TODO - Handle keyformat correctly
                            previous_key = Some(Keys {
                                bytes: if key.key_format.is_none() {
                                    let request =
                                        client.get(
                                            segment
                                                .key_url(baseurl.as_ref().unwrap_or(
                                                    &stream.uri.parse::<Url>().unwrap(),
                                                ))?
                                                .unwrap(),
                                        );
                                    let response = request.send()?;
                                    response.bytes()?.to_vec()
                                } else {
                                    vec![]
                                },
                                iv: key.iv.clone(),
                                method: key.method.clone(),
                            });
                        }
                        KeyMethod::Cenc | KeyMethod::SampleAes => {
                            let mut decryption_keys = HashMap::new();

                            if all_keys {
                                for key in &keys {
                                    if let Some(kid) = &key.0 {
                                        decryption_keys.insert(kid.to_owned(), key.1.to_owned());
                                    }
                                }
                            } else {
                                let default_kid = stream.default_kid();

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

                            if decryption_keys.len() == 0 {
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
                                ));
                            }

                            previous_key = Some(Keys::from_hex_keys(decryption_keys));
                        }
                        _ => previous_key = None,
                    }
                }
            }

            let mut request = client.get(
                segment.seg_url(
                    baseurl
                        .as_ref()
                        .unwrap_or(&stream.uri.parse::<Url>().unwrap()),
                )?,
            );

            if let Some((range, end)) = segment.map_range(previous_byterange_end) {
                request = request.header(RANGE, range);
                previous_byterange_end = end;
            }

            let thread_data = ThreadData {
                downloaded_bytes,
                index: i,
                keys: previous_key.clone(),
                map: previous_map.clone(),
                merger: merger.clone(),
                pb: pb.clone(),
                relative_size: Some(relative_size.iter().sum()),
                request,
                timer: timer.clone(),
                total_retries: retry_count,
            };

            if no_decrypt {
                previous_map = None;
            }

            pool.execute(move || {
                if let Err(e) = thread_data.perform() {
                    let _lock = thread_data.pb.lock().unwrap();
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
                temp_file.colorize("bold red")
            );
        }

        downloaded_bytes += merger.stored();

        pb.lock().unwrap().write(format!(
            " {} video stream successfully",
            "Downloaded".colorize("bold green")
        ));
    }

    println!();
    // TODO - ignore --outuput with --no-decrypt
    Ok(())
}

struct ThreadData {
    downloaded_bytes: usize,
    index: usize,
    keys: Option<Keys>,
    map: Option<Vec<u8>>,
    merger: Arc<Mutex<Merger>>,
    pb: Arc<Mutex<RichProgress>>,
    relative_size: Option<usize>,
    request: RequestBuilder,
    timer: Arc<Instant>,
    total_retries: u8,
}

impl ThreadData {
    fn perform(&self) -> Result<()> {
        let mut segment = self.map.clone().unwrap_or(vec![]);
        segment.append(&mut self.download_segment()?);

        if let Some(keys) = &self.keys {
            segment = keys.decrypt(segment)?;
        }

        let mut merger = self.merger.lock().unwrap();
        merger.write(self.index, &segment)?;
        merger.flush()?;

        self.notify(merger.stored(), merger.estimate())?;
        Ok(())
    }

    fn download_segment(&self) -> Result<Vec<u8>> {
        let fetch_segment = || -> Result<Vec<u8>, reqwest::Error> {
            Ok(self.request.try_clone().unwrap().send()?.bytes()?.to_vec())
        };

        let mut retries = 0;
        let data = loop {
            match fetch_segment() {
                Ok(bytes) => {
                    // Download Speed
                    let elapsed_time = self.timer.elapsed().as_secs() as usize;
                    if elapsed_time != 0 {
                        let stored = self.merger.lock().unwrap().stored() + bytes.len();
                        self.pb.lock().unwrap().replace(
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
                    if self.total_retries > retries {
                        self.pb.lock().unwrap().write(check_reqwest_error(&e)?);
                        retries += 1;
                        continue;
                    } else {
                        bail!(
                            "reached maximum number of retries to download segment at index {}.",
                            self.index
                        )
                    }
                }
            }
        };

        Ok(data)
    }

    fn notify(&self, stored: usize, estimate: usize) -> Result<()> {
        let mut pb = self.pb.lock().unwrap();
        pb.replace(
            0,
            Column::Text(format!(
                "[bold blue]{}",
                utils::format_download_bytes(
                    self.downloaded_bytes + stored,
                    if let Some(size) = self.relative_size {
                        self.downloaded_bytes + size + estimate
                    } else {
                        self.downloaded_bytes + estimate
                    }
                ),
            )),
        );
        pb.update(1);
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

// use super::Stream;
// use anyhow::{bail, Result};
// use kdam::term::Colorizer;
// use serde::{Deserialize, Serialize};

// #[derive(Clone, Serialize, Deserialize)]
// pub struct Progress {
//     pub audio: Option<Stream>,
//     pub directory: Option<String>,
//     pub output: Option<String>,
//     pub subtitles: Option<Stream>,
//     pub video: Stream,
// }

// impl Progress {
//     pub fn mux(&self) -> Result<()> {
//         if let Some(output) = &self.output {
//             let mut args = vec!["-i".to_owned(), self.video.path(&self.directory)];

//             // args.push("-metadata".to_owned());
//             // args.push(format!("title=\"{}\"", self.video.url));

//             // if let StreamData {
//             //     language: Some(language),
//             //     ..
//             // } = &self.video
//             // {
//             //     args.push("-metadata".to_owned());
//             //     args.push(format!("language={}", language));
//             // }

//             if let Some(audio) = &self.audio {
//                 args.push("-i".to_owned());
//                 args.push(audio.path(&self.directory));
//             }

//             if let Some(subtitles) = &self.subtitles {
//                 args.push("-i".to_owned());
//                 args.push(subtitles.path(&self.directory));
//             }

//             args.push("-c:v".to_owned());
//             args.push("copy".to_owned());
//             args.push("-c:a".to_owned());
//             args.push("copy".to_owned());

//             if self.subtitles.is_some() {
//                 args.push("-scodec".to_owned());

//                 if output.ends_with(".mp4") {
//                     args.push("mov_text".to_owned());
//                 } else {
//                     args.push("srt".to_owned());
//                 }
//             }

//             // args.push("-metadata".to_owned());
//             // args.push(format!("title=\"{}\"", self.video.url));

//             if let Some(Stream {
//                 language: Some(language),
//                 ..
//             }) = &self.audio
//             {
//                 args.push("-metadata:s:a:0".to_owned());
//                 args.push(format!("language={}", language));
//             }

//             if let Some(Stream {
//                 language: Some(language),
//                 ..
//             }) = &self.subtitles
//             {
//                 args.push("-metadata:s:s:0".to_owned());
//                 args.push(format!("language={}", language));
//                 args.push("-disposition:s:0".to_owned());
//                 args.push("default".to_owned());
//             }

//             args.push(output.to_owned());

//             println!(
//                 "Executing {} {}",
//                 "ffmpeg".colorize("cyan"),
//                 args.join(" ").colorize("cyan")
//             );

//             if std::path::Path::new(output).exists() {
//                 std::fs::remove_file(output)?;
//             }

//             let code = std::process::Command::new("ffmpeg")
//                 .args(args)
//                 .stderr(std::process::Stdio::null())
//                 .spawn()?
//                 .wait()?;

//             if !code.success() {
//                 bail!("FFMPEG exited with code {}", code.code().unwrap_or(1))
//             }

//             if let Some(audio) = &self.audio {
//                 std::fs::remove_file(&audio.path(&self.directory))?;
//             }

//             if let Some(subtitles) = &self.subtitles {
//                 std::fs::remove_file(&subtitles.path(&self.directory))?;
//             }

//             std::fs::remove_file(&self.video.path(&self.directory))?;
//         }

//         Ok(())
//     }
// }
