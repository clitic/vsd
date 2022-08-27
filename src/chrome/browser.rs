use super::{middleware, utils};
use anyhow::{anyhow, Result};
use headless_chrome::protocol::network::events::ResourceType;
use headless_chrome::{Browser, LaunchOptionsBuilder};
use kdam::term::Colorizer;

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

    tab.enable_response_handling(Box::new(move |params, _| {
        let url = params.response.url.split('?').next().unwrap();

        if url.contains(".m3u") || url.contains(".mpd") {
            println!(
                "{}\n{}",
                "-".repeat(crate::utils::get_columns() as usize)
                    .colorize("#FFA500"),
                url
            );
        }
    }))
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

    tab.enable_response_handling(Box::new(move |params, get_response_body| {
        if params._type == ResourceType::XHR || params._type == ResourceType::Fetch {
            let url = params.response.url.split('?').next().unwrap();

            if url.contains(".m3u")
                || url.contains(".mpd")
                || url.contains(".vtt")
                || url.contains(".srt")
            {
                if let Ok(body) = get_response_body() {
                    middleware::save_to_disk(url, body, build).unwrap();
                }
            }
        }
    }))
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
