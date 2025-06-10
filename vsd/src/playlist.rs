/*
    REFERENCES
    ----------

    1.https://en.wikipedia.org/wiki/Filename#Reserved_characters_and_words
    2. https://en.wikipedia.org/wiki/Comparison_of_file_systems#Limits

*/

use anyhow::{Ok, Result, anyhow, bail};
use kdam::term::Colorizer;
use requestty::prompt::style::Stylize;
use reqwest::{
    Url,
    blocking::Client,
    header::{self, HeaderValue},
};
use serde::Serialize;
use std::{collections::HashMap, ffi::OsStr, fmt::Display, io::Write, path::PathBuf};

#[derive(Serialize)]
pub struct MasterPlaylist {
    pub playlist_type: PlaylistType,
    pub uri: String,
    pub streams: Vec<MediaPlaylist>,
}

impl MasterPlaylist {
    pub fn sort_streams(mut self) -> Self {
        let mut video_streams = vec![];
        let mut audio_streams = vec![];
        let subtitle_streams = vec![];
        let mut undefined_streams = vec![];

        for stream in self.streams {
            match stream.media_type {
                MediaType::Audio => {
                    let bandwidth = stream.bandwidth.unwrap_or(0);
                    let channels = stream.channels.unwrap_or(0.0);
                    audio_streams.push((stream, bandwidth, channels));
                }
                MediaType::Subtitles => undefined_streams.push(stream),
                MediaType::Undefined => undefined_streams.push(stream),
                MediaType::Video => {
                    let bandwidth = stream.bandwidth.unwrap_or(0);
                    let pixels = if let Some((w, h)) = &stream.resolution {
                        w * h
                    } else {
                        0
                    };
                    video_streams.push((stream, bandwidth, pixels));
                }
            }
        }

        video_streams.sort_by(|x, y| y.1.cmp(&x.1));
        video_streams.sort_by(|x, y| y.2.cmp(&x.2));
        audio_streams.sort_by(|x, y| y.1.cmp(&x.1));
        audio_streams.sort_by(|x, y| y.2.total_cmp(&x.2));

        self.streams = video_streams
            .into_iter()
            .map(|x| x.0)
            .chain(audio_streams.into_iter().map(|x| x.0))
            .chain(subtitle_streams)
            .chain(undefined_streams)
            .collect::<Vec<_>>();

        self
    }

    pub fn list_streams(&self) {
        println!("{}", "------- Video Streams --------".colorize("cyan"));

        for (i, stream) in self.streams.iter().enumerate() {
            if stream.media_type == MediaType::Video {
                println!("{:>2}) {}", i + 1, stream.display_video_stream());
            }
        }

        println!("{}", "------- Audio Streams --------".colorize("cyan"));

        for (i, stream) in self.streams.iter().enumerate() {
            if stream.media_type == MediaType::Audio {
                println!("{:>2}) {}", i + 1, stream.display_audio_stream());
            }
        }

        println!("{}", "------ Subtitle Streams ------".colorize("cyan"));

        for (i, stream) in self.streams.iter().enumerate() {
            if stream.media_type == MediaType::Subtitles {
                println!("{:>2}) {}", i + 1, stream.display_subtitle_stream());
            }
        }

        println!("{}", "------------------------------".colorize("cyan"));
    }

    pub fn select_streams(self, auto_opts: &AutomationOptions) -> Result<Vec<MediaPlaylist>> {
        if !auto_opts.interactive && !auto_opts.interactive_raw {
            for stream in &self.streams {
                println!(
                    "     {} [{:>5}] {}",
                    "Stream".colorize("cyan"),
                    stream.media_type.to_string(),
                    stream.display_stream()
                );
            }
        }

        let mut video_streams = vec![];
        let mut audio_streams = vec![];
        let mut subtitle_streams = vec![];
        // TODO - Add support for downloading undefined streams
        let mut undefined_streams = vec![];

        for stream in self.streams.into_iter().enumerate() {
            match stream.1.media_type {
                MediaType::Audio => audio_streams.push(stream),
                MediaType::Subtitles => subtitle_streams.push(stream),
                MediaType::Undefined => undefined_streams.push(stream),
                MediaType::Video => video_streams.push(stream),
            }
        }

        let mut select_streams = auto_opts.select_streams.clone();

        if select_streams.is_empty() {
            if !auto_opts.skip_video {
                if auto_opts.all_streams {
                    for (i, _) in &video_streams {
                        select_streams.push(*i);
                    }
                } else {
                    match &auto_opts.quality {
                        Quality::Lowest => {
                            select_streams.push(video_streams.len() - 1);
                        }
                        Quality::Highest => {
                            select_streams.push(0);
                        }
                        _ => {
                            let mut has_resolution = None;
                            let mut has_height = None;

                            let (w, h) = match &auto_opts.quality {
                                Quality::Resolution(w, h) => (*w as u64, *h as u64),
                                Quality::Youtube144p => (256, 144),
                                Quality::Youtube240p => (426, 240),
                                Quality::Youtube360p => (640, 360),
                                Quality::Youtube480p => (854, 480),
                                Quality::Youtube720p => (1280, 720),
                                Quality::Youtube1080p => (1920, 1080),
                                Quality::Youtube2k => (2048, 1080),
                                Quality::Youtube1440p => (2560, 1440),
                                Quality::Youtube4k => (3840, 2160),
                                Quality::Youtube8k => (7680, 4320),
                                _ => unreachable!(),
                            };

                            for (i, stream) in &video_streams {
                                if has_resolution.is_some() && has_height.is_some() {
                                    break;
                                }

                                if let Some((video_w, video_h)) = &stream.resolution {
                                    if h == *video_h {
                                        has_height = Some(i);

                                        if w == *video_w {
                                            has_resolution = Some(i);
                                        }
                                    }
                                }
                            }

                            if let Some(i) = has_resolution.or(has_height) {
                                select_streams.push(*i);
                            }
                        }
                    }
                }
            }

            if !auto_opts.skip_audio {
                if auto_opts.all_streams {
                    for (i, _) in &audio_streams {
                        select_streams.push(*i);
                    }
                } else {
                    if auto_opts.audio_lang.is_empty() {
                        if let Some(stream) = audio_streams.first() {
                            select_streams.push(stream.0);
                        }
                    } else {
                        for stream in &audio_streams {
                            if let Some(lang) = &stream.1.language {
                                if auto_opts.audio_lang.iter().any(|x| {
                                    x.to_lowercase().get(0..2) == lang.to_lowercase().get(0..2)
                                }) {
                                    select_streams.push(stream.0);
                                }
                            }
                        }
                    }
                }
            }

            if !auto_opts.skip_subs {
                if auto_opts.all_streams {
                    for (i, _) in &subtitle_streams {
                        select_streams.push(*i);
                    }
                } else {
                    if auto_opts.subs_lang.is_empty() {
                        if let Some(stream) = subtitle_streams.first() {
                            select_streams.push(stream.0);
                        }
                    } else {
                        for stream in &subtitle_streams {
                            if let Some(lang) = &stream.1.language {
                                if auto_opts.subs_lang.iter().any(|x| {
                                    x.to_lowercase().get(0..2) == lang.to_lowercase().get(0..2)
                                }) {
                                    select_streams.push(stream.0);
                                }
                            }
                        }
                    }
                }
            }
        } else {
            // BUG - Panics when user inputs 0
            select_streams = select_streams.iter().map(|x| x - 1).collect();
        }

        let mut choices_with_default = vec![];
        let mut choices_with_default_ranges: [std::ops::Range<usize>; 4] =
            [(0..0), (0..0), (0..0), (0..0)];

        choices_with_default.push(requestty::Separator(
            "─────── Video Streams ────────".to_owned(),
        ));
        choices_with_default.extend(video_streams.iter().map(|(i, x)| {
            requestty::Choice((x.display_video_stream(), select_streams.contains(i)))
        }));
        choices_with_default_ranges[0] = 1..choices_with_default.len();
        choices_with_default.push(requestty::Separator(
            "─────── Audio Streams ────────".to_owned(),
        ));
        choices_with_default.extend(audio_streams.iter().map(|(i, x)| {
            requestty::Choice((x.display_audio_stream(), select_streams.contains(i)))
        }));

        if auto_opts.interactive {
            choices_with_default_ranges[1] =
                (choices_with_default_ranges[0].end + 1)..choices_with_default.len();
        } else {
            choices_with_default_ranges[1] =
                choices_with_default_ranges[0].end..(choices_with_default.len() - 1);
        }

        choices_with_default.push(requestty::Separator(
            "────── Subtitle Streams ──────".to_owned(),
        ));
        choices_with_default.extend(subtitle_streams.iter().map(|(i, x)| {
            requestty::Choice((x.display_subtitle_stream(), select_streams.contains(i)))
        }));

        if auto_opts.interactive {
            choices_with_default_ranges[2] =
                (choices_with_default_ranges[1].end + 1)..choices_with_default.len();
        } else {
            choices_with_default_ranges[2] =
                choices_with_default_ranges[1].end..(choices_with_default.len() - 2);
        }

        if auto_opts.interactive {
            let question = requestty::Question::multi_select("streams")
                .should_loop(false)
                .message("Select streams to download")
                .choices_with_default(choices_with_default)
                .transform(|choices, _, backend| {
                    backend.write_styled(
                        &choices
                            .iter()
                            .map(|x| x.text.split_whitespace().collect::<Vec<_>>().join(" "))
                            .collect::<Vec<_>>()
                            .join(" | ")
                            .cyan(),
                    )
                })
                .build();

            let answer = requestty::prompt_one(question)?;

            let mut selected_streams = vec![];
            let mut video_streams_offset = 1;
            let mut audio_streams_offset = video_streams_offset + video_streams.len() + 1;
            let mut subtitle_streams_offset = audio_streams_offset + audio_streams.len() + 1;

            for selected_item in answer.as_list_items().unwrap() {
                if choices_with_default_ranges[0].contains(&selected_item.index) {
                    selected_streams.push(
                        video_streams
                            .remove(selected_item.index - video_streams_offset)
                            .1,
                    );
                    video_streams_offset += 1;
                } else if choices_with_default_ranges[1].contains(&selected_item.index) {
                    selected_streams.push(
                        audio_streams
                            .remove(selected_item.index - audio_streams_offset)
                            .1,
                    );
                    audio_streams_offset += 1;
                } else if choices_with_default_ranges[2].contains(&selected_item.index) {
                    selected_streams.push(
                        subtitle_streams
                            .remove(selected_item.index - subtitle_streams_offset)
                            .1,
                    );
                    subtitle_streams_offset += 1;
                }
            }

            Ok(selected_streams)
        } else {
            if auto_opts.interactive_raw {
                println!("Select streams to download:");
            }

            let mut selected_choices_index = vec![];
            let mut index = 1;

            for choice in choices_with_default {
                if let requestty::Separator(seperator) = choice {
                    if auto_opts.interactive_raw {
                        println!("{}", seperator.replace('─', "-").colorize("cyan"));
                    }
                } else {
                    let (message, selected) = choice.unwrap_choice();

                    if selected {
                        selected_choices_index.push(index);
                    }

                    if auto_opts.interactive_raw {
                        println!(
                            "{:2}) [{}] {}",
                            index,
                            if selected {
                                "x".colorize("green")
                            } else {
                                " ".to_owned()
                            },
                            message
                        );
                    }
                    index += 1;
                }
            }

            if auto_opts.interactive_raw {
                println!("{}", "------------------------------".colorize("cyan"));
                print!(
                    "Press enter to proceed with defaults.\n\
                        Or select streams to download (1, 2, etc.): "
                );
                std::io::stdout().flush()?;
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;

                println!("{}", "------------------------------".colorize("cyan"));

                let input = input.trim();

                if !input.is_empty() {
                    selected_choices_index = input
                        .split(',')
                        .filter_map(|x| x.trim().parse::<usize>().ok())
                        .collect::<Vec<usize>>();
                }
            }

            let mut selected_streams = vec![];
            let mut video_streams_offset = 1;
            let mut audio_streams_offset = video_streams_offset + video_streams.len();
            let mut subtitle_streams_offset = audio_streams_offset + audio_streams.len();

            for i in selected_choices_index {
                if choices_with_default_ranges[0].contains(&i) {
                    let stream = video_streams.remove(i - video_streams_offset).1;
                    println!(
                        "   {} [{:>5}] {}",
                        "Selected".colorize("bold green"),
                        stream.media_type.to_string(),
                        stream.display_stream()
                    );
                    selected_streams.push(stream);
                    video_streams_offset += 1;
                } else if choices_with_default_ranges[1].contains(&i) {
                    let stream = audio_streams.remove(i - audio_streams_offset).1;
                    println!(
                        "   {} [{:>5}] {}",
                        "Selected".colorize("bold green"),
                        stream.media_type.to_string(),
                        stream.display_stream()
                    );
                    selected_streams.push(stream);
                    audio_streams_offset += 1;
                } else if choices_with_default_ranges[2].contains(&i) {
                    let stream = subtitle_streams.remove(i - subtitle_streams_offset).1;
                    println!(
                        "   {} [{:>5}] {}",
                        "Selected".colorize("bold green"),
                        stream.media_type.to_string(),
                        stream.display_stream()
                    );
                    selected_streams.push(stream);
                    subtitle_streams_offset += 1;
                }
            }

            Ok(selected_streams)
        }
    }
}

#[derive(Default, Serialize)]
pub struct MediaPlaylist {
    pub bandwidth: Option<u64>,
    pub channels: Option<f32>,
    pub codecs: Option<String>,
    pub extension: Option<String>,
    pub frame_rate: Option<f32>,
    pub id: String,
    pub i_frame: bool,
    pub language: Option<String>,
    pub live: bool,
    pub media_sequence: u64,
    pub media_type: MediaType,
    pub playlist_type: PlaylistType,
    pub resolution: Option<(u64, u64)>,
    pub segments: Vec<Segment>,
    pub uri: String,
}

impl MediaPlaylist {
    // pub fn is_hls(&self) -> bool {
    //     matches!(&self.playlist_type, PlaylistType::Hls)
    // }

    pub fn default_kid(&self) -> Option<String> {
        if let Some(segment) = self.segments.first() {
            if let Some(Key {
                default_kid: Some(x),
                ..
            }) = &segment.key
            {
                return Some(x.replace('-', "").to_lowercase());
            }
        }

        None
    }

    pub fn extension(&self) -> &OsStr {
        if let Some(ext) = &self.extension {
            return OsStr::new(ext);
        }

        let mut ext = match &self.playlist_type {
            PlaylistType::Hls => "ts",
            PlaylistType::Dash => "m4s",
        };

        if let Some(segment) = self.segments.first() {
            if let Some(init) = &segment.map {
                if init.uri.ends_with(".mp4") {
                    ext = "mp4";
                }
            }

            if segment.uri.ends_with(".mp4") {
                ext = "mp4";
            }
        }

        OsStr::new(ext)
    }

    pub fn file_path(&self, directory: Option<&PathBuf>, ext: &OsStr) -> PathBuf {
        let prefix = match &self.media_type {
            MediaType::Audio => "vsd-audio",
            MediaType::Subtitles => "vsd-subtitles",
            MediaType::Undefined => "vsd-undefined",
            MediaType::Video => "vsd-video",
        };

        let mut path = PathBuf::from(format!("{}-{}.{}", prefix, self.id, ext.to_string_lossy()));

        if let Some(directory) = directory {
            path = directory.join(path);
        }

        if path.exists() {
            for i in 1.. {
                path.set_file_name(format!(
                    "{}-{}-{}.{}",
                    prefix,
                    self.id,
                    i,
                    ext.to_string_lossy()
                ));

                if !path.exists() {
                    return path;
                }
            }
        }

        path
    }

    pub fn display_stream(&self) -> String {
        match self.media_type {
            MediaType::Audio => self.display_audio_stream(),
            MediaType::Subtitles => self.display_subtitle_stream(),
            MediaType::Undefined => "".to_owned(),
            MediaType::Video => self.display_video_stream(),
        }
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
    }

    fn display_video_stream(&self) -> String {
        let resolution = if let Some((w, h)) = self.resolution {
            match (w, h) {
                (256, 144) => "144p".to_owned(),
                (426, 240) => "240p".to_owned(),
                (640, 360) => "360p".to_owned(),
                (854, 480) => "480p".to_owned(),
                (1280, 720) => "720p".to_owned(),
                (1920, 1080) => "1080p".to_owned(),
                (2048, 1080) => "2K".to_owned(),
                (2560, 1440) => "1440p".to_owned(),
                (3840, 2160) => "4K".to_owned(),
                (7680, 4320) => "8K".to_owned(),
                (w, h) => format!("{}x{}", w, h),
            }
        } else {
            "?".to_owned()
        };

        let bandwidth = if let Some(bandwidth) = self.bandwidth {
            crate::utils::format_bytes(bandwidth as usize, 2)
        } else {
            ("?".to_owned(), "?".to_owned(), "?".to_owned())
        };

        let mut extra = format!(
            "(codecs: {}",
            self.codecs.as_ref().unwrap_or(&"?".to_owned())
        );

        if let Some(frame_rate) = self.frame_rate {
            extra += &format!(", frame_rate: {}", frame_rate);
        }

        if self.i_frame {
            extra += ", iframe";
        }

        if self.live {
            extra += ", live";
        }

        extra += ")";

        format!(
            "{:9} {:>7} {}/s {}",
            resolution, bandwidth.0, bandwidth.1, extra
        )
    }

    fn display_audio_stream(&self) -> String {
        let mut extra = format!(
            "language: {}",
            self.language.as_ref().unwrap_or(&"?".to_owned())
        );

        if let Some(codecs) = &self.codecs {
            extra += &format!(", codecs: {}", codecs);
        }

        if let Some(bandwidth) = self.bandwidth {
            extra += &format!(
                ", bandwidth: {}/s",
                crate::utils::format_bytes(bandwidth as usize, 2).2
            );
        }

        if let Some(channels) = self.channels {
            extra += &format!(", channels: {}", channels);
        }

        if self.live {
            extra += ", live";
        }

        extra
    }

    pub fn display_subtitle_stream(&self) -> String {
        let mut extra = format!(
            "language: {}",
            self.language.as_ref().unwrap_or(&"?".to_owned())
        );

        if let Some(codecs) = &self.codecs {
            extra += &format!(", codecs: {}", codecs);
        }

        extra
    }

    pub fn estimate_size(
        &self,
        base_url: &Option<Url>,
        client: &Client,
        query: &HashMap<String, String>,
    ) -> Result<usize> {
        let base_url = base_url.clone().unwrap_or(self.uri.parse::<Url>().unwrap());
        let total_segments = self.segments.len();

        if let Some(segment) = self.segments.first() {
            let url = base_url.join(&segment.uri)?;
            let mut request = client.head(url.clone()).query(query);

            if total_segments > 1 {
                if let Some(range) = &segment.range {
                    request = request.header(header::RANGE, range.as_header_value());
                }
            }

            let response = request.send()?;
            let content_length = response
                .headers()
                .get(header::CONTENT_LENGTH)
                .unwrap()
                .to_str()
                .unwrap()
                .parse::<usize>()
                .unwrap();

            if total_segments > 1 {
                return Ok(total_segments * content_length);
            } else {
                return Ok(content_length);
            }
        }

        Ok(0)
    }

    pub fn split_segment(
        &mut self,
        base_url: &Option<Url>,
        client: &Client,
        query: &HashMap<String, String>,
    ) -> Result<()> {
        if self.segments.len() > 1 {
            return Ok(());
        }

        let base_url = base_url.clone().unwrap_or(self.uri.parse::<Url>().unwrap());
        let segment = self.segments.remove(0);
        let url = base_url.join(&segment.uri)?;
        let response = client.head(url).query(query).send()?;
        let content_length = response
            .headers()
            .get(header::CONTENT_LENGTH)
            .unwrap()
            .to_str()
            .unwrap()
            .parse::<usize>()
            .unwrap();

        let ranges = PartialRangeIter {
            end: content_length as u64 - 1, // content_length should never be 0.
            start: 0,
        };

        for (i, range) in ranges.enumerate() {
            if i == 0 {
                let mut segment_copy = segment.clone();
                segment_copy.range = Some(range);
                self.segments.push(segment_copy);
            } else {
                self.segments.push(Segment {
                    duration: segment.duration,
                    range: Some(range),
                    uri: segment.uri.clone(),
                    ..Default::default()
                });
            }
        }

        Ok(())
    }
}

#[derive(Default, Serialize)]
pub enum PlaylistType {
    Dash,
    #[default]
    Hls,
}

#[derive(Clone, Default, PartialEq, Serialize)]
pub enum MediaType {
    Audio,
    Subtitles,
    #[default]
    Undefined,
    Video,
}

impl Display for MediaType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Audio => "audio",
                Self::Subtitles => "subs",
                Self::Undefined => "und",
                Self::Video => "video",
            }
        )
    }
}

#[derive(Clone, PartialEq, Serialize)]
pub enum KeyMethod {
    Aes128,
    Mp4Decrypt,
    None,
    Other(String),
    SampleAes,
}

#[derive(Clone, Serialize)]
pub struct Range {
    pub start: u64,
    pub end: u64,
}

impl Range {
    pub fn as_header_value(&self) -> HeaderValue {
        HeaderValue::from_str(&format!("bytes={}-{}", self.start, self.end)).unwrap()
    }
}

#[derive(Clone, Serialize)]
pub struct Map {
    pub uri: String,
    pub range: Option<Range>,
}

#[derive(Clone, Serialize)]
pub struct Key {
    pub default_kid: Option<String>,
    pub iv: Option<String>,
    pub key_format: Option<String>,
    pub method: KeyMethod,
    pub uri: Option<String>,
}

impl Key {
    pub fn key(&self, bytes: &[u8]) -> Result<[u8; 16]> {
        if bytes.len() != 16 {
            bail!("invalid key size.");
        }

        let mut key = [0_u8; 16];
        key.copy_from_slice(bytes);
        Ok(key)
    }

    pub fn iv(&self, sequence: u64) -> Result<[u8; 16]> {
        Ok(if let Some(actual_iv) = self.iv.as_ref() {
            let iv = if let Some(stripped_iv) = actual_iv.strip_prefix("0x") {
                stripped_iv
            } else {
                actual_iv
            };
            u128::from_str_radix(iv, 16)
                .map_err(|_| anyhow!("invalid iv size."))?
                .to_be_bytes()
        } else {
            (sequence as u128).to_be_bytes()
        })
    }
}

#[derive(Clone, Default, Serialize)]
pub struct Segment {
    pub range: Option<Range>,
    pub duration: f32, // consider changing it to f64
    pub key: Option<Key>,
    pub map: Option<Map>,
    pub uri: String,
}

const BUFFER_SIZE: u64 = 1024 * 1024 * 2; // 2 MiB

/// https://rust-lang-nursery.github.io/rust-cookbook/web/clients/download.html#make-a-partial-download-with-http-range-headers
struct PartialRangeIter {
    end: u64,
    start: u64,
}

impl Iterator for PartialRangeIter {
    type Item = Range;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start > self.end {
            None
        } else {
            let prev_start = self.start;
            self.start += std::cmp::min(BUFFER_SIZE, self.end - self.start + 1);
            Some(Range {
                start: prev_start,
                end: self.start - 1,
            })
        }
    }
}

#[derive(Debug, Clone)]
pub enum Quality {
    Lowest,
    Highest,
    Resolution(u16, u16),
    Youtube144p,
    Youtube240p,
    Youtube360p,
    Youtube480p,
    Youtube720p,
    Youtube1080p,
    Youtube2k,
    Youtube1440p,
    Youtube4k,
    Youtube8k,
}

pub struct AutomationOptions {
    pub all_streams: bool,
    pub audio_lang: Vec<String>,
    pub interactive: bool,
    pub interactive_raw: bool,
    pub select_streams: Vec<usize>,
    pub skip_audio: bool,
    pub skip_video: bool,
    pub skip_subs: bool,
    pub subs_lang: Vec<String>,
    pub quality: Quality,
}
