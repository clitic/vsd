use anyhow::Result;
use colored::Colorize;
use log::info;
use reqwest::{
    Client, Url,
    header::{self, HeaderValue},
};
use serde::Serialize;
use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    io::Write,
    path::PathBuf,
};

use crate::{
    automation::{self, InteractionType, SelectOptions, VideoPreference},
    progress::ByteSize,
};

#[derive(Serialize)]
pub struct MasterPlaylist {
    pub playlist_type: PlaylistType,
    pub uri: String,
    pub streams: Vec<MediaPlaylist>,
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

#[derive(Clone, Default, Serialize)]
pub struct Segment {
    pub range: Option<Range>,
    pub duration: f32, // consider changing it to f64
    pub key: Option<Key>,
    pub map: Option<Map>,
    pub uri: String,
}

#[derive(Clone, Serialize)]
pub struct Range {
    pub end: u64,
    pub start: u64,
}

#[derive(Clone, Serialize)]
pub struct Key {
    pub default_kid: Option<String>,
    pub iv: Option<String>,
    pub key_format: Option<String>,
    pub method: KeyMethod,
    pub uri: Option<String>,
}

#[derive(Clone, PartialEq, Serialize)]
pub enum KeyMethod {
    Aes128,
    CencCbcs,
    None,
    Other(String),
    SampleAes,
}

#[derive(Clone, Serialize)]
pub struct Map {
    pub uri: String,
    pub range: Option<Range>,
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
        info!("{}", "------- Video Streams --------".cyan());

        for (i, stream) in self.streams.iter().enumerate() {
            if stream.media_type == MediaType::Video {
                info!("{:>2}) {}", i + 1, stream.display_video_stream());
            }
        }

        info!("{}", "------- Audio Streams --------".cyan());

        for (i, stream) in self.streams.iter().enumerate() {
            if stream.media_type == MediaType::Audio {
                info!("{:>2}) {}", i + 1, stream.display_audio_stream());
            }
        }

        info!("{}", "------ Subtitle Streams ------".cyan());

        for (i, stream) in self.streams.iter().enumerate() {
            if stream.media_type == MediaType::Subtitles {
                info!("{:>2}) {}", i + 1, stream.display_subs_stream());
            }
        }

        info!("{}", "------------------------------".cyan());
    }

    pub fn select_streams(self, select_opts: &mut SelectOptions) -> Result<Vec<MediaPlaylist>> {
        let interactions = automation::load_interaction_type();

        if let InteractionType::None = interactions {
            for stream in &self.streams {
                info!(
                    "Found {} stream: {}",
                    stream.media_type,
                    stream.display_stream().dimmed(),
                );
            }
        }

        let mut video_streams = vec![];
        let mut audio_streams = vec![];
        let mut sub_streams = vec![];
        // TODO - Add support for downloading undefined streams
        let mut undefined_streams = vec![];

        for stream in self.streams.into_iter().enumerate() {
            match stream.1.media_type {
                MediaType::Audio => audio_streams.push(stream),
                MediaType::Subtitles => sub_streams.push(stream),
                MediaType::Undefined => undefined_streams.push(stream),
                MediaType::Video => video_streams.push(stream),
            }
        }

        let mut selected_streams = HashSet::new();

        if select_opts.video.all {
            for (i, _) in &video_streams {
                selected_streams.insert(*i);
            }
        } else {
            let mut selected_vstreams = HashSet::new();

            for (i, _) in &video_streams {
                if select_opts.stream_numbers.iter().any(|x| (*x - 1) == *i) {
                    selected_vstreams.insert(*i);
                }
            }

            match &select_opts.video.preference {
                VideoPreference::Best => {
                    if let Some((i, _)) = video_streams.first() {
                        selected_vstreams.insert(*i);
                    }
                }
                VideoPreference::None => (),
                VideoPreference::Worst => {
                    if let Some((i, _)) = video_streams.last() {
                        selected_vstreams.insert(*i);
                    }
                }
            };

            for (i, stream) in &video_streams {
                if let Some((w, h)) = &stream.resolution
                    && select_opts
                        .video
                        .resolutions
                        .contains(&(*w as u16, *h as u16))
                {
                    selected_vstreams.insert(*i);
                }
            }

            if select_opts.video.skip && !selected_vstreams.is_empty() {
                for (i, _) in &video_streams {
                    if !selected_vstreams.contains(i) {
                        selected_streams.insert(*i);
                    }
                }
            } else if !select_opts.video.skip {
                if selected_vstreams.is_empty()
                    && let Some((i, _)) = video_streams.first()
                {
                    selected_vstreams.insert(*i);
                }

                for i in selected_vstreams {
                    selected_streams.insert(i);
                }
            }
        }

        if select_opts.audio.all {
            for (i, _) in &audio_streams {
                selected_streams.insert(*i);
            }
        } else {
            let mut selected_astreams = HashSet::new();

            for (i, _) in &audio_streams {
                if select_opts.stream_numbers.iter().any(|x| (*x - 1) == *i) {
                    selected_astreams.insert(*i);
                }
            }

            for (i, stream) in &audio_streams {
                if let Some(stream_lang) = &stream.language
                    && select_opts.audio.contains_exact_lang(stream_lang)
                {
                    selected_astreams.insert(*i);
                }
            }

            for (i, stream) in &audio_streams {
                if let Some(stream_lang) = &stream.language
                    && select_opts.audio.contains_siml_lang(stream_lang)
                {
                    selected_astreams.insert(*i);
                }
            }

            if select_opts.audio.skip && !selected_astreams.is_empty() {
                for (i, _) in &audio_streams {
                    if !selected_astreams.contains(i) {
                        selected_streams.insert(*i);
                    }
                }
            } else if !select_opts.audio.skip {
                if selected_astreams.is_empty()
                    && let Some((i, _)) = audio_streams.first()
                {
                    selected_astreams.insert(*i);
                }

                for i in selected_astreams {
                    selected_streams.insert(i);
                }
            }
        }

        if select_opts.subs.all {
            for (i, _) in &sub_streams {
                selected_streams.insert(*i);
            }
        } else {
            let mut selected_sstreams = HashSet::new();

            for (i, _) in &sub_streams {
                if select_opts.stream_numbers.iter().any(|x| (*x - 1) == *i) {
                    selected_sstreams.insert(*i);
                }
            }

            for (i, stream) in &sub_streams {
                if let Some(stream_lang) = &stream.language
                    && select_opts.subs.contains_exact_lang(stream_lang)
                {
                    selected_sstreams.insert(*i);
                }
            }

            for (i, stream) in &sub_streams {
                if let Some(stream_lang) = &stream.language
                    && select_opts.subs.contains_siml_lang(stream_lang)
                {
                    selected_sstreams.insert(*i);
                }
            }

            if select_opts.subs.skip && !selected_sstreams.is_empty() {
                for (i, _) in &sub_streams {
                    if !selected_sstreams.contains(i) {
                        selected_streams.insert(*i);
                    }
                }
            } else if !select_opts.subs.skip {
                if selected_sstreams.is_empty()
                    && let Some((i, _)) = sub_streams.first()
                {
                    selected_sstreams.insert(*i);
                }

                for i in selected_sstreams {
                    selected_streams.insert(i);
                }
            }
        }

        let mut choices_with_default = vec![];
        let mut choices_with_default_ranges: [std::ops::Range<usize>; 4] =
            [(0..0), (0..0), (0..0), (0..0)];

        choices_with_default.push(requestty::Separator(
            "─────── Video Streams ────────".to_owned(),
        ));
        choices_with_default.extend(video_streams.iter().map(|(i, x)| {
            requestty::Choice((x.display_video_stream(), selected_streams.contains(i)))
        }));
        choices_with_default_ranges[0] = 1..choices_with_default.len();
        choices_with_default.push(requestty::Separator(
            "─────── Audio Streams ────────".to_owned(),
        ));
        choices_with_default.extend(audio_streams.iter().map(|(i, x)| {
            requestty::Choice((x.display_audio_stream(), selected_streams.contains(i)))
        }));

        if let InteractionType::Modern = interactions {
            choices_with_default_ranges[1] =
                (choices_with_default_ranges[0].end + 1)..choices_with_default.len();
        } else {
            choices_with_default_ranges[1] =
                choices_with_default_ranges[0].end..(choices_with_default.len() - 1);
        }

        choices_with_default.push(requestty::Separator(
            "────── Subtitle Streams ──────".to_owned(),
        ));
        choices_with_default.extend(sub_streams.iter().map(|(i, x)| {
            requestty::Choice((x.display_subs_stream(), selected_streams.contains(i)))
        }));

        if let InteractionType::Modern = interactions {
            choices_with_default_ranges[2] =
                (choices_with_default_ranges[1].end + 1)..choices_with_default.len();
        } else {
            choices_with_default_ranges[2] =
                choices_with_default_ranges[1].end..(choices_with_default.len() - 2);
        }

        if let InteractionType::Modern = interactions {
            let question = requestty::Question::multi_select("streams")
                .should_loop(false)
                .message("Select streams to download")
                .choices_with_default(choices_with_default)
                .transform(|choices, _, backend| {
                    backend.write_styled(&requestty::prompt::style::Stylize::cyan(
                        &choices
                            .iter()
                            .map(|x| x.text.split_whitespace().collect::<Vec<_>>().join(" "))
                            .collect::<Vec<_>>()
                            .join(" | "),
                    ))
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
                        sub_streams
                            .remove(selected_item.index - subtitle_streams_offset)
                            .1,
                    );
                    subtitle_streams_offset += 1;
                }
            }

            Ok(selected_streams)
        } else {
            if let InteractionType::Raw = interactions {
                info!("Select streams to download:");
            }

            let mut selected_choices_index = vec![];
            let mut index = 1;

            for choice in choices_with_default {
                if let requestty::Separator(seperator) = choice {
                    if let InteractionType::Raw = interactions {
                        info!("{}", seperator.replace('─', "-").cyan());
                    }
                } else {
                    let (message, selected) = choice.unwrap_choice();

                    if selected {
                        selected_choices_index.push(index);
                    }

                    if let InteractionType::Raw = interactions {
                        info!(
                            "{:2}) [{}] {}",
                            index,
                            if selected { "x".green() } else { " ".normal() },
                            message
                        );
                    }
                    index += 1;
                }
            }

            if let InteractionType::Raw = interactions {
                info!("{}", "------------------------------".cyan());
                print!(
                    "Press enter to proceed with defaults.\n\
                        Or select streams to download (1, 2, etc.): "
                );
                std::io::stdout().flush()?;
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;

                info!("{}", "------------------------------".cyan());

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
                    info!(
                        "Selected {} stream: {}",
                        stream.media_type,
                        stream.display_stream().bold()
                    );
                    selected_streams.push(stream);
                    video_streams_offset += 1;
                } else if choices_with_default_ranges[1].contains(&i) {
                    let stream = audio_streams.remove(i - audio_streams_offset).1;
                    info!(
                        "Selected {} stream: {}",
                        stream.media_type,
                        stream.display_stream().bold()
                    );
                    selected_streams.push(stream);
                    audio_streams_offset += 1;
                } else if choices_with_default_ranges[2].contains(&i) {
                    let stream = sub_streams.remove(i - subtitle_streams_offset).1;
                    info!(
                        "Selected {} stream: {}",
                        stream.media_type,
                        stream.display_stream().bold()
                    );
                    selected_streams.push(stream);
                    subtitle_streams_offset += 1;
                }
            }

            Ok(selected_streams)
        }
    }
}

impl MediaPlaylist {
    pub fn default_kid(&self) -> Option<String> {
        if let Some(Segment {
            key: Some(Key {
                default_kid: Some(x),
                ..
            }),
            ..
        }) = self.segments.first()
        {
            return Some(x.to_ascii_lowercase().replace('-', ""));
        }

        None
    }

    fn display_audio_stream(&self) -> String {
        let mut extra = format!(
            "lang: {}",
            self.language.as_ref().unwrap_or(&"?".to_owned())
        );

        if let Some(bandwidth) = self.bandwidth {
            extra += &format!(", bandwidth: {}", ByteSize(bandwidth as usize));
        }

        if let Some(codecs) = &self.codecs {
            extra += &format!(", codecs: {codecs}");
        }

        if let Some(channels) = self.channels {
            extra += &format!(", channels: {channels}");
        }

        if self.live {
            extra += ", live";
        }

        extra
    }

    fn display_subs_stream(&self) -> String {
        let mut extra = format!(
            "lang: {}",
            self.language.as_ref().unwrap_or(&"?".to_owned())
        );

        if let Some(codecs) = &self.codecs {
            extra += &format!(", codecs: {codecs}");
        }

        extra
    }

    fn display_video_stream(&self) -> String {
        let mut extra = format!(
            "res: {}",
            if let Some((w, h)) = self.resolution {
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
                    (w, h) => format!("{w}x{h}"),
                }
            } else {
                "?".to_owned()
            }
        );

        if let Some(bandwidth) = self.bandwidth {
            extra += &format!(", bandwidth: {}", ByteSize(bandwidth as usize));
        } else {
            extra += "bandwidth: ?";
        }

        extra += &format!(
            ", codecs: {}",
            self.codecs.as_ref().unwrap_or(&"?".to_owned())
        );

        if let Some(frame_rate) = self.frame_rate {
            extra += &format!(", frame_rate: {frame_rate}");
        }

        if self.i_frame {
            extra += ", iframe";
        }

        if self.live {
            extra += ", live";
        }

        extra
    }

    pub fn display_stream(&self) -> String {
        match self.media_type {
            MediaType::Audio => self.display_audio_stream(),
            MediaType::Subtitles => self.display_subs_stream(),
            MediaType::Undefined => "".to_owned(),
            MediaType::Video => self.display_video_stream(),
        }
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
    }

    pub fn extension(&self) -> &str {
        if let Some(ext) = &self.extension {
            return ext;
        }

        let mut ext = match &self.playlist_type {
            PlaylistType::Hls => "ts",
            PlaylistType::Dash => "m4s",
        };

        if let Some(segment) = self.segments.first() {
            if let Some(init) = &segment.map
                && init.uri.ends_with(".mp4")
            {
                ext = "mp4";
            }

            if segment.uri.ends_with(".mp4") {
                ext = "mp4";
            }
        }

        ext
    }

    pub fn path(&self, directory: Option<&PathBuf>) -> PathBuf {
        let prefix = match &self.media_type {
            MediaType::Audio => "vsd-audio",
            MediaType::Subtitles => "vsd-subtitles",
            MediaType::Undefined => "vsd-undefined",
            MediaType::Video => "vsd-video",
        };

        let mut path = PathBuf::from(format!("{}-{}.{}", prefix, self.id, self.extension()));

        if let Some(directory) = directory {
            path = directory.join(path);
        }

        path
    }

    pub async fn split_segment(
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
        let response = client.head(url).query(query).send().await?;
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

impl Display for MediaType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Audio => "aud",
                Self::Subtitles => "sub",
                Self::Undefined => "und",
                Self::Video => "vid",
            }
        )
    }
}

impl Range {
    pub fn as_header_value(&self) -> HeaderValue {
        HeaderValue::from_str(&format!("bytes={}-{}", self.start, self.end)).unwrap()
    }
}

impl Key {
    pub async fn key(
        &self,
        base_url: &Url,
        client: &Client,
        query: &HashMap<String, String>,
    ) -> Result<[u8; 16]> {
        let url = base_url.join(self.uri.as_ref().unwrap())?;
        let request = client.get(url).query(query);
        let response = request.send().await?;
        let bytes = response.bytes().await?;
        Ok(bytes.as_ref().try_into()?)
    }

    pub fn iv(&self, sequence: u64) -> Result<[u8; 16]> {
        if let Some(iv) = self.iv.as_ref() {
            return Ok(u128::from_str_radix(
                iv.trim_start_matches("0x").trim_start_matches("0X"),
                16,
            )?
            .to_be_bytes());
        }

        Ok((sequence as u128).to_be_bytes())
    }
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
