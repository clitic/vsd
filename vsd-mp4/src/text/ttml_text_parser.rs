/*
    REFERENCES
    ----------

    1. https://github.com/shaka-project/shaka-player/blob/db8987d6dfdb59b9f6d187051d47edf6d846a9ed/lib/text/ttml_text_parser.js
    2. https://w3c.github.io/ttml2
    3. https://www.speechpad.com/captions/ttml

*/

//! Parse ttml content.

use crate::text::{Cue, Subtitles};
pub use quick_xml::de::DeError;
use serde::Deserialize;
use std::num::ParseFloatError;

/// Parse xml as ttml content.
pub fn parse(xml: &str) -> Result<TT, DeError> {
    quick_xml::de::from_str(xml)
}

#[derive(Debug, Deserialize)]
pub struct TT {
    #[serde(rename = "body", default)]
    pub body: Body,
}

#[derive(Debug, Default, Deserialize)]
pub struct Body {
    #[serde(rename = "div", default)]
    pub divs: Vec<Div>,
}

#[derive(Debug, Deserialize)]
pub struct Div {
    #[serde(rename = "p", default)]
    pub paragraphs: Vec<Paragraph>,
}

#[derive(Debug, Deserialize)]
pub struct Paragraph {
    #[serde(rename = "@begin")]
    pub begin: String,
    #[serde(rename = "@end")]
    pub end: String,
    #[serde(rename = "$value", default)]
    pub content: Vec<TtmlContent>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TtmlContent {
    #[serde(rename = "$text")]
    Text(String),
    Span(Span),
    #[serde(rename = "br")]
    Br,
}

#[derive(Debug, Deserialize)]
pub struct Span {
    #[serde(rename = "@tts:color", alias = "@color")]
    pub color: Option<String>,

    #[serde(rename = "@tts:fontStyle", alias = "@fontStyle")]
    pub font_style: Option<String>,

    #[serde(rename = "@tts:fontWeight", alias = "@fontWeight")]
    pub font_weight: Option<String>,

    #[serde(rename = "@tts:textDecoration", alias = "@textDecoration")]
    pub text_decoration: Option<String>,

    #[serde(rename = "$value", default)]
    pub content: Vec<TtmlContent>,
}

impl TtmlContent {
    fn format(&self) -> String {
        match self {
            TtmlContent::Text(t) => t.clone(),
            TtmlContent::Br => "\n".to_owned(),
            TtmlContent::Span(span) => {
                let inner_text: String = span.content.iter().map(|x| x.format()).collect();
                span.format(inner_text)
            }
        }
    }
}

impl Span {
    fn format(&self, mut value: String) -> String {
        if let Some(font_weight) = &self.font_weight
            && font_weight == "bold"
        {
            value = format!("<b>{value}<b>");
        }

        if let Some(font_style) = &self.font_style
            && (font_style == "italic" || font_style == "oblique")
        {
            value = format!("<i>{value}</i>");
        }

        if let Some(text_decoration) = &self.text_decoration
            && text_decoration == "underline"
        {
            value = format!("<u>{value}</u>");
        }

        if let Some(color) = &self.color {
            value = format!("<font color=\"{color}\">{value}</font>");
        }

        value
    }
}

impl TT {
    pub(super) fn into_cues(self) -> Vec<Cue> {
        let mut cues = Vec::new();

        for div in self.body.divs {
            for paragraph in &div.paragraphs {
                cues.push(Cue {
                    end_time: parse_ttml_time(&paragraph.end).unwrap(),
                    _id: String::new(),
                    payload: paragraph.content.iter().map(|x| x.format()).collect(),
                    settings: String::new(),
                    start_time: parse_ttml_time(&paragraph.begin).unwrap(),
                });
            }
        }

        cues
    }

    pub fn into_subtitles(self) -> Subtitles {
        Subtitles::new(self.into_cues())
    }
}

fn parse_ttml_time(input: &str) -> Result<f32, ParseFloatError> {
    assert!(!input.trim().is_empty());

    let seconds = if input.contains(':') {
        parse_clock_time(input, 30.0)?
    } else {
        parse_offset_time(input, 30.0, 1000.0)?
    };

    Ok(seconds)
}

/// Handles "HH:MM:SS.mmm" (Standard) and "HH:MM:SS:FF" (SMPTE)
fn parse_clock_time(input: &str, frame_rate: f32) -> Result<f32, ParseFloatError> {
    let parts = input.split(':').collect::<Vec<&str>>();
    assert!(!(parts.len() < 3 || parts.len() > 4));

    let h = parts[0].parse::<f32>()?;
    let m = parts[1].parse::<f32>()?;

    let (s, frames) = if parts.len() == 4 {
        let s = parts[2].parse::<f32>()?;
        let f = parts[3].parse::<f32>()?;
        (s, f)
    } else {
        let s = parts[2].parse::<f32>()?;
        (s, 0.0)
    };

    Ok((h * 3600.0) + (m * 60.0) + s + (frames / frame_rate))
}

/// Handles "10h", "500ms", "24f", etc.
fn parse_offset_time(input: &str, frame_rate: f32, tick_rate: f32) -> Result<f32, ParseFloatError> {
    let split_idx = input
        .rfind(|c: char| c.is_ascii_digit() || c == '.')
        .unwrap();

    let (value, unit) = input.split_at(split_idx + 1);
    let value = value.parse::<f32>()?;

    match unit.trim() {
        "h" => Ok(value * 3600.0),
        "m" => Ok(value * 60.0),
        "s" => Ok(value),
        "ms" => Ok(value / 1000.0),
        "f" => Ok(value / frame_rate),
        "t" => Ok(value / tick_rate),
        _ => unreachable!(),
    }
}
