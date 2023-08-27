use super::save::{cookie_parser, proxy_address_parser};
use crate::{
    cookie::{CookieJar, CookieParam},
    playlist::PlaylistType,
};
use anyhow::{anyhow, bail, Result};
use clap::Args;
use reqwest::{
    blocking::Client,
    header,
    header::{HeaderMap, HeaderName, HeaderValue},
    Proxy, Url,
};
use std::{path::Path, sync::Arc};

type CookieParams = Vec<CookieParam>;

/// Parse DASH and HLS playlists.
#[derive(Debug, Clone, Args)]
pub struct Parse {
    /// http(s):// | .mpd | .xml | .m3u8
    #[arg(required = true)]
    pub input: String,

    /// Base url to be used for building absolute url to segment.
    /// This flag is usually needed for local input files.
    /// By default redirected playlist url is used.
    #[arg(long)]
    pub base_url: Option<Url>,

    /// Fill request client with some existing cookies value.
    /// Cookies value can be same as document.cookie or in json format same as puppeteer.
    #[arg(long, help_heading = "Client Options", default_value = "[]", hide_default_value = true, value_parser = cookie_parser)]
    pub cookies: CookieParams,

    /// Custom headers for requests.
    /// This option can be used multiple times.
    #[arg(long, help_heading = "Client Options", num_args = 2, value_names = &["KEY", "VALUE"])]
    pub header: Vec<String>, // Vec<(String, String)> not supported

    /// Skip checking and validation of site certificates.
    #[arg(long, help_heading = "Client Options")]
    pub no_certificate_checks: bool,

    /// Set http(s) / socks proxy address for requests.
    #[arg(long, help_heading = "Client Options", value_parser = proxy_address_parser)]
    pub proxy: Option<Proxy>,

    /// Fill request client with some existing cookies per domain.
    /// First value for this option is set-cookie header and second value is url which was requested to send this set-cookie header.
    /// Example `--set-cookie "foo=bar; Domain=yolo.local" https://yolo.local`.
    /// This option can be used multiple times.
    #[arg(long, help_heading = "Client Options", num_args = 2, value_names = &["SET_COOKIE", "URL"])]
    pub set_cookie: Vec<String>, // Vec<(String, String)> not supported

    /// Update and set user agent header for requests.
    #[arg(
        long,
        help_heading = "Client Options",
        default_value = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/112.0.0.0 Safari/537.36"
    )]
    pub user_agent: String,
}

impl Parse {
    pub fn execute(self) -> Result<()> {
        let mut client_builder = Client::builder()
            .danger_accept_invalid_certs(self.no_certificate_checks)
            .user_agent(self.user_agent)
            .cookie_store(true);

        if !self.header.is_empty() {
            let mut headers = HeaderMap::new();

            for i in (0..self.header.len()).step_by(2) {
                headers.insert(
                    self.header[i].parse::<HeaderName>()?,
                    self.header[i + 1].parse::<HeaderValue>()?,
                );
            }

            client_builder = client_builder.default_headers(headers);
        }

        if let Some(proxy) = self.proxy {
            client_builder = client_builder.proxy(proxy);
        }

        let mut jar = CookieJar::new();

        if !self.set_cookie.is_empty() {
            for i in (0..self.set_cookie.len()).step_by(2) {
                jar.add_cookie_str(&self.set_cookie[i], &self.set_cookie[i + 1].parse::<Url>()?);
            }
        }

        for cookie in self.cookies {
            if let Some(url) = &cookie.url {
                jar.add_cookie_str(&format!("{}", cookie.as_cookie()), &url.parse::<Url>()?);
            } else {
                jar.add_cookie(cookie.as_cookie());
            }
        }

        let client = client_builder.cookie_provider(Arc::new(jar)).build()?;
        let mut playlist_url = self
            .base_url
            .clone()
            .unwrap_or_else(|| "https://example.com".parse::<Url>().unwrap());

        let mut playlist_type = None;
        let path = Path::new(&self.input);

        let playlist = if path.exists() {
            let text = std::fs::read_to_string(path)?;

            if playlist_type.is_none() {
                if text.contains("<MPD") {
                    playlist_type = Some(PlaylistType::Dash);
                } else if text.contains("#EXTM3U") {
                    playlist_type = Some(PlaylistType::Hls);
                }
            }

            text
        } else {
            let response = client.get(self.input).send()?;
            playlist_url = response.url().to_owned();

            if let Some(content_type) = response.headers().get(header::CONTENT_TYPE) {
                match content_type.as_bytes() {
                    b"application/dash+xml" | b"video/vnd.mpeg.dash.mpd" => {
                        playlist_type = Some(PlaylistType::Dash)
                    }
                    b"application/x-mpegurl" | b"application/vnd.apple.mpegurl" => {
                        playlist_type = Some(PlaylistType::Hls)
                    }
                    _ => (),
                }
            }

            let text = response.text()?;

            if playlist_type.is_none() {
                if text.contains("<MPD") {
                    playlist_type = Some(PlaylistType::Dash);
                } else if text.contains("#EXTM3U") {
                    playlist_type = Some(PlaylistType::Hls);
                }
            }

            text
        };

        let playlist = match playlist_type {
            Some(PlaylistType::Dash) => {
                let mpd = dash_mpd::parse(&playlist).map_err(|x| {
                    anyhow!(
                        "couldn't parse response as dash playlist (failed with {}).\n\n{}",
                        x,
                        playlist
                    )
                })?;
                let mut playlist = crate::dash::parse_as_master(&mpd, playlist_url.as_str());

                for stream in playlist.streams.iter_mut() {
                    crate::dash::push_segments(
                        &mpd,
                        stream,
                        self.base_url.as_ref().unwrap_or(&playlist_url).as_str(),
                    )?;
                    stream.uri = playlist_url.as_str().to_owned();
                }

                playlist
            }
            Some(PlaylistType::Hls) => match m3u8_rs::parse_playlist_res(playlist.as_bytes()) {
                Ok(m3u8_rs::Playlist::MasterPlaylist(m3u8)) => {
                    let mut playlist = crate::hls::parse_as_master(&m3u8, playlist_url.as_str());

                    for stream in playlist.streams.iter_mut() {
                        stream.uri = self
                            .base_url
                            .as_ref()
                            .unwrap_or(&playlist_url)
                            .join(&stream.uri)?
                            .to_string();
                        let response = client.get(&stream.uri).send()?;
                        let text = response.text()?;
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

                    playlist
                }
                Ok(m3u8_rs::Playlist::MediaPlaylist(m3u8)) => {
                    let mut media_playlist = crate::playlist::MediaPlaylist {
                        uri: playlist_url.to_string(),
                        ..Default::default()
                    };
                    crate::hls::push_segments(&m3u8, &mut media_playlist);
                    crate::playlist::MasterPlaylist {
                        playlist_type: PlaylistType::Hls,
                        streams: vec![media_playlist],
                        uri: playlist_url.to_string(),
                    }
                }
                Err(x) => bail!(
                    "couldn't parse response as hls playlist (failed with {}).\n\n{}\n\n{}",
                    x,
                    playlist_url,
                    playlist
                ),
            },
            _ => bail!(
                "couldn't determine playlist type, only DASH and HLS playlists are supported."
            ),
        };

        serde_json::to_writer(std::io::stdout(), &playlist)?;
        Ok(())
    }
}
