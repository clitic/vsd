use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct StreamData {
    pub url: String,
    pub file: String,
    pub downloaded: usize,
    pub total: usize,
    pub playlist: String,
}

impl StreamData {
    pub fn new(url: &str, file: &str, playlist: &str) -> Result<Self> {
        Ok(Self {
            url: url.to_owned(),
            file: file.to_owned(),
            downloaded: 0,
            total: m3u8_rs::parse_media_playlist_res(&playlist.as_bytes())
                .map_err(|_| anyhow!("Couldn't parse {} as media playlist.", url))?
                .segments
                .len(),
            playlist: playlist.to_owned(),
        })
    }

    pub fn to_playlist(&self) -> m3u8_rs::MediaPlaylist {
        m3u8_rs::parse_media_playlist_res(self.playlist.as_bytes())
            .map_err(|_| anyhow!("Couldn't parse {} as media playlist.", self.url))
            .unwrap()
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Progress {
    pub json_file: String,
    pub current: String,
    pub video: StreamData,
    pub audio: Option<StreamData>,
    pub subtitles: Option<String>,
}

impl Progress {
    pub fn new_empty() -> Self {
        Self {
            json_file: "".to_owned(),
            current: "video".to_owned(),
            video: StreamData::default(),
            audio: None,
            subtitles: None,
        }
    }

    pub fn json_file(&mut self, file: &str) {
        self.json_file = file.to_owned();
    }

    pub fn current(&mut self, current: &str) {
        self.current = current.to_owned();
    }

    pub fn update(&mut self, pos: usize, total: usize, json_file: &std::fs::File) {
        match self.current.as_str() {
            "video" => {
                self.video.downloaded = pos;
                self.video.total = total;
            }

            "audio" => {
                if let Some(audio) = &mut self.audio {
                    audio.downloaded = pos;
                    audio.total = total;
                }
            }
            _ => (),
        }

        serde_json::to_writer_pretty(json_file, self).unwrap();
    }

    pub fn downloaded(&self) -> usize {
        return match self.current.as_str() {
            "video" => self.video.downloaded,

            "audio" => {
                if let Some(audio) = &self.audio {
                    audio.downloaded
                } else {
                    0
                }
            }

            _ => 0,
        };
    }
}
