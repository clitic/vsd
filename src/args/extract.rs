use crate::subtitles::MP4Subtitles;
use anyhow::{anyhow, Result};
use clap::{ArgEnum, Args};

#[derive(Debug, Clone, ArgEnum)]
pub enum Format {
    VTT,
    SRT,
}


//   -timescale TIMESCALE, --timescale TIMESCALE
//                         set timescale manually if no init segment
//   -init-path INIT_PATH, --init-path INIT_PATH
//                         init segment path
//   -segment-time SEGMENT_TIME, --segment-time SEGMENT_TIME
//                         single segment duration, usually needed for ttml
//                         content, calculation method: d / timescale

// https://github.com/xhlove/dash-subtitle-extractor

/// Extract subtitles embedded inside an mp4 file.
#[derive(Debug, Clone, Args)]
pub struct Extract {
    /// List of segment files like init.mp4, *.m4s etc.
    /// A single file mp4 can also be provided.
    #[clap(required = true)]
    pub files: Vec<String>,

    /// Subtitles output format.
    #[clap(long, arg_enum, default_value_t = Format::SRT)]
    pub format: Format,
}

impl Extract {
    pub fn perform(&self) -> Result<()> {
        let mut subtitles_data = vec![];

        for pattern in &self.files {
            for file in glob::glob(pattern)? {
                subtitles_data.extend_from_slice(&std::fs::read(file?)?);
            }
        }

        let split_data = mp4decrypt::mp4split(&subtitles_data).map_err(|x| anyhow!(x))?;

        let subtitles = MP4Subtitles::from_init(&split_data[0])
            .map_err(|x| anyhow!(x))?
            .to_subtitles(&split_data[1..])
            .map_err(|x| anyhow!(x))?;

        match &self.format {
            Format::VTT => print!("{}", subtitles.to_vtt()),
            Format::SRT => print!("{}", subtitles.to_srt()),
        }

        Ok(())
    }
}
