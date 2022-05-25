use std::str::FromStr;

use anyhow::{bail, Context, Result};
use reqwest::blocking::Client;
use reqwest::blocking::Response;
use reqwest::header;
use reqwest::StatusCode;

pub struct PartialRangeIter {
    start: u64,
    end: u64,
    buffer_size: u32,
}

impl PartialRangeIter {
    pub fn new(start: u64, end: u64, buffer_size: u32) -> Result<Self> {
        if buffer_size == 0 {
            bail!("invalid buffer_size, give a value greater than zero.");
        }

        Ok(Self {
            start,
            end,
            buffer_size,
        })
    }
}

impl Iterator for PartialRangeIter {
    type Item = header::HeaderValue;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start > self.end {
            None
        } else {
            let prev_start = self.start;
            self.start += std::cmp::min(self.buffer_size as u64, self.end - self.start + 1);
            Some(
                header::HeaderValue::from_str(&format!("bytes={}-{}", prev_start, self.start - 1))
                    .expect("string provided by format!"),
            )
        }
    }
}

#[derive(Clone)]
pub struct Downloader {
    pub client: Client,
}

impl Downloader {
    pub fn new() -> Result<Self> {
        Ok(Self {
            client:  Client::builder().user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/101.0.4951.64 Safari/537.36").build()?,
        })
    }

    pub fn new_custom(user_agent: String, header: Vec<String>, proxy_address: Option<String>) -> Result<Self> {
        let mut client_builder = Client::builder().user_agent(user_agent);

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

        Ok(Self {
            client: client_builder.build()?,
        })
    }

    pub fn head(&self, url: String) -> Result<Response> {
        Ok(self.client.head(url).send()?)
    }

    pub fn get(&self, url: String) -> Result<Response> {
        Ok(self.client.get(url).send()?)
    }

    pub fn get_range(&self, url: String, range: header::HeaderValue) -> Result<Response> {
        Ok(self.client.get(url).header(header::RANGE, range).send()?)
    }

    pub fn get_bytes(&self, url: String) -> Result<Vec<u8>> {
        let resp = self.client.get(&url).send()?;

        if resp.status() == StatusCode::OK {
            Ok(resp.bytes()?.to_vec())
        } else {
            bail!("{} download failed. http code: {}", url, resp.status());
        }
    }

    pub fn get_bytes_range(&self, url: String, start: u64, end: u64) -> Result<Vec<u8>> {
        let range = header::HeaderValue::from_str(&format!("bytes={}-{}", start, end))?;
        let resp = self.client.get(&url).header(header::RANGE, range).send()?;

        if resp.status() == StatusCode::OK {
            Ok(resp.bytes()?.to_vec())
        } else {
            bail!("{} download failed. http code: {}", url, resp.status());
        }
    }


    // pub fn get_bytes_stream(&self, url: String, chunk_size: u32) {
    //     let response = self.head(url.clone());
    //     let length = content_length(&response);

    //     for range in PartialRangeIter::new(0, length - 1, chunk_size) {

    //         let mut response = self.get_range(url.clone(), range);

    //         let status = response.status();
    //         if !(status == StatusCode::OK || status == StatusCode::PARTIAL_CONTENT) {
    //             panic!("Unexpected server response: {}", status);
    //         }

    //         response.bytes();
    //     }

    //     let content = response.bytes().unwrap();

    // }
}

pub fn content_length(response: &Response) -> Result<u64> {
    if let Some(length) = response.headers().get(header::CONTENT_LENGTH) {
        Ok(length
            .to_str()?
            .parse::<u64>()
            .context("failed to parse content length")?)
    } else {
        bail!("response doesn't include the content length");
    }
}
