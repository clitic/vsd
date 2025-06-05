use std::collections::HashMap;

use super::fetch::Metadata;
use crate::{
    playlist::{AutomationOptions, MasterPlaylist, MediaPlaylist, PlaylistType},
    utils,
};
use anyhow::{anyhow, bail, Result};
use kdam::term::Colorizer;
use reqwest::{blocking::Client, Url};

pub fn list_all_streams(meta: &Metadata) -> Result<()> {
    match meta.pl_type {
        Some(PlaylistType::Dash) => {
            let mpd = dash_mpd::parse(&meta.text)
                .map_err(|_| anyhow!("couldn't parse response ({}) as dash playlist.", meta.url))?;
            crate::dash::parse_as_master(&mpd, meta.url.as_ref()).sort_streams().list_streams();
        }
        Some(PlaylistType::Hls) => match m3u8_rs::parse_playlist_res(meta.text.as_bytes()) {
            Ok(m3u8_rs::Playlist::MasterPlaylist(m3u8)) => {
                crate::hls::parse_as_master(&m3u8, meta.url.as_ref()).sort_streams().list_streams()
            }

            Ok(m3u8_rs::Playlist::MediaPlaylist(_)) => {
                println!("------ {} ------", "Undefined Streams".colorize("cyan"));
                println!(" 1) {}", meta.url);
            }
            Err(_) => bail!("couldn't parse response ({}) as hls playlist.", meta.url),
        },
        _ => bail!("couldn't determine playlist type, only DASH and HLS playlists are supported."),
    }
    Ok(())
}

pub fn parse_all_streams(
    base_url: Option<Url>,
    client: &Client,
    meta: &Metadata,
    query: &HashMap<String, String>,
) -> Result<MasterPlaylist> {
    match meta.pl_type {
        Some(PlaylistType::Dash) => {
            let mpd = dash_mpd::parse(&meta.text)
                .map_err(|_| anyhow!("couldn't parse response ({}) as dash playlist.", meta.url))?;
            let mut playlist = crate::dash::parse_as_master(&mpd, meta.url.as_ref());

            for stream in playlist.streams.iter_mut() {
                crate::dash::push_segments(
                    &mpd,
                    stream,
                    base_url.as_ref().unwrap_or(&meta.url).as_str(),
                )?;
                stream.uri = meta.url.as_ref().to_owned();
            }

            Ok(playlist)
        }
        Some(PlaylistType::Hls) => match m3u8_rs::parse_playlist_res(meta.text.as_bytes()) {
            Ok(m3u8_rs::Playlist::MasterPlaylist(m3u8)) => {
                let mut playlist = crate::hls::parse_as_master(&m3u8, meta.url.as_ref());

                for stream in playlist.streams.iter_mut() {
                    stream.uri = base_url
                        .as_ref()
                        .unwrap_or(&meta.url)
                        .join(&stream.uri)?
                        .to_string();

                    let text;
                    if let Some(bs) = stream
                        .uri
                        .strip_prefix("data:application/x-mpegurl;base64,")
                    {
                        let decoded = utils::decode_base64(bs)?;
                        text = String::from_utf8(decoded)?;
                    } else {
                        let response = client.get(&stream.uri).query(query).send()?;
                        text = response.text()?;
                    }

                    let media_playlist = m3u8_rs::parse_media_playlist_res(text.as_bytes())
                        .map_err(|_| {
                            anyhow!("couldn't parse response ({}) as hls playlist.", meta.url)
                        })?;
                    crate::hls::push_segments(&media_playlist, stream);
                }

                Ok(playlist)
            }
            Ok(m3u8_rs::Playlist::MediaPlaylist(m3u8)) => {
                let mut media_playlist = crate::playlist::MediaPlaylist {
                    uri: meta.url.as_ref().to_owned(),
                    ..Default::default()
                };
                crate::hls::push_segments(&m3u8, &mut media_playlist);
                Ok(MasterPlaylist {
                    playlist_type: PlaylistType::Hls,
                    streams: vec![media_playlist],
                    uri: meta.url.as_ref().to_owned(),
                })
            }
            Err(x) => bail!(
                "couldn't parse response as hls playlist (failed with {}).\n\n{}\n\n{}",
                x,
                meta.url,
                meta.text
            ),
        },
        _ => bail!("couldn't determine playlist type, only DASH and HLS playlists are supported."),
    }
}

pub fn parse_selected_streams(
    auto_opts: &AutomationOptions,
    base_url: Option<Url>,
    client: &Client,
    meta: &Metadata,
    query: &HashMap<String, String>,
) -> Result<Vec<MediaPlaylist>> {
    match meta.pl_type {
        Some(PlaylistType::Dash) => {
            let mpd = dash_mpd::parse(&meta.text)
                .map_err(|_| anyhow!("couldn't parse response ({}) as dash playlist.", meta.url))?;
            let mut streams = crate::dash::parse_as_master(&mpd, meta.url.as_ref())
                .sort_streams()
                .select_streams(auto_opts)?;

            for stream in &mut streams {
                crate::dash::push_segments(
                    &mpd,
                    stream,
                    base_url.as_ref().unwrap_or(&meta.url).as_str(),
                )?;
                stream.uri = meta.url.as_ref().to_owned();
            }

            Ok(streams)
        }
        Some(PlaylistType::Hls) => match m3u8_rs::parse_playlist_res(meta.text.as_bytes()) {
            Ok(m3u8_rs::Playlist::MasterPlaylist(m3u8)) => {
                let mut streams = crate::hls::parse_as_master(&m3u8, meta.url.as_str())
                    .sort_streams()
                    .select_streams(auto_opts)?;

                for stream in &mut streams {
                    stream.uri = base_url
                        .as_ref()
                        .unwrap_or(&meta.url)
                        .join(&stream.uri)?
                        .to_string();

                    let text;
                    if let Some(bs) = stream
                        .uri
                        .strip_prefix("data:application/x-mpegurl;base64,")
                    {
                        let decoded = utils::decode_base64(bs)?;
                        text = String::from_utf8(decoded)?;
                    } else {
                        let response = client.get(&stream.uri).query(query).send()?;
                        text = response.text()?;
                    }

                    let media_playlist = m3u8_rs::parse_media_playlist_res(text.as_bytes())
                        .map_err(|x| {
                            anyhow!(
                                "couldn't parse response as hls playlist (failed with {}).\n\n{}\n\n{}",
                                x,
                                stream.uri,
                                text
                            )
                        })?;
                    crate::hls::push_segments(&media_playlist, stream);
                }

                Ok(streams)
            }
            Ok(m3u8_rs::Playlist::MediaPlaylist(m3u8)) => {
                let mut media_playlist = MediaPlaylist {
                    uri: meta.url.as_ref().to_owned(),
                    ..Default::default()
                };
                crate::hls::push_segments(&m3u8, &mut media_playlist);
                Ok(vec![media_playlist])
            }
            Err(_) => bail!("couldn't parse response ({}) as hls playlist.", meta.url),
        },
        _ => bail!("couldn't determine playlist type, only DASH and HLS playlists are supported."),
    }
}
