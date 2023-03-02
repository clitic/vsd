use crate::mp4parser::{Mp4TtmlParser, Mp4VttParser, Subtitles};
use anyhow::{anyhow, bail, Result};
use clap::{Args, ValueEnum};

#[derive(Debug, Clone, ValueEnum)]
pub enum Format {
    Srt,
    Vtt,
}

/// Extract subtitles embedded inside an mp4 file.
///
/// This is based on the https://github.com/xhlove/dash-subtitle-extractor
#[derive(Debug, Clone, Args)]
pub struct Extract {
    // TODO - Write docs
    /// List of subtitles segment files where first file is init.mp4 and following files are *.m4s (segments).
    /// A single mp4 file can also be provided.
    #[arg(required = true)]
    input: String,

    /// Subtitles output format.
    #[arg(short, long, value_enum, default_value_t = Format::Srt)]
    format: Format,

    // /// Set timescale manually if no init segment is present.
    // /// If timescale is set to anything then webvtt codec is used.
    // #[arg(short, long)]
    // timescale: Option<u32>,
    //   -segment-time SEGMENT_TIME, --segment-time SEGMENT_TIME
    //                         single segment duration, usually needed for ttml
    //                         content, calculation method: d / timescale
}

impl Extract {
    pub fn perform(&self) -> Result<()> {
        let mut data = std::fs::read(self.input)?;
        let vtt = Mp4VttParser::parse_init(&data);
        let ttml = Mp4TtmlParser::parse_init(&data);

        let cues;

        if let Ok(vtt) = vtt {
            cues = vtt.parse_media(&data, None).map_err(|x| anyhow!(x))?;
        } else if let Ok(ttml) = ttml {
            cues = ttml.parse_media(&data).map_err(|x| anyhow!(x))?;
        } else {
            bail!("mp4parser.text: cannot determine subtitles codec because WVTT/STPP box is not found.");
        }

        let subtitles = Subtitles::new(cues);

        print!(
            "{}",
            match &self.format {
                Format::Srt => subtitles.to_srt(),
                Format::Vtt => subtitles.to_vtt(),
            }
        );

        Ok(())
    }
}
