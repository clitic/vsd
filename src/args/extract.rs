use crate::subtitles::MP4Subtitles;
use anyhow::{anyhow, bail, Result};
use clap::{ArgEnum, Args};

#[derive(Debug, Clone, ArgEnum)]
pub enum Format {
    SRT,
    VTT,
}

/// Extract subtitles embedded inside an mp4 file.
///
/// This is based on the https://github.com/xhlove/dash-subtitle-extractor
#[derive(Debug, Clone, Args)]
pub struct Extract {
    /// List of subtitles segment files where first file is init.mp4 and following files are *.m4s (segments).
    /// A single mp4 file can also be provided.
    #[clap(required = true)]
    files: Vec<String>,

    /// Subtitles output format.
    #[clap(short, long, arg_enum, default_value_t = Format::SRT)]
    format: Format,

    /// Set timescale manually if no init segment is present.
    /// If timescale is set to anything then webvtt codec is used.
    #[clap(short, long)]
    timescale: Option<u32>,
    //   -segment-time SEGMENT_TIME, --segment-time SEGMENT_TIME
    //                         single segment duration, usually needed for ttml
    //                         content, calculation method: d / timescale
}

impl Extract {
    pub fn perform(&self) -> Result<()> {
        let mut files = vec![];

        for pattern in &self.files {
            for file in glob::glob(pattern)? {
                files.push(file?);
            }
        }

        if files.len() == 0 {
            bail!("at least one file is required to extract subtitles")
        }

        let subtitles = if files.len() == 1 {
            let split_data =
                mp4decrypt::mp4split(&std::fs::read(&files[0])?).map_err(|x| anyhow!(x))?;

            let mut subtitles =
                MP4Subtitles::new(&split_data[0], self.timescale).map_err(|x| anyhow!(x))?;

            for data in &split_data[1..] {
                subtitles.add_cue(data).map_err(|x| anyhow!(x))?;
            }

            subtitles.to_subtitles()
        } else {
            let mut subtitles = MP4Subtitles::new(&std::fs::read(&files[0])?, self.timescale)
                .map_err(|x| anyhow!(x))?;

            for file in &files[1..] {
                subtitles
                    .add_cue(&std::fs::read(file)?)
                    .map_err(|x| anyhow!(x))?;
            }

            subtitles.to_subtitles()
        };

        print!(
            "{}",
            match &self.format {
                Format::SRT => subtitles.to_srt(),
                Format::VTT => subtitles.to_vtt(),
            }
        );

        Ok(())
    }
}
