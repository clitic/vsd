use anyhow::{anyhow, bail, Result};
use kdam::term::Colorizer;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Seek, SeekFrom};
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

    pub fn filename(&self, suffix: &str, ext: Option<&str>) -> String {
        format!(
            "({}) {}{}",
            suffix,
            Path::new(&self.file).file_stem().unwrap().to_str().unwrap(),
            if let Some(ext) = ext {
                if ext.starts_with(".") {
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

    pub fn set_extension(&mut self, ext: &str) {
        let mut path = PathBuf::from(&self.file);
        path.set_extension(ext);
        self.file = path.to_str().unwrap().to_owned();
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Progress {
    pub file: String,
    pub video: StreamData,
    pub audio: Option<StreamData>,
    pub subtitles: Option<StreamData>,
}

impl Progress {
    pub fn new_empty() -> Self {
        Self {
            file: "".to_owned(),
            video: StreamData::default(),
            audio: None,
            subtitles: None,
        }
    }

    pub fn set_json_file(&mut self) {
        self.file = self.video.filename("resume", Some("json"));
    }

    pub fn update(&mut self, stream: &str, pos: usize, writer: &mut File) -> Result<()> {
        match stream {
            "video" => {
                self.video.downloaded = pos;
            }

            "audio" => {
                if let Some(audio) = &mut self.audio {
                    audio.downloaded = pos;
                }
            }

            _ => (),
        }

        writer.seek(SeekFrom::Start(0))?;
        serde_json::to_writer(writer, self)?;
        Ok(())
    }

    pub fn downloaded(&self, stream: &str) -> usize {
        return match stream {
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

    pub fn transmux_trancode(&self, output: Option<String>, alternative: bool) -> Result<()> {
        if let Some(output) = &output {
            let mut args = vec!["-i".to_owned(), self.video.file.clone()];

            // args.push("-metadata".to_owned());
            // args.push(format!("title=\"{}\"", self.video.url));

            // if let StreamData {
            //     language: Some(language),
            //     ..
            // } = &self.video
            // {
            //     args.push("-metadata".to_owned());
            //     args.push(format!("language={}", language));
            // }

            if !alternative {
                if let Some(audio) = &self.audio {
                    args.push("-i".to_owned());
                    args.push(audio.file.clone());
                }

                if let Some(subtitles) = &self.subtitles {
                    args.push("-i".to_owned());
                    args.push(subtitles.file.clone());
                }

                args.push("-c:v".to_owned());
                args.push("copy".to_owned());

                if self.audio.is_some() {
                    args.push("-c:a".to_owned());
                    args.push("copy".to_owned());
                }

                if self.subtitles.is_some() {
                    args.push("-scodec".to_owned());

                    if output.ends_with(".mp4") {
                        args.push("mov_text".to_owned());
                    } else {
                        args.push("srt".to_owned());
                    }
                }

                // args.push("-metadata".to_owned());
                // args.push(format!("title=\"{}\"", self.video.url));

                if let Some(StreamData {
                    language: Some(language),
                    ..
                }) = &self.audio
                {
                    args.push("-metadata:s:a:0".to_owned());
                    args.push(format!("language={}", language));
                }

                if let Some(StreamData {
                    language: Some(language),
                    ..
                }) = &self.subtitles
                {
                    args.push("-metadata:s:s:0".to_owned());
                    args.push(format!("language={}", language));
                    args.push("-disposition:s:0".to_owned());
                    args.push("default".to_owned());
                }
            }

            args.push(output.to_owned());

            println!(
                "Executing {} {}",
                "ffmpeg".colorize("cyan"),
                args.join(" ").colorize("cyan")
            );

            if std::path::Path::new(output).exists() {
                std::fs::remove_file(output)?;
            }

            let code = std::process::Command::new("ffmpeg")
                .args(args)
                .stderr(std::process::Stdio::null())
                .spawn()?
                .wait()?;

            if !code.success() {
                bail!("FFMPEG exited with code {}.", code.code().unwrap_or(1))
            }

            if let Some(audio) = &self.audio {
                std::fs::remove_file(&audio.file)?;
            }

            if let Some(subtitles) = &self.subtitles {
                std::fs::remove_file(&subtitles.file)?;
            }

            std::fs::remove_file(&self.video.file)?;
        }

        if std::path::Path::new(&self.file).exists() {
            std::fs::remove_file(&self.file)?;
        }
        Ok(())
    }
}
