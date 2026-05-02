use super::{MAX_THREADS, mux::Stream};
use crate::{
    playlist::{MediaPlaylist, MediaType},
    progress::Progress,
    utils,
};
use anyhow::Result;
use colored::Colorize;
use log::{debug, error, info, warn};
use reqwest::{Client, Url, header};
use std::{path::PathBuf, sync::atomic::Ordering};
use tokio::{fs::File, io::AsyncWriteExt, task::JoinSet};
use vsd_mp4::text::{Mp4TtmlParser, Mp4VttParser, ttml_text_parser};

enum SubtitleType {
    Mp4Vtt,
    Mp4Ttml,
    SrtText,
    TtmlText,
    Unknown,
    VttText,
}

fn detect_codec(codecs: Option<&str>, data: &[u8], ext: &str) -> (&'static str, SubtitleType) {
    if let Some(codecs) = codecs {
        match codecs.to_lowercase().as_str() {
            "vtt" => return ("vtt", SubtitleType::VttText),
            "wvtt" => return ("vtt", SubtitleType::Mp4Vtt),
            "stpp" | "stpp.ttml" | "stpp.ttml.im1t" => return ("srt", SubtitleType::Mp4Ttml),
            _ => (),
        }
    }

    if data.starts_with(b"WEBVTT") || ext == "vtt" {
        ("vtt", SubtitleType::VttText)
    } else if data.starts_with(b"1") || ext == "srt" {
        ("srt", SubtitleType::SrtText)
    } else if data.starts_with(b"<?xml") || data.starts_with(b"<tt") || ext == "ttml" {
        ("srt", SubtitleType::TtmlText)
    } else if Mp4VttParser::from_init(data).is_ok() {
        ("vtt", SubtitleType::Mp4Vtt)
    } else if Mp4TtmlParser::from_init(data).is_ok() {
        ("srt", SubtitleType::Mp4Ttml)
    } else {
        warn!("Stream uses unknown subtitle codec.");
        ("txt", SubtitleType::Unknown)
    }
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
        warn!("Stream skipped because no segments were found.");
        return Ok(());
    }

    let base_url = base_url
        .clone()
        .unwrap_or(stream.uri.parse::<Url>().unwrap());
    let segment = &stream.segments[0];
    let mut data = Vec::new();
    let ext = stream.extension();
    let mut temp_file = stream.path(directory);

    if let Some(map) = &segment.map {
        let url = base_url.join(&map.uri)?;
        let mut request = client.get(url).query(query);

        if let Some(range) = &map.range {
            request = request.header(header::RANGE, range);
        }

        let response = request.send().await?;
        let mut bytes = utils::fetch_bytes(response).await?;
        data.append(&mut bytes);
    }

    let url = base_url.join(&segment.uri)?;
    let mut request = client.get(url).query(query);

    if let Some(range) = &segment.range {
        request = request.header(header::RANGE, range);
    }

    let response = request.send().await?;
    let mut bytes = utils::fetch_bytes(response).await?;
    let size = bytes.len();
    data.append(&mut bytes);

    let (ext, codec) = detect_codec(stream.codecs.as_deref(), &data, ext);

    temp_file = temp_file.with_extension(ext);
    temp_files.push(Stream {
        language: stream.language.clone(),
        media_type: stream.media_type.clone(),
        path: temp_file.clone(),
    });
    info!("Saving [{}] {}", "sub".green(), temp_file.to_string_lossy());
    pb.update(size);

    let remaining = &stream.segments[1..];

    if !remaining.is_empty() {
        let max_threads = MAX_THREADS.load(Ordering::SeqCst) as usize;
        let mut set: JoinSet<(usize, Vec<u8>)> = JoinSet::new();
        let mut results = vec![None; remaining.len()];

        for (i, segment) in remaining.iter().enumerate() {
            while set.len() >= max_threads {
                if let Some(Ok((i, bytes))) = set.join_next().await {
                    pb.update(bytes.len());
                    results[i] = Some(bytes);
                }
            }

            let url = base_url.join(&segment.uri)?;
            let mut request = client.get(url).query(query);

            if let Some(range) = &segment.range {
                request = request.header(header::RANGE, range);
            }

            set.spawn(async move {
                let response = request.send().await.unwrap_or_else(|e| {
                    error!("{}", e);
                    std::process::exit(1);
                });
                let bytes = utils::fetch_bytes(response).await.unwrap_or_else(|e| {
                    error!("{}", e);
                    std::process::exit(1);
                });
                (i, bytes)
            });
        }

        while let Some(Ok((i, bytes))) = set.join_next().await {
            pb.update(bytes.len());
            results[i] = Some(bytes);
        }

        for result in results {
            if let Some(mut bytes) = result {
                data.append(&mut bytes);
            }
        }
    }

    eprintln!();

    let output = match codec {
        SubtitleType::Mp4Vtt => {
            debug!("Extracting wvtt subtitles.");
            let vtt = Mp4VttParser::from_init(&data)?;
            vtt.parse(&data, None)?.as_vtt().into_bytes()
        }
        SubtitleType::Mp4Ttml => {
            debug!("Extracting stpp subtitles.");
            let ttml = Mp4TtmlParser::from_init(&data)?;
            ttml.parse(&data)?.as_srt().into_bytes()
        }
        SubtitleType::TtmlText => {
            debug!("Extracting ttml+xml subtitles.");
            ttml_text_parser::parse_bytes(&data)?
                .into_subtitles()
                .as_srt()
                .into_bytes()
        }
        _ => data,
    };

    File::create(&temp_file)
        .await?
        .write_all(&output)
        .await?;

    Ok(())
}
