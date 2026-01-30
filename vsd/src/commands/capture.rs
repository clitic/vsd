use crate::utils;
use anyhow::{Result, anyhow};
use chromiumoxide::{
    Browser, BrowserConfig,
    cdp::browser_protocol::network::{EventRequestWillBeSent, Request, ResourceType},
};
use clap::{
    Args,
    builder::{PossibleValuesParser, TypedValueParser},
};
use colored::Colorize;
use log::info;
use serde_json::Value;
use std::{fs::File, path::PathBuf};
use tokio_stream::StreamExt;

/// Capture playlists and subtitles requests from a website.
#[derive(Args, Clone, Debug)]
#[clap(
    long_about = "Capture playlists and subtitles requests from a website.\n\n\
Requires one of the following browsers to be installed:\n\
* chrome   - https://www.google.com/chrome\n\
* chromium - https://www.chromium.org/getting-involved/download-chromium\n\n\
This command launches an automated browser instance and listen on requests. \
Behavior may vary, and it may not work as expected on all websites. \
This is equivalent to manually doing:\n\
Inspect -> Network -> Fetch/XHR -> Filter by extension -> Copy as cURL (bash)"
)]
pub struct Capture {
    /// http(s)://
    #[arg(required = true)]
    url: String,

    /// Launch browser with cookies loaded from a json file.
    #[arg(long, value_name = "PATH")]
    cookies: Option<PathBuf>,

    /// List of file extensions to be filter out seperated by comma.
    #[arg(
        long,
        default_value = ".m3u,.m3u8,.mpd,.vtt,.ttml,.srt",
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
    #[arg(
        long, default_value = "fetch,xhr", value_delimiter = ',',
        value_parser = PossibleValuesParser::new([
            "document", "stylesheet", "image", "media", "font", "script", "texttrack",
            "xhr", "fetch", "prefetch", "eventsource", "websocket", "manifest",
            "signedexchange", "ping", "cspviolationreport", "preflight", "fedcm", "other",
        ]).map(|s| s.parse::<ResourceType>().unwrap())
    )]
    resource_types: Vec<ResourceType>,

    /// Save browser cookies in vsd-cookies.json file.
    #[arg(long)]
    save_cookies: bool,
}

impl Capture {
    pub async fn execute(self) -> Result<()> {
        let mut config = BrowserConfig::builder().viewport(None).no_sandbox();

        if self.headless {
            config = config.new_headless_mode();
        } else {
            config = config.with_head();
        }

        if let Some(proxy) = self.proxy {
            config = config.arg(format!("--proxy-server=\"{}\"", proxy));
        }

        let (mut browser, mut handler) =
            Browser::launch(config.build().map_err(|x| anyhow!(x))?).await?;

        let handle = tokio::spawn(async move {
            while let Some(h) = handler.next().await {
                if h.is_err() {
                    break;
                }
            }
        });

        if let Some(cookies) = self.cookies {
            browser
                .set_cookies(serde_json::from_reader(File::open(cookies)?)?)
                .await?;
        }

        let extensions = self.extensions;
        let resource_types = self.resource_types;

        let page = browser.new_page("about:blank").await?;
        let mut request_events = page.event_listener::<EventRequestWillBeSent>().await?;

        tokio::spawn(async move {
            while let Some(event) = request_events.next().await {
                if let Some(resource_type) = &event.r#type {
                    let url = &event.request.url;
                    let url = url.split('?').next().unwrap();

                    if extensions.iter().any(|x| url.ends_with(x))
                        && resource_types.iter().any(|x| x == resource_type)
                    {
                        log_curl_cmd(&event.request);
                    }
                }
            }
        });

        page.goto(&self.url).await?;

        tokio::signal::ctrl_c().await.unwrap();

        if self.save_cookies {
            serde_json::to_writer(
                File::create("vsd-cookies.json")?,
                &browser.get_cookies().await?,
            )?;
        }

        browser.close().await?;
        handle.await?;
        Ok(())
    }
}

fn log_curl_cmd(request: &Request) {
    let mut cmd = format!("curl -X {}", request.method);

    if let Value::Object(obj) = request.headers.inner() {
        for (key, value) in obj {
            if key.to_lowercase() == "content-length" {
                continue;
            }

            let value = value.as_str().unwrap_or("");
            let value = value.replace("'", "'\\''");
            cmd.push_str(&format!(" {} '{}: {}'", "-H", key, value));
        }
    }

    if let Some(entries) = &request.post_data_entries {
        let mut body = String::new();

        for entry in entries {
            if let Some(b64_str) = &entry.bytes
                && let Ok(decoded_bytes) = utils::decode_base64(b64_str)
            {
                body.push_str(&String::from_utf8_lossy(&decoded_bytes));
            }
        }

        if !body.is_empty() {
            let body = body.replace("'", "'\\''");
            cmd.push_str(&format!(" -d '{}'", body));
        }
    }

    let url = request.url.replace("'", "'\\''");
    cmd.push_str(&format!(" '{}' --compressed", url.cyan()));

    info!("{}", "-".repeat(40).dimmed());
    info!("{}", cmd);
}
