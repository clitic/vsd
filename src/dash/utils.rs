use std::collections::HashMap;
use anyhow::Result;

pub fn join_url(url1: &str, url2: &str) -> Result<String> {
    Ok(url1.parse::<reqwest::Url>()?.join(url2)?.as_str().to_owned())
}

pub fn iso8601_duration_to_seconds(duration: &str) -> Result<f32, String> {
    match iso8601::duration(duration)? {
        iso8601::Duration::YMDHMS {
            year,
            month,
            day,
            hour,
            minute,
            second,
            millisecond,
        } => Ok((year as f32 * 365.0 * 31.0 * 24.0 * 60.0 * 60.0)
            + (month as f32 * 31.0 * 24.0 * 60.0 * 60.0)
            + (day as f32 * 24.0 * 60.0 * 60.0)
            + (hour as f32 * 60.0 * 60.0)
            + (minute as f32 * 60.0)
            + second as f32
            + (millisecond as f32 * 0.001)),
        iso8601::Duration::Weeks(w) => Ok(w as f32 * 60.0 * 60.0 * 24.0 * 7.0),
    }
}

// https://github.com/emarsden/dash-mpd-rs/blob/main/src/fetch.rs#L321
pub fn resolve_url_template(template: &str, params: &HashMap<&str, String>) -> String {
    let mut result = template.to_string();
    for k in ["RepresentationID", "Number", "Time", "Bandwidth"] {
        // first check for simple case eg $Number$
        let ident = format!("${}$", k);
        if result.contains(&ident) {
            if let Some(value) = params.get(k as &str) {
                result = result.replace(&ident, value);
            }
        }
        // now check for complex case eg $Number%06d$
        let re = format!("\\${}%0([\\d])d\\$", k);
        let ident_re = regex::Regex::new(&re).unwrap();
        if let Some(cap) = ident_re.captures(&result) {
            if let Some(value) = params.get(k as &str) {
                let width: usize = cap[1].parse::<usize>().unwrap();
                let count = format!("{:0>width$}", value, width = width);
                let m = ident_re.find(&result).unwrap();
                result = result[..m.start()].to_owned() + &count + &result[m.end()..];
            }
        }
    }
    result
}
