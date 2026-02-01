use anyhow::{Result, bail};
use clap::{Args, ValueEnum};
use std::path::PathBuf;
use tokio::{
    fs::{self, File},
    io::{self, AsyncWriteExt, BufReader},
    process::Command,
};

/// Merge multiple media segments into a single file.
#[derive(Args, Clone, Debug)]
pub struct Merge {
    /// Glob patterns for input files (e.g., `*.ts`, `segment_*.m4s`).
    ///
    /// At least two files must match the provided patterns.
    #[arg(required = true)]
    input: Vec<String>,

    /// Destination path for the merged output file.
    #[arg(short, long, required = true)]
    output: PathBuf,

    /// Merge strategy to use.
    ///
    /// `binary` performs a raw byte concatenation, while `ffmpeg` uses
    /// ffmpeg's concat demuxer for container-aware merging.
    #[arg(short = 't', long = "type", value_enum, default_value_t = MergeType::Binary)]
    typ: MergeType,
}

#[derive(Debug, Clone, ValueEnum)]
enum MergeType {
    Binary,
    FFmpeg,
}

impl Merge {
    pub async fn execute(self) -> Result<()> {
        let output_canonical = self.output.canonicalize().ok();
        let mut files = self
            .input
            .iter()
            .filter_map(|p| glob::glob(p).ok())
            .flatten()
            .filter_map(|res| res.ok())
            .filter(|f| {
                if let Some(out) = output_canonical.as_ref() {
                    return f.canonicalize().ok().as_ref() != Some(out);
                }
                true
            })
            .collect::<Vec<_>>();
        files.sort();

        if files.len() < 2 {
            bail!("At least two files are required to perform a merge.");
        }

        match self.typ {
            MergeType::Binary => {
                const BUFFER_SIZE: usize = 1024 * 1024 * 10;
                let mut output = File::create(self.output).await?;

                for path in files {
                    let file = File::open(path).await?;
                    let mut reader = BufReader::with_capacity(BUFFER_SIZE, file);
                    io::copy(&mut reader, &mut output).await?;
                }

                output.flush().await?;
            }
            MergeType::FFmpeg => {
                let content = files
                    .iter()
                    .map(|f| format!("file '{}'", f.to_string_lossy().replace('\'', "'\\''")))
                    .collect::<Vec<_>>()
                    .join("\n");

                fs::write("vsd-merge.txt", content).await?;
                let status = Command::new("ffmpeg")
                    .args([
                        "-hide_banner",
                        "-loglevel",
                        "error",
                        "-y",
                        "-f",
                        "concat",
                        "-safe",
                        "0",
                        "-i",
                        "vsd-merge.txt",
                        "-c",
                        "copy",
                        &self.output.to_string_lossy(),
                    ])
                    .status()
                    .await?;
                fs::remove_file("vsd-merge.txt").await?;

                if !status.success() {
                    bail!("FFmpeg exited with code {}.", status.code().unwrap_or(1))
                }
            }
        }

        Ok(())
    }
}
