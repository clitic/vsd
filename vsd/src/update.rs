use reqwest::blocking::Client;
use serde::Deserialize;
use kdam::term::Colorizer;

#[derive(Deserialize)]
struct Releases {
    url: String,
    version: String,
}

pub(super) fn check_for_new_release(client: &Client) {
    if let Ok(response) = client
        .get("https://raw.githubusercontent.com/clitic/vsd/main/vsd/releases.json")
        .send()
    {
        if let Ok(text) = response.text() {
            if let Ok(releases) = serde_json::from_str::<Vec<Releases>>(&text) {
                if let Some(latest) = releases.first() {
                    if latest.version != env!("CARGO_PKG_VERSION") {
                        println!(
                            "     {} a new release of vsd is available {} -> {}\n            {}",
                            "Notice".colorize("bold cyan"),
                            env!("CARGO_PKG_VERSION").colorize("bold red"),
                            latest.version.colorize("bold green"),
                            latest.url
                        );
                    }
                }
            }
        }
    }
}
