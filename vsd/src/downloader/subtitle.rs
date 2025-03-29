use super::mux::Stream;
use crate::{playlist::{MediaPlaylist, MediaType}, utils};
use anyhow::{anyhow, bail, Result};
use kdam::{term::Colorizer, BarExt, Column, RichProgress};
use reqwest::{blocking::Client, header, Url};
use std::{fs::File, io::Write, path::PathBuf};
use vsd_mp4::text::{ttml_text_parser, Mp4TtmlParser, Mp4VttParser};

enum SubtitleType {
    Mp4Vtt,
    Mp4Ttml,
    SrtText,
    TtmlText,
    VttText,
}

pub fn download_subtitle_stream(
    base_url: &Option<Url>,
    client: &Client,
    directory: &Option<PathBuf>,
    stream: &MediaPlaylist,
    pb: &mut RichProgress,
    temp_files: &mut Vec<Stream>,
) -> Result<()> {
    pb.write(format!(
        " {} {} stream {}",
        "Processing".colorize("bold green"),
        stream.media_type,
        stream.display_stream().colorize("cyan"),
    ))?;

    let length = stream.segments.len();

    if length == 0 {
        pb.write(format!(
            "    {} skipping stream (no segments)",
            "Warning".colorize("bold yellow"),
        ))?;
        return Ok(());
    }

    pb.pb.total = length;

    let mut ext = stream.extension();
    let mut codec = None;

    if let Some(codecs) = &stream.codecs {
        match codecs.as_str() {
            "vtt" => {
                ext = "vtt".to_owned();
                codec = Some(SubtitleType::VttText);
            }
            "wvtt" => {
                ext = "vtt".to_owned();
                codec = Some(SubtitleType::Mp4Vtt);
            }
            "stpp" | "stpp.ttml" | "stpp.ttml.im1t" | "stpp.TTML.im1t" => {
                ext = "srt".to_owned();
                codec = Some(SubtitleType::Mp4Ttml);
            }
            _ => (),
        }
    }
    let mut temp_file = PathBuf::new();

    let mut first_run = true;
    let mut subtitles_data = vec![];

    let stream_base_url = base_url
        .clone()
        .unwrap_or(stream.uri.parse::<Url>().unwrap());

    for segment in &stream.segments {
        if let Some(map) = &segment.map {
            let url = stream_base_url.join(&map.uri)?;
            let mut request = client.get(url);

            if let Some(range) = &map.range {
                request = request.header(header::RANGE, range.as_header_value());
            }

            let response = request.send()?;
            let bytes = response.bytes()?;
            subtitles_data.extend_from_slice(&bytes);
        }

        let url = stream_base_url.join(&segment.uri)?;
        let mut request = client.get(url);

        if let Some(range) = &segment.range {
            request = request.header(header::RANGE, range.as_header_value());
        }

        let response = request.send()?;
        let bytes = response.bytes()?;
        subtitles_data.extend_from_slice(&bytes);

        if first_run {
            first_run = false;

            if subtitles_data.starts_with(b"WEBVTT") {
                ext = "vtt".to_owned();
                codec = Some(SubtitleType::VttText);
            } else if subtitles_data.starts_with(b"1") {
                ext = "srt".to_owned();
                codec = Some(SubtitleType::SrtText);
            } else if subtitles_data.starts_with(b"<?xml") || subtitles_data.starts_with(b"<tt") {
                ext = "srt".to_owned();
                codec = Some(SubtitleType::TtmlText);
            } else if codec.is_none() {
                bail!("could'nt determine subtitle codec.");
            }

            temp_file = stream.file_path(directory, &ext);
            temp_files.push(Stream {
                language: stream.language.clone(),
                media_type: stream.media_type.clone(),
                path: temp_file.clone(),
            });
            pb.write(format!(
                "{} stream to {}",
                "Downloading".colorize("bold green"),
                temp_file.to_string_lossy().colorize("cyan")
            ))?;
        }

        pb.replace(
            0,
            Column::Text(format!(
                "[bold blue]{}",
                utils::format_bytes(subtitles_data.len(), 2).2
            )),
        );
        pb.update(1)?;
    }

    match codec {
        Some(SubtitleType::Mp4Vtt) => {
            pb.write(format!(
                " {} wvtt subtitles",
                "Extracting".colorize("bold cyan"),
            ))?;

            let vtt = Mp4VttParser::parse_init(&subtitles_data)?;
            let subtitles = vtt.parse_media(&subtitles_data, None)?;
            File::create(&temp_file)?.write_all(subtitles.as_vtt().as_bytes())?;
        }
        Some(SubtitleType::Mp4Ttml) => {
            pb.write(format!(
                " {} stpp subtitles",
                "Extracting".colorize("bold cyan"),
            ))?;

            let ttml = Mp4TtmlParser::parse_init(&subtitles_data)?;
            let subtitles = ttml.parse_media(&subtitles_data)?;
            File::create(&temp_file)?.write_all(subtitles.as_srt().as_bytes())?;
        }
        Some(SubtitleType::TtmlText) => {
            pb.write(format!(
                " {} ttml+xml subtitles",
                "Extracting".colorize("bold cyan"),
            ))?;

            let xml = String::from_utf8(subtitles_data)
                .map_err(|_| anyhow!("cannot decode subtitles as valid utf-8 data."))?;
            let ttml = ttml_text_parser::parse(&xml).map_err(|x| {
                anyhow!(
                    "couldn't parse xml string as ttml content.\n\n{}\n\n{:#?}",
                    xml,
                    x,
                )
            })?;
            File::create(&temp_file)?.write_all(ttml.into_subtitles().as_srt().as_bytes())?;
        }
        _ => File::create(&temp_file)?.write_all(&subtitles_data)?,
    };

    pb.write(format!(
        " {} stream successfully",
        "Downloaded".colorize("bold green"),
    ))?;
    eprintln!();
    pb.reset(Some(0));
    Ok(())
}

pub fn download_subtitle_streams(
    base_url: &Option<Url>,
    client: &Client,
    directory: &Option<PathBuf>,
    streams: &[MediaPlaylist],
    pb: &mut RichProgress,
    temp_files: &mut Vec<Stream>,
) -> Result<()> {
    for stream in streams {
        if stream.media_type == MediaType::Subtitles {
            download_subtitle_stream(base_url, client, directory, stream, pb, temp_files)?;
        }
    }

    Ok(())
}
