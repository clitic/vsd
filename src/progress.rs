use anyhow::{anyhow, bail, Result};
use kdam::term::Colorizer;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::path::Path;

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

    pub fn relative_filename(&self, prefix: &str, ext: &str) -> String {
        format!(
            "{}{}{}",
            Path::new(&self.file).file_stem().unwrap().to_str().unwrap(),
            prefix,
            ext
        )
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Progress {
    pub json_file: String,
    pub current: String,
    pub video: StreamData,
    pub audio: Option<StreamData>,
    pub subtitles: Option<(String, Option<String>)>,
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

                if let Some((subtitles, _)) = &self.subtitles {
                    let path = self.video.relative_filename("_subtitles", "");
                    File::create(&path)?.write_all(subtitles.as_bytes())?;
                    args.push("-i".to_owned());
                    args.push(path);
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

                args.push("-metadata".to_owned());
                args.push(format!("title=\"{}\"", self.video.url));

                if let Some(StreamData {
                    language: Some(language),
                    ..
                }) = &self.audio
                {
                    args.push("-metadata:s:a:0".to_owned());
                    args.push(format!("language={}", language));
                }

                if let Some((_, Some(language))) = &self.subtitles {
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

            if self.subtitles.is_some() {
                let path = self.video.relative_filename("_subtitles", "");
                std::fs::remove_file(&path)?;
            }

            std::fs::remove_file(&self.video.file)?;
        }

        if std::path::Path::new(&self.json_file).exists() {
            std::fs::remove_file(&self.json_file)?;
        }
        Ok(())
    }
}
