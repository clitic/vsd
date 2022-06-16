use std::str::FromStr;

use anyhow::{bail, Result};
use kdam::term::Colorizer;
use reqwest::{blocking::Response, header};

fn check_status_code(resp: &Response) -> Result<()> {
    if resp.status() != reqwest::StatusCode::OK {
        bail!(
            "{} download failed with {} {} status code.",
            resp.url().as_str(),
            "HTTP".colorize("magenta"),
            resp.status().colorize("bold red")
        );
    }

    Ok(())
}

#[derive(Debug, Clone)]
pub struct Downloader {
    client: reqwest::blocking::Client,
}

impl Downloader {
    pub fn new(
        user_agent: &str,
        header: &Vec<String>,
        proxy_address: &Option<String>,
        cookies: &Vec<String>,
    ) -> Result<Self> {
        let mut client_builder = reqwest::blocking::Client::builder().user_agent(user_agent);

        if header.len() != 0 {
            let mut headers = header::HeaderMap::new();

            for i in (0..headers.len()).step_by(2) {
                headers.insert(
                    header::HeaderName::from_str(header[i].as_str())?,
                    header::HeaderValue::from_str(header[i + 1].as_str())?,
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

        if cookies.len() != 0 {
            let jar = reqwest::cookie::Jar::default();
            jar.add_cookie_str(&cookies[0], &cookies[1].parse::<reqwest::Url>()?);
            client_builder = client_builder.cookie_store(true);
            client_builder = client_builder.cookie_provider(std::sync::Arc::new(jar));
        }

        Ok(Self {
            client: client_builder.build()?,
        })
    }

    pub fn get(&self, url: &str) -> Result<Response> {
        let resp = self.client.get(url).send()?;
        check_status_code(&resp)?;
        Ok(resp)
    }

    pub fn get_bytes(&self, url: &str) -> Result<Vec<u8>> {
        let resp = self.client.get(url).send()?;
        check_status_code(&resp)?;
        Ok(resp.bytes()?.to_vec())
    }

    pub fn get_bytes_range(&self, url: &str, start: u64, end: u64) -> Result<Vec<u8>> {
        let range = header::HeaderValue::from_str(&format!("bytes={}-{}", start, end))?;
        let resp = self.client.get(url).header(header::RANGE, range).send()?;
        check_status_code(&resp)?;
        Ok(resp.bytes()?.to_vec())
    }
}
