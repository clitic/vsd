use crate::error::{Error, Result};
use clap::{Args, ValueEnum};
use std::{fs, path::PathBuf};
use vsd_mp4::text::{Mp4TtmlParser, Mp4VttParser};

/// Extract subtitles from mp4 boxes.
#[derive(Args, Clone, Debug)]
pub struct Extract {
    /// Path of mp4 file which either contains WVTT or STPP box.
    /// If there are multiple fragments of same mp4 file,
    /// then merge them using `merge` sub-command.
    #[arg(required = true)]
    input: PathBuf,

    /// Codec for output subtitles.
    #[arg(short, long, value_enum, default_value_t = Codec::Webvtt)]
    codec: Codec,
}

#[derive(Clone, Debug, ValueEnum)]
pub enum Codec {
    Subrip,
    Webvtt,
}

impl Extract {
    pub fn execute(self) -> Result<()> {
        let data = fs::read(self.input).map_err(|_| Error::new("input file couldn't be read."))?;
        let subtitles;

        if let Ok(vtt) = Mp4VttParser::parse_init(&data) {
            subtitles = vtt.parse_media(&data, None).map_err(|e| {
                Error::new(format!(
                    "media segment couldn't be parsed. (vsd-mp4-error: {})",
                    e
                ))
            })?;
        } else if let Ok(ttml) = Mp4TtmlParser::parse_init(&data) {
            subtitles = ttml.parse_media(&data).map_err(|e| {
                Error::new(format!(
                    "media segment couldn't be parsed. (vsd-mp4-error: {})",
                    e
                ))
            })?;
        } else {
            return Err(
                "cannot determine subtitles codec because neither WVTT nor STPP box is found."
                    .into(),
            );
        }

        print!(
            "{}",
            match &self.codec {
                Codec::Subrip => subtitles.as_srt(),
                Codec::Webvtt => subtitles.as_vtt(),
            }
        );

        Ok(())
    }
}
