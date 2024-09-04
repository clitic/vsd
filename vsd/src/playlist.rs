/*
    REFERENCES
    ----------

    1.https://en.wikipedia.org/wiki/Filename#Reserved_characters_and_words
    2. https://en.wikipedia.org/wiki/Comparison_of_file_systems#Limits

*/

use crate::commands::Quality;
use anyhow::{bail, Result};
use kdam::term::Colorizer;
use requestty::prompt::style::Stylize;
use reqwest::{header::HeaderValue, Url};
use serde::Serialize;
use std::{fmt::Display, io::Write, path::PathBuf};

#[derive(Serialize)]
pub(crate) struct MasterPlaylist {
    pub(crate) playlist_type: PlaylistType,
    pub(crate) uri: String,
    pub(crate) streams: Vec<MediaPlaylist>,
}

impl MasterPlaylist {
    pub(crate) fn sort_streams(
        mut self,
        prefer_audio_lang: Option<String>,
        prefer_subs_lang: Option<String>,
    ) -> Self {
        let prefer_audio_lang = prefer_audio_lang.map(|x| x.to_lowercase());
        let prefer_subs_lang = prefer_subs_lang.map(|x| x.to_lowercase());

        let mut video_streams = vec![];
        let mut audio_streams = vec![];
        let mut subtitle_streams = vec![];
        let mut undefined_streams = vec![];

        for stream in self.streams {
            match stream.media_type {
                MediaType::Audio => {
                    let mut language_factor = 0;

                    if let Some(playlist_lang) = &stream.language.as_ref().map(|x| x.to_lowercase())
                    {
                        if let Some(prefer_lang) = &prefer_audio_lang {
                            if playlist_lang == prefer_lang {
                                language_factor = 2;
                            } else if playlist_lang.get(0..2) == prefer_lang.get(0..2) {
                                language_factor = 1;
                            }
                        }
                    }

                    let channels = stream.channels.unwrap_or(0.0);
                    let bandwidth = stream.bandwidth.unwrap_or(0);

                    audio_streams.push((stream, language_factor, channels, bandwidth));
                }
                MediaType::Subtitles => {
                    let mut language_factor = 0;

                    if let Some(playlist_lang) = &stream.language.as_ref().map(|x| x.to_lowercase())
                    {
                        if let Some(prefer_lang) = &prefer_subs_lang {
                            if playlist_lang == prefer_lang {
                                language_factor = 2;
                            } else if playlist_lang.get(0..2) == prefer_lang.get(0..2) {
                                language_factor = 1;
                            }
                        }
                    }

                    subtitle_streams.push((stream, language_factor));
                }
                MediaType::Undefined => undefined_streams.push(stream),
                MediaType::Video => {
                    let pixels = if let Some((w, h)) = &stream.resolution {
                        w * h
                    } else {
                        0
                    };

                    let bandwidth = stream.bandwidth.unwrap_or(0);

                    video_streams.push((stream, pixels, bandwidth));
                }
            }
        }

        video_streams.sort_by(|x, y| y.2.cmp(&x.2));
        video_streams.sort_by(|x, y| y.1.cmp(&x.1));
        audio_streams.sort_by(|x, y| y.3.cmp(&x.3));
        audio_streams.sort_by(|x, y| y.2.total_cmp(&x.2));
        audio_streams.sort_by(|x, y| y.1.cmp(&x.1));
        subtitle_streams.sort_by(|x, y| y.1.cmp(&x.1));

        self.streams = video_streams
            .into_iter()
            .map(|x| x.0)
            .chain(audio_streams.into_iter().map(|x| x.0))
            .chain(subtitle_streams.into_iter().map(|x| x.0))
            .chain(undefined_streams)
            .collect::<Vec<_>>();

        self
    }

    fn select_video_stream(&self, quality: &Quality) -> Option<usize> {
        let video_streams = self
            .streams
            .iter()
            .filter(|x| x.media_type == MediaType::Video)
            .enumerate();

        let mut has_resolution = None;
        let mut has_height = None;

        let (w, h) = match quality {
            Quality::Lowest => return Some(video_streams.count() - 1),
            Quality::Highest => return Some(0),
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
        };

        for (i, stream) in video_streams {
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

        has_resolution.or(has_height)
    }

    pub(crate) fn select_streams(
        self,
        quality: Quality,
        skip_prompts: bool,
        raw_prompts: bool,
    ) -> Result<(Vec<MediaPlaylist>, Vec<MediaPlaylist>)> {
        let default_video_stream_index = self.select_video_stream(&quality);

        if let Some(default_video_stream_index) = default_video_stream_index {
            let mut video_streams = vec![];
            let mut audio_streams = vec![];
            let mut subtitle_streams = vec![];
            // TODO - Add support for downloading undefined streams
            let mut undefined_streams = vec![];

            for stream in self.streams {
                match stream.media_type {
                    MediaType::Audio => audio_streams.push(stream),
                    MediaType::Subtitles => subtitle_streams.push(stream),
                    MediaType::Undefined => undefined_streams.push(stream),
                    MediaType::Video => video_streams.push(stream),
                }
            }

            let mut choices_with_default = vec![];
            let mut choices_with_default_ranges: [std::ops::Range<usize>; 4] =
                [(0..0), (0..0), (0..0), (0..0)];

            choices_with_default.push(requestty::Separator(
                "─────── Video Streams ────────".to_owned(),
            ));
            choices_with_default.extend(video_streams.iter().enumerate().map(|(i, x)| {
                requestty::Choice((x.display_video_stream(), i == default_video_stream_index))
            }));
            choices_with_default_ranges[0] = 1..choices_with_default.len();
            choices_with_default.push(requestty::Separator(
                "─────── Audio Streams ────────".to_owned(),
            ));
            choices_with_default.extend(
                audio_streams
                    .iter()
                    .enumerate()
                    .map(|(i, x)| requestty::Choice((x.display_audio_stream(), i == 0))),
            );

            if skip_prompts || raw_prompts {
                choices_with_default_ranges[1] =
                    choices_with_default_ranges[0].end..(choices_with_default.len() - 1);
            } else {
                choices_with_default_ranges[1] =
                    (choices_with_default_ranges[0].end + 1)..choices_with_default.len();
            }

            choices_with_default.push(requestty::Separator(
                "────── Subtitle Streams ──────".to_owned(),
            ));
            choices_with_default.extend(
                subtitle_streams
                    .iter()
                    .enumerate()
                    .map(|(i, x)| requestty::Choice((x.display_subtitle_stream(), i == 0))),
            );

            if skip_prompts || raw_prompts {
                choices_with_default_ranges[2] =
                    choices_with_default_ranges[1].end..(choices_with_default.len() - 2);
            } else {
                choices_with_default_ranges[2] =
                    (choices_with_default_ranges[1].end + 1)..choices_with_default.len();
            }

            // println!("{:?}", choices_with_default_ranges);

            if skip_prompts || raw_prompts {
                println!("Select streams to download:");
                let mut selected_choices_index = vec![];
                let mut index = 1;

                for choice in choices_with_default {
                    if let requestty::Separator(seperator) = choice {
                        println!("{}", seperator.replace('─', "-"));
                    } else {
                        let (message, selected) = choice.unwrap_choice();

                        if selected {
                            selected_choices_index.push(index);
                        }

                        println!(
                            "{:2}) [{}] {}",
                            index,
                            if selected { 'x' } else { ' ' },
                            message
                        );
                        index += 1;
                    }
                }

                println!("------------------------------");

                if raw_prompts && !skip_prompts {
                    print!(
                        "Press enter to proceed with defaults.\n\
                        Or select streams to download (1, 2, etc.): "
                    );
                    std::io::stdout().flush()?;
                    let mut input = String::new();
                    std::io::stdin().read_line(&mut input)?;

                    println!("------------------------------");

                    let input = input.trim();

                    if !input.is_empty() {
                        selected_choices_index = input
                            .split(',')
                            .filter_map(|x| x.trim().parse::<usize>().ok())
                            .collect::<Vec<usize>>();
                    }
                }

                let mut selected_streams = vec![];
                let mut selected_subtitle_streams = vec![];
                let mut video_streams_offset = 1;
                let mut audio_streams_offset = video_streams_offset + video_streams.len();
                let mut subtitle_streams_offset = audio_streams_offset + audio_streams.len();

                for i in selected_choices_index {
                    if choices_with_default_ranges[0].contains(&i) {
                        let stream = video_streams.remove(i - video_streams_offset);
                        println!(
                            "   {} {}",
                            "Selected".colorize("bold green"),
                            stream.display_stream()
                        );
                        selected_streams.push(stream);
                        video_streams_offset += 1;
                    } else if choices_with_default_ranges[1].contains(&i) {
                        let stream = audio_streams.remove(i - audio_streams_offset);
                        println!(
                            "   {} {}",
                            "Selected".colorize("bold green"),
                            stream.display_stream()
                        );
                        selected_streams.push(stream);
                        audio_streams_offset += 1;
                    } else if choices_with_default_ranges[2].contains(&i) {
                        let stream = subtitle_streams.remove(i - subtitle_streams_offset);
                        println!(
                            "   {} {}",
                            "Selected".colorize("bold green"),
                            stream.display_stream()
                        );
                        selected_subtitle_streams.push(stream);
                        subtitle_streams_offset += 1;
                    }
                }

                Ok((selected_streams, selected_subtitle_streams))
            } else {
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
                let mut selected_subtitle_streams = vec![];
                let mut video_streams_offset = 1;
                let mut audio_streams_offset = video_streams_offset + video_streams.len() + 1;
                let mut subtitle_streams_offset = audio_streams_offset + audio_streams.len() + 1;

                for selected_item in answer.as_list_items().unwrap() {
                    if choices_with_default_ranges[0].contains(&selected_item.index) {
                        selected_streams
                            .push(video_streams.remove(selected_item.index - video_streams_offset));
                        video_streams_offset += 1;
                    } else if choices_with_default_ranges[1].contains(&selected_item.index) {
                        selected_streams
                            .push(audio_streams.remove(selected_item.index - audio_streams_offset));
                        audio_streams_offset += 1;
                    } else if choices_with_default_ranges[2].contains(&selected_item.index) {
                        selected_subtitle_streams.push(
                            subtitle_streams.remove(selected_item.index - subtitle_streams_offset),
                        );
                        subtitle_streams_offset += 1;
                    }
                }

                Ok((selected_streams, selected_subtitle_streams))
            }
        } else {
            bail!("playlist doesn't contain pre-selected video quality stream.")
        }
    }
}

#[derive(Default, Serialize)]
pub(crate) struct MediaPlaylist {
    pub(crate) bandwidth: Option<u64>,
    pub(crate) channels: Option<f32>,
    pub(crate) codecs: Option<String>,
    pub(crate) extension: Option<String>,
    pub(crate) frame_rate: Option<f32>,
    pub(crate) i_frame: bool,
    pub(crate) language: Option<String>,
    pub(crate) live: bool,
    pub(crate) media_type: MediaType,
    pub(crate) playlist_type: PlaylistType,
    pub(crate) resolution: Option<(u64, u64)>,
    pub(crate) segments: Vec<Segment>,
    pub(crate) uri: String,
}

impl MediaPlaylist {
    pub(crate) fn is_hls(&self) -> bool {
        matches!(&self.playlist_type, PlaylistType::Hls)
    }

    pub(crate) fn default_kid(&self) -> Option<String> {
        if let Some(segment) = self.segments.get(0) {
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

    pub(crate) fn extension(&self) -> String {
        if let Some(ext) = &self.extension {
            return ext.to_owned();
        }

        let mut ext = match &self.playlist_type {
            PlaylistType::Hls => "ts",
            PlaylistType::Dash => "m4s",
        };

        if let Some(segment) = self.segments.get(0) {
            if let Some(init) = &segment.map {
                if init.uri.ends_with(".mp4") {
                    ext = "mp4";
                }
            }

            if segment.uri.ends_with(".mp4") {
                ext = "mp4";
            }
        }

        ext.to_owned()
    }

    pub(crate) fn file_path(&self, directory: &Option<PathBuf>, ext: &str) -> PathBuf {
        let mut filename = self
            .uri
            .split('?')
            .next()
            .unwrap()
            .split('/')
            .last()
            .unwrap_or("undefined")
            .chars()
            .map(|x| match x {
                '/' | '\\' | '?' | '%' | '*' | ':' | '|' | '"' | '<' | '>' | '.' | ';' | '='
                | ' ' => '_',
                _ => x,
            })
            .collect::<String>();

        if filename.len() > 128 {
            filename = filename[..128].to_owned();
        }

        let filename = PathBuf::from(filename).with_extension("");

        let prefix = match &self.media_type {
            MediaType::Audio => "vsd_audio",
            MediaType::Subtitles => "vsd_subtitles",
            MediaType::Undefined => "vsd_undefined",
            MediaType::Video => "vsd_video",
        };

        let mut path = PathBuf::from(format!("{}_{}.{}", prefix, filename.to_string_lossy(), ext));

        if let Some(directory) = directory {
            path = directory.join(path);
        }

        if path.exists() {
            for i in 1.. {
                path.set_file_name(format!(
                    "{}_{}_({}).{}",
                    prefix,
                    filename.to_string_lossy(),
                    i,
                    ext
                ));

                if !path.exists() {
                    return path;
                }
            }
        }

        path
    }

    pub(crate) fn display_stream(&self) -> String {
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

    pub(crate) fn display_subtitle_stream(&self) -> String {
        let mut extra = format!(
            "language: {}",
            self.language.as_ref().unwrap_or(&"?".to_owned())
        );

        if let Some(codecs) = &self.codecs {
            extra += &format!(", codecs: {}", codecs);
        }

        extra
    }

    pub(crate) fn add_query(&mut self, query: &str) {
        for segment in &mut self.segments {
            if let Some(map) = &mut segment.map {
                let mut uri = map.uri.clone();

                if uri.ends_with("??") || !uri.contains('?') {
                    uri += "?";
                } else if (uri.ends_with("&&") || !uri.ends_with('&')) && !uri.ends_with('?') {
                    uri += "&";
                }

                uri += query;
                map.uri = uri;
            }

            let mut uri = segment.uri.clone();

            if uri.ends_with("??") || !uri.contains('?') {
                uri += "?";
            } else if (uri.ends_with("&&") || !uri.ends_with('&')) && !uri.ends_with('?') {
                uri += "&";
            }

            uri += query;
            segment.uri = uri;
        }
    }
}

#[derive(Default, Serialize)]
pub(crate) enum PlaylistType {
    Dash,
    #[default]
    Hls,
}

#[derive(Clone, Default, PartialEq, Serialize)]
pub(crate) enum MediaType {
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
                Self::Subtitles => "subtitles",
                Self::Undefined => "undefined",
                Self::Video => "video",
            }
        )
    }
}

#[derive(Clone, PartialEq, Serialize)]
pub(crate) enum KeyMethod {
    Aes128,
    Cenc,
    None,
    Other(String),
    SampleAes,
}

#[derive(Clone, Serialize)]
pub(crate) struct Range {
    pub(crate) start: u64,
    pub(crate) end: u64,
}

impl Range {
    pub(crate) fn as_header_value(&self) -> HeaderValue {
        HeaderValue::from_str(&format!("bytes={}-{}", self.start, self.end)).unwrap()
    }
}

#[derive(Clone, Serialize)]
pub(crate) struct Map {
    pub(crate) uri: String,
    pub(crate) range: Option<Range>,
}

/*

.mpd (with encryption) converted to .m3u8

#EXT-X-KEY:METHOD=SAMPLE-AES,URI="data:text/plain;base64,AAAAXHBzc2gAAAAA7e+LqXnWSs6jyCfc1R0h7QAAADwSEDAvgN1BHkiGvKW7H4AYoCQSEDAvgN1BHkiGvKW7H4AYoCQSEDAvgN1BHkiGvKW7H4AYoCRI88aJmwY=",KEYID=0x302F80DD411E4886BCA5BB1F8018A024,IV=0x77FD1889AAF4143B085548B3C0F95B9A,KEYFORMATVERSIONS="1",KEYFORMAT="urn:uuid:edef8ba9-79d6-4ace-a3c8-27dcd51d21ed"
#EXT-X-KEY:METHOD=SAMPLE-AES,URI="skd://302f80dd-411e-4886-bca5-bb1f8018a024:77FD1889AAF4143B085548B3C0F95B9A",KEYFORMATVERSIONS="1",KEYFORMAT="com.apple.streamingkeydelivery"
#EXT-X-KEY:METHOD=SAMPLE-AES-CTR,KEYFORMAT="com.microsoft.playready",KEYFORMATVERSIONS="1",URI="data:text/plain;charset=UTF-16;base64,xAEAAAEAAQC6ATwAVwBSAE0ASABFAEEARABFAFIAIAB4AG0AbABuAHMAPQAiAGgAdAB0AHAAOgAvAC8AcwBjAGgAZQBtAGEAcwAuAG0AaQBjAHIAbwBzAG8AZgB0AC4AYwBvAG0ALwBEAFIATQAvADIAMAAwADcALwAwADMALwBQAGwAYQB5AFIAZQBhAGQAeQBIAGUAYQBkAGUAcgAiACAAdgBlAHIAcwBpAG8AbgA9ACIANAAuADAALgAwAC4AMAAiAD4APABEAEEAVABBAD4APABQAFIATwBUAEUAQwBUAEkATgBGAE8APgA8AEsARQBZAEwARQBOAD4AMQA2ADwALwBLAEUAWQBMAEUATgA+ADwAQQBMAEcASQBEAD4AQQBFAFMAQwBUAFIAPAAvAEEATABHAEkARAA+ADwALwBQAFIATwBUAEUAQwBUAEkATgBGAE8APgA8AEsASQBEAD4AOQBmAEIAMQAxAEsAMQB0AC8ARQBtAFEANABYAEMATQBjAEoANgBnAEkAZwA9AD0APAAvAEsASQBEAD4APAAvAEQAQQBUAEEAPgA8AC8AVwBSAE0ASABFAEEARABFAFIAPgA="

*/
#[derive(Clone, Serialize)]
pub(crate) struct Key {
    pub(crate) default_kid: Option<String>,
    pub(crate) iv: Option<String>,
    pub(crate) key_format: Option<String>,
    pub(crate) method: KeyMethod,
    pub(crate) uri: Option<String>,
}

#[derive(Clone, Default, Serialize)]
pub(crate) struct Segment {
    pub(crate) range: Option<Range>,
    pub(crate) duration: f32, // consider changing it to f64
    pub(crate) key: Option<Key>,
    pub(crate) map: Option<Map>,
    pub(crate) uri: String,
}
