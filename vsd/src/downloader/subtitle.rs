use super::mux::Stream;
use crate::{
    playlist::{MediaPlaylist, MediaType},
    progress::Progress,
};
use anyhow::Result;
use colored::Colorize;
use log::{info, warn};
use reqwest::{Client, Url, header};
use std::path::PathBuf;
use tokio::{fs::File, io::AsyncWriteExt};
use vsd_mp4::text::{Mp4TtmlParser, Mp4VttParser, ttml_text_parser};

enum SubtitleType {
    Mp4Vtt,
    Mp4Ttml,
    SrtText,
    TtmlText,
    Unknown,
    VttText,
}

pub async fn download_subtitle_streams(
    client: &Client,
    streams: &[MediaPlaylist],
    base_url: &Option<Url>,
    query: &Vec<(String, String)>,
    directory: Option<&PathBuf>,
    temp_files: &mut Vec<Stream>,
) -> Result<()> {
    let mut i = 0;
    let total = streams.len();

    for stream in streams {
        if stream.media_type == MediaType::Subtitles {
            i += 1;
            download_subtitle_stream(
                client,
                stream,
                base_url,
                query,
                directory,
                temp_files,
                Progress::new(&format!("{}/{}", i, total), stream.segments.len()),
            )
            .await?;
        }
    }

    Ok(())
}

async fn download_subtitle_stream(
    client: &Client,
    stream: &MediaPlaylist,
    base_url: &Option<Url>,
    query: &Vec<(String, String)>,
    directory: Option<&PathBuf>,
    temp_files: &mut Vec<Stream>,
    pb: Progress,
) -> Result<()> {
    info!(
        "DownLD [{}] {}",
        stream.media_type.to_string().green(),
        stream.display().cyan(),
    );

    if stream.segments.is_empty() {
        warn!("Skipping stream (no segments)",);
        return Ok(());
    }

    let mut ext = stream.extension();
    let mut codec = None;

    if let Some(codecs) = &stream.codecs {
        match codecs.to_lowercase().as_str() {
            "vtt" => {
                ext = "vtt";
                codec = Some(SubtitleType::VttText);
            }
            "wvtt" => {
                ext = "vtt";
                codec = Some(SubtitleType::Mp4Vtt);
            }
            "stpp" | "stpp.ttml" | "stpp.ttml.im1t" => {
                ext = "srt";
                codec = Some(SubtitleType::Mp4Ttml);
            }
            _ => (),
        }
    }

    let mut temp_file = stream.path(directory);
    let mut first_run = true;
    let mut subs_data = vec![];

    let stream_base_url = base_url
        .clone()
        .unwrap_or(stream.uri.parse::<Url>().unwrap());

    for segment in &stream.segments {
        if let Some(map) = &segment.map {
            let url = stream_base_url.join(&map.uri)?;
            let mut request = client.get(url).query(query);

            if let Some(range) = &map.range {
                request = request.header(header::RANGE, range);
            }

            let response = request.send().await?;
            let bytes = response.bytes().await?;
            subs_data.extend_from_slice(&bytes);
        }

        let url = stream_base_url.join(&segment.uri)?;
        let mut request = client.get(url).query(query);

        if let Some(range) = &segment.range {
            request = request.header(header::RANGE, range);
        }

        let response = request.send().await?;
        let bytes = response.bytes().await?;
        subs_data.extend_from_slice(&bytes);

        if first_run {
            if codec.is_none() {
                if subs_data.starts_with(b"WEBVTT") || ext == "vtt" {
                    ext = "vtt";
                    codec = Some(SubtitleType::VttText);
                } else if subs_data.starts_with(b"1") || ext == "srt" {
                    ext = "srt";
                    codec = Some(SubtitleType::SrtText);
                } else if subs_data.starts_with(b"<?xml")
                    || subs_data.starts_with(b"<tt")
                    || ext == "ttml"
                {
                    ext = "srt";
                    codec = Some(SubtitleType::TtmlText);
                } else if Mp4VttParser::from_init(&subs_data).is_ok() {
                    ext = "vtt";
                    codec = Some(SubtitleType::Mp4Vtt);
                } else if Mp4TtmlParser::from_init(&subs_data).is_ok() {
                    ext = "srt";
                    codec = Some(SubtitleType::Mp4Ttml);
                } else {
                    warn!("Unknown subtitle codec used",);
                    ext = "txt";
                    codec = Some(SubtitleType::Unknown);
                }
            }

            first_run = false;
            temp_file = temp_file.with_extension(ext);
            temp_files.push(Stream {
                language: stream.language.clone(),
                media_type: stream.media_type.clone(),
                path: temp_file.clone(),
            });
            info!("Saving [{}] {}", "sub".green(), temp_file.to_string_lossy());
        }

        pb.update(bytes.len());
    }

    eprintln!();

    match codec {
        Some(SubtitleType::Mp4Vtt) => {
            info!("Xtract [{}] wvtt", "sub".cyan());
            let vtt = Mp4VttParser::from_init(&subs_data)?;
            let subs = vtt.parse(&subs_data, None)?;
            File::create(&temp_file)
                .await?
                .write_all(subs.as_vtt().as_bytes())
                .await?;
        }
        Some(SubtitleType::Mp4Ttml) => {
            info!("Xtract [{}] stpp", "sub".cyan());
            let ttml = Mp4TtmlParser::from_init(&subs_data)?;
            let subs = ttml.parse(&subs_data)?;
            File::create(&temp_file)
                .await?
                .write_all(subs.as_srt().as_bytes())
                .await?;
        }
        Some(SubtitleType::TtmlText) => {
            info!("Xtract [{}] ttml+xml", "sub".cyan());
            let ttml = ttml_text_parser::parse_bytes(&subs_data)?;
            File::create(&temp_file)
                .await?
                .write_all(ttml.into_subtitles().as_srt().as_bytes())
                .await?;
        }
        _ => {
            File::create(&temp_file)
                .await?
                .write_all(&subs_data)
                .await?
        }
    };
    Ok(())
}
