use anyhow::Result;
use colored::Colorize;
use log::info;
use reqwest::{
    Client, Url,
    header::{self, HeaderValue},
};
use serde::Serialize;
use std::{collections::HashMap, fmt::Display, path::PathBuf, sync::Arc};

use crate::{automation::SelectOptions, progress::ByteSize, stream_selector::StreamSelector};

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
    Cenc,
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
                info!("{:>2}) {}", i + 1, stream);
            }
        }

        info!("{}", "------- Audio Streams --------".cyan());

        for (i, stream) in self.streams.iter().enumerate() {
            if stream.media_type == MediaType::Audio {
                info!("{:>2}) {}", i + 1, stream);
            }
        }

        info!("{}", "------ Subtitle Streams ------".cyan());

        for (i, stream) in self.streams.iter().enumerate() {
            if stream.media_type == MediaType::Subtitles {
                info!("{:>2}) {}", i + 1, stream);
            }
        }

        info!("{}", "------------------------------".cyan());
    }

    pub fn select_streams(self, select_opts: &mut SelectOptions) -> Result<Vec<MediaPlaylist>> {
        StreamSelector::new(self.streams).select(select_opts)
    }
}

impl Display for MediaPlaylist {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self.media_type {
            MediaType::Audio => {
                let mut msg = format!(
                    "{:>9}",
                    truncate(self.language.as_deref().unwrap_or("?"), 9)
                );

                if let Some(bandwidth) = self.bandwidth {
                    msg += &format!(" | {:>9}", ByteSize(bandwidth as usize).to_string());
                } else {
                    msg += &format!(" | {:>9}", "?");
                }

                msg += &format!(
                    " | {:>10}",
                    truncate(self.codecs.as_deref().unwrap_or("?"), 10)
                );

                if let Some(channels) = self.channels {
                    msg += &format!(" | {channels} ch");
                } else {
                    msg += " | ? ch";
                }

                if self.live {
                    msg += " | live";
                }

                msg
            }
            MediaType::Subtitles => format!(
                "{:>9} | {:>9} | {:>10}",
                truncate(self.language.as_deref().unwrap_or("?"), 9),
                "?KiB",
                truncate(self.codecs.as_deref().unwrap_or("?"), 10)
            ),
            MediaType::Undefined => format!("{:>9} | {:>9} | {:>10}", "?", "?", "?"),
            MediaType::Video => {
                let mut msg = format!(
                    "{:>9}",
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
                    msg += &format!(" | {:>9}", ByteSize(bandwidth as usize).to_string());
                } else {
                    msg += &format!(" | {:>9}", "?");
                }

                msg += &format!(
                    " | {:>10}",
                    truncate(self.codecs.as_deref().unwrap_or("?"), 10)
                );

                if let Some(frame_rate) = self.frame_rate {
                    msg += &format!(" | {frame_rate} fps");
                } else {
                    msg += " | ? fps";
                }

                if self.live {
                    msg += " | live";
                }

                if self.i_frame {
                    msg += " | iframe";
                }

                msg
            }
        };

        write!(f, "{}", msg)
    }
}

impl MediaPlaylist {
    pub fn display(&self) -> String {
        self.to_string()
            .split('|')
            .map(|x| x.replace(" ", ""))
            .collect::<Vec<String>>()
            .join(" ")
    }

    pub async fn init_seg(
        &self,
        base_url: &Url,
        client: &Client,
        query: &HashMap<String, String>,
    ) -> Result<Option<Arc<Vec<u8>>>> {
        if let Some(Segment { map: Some(map), .. }) = self.segments.first() {
            let url = base_url.join(&map.uri)?;
            let mut request = client.get(url).query(query);

            if let Some(range) = &map.range {
                request = request.header(header::RANGE, range.as_header_value());
            }

            let response = request.send().await?;
            let bytes = response.bytes().await?;
            return Ok(Some(Arc::new(bytes.to_vec())));
        }

        Ok(None)
    }

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
            MediaType::Audio => "vsd-aud",
            MediaType::Subtitles => "vsd-sub",
            MediaType::Undefined => "vsd-und",
            MediaType::Video => "vsd-vid",
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

fn truncate(s: &str, width: usize) -> String {
    if s.chars().count() > width {
        let mut truncated = s.chars().take(width - 1).collect::<String>();
        truncated.push('…');
        truncated
    } else {
        s.to_owned()
    }
}
