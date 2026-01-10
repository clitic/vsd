use std::collections::HashSet;

pub struct Prompter {
    pub interactive: bool,
    pub interactive_raw: bool,
}

#[derive(Debug)]
pub struct SelectOptions {
    pub audio: AudioSubs,
    pub stream_numbers: HashSet<usize>,
    pub subs: AudioSubs,
    pub video: Video,
}

#[derive(Debug)]
pub struct Video {
    pub all: bool,
    pub preference: VideoPreference,
    pub resolutions: HashSet<(u16, u16)>,
    pub skip: bool,
}

#[derive(Debug)]
pub enum VideoPreference {
    Best,
    None,
    Worst,
}

#[derive(Debug)]
pub struct AudioSubs {
    pub all: bool,
    pub languages: HashSet<String>,
    pub skip: bool,
}

impl SelectOptions {
    pub fn parse(data: &str) -> SelectOptions {
        let mut auto = SelectOptions {
            audio: AudioSubs {
                all: false,
                languages: HashSet::new(),
                skip: false,
            },
            stream_numbers: HashSet::new(),
            subs: AudioSubs {
                all: false,
                languages: HashSet::new(),
                skip: false,
            },
            video: Video {
                all: false,
                preference: VideoPreference::None,
                resolutions: HashSet::new(),
                skip: false,
            },
        };

        for stream in data.split_terminator(':') {
            if let Some((_type, queries)) = stream.split_once('=') {
                match _type {
                    "v" => {
                        for query in queries.split_terminator(',') {
                            if let Ok(stream_number) = query.parse::<usize>() {
                                auto.stream_numbers.insert(stream_number);
                            }

                            match query {
                                "all" => auto.video.all = true,
                                "skip" => auto.video.skip = true,
                                // resolutions
                                "best" | "high" | "max" => {
                                    auto.video.preference = VideoPreference::Best;
                                }
                                "low" | "min" | "worst" => {
                                    auto.video.preference = VideoPreference::Worst;
                                }
                                "144p" => {
                                    let _ = auto.video.resolutions.insert((256, 144));
                                }
                                "240p" => {
                                    let _ = auto.video.resolutions.insert((426, 240));
                                }
                                "360p" => {
                                    let _ = auto.video.resolutions.insert((640, 360));
                                }
                                "480p" => {
                                    let _ = auto.video.resolutions.insert((854, 480));
                                }
                                "720p" | "hd" => {
                                    let _ = auto.video.resolutions.insert((1280, 720));
                                }
                                "1080p" | "fhd" => {
                                    let _ = auto.video.resolutions.insert((1920, 1080));
                                }
                                "2k" => {
                                    let _ = auto.video.resolutions.insert((2048, 1080));
                                }
                                "1440p" | "qhd" => {
                                    let _ = auto.video.resolutions.insert((2560, 1440));
                                }
                                "4k" => {
                                    let _ = auto.video.resolutions.insert((3840, 2160));
                                }
                                "8k" => {
                                    let _ = auto.video.resolutions.insert((7680, 4320));
                                }
                                x => {
                                    if let Some((w, h)) = x.split_once('x')
                                        && let (Ok(w), Ok(h)) = (w.parse::<u16>(), h.parse::<u16>())
                                    {
                                        auto.video.resolutions.insert((w, h));
                                    }
                                }
                            }
                        }
                    }
                    "a" => {
                        for query in queries.split_terminator(',') {
                            if let Ok(stream_number) = query.parse::<usize>() {
                                auto.stream_numbers.insert(stream_number);
                            }

                            match query {
                                "all" => auto.audio.all = true,
                                "skip" => auto.audio.skip = true,
                                x => {
                                    auto.audio.languages.insert(x.to_owned());
                                }
                            }
                        }
                    }
                    "s" => {
                        for query in queries.split_terminator(',') {
                            if let Ok(stream_number) = query.parse::<usize>() {
                                auto.stream_numbers.insert(stream_number);
                            }

                            match query {
                                "all" => auto.subs.all = true,
                                "skip" => auto.subs.skip = true,
                                x => {
                                    auto.subs.languages.insert(x.to_owned());
                                }
                            }
                        }
                    }
                    _ => (),
                }
            }
        }

        auto
    }
}

impl AudioSubs {
    pub fn contains_exact_lang(&mut self, s_lang: &str) -> bool {
        let languages = self.languages.clone();

        for lang in &languages {
            if lang == s_lang {
                self.languages.remove(lang);
                return true;
            }
        }

        false
    }

    pub fn contains_siml_lang(&mut self, s_lang: &str) -> bool {
        let languages = self.languages.clone();

        for lang in &languages {
            if lang.to_lowercase().get(0..2) == s_lang.to_lowercase().get(0..2) {
                self.languages.remove(lang);
                return true;
            }
        }

        false
    }
}
