use anyhow::Result;
use serde::Serialize;

#[derive(Serialize)]
pub(crate) enum MediaType {
    Audio,
    Subtitles,
    Undefined,
    Video,
}

#[derive(Serialize)]
pub(crate) enum PlaylistType {
    Dash,
    Hls,
}

#[derive(Serialize)]
pub(crate) struct ByteRange {
    pub(crate) length: u64,
    pub(crate) offset: Option<u64>,
}

#[derive(Serialize)]
pub(crate) struct Map {
    pub(crate) uri: String,
    pub(crate) byte_range: Option<ByteRange>,
}

#[derive(Serialize)]
pub(crate) enum KeyMethod {
    Aes128,
    Cenc,
    None,
    Undefined,
}

#[derive(Serialize)]
pub(crate) struct Key {
    pub(crate) default_kid: Option<String>,
    pub(crate) iv: Option<String>,
    pub(crate) method: KeyMethod,
    pub(crate) uri: String,
}

#[derive(Default, Serialize)]
pub(crate) struct Segment {
    pub(crate) byte_range: Option<ByteRange>,
    pub(crate) duration: f32,
    pub(crate) key: Option<Key>,
    pub(crate) map: Option<Map>,
    pub(crate) uri: String,
}

impl Segment {
    pub(crate) fn seg_url(&self, baseurl: &str) -> Result<reqwest::Url> {
        if self.uri.starts_with("http") || self.uri.starts_with("ftp") {
            Ok(self.uri.parse::<reqwest::Url>()?)
        } else {
            Ok(baseurl.parse::<reqwest::Url>()?.join(&self.uri)?)
        }
    }

    pub(crate) fn map_url(&self, baseurl: &str) -> Result<Option<reqwest::Url>> {
        if let Some(map) = &self.map {
            if self.uri.starts_with("http") || self.uri.starts_with("ftp") {
                return Ok(Some(map.uri.parse::<reqwest::Url>()?));
            } else {
                return Ok(Some(baseurl.parse::<reqwest::Url>()?.join(&map.uri)?));
            }
        }

        Ok(None)
    }

    pub(crate) fn key_url(&self, baseurl: &str) -> Result<Option<reqwest::Url>> {
        if let Some(key) = &self.key {
            if self.uri.starts_with("http") || self.uri.starts_with("ftp") {
                return Ok(Some(key.uri.parse::<reqwest::Url>()?));
            } else {
                return Ok(Some(baseurl.parse::<reqwest::Url>()?.join(&key.uri)?));
            }
        }

        Ok(None)
    }

    pub(crate) fn seg_range(&self, previous_byterange_end: u64) -> Option<(String, u64)> {
        if let Some(byte_range) = &self.byte_range {
            let offset = byte_range.offset.unwrap_or(0);

            let (start, end) = if offset == 0 {
                (
                    previous_byterange_end,
                    previous_byterange_end + byte_range.length - 1,
                )
            } else {
                (byte_range.length, byte_range.length + offset - 1)
            };

            Some((format!("bytes={}-{}", start, end), end))
        } else {
            None
        }
    }

    pub(crate) fn map_range(&self, previous_byterange_end: u64) -> Option<(String, u64)> {
        if let Some(map) = &self.map {
            if let Some(byte_range) = &map.byte_range {
                let offset = byte_range.offset.unwrap_or(0);

                let (start, end) = if offset == 0 {
                    (
                        previous_byterange_end,
                        previous_byterange_end + byte_range.length - 1,
                    )
                } else {
                    (byte_range.length, byte_range.length + offset - 1)
                };

                return Some((format!("bytes={}-{}", start, end), end));
            }
        }

        None
    }
}

#[derive(Serialize)]
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
    pub(crate) fn url(&self, baseurl: &str) -> Result<reqwest::Url> {
        // self.uri.starts_with("dash://")
        if self.uri.starts_with("http") || self.uri.starts_with("ftp") {
            Ok(self.uri.parse::<reqwest::Url>()?)
        } else {
            Ok(baseurl.parse::<reqwest::Url>()?.join(&self.uri)?)
        }
    }

    pub(crate) fn is_hls(&self) -> bool {
        match &self.playlist_type {
            PlaylistType::Hls => true,
            _ => false,
        }
    }

    pub(crate) fn is_dash(&self) -> bool {
        match &self.playlist_type {
            PlaylistType::Dash => true,
            _ => false,
        }
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
            if let Some(init) = segment.map {
                if init.uri.ends_with(".mp4") {
                    ext = "m4s";
                }
            }

            if segment.uri.ends_with(".mp4") {
                ext = "mp4";
            }
        }

        ext.to_owned()
    }
}

#[derive(Serialize)]
pub(crate) struct MasterPlaylist {
    pub(crate) playlist_type: PlaylistType,
    pub(crate) uri: String,
    pub(crate) streams: Vec<MediaPlaylist>,
}

impl MasterPlaylist {
    pub(crate) fn url(&self, baseurl: &str) -> Result<reqwest::Url> {
        if self.uri.starts_with("http") || self.uri.starts_with("ftp") {
            Ok(self.uri.parse::<reqwest::Url>()?)
        } else {
            Ok(baseurl.parse::<reqwest::Url>()?.join(&self.uri)?)
        }
    }

    pub(crate) fn is_hls(&self) -> bool {
        match self.playlist_type {
            PlaylistType::Hls => true,
            _ => false,
        }
    }

    pub(crate) fn is_dash(&self) -> bool {
        match self.playlist_type {
            PlaylistType::Dash => true,
            _ => false,
        }
    }

    pub(crate) fn sort_streams(
        &mut self,
        prefer_audio_lang: Option<String>,
        prefer_subs_lang: Option<String>,
    ) {
        let prefer_audio_lang = prefer_audio_lang.map(|x| x.to_lowercase());
        let prefer_subs_lang = prefer_subs_lang.map(|x| x.to_lowercase());

        let mut video_streams = self
            .streams
            .into_iter()
            .filter_map(|x| {
                if matches!(x.media_type, MediaType::Video) {
                    let pixels = if let Some((w, h)) = &x.resolution {
                        w * h
                    } else {
                        0
                    };

                    Some((x, pixels, x.bandwidth.unwrap_or(0)))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        video_streams.sort_by(|x, y| y.2.cmp(&x.2));
        video_streams.sort_by(|x, y| y.1.cmp(&x.1));

        let mut audio_streams = self
            .streams
            .into_iter()
            .filter_map(|x| {
                if matches!(x.media_type, MediaType::Audio) {
                    let mut language_factor = 0;

                    if let Some(playlist_lang) = x.language.as_ref().map(|x| x.to_lowercase()) {
                        if let Some(prefer_lang) = prefer_audio_lang {
                            if playlist_lang == prefer_lang {
                                language_factor = 2;
                            } else if playlist_lang.get(0..2) == prefer_lang.get(0..2) {
                                language_factor = 1;
                            }
                        }
                    }

                    Some((
                        x,
                        language_factor,
                        x.channels.unwrap_or(0.0),
                        x.bandwidth.unwrap_or(0),
                    ))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        audio_streams.sort_by(|x, y| y.3.cmp(&x.3));
        audio_streams.sort_by(|x, y| y.2.total_cmp(&x.2));
        audio_streams.sort_by(|x, y| y.1.cmp(&x.1));

        let mut subtitles_streams = self
            .streams
            .into_iter()
            .filter_map(|x| {
                if matches!(x.media_type, MediaType::Subtitles) {
                    let mut language_factor = 0;

                    if let Some(playlist_lang) = x.language.as_ref().map(|x| x.to_lowercase()) {
                        if let Some(prefer_lang) = prefer_subs_lang {
                            if playlist_lang == prefer_lang {
                                language_factor = 2;
                            } else if playlist_lang.get(0..2) == prefer_lang.get(0..2) {
                                language_factor = 1;
                            }
                        }
                    }

                    Some((x, language_factor))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        subtitles_streams.sort_by(|x, y| y.1.cmp(&x.1));

        self.streams = video_streams
            .into_iter()
            .map(|x| x.0)
            .chain(audio_streams.into_iter().map(|x| x.0))
            .chain(subtitles_streams.into_iter().map(|x| x.0))
            .collect::<Vec<_>>();
    }
}
