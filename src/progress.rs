use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamData {
    pub url: String,
    pub file: String,
    pub downloaded: usize,
    pub total: usize,
}

impl StreamData {
    pub fn new(url: &str, file: &str) -> Self {
        Self {
            url: url.to_owned(),
            file: file.to_owned(),
            downloaded: 0,
            total: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgress {
    pub json_file: String,
    pub current: String,
    pub stream: StreamData,
    pub audio: Option<StreamData>,
    pub subtitle: Option<StreamData>,
}

impl DownloadProgress {
    pub fn new_empty() -> Self {
        Self {
            json_file: "".to_owned(),
            current: "stream".to_owned(),
            stream: StreamData::new("", ""),
            audio: None,
            subtitle: None,
        }
    }

    pub fn file(&mut self, file: &str) {
        self.json_file = file.to_owned();
    }

    pub fn current(&mut self, current: &str) {
        self.current = current.to_owned();
    }

    pub fn update(&mut self, pos: usize, total: usize) {
        match self.current.as_str() {
            "stream" => {
                self.stream.downloaded = pos;
                self.stream.total = total;
            }

            "audio" => {
                if let Some(audio) = &mut self.audio {
                    audio.downloaded = pos;
                    audio.total = total;
                }
            }

            "subtitle" => {
                if let Some(subtitle) = &mut self.subtitle {
                    subtitle.downloaded = pos;
                    subtitle.total = total;
                }
            }

            _ => (),
        }

        serde_json::to_writer_pretty(std::fs::File::create(&self.json_file).unwrap(), self)
            .unwrap();
    }

    pub fn downloaded(&self) -> usize {
        return match self.current.as_str() {
            "stream" => self.stream.downloaded,

            "audio" => {
                if let Some(audio) = &self.audio {
                    audio.downloaded
                } else {
                    0
                }
            }

            "subtitle" => {
                if let Some(subtitle) = &self.subtitle {
                    subtitle.downloaded
                } else {
                    0
                }
            }

            _ => 0,
        };
    }
}
