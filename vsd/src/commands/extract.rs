use anyhow::{anyhow, bail, Result};
use clap::{Args, ValueEnum};
use std::path::PathBuf;
use vsd_mp4::text::{Mp4TtmlParser, Mp4VttParser, Subtitles};

/// Extract subtitles from mp4 boxes.
#[derive(Debug, Clone, Args)]
pub struct Extract {
    /// Path of mp4 file which either contains WVTT or STPP boxes.
    /// If there are multiple segments of same mp4 file,
    /// then merge them using `merge` sub-command.
    #[arg(required = true)]
    input: PathBuf,

    /// Codec for output subtitles.
    #[arg(short, long, value_enum, default_value_t = Codec::Subrip)]
    codec: Codec,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum Codec {
    Subrip,
    Webvtt,
}

impl Extract {
    pub fn execute(self) -> Result<()> {
        let data = std::fs::read(self.input)?;
        let cues;

        if let Ok(vtt) = Mp4VttParser::parse_init(&data) {
            cues = vtt.parse_media(&data, None).map_err(|x| anyhow!(x))?;
        } else if let Ok(ttml) = Mp4TtmlParser::parse_init(&data) {
            cues = ttml.parse_media(&data).map_err(|x| anyhow!(x))?;
        } else {
            bail!(
                "cannot determine subtitles codec because neither WVTT nor STPP boxes are found."
            );
        }

        let subtitles = Subtitles::new(cues);

        print!(
            "{}",
            match &self.codec {
                Codec::Subrip => subtitles.to_srt(),
                Codec::Webvtt => subtitles.to_vtt(),
            }
        );

        Ok(())
    }
}
