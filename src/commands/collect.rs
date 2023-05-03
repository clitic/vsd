#![cfg(feature = "chrome")]

use crate::utils;
use anyhow::Result;
use clap::Args;
use cookie::Cookie;
use headless_chrome::{
    protocol::cdp::Network::{
        events::ResponseReceivedEventParams, CookieParam, GetResponseBodyReturnObject, ResourceType,
    },
    Browser, LaunchOptionsBuilder,
};
use kdam::term::Colorizer;
use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
    sync::mpsc,
};

type CookieParams = Vec<CookieParam>;

/// Collect playlists and subtitles from a website and save them locally.
#[derive(Debug, Clone, Args)]
#[clap(
    long_about = "Collect playlists and subtitles from a website and save them locally.\n\n\
Requires any one of these to be installed:\n\
1. chrome - https://www.google.com/chrome\n\
2. chromium - https://www.chromium.org/getting-involved/download-chromium\n\n\
Launch Google Chrome and collect .m3u8 (HLS), .mpd (Dash) and subtitles from a website and save them locally. \
This is done by reading the request response sent by chrome to server. \
This command might not work always as expected."
)]
pub struct Collect {
    /// http(s)://
    #[arg(required = true)]
    url: String,

    /// Fill browser with some existing cookies value.
    /// It can be document.cookie value or in json format same as puppeteer.
    #[arg(long, default_value = "[]", hide_default_value = true, value_parser = cookie_parser)]
    cookies: CookieParams,

    /// Change directory path for downloaded files.
    /// By default current working directory is used.
    #[arg(short, long)]
    directory: Option<PathBuf>,

    /// Launch browser without a window.
    #[arg(long)]
    headless: bool,

    /// Do not download and save responses.
    #[arg(short, long)]
    no_save: bool,
}

fn cookie_parser(s: &str) -> Result<CookieParams, String> {
    if Path::new(s).exists() {
        Ok(serde_json::from_slice::<CookieParams>(
            &std::fs::read(s).map_err(|_| format!("could not read {}.", s))?,
        )
        .map_err(|x| format!("could not deserialize cookies from json file. {}", x))?)
    } else {
        if let Ok(cookies) = serde_json::from_str::<CookieParams>(s) {
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
}

impl Collect {
    pub fn perform(self) -> Result<()> {
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

        let directory = if self.no_save {
            None
        } else {
            self.directory.clone()
        };
        let no_save = self.no_save;

        println!(
            " {} registering response listeners",
            "Browser".colorize("bold cyan")
        );
        tab.register_response_handling(
            "vsd-collect",
            Box::new(move |params, get_response_body| {
                handler(params, get_response_body, &directory, no_save);
            }),
        )?;

        if let Some(directory) = &self.directory {
            if !directory.exists() {
                std::fs::create_dir_all(directory)?;
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
            " {} deregistering response listeners and closing browser",
            "Browser".colorize("bold cyan")
        );
        let _ = tab.deregister_response_handling("vsd-collect")?;

        if let Some(directory) = &self.directory {
            if std::fs::read_dir(directory)?.next().is_none() {
                println!(
                    "{} {}",
                    "Deleting".colorize("bold red"),
                    directory.to_string_lossy()
                );
                std::fs::remove_dir(directory)?;
            }
        }

        Ok(())
    }
}

fn handler(
    params: ResponseReceivedEventParams,
    get_response_body: &dyn Fn() -> Result<GetResponseBodyReturnObject>,
    directory: &Option<PathBuf>,
    no_save: bool,
) {
    if let ResourceType::Xhr | ResourceType::Fetch = params.Type {
        let splitted_url = params.response.url.split('?').next().unwrap();

        if splitted_url.ends_with(".m3u")
            || splitted_url.ends_with(".m3u8")
            || splitted_url.ends_with(".mpd")
            || splitted_url.ends_with(".vtt")
            || splitted_url.ends_with(".srt")
        {
            if no_save {
                println!(
                    "{} {}",
                    "Detected".colorize("bold green"),
                    params.response.url,
                );
                return ();
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
                    } else {
                        if file.write_all(body.body.as_bytes()).is_err() {
                            println!(
                                "  {} could'nt write response all bytes to {}",
                                "Saving".colorize("bold red"),
                                path.to_string_lossy(),
                            );
                        }
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
