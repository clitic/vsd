use crate::{
    playlist::{MediaPlaylist, MediaType},
    utils,
};
use anyhow::{Result, bail};
use colored::Colorize;
use log::{info, warn};
use std::{ffi::OsStr, path::PathBuf, process::Stdio};
use tokio::{fs, process::Command};

pub struct Stream {
    pub language: Option<String>,
    pub media_type: MediaType,
    pub path: PathBuf,
}

pub async fn delete_temp_files(directory: Option<&PathBuf>, temp_files: &[Stream]) -> Result<()> {
    for temp_file in temp_files {
        info!("Deleting {}", temp_file.path.to_string_lossy());
        fs::remove_file(&temp_file.path).await?;
    }

    if let Some(directory) = directory
        && directory.read_dir()?.next().is_none()
    {
        info!("Deleting {}", directory.to_string_lossy());
        fs::remove_dir(directory).await?;
    }

    Ok(())
}

pub async fn ffmpeg(
    output: Option<&PathBuf>,
    subs_codec: &str,
    temp_files: &[Stream],
) -> Result<()> {
    let output = output.unwrap();

    let sub_streams_present = temp_files
        .iter()
        .filter(|x| x.media_type == MediaType::Subtitles)
        .count()
        > 0;

    let temp_files = temp_files
        .iter()
        .filter(|x| x.media_type == MediaType::Video)
        .chain(
            temp_files
                .iter()
                .filter(|x| x.media_type == MediaType::Audio),
        )
        .chain(
            temp_files
                .iter()
                .filter(|x| x.media_type == MediaType::Subtitles),
        )
        .collect::<Vec<_>>();

    let mut args = Vec::new();

    for temp_file in &temp_files {
        args.extend_from_slice(&["-i".to_owned(), temp_file.path.to_string_lossy().into()]);
    }

    if args.len() == 2 {
        // Working on single stream
        args.extend_from_slice(&[
            "-c:v".to_owned(),
            "copy".to_owned(),
            "-c:a".to_owned(),
            "copy".to_owned(),
        ]);
    } else {
        // Working on multiple streams
        for i in 0..temp_files.len() {
            args.extend_from_slice(&["-map".to_owned(), i.to_string()]);
        }

        let mut audio_index = 0;
        let mut subtitle_index = 0;

        for temp_file in &temp_files {
            match temp_file.media_type {
                MediaType::Audio => {
                    if let Some(language) = &temp_file.language {
                        args.extend_from_slice(&[
                            format!("-metadata:s:a:{audio_index}"),
                            format!("language={language}"),
                        ]);
                    }

                    audio_index += 1;
                }
                MediaType::Subtitles => {
                    if let Some(language) = &temp_file.language {
                        args.extend_from_slice(&[
                            format!("-metadata:s:s:{subtitle_index}"),
                            format!("language={language}"),
                        ]);
                    }

                    subtitle_index += 1;
                }
                _ => (),
            }
        }

        if sub_streams_present {
            args.extend_from_slice(&["-disposition:s:0".to_owned(), "default".to_owned()]);

            if subs_codec == "copy" {
                if output.extension() == Some(OsStr::new("mp4")) {
                    args.extend_from_slice(&["-c:s".to_owned(), "mov_text".to_owned()]);
                } else {
                    args.extend_from_slice(&["-c:s".to_owned(), "copy".to_owned()]);
                }
            } else {
                args.extend_from_slice(&["-c:s".to_owned(), subs_codec.to_owned()]);
            }
        }

        args.extend_from_slice(&[
            "-c:v".to_owned(),
            "copy".to_owned(),
            "-c:a".to_owned(),
            "copy".to_owned(),
        ]);
    }

    args.push(output.to_string_lossy().into());

    if output.exists() {
        info!("Deleting {}", output.to_string_lossy());
        fs::remove_file(output).await?;
    }

    info!(
        "Executing {} {}",
        "ffmpeg".bold(),
        args.iter()
            .map(|x| if x.contains(' ') {
                format!("\"{x}\"")
            } else {
                x.to_owned()
            })
            .collect::<Vec<_>>()
            .join(" ")
            .bold()
    );

    let code = Command::new(utils::find_ffmpeg().unwrap())
        .args(args)
        .stderr(Stdio::null())
        .spawn()?
        .wait()
        .await?;

    if !code.success() {
        bail!("ffmpeg exited with code {}", code.code().unwrap_or(1));
    }

    Ok(())
}

pub fn should_mux(
    no_decrypt: bool,
    no_merge: bool,
    output: Option<&PathBuf>,
    streams: &[MediaPlaylist],
) -> bool {
    if output.is_none() {
        return false;
    }

    if no_decrypt {
        warn!("--output is ignored when --no-decrypt is used.");
        return false;
    }

    let subtitle_streams = streams
        .iter()
        .filter(|x| x.media_type == MediaType::Subtitles)
        .collect::<Vec<_>>();

    if no_merge && subtitle_streams.is_empty() {
        warn!("--output is ignored when --no-merge is used.");
        return false;
    }

    let video_streams = streams
        .iter()
        .filter(|x| x.media_type == MediaType::Video)
        .collect::<Vec<_>>();
    let audio_streams = streams
        .iter()
        .filter(|x| x.media_type == MediaType::Audio)
        .collect::<Vec<_>>();

    if video_streams.len() > 1 {
        warn!("--output flag is ignored when multiple video streams are selected.");
        return false;
    }

    if video_streams.is_empty()
        && (audio_streams.len() > 1
            || subtitle_streams.len() > 1
            || (!audio_streams.is_empty() && !subtitle_streams.is_empty()))
    {
        warn!(
            "--output is ignored when no video streams are selected but multiple audio/subtitle streams are selected."
        );
        return false;
    }

    if no_merge && !subtitle_streams.is_empty() {
        warn!("subtitle streams are always merged even if --no-merge is used.");
    }

    true
}
