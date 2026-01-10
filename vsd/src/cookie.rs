/*
    REFERENCES
    ----------

    1. https://docs.rs/headless_chrome/1.0.17/headless_chrome/protocol/cdp/Network/struct.CookieParam.html

*/

use cookie::Cookie;
use reqwest::{
    Url,
    cookie::{CookieStore, Jar},
    header::HeaderValue,
};
use serde::Deserialize;

#[allow(dead_code)]
#[derive(Clone, Debug, Default, Deserialize)]
pub struct CookieParam {
    #[serde(default)]
    #[serde(rename = "name")]
    pub name: String,
    #[serde(default)]
    #[serde(rename = "value")]
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    #[serde(rename = "url")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    #[serde(rename = "domain")]
    pub domain: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    #[serde(rename = "path")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    #[serde(rename = "secure")]
    pub secure: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    #[serde(rename = "httpOnly")]
    pub http_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "sameSite")]
    pub same_site: Option<CookieSameSite>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "expires")]
    pub expires: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "priority")]
    pub priority: Option<CookiePriority>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    #[serde(rename = "sameParty")]
    pub same_party: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "sourceScheme")]
    pub source_scheme: Option<CookieSourceScheme>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    #[serde(rename = "sourcePort")]
    pub source_port: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "partitionKey")]
    pub partition_key: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub enum CookiePriority {
    #[serde(rename = "Low")]
    Low,
    #[serde(rename = "Medium")]
    Medium,
    #[serde(rename = "High")]
    High,
}

#[derive(Clone, Debug, Deserialize)]
pub enum CookieSameSite {
    #[serde(rename = "Strict")]
    Strict,
    #[serde(rename = "Lax")]
    Lax,
    #[serde(rename = "None")]
    None,
}

#[derive(Clone, Debug, Deserialize)]
pub enum CookieSourceScheme {
    #[serde(rename = "Unset")]
    Unset,
    #[serde(rename = "NonSecure")]
    NonSecure,
    #[serde(rename = "Secure")]
    Secure,
}

impl CookieParam {
    pub fn new(name: &str, value: &str) -> Self {
        Self {
            name: name.to_owned(),
            value: value.to_owned(),
            ..Default::default()
        }
    }

    pub fn as_cookie(&self) -> Cookie<'_> {
        if self.url.is_some() {
            let mut cookie = Cookie::new(&self.name, &self.value);

            if let Some(domain) = &self.domain {
                cookie.set_domain(domain);
            }

            if let Some(path) = &self.path {
                cookie.set_path(path);
            }

            cookie.set_secure(self.secure);
            cookie.set_http_only(self.http_only);

            if let Some(same_site) = &self.same_site {
                match same_site {
                    CookieSameSite::Strict => cookie.set_same_site(cookie::SameSite::Strict),
                    CookieSameSite::Lax => cookie.set_same_site(cookie::SameSite::Lax),
                    CookieSameSite::None => cookie.set_same_site(cookie::SameSite::None),
                }
            }

            if let Some(expires) = &self.expires {
                let mut now = cookie::time::OffsetDateTime::now_utc();
                now += cookie::time::Duration::seconds_f64(*expires);
                cookie.set_expires(now);
            }

            cookie
        } else {
            Cookie::new(&self.name, &self.value)
        }
    }
}

pub struct CookieJar {
    document_cookie: String,
    inner: Jar,
}

impl CookieJar {
    pub fn new() -> Self {
        Self {
            document_cookie: "".to_owned(),
            inner: Jar::default(),
        }
    }

    pub fn add_cookie(&mut self, cookie: Cookie) {
        self.document_cookie += &format!("{}; ", cookie.stripped());
    }

    pub fn add_cookie_str(&self, cookie: &str, url: &Url) {
        self.inner.add_cookie_str(cookie, url)
    }
}

impl CookieStore for CookieJar {
    fn cookies(&self, url: &Url) -> Option<HeaderValue> {
        if self.document_cookie.is_empty() {
            self.inner.cookies(url)
        } else if let Some(cookies) = self.inner.cookies(url) {
            Some(
                HeaderValue::from_str(
                    &(self.document_cookie.clone()
                        + cookies
                            .to_str()
                            .expect("could not convert cookie header value to string.")),
                )
                .expect("could not construct cookie header value."),
            )
        } else {
            Some(
                HeaderValue::from_str(&self.document_cookie)
                    .expect("could not construct cookie header value."),
            )
        }
    }

    fn set_cookies(&self, cookie_headers: &mut dyn Iterator<Item = &HeaderValue>, url: &Url) {
        self.inner.set_cookies(cookie_headers, url)
    }
}
