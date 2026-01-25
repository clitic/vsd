use std::{
    collections::HashSet,
    sync::atomic::{AtomicU8, Ordering},
};

static INTERACTION_TYPE: AtomicU8 = AtomicU8::new(InteractionType::None as u8);

pub enum InteractionType {
    Modern,
    None,
    Raw,
}

pub fn set_interaction_type(_type: InteractionType) {
    INTERACTION_TYPE.store(_type as u8, Ordering::SeqCst);
}

pub fn load_interaction_type() -> InteractionType {
    match INTERACTION_TYPE.load(Ordering::SeqCst) {
        0 => InteractionType::Modern,
        1 => InteractionType::None,
        2 => InteractionType::Raw,
        _ => unreachable!(),
    }
}

#[derive(Debug)]
pub struct SelectOptions {
    pub vid: Video,
    pub aud: AudioSubs,
    pub sub: AudioSubs,
    pub indices: HashSet<usize>,
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

impl std::str::FromStr for SelectOptions {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut auto = SelectOptions {
            vid: Video {
                all: false,
                preference: VideoPreference::None,
                resolutions: HashSet::new(),
                skip: false,
            },
            aud: AudioSubs {
                all: false,
                languages: HashSet::new(),
                skip: false,
            },
            sub: AudioSubs {
                all: false,
                languages: HashSet::new(),
                skip: false,
            },
            indices: HashSet::new(),
        };

        if !s.contains(':') || s.contains(',') {
            s.split(',')
                .filter_map(|x| x.trim().parse::<usize>().ok())
                .for_each(|x| {
                    let _ = auto.indices.insert(x);
                });
        }

        for stream in s.split_terminator(':') {
            if let Some((_type, queries)) = stream.split_once('=') {
                match _type {
                    "v" => {
                        for query in queries.split_terminator(',') {
                            if let Ok(stream_number) = query.parse::<usize>() {
                                auto.indices.insert(stream_number);
                            }

                            match query {
                                "all" => auto.vid.all = true,
                                "skip" => auto.vid.skip = true,
                                // resolutions
                                "best" | "high" | "max" => {
                                    auto.vid.preference = VideoPreference::Best;
                                }
                                "low" | "min" | "worst" => {
                                    auto.vid.preference = VideoPreference::Worst;
                                }
                                "144p" => {
                                    let _ = auto.vid.resolutions.insert((256, 144));
                                }
                                "240p" => {
                                    let _ = auto.vid.resolutions.insert((426, 240));
                                }
                                "360p" => {
                                    let _ = auto.vid.resolutions.insert((640, 360));
                                }
                                "480p" => {
                                    let _ = auto.vid.resolutions.insert((854, 480));
                                }
                                "720p" | "hd" => {
                                    let _ = auto.vid.resolutions.insert((1280, 720));
                                }
                                "1080p" | "fhd" => {
                                    let _ = auto.vid.resolutions.insert((1920, 1080));
                                }
                                "2k" => {
                                    let _ = auto.vid.resolutions.insert((2048, 1080));
                                }
                                "1440p" | "qhd" => {
                                    let _ = auto.vid.resolutions.insert((2560, 1440));
                                }
                                "4k" => {
                                    let _ = auto.vid.resolutions.insert((3840, 2160));
                                }
                                "8k" => {
                                    let _ = auto.vid.resolutions.insert((7680, 4320));
                                }
                                x => {
                                    if let Some((w, h)) = x.split_once('x')
                                        && let (Ok(w), Ok(h)) = (w.parse::<u16>(), h.parse::<u16>())
                                    {
                                        auto.vid.resolutions.insert((w, h));
                                    }
                                }
                            }
                        }
                    }
                    "a" => {
                        for query in queries.split_terminator(',') {
                            if let Ok(stream_number) = query.parse::<usize>() {
                                auto.indices.insert(stream_number);
                            }

                            match query {
                                "all" => auto.aud.all = true,
                                "skip" => auto.aud.skip = true,
                                x => {
                                    auto.aud.languages.insert(x.to_owned());
                                }
                            }
                        }
                    }
                    "s" => {
                        for query in queries.split_terminator(',') {
                            if let Ok(stream_number) = query.parse::<usize>() {
                                auto.indices.insert(stream_number);
                            }

                            match query {
                                "all" => auto.sub.all = true,
                                "skip" => auto.sub.skip = true,
                                x => {
                                    auto.sub.languages.insert(x.to_owned());
                                }
                            }
                        }
                    }
                    _ => (),
                }
            }
        }

        Ok(auto)
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
