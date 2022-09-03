#[derive(Debug, Clone, Default)]
pub struct MPDMediaSegmentTag {
    pub init: bool,
    pub single: bool,
    pub kid: Option<String>,
}

impl MPDMediaSegmentTag {
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

impl From<Vec<m3u8_rs::ExtTag>> for MPDMediaSegmentTag {
    fn from(tags: Vec<m3u8_rs::ExtTag>) -> Self {
        let mut mpd_tags = Self::default();

        for tag in tags {
            if tag.tag == "DASH-INIT" {
                mpd_tags.init = true;
            } else if tag.tag == "DASH-SINGLE" {
                mpd_tags.single = true;
            } else if tag.tag == "DASH-KID" {
                mpd_tags.kid = tag.rest;
            }
        }

        mpd_tags
    }
}

impl Into<Vec<m3u8_rs::ExtTag>> for MPDMediaSegmentTag {
    fn into(self) -> Vec<m3u8_rs::ExtTag> {
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
                rest: self.kid.clone(),
            });
        }

        m3u8_tags
    }
}
