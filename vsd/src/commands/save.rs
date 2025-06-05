use crate::{
    cookie::{CookieJar, CookieParam},
    downloader::{self, encryption::Decrypter, Prompts},
    playlist::{KeyMethod, Quality},
};
use anyhow::Result;
use clap::Args;
use cookie::Cookie;
use kdam::term::Colorizer;
use reqwest::{
    blocking::Client,
    header::{HeaderMap, HeaderName, HeaderValue},
    Proxy, Url,
};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

type CookieParams = Vec<CookieParam>;

/// Download DASH and HLS playlists.
#[derive(Debug, Clone, Args)]
pub struct Save {
    /// http(s):// | .mpd | .xml | .m3u8
    #[arg(required = true)]
    pub input: String,

    /// Base url to be used for building absolute url to segment.
    /// This flag is usually needed for local input files.
    /// By default redirected playlist url is used.
    #[arg(long)]
    pub base_url: Option<Url>,

    /// Change directory path for temporarily downloaded files.
    /// By default current working directory is used.
    #[arg(short, long)]
    pub directory: Option<PathBuf>,

    /// Mux all downloaded streams to a video container (.mp4, .mkv, etc.) using ffmpeg.
    /// Note that existing files will be overwritten and downloaded streams will be deleted.
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Parse playlist and returns it in json format.
    /// Note that `--output` flag is ignored when this flag is used.
    #[arg(long)]
    pub parse: bool,

    /// Raw style input prompts for old and unsupported terminals.
    #[arg(long)]
    pub raw_prompts: bool,

    /// Preferred language when multiple audio streams with different languages are available.
    /// Must be in RFC 5646 format (eg. fr or en-AU).
    /// If a preference is not specified and multiple audio streams are present,
    /// the first one listed in the manifest will be downloaded.
    #[arg(long, help_heading = "Automation Options")]
    pub prefer_audio_lang: Option<String>,

    /// Preferred language when multiple subtitles streams with different languages are available.
    /// Must be in RFC 5646 format (eg. fr or en-AU).
    /// If a preference is not specified and multiple subtitles streams are present,
    /// the first one listed in the manifest will be downloaded.
    #[arg(long, help_heading = "Automation Options")]
    pub prefer_subs_lang: Option<String>,

    /// Automatic selection of some standard resolution streams with highest bandwidth stream variant from playlist.
    /// If matching resolution of WIDTHxHEIGHT is not found then only resolution HEIGHT would be considered for selection.
    /// comman values: [lowest, min, 144p, 240p, 360p, 480p, 720p, hd, 1080p, fhd, 2k, 1440p, qhd, 4k, 8k, highest, max]
    #[arg(short, long, help_heading = "Automation Options", default_value = "highest", value_name = "WIDTHxHEIGHT|HEIGHTp", value_parser = quality_parser)]
    pub quality: Quality,

    /// Skip user input prompts and proceed with defaults.
    #[arg(long, help_heading = "Automation Options")]
    pub skip_prompts: bool,

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

    /// Skip passing query parameters where not needed.
    #[arg(long, help_heading = "Client Options")]
    pub no_query_pass: bool,

    /// Set http(s) / socks proxy address for requests.
    #[arg(long, help_heading = "Client Options", value_parser = proxy_address_parser)]
    pub proxy: Option<Proxy>,

    /// Set query parameters for requests.
    #[arg(long, help_heading = "Client Options")]
    pub query: Option<String>,

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
        default_value = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/137.0.0.0 Safari/537.36"
    )]
    pub user_agent: String,

    /// Keys for decrypting encrypted streams.
    /// KID:KEY should be specified in hex format.
    /// While a single KEY should be specified as file path.
    #[arg(short, long, help_heading = "Decrypt Options", value_name = "KEY | KID:KEY;KID:KEY...", default_value_t = Decrypter::None, value_parser = keys_parser)]
    pub keys: Decrypter,

    /// Download encrypted streams without decrypting them.
    /// Note that --output flag is ignored if this flag is used.
    #[arg(long, help_heading = "Decrypt Options")]
    pub no_decrypt: bool,

    /// Maximum number of retries to download an individual segment.
    #[arg(long, help_heading = "Download Options", default_value_t = 15)]
    pub retry_count: u8,

    /// Download streams without merging them.
    /// Note that --output flag is ignored if this flag is used.
    #[arg(long, help_heading = "Download Options")]
    pub no_merge: bool,

    /// Maximum number of threads for parllel downloading of segments.
    /// Number of threads should be in range 1-16 (inclusive).
    #[arg(short, long, help_heading = "Download Options", default_value_t = 5, value_parser = clap::value_parser!(u8).range(1..=16))]
    pub threads: u8,
}

impl Save {
    fn client(&self) -> Result<Client> {
        let mut client_builder = Client::builder()
            .cookie_store(true)
            .danger_accept_invalid_certs(self.no_certificate_checks)
            .user_agent(&self.user_agent);

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

        if let Some(proxy) = &self.proxy {
            client_builder = client_builder.proxy(proxy.clone());
        }

        let mut jar = CookieJar::new();

        if !self.set_cookie.is_empty() {
            for i in (0..self.set_cookie.len()).step_by(2) {
                jar.add_cookie_str(&self.set_cookie[i], &self.set_cookie[i + 1].parse::<Url>()?);
            }
        }

        for cookie in &self.cookies {
            if let Some(url) = &cookie.url {
                jar.add_cookie_str(&format!("{}", cookie.as_cookie()), &url.parse::<Url>()?);
            } else {
                jar.add_cookie(cookie.as_cookie());
            }
        }

        let client = client_builder.cookie_provider(Arc::new(jar)).build()?;
        Ok(client)
    }

    pub fn execute(mut self) -> Result<()> {
        let client = self.client()?;
        let prompts = Prompts {
            skip: self.skip_prompts,
            raw: self.raw_prompts,
        };
        let meta =
            downloader::fetch_playlist(self.base_url.clone(), &client, &self.input, &prompts)?;

        if self.parse {
            let playlist = downloader::parse_all_streams(self.base_url.clone(), &client, &meta)?;
            serde_json::to_writer(std::io::stdout(), &playlist)?;
        } else {
            let mut streams = downloader::parse_selected_streams(
                self.base_url.clone(),
                &client,
                &meta,
                self.prefer_audio_lang,
                self.prefer_subs_lang,
                &prompts,
                self.quality,
            )?;

            if !self.no_query_pass {
                if let Some(query) = self.query.as_mut() {
                    if query.starts_with('&') {
                        *query = query.trim_start_matches('&').to_owned();
                    }
                }

                streams.iter_mut().for_each(|x| {
                    if let Some(query) = self.query.clone().or(x
                        .uri
                        .parse::<Url>()
                        .unwrap()
                        .query()
                        .map(|y| y.to_owned()))
                    {
                        x.add_query(&query);
                    }
                });
            }

            downloader::download(
                self.base_url,
                client,
                self.directory,
                self.keys,
                self.no_decrypt,
                self.no_merge,
                self.output,
                streams,
                self.retry_count,
                self.threads,
            )?;
        }

        Ok(())
    }
}

fn cookie_parser(s: &str) -> Result<CookieParams, String> {
    if Path::new(s).exists() {
        Ok(serde_json::from_slice::<CookieParams>(
            &std::fs::read(s).map_err(|_| format!("could not read {}.", s))?,
        )
        .map_err(|_| "could not deserialize cookies from json file.")?)
    } else if let Ok(cookies) = serde_json::from_str::<CookieParams>(s) {
        Ok(cookies)
    } else {
        let mut cookies = vec![];
        for cookie in Cookie::split_parse(s) {
            match cookie {
                Ok(x) => cookies.push(CookieParam::new(x.name(), x.value())),
                Err(_) => return Err("could not split parse cookies.".to_owned()),
            }
        }
        Ok(cookies)
    }
}

fn keys_parser(s: &str) -> Result<Decrypter, String> {
    if Path::new(s).exists() {
        let bytes = fs::read(s).map_err(|_| "could'nt read key file.")?;

        if bytes.len() != 16 {
            return Err("invalid key size.".to_owned());
        }

        let mut key = [0_u8; 16];
        key.copy_from_slice(&bytes);

        Ok(Decrypter::new_hls_aes(key, [0_u8; 16], &KeyMethod::None))
    } else {
        let mut kid_key_pairs = HashMap::new();

        for pair in s.split(';') {
            if let Some((kid, key)) = pair.split_once(':') {
                let kid = kid.replace('-', "").to_ascii_lowercase();
                let key = key.replace('-', "").to_ascii_lowercase();

                if kid.len() == 32
                    && key.len() == 32
                    && kid.chars().all(|c| c.is_ascii_hexdigit())
                    && key.chars().all(|c| c.is_ascii_hexdigit())
                {
                    kid_key_pairs.insert(kid, key);
                } else {
                    return Err("invalid kid key format used.".to_owned());
                }
            }
        }

        Ok(Decrypter::Mp4Decrypt(kid_key_pairs))
    }
}

fn proxy_address_parser(s: &str) -> Result<Proxy, String> {
    Proxy::all(s).map_err(|x| x.to_string())
}

fn quality_parser(s: &str) -> Result<Quality, String> {
    Ok(match s.to_lowercase().as_str() {
        "lowest" | "min" => Quality::Lowest,
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
        x if x.ends_with('p') => Quality::Resolution(
            0,
            x.trim_end_matches('p')
                .parse::<u16>()
                .map_err(|_| "could not parse resolution HEIGHT.".to_owned())?,
        ),
        x => {
            if let Some((w, h)) = x.split_once('x') {
                Quality::Resolution(
                    w.parse::<u16>()
                        .map_err(|_| "could not parse resolution WIDTH.".to_owned())?,
                    h.parse::<u16>()
                        .map_err(|_| "could not parse resolution HEIGHT.".to_owned())?,
                )
            } else {
                Err(format!(
                    "could not parse resolution WIDTHxHEIGHT. comman values: [{}]",
                    [
                        "lowest", "min", "144p", "240p", "360p", "480p", "720p", "hd", "1080p",
                        "fhd", "2k", "1440p", "qhd", "4k", "8k", "highest", "max"
                    ]
                    .iter()
                    .map(|x| x.colorize("green"))
                    .collect::<Vec<_>>()
                    .join(", "),
                ))?
            }
        }
    })
}
