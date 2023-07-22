use anyhow::{bail, Result};
use clap::{Args, ValueEnum};
use std::{
    fs,
    fs::File,
    io::{BufReader, Read, Write},
    process::Command,
};

/// Merge multiple segments to a single file.
#[derive(Debug, Clone, Args)]
pub struct Merge {
    /// List of files (at least 2) to merge together e.g. *.ts, *.m4s etc. .
    #[arg(required = true)]
    files: Vec<String>,

    /// Path for merged output file.
    #[arg(short, long, required = true)]
    output: String,

    /// Type of merge to be performed.
    #[arg(short, long, value_enum, default_value_t = MergeType::Binary)]
    _type: MergeType,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum MergeType {
    Binary,
    Ffmpeg,
}

impl Merge {
    pub fn execute(self) -> Result<()> {
        let mut files = vec![];

        for pattern in &self.files {
            for file in glob::glob(pattern)? {
                files.push(file?);
            }
        }

        if 1 >= files.len() {
            bail!("At least 2 files are required to merge together.")
        }

        match self._type {
            MergeType::Binary => {
                let mut output = File::create(self.output)?;
                let buffer_size = 1024 * 1024 * 2; // 2 MiB

                for file in files {
                    let file_size = fs::metadata(&file)?.len();

                    if buffer_size >= file_size {
                        let buf = fs::read(file)?;
                        output.write_all(&buf)?;
                    } else {
                        let file = File::open(file)?;
                        let mut reader = BufReader::new(file);

                        loop {
                            let mut buf = vec![];
                            reader.by_ref().take(buffer_size).read_to_end(&mut buf)?;

                            if buf.is_empty() {
                                break;
                            }

                            output.write_all(&buf)?;
                        }
                    }
                }
            }
            MergeType::Ffmpeg => {
                let concat_file = "vsd-ffmpeg-concat.txt";
                let mut concat = File::create(concat_file)?;

                for file in files {
                    let file = file.to_string_lossy();

                    if file != self.output {
                        concat.write_fmt(format_args!("file '{}'\n", file))?;
                    }
                }

                let status = Command::new("ffmpeg")
                    .args([
                        "-hide_banner",
                        "-y",
                        "-f",
                        "concat",
                        "-i",
                        concat_file,
                        "-c",
                        "copy",
                        &self.output,
                    ])
                    .spawn()?
                    .wait()?;

                if !status.success() {
                    bail!("ffmpeg exited with code {}.", status.code().unwrap_or(1))
                }

                fs::remove_file(concat_file)?;
            }
        }

        Ok(())
    }
}
