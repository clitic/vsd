use crate::error::{Error, Result};
use clap::{Args, ValueEnum};
use std::{
    fs,
    fs::File,
    io::{BufReader, Read, Write},
    process::Command,
};

/// Merge multiple segments to a single file.
#[derive(Args, Clone, Debug)]
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

#[derive(Clone, Debug, ValueEnum)]
pub enum MergeType {
    Binary,
    Ffmpeg,
}

impl Merge {
    pub fn execute(self) -> Result<()> {
        let mut files = vec![];

        for pattern in &self.files {
            for file in glob::glob(pattern).map_err(|_| Error::new("glob pattern is invalid."))? {
                files.push(file.map_err(|_| Error::new_io("glob file couln't be read."))?);
            }
        }

        if 1 >= files.len() {
            return Err("at least 2 files are required to merge together.".into());
        }

        match self._type {
            MergeType::Binary => {
                let mut output = File::create(self.output)
                    .map_err(|_| Error::new_io("cannot create output file."))?;
                let buffer_size = 1024 * 1024 * 2; // 2 MiB

                for file in files {
                    let file_size = fs::metadata(&file)
                        .map_err(|_| {
                            Error::new_io(format!(
                                "metadata of `{}` file couldn't be read.",
                                file.to_string_lossy()
                            ))
                        })?
                        .len();

                    if buffer_size >= file_size {
                        let buf = fs::read(file).map_err(|_| {
                            Error::new_io(format!(
                                "`{}` file couldn't be read.",
                                file.to_string_lossy()
                            ))
                        })?;
                        output
                            .write_all(&buf)
                            .map_err(|_| Error::new_io("cannot write to output file."))?;
                    } else {
                        let mut reader = BufReader::new(File::open(file).map_err(|_| {
                            Error::new_io(format!(
                                "cannot open `{}` file for reading.",
                                file.to_string_lossy()
                            ))
                        })?);

                        loop {
                            let mut buf = vec![];
                            reader
                                .by_ref()
                                .take(buffer_size)
                                .read_to_end(&mut buf)
                                .map_err(|_| {
                                    Error::new_io(format!(
                                        "cannot read `{}` file.",
                                        file.to_string_lossy()
                                    ))
                                })?;

                            if buf.is_empty() {
                                break;
                            }

                            output
                                .write_all(&buf)
                                .map_err(|_| Error::new_io("cannot write to output file."))?;
                        }
                    }
                }
            }
            MergeType::Ffmpeg => {
                let concat_file = "vsd-ffmpeg-concat.txt";
                let mut concat = File::create(concat_file)
                    .map_err(|_| Error::new_io("cannot create ffmpeg concat file."))?;

                for file in files {
                    let file = file.to_string_lossy();

                    if file != self.output {
                        concat
                            .write_fmt(format_args!("file '{}'\n", file))
                            .map_err(|_| Error::new_io("cannot write to ffmpeg concat file."))?;
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
                    .spawn()
                    .map_err(|_| Error::new_io("cannot spawn ffmpeg process."))?
                    .wait()
                    .map_err(|_| Error::new_io("cannot wait for ffmpeg process."))?;

                if !status.success() {
                    return Err(
                        format!("ffmpeg exited with code {}.", status.code().unwrap_or(1)).into(),
                    );
                }

                fs::remove_file(concat_file)
                    .map_err(|_| Error::new_io("cannot remove ffmpeg concat file."))?;
            }
        }

        Ok(())
    }
}
