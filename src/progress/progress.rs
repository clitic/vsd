use super::StreamData;
use anyhow::{bail, Result};
use kdam::term::Colorizer;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Seek, SeekFrom, Write};

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

    pub fn set_progress_file(&mut self) {
        self.file = self.video.set_extension("vsd");
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
        writer.write_all(&bincode::serialize(self)?)?;
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

    pub fn mux(
        &self,
        output: &Option<String>,
        alternative_media_type: &Option<m3u8_rs::AlternativeMediaType>,
    ) -> Result<()> {
        if let Some(output) = output {
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

            if alternative_media_type.is_none() {
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
