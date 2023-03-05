use super::Stream;
use anyhow::{bail, Result};
use kdam::term::Colorizer;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Progress {
    pub audio: Option<Stream>,
    pub directory: Option<String>,
    pub output: Option<String>,
    pub subtitles: Option<Stream>,
    pub video: Stream,
}

impl Progress {
    pub fn mux(&self) -> Result<()> {
        if let Some(output) = &self.output {
            let mut args = vec!["-i".to_owned(), self.video.path(&self.directory)];

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

            if let Some(audio) = &self.audio {
                args.push("-i".to_owned());
                args.push(audio.path(&self.directory));
            }

            if let Some(subtitles) = &self.subtitles {
                args.push("-i".to_owned());
                args.push(subtitles.path(&self.directory));
            }

            args.push("-c:v".to_owned());
            args.push("copy".to_owned());
            args.push("-c:a".to_owned());
            args.push("copy".to_owned());

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

            if let Some(Stream {
                language: Some(language),
                ..
            }) = &self.audio
            {
                args.push("-metadata:s:a:0".to_owned());
                args.push(format!("language={}", language));
            }

            if let Some(Stream {
                language: Some(language),
                ..
            }) = &self.subtitles
            {
                args.push("-metadata:s:s:0".to_owned());
                args.push(format!("language={}", language));
                args.push("-disposition:s:0".to_owned());
                args.push("default".to_owned());
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
                bail!("FFMPEG exited with code {}", code.code().unwrap_or(1))
            }

            if let Some(audio) = &self.audio {
                std::fs::remove_file(&audio.path(&self.directory))?;
            }

            if let Some(subtitles) = &self.subtitles {
                std::fs::remove_file(&subtitles.path(&self.directory))?;
            }

            std::fs::remove_file(&self.video.path(&self.directory))?;
        }

        Ok(())
    }
}
