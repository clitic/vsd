use super::{middleware, utils};
use anyhow::{anyhow, Result};
use headless_chrome::browser::tab::RequestInterceptionDecision;
use headless_chrome::protocol::network::methods::RequestPattern;
use headless_chrome::{Browser, LaunchOptionsBuilder};
use kdam::term::Colorizer;
use reqwest::blocking::Client;
use std::sync::Arc;

pub fn capture(url: &str, headless: bool) -> Result<()> {
    utils::launch_message(headless);

    let browser = Browser::new(
        LaunchOptionsBuilder::default()
            .headless(headless)
            .build()
            .map_err(|e| anyhow!(e))?,
    )
    .map_err(|e| anyhow!(e.to_string()))?;

    let tab = browser
        .wait_for_initial_tab()
        .map_err(|e| anyhow!(e.to_string()))?;

    tab.enable_request_interception(
        &[RequestPattern {
            url_pattern: None,
            resource_type: Some("XHR"),
            interception_stage: Some("Request"),
        }],
        Box::new(move |_, _, intercepted| {
            if intercepted.request.url.contains(".m3u") || intercepted.request.url.contains(".mpd")
            {
                println!(
                    "{}\n{}",
                    "-".repeat(crate::utils::get_columns() as usize)
                        .colorize("#FFA500"),
                    intercepted.request.url
                );
            }

            RequestInterceptionDecision::Continue
        }),
    )
    .map_err(|e| anyhow!(e.to_string()))?;

    tab.navigate_to(url).map_err(|e| anyhow!(e.to_string()))?;
    utils::warning_message();
    std::thread::sleep(std::time::Duration::from_secs(60 * 3));
    Ok(())
}

pub fn collect(url: &str, headless: bool, build: bool) -> Result<()> {
    utils::launch_message(headless);

    let browser = Browser::new(
        LaunchOptionsBuilder::default()
            .headless(headless)
            .build()
            .map_err(|e| anyhow!(e))?,
    )
    .map_err(|e| anyhow!(e.to_string()))?;

    let tab = browser
        .wait_for_initial_tab()
        .map_err(|e| anyhow!(e.to_string()))?;

    let client = Arc::new(Client::new());

    tab.enable_request_interception(
        &[RequestPattern {
            url_pattern: None,
            resource_type: Some("XHR"),
            interception_stage: Some("Request"),
        }],
        Box::new(move |_, _, intercepted| {
            middleware::intercept(intercepted, client.clone(), build).unwrap()
        }),
    )
    .map_err(|e| anyhow!(e.to_string()))?;

    tab.navigate_to(url).map_err(|e| anyhow!(e.to_string()))?;

    println!(
        "Using {} method for collection which might {} as expected.",
        "COMMON".colorize("cyan"),
        "not work".colorize("bold red")
    );

    utils::warning_message();
    std::thread::sleep(std::time::Duration::from_secs(60 * 3));
    Ok(())
}
