use crate::utils;
use anyhow::Result;
use clap::{Args, ValueEnum};
use cookie::Cookie;
use headless_chrome::{
    Browser, LaunchOptionsBuilder,
    protocol::cdp::Network::{
        CookieParam, GetResponseBodyReturnObject, ResourceType, events::ResponseReceivedEventParams,
    },
};
use kdam::term::Colorizer;
use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    sync::mpsc,
};

type CookieParams = Vec<CookieParam>;

/// Capture playlists and subtitles from a website.
#[derive(Args, Clone, Debug)]
#[clap(long_about = "Capture playlists and subtitles from a website.\n\n\
Requires any one of these browser to be installed:\n\
1. chrome - https://www.google.com/chrome\n\
2. chromium - https://www.chromium.org/getting-involved/download-chromium\n\n\
Launches browser and capture files based on extension matching.\n\
It is same as doing these steps: Inspect > Network > Fetch/XHR > filter extension and viewing url.\n\
It uses response handling for capturing request information, no requests are intercepted.\n\
Note that this might not work always as expected on every website.")]
pub struct Capture {
    /// http(s)://
    #[arg(required = true)]
    url: String,

    /// Fill browser with some existing cookies value.
    /// Cookies value can be same as document.cookie or in json format same as puppeteer.
    #[arg(long, default_value = "", hide_default_value = true, value_parser = cookie_parser)]
    cookies: CookieParams,

    /// Change directory path for downloaded files.
    /// By default current working directory is used.
    #[arg(short, long)]
    directory: Option<PathBuf>,

    /// List of file extensions to be filter out seperated by comma.
    #[arg(
        long,
        default_values_t = [
            "m3u".to_owned(),
            "m3u8".to_owned(),
            "mpd".to_owned(),
            "vtt".to_owned(),
            "srt".to_owned(),
        ],
        value_delimiter = ','
    )]
    extensions: Vec<String>,

    /// Launch browser without a window.
    #[arg(long)]
    headless: bool,

    /// Launch browser with a proxy.
    #[arg(long)]
    proxy: Option<String>,

    /// List of resource types to be filter out seperated by commas.
    #[arg(long, value_enum, default_values_t = [ResourceTypeCopy::Xhr, ResourceTypeCopy::Fetch], value_delimiter = ',')]
    resource_types: Vec<ResourceTypeCopy>,

    /// Save captured requests responses locally.
    #[arg(long)]
    save: bool,

    /// Save session cookies in json format locally.
    #[arg(long)]
    save_cookies: bool,
}

impl Capture {
    pub fn execute(self) -> Result<()> {
        let (tx, rx) = mpsc::channel();
        ctrlc::set_handler(move || {
            tx.send(())
                .expect("could not send shutdown signal on channel.")
        })?;

        println!(
            "       {} sometimes video starts playing but links are not detected",
            "Note".colorize("cyan")
        );

        println!(
            "    {} launching in {} mode",
            "Browser".colorize("cyan"),
            if self.headless {
                "headless (no window)"
            } else {
                "headful (window)"
            }
        );

        let browser = Browser::new(
            LaunchOptionsBuilder::default()
                .headless(self.headless)
                .proxy_server(self.proxy.as_deref())
                .build()?,
        )?;
        let tab = browser.new_tab()?;

        println!("    {} setting cookies", "Browser".colorize("cyan"));
        tab.set_cookies(self.cookies)?;

        let directory = if self.save {
            self.directory.clone()
        } else {
            None
        };
        let filters = Filters {
            extensions: self.extensions,
            resource_types: self.resource_types,
        };
        let save = self.save;

        println!(
            "    {} registering response listener",
            "Browser".colorize("cyan")
        );
        tab.register_response_handling(
            "vsd-capture",
            Box::new(move |params, get_response_body| {
                handler(
                    directory.as_ref(),
                    &filters,
                    get_response_body,
                    params,
                    save,
                );
            }),
        )?;

        println!(
            "    {} navigating to {}",
            "Browser".colorize("cyan"),
            self.url
        );
        tab.navigate_to(&self.url)?;

        println!(
            "       {} waiting for CTRL+C signal",
            "Note".colorize("cyan")
        );
        rx.recv()?;
        println!(
            "    {} deregistering response listener and closing browser",
            "Browser".colorize("cyan")
        );
        let _ = tab.deregister_response_handling("vsd-capture")?;

        if self.save_cookies {
            println!("{} session cookies", "Downloading".colorize("bold green"));

            if let Some(directory) = &self.directory {
                if !directory.exists() {
                    fs::create_dir_all(directory).unwrap();
                }
            };

            let mut path = PathBuf::from("cookies.json");

            if let Some(directory) = &self.directory {
                path = directory.join(path);
            }

            let file = File::create(path)?;
            serde_json::to_writer(file, &tab.get_cookies()?)?;
        }
        Ok(())
    }
}

fn handler(
    directory: Option<&PathBuf>,
    filters: &Filters,
    get_response_body: &dyn Fn() -> Result<GetResponseBodyReturnObject>,
    params: ResponseReceivedEventParams,
    save: bool,
) {
    if !filters.pass(&params.Type, &params.response.url) {
        return;
    }

    if save {
        println!(
            "{} {}",
            "Downloading".colorize("bold green"),
            params.response.url,
        );

        if let Ok(body) = get_response_body() {
            if let Some(directory) = directory {
                if !directory.exists() {
                    fs::create_dir_all(directory).unwrap();
                }
            };

            let mut path = PathBuf::from(
                params
                    .response
                    .url
                    .split('?')
                    .next()
                    .unwrap()
                    .split('/')
                    .next_back()
                    .unwrap_or("und"),
            );

            if path.extension().is_none() {
                path.set_extension("txt");
            }

            if let Some(directory) = directory {
                path = directory.join(path);
            }

            let mut file = File::create(path).unwrap();

            if body.base_64_encoded {
                file.write_all(&utils::decode_base64(&body.body).unwrap())
                    .unwrap();
            } else {
                file.write_all(body.body.as_bytes()).unwrap();
            }
        }
    } else {
        println!(
            "   {} {}",
            "Detected".colorize("bold green"),
            params.response.url,
        );
    }
}

struct Filters {
    extensions: Vec<String>,
    resource_types: Vec<ResourceTypeCopy>,
}

impl Filters {
    fn pass(&self, resource_type: &ResourceType, url: &str) -> bool {
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

#[derive(Clone, Debug, ValueEnum)]
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
    if s.is_empty() {
        Ok(Vec::new())
    } else if Path::new(s).exists() {
        Ok(serde_json::from_slice::<CookieParams>(
            &fs::read(s).map_err(|_| format!("could not read {}.", s))?,
        )
        .map_err(|_| format!("could not deserialize cookies from {}.", s))?)
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
                Err(_) => return Err("could not split parse cookies.".to_owned()),
            }
        }

        Ok(cookies)
    }
}
