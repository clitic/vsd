use crate::{
    playlist::{MediaPlaylist, MediaType},
    utils,
};
use anyhow::{bail, Result};
use kdam::term::Colorizer;
use std::{
    ffi::OsStr,
    fs,
    path::PathBuf,
    process::{Command, Stdio},
};

pub struct Stream {
    pub language: Option<String>,
    pub media_type: MediaType,
    pub path: PathBuf,
}

pub fn should_mux(
    no_decrypt: bool,
    no_merge: bool,
    streams: &[MediaPlaylist],
    output: Option<&PathBuf>,
) -> bool {
    if output.is_none() {
        return false;
    }

    if no_decrypt {
        println!(
            "    {} --output is ignored when --no-decrypt is used",
            "Warning".colorize("bold yellow")
        );
        return false;
    }

    let subtitle_streams = streams
        .iter()
        .filter(|x| x.media_type == MediaType::Subtitles)
        .collect::<Vec<_>>();

    if no_merge && subtitle_streams.is_empty() {
        println!(
            "    {} --output is ignored when --no-merge is used",
            "Warning".colorize("bold yellow")
        );
        return false;
    }

    let output = output.unwrap();

    // Check if output file extension matches with actual stream file extension.
    if streams.len() == 1 && output.extension() == Some(OsStr::new(&streams.first().unwrap().extension())) {
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
        println!(
            "    {} --output flag is ignored when multiple video streams are selected",
            "Warning".colorize("bold yellow")
        );
        return false;
    }

    if video_streams.is_empty()
        && (audio_streams.len() > 1
            || subtitle_streams.len() > 1
            || (!audio_streams.is_empty() && !subtitle_streams.is_empty()))
    {
        println!(
            "    {} --output is ignored when no video streams are selected but multiple audio/subtitle streams are selected",
            "Warning".colorize("bold yellow")
        );
        return false;
    }

    if no_merge && !subtitle_streams.is_empty() {
        println!(
            "    {} subtitle streams are always merged even if --no-merge is used",
            "Warning".colorize("bold yellow")
        );
    }

    true
}

pub fn ffmpeg(output: Option<&PathBuf>, temp_files: &[Stream]) -> Result<()> {
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
                            format!("-metadata:s:a:{}", audio_index),
                            format!("language={}", language),
                        ]);
                    }

                    audio_index += 1;
                }
                MediaType::Subtitles => {
                    if let Some(language) = &temp_file.language {
                        args.extend_from_slice(&[
                            format!("-metadata:s:s:{}", subtitle_index),
                            format!("language={}", language),
                        ]);
                    }

                    subtitle_index += 1;
                }
                _ => (),
            }
        }

        if sub_streams_present {
            args.extend_from_slice(&["-disposition:s:0".to_owned(), "default".to_owned()]);
        }

        if sub_streams_present && output.extension() == Some(OsStr::new("mp4")) {
            args.extend_from_slice(&["-c:s".to_owned(), "mov_text".to_owned()]);
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
        println!(
            "   {} {}",
            "Deleting".colorize("bold red"),
            output.to_string_lossy()
        );
        fs::remove_file(output)?;
    }

    println!(
        "  {} ffmpeg {}",
        "Executing".colorize("bold cyan"),
        args.iter()
            .map(|x| if x.contains(' ') {
                format!("\"{}\"", x)
            } else {
                x.to_owned()
            })
            .collect::<Vec<_>>()
            .join(" ")
    );

    let code = Command::new(utils::find_ffmpeg().unwrap())
        .args(args)
        .stderr(Stdio::null())
        .spawn()?
        .wait()?;

    if !code.success() {
        bail!("ffmpeg exited with code {}", code.code().unwrap_or(1));
    }

    Ok(())
}

pub fn delete_temp_files(directory: Option<&PathBuf>, temp_files: &[Stream]) -> Result<()> {
    for temp_file in temp_files {
        println!(
            "   {} {}",
            "Deleting".colorize("bold red"),
            temp_file.path.to_string_lossy()
        );
        fs::remove_file(&temp_file.path)?;
    }

    if let Some(directory) = directory {
        if directory.read_dir()?.next().is_none() {
            println!(
                "   {} {}",
                "Deleting".colorize("bold red"),
                directory.to_string_lossy()
            );
            fs::remove_dir(directory)?;
        }
    }

    Ok(())
}
