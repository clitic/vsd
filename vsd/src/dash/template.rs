/*
    REFERENCES
    ----------

    1. https://github.com/emarsden/dash-mpd-rs/blob/7e985069fd95fd5d9993b7610c28228d2448aea7/src/fetch.rs#L1422-L1460

*/

use regex::Regex;
use std::collections::HashMap;

pub(super) struct Template {
    re_representation_id: Regex,
    re_number: Regex,
    re_time: Regex,
    re_bandwidth: Regex,
    vars: HashMap<String, String>,
}

impl Template {
    pub(super) fn new(vars: HashMap<String, String>) -> Self {
        Self {
            re_representation_id: Regex::new("\\$RepresentationID%0([\\d])d\\$").unwrap(),
            re_number: Regex::new("\\$Number%0([\\d])d\\$").unwrap(),
            re_time: Regex::new("\\$Time%0([\\d])d\\$").unwrap(),
            re_bandwidth: Regex::new("\\$Bandwidth%0([\\d])d\\$").unwrap(),
            vars,
        }
    }

    pub(super) fn insert(&mut self, var: &str, val: String) {
        self.vars.insert(var.to_owned(), val);
    }

    pub(super) fn resolve(&self, template: &str) -> String {
        let mut template = template.to_owned();

        for (var, ident_re) in [
            ("RepresentationID", &self.re_representation_id),
            ("Number", &self.re_number),
            ("Time", &self.re_time),
            ("Bandwidth", &self.re_bandwidth),
        ] {
            let ident = format!("${var}$");

            if template.contains(&ident) {
                if let Some(value) = self.vars.get(var) {
                    template = template.replace(&ident, value);
                }
            }

            if let Some(cap) = ident_re.captures(&template) {
                if let Some(value) = self.vars.get(var) {
                    let count = format!(
                        "{:0>width$}",
                        value,
                        width = cap[1].parse::<usize>().unwrap()
                    );
                    let m = ident_re.find(&template).unwrap();
                    template = template[..m.start()].to_owned() + &count + &template[m.end()..];
                }
            }
        }

        template
    }
}
