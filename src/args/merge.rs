use anyhow::{bail, Result};
use clap::Args;
use std::fs::File;
use std::io::Write;
use std::path::Path;

/// Merge multiple segments to a single file.
#[derive(Debug, Clone, Args)]
pub struct Merge {
    /// List of files to merge together like *.ts, *.m4s etc.
    #[clap(required = true)]
    pub files: Vec<String>,

    /// Path  of merged output file.
    #[clap(short, long, required = true)]
    pub output: String,

    /// Merge using ffmpeg instead of binary merge.
    #[clap(long)]
    pub ffmpeg: bool,
}

impl Merge {
    pub fn perform(&self) -> Result<()> {
        let mut files = vec![];

        for pattern in &self.files {
            for file in glob::glob(pattern)? {
                files.push(file?);
            }
        }

        if files.len() <= 1 {
            bail!("at least two files are required to merge together")
        }

        if self.ffmpeg {
            let mut concat_file = "ffmpeg-concat.txt".to_owned();

            for i in 1.. {
                if Path::new(&concat_file).exists() {
                    concat_file = format!("ffmpeg-concat-{}.txt", i);
                } else {
                    break;
                }
            }

            let mut concat = File::create(&concat_file)?;

            for file in files {
                let file = file.to_str().unwrap();

                if file != self.output {
                    concat.write_all(format!("file '{}'\n", file).as_bytes())?;
                }
            }

            let code = std::process::Command::new("ffmpeg")
                .args([
                    "-hide_banner",
                    "-y",
                    "-f",
                    "concat",
                    "-i",
                    &concat_file,
                    "-c",
                    "copy",
                    &self.output,
                ])
                .spawn()?
                .wait()?;

            if !code.success() {
                bail!("FFMPEG exited with code {}", code.code().unwrap_or(1))
            }

            std::fs::remove_file(&concat_file)?;
        } else {
            let mut merged = File::create(&self.output)?;

            for file in files {
                merged.write_all(&std::fs::read(file)?)?;
            }
        }

        Ok(())
    }
}
