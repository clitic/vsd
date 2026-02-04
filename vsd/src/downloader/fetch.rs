use crate::{
    options::{Interaction, SelectOptions},
    playlist::{MasterPlaylist, MediaPlaylist, PlaylistType},
    utils,
};
use anyhow::{Result, anyhow, bail};
use base64::Engine;
use colored::Colorize;
use log::info;
use reqwest::{Client, Url, header};
use std::path::Path;
use tokio::fs;

pub struct FetchedPlaylist {
    url: Url,
    data: Vec<u8>,
    playlist_type: Option<PlaylistType>,
}

impl FetchedPlaylist {
    pub async fn new(
        input: &str,
        client: &Client,
        base_url: Option<&Url>,
        query: &Vec<(String, String)>,
    ) -> Result<Self> {
        let path = Path::new(input);
        let mut typ = None;

        if path.exists() {
            if base_url.is_none() {
                bail!("Base URL is required for local playlist file.");
            }

            match path.extension() {
                Some(ext) if ext == "m3u" || ext == "m3u8" => typ = Some(PlaylistType::Hls),
                Some(ext) if ext == "mpd" => typ = Some(PlaylistType::Dash),
                _ => (),
            }

            Ok(Self {
                url: base_url.unwrap().clone(),
                data: fs::read(path).await?,
                playlist_type: typ,
            })
        } else if let Ok(input) = input.parse::<Url>() {
            let response = client.get(input).query(query).send().await?;
            let status = response.status();

            if status.is_client_error() || status.is_server_error() {
                bail!(
                    "Playlist request failed ({}): '{}'",
                    status,
                    response.text().await?
                );
            }

            if let Some(content_type) = response.headers().get(header::CONTENT_TYPE) {
                match content_type.as_bytes() {
                    b"application/dash+xml" | b"video/vnd.mpeg.dash.mpd" => {
                        typ = Some(PlaylistType::Dash)
                    }
                    b"application/x-mpegurl" | b"application/vnd.apple.mpegurl" => {
                        typ = Some(PlaylistType::Hls)
                    }
                    _ => (),
                }
            }

            Ok(Self {
                url: response.url().to_owned(),
                data: response.bytes().await?.to_vec(),
                playlist_type: typ,
            })
        } else {
            bail!("Unable to determine the input playlist type.");
        }
    }

    fn playlist_type(&self) -> Result<PlaylistType> {
        if let Some(typ) = &self.playlist_type {
            return Ok(typ.to_owned());
        }
        if self.data.windows(7).any(|w| w == b"#EXTM3U") {
            return Ok(PlaylistType::Hls);
        }
        if self.data.windows(4).any(|w| w == b"<MPD") {
            return Ok(PlaylistType::Dash);
        }
        bail!("Unable to determine the input playlist type.");
    }

    pub fn list_streams(&self) -> Result<()> {
        match self.playlist_type()? {
            PlaylistType::Dash => {
                let xml = String::from_utf8_lossy(&self.data);
                let mpd = dash_mpd::parse(&xml)
                    .map_err(|e| anyhow!("Failed to parse DASH playlist: {e}"))?;
                crate::dash::parse_as_master(&mpd, self.url.as_ref())
                    .sort_streams()
                    .list_streams();
            }
            PlaylistType::Hls => match m3u8_rs::parse_playlist_res(&self.data)
                .map_err(|e| anyhow!("Failed to parse HLS playlist: {e}"))?
            {
                m3u8_rs::Playlist::MasterPlaylist(m3u8) => {
                    crate::hls::parse_as_master(&m3u8, self.url.as_ref())
                        .sort_streams()
                        .list_streams()
                }
                m3u8_rs::Playlist::MediaPlaylist(_) => {
                    info!("------ {} ------", "Undefined Streams".cyan());
                    info!(" 1) {}", self.url);
                }
            },
        }
        Ok(())
    }

    pub async fn as_master_playlist(
        &self,
        client: &Client,
        query: &Vec<(String, String)>,
        mut select_opts: SelectOptions,
        interaction: Interaction,
        parse_everything: bool,
    ) -> Result<MasterPlaylist> {
        match self.playlist_type()? {
            PlaylistType::Dash => {
                let xml = String::from_utf8_lossy(&self.data);
                let mpd = dash_mpd::parse(&xml)
                    .map_err(|e| anyhow!("Failed to parse DASH playlist: {e}"))?;

                let mut playlist = if parse_everything {
                    crate::dash::parse_as_master(&mpd, self.url.as_str())
                } else {
                    crate::dash::parse_as_master(&mpd, self.url.as_str())
                        .sort_streams()
                        .select_streams(&mut select_opts, interaction)?
                };

                for stream in &mut playlist.streams {
                    crate::dash::push_segments(&mpd, stream, client, self.url.as_str(), query)
                        .await?;
                }

                Ok(playlist)
            }
            PlaylistType::Hls => match m3u8_rs::parse_playlist_res(&self.data)
                .map_err(|e| anyhow!("Failed to parse HLS playlist: {e}"))?
            {
                m3u8_rs::Playlist::MasterPlaylist(playlist) => {
                    let mut playlist = if parse_everything {
                        crate::hls::parse_as_master(&playlist, self.url.as_str())
                    } else {
                        crate::hls::parse_as_master(&playlist, self.url.as_str())
                            .sort_streams()
                            .select_streams(&mut select_opts, interaction)?
                    };

                    for stream in &mut playlist.streams {
                        let data;
                        if let Some(bs) = stream
                            .uri
                            .strip_prefix("data:application/x-mpegurl;base64,")
                        {
                            data = base64::engine::general_purpose::STANDARD.decode(bs)?;
                        } else {
                            stream.uri = self.url.join(&stream.uri)?.to_string();
                            let response = client.get(&stream.uri).query(query).send().await?;
                            let status = response.status();

                            if status.is_client_error() || status.is_server_error() {
                                bail!(
                                    "Playlist request failed ({}): '{}'",
                                    status,
                                    response.text().await?
                                );
                            }

                            data = response.bytes().await?.to_vec();
                        }

                        let media_playlist = m3u8_rs::parse_media_playlist_res(&data)
                            .map_err(|e| anyhow!("Failed to parse HLS playlist: {e}"))?;
                        crate::hls::push_segments(&media_playlist, stream);
                    }

                    Ok(playlist)
                }
                m3u8_rs::Playlist::MediaPlaylist(playlist) => {
                    let mut media_playlist = MediaPlaylist {
                        id: utils::gen_id(self.url.as_str(), ""),
                        uri: self.url.as_str().to_owned(),
                        ..Default::default()
                    };
                    crate::hls::push_segments(&playlist, &mut media_playlist);
                    Ok(MasterPlaylist {
                        playlist_type: PlaylistType::Hls,
                        streams: vec![media_playlist],
                        uri: self.url.as_str().to_owned(),
                    })
                }
            },
        }
    }
}
