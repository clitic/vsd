use anyhow::Result;
use colored::Colorize;
use log::info;
use reqwest::{
    Client, Url,
    header::{self, HeaderValue},
};
use serde::Serialize;
use std::{cmp::Reverse, collections::HashMap, fmt::Display, path::PathBuf, sync::Arc};

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
        let mut vid_streams = Vec::new();
        let mut aud_streams = Vec::new();
        let mut sub_streams = Vec::new();
        let mut und_streams = Vec::new();

        for stream in self.streams {
            match stream.media_type {
                MediaType::Video => vid_streams.push(stream),
                MediaType::Audio => aud_streams.push(stream),
                MediaType::Subtitles => sub_streams.push(stream),
                MediaType::Undefined => und_streams.push(stream),
            }
        }

        vid_streams.sort_by_key(|s| {
            let pixels = s.resolution.map_or(0, |(w, h)| w * h);
            let bandwidth = s.bandwidth.unwrap_or_default();
            Reverse((pixels, bandwidth))
        });

        aud_streams.sort_by_key(|s| {
            let channels = (s.channels.unwrap_or_default() * 10.0) as u32;
            let bandwidth = s.bandwidth.unwrap_or_default();
            Reverse((channels, bandwidth))
        });

        self.streams = vid_streams
            .into_iter()
            .chain(aud_streams)
            .chain(sub_streams)
            .chain(und_streams)
            .collect();

        self
    }

    pub fn list_streams(&self) {
        for (media_type, header) in [
            (MediaType::Video, "------- Video Streams --------"),
            (MediaType::Audio, "------- Audio Streams --------"),
            (MediaType::Subtitles, "------ Subtitle Streams ------"),
        ] {
            info!("{}", header.cyan());
            for (i, stream) in self.streams.iter().enumerate() {
                if stream.media_type == media_type {
                    info!("{:>2}) {}", i + 1, stream);
                }
            }
        }

        info!("{}", "------------------------------".cyan());
    }

    pub fn select_streams(self, select_opts: &mut SelectOptions) -> Result<Vec<MediaPlaylist>> {
        StreamSelector::new(self.streams).select(select_opts)
    }
}

impl MediaPlaylist {
    pub fn default_kid(&self) -> Option<String> {
        self.segments
            .first()
            .and_then(|s| s.key.as_ref())
            .and_then(|k| k.default_kid.as_ref())
            .map(|kid| kid.to_ascii_lowercase().replace('-', ""))
    }

    pub fn extension(&self) -> &str {
        if let Some(ext) = &self.extension {
            return ext;
        }

        if let Some(seg) = self.segments.first() {
            let is_mp4 = |uri: &str| uri.split('?').next().is_some_and(|p| p.ends_with(".mp4"));
            if is_mp4(&seg.uri) || seg.map.as_ref().is_some_and(|m| is_mp4(&m.uri)) {
                return "mp4";
            }
        }

        match self.playlist_type {
            PlaylistType::Hls => "ts",
            PlaylistType::Dash => "m4s",
        }
    }

    pub fn path(&self, directory: Option<&PathBuf>) -> PathBuf {
        let filename = format!("vsd-{}-{}.{}", self.media_type, self.id, self.extension());
        directory
            .map(|d| d.join(&filename))
            .unwrap_or_else(|| PathBuf::from(filename))
    }

    pub async fn fetch_init_seg(
        &self,
        base_url: &Url,
        client: &Client,
        query: &HashMap<String, String>,
    ) -> Result<Option<Arc<Vec<u8>>>> {
        let Some(Segment { map: Some(map), .. }) = self.segments.first() else {
            return Ok(None);
        };

        let mut request = client.get(base_url.join(&map.uri)?).query(query);
        if let Some(range) = &map.range {
            request = request.header(header::RANGE, range.as_header_value());
        }

        let bytes = request.send().await?.bytes().await?;
        Ok(Some(Arc::new(bytes.to_vec())))
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

impl MediaPlaylist {
    fn truncate(s: &str, width: usize) -> String {
        if s.chars().count() > width {
            let mut truncated = s.chars().take(width - 1).collect::<String>();
            truncated.push('…');
            truncated
        } else {
            s.to_owned()
        }
    }

    fn fmt_resolution(&self) -> String {
        self.resolution
            .map(|(w, h)| {
                match (w, h) {
                    (256, 144) => "144p",
                    (426, 240) => "240p",
                    (640, 360) => "360p",
                    (854, 480) => "480p",
                    (1280, 720) => "720p",
                    (1920, 1080) => "1080p",
                    (2048, 1080) => "2K",
                    (2560, 1440) => "1440p",
                    (3840, 2160) => "4K",
                    (7680, 4320) => "8K",
                    _ => return format!("{w}x{h}"),
                }
                .into()
            })
            .unwrap_or_else(|| "?".into())
    }

    fn fmt_bandwidth(&self) -> String {
        self.bandwidth
            .map(|b| ByteSize(b as usize).to_string())
            .unwrap_or_else(|| "?".into())
    }

    fn fmt_codecs(&self) -> String {
        Self::truncate(self.codecs.as_deref().unwrap_or("?"), 10)
    }

    fn fmt_language(&self) -> String {
        Self::truncate(self.language.as_deref().unwrap_or("?"), 9)
    }

    pub fn display(&self) -> String {
        self.to_string()
            .split('|')
            .map(|x| x.replace(" ", ""))
            .collect::<Vec<String>>()
            .join(" ")
    }
}

impl Display for MediaPlaylist {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.media_type {
            MediaType::Video => {
                write!(
                    f,
                    "{:>9} | {:>9} | {:>10} | {} fps",
                    self.fmt_resolution(),
                    self.fmt_bandwidth(),
                    self.fmt_codecs(),
                    self.frame_rate.map_or("?".into(), |r| r.to_string())
                )?;
                if self.live {
                    write!(f, " | live")?;
                }
                if self.i_frame {
                    write!(f, " | iframe")?;
                }
            }
            MediaType::Audio => {
                write!(
                    f,
                    "{:>9} | {:>9} | {:>10} | {} ch",
                    self.fmt_language(),
                    self.fmt_bandwidth(),
                    self.fmt_codecs(),
                    self.channels.map_or("?".into(), |c| c.to_string())
                )?;
                if self.live {
                    write!(f, " | live")?;
                }
            }
            MediaType::Subtitles => {
                write!(
                    f,
                    "{:>9} | {:>9} | {:>10}",
                    self.fmt_language(),
                    "?KiB",
                    self.fmt_codecs()
                )?;
            }
            MediaType::Undefined => {
                write!(f, "{:>9} | {:>9} | {:>10}", "?", "?", "?")?;
            }
        }
        Ok(())
    }
}
