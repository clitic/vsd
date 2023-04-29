use crate::cookie::{CookieJar, CookieParam};
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
    pub output: Option<String>,

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
    /// possible values: [lowest, min, 144p, 240p, 360p, 480p, 720p, hd, 1080p, fhd, 2k, 1440p, qhd, 4k, 8k, highest, max]
    #[arg(short, long, help_heading = "Automation Options", default_value = "highest", value_name = "WIDTHxHEIGHT", value_parser = quality_parser)]
    pub quality: Quality,

    /// Skip user input prompts and proceed with defaults.
    #[arg(long, help_heading = "Automation Options")]
    pub skip_prompts: bool,

    /// Fill request client with some existing cookies value.
    /// It can be document.cookie value or in json format same as puppeteer.
    #[arg(long, help_heading = "Client Options", value_parser = cookie_parser)]
    pub cookies: CookieParams,

    /// Custom headers for requests.
    /// This option can be used multiple times.
    #[arg(long, help_heading = "Client Options", num_args = 2, value_names = &["KEY", "VALUE"])]
    pub header: Vec<String>, // Vec<(String, String)> not supported

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

    /// Use all supplied keys for decryption instead of using keys which matches with default kid only.
    #[arg(long, help_heading = "Decrypt Options")]
    pub all_keys: bool,

    /// Keys for decrypting encrypted streams.
    /// If streams are encrypted with a single key then there is no need to specify key id
    /// else specify decryption key in format KID:KEY.
    /// KEY value can be specified in hex, base64 or file format.
    /// This option can be used multiple times.
    #[arg(short, long, help_heading = "Decrypt Options", value_name = "KEY|KID:KEY", value_parser = key_parser)]
    pub key: Vec<(Option<String>, String)>,

    /// Download encrypted streams without decrypting them.
    /// Note that --output flag is ignored if this flag is used.
    #[arg(long, help_heading = "Decrypt Options")]
    pub no_decrypt: bool,

    /// Maximum number of retries to download an individual segment.
    #[arg(long, help_heading = "Download Options", default_value_t = 15)]
    pub retry_count: u8,

    /// Maximum number of threads for parllel downloading of segments.
    /// Number of threads should be in range 1-16 (inclusive).
    #[arg(short, long, help_heading = "Download Options", default_value_t = 5, value_parser = clap::value_parser!(u8).range(1..=16))]
    pub threads: u8,
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
                "lowest", "144p", "240p", "360p", "480p", "720p", "hd", "1080p", "fhd", "2k",
                "1440p", "qhd", "4k", "8k", "highest", "max"
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
    let (key_id, mut key) = if let Some((key_id, key)) = s.split_once(':') {
        (Some(key_id.to_lowercase().replace('-', "")), key.to_owned())
    } else {
        (None, s.to_owned())
    };

    if let Ok(decoded_key) = openssl::base64::decode_block(&key) {
        key = hex::encode(decoded_key);
    } else {
        let key_file = Path::new(&key);

        if key_file.exists() {
            if key_file.is_file() {
                key = hex::encode(
                    std::fs::read(&key_file)
                        .map_err(|_| format!("cannot read key from {}", key))?,
                )
            } else {
                return Err("cannot read key from a non file path".to_owned());
            }
        }
    }

    Ok((key_id, key.to_lowercase()))
}

fn cookie_parser(s: &str) -> Result<CookieParams, String> {
    if Path::new(s).exists() {
        Ok(serde_json::from_slice::<CookieParams>(
            &std::fs::read(s).map_err(|_| format!("could not read {}", s))?,
        )
        .map_err(|x| {
            format!(
                "could not deserialize cookies from json file (failed with {}).",
                x
            )
        })?)
    } else {
        if let Ok(cookies) = serde_json::from_str::<CookieParams>(s) {
            Ok(cookies)
        } else {
            let mut cookies = vec![];

            for cookie in Cookie::split_parse(s) {
                match cookie {
                    Ok(x) => cookies.push(CookieParam::new(x.name(), x.value())),
                    Err(e) => {
                        return Err(format!(
                            "could not split parse cookies (failed with {}).",
                            e
                        ))
                    }
                }
            }

            Ok(cookies)
        }
    }
}

fn proxy_address_parser(s: &str) -> Result<Proxy, String> {
    Proxy::all(s).map_err(|x| x.to_string())
}

impl Save {
    pub fn perform(self) -> Result<()> {
        let mut client_builder = Client::builder()
            .user_agent(&self.user_agent)
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

        if let Some(proxy) = &self.proxy {
            client_builder = client_builder.proxy(proxy.to_owned());
        }

        let mut jar = CookieJar::new();

        if !self.set_cookie.is_empty() {
            for i in (0..self.set_cookie.len()).step_by(2) {
                jar.add_cookie_str(&self.set_cookie[i], &self.set_cookie[i + 1].parse::<Url>()?);
            }
        }

        for cookie in &self.cookies {
            println!("{:?}", cookie);
            if let Some(url) = &cookie.url {
                jar.add_cookie_str(&format!("{}", cookie.as_cookie()), &url.parse::<Url>()?);
            } else {
                jar.add_cookie(cookie.as_cookie());
            }
        }

        let client = client_builder.cookie_provider(Arc::new(jar)).build()?;

        crate::downloader::download(
            self.all_keys,
            self.base_url,
            client,
            self.directory,
            &self.input,
            self.key,
            self.no_decrypt,
            self.output,
            self.prefer_audio_lang,
            self.prefer_subs_lang,
            self.quality,
            self.skip_prompts,
            self.raw_prompts,
            self.retry_count,
            self.threads,
        )?;

        Ok(())
    }
}
