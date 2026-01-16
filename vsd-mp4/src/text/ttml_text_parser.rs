//! Parse ttml content.

/*
    REFERENCES
    ----------

    1. https://github.com/shaka-project/shaka-player/blob/db8987d6dfdb59b9f6d187051d47edf6d846a9ed/lib/text/ttml_text_parser.js
    2. https://w3c.github.io/ttml3
    3. https://www.speechpad.com/captions/ttml

*/

use crate::text::{Cue, Subtitles};
use serde::Deserialize;

pub use quick_xml::de::DeError;

// TODO - Parse span (cdata) in `p` node when quick-xml supports cdata+text parsing.
// https://docs.rs/quick-xml/latest/quick_xml/de/index.html
/// Parse xml as ttml content.
pub fn parse(xml: &str) -> Result<TT, DeError> {
    let mut xml = xml
        .replace("<br></br>", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n");

    while let (Some(start), Some(end)) = (xml.find("<span"), xml.find("span>")) {
        let span_match = xml.get(start..(end + 5)).unwrap();
        let sub_span = xml.get((start + 5)..(end + 5)).unwrap();

        if let (Some(sub_span_start), Some(sub_span_end)) =
            (sub_span.find("<span"), sub_span.find("span>"))
        {
            let sub_span_match = sub_span.get(sub_span_start..(sub_span_end + 5)).unwrap();
            // println!("sub-span-match: {}", span_match);
            let span = quick_xml::de::from_str::<Span>(sub_span_match)?;
            xml = xml.replace(sub_span_match, &span.format());
            continue;
        }

        // println!("span-match: {}", span_match);
        let span = quick_xml::de::from_str::<Span>(span_match)?;
        xml = xml.replace(span_match, &span.format());
    }

    quick_xml::de::from_str(&xml)
}

#[derive(Deserialize)]
struct Span {
    #[serde(rename = "@color")]
    color: Option<String>,
    #[serde(rename = "@fontStyle")]
    font_style: Option<String>,
    #[serde(rename = "@fontWeight")]
    font_weight: Option<String>,
    #[serde(rename = "@textDecoration")]
    text_decoration: Option<String>,
    #[serde(rename = "$value", default)]
    value: String,
}

impl Span {
    fn format(&self) -> String {
        let mut value = self.value.clone();

        if let Some(font_weight) = &self.font_weight
            && font_weight == "bold"
        {
            value = format!("{{b}}{value}{{/b}}");
        }

        if let Some(font_style) = &self.font_style
            && font_style == "italic"
        {
            value = format!("{{i}}{value}{{/i}}");
        }

        if let Some(text_decoration) = &self.text_decoration
            && text_decoration == "underline"
        {
            value = format!("{{u}}{value}{{/u}}");
        }

        if let Some(color) = &self.color {
            value = format!("{{font color=\"{color}\">{value}{{/font}}");
            value = format!("<font color=\"{color}\">{value}</font>");
        }

        value
    }
}

#[derive(Deserialize)]
pub struct TT {
    #[serde(rename = "body")]
    pub body: Body,
}

#[derive(Deserialize)]
pub struct Body {
    #[serde(rename = "div", default)]
    pub divs: Vec<Div>,
}

#[derive(Deserialize)]
pub struct Div {
    #[serde(rename = "p", default)]
    pub paragraphs: Vec<Paragraph>,
}

#[derive(Deserialize)]
pub struct Paragraph {
    #[serde(rename = "@begin")]
    pub begin: String,
    #[serde(rename = "@end")]
    pub end: String,
    #[serde(rename = "$value")]
    pub value: String,
}

impl TT {
    pub(super) fn into_cues(self) -> Vec<Cue> {
        let mut cues = vec![];

        for div in self.body.divs {
            for paragraph in &div.paragraphs {
                cues.push(Cue {
                    end_time: duration(&paragraph.end).unwrap_or_else(|_| {
                        panic!(
                            "mp4parser.ttmltextparser: could'nt convert {} to seconds.",
                            paragraph.end
                        )
                    }),
                    _id: String::new(),
                    payload: paragraph
                        .value
                        .replace("{b}", "<b>")
                        .replace("{/b}", "</b>")
                        .replace("{i}", "<i>")
                        .replace("{/i}", "</i>")
                        .replace("{u}", "<u>")
                        .replace("{/u}", "</u>")
                        .replace("{font", "<font")
                        .replace("{/font}", "</font>"),
                    settings: String::new(),
                    start_time: duration(&paragraph.begin).unwrap_or_else(|_| {
                        panic!(
                            "mp4parser.ttmltextparser: could'nt convert {} to seconds.",
                            paragraph.end
                        )
                    }),
                });
            }
        }

        cues
    }

    pub fn into_subtitles(self) -> Subtitles {
        Subtitles::new(self.into_cues())
    }
}

fn duration(duration: &str) -> Result<f32, std::num::ParseFloatError> {
    let duration = duration.replace('s', "").replace(',', ".");
    let is_frame = duration.split(':').count() >= 4;
    let mut duration = duration.split(':').rev();
    let mut total_seconds = 0.0;

    if is_frame && let Some(seconds) = duration.next() {
        total_seconds += seconds.parse::<f32>()? / 1000.0;
    }

    if let Some(seconds) = duration.next() {
        total_seconds += seconds.parse::<f32>()?;
    }

    if let Some(minutes) = duration.next() {
        total_seconds += minutes.parse::<f32>()? * 60.0;
    }

    if let Some(hours) = duration.next() {
        total_seconds += hours.parse::<f32>()? * 3600.0;
    }

    Ok(total_seconds)
}
