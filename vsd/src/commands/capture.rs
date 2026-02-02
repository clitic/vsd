use crate::cookie::Cookies;
use anyhow::{Result, anyhow};
use base64::Engine;
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
use std::path::PathBuf;
use tokio::fs;
use tokio_stream::StreamExt;

/// Capture playlist requests from a website.
#[derive(Args, Clone, Debug)]
#[clap(long_about = "Capture playlist requests from a website.\n\n\
Requires any one of these browsers:\n\n\
- [chrome](https://www.google.com/chrome)\n\
- [chromium](https://www.chromium.org/getting-involved/download-chromium)\n\n\
This command launches an automated browser instance and listen on network requests. \
Behavior may vary, and it may not work as expected on all websites. \
This is equivalent to manually doing:\n\n\
Inspect -> Network -> Fetch/XHR -> Filter by extension -> Copy as cURL (bash)")]
pub struct Capture {
    /// HTTP(S)://
    #[arg(required = true)]
    input: String,

    /// Launch browser with cookies (netscape cookie file).
    #[arg(long, value_name = "PATH")]
    cookies: Option<PathBuf>,

    /// List of file extensions to be filtered out separated by comma.
    #[arg(
        long,
        value_name = "EXT",
        default_value = ".m3u,.m3u8,.mpd,.vtt,.ttml,.srt",
        value_delimiter = ','
    )]
    extensions: Vec<String>,

    /// Launch browser in headless mode (without a window).
    #[arg(long)]
    headless: bool,

    /// Launch browser with a proxy.
    #[arg(long)]
    proxy: Option<String>,

    /// List of resource types to be filtered out separated by comma.
    #[arg(
        long,
        value_name = "TYPES",
        default_value = "fetch,xhr",
        value_delimiter = ',',
        value_parser = PossibleValuesParser::new([
            "document", "stylesheet", "image", "media", "font", "script", "texttrack",
            "xhr", "fetch", "prefetch", "eventsource", "websocket", "manifest",
            "signedexchange", "ping", "cspviolationreport", "preflight", "fedcm", "other",
        ]).map(|s| s.parse::<ResourceType>().unwrap())
    )]
    resource_types: Vec<ResourceType>,

    /// Save browser cookies in cookies.txt (netscape cookie file).
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

        if let Some(path) = self.cookies {
            browser
                .set_cookies(Cookies::parse(&fs::read(path).await?)?.into())
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

        page.goto(&self.input).await?;

        tokio::signal::ctrl_c().await.unwrap();

        if self.save_cookies {
            let cookies = browser.get_cookies().await?;
            let cookies: Cookies = (&cookies).into();
            fs::write("cookies.txt", cookies.to_netscape()).await?;
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
                && let Ok(decoded_bytes) = base64::engine::general_purpose::STANDARD.decode(b64_str)
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
