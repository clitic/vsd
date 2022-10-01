use std::collections::HashMap;

type OtherAttributes = Option<HashMap<String, m3u8_rs::QuotedOrUnquoted>>;
type ExtTags = Vec<m3u8_rs::ExtTag>;

#[derive(Debug, Default)]
pub struct PlaylistTag {
    pub codecs: Option<String>,
    pub bandwidth: Option<usize>,
    pub extension: Option<String>,
    pub vtt: bool,
    pub ttml: bool,
}

impl PlaylistTag {
    pub fn codecs(mut self, codecs: Option<String>) -> Self {
        self.codecs = codecs;
        self
    }

    pub fn bandwidth(mut self, bandwidth: Option<usize>) -> Self {
        self.bandwidth = bandwidth;
        self
    }

    pub fn extension(mut self, extension: Option<String>) -> Self {
        self.extension = extension;
        self
    }

    pub fn build(self) -> Self {
        self
    }
}

impl From<&OtherAttributes> for PlaylistTag {
    fn from(tags: &OtherAttributes) -> Self {
        let mut mpd_tags = Self::default();

        if let Some(tags) = tags {
            for (k, v) in tags.iter().map(|(x, y)| {
                (
                    x,
                    match y {
                        m3u8_rs::QuotedOrUnquoted::Unquoted(value) => value,
                        m3u8_rs::QuotedOrUnquoted::Quoted(value) => value,
                    },
                )
            }) {
                match k.as_str() {
                    "CODECS" => {
                        mpd_tags.codecs = Some(v.to_owned());

                        if v == "wvtt" {
                            mpd_tags.vtt = true;
                        } else if v == "stpp" {
                            mpd_tags.ttml = true;
                        }
                    }
                    "BANDWIDTH" => mpd_tags.bandwidth = Some(v.parse::<usize>().unwrap()),
                    "EXTENSION" => mpd_tags.extension = Some(v.to_owned()),
                    _ => (),
                }
            }
        }

        mpd_tags
    }
}

impl Into<OtherAttributes> for PlaylistTag {
    fn into(self) -> OtherAttributes {
        let mut m3u8_tags = HashMap::new();

        if let Some(codecs) = &self.codecs {
            m3u8_tags.insert(
                "CODECS".to_owned(),
                m3u8_rs::QuotedOrUnquoted::Quoted(codecs.to_owned()),
            );
        }

        if let Some(bandwidth) = &self.bandwidth {
            m3u8_tags.insert(
                "BANDWIDTH".to_owned(),
                m3u8_rs::QuotedOrUnquoted::Unquoted(bandwidth.to_string()),
            );
        }

        if let Some(extension) = &self.extension {
            m3u8_tags.insert(
                "EXTENSION".to_owned(),
                m3u8_rs::QuotedOrUnquoted::Quoted(extension.to_owned()),
            );
        }

        if !m3u8_tags.is_empty() {
            Some(m3u8_tags)
        } else {
            None
        }
    }
}

impl From<&ExtTags> for PlaylistTag {
    fn from(tags: &ExtTags) -> Self {
        let mut mpd_tags = HashMap::new();

        for (k, v) in tags.iter().filter_map(|x| {
            x.rest.as_ref().map(|rest| (x.tag.to_owned(), m3u8_rs::QuotedOrUnquoted::Quoted(rest.to_owned())))
        }) {
            mpd_tags.insert(k, v);
        }

        if !mpd_tags.is_empty() {
            Self::from(&Some(mpd_tags))
        } else {
            Self::default()
        }
    }
}

impl Into<ExtTags> for PlaylistTag {
    fn into(self) -> ExtTags {
        let other_attributes: OtherAttributes = self.into();
        let mut m3u8_tags = vec![];

        if let Some(other_attributes) = other_attributes {
            for (k, v) in other_attributes.iter().map(|(x, y)| {
                (
                    x,
                    match y {
                        m3u8_rs::QuotedOrUnquoted::Unquoted(value) => value,
                        m3u8_rs::QuotedOrUnquoted::Quoted(value) => value,
                    },
                )
            }) {
                m3u8_tags.push(m3u8_rs::ExtTag {
                    tag: k.to_owned(),
                    rest: Some(v.to_owned()),
                });
            }
        }

        m3u8_tags
    }
}

#[derive(Debug, Default)]
pub struct SegmentTag {
    pub init: bool,
    pub single: bool,
    pub kid: Option<String>,
}

impl SegmentTag {
    pub fn init(mut self, init: bool) -> Self {
        self.init = init;
        self
    }

    pub fn single(mut self, single: bool) -> Self {
        self.single = single;
        self
    }

    pub fn kid(mut self, kid: Option<String>) -> Self {
        self.kid = kid;
        self
    }

    pub fn build(self) -> Self {
        self
    }
}

impl From<&ExtTags> for SegmentTag {
    fn from(tags: &ExtTags) -> Self {
        let mut mpd_tags = Self::default();

        for tag in tags {
            match tag.tag.as_str() {
                "DASH-INIT" => mpd_tags.init = true,
                "DASH-SINGLE" => mpd_tags.single = true,
                "DASH-KID" => mpd_tags.kid = tag.rest.clone(),
                _ => (),
            }
        }

        mpd_tags
    }
}

impl Into<ExtTags> for SegmentTag {
    fn into(self) -> ExtTags {
        let mut m3u8_tags = vec![];

        if self.init {
            m3u8_tags.push(m3u8_rs::ExtTag {
                tag: "DASH-INIT".to_owned(),
                rest: None,
            });
        }

        if self.single {
            m3u8_tags.push(m3u8_rs::ExtTag {
                tag: "DASH-SINGLE".to_owned(),
                rest: None,
            });
        }

        if self.kid.is_some() {
            m3u8_tags.push(m3u8_rs::ExtTag {
                tag: "DASH-KID".to_owned(),
                rest: self.kid,
            });
        }

        m3u8_tags
    }
}
