use reqwest::cookie::{CookieStore, Jar};
use reqwest::header::HeaderValue;
use reqwest::Url;

pub(super) struct CookieJar {
    cookie: Option<String>,
    inner: Jar,
}

impl CookieJar {
    pub(super) fn new(cookie: Option<String>) -> Self {
        Self {
            cookie: cookie.map(|x| x.trim().trim_end_matches(';').to_owned()),
            inner: Jar::default(),
        }
    }

    pub(super) fn add_cookie_str(&self, cookie: &str, url: &Url) {
        self.inner.add_cookie_str(cookie, url)
    }
}

impl CookieStore for CookieJar {
    fn cookies(&self, url: &Url) -> Option<HeaderValue> {
        if let Some(cookie) = &self.cookie {
            self.inner
                .cookies(url)
                .map(|x| HeaderValue::from_static(&format!("{}; {}", cookie, x.to_str().unwrap())))
        } else {
            self.inner.cookies(url)
        }
    }

    fn set_cookies(&self, cookie_headers: &mut dyn Iterator<Item = &HeaderValue>, url: &Url) {
        self.inner.set_cookies(cookie_headers, url)
    }
}
