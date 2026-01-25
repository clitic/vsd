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

pub fn set_interaction_type(itype: InteractionType) {
    INTERACTION_TYPE.store(itype as u8, Ordering::SeqCst);
}

pub fn get_interaction_type() -> InteractionType {
    match INTERACTION_TYPE.load(Ordering::SeqCst) {
        0 => InteractionType::Modern,
        1 => InteractionType::None,
        2 => InteractionType::Raw,
        _ => unreachable!(),
    }
}

#[derive(Debug, Default)]
pub struct SelectOptions {
    pub vid: Preferences,
    pub aud: Preferences,
    pub sub: Preferences,
    pub stream_indices: HashSet<usize>,
    pub strict_indices: bool,
}

#[derive(Debug, Default)]
pub struct Preferences {
    pub all: bool,
    pub skip: bool,
    pub languages: HashSet<String>,
    pub resolutions: HashSet<(u16, u16)>,
    pub quality: Quality,
}

#[derive(Debug, Default)]
pub enum Quality {
    Best,
    #[default]
    None,
    Worst,
}

impl std::str::FromStr for SelectOptions {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut opts = Self::default();

        if !s.contains(':') || s.contains(',') {
            s.split(',')
                .filter_map(|x| x.trim().parse::<usize>().ok())
                .filter_map(|x| x.checked_sub(1))
                .for_each(|x| {
                    let _ = opts.stream_indices.insert(x);
                });
            opts.strict_indices = true;
            return Ok(opts);
        }

        for stream in s.split_terminator(':') {
            if let Some((code, queries)) = stream.split_once('=') {
                match code {
                    "v" => {
                        for query in queries.split_terminator(',').map(|x| x.trim()) {
                            if let Some(idx) = query
                                .parse::<usize>()
                                .ok()
                                .map(|x| x.checked_sub(1))
                                .flatten()
                            {
                                let _ = opts.stream_indices.insert(idx);
                            }

                            match query {
                                "all" => opts.vid.all = true,
                                "skip" => opts.vid.skip = true,
                                "best" | "high" | "max" => {
                                    opts.vid.quality = Quality::Best;
                                }
                                "low" | "min" | "worst" => {
                                    opts.vid.quality = Quality::Worst;
                                }
                                "144p" => {
                                    let _ = opts.vid.resolutions.insert((256, 144));
                                }
                                "240p" => {
                                    let _ = opts.vid.resolutions.insert((426, 240));
                                }
                                "360p" => {
                                    let _ = opts.vid.resolutions.insert((640, 360));
                                }
                                "480p" => {
                                    let _ = opts.vid.resolutions.insert((854, 480));
                                }
                                "720p" | "hd" => {
                                    let _ = opts.vid.resolutions.insert((1280, 720));
                                }
                                "1080p" | "fhd" => {
                                    let _ = opts.vid.resolutions.insert((1920, 1080));
                                }
                                "2k" => {
                                    let _ = opts.vid.resolutions.insert((2048, 1080));
                                }
                                "1440p" | "qhd" => {
                                    let _ = opts.vid.resolutions.insert((2560, 1440));
                                }
                                "4k" => {
                                    let _ = opts.vid.resolutions.insert((3840, 2160));
                                }
                                "8k" => {
                                    let _ = opts.vid.resolutions.insert((7680, 4320));
                                }
                                x if x.contains('x') => {
                                    if let Some((w, h)) = x.split_once('x')
                                        && let (Ok(w), Ok(h)) = (w.parse::<u16>(), h.parse::<u16>())
                                    {
                                        opts.vid.resolutions.insert((w, h));
                                    }
                                }
                                _ => (),
                            }
                        }
                    }
                    "a" => {
                        for query in queries.split_terminator(',').map(|x| x.trim()) {
                            if let Some(stream_number) = query
                                .parse::<usize>()
                                .ok()
                                .map(|x| x.checked_sub(1))
                                .flatten()
                            {
                                let _ = opts.stream_indices.insert(stream_number);
                            }

                            match query {
                                "all" => opts.aud.all = true,
                                "skip" => opts.aud.skip = true,
                                x => {
                                    opts.aud.languages.insert(x.to_owned());
                                }
                            }
                        }
                    }
                    "s" => {
                        for query in queries.split_terminator(',').map(|x| x.trim()) {
                            if let Some(stream_number) = query
                                .parse::<usize>()
                                .ok()
                                .map(|x| x.checked_sub(1))
                                .flatten()
                            {
                                let _ = opts.stream_indices.insert(stream_number);
                            }

                            match query {
                                "all" => opts.sub.all = true,
                                "skip" => opts.sub.skip = true,
                                x => {
                                    opts.sub.languages.insert(x.to_owned());
                                }
                            }
                        }
                    }
                    _ => (),
                }
            }
        }

        Ok(opts)
    }
}

impl Preferences {
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
