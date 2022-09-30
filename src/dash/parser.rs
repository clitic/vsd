// REFERENCES: https://github.com/emarsden/dash-mpd-rs

use super::utils;
use serde::Deserialize;
use std::collections::HashMap;

pub fn parse(xml: &[u8]) -> Result<MPD, quick_xml::de::DeError> {
    quick_xml::de::from_reader::<_, MPD>(xml)
}

#[derive(Debug, Default, Deserialize)]
pub struct MPD {
    #[serde(rename = "type")]
    pub _type: Option<String>,
    #[serde(rename = "maxSegmentDuration")]
    pub max_segment_duration: Option<String>,
    #[serde(rename = "availabilityStartTime")]
    pub availability_start_time: Option<String>,
    #[serde(rename = "timeShiftBufferDepth")]
    pub time_shift_buffer_depth: Option<String>,
    #[serde(rename = "publishTime")]
    pub publish_time: Option<String>,
    #[serde(rename = "mediaPresentationDuration")]
    pub media_presentation_duration: Option<String>,
    #[serde(rename = "BaseURL")]
    pub baseurl: Option<String>,
    #[serde(rename = "Period", default)]
    pub period: Vec<Period>,
}

#[derive(Debug, Default, Deserialize)]
pub struct Period {
    pub id: Option<String>,
    pub duration: Option<String>,
    #[serde(rename = "BaseURL")]
    pub baseurl: Option<String>,
    #[serde(rename = "AdaptationSet", default)]
    pub adaptation_set: Vec<AdaptationSet>,
}

#[derive(Debug, Default, Deserialize)]
pub struct AdaptationSet {
    #[serde(rename = "mimeType")]
    pub mime_type: Option<String>,
    #[serde(rename = "contentType")]
    pub content_type: Option<String>,
    pub codecs: Option<String>,
    pub lang: Option<String>,
    #[serde(rename = "frameRate")]
    pub frame_rate: Option<String>,
    #[serde(rename = "AudioChannelConfiguration")]
    pub audio_channel_configuration: Option<AudioChannelConfiguration>,
    #[serde(rename = "BaseURL")]
    pub baseurl: Option<String>,
    #[serde(rename = "SegmentTemplate")]
    pub segment_template: Option<SegmentTemplate>,
    #[serde(rename = "ContentProtection", default)]
    pub content_protection: Vec<ContentProtection>,
    #[serde(rename = "Representation", default)]
    pub representation: Vec<Representation>,
}

#[derive(Debug, Default, Deserialize)]
pub struct Representation {
    pub id: Option<String>,
    #[serde(rename = "mimeType")]
    pub mime_type: Option<String>,
    #[serde(rename = "contentType")]
    pub content_type: Option<String>,
    pub codecs: Option<String>,
    #[serde(rename = "frameRate")]
    pub frame_rate: Option<String>,
    pub bandwidth: Option<u64>,
    pub lang: Option<String>,
    pub width: Option<u64>,
    pub height: Option<u64>,
    #[serde(rename = "AudioChannelConfiguration")]
    pub audio_channel_configuration: Option<AudioChannelConfiguration>,
    #[serde(rename = "Role")]
    pub role: Option<Role>,
    #[serde(rename = "BaseURL")]
    pub baseurl: Option<String>,
    #[serde(rename = "SegmentBase")]
    pub segment_base: Option<SegmentBase>,
    #[serde(rename = "SegmentList")]
    pub segment_list: Option<SegmentList>,
    #[serde(rename = "SegmentTemplate")]
    pub segment_template: Option<SegmentTemplate>,
    #[serde(rename = "ContentProtection", default)]
    pub content_protection: Vec<ContentProtection>,
}

#[derive(Debug, Default, Deserialize)]
pub struct AudioChannelConfiguration {
    pub value: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct Role {
    pub value: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct SegmentBase {
    #[serde(rename = "Initialization")]
    pub initialization: Option<Initialization>,
}

#[derive(Debug, Default, Deserialize)]
pub struct Initialization {
    #[serde(rename = "sourceURL")]
    pub source_url: Option<String>,
    pub range: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct SegmentList {
    pub duration: Option<String>,
    pub timescale: Option<String>,
    #[serde(rename = "Initialization")]
    pub initialization: Option<Initialization>,
    #[serde(rename = "SegmentURL", default)]
    pub segment_urls: Vec<SegmentURL>,
}

#[derive(Debug, Default, Deserialize)]
pub struct SegmentURL {
    pub media: Option<String>,
    #[serde(rename = "mediaRange")]
    pub media_range: Option<String>,
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct SegmentTemplate {
    pub media: Option<String>,
    pub duration: Option<String>,
    pub timescale: Option<String>,
    #[serde(rename = "startNumber")]
    pub start_number: Option<usize>,
    pub initialization: Option<String>,
    #[serde(rename = "SegmentTimeline")]
    pub segment_timeline: Option<SegmentTimeline>,
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct SegmentTimeline {
    #[serde(rename = "S", default)]
    pub s: Vec<S>,
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct S {
    pub t: Option<i64>,
    pub d: i64,
    pub r: Option<i64>,
}

#[derive(Debug, Default, Deserialize)]
pub struct ContentProtection {
    #[serde(rename = "cenc:default_KID")]
    pub default_kid: Option<String>,
}

impl MPD {
    pub fn live(&self) -> bool {
        if let Some(_type) = &self._type {
            if _type == "dynamic" {
                return true;
            }
        }

        false
    }
}

impl Period {
    pub(super) fn duration(&self, mpd: &MPD) -> f32 {
        if let Some(duration) = &self.duration {
            utils::iso8601_duration_to_seconds(&duration).unwrap()
        } else if let Some(duration) = &mpd.media_presentation_duration {
            utils::iso8601_duration_to_seconds(&duration).unwrap()
        } else {
            0.0
        }
    }
}

impl AdaptationSet {
    pub(super) fn mime_type(&self) -> Option<String> {
        if let Some(content_type) = &self.content_type {
            Some(content_type.to_owned())
        } else if let Some(mime_type) = &self.mime_type {
            Some(mime_type.to_owned())
        } else {
            None
        }
    }

    pub(super) fn frame_rate(&self) -> Option<f64> {
        if let Some(frame_rate) = &self.frame_rate {
            if frame_rate.contains("/") {
                return Some(
                    frame_rate
                        .split('/')
                        .nth(0)
                        .unwrap()
                        .parse::<f64>()
                        .unwrap()
                        / frame_rate
                            .split('/')
                            .nth(1)
                            .unwrap()
                            .parse::<f64>()
                            .unwrap(),
                );
            } else {
                return frame_rate.parse::<f64>().ok();
            }
        }

        None
    }

    pub(super) fn channels(&self) -> Option<String> {
        if let Some(audio_channel_configuration) = &self.audio_channel_configuration {
            if audio_channel_configuration.value.is_some() {
                return audio_channel_configuration.value.clone();
            }
        }

        None
    }

    pub(super) fn encryption_type(&self) -> Option<String> {
        for content_protection in &self.content_protection {
            if content_protection.default_kid.is_some() {
                return Some("CENC".to_string());
            }
        }

        if self.content_protection.len() != 0 {
            return Some("UNKNOWN".to_string());
        }

        None
    }

    pub(super) fn default_kid(&self) -> Option<String> {
        for content_protection in &self.content_protection {
            if content_protection.default_kid.is_some() {
                return content_protection.default_kid.clone();
            }
        }

        None
    }
}

impl Representation {
    fn get_mime_type(&self) -> Option<String> {
        if let Some(content_type) = &self.content_type {
            Some(content_type.to_owned())
        } else if let Some(mime_type) = &self.mime_type {
            Some(mime_type.to_owned())
        } else {
            None
        }
    }

    pub(super) fn media_type(&self, adaptation_set: &AdaptationSet) -> m3u8_rs::AlternativeMediaType {
        let mime_type = if let Some(mime_type) = adaptation_set.mime_type() {
            mime_type
        } else {
            self.get_mime_type().unwrap_or("".to_owned())
        };

        let codecs = self.codecs(adaptation_set).unwrap_or("".to_owned());
        if codecs == "stpp" || codecs == "wvtt" {
            return m3u8_rs::AlternativeMediaType::Subtitles;
        }

        if let Some(role) = &self.role {
            if let Some(value) = &role.value {
                if value == "subtitle" {
                    return m3u8_rs::AlternativeMediaType::Subtitles;
                }
            }
        }

        match mime_type.split('/').next().unwrap() {
            "video" => m3u8_rs::AlternativeMediaType::Video,
            "audio" => m3u8_rs::AlternativeMediaType::Audio,
            "text" => m3u8_rs::AlternativeMediaType::Subtitles,
            x => m3u8_rs::AlternativeMediaType::Other(x.to_owned()),
        }
    }

    pub(super) fn extension(&self, adaptation_set: &AdaptationSet) -> Option<String> {
        let mime_type = if let Some(mime_type) = adaptation_set.mime_type() {
            mime_type
        } else {
            self.get_mime_type().unwrap_or("".to_owned())
        };

        mime_type.split('/').nth(1).map(|x| x.to_owned())
    }

    pub(super) fn codecs(&self, adaptation_set: &AdaptationSet) -> Option<String> {
        if self.codecs.is_some() {
            self.codecs.clone()
        } else if adaptation_set.codecs.is_some() {
            adaptation_set.codecs.clone()
        } else {
            None
        }
    }

    pub(super) fn lang(&self, adaptation_set: &AdaptationSet) -> Option<String> {
        if self.lang.is_some() {
            self.lang.clone()
        } else if adaptation_set.lang.is_some() {
            adaptation_set.lang.clone()
        } else {
            None
        }
    }

    pub(super) fn frame_rate(&self, adaptation_set: &AdaptationSet) -> Option<f64> {
        if let Some(frame_rate) = &self.frame_rate {
            if frame_rate.contains("/") {
                return Some(
                    frame_rate
                        .split('/')
                        .nth(0)
                        .unwrap()
                        .parse::<f64>()
                        .unwrap()
                        / frame_rate
                            .split('/')
                            .nth(1)
                            .unwrap()
                            .parse::<f64>()
                            .unwrap(),
                );
            } else {
                return frame_rate.parse::<f64>().ok();
            }
        }

        adaptation_set.frame_rate()
    }

    pub(super) fn channels(&self, adaptation_set: &AdaptationSet) -> Option<String> {
        if let Some(audio_channel_configuration) = &self.audio_channel_configuration {
            if audio_channel_configuration.value.is_some() {
                return audio_channel_configuration.value.clone();
            }
        }

        adaptation_set.channels()
    }

    pub(super) fn encryption_type(&self, adaptation_set: &AdaptationSet) -> Option<String> {
        for content_protection in &self.content_protection {
            if content_protection.default_kid.is_some() {
                return Some("CENC".to_string());
            }
        }

        adaptation_set.encryption_type()
    }

    pub(super) fn default_kid(&self, adaptation_set: &AdaptationSet) -> Option<String> {
        for content_protection in &self.content_protection {
            if content_protection.default_kid.is_some() {
                return content_protection.default_kid.clone();
            }
        }

        adaptation_set.default_kid()
    }

    pub(super) fn template_vars(&self) -> HashMap<String, String> {
        let mut vars = HashMap::new();

        vars.insert("RepresentationID".to_owned(), self.id.clone().unwrap_or("".to_owned()));

        if let Some(bandwidth) = &self.bandwidth {
            vars.insert("Bandwidth".to_owned(), bandwidth.to_string());
        } else {
            vars.insert("Bandwidth".to_owned(), "".to_owned());
        }

        vars
    }

    pub(super) fn segment_template(&self, adaptation_set: &AdaptationSet) -> Option<SegmentTemplate> {
        if let Some(segment_template) = &self.segment_template {
            Some(segment_template.to_owned())
        } else if let Some(segment_template) = &adaptation_set.segment_template {
            Some(segment_template.to_owned())
        } else {
            None
        }
    }
}

impl SegmentList {
    pub(super) fn segment_duration(&self) -> f32 {
        self.duration
            .as_ref()
            .map(|x| x.parse::<f32>().unwrap())
            .unwrap_or(1.0)
            / self
                .timescale
                .as_ref()
                .map(|x| x.parse::<f32>().unwrap())
                .unwrap_or(1.0)
    }
}

impl SegmentTemplate {
    pub(super) fn timescale(&self) -> f32 {
        self.timescale
            .as_ref()
            .map(|x| x.parse::<f32>().unwrap())
            .unwrap_or(1.0)
    }

    pub(super) fn duration(&self) -> f32 {
        self.duration
            .as_ref()
            .map(|x| x.parse::<f32>().unwrap())
            .unwrap_or(1.0)
    }

    pub(super) fn start_number(&self) -> usize {
        self.start_number.unwrap_or(0)
    }
}
