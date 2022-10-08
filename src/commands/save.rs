use crate::download::DownloadState;
use crate::progress::DownloadProgress;
use anyhow::{bail, Result};
use clap::Args;
use kdam::term::Colorizer;
use reqwest::blocking::Client;
use reqwest::cookie::Jar;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::{Proxy, Url};
use std::path::PathBuf;
use std::sync::Arc;

/// Download and save HLS and DASH playlists to disk.
#[derive(Debug, Clone, Args)]
pub struct Save {
    /// http(s):// | .m3u8 | .m3u | .mpd | .xml
    #[arg(required = true)]
    pub input: String,

    /// Download alternative audio or subtitles stream from playlist instead all streams.
    /// For downloading video stream only, use `--skip` flag.
    #[arg(short, long)]
    pub alternative: bool,

    /// Base url for all segments.
    /// Usually needed for local m3u8 file.
    #[arg(long)]
    pub baseurl: Option<String>,

    /// Change directory path for temporarily downloaded files.
    /// By default current working directory is used.
    #[arg(short, long)]
    pub directory: Option<String>,

    /// Decryption keys for decrypting CENC encrypted streams.
    /// Key value should be specified in hex.
    /// Use `base64:` prefix if key is in base64 format.
    /// Streams encrypted with a single key can use `--key base64:MhbcGzyxPfkOsp3FS8qPyA==` like key format.
    /// Streams encrypted with multiple keys can use `--key eb676abbcb345e96bbcf616630f1a3da:100b6c20940f779a4589152b57d2dacb like key format.
    /// This option can be used multiple times.
    #[arg(short, long, value_name = "<KID:(base64:)KEY>|(base64:)KEY", value_parser = key_parser)]
    pub key: Vec<(Option<String>, String)>,

    /// Mux all downloaded streams to a video container (.mp4, .mkv, etc.) using ffmpeg.
    /// Note that existing files will be overwritten and downloaded streams will be deleted.
    #[arg(short, long, value_parser = output_parser)]
    pub output: Option<String>,

    /// Preferred language when multiple audio streams with different languages are available.
    /// Must be in RFC 5646 format (eg. fr or en-AU).
    /// If a preference is not specified and multiple audio streams are present,
    /// the first one listed in the manifest will be downloaded.
    #[arg(long)]
    pub prefer_audio_lang: Option<String>,

    /// Preferred language when multiple subtitles streams with different languages are available.
    /// Must be in RFC 5646 format (eg. fr or en-AU).
    /// If a preference is not specified and multiple subtitles streams are present,
    /// the first one listed in the manifest will be downloaded.
    #[arg(long)]
    pub prefer_subs_lang: Option<String>,

    /// Automatic selection of some standard resolution streams with highest bandwidth stream variant from playlist.
    /// possible values: [144p, 240p, 360p, 480p, 720p, hd, 1080p, fhd, 2k, 1440p, qhd, 4k, 8k, highest, max, select-later]
    #[arg(short, long, default_value = "select-later", value_name = "WIDTHxHEIGHT", value_parser = quality_parser)]
    pub quality: Quality,

    /// Raw style input prompts for old and unsupported terminals.
    #[arg(long)]
    pub raw_prompts: bool,

    // /// Record duration for live playlist in seconds.
    // #[arg(long)]
    // pub record_duration: Option<f32>,

    /// Maximum number of retries to download an individual segment.
    #[arg(long, default_value_t = 15)]
    pub retry_count: u8,

    /// Skip downloading and muxing alternative streams.
    #[arg(short, long)]
    pub skip: bool,

    /// Maximum number of threads for parllel downloading of segments.
    /// Number of threads should be in range 1-16 (inclusive).
    #[arg(short, long, default_value_t = 5, value_parser = clap::value_parser!(u8).range(1..=16))]
    pub threads: u8,

    /// Enable cookie store and fill it with some existing cookies.
    /// Example `--cookies "foo=bar; Domain=yolo.local" https://yolo.local`.
    /// This option can be used multiple times.
    #[arg(long, number_of_values = 2, value_names = &["COOKIES", "URL"], help_heading = "Client Options")]
    pub cookies: Vec<String>, // Vec<Vec<String>> not supported

    /// Enable cookie store which allows cookies to be stored.
    #[arg(long, help_heading = "Client Options")]
    pub enable_cookies: bool,

    /// Custom headers for requests.
    /// This option can be used multiple times.
    #[arg(long, number_of_values = 2, value_names = &["KEY", "VALUE"], help_heading = "Client Options")]
    pub header: Vec<String>, // Vec<Vec<String>> not supported

    /// Set http or https proxy address for requests.
    #[arg(long, value_parser = proxy_address_parser, help_heading = "Client Options")]
    pub proxy_address: Option<String>,

    /// Update and set custom user agent for requests.
    #[arg(
        long,
        default_value = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/101.0.4951.64 Safari/537.36",
        help_heading = "Client Options"
    )]
    pub user_agent: String,
}

#[derive(Debug, Clone)]
pub enum Quality {
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
    Resolution(u16, u16),
    Highest,
    SelectLater,
}

fn quality_parser(s: &str) -> Result<Quality, String> {
    Ok(match s.to_lowercase().as_str() {
        "144p" => Quality::Youtube144p,
        "240p" => Quality::Youtube240p,
        "360p" => Quality::Youtube360p,
        "480p" => Quality::Youtube480p,
        "720p" | "hd" => Quality::Youtube720p,
        "1080p" | "fhd" => Quality::Youtube1080p,
        "2k" => Quality::Youtube2k,
        "1440p" | "qhd" => Quality::Youtube1440p,
        "4k" => Quality::Youtube4k,
        "8k" => Quality::Youtube8k,
        "highest" | "max" => Quality::Highest,
        "select-later" => Quality::SelectLater,
        x if x.contains('x') => {
            if let (Some(w), Some(h)) = (x.split('x').next(), x.split('x').nth(1)) {
                Quality::Resolution(
                    w.parse::<u16>().map_err(|_| "invalid width".to_owned())?,
                    h.parse::<u16>().map_err(|_| "invalid height".to_owned())?,
                )
            } else {
                Err("incorrect resolution format".to_owned())?
            }
        }
        _ => Err(format!(
            "\npossible values: [{}]\nFor custom resolution use {}",
            [
                "144p",
                "240p",
                "360p",
                "480p",
                "720p",
                "hd",
                "1080p",
                "fhd",
                "2k",
                "1440p",
                "qhd",
                "4k",
                "8k",
                "highest",
                "max",
                "select-later",
            ]
            .iter()
            .map(|x| x.colorize("green"))
            .collect::<Vec<_>>()
            .join(", "),
            "WIDTHxHEIGHT".colorize("green")
        ))?,
    })
}

fn key_parser(s: &str) -> Result<(Option<String>, String), String> {
    let key = if s.contains(':') && !s.starts_with("base64") {
        let kid = s.split(':').next().unwrap();

        (
            Some(kid.replace('-', "")),
            s.trim_start_matches(kid)
                .trim_start_matches(':')
                .to_string(),
        )
    } else {
        (None, s.to_owned())
    };

    if key.1.starts_with("base64:") {
        Ok((
            key.0,
            openssl::bn::BigNum::from_slice(
                &openssl::base64::decode_block(key.1.trim_start_matches("base64:"))
                    .map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?
            .to_hex_str()
            .map_err(|e| e.to_string())?
            .to_ascii_lowercase(),
        ))
    } else {
        Ok(key)
    }
}

fn find_ffmpeg() -> Option<String> {
    Some(
        std::env::var("PATH")
            .ok()?
            .split(if cfg!(target_os = "windows") {
                ';'
            } else {
                ':'
            })
            .find(|s| {
                std::path::Path::new(s)
                    .join(if cfg!(target_os = "windows") {
                        "ffmpeg.exe"
                    } else {
                        "ffmpeg"
                    })
                    .exists()
            })?
            .to_owned(),
    )
}

fn output_parser(s: &str) -> Result<String, String> {
    if find_ffmpeg().is_some() {
        Ok(s.to_owned())
    } else {
        Err("Could'nt locate ffmpeg in PATH.\n\
        Install ffmpeg from https://www.ffmpeg.org/download.html"
            .to_owned())
    }
}

fn proxy_address_parser(s: &str) -> Result<String, String> {
    if s.starts_with("http://") || s.starts_with("https://") {
        Ok(s.to_owned())
    } else {
        Err("Proxy address should start with `http(s)://` only".to_string())
    }
}

impl Save {
    fn client(&self) -> Result<Arc<Client>> {
        let mut client_builder = Client::builder().user_agent(&self.user_agent);

        if !self.header.is_empty() {
            let mut headers = HeaderMap::new();

            for i in (0..headers.len()).step_by(2) {
                headers.insert(
                    self.header[i].parse::<HeaderName>()?,
                    self.header[i + 1].parse::<HeaderValue>()?,
                );
            }

            client_builder = client_builder.default_headers(headers);
        }

        if let Some(proxy) = &self.proxy_address {
            if proxy.starts_with("https") {
                client_builder = client_builder.proxy(Proxy::https(proxy)?);
            } else if proxy.starts_with("http") {
                client_builder = client_builder.proxy(Proxy::http(proxy)?);
            }
        }

        if self.enable_cookies || !self.cookies.is_empty() {
            client_builder = client_builder.cookie_store(true);
        }

        if !self.cookies.is_empty() {
            let jar = Jar::default();

            for i in (0..self.cookies.len()).step_by(2) {
                jar.add_cookie_str(&self.cookies[i], &self.cookies[i + 1].parse::<Url>()?);
            }

            client_builder = client_builder.cookie_provider(Arc::new(jar));
        }

        Ok(Arc::new(client_builder.build()?))
    }

    pub fn get_url(&self, uri: &str) -> Result<String> {
        if uri.starts_with("http") {
            Ok(uri.to_owned())
        } else if let Some(baseurl) = &self.baseurl {
            Ok(Url::parse(baseurl)?.join(uri)?.to_string())
        } else {
            if !self.input.starts_with("http") {
                bail!(
                    "Non HTTP input should have {} set explicitly.",
                    "--baseurl".colorize("bold green")
                )
            }

            Ok(Url::parse(&self.input)?.join(uri)?.to_string())
        }
    }

    pub fn tempfile(&self) -> String {
        let output = self
            .input
            .split('?')
            .next()
            .unwrap()
            .split('/')
            .last()
            .unwrap();

        let output = if output.ends_with(".m3u") || output.ends_with(".m3u8") {
            if output.ends_with(".ts.m3u8") {
                output.trim_end_matches(".m3u8").to_owned()
            } else {
                let mut path = PathBuf::from(&output);
                path.set_extension("ts");
                path.to_str().unwrap().to_owned()
            }
        } else if output.ends_with(".mpd") || output.ends_with(".xml") {
            let mut path = PathBuf::from(&output);
            path.set_extension("m4s");
            path.to_str().unwrap().to_owned()
        } else {
            let mut path = PathBuf::from(
                output
                    .replace('<', "-")
                    .replace('>', "-")
                    .replace(':', "-")
                    .replace('\"', "-")
                    .replace('/', "-")
                    .replace('\\', "-")
                    .replace('|', "-")
                    .replace('?', "-"),
            );
            path.set_extension("mp4");
            path.to_str().unwrap().to_owned()
        };

        output
    }

    pub fn input_type(&self) -> InputType {
        let url = self.input.split('?').next().unwrap();

        if url.starts_with("http") {
            if url.ends_with(".m3u") || url.ends_with(".m3u8") || url.ends_with("ts.m3u8") {
                InputType::HlsUrl
            } else if url.ends_with(".mpd") || url.ends_with(".xml") {
                InputType::DashUrl
            } else {
                InputType::Website
            }
        } else if url.ends_with(".m3u") || url.ends_with(".m3u8") || url.ends_with("ts.m3u8") {
            InputType::HlsLocalFile
        } else if url.ends_with(".mpd") || url.ends_with(".xml") {
            InputType::DashLocalFile
        } else {
            InputType::LocalFile
        }
    }

    #[allow(clippy::wrong_self_convention)]
    pub fn to_download_state(mut self) -> Result<DownloadState> {
        let client = self.client()?;

        if self.input_type().is_website() {
            println!(
                "{} website for HLS and DASH stream links.",
                "Scraping".colorize("bold green"),
            );
            let links = crate::utils::find_hls_dash_links(&client.get(&self.input).send()?.text()?);

            match links.len() {
                0 => bail!(crate::utils::scrape_website_message(&self.input)),
                1 => {
                    self.input = links[0].clone();
                    println!("{} {}", "Found".colorize("bold green"), &links[0]);
                }
                _ => {
                    let mut elinks = vec![];
                    for (i, link) in links.iter().enumerate() {
                        elinks.push(format!("{:2}) {}", i + 1, link));
                    }
                    let index = crate::utils::select(
                        "Select one link:".to_string(),
                        &elinks,
                        self.raw_prompts,
                    )?;
                    self.input = links[index].clone();
                }
            }
        }

        Ok(DownloadState {
            alternative_media_type: None,
            args: self,
            cenc_encrypted_audio: false,
            cenc_encrypted_video: false,
            client,
            dash: false,
            progress: DownloadProgress::new_empty(),
        })
    }
}

pub enum InputType {
    HlsUrl,
    DashUrl,
    Website,
    HlsLocalFile,
    DashLocalFile,
    LocalFile,
}

impl InputType {
    pub fn is_website(&self) -> bool {
        matches!(self, Self::Website)
    }

    pub fn is_hls(&self) -> bool {
        matches!(self, Self::HlsUrl | Self::HlsLocalFile)
    }

    pub fn is_dash(&self) -> bool {
        matches!(self, Self::DashUrl | Self::DashLocalFile)
    }
}
