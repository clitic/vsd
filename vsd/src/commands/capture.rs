#![cfg(feature = "browser")]

use crate::utils;
use anyhow::Result;
use clap::{Args, ValueEnum};
use cookie::Cookie;
use headless_chrome::{
    protocol::cdp::Network::{
        events::ResponseReceivedEventParams, CookieParam, GetResponseBodyReturnObject, ResourceType,
    },
    Browser, LaunchOptionsBuilder,
};
use kdam::term::Colorizer;
use std::{
    fs,
    fs::File,
    io::Write,
    path::{Path, PathBuf},
    sync::mpsc,
};

type CookieParams = Vec<CookieParam>;

/// Capture playlists and subtitles from a website.
#[derive(Debug, Clone, Args)]
#[clap(long_about = "Capture playlists and subtitles from a website.\n\n\
Requires any one of these browser to be installed:\n\
1. chrome - https://www.google.com/chrome\n\
2. chromium - https://www.chromium.org/getting-involved/download-chromium\n\n\
Launch browser and capture files based on extension matching. \
This command work same as doing Inspect > Network > Fetch/XHR (default) > filter extension and viewing url in a browser. \
The implementation for this command is based on response handling. \
Note that this command might not work always as expected on every website.")]
pub struct Capture {
    /// http(s)://
    #[arg(required = true)]
    url: String,

    /// Fill browser with some existing cookies value.
    /// Cookies value can be same as document.cookie or in json format same as puppeteer.
    #[arg(long, default_value = "[]", hide_default_value = true, value_parser = cookie_parser)]
    cookies: CookieParams,

    /// Change directory path for saved files.
    /// By default current working directory is used.
    #[arg(short, long)]
    directory: Option<PathBuf>,

    /// List of file extensions to be filter out.
    /// This option can be used multiple times.
    #[arg(
        short, long,
        default_values_t = [
            "m3u".to_owned(),
            "m3u8".to_owned(),
            "mpd".to_owned(),
            "vtt".to_owned(),
            "srt".to_owned(),
        ]
    )]
    extensions: Vec<String>,

    /// Launch browser without a window.
    #[arg(long)]
    headless: bool,

    /// List of resource types to be filter out.
    /// This option can be used multiple times.
    #[arg(short, long, value_enum, default_values_t = [ResourceTypeCopy::Xhr, ResourceTypeCopy::Fetch])]
    resource_types: Vec<ResourceTypeCopy>,

    /// Save captured requests responses locally.
    #[arg(short, long)]
    save: bool,
}

#[derive(Debug, Clone, ValueEnum)]
enum ResourceTypeCopy {
    All,
    Document,
    Stylesheet,
    Image,
    Media,
    Font,
    Script,
    TextTrack,
    Xhr,
    Fetch,
    EventSource,
    WebSocket,
    Manifest,
    SignedExchange,
    Ping,
    CspViolationReport,
    Preflight,
    Other,
}

fn cookie_parser(s: &str) -> Result<CookieParams, String> {
    if Path::new(s).exists() {
        Ok(serde_json::from_slice::<CookieParams>(
            &fs::read(s).map_err(|_| format!("could not read {}.", s))?,
        )
        .map_err(|x| format!("could not deserialize cookies from json file. {}", x))?)
    } else if let Ok(cookies) = serde_json::from_str::<CookieParams>(s) {
        Ok(cookies)
    } else {
        let mut cookies = vec![];

        for cookie in Cookie::split_parse(s) {
            match cookie {
                Ok(x) => cookies.push(CookieParam {
                    name: x.name().to_owned(),
                    value: x.value().to_owned(),
                    url: None,
                    domain: None,
                    path: None,
                    secure: None,
                    http_only: None,
                    same_site: None,
                    expires: None,
                    priority: None,
                    same_party: None,
                    source_scheme: None,
                    source_port: None,
                    partition_key: None,
                }),
                Err(e) => return Err(format!("could not split parse cookies. {}", e)),
            }
        }

        Ok(cookies)
    }
}

impl Capture {
    pub fn execute(self) -> Result<()> {
        let (tx, rx) = mpsc::channel();
        ctrlc::set_handler(move || {
            tx.send(())
                .expect("could not send shutdown signal on channel.")
        })?;

        println!(
            "    {} sometimes video starts playing but links are not detected",
            "INFO".colorize("bold cyan")
        );

        println!(
            " {} launching in {} mode",
            "Browser".colorize("bold cyan"),
            if self.headless {
                "headless (no window)"
            } else {
                "headful (window)"
            }
        );

        let browser = Browser::new(
            LaunchOptionsBuilder::default()
                .headless(self.headless)
                .build()?,
        )?;
        let tab = browser.new_tab()?;

        println!(" {} setting cookies", "Browser".colorize("bold cyan"));
        tab.set_cookies(self.cookies)?;

        let filters = Filters {
            extensions: self.extensions,
            resource_types: self.resource_types,
        };
        let directory = if self.save {
            self.directory.clone()
        } else {
            None
        };
        let save = self.save;

        println!(
            " {} registering response listener",
            "Browser".colorize("bold cyan")
        );
        tab.register_response_handling(
            "vsd_capture",
            Box::new(move |params, get_response_body| {
                handler(params, get_response_body, &filters, &directory, save);
            }),
        )?;

        if let Some(directory) = &self.directory {
            if !directory.exists() {
                fs::create_dir_all(directory)?;
            }
        };

        println!(
            " {} navigating to {}",
            "Browser".colorize("bold cyan"),
            self.url
        );
        tab.navigate_to(&self.url)?;

        println!(
            "    {} waiting for CTRL+C signal",
            "INFO".colorize("bold cyan")
        );
        rx.recv()?;
        println!(
            " {} deregistering response listener and closing browser",
            "Browser".colorize("bold cyan")
        );
        let _ = tab.deregister_response_handling("vsd_capture")?;

        if let Some(directory) = &self.directory {
            if fs::read_dir(directory)?.next().is_none() {
                println!(
                    "{} {}",
                    "Deleting".colorize("bold red"),
                    directory.to_string_lossy()
                );
                fs::remove_dir(directory)?;
            }
        }

        Ok(())
    }
}

fn handler(
    params: ResponseReceivedEventParams,
    get_response_body: &dyn Fn() -> Result<GetResponseBodyReturnObject>,
    filters: &Filters,
    directory: &Option<PathBuf>,
    save: bool,
) {
    if !filters.pass(&params.response.url, &params.Type) {
        return;
    }

    if !save {
        println!(
            "{} {}",
            "Detected".colorize("bold green"),
            params.response.url,
        );
        return;
    }

    let path = file_path(&params.response.url, directory);
    println!(
        "  {} {} response to {}",
        "Saving".colorize("bold green"),
        params.response.url,
        path.to_string_lossy()
    );

    if let Ok(body) = get_response_body() {
        if let Ok(mut file) = File::create(&path) {
            if body.base_64_encoded {
                let decoded_body = utils::decode_base64(&body.body);
                if file
                    .write_all(
                        decoded_body
                            .as_ref()
                            .map(|x| x.as_slice())
                            .unwrap_or(body.body.as_bytes()),
                    )
                    .is_err()
                {
                    println!(
                        "  {} could'nt write response all bytes to {}",
                        "Saving".colorize("bold red"),
                        path.to_string_lossy(),
                    );
                }
            } else if file.write_all(body.body.as_bytes()).is_err() {
                println!(
                    "  {} could'nt write response all bytes to {}",
                    "Saving".colorize("bold red"),
                    path.to_string_lossy(),
                );
            }
        } else {
            println!(
                "  {} could'nt create {} file",
                "Saving".colorize("bold red"),
                path.to_string_lossy(),
            );
        }
    } else {
        println!(
            "  {} could'nt read response body for {}",
            "Saving".colorize("bold red"),
            params.response.url,
        );
    }
}

struct Filters {
    extensions: Vec<String>,
    resource_types: Vec<ResourceTypeCopy>,
}

impl Filters {
    fn pass(&self, url: &str, resource_type: &ResourceType) -> bool {
        let splitted_url = url.split('?').next().unwrap();
        let extension_matched = self
            .extensions
            .iter()
            .any(|x| splitted_url.ends_with(&(".".to_owned() + x)));
        let resource_type_matched = self.resource_types.iter().any(|x| {
            let _type = match x {
                ResourceTypeCopy::All => return true,
                ResourceTypeCopy::Document => ResourceType::Document,
                ResourceTypeCopy::Stylesheet => ResourceType::Stylesheet,
                ResourceTypeCopy::Image => ResourceType::Image,
                ResourceTypeCopy::Media => ResourceType::Media,
                ResourceTypeCopy::Font => ResourceType::Font,
                ResourceTypeCopy::Script => ResourceType::Script,
                ResourceTypeCopy::TextTrack => ResourceType::TextTrack,
                ResourceTypeCopy::Xhr => ResourceType::Xhr,
                ResourceTypeCopy::Fetch => ResourceType::Fetch,
                ResourceTypeCopy::EventSource => ResourceType::EventSource,
                ResourceTypeCopy::WebSocket => ResourceType::WebSocket,
                ResourceTypeCopy::Manifest => ResourceType::Manifest,
                ResourceTypeCopy::SignedExchange => ResourceType::SignedExchange,
                ResourceTypeCopy::Ping => ResourceType::Ping,
                ResourceTypeCopy::CspViolationReport => ResourceType::CspViolationReport,
                ResourceTypeCopy::Preflight => ResourceType::Preflight,
                ResourceTypeCopy::Other => ResourceType::Other,
            };

            &_type == resource_type
        });

        extension_matched && resource_type_matched
    }
}

fn file_path(url: &str, directory: &Option<PathBuf>) -> PathBuf {
    let mut filename = PathBuf::from(
        url.split('?')
            .next()
            .unwrap()
            .split('/')
            .last()
            .unwrap_or("undefined")
            .chars()
            .map(|x| match x {
                '<' | '>' | ':' | '\"' | '\\' | '|' | '?' => '_',
                _ => x,
            })
            .collect::<String>(),
    );

    let ext = filename
        .extension()
        .and_then(|x| x.to_str())
        .unwrap_or("undefined")
        .to_owned();
    filename.set_extension("");
    let prefix = "vsd_collect";

    let mut path = PathBuf::from(format!("{}_{}.{}", prefix, filename.to_string_lossy(), ext));

    if let Some(directory) = directory {
        path = directory.join(path);
    }

    if path.exists() {
        for i in 1.. {
            path.set_file_name(format!(
                "{}_{}_({}).{}",
                prefix,
                filename.to_string_lossy(),
                i,
                ext
            ));

            if !path.exists() {
                return path;
            }
        }
    }

    path
}
