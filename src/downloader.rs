use anyhow::{bail, Result};
use kdam::term::Colorizer;
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::StatusCode;
// use std::io::Write;
use std::str::FromStr;

pub fn can_retry(error: &reqwest::Error) -> Result<String> {
    let url = error.url().map(|x| x.to_string()).unwrap_or("".to_owned());

    if error.is_timeout() {
        return Ok(format!(
            "{} {}",
            "REQUEST TIMEOUT".colorize("bold red"),
            url
        ));
    } else if error.is_connect() {
        return Ok(format!(
            "{} {}",
            "CONNECTION ERROR".colorize("bold red"),
            url
        ));
    }
    if let Some(status) = error.status() {
        match status {
            StatusCode::REQUEST_TIMEOUT => Ok(format!(
                "{} {}",
                "REQUEST TIMEOUT".colorize("bold red"),
                url
            )),
            StatusCode::TOO_MANY_REQUESTS => Ok(format!(
                "{ }{}",
                "TOO MANY REQUESTS".colorize("bold red"),
                url
            )),
            StatusCode::SERVICE_UNAVAILABLE => Ok(format!(
                "{} {}",
                "SERVICE UNAVAILABLE".colorize("bold red"),
                url
            )),
            StatusCode::GATEWAY_TIMEOUT => Ok(format!(
                "{} {}",
                "GATEWAY TIMEOUT".colorize("bold red"),
                url
            )),
            _ => bail!("HTTP {} {}", status.to_string().colorize("bold red"), url),
        }
    } else {
        bail!("{} download failed.", url)
    }
}


pub fn create_client(
    user_agent: &str,
    header: &Vec<String>,
    proxy_address: &Option<String>,
    enable_cookies: bool,
    cookies: &Vec<String>,
) -> Result<Client> {
    let mut client_builder = Client::builder().user_agent(user_agent);

    if !header.is_empty() {
        let mut headers = HeaderMap::new();

        for i in (0..headers.len()).step_by(2) {
            headers.insert(
                HeaderName::from_str(header[i].as_str())?,
                HeaderValue::from_str(header[i + 1].as_str())?,
            );
        }

        client_builder = client_builder.default_headers(headers);
    }

    if let Some(proxy) = proxy_address {
        if proxy.starts_with("https") {
            client_builder = client_builder.proxy(reqwest::Proxy::https(proxy)?);
        } else if proxy.starts_with("http") {
            client_builder = client_builder.proxy(reqwest::Proxy::http(proxy)?);
        }
    }

    if enable_cookies || !cookies.is_empty() {
        client_builder = client_builder.cookie_store(true);
    }

    if !cookies.is_empty() {
        let jar = reqwest::cookie::Jar::default();

        for i in (0..cookies.len()).step_by(2) {
            jar.add_cookie_str(&cookies[i], &cookies[i + 1].parse::<reqwest::Url>()?);
        }

        client_builder = client_builder.cookie_provider(std::sync::Arc::new(jar));
    }

    Ok(client_builder.build()?)
}

// #[derive(Debug, Clone)]
// pub struct Downloader {
//     client: reqwest::blocking::Client,
// }

// impl Downloader {


//     pub fn get(&self, url: &str) -> Result<Response> {
//         let resp = self.client.get(url).send()?;
//         check_status(&resp)?;
//         Ok(resp)
//     }

//     pub fn get_json(&self, url: &str) -> Result<serde_json::Value> {
//         let resp = self.client.get(url).send()?;
//         check_status(&resp)?;
//         Ok(serde_json::from_str(&resp.text()?)?)
//     }

//     pub fn get_bytes(&self, url: &str) -> Result<Vec<u8>> {
//         let resp = self.client.get(url).send()?;
//         check_status(&resp)?;
//         Ok(resp.bytes()?.to_vec())
//     }

//     pub fn get_bytes_range(&self, url: &str, start: u64, end: u64) -> Result<Vec<u8>> {
//         let range = header::HeaderValue::from_str(&format!("bytes={}-{}", start, end))?;
//         let resp = self.client.get(url).header(header::RANGE, range).send()?;
//         check_status(&resp)?;
//         Ok(resp.bytes()?.to_vec())
//     }

//     pub fn write_to_file(&self, url: &str, path: &str) -> Result<()> {
//         std::fs::File::create(path)?.write_all(&self.get_bytes(url)?)?;
//         Ok(())
//     }
// }
