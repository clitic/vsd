use anyhow::Result;
use regex::Regex;
use std::collections::HashMap;

pub fn join_url(url1: &str, url2: &str) -> Result<String> {
    Ok(url1
        .parse::<reqwest::Url>()?
        .join(url2)?
        .as_str()
        .to_owned())
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

pub struct TemplateResolver {
    re_representation_id: Regex,
    re_number: Regex,
    re_time: Regex,
    re_bandwidth: Regex,
    vars: HashMap<String, String>
}

impl TemplateResolver {
    pub fn new(vars: HashMap<String, String>) -> Self {
        Self {
            re_representation_id: Regex::new("\\$RepresentationID%0([\\d])d\\$").unwrap(),
            re_number: Regex::new("\\$Number%0([\\d])d\\$").unwrap(),
            re_time: Regex::new("\\$Time%0([\\d])d\\$").unwrap(),
            re_bandwidth: Regex::new("\\$Bandwidth%0([\\d])d\\$").unwrap(),
            vars,
        }
    }

    pub fn insert(&mut self, var: &str, val: String) {
        self.vars.insert(var.to_owned(), val);
    }

    // https://github.com/emarsden/dash-mpd-rs/blob/main/src/fetch.rs#L321
    pub fn resolve(&self, template: &str) -> String {
        let mut template = template.to_owned();

        for (var, ident_re) in [
            ("RepresentationID", &self.re_representation_id),
            ("Number", &self.re_number),
            ("Time", &self.re_time),
            ("Bandwidth", &self.re_bandwidth),
        ] {
            let ident = format!("${}$", var);

            if template.contains(&ident) {
                if let Some(value) = self.vars.get(var) {
                    template = template.replace(&ident, value);
                }
            }

            if let Some(cap) = ident_re.captures(&template) {
                if let Some(value) = self.vars.get(var) {
                    let width: usize = cap[1].parse::<usize>().unwrap();
                    let count = format!("{:0>width$}", value, width = width);
                    let m = ident_re.find(&template).unwrap();
                    template = template[..m.start()].to_owned() + &count + &template[m.end()..];
                }
            }
        }

        template
    }
}
