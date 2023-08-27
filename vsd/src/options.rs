use crate::{
    commands::Quality,
    dash,
    error::{Error, Result},
    hls,
    playlist::{MasterPlaylist, MediaPlaylist, PlaylistType},
    utils,
};
use kdam::term::Colorizer;
use requestty::Question;
use reqwest::header::CONTENT_TYPE;
use std::{
    fs, io,
    io::Write,
    path::{Path, PathBuf},
};
use url::Url;

pub(crate) type Client = reqwest::Client;

#[derive(Debug, Clone)]
pub(crate) enum Input {
    File(PathBuf),
    Url(Url),
}

impl Input {
    pub(crate) async fn fetch(&self, client: &Client) -> Result<Playlist> {
        match self {
            Input::File(file) => {
                let mut playlist_type = None;

                if let Some(ext) = file.extension() {
                    if ext == "mpd" {
                        playlist_type = Some(PlaylistType::Dash);
                    } else if ext == "m3u" || ext == "m3u8" {
                        playlist_type = Some(PlaylistType::Hls);
                    }
                }

                let text = fs::read_to_string(file).map_err(|_| {
                    Error::new(format!(
                        "`{}` file couldn't be read",
                        file.to_string_lossy()
                    ))
                })?;

                if playlist_type.is_none() {
                    if text.contains("<MPD") {
                        playlist_type = Some(PlaylistType::Dash);
                    } else if text.contains("#EXTM3U") {
                        playlist_type = Some(PlaylistType::Hls);
                    }
                }

                Ok(Playlist {
                    text,
                    playlist_type,
                    url: None,
                })
            }
            Input::Url(url) => {
                let mut playlist_type = None;
                let path = url.path();

                if path.ends_with(".mpd") || path.ends_with(".xml") {
                    playlist_type = Some(PlaylistType::Dash);
                } else if path.ends_with(".m3u") || path.ends_with(".m3u8") {
                    playlist_type = Some(PlaylistType::Hls);
                }

                let response = client.get(url.as_str()).send().await.map_err(|e| {
                    Error::new(format!(
                        "Cannot send GET request to {} (reqwest-error: {})",
                        url, e
                    ))
                })?;
                let status = response.status();

                if status.is_client_error() || status.is_server_error() {
                    return Err(Error::new(format!(
                        "{} couldn't be reached (status-code: {})",
                        url, status
                    )));
                }

                if let Some(content_type) = response.headers().get(CONTENT_TYPE) {
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

                let url = Some(response.url().as_str().parse::<Url>().unwrap());
                let text = response.text().await.map_err(|_| {
                    Error::new(format!(
                        "{} response couldn't be decoded as text",
                        url.as_ref().unwrap()
                    ))
                })?;

                if playlist_type.is_none() {
                    if text.contains("<MPD") {
                        playlist_type = Some(PlaylistType::Dash);
                    } else if text.contains("#EXTM3U") {
                        playlist_type = Some(PlaylistType::Hls);
                    }
                }

                Ok(Playlist {
                    text,
                    playlist_type,
                    url,
                })
            }
        }
    }
}

pub(crate) struct Playlist {
    text: String,
    playlist_type: Option<PlaylistType>,
    url: Option<Url>,
}

impl Playlist {
    pub(crate) fn is_site(&self) -> bool {
        self.playlist_type.is_none() && self.url.is_some()
    }

    // pub(crate) fn requires_base_url(&self) -> bool {
    //     self.url.is_none()
    // }

    pub(crate) async fn parse_sort_select(
        &self,
        base_url: Option<&Url>,
        client: &Client,
        preferences: &Preferences,
        prompts: &Prompts,
        quality: &Quality,
    ) -> Result<MasterPlaylist> {
        if base_url.is_none() && self.url.is_none() {
            return Err(Error::new(
                "`base_url` is required to continue further".to_owned(),
            ));
        }

        let url = base_url.or(self.url.as_ref()).unwrap();

        match &self.playlist_type {
            Some(PlaylistType::Dash) => match dash_mpd::parse(&self.text) {
                Ok(mpd) => {
                    let mut streams = dash::parse_as_master(&mpd, url.as_str())
                        .sort_streams(&preferences)
                        .select_streams(&quality, &prompts)?;

                    for stream in streams.iter_mut() {
                        dash::push_segments(&mpd, stream, url.as_str())?;
                        stream.uri = url.as_str().to_owned();
                    }

                    Ok(MasterPlaylist {
                        playlist_type: PlaylistType::Dash,
                        streams,
                        uri: url.as_str().to_owned(),
                    })
                }
                Err(e) => Err(Error::new(format!(
                    "Couldn't parse DASH playlist (dash-mpd-error: {})",
                    e
                ))),
            },
            Some(PlaylistType::Hls) => match m3u8_rs::parse_playlist_res(self.text.as_bytes()) {
                Ok(m3u8_rs::Playlist::MasterPlaylist(m3u8)) => {
                    let mut streams = hls::parse_as_master(&m3u8, url.as_str())
                        .sort_streams(&preferences)
                        .select_streams(&quality, &prompts)?;

                    for stream in streams.iter_mut() {
                        stream.uri = url.join(&stream.uri).unwrap().to_string();

                        let response = client.get(&stream.uri).send().await.map_err(|e| {
                            Error::new(format!(
                                "Cannot send GET request to {} (reqwest-error: {})",
                                stream.uri, e
                            ))
                        })?;
                        let status = response.status();

                        if status.is_client_error() || status.is_server_error() {
                            return Err(Error::new(format!(
                                "{} couldn't be reached (status-code: {})",
                                stream.uri, status
                            )));
                        }

                        let text = response.text().await.map_err(|_| {
                            Error::new(format!(
                                "{} response couldn't be decoded as text",
                                stream.uri
                            ))
                        })?;

                        let media_playlist = m3u8_rs::parse_media_playlist_res(text.as_bytes())
                            .map_err(|_| {
                                Error::new("Couldn't parse HLS media playlist".to_owned())
                            })?;
                        hls::push_segments(&media_playlist, stream);
                    }

                    Ok(MasterPlaylist {
                        playlist_type: PlaylistType::Hls,
                        streams,
                        uri: url.as_str().to_owned(),
                    })
                }
                Ok(m3u8_rs::Playlist::MediaPlaylist(m3u8)) => {
                    let mut media_playlist = MediaPlaylist {
                        uri: url.as_str().to_owned(),
                        ..Default::default()
                    };

                    hls::push_segments(&m3u8, &mut media_playlist);

                    Ok(MasterPlaylist {
                        playlist_type: PlaylistType::Hls,
                        streams: vec![media_playlist],
                        uri: url.as_str().to_owned(),
                    })
                }
                Err(_) => Err(Error::new("Couldn't parse HLS playlist".to_owned())),
            },
            _ => Err(Error::new(
                "Couldn't determine playlist type (only DASH and HLS playlists are supported)"
                    .to_owned(),
            )),
        }
    }

    /// panics if url is none
    pub(crate) fn scrape(&self, prompts: &Prompts) -> Result<Input> {
        println!(
            "   {} website for DASH and HLS playlists",
            "Scraping".colorize("bold cyan")
        );

        let url;
        let urls = utils::scrape_playlist_links(&self.text);

        match urls.len() {
            0 => {
                return Err(Error::new(utils::scrape_playlist_msg(
                    self.url.as_ref().unwrap().as_str(),
                )))
            }
            1 => {
                println!("      {} {}", "Found".colorize("bold green"), &urls[0]);
                url = urls[0].parse::<Url>().unwrap();
            }
            _ => {
                if prompts.skip || prompts.raw {
                    println!("Select one playlist:");

                    for (i, link) in urls.iter().enumerate() {
                        println!("{:2}) [{}] {}", i + 1, if i == 0 { 'x' } else { ' ' }, link);
                    }

                    println!("------------------------------");

                    let mut index = 0;

                    if prompts.raw && !prompts.skip {
                        print!(
                            "Press enter to proceed with defaults.\n\
                            Or select playlist to download (1, 2, etc.): "
                        );
                        io::stdout().flush().unwrap();
                        let mut input = String::new();
                        io::stdin()
                            .read_line(&mut input)
                            .map_err(|_| Error::new("User input couldn't be read".to_owned()))?;

                        println!("------------------------------");

                        let input = input.trim();

                        if !input.is_empty() {
                            index = input.parse::<usize>().map_err(|_| {
                                Error::new(
                                    "User input couldn't be parsed as a valid positive integer"
                                        .to_owned(),
                                )
                            })? - 1;
                        }
                    }

                    url = urls
                        .get(index)
                        .ok_or_else(|| {
                            Error::new("User selected playlist is out of index bounds".to_owned())
                        })?
                        .parse::<Url>()
                        .unwrap();
                    println!("   {} {}", "Selected".colorize("bold green"), url);
                } else {
                    let question = Question::select("scraped-playlists")
                        .message("Select one playlist")
                        .should_loop(false)
                        .choices(urls)
                        .build();
                    let answer = requestty::prompt_one(question).map_err(|e| {
                        Error::new(format!(
                            "User input couldn't be captured (requestty-error: {})",
                            e
                        ))
                    })?;
                    url = answer.as_list_item().unwrap().text.parse::<Url>().unwrap();
                }
            }
        }

        Ok(Input::Url(url))
    }
}

pub(crate) struct Prompts {
    pub(crate) raw: bool,
    pub(crate) skip: bool,
}

pub(crate) struct Preferences {
    pub(crate) audio_lang: Option<String>,
    pub(crate) subs_lang: Option<String>,
}

impl Preferences {
    pub(crate) fn audio_lang_factor(&self, other_lang: &str) -> u8 {
        if let Some(prefer_lang) = &self.audio_lang {
            let prefer_lang = prefer_lang.to_lowercase();
            let other_lang = other_lang.to_lowercase();

            if prefer_lang == other_lang {
                return 2;
            } else if prefer_lang.get(0..2) == other_lang.get(0..2) {
                return 1;
            }
        }

        0
    }

    pub(crate) fn subs_lang_factor(&self, other_lang: &str) -> u8 {
        if let Some(prefer_lang) = &self.subs_lang {
            let prefer_lang = prefer_lang.to_lowercase();
            let other_lang = other_lang.to_lowercase();

            if prefer_lang == other_lang {
                return 2;
            } else if prefer_lang.get(0..2) == other_lang.get(0..2) {
                return 1;
            }
        }

        0
    }
}

#[derive(Debug, Clone)]
pub enum Quality {
    Lowest,
    Highest,
    Resolution(u16, u16),
    Youtube144p,
    Youtube240p,
    Youtube360p,
    Youtube480p,
    Youtube720p,
    Youtube1080p,
    Youtube2k,
    Youtube1440p,
    Youtube4k,
    Youtube8k,
}
