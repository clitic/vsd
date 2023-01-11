// REFERENCES: https://www.w3.org/TR/2018/REC-ttml2-20181108

use super::Cue;
use crate::utils;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
// #[serde(rename = "tt")]
pub(super) struct TT<T> {
    pub(super) body: Body<T>,
}

impl TT<DivValue> {
    pub(super) fn format(&self) -> Vec<String> {
        let span_re = regex::Regex::new("<span.*span>").unwrap();
        let mut formatted_p = vec![];

        for p in &self.body.div.p {
            let mut p_clone = p.replace("<br></br>", "\n").replace("<br/>", "\n");

            for m in span_re.find_iter(p) {
                let span = quick_xml::de::from_str::<Span>(m.as_str()).unwrap();
                p_clone = p.replace(m.as_str(), &span.format());
            }

            formatted_p.push(p_clone);
        }

        formatted_p
    }
}

impl TT<DivAttributes> {
    pub(super) fn to_cues(&self, value: &TT<DivValue>) -> Vec<Cue> {
        let mut cues = vec![];

        for (p, payload) in self.body.div.p.iter().zip(value.format()) {
            cues.push(Cue::new(
                &payload,
                utils::duration(&p.begin).unwrap(),
                utils::duration(&p.end).unwrap(),
            ));
        }

        cues
    }
}

#[derive(Debug, Deserialize)]
pub(super) struct Body<T> {
    #[serde(rename = "div")]
    pub(super) div: T,
}

#[derive(Debug, Deserialize)]
pub(super) struct DivValue {
    #[serde(rename = "p", default)]
    pub(super) p: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct DivAttributes {
    #[serde(rename = "p", default)]
    pub(super) p: Vec<Paragraph>,
}

#[derive(Debug, Deserialize)]
pub(super) struct Paragraph {
    #[serde(rename = "begin")]
    pub(super) begin: String,
    #[serde(rename = "end")]
    pub(super) end: String,
    // #[serde(rename = "$value")]
    // pub(super) value: String,
}

#[derive(Debug, Default, Clone, Deserialize)]
// #[serde(rename = "span")]
struct Span {
    // #[serde(rename = "@tts:fontStyle")]
    #[serde(rename = "@fontStyle")]
    font_style: Option<String>,
    #[serde(rename = "$value")]
    value: String,
}

impl Span {
    fn format(&self) -> String {
        if let Some(font_style) = &self.font_style {
            if font_style == "italic" || font_style == "oblique" {
                return format!("<i>{}</i>", self.value);
            }
        }

        self.value.to_owned()
    }
}
