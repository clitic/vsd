use anyhow::{Result, bail};
use clap::{Args, ValueEnum};
use std::path::PathBuf;
use tokio::fs;
use vsd_mp4::text::{Mp4TtmlParser, Mp4VttParser};

/// Extract subtitles from a fragmented MP4 file.
#[derive(Args, Clone, Debug)]
pub struct Extract {
    /// Path to an MP4 file containing WVTT (WebVTT) or STPP (TTML) subtitle boxes.
    /// For fragmented MP4 files split across multiple segments, use the `merge`
    /// sub-command first to combine them into a single file.
    #[arg(required = true)]
    input: PathBuf,

    /// Output subtitle format.
    #[arg(short, long, value_enum, default_value_t = Codec::Webvtt)]
    codec: Codec,

    /// Destination file path for extracted subtitles.
    ///
    /// If `provided`, the codec is inferred from the file extension (`.srt` or `.vtt`).
    /// If `omitted`, subtitles are printed to stdout.
    #[arg(short, long, value_name = "PATH")]
    output: Option<PathBuf>,
}

#[derive(Clone, Debug, ValueEnum)]
enum Codec {
    Subrip,
    Webvtt,
}

impl Extract {
    pub async fn execute(self) -> Result<()> {
        let data = fs::read(self.input).await?;
        let subtitles;

        if let Ok(vtt) = Mp4VttParser::from_init(&data) {
            subtitles = vtt.parse(&data, None)?;
        } else if let Ok(ttml) = Mp4TtmlParser::from_init(&data) {
            subtitles = ttml.parse(&data)?;
        } else {
            bail!("Unable to determine the subtitle codec: neither WVTT nor STPP box was found.");
        }

        if let Some(path) = self.output {
            let ext = path
                .extension()
                .and_then(|x| match x.to_str() {
                    Some("srt") => Some(Codec::Subrip),
                    Some("vtt") => Some(Codec::Webvtt),
                    _ => None,
                })
                .unwrap_or(self.codec);
            fs::write(
                &path,
                match ext {
                    Codec::Subrip => subtitles.as_srt(),
                    Codec::Webvtt => subtitles.as_vtt(),
                },
            )
            .await?;
        } else {
            print!(
                "{}",
                match &self.codec {
                    Codec::Subrip => subtitles.as_srt(),
                    Codec::Webvtt => subtitles.as_vtt(),
                }
            );
        }

        Ok(())
    }
}
