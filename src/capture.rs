use headless_chrome::browser::tab::RequestInterceptionDecision;
use headless_chrome::protocol::network::methods::RequestPattern;
use headless_chrome::{Browser, LaunchOptionsBuilder};

pub fn run(url: String, headless: bool) {
    let browser = Browser::new(
        LaunchOptionsBuilder::default()
            .headless(headless)
            .build()
            .unwrap(),
    )
    .unwrap();
    let tab = browser.wait_for_initial_tab().unwrap();
    tab.navigate_to(url.as_str()).unwrap();

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
    .unwrap();

    std::thread::sleep(std::time::Duration::from_secs(60 * 3));
}
