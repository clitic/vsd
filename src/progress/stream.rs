use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Stream {
    pub url: String,
    pub language: Option<String>,
    file: String,
    pub downloaded: usize,
    pub total: usize,
    pub playlist: String,
}

impl Stream {
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

    pub fn path(&self, directory: &Option<String>) -> String {
        if let Some(directory) = directory {
            finalize_path(&Path::new(directory)
                .join(&self.file)
                .to_str()
                .unwrap()
                .to_owned())
        } else {
            self.file.to_owned()
        }
    }

    pub fn filename(&self, suffix: &str, ext: Option<&str>) -> String {
        finalize_path(&format!(
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
        ))
    }

    pub fn set_suffix(&mut self, suffix: &str) {
        self.file = finalize_path(&format!("({}) {}", suffix, self.file));
    }

    pub fn set_extension(&self, ext: &str) -> String {
        let mut path = PathBuf::from(&self.file);
        path.set_extension(ext);
        path.to_str().unwrap().to_owned()
    }

    pub fn set_extension_mut(&mut self, ext: &str) {
        self.file = finalize_path(&self.set_extension(ext));
    }
}

fn finalize_path(path: &str) -> String {
    if Path::new(path).exists() {
        let stemed_path = Path::new(path).file_stem().unwrap().to_str().unwrap();
        let ext = Path::new(path).extension().unwrap().to_str().unwrap();

        for i in 1.. {
            let core_file_copy = format!("{} ({}).{}", stemed_path, i, ext);

            if !Path::new(&core_file_copy).exists() {
                return core_file_copy;
            }
        }
    }

    path.to_owned()
}
