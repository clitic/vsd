use anyhow::{anyhow, Result};
use headless_chrome::browser::tab::RequestInterceptionDecision;
use headless_chrome::protocol::network::methods::RequestPattern;
use headless_chrome::{Browser, LaunchOptionsBuilder};

pub fn run(url: String, headless: bool) -> Result<()> {
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
    tab.navigate_to(url.as_str())
        .map_err(|e| anyhow!(e.to_string()))?;

    tab.enable_request_interception(
        &[RequestPattern {
            url_pattern: None,
            resource_type: Some("XHR"),
            interception_stage: None,
        }],
        Box::new(|_transport, _session_id, intercepted| {
            if intercepted.request.url.contains(".m3u") || intercepted.request.url.contains(".mpd")
            {
                println!("• {}", intercepted.request.url);
            }

            RequestInterceptionDecision::Continue
        }),
    )
    .map_err(|e| anyhow!(e.to_string()))?;

    std::thread::sleep(std::time::Duration::from_secs(60 * 3));
    Ok(())
}
