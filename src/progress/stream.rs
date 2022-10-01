use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct StreamData {
    pub url: String,
    pub language: Option<String>,
    pub file: String,
    pub downloaded: usize,
    pub total: usize,
    pub playlist: String,
}

impl StreamData {
    pub fn new(url: &str, language: Option<String>, file: &str, playlist: &str) -> Result<Self> {
        Ok(Self {
            url: url.to_owned(),
            language,
            file: file.to_owned(),
            downloaded: 0,
            total: m3u8_rs::parse_media_playlist_res(playlist.as_bytes())
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

    pub fn filename(&self, suffix: &str, ext: Option<&str>) -> String {
        format!(
            "({}) {}{}",
            suffix,
            Path::new(&self.file).file_stem().unwrap().to_str().unwrap(),
            if let Some(ext) = ext {
                if ext.starts_with('.') {
                    ext.to_owned()
                } else {
                    ".".to_owned() + ext
                }
            } else {
                "".to_owned()
            }
        )
    }

    pub fn set_suffix(&mut self, suffix: &str) {
        self.file = format!("({}) {}", suffix, self.file);
    }

    pub fn set_extension(&self, ext: &str) -> String {
        let mut path = PathBuf::from(&self.file);
        path.set_extension(ext);
        path.to_str().unwrap().to_owned()
    }

    pub fn set_extension_mut(&mut self, ext: &str) {
        self.file = self.set_extension(ext);
    }
}
