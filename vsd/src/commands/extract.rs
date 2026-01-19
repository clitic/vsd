use anyhow::{Result, bail};
use clap::{Args, ValueEnum};
use std::path::PathBuf;
use vsd_mp4::text::{Mp4TtmlParser, Mp4VttParser};

/// Extract subtitles from mp4 boxes.
#[derive(Args, Clone, Debug)]
pub struct Extract {
    /// Path of mp4 file which either contains WVTT or STPP box.
    /// If there are multiple fragments of same mp4 file,
    /// then merge them using merge sub-command.
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
        let data = std::fs::read(self.input)?;
        let subtitles;

        if let Ok(vtt) = Mp4VttParser::from_init(&data) {
            subtitles = vtt.parse(&data, None)?;
        } else if let Ok(ttml) = Mp4TtmlParser::from_init(&data) {
            subtitles = ttml.parse(&data)?;
        } else {
            bail!("cannot determine subtitles codec because neither WVTT nor STPP box is found.");
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
