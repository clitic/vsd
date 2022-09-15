use super::mp4parser;
use super::MP4Parser;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub(super) struct Cue {
    pub(super) start_time: f32,
    pub(super) end_time: f32,
    pub(super) payload: String,
    pub(super) settings: String,
}

impl Cue {
    pub(super) fn new(payload: &str, start_time: f32, end_time: f32) -> Self {
        Self {
            start_time,
            end_time,
            payload: payload.to_owned(),
            settings: "".to_owned(),
        }
    }

    pub(super) fn parse_vttc(data: &[u8], start_time: f32, end_time: f32) -> Option<Self> {
        let payload = Arc::new(Mutex::new(String::new()));
        let id = Arc::new(Mutex::new(String::new()));
        let settings = Arc::new(Mutex::new(String::new()));

        let payload_c = payload.clone();
        let id_c = id.clone();
        let settings_c = settings.clone();

        MP4Parser::default()
            .basic(
                "payl",
                mp4parser::alldata(Arc::new(move |data| {
                    *payload_c.lock().unwrap() = String::from_utf8(data).unwrap();
                    Ok(())
                })),
            )
            .basic(
                "iden",
                mp4parser::alldata(Arc::new(move |data| {
                    *id_c.lock().unwrap() = String::from_utf8(data).unwrap();
                    Ok(())
                })),
            )
            .basic(
                "sttg",
                mp4parser::alldata(Arc::new(move |data| {
                    *settings_c.lock().unwrap() = String::from_utf8(data).unwrap();
                    Ok(())
                })),
            )
            .parse(data, None, None)
            .unwrap();

        let payload = payload.lock().unwrap().to_owned();
        // let id = id.lock().unwrap().to_owned();

        if !payload.is_empty() {
            return Some(Self {
                start_time,
                end_time,
                payload,
                settings: settings.lock().unwrap().to_owned(),
            });
        }

        None
    }
}

pub struct Subtitles {
    cues: Vec<Cue>,
}

impl Subtitles {
    pub(super) fn new(cues: Vec<Cue>) -> Self {
        Self {
            cues: cues
                .iter()
                .filter(|x| !(x.payload == "" || (x.start_time == x.end_time)))
                .map(|x| x.to_owned())
                .collect(),
        }
    }

    pub(super) fn _merge(&self) -> Self {
        let mut cues = vec![];
        let mut prev_cue: Option<Cue> = None;

        for cue in &self.cues {
            let mut pushed = false;

            if let Some(old_cue) = &mut prev_cue {
                // println!("{} {} {}", cue.start_time == old_cue.end_time, cue.start_time, cue.end_time);
                if cue.start_time == old_cue.end_time {
                    old_cue.payload += "\n";
                    old_cue.payload += &cue.payload;
                    old_cue.end_time = cue.end_time;
                } else {
                    cues.push(old_cue.clone());
                    pushed = true;
                }
            }

            if pushed {
                prev_cue = None;
            } else {
                prev_cue = Some(cue.clone());
            }
        }

        Self { cues }
    }

    pub fn to_vtt(&self) -> String {
        let mut subtitles = "WEBVTT\n\n".to_owned();

        for cue in &self.cues {
            subtitles.push_str(&format!(
                "{} --> {} {}\n{}\n\n",
                seconds_to_timestamp(cue.start_time, "."),
                seconds_to_timestamp(cue.end_time, "."),
                cue.settings,
                cue.payload
            ))
        }

        subtitles
    }

    pub fn to_srt(&self) -> String {
        let mut subtitles = String::new();

        for (i, cue) in self.cues.iter().enumerate() {
            subtitles.push_str(&format!(
                "{}\n{} --> {}\n{}\n\n",
                i + 1,
                seconds_to_timestamp(cue.start_time, ","),
                seconds_to_timestamp(cue.end_time, ","),
                cue.payload
            ))
        }

        subtitles
    }
}

fn seconds_to_timestamp(seconds: f32, millisecond_sep: &str) -> String {
    let divmod = |x: usize, y: usize| (x / y as usize, x % y);

    let (seconds, milliseconds) = divmod((seconds * 1000.0) as usize, 1000);
    let (minutes, seconds) = divmod(seconds, 60);
    let (hours, minutes) = divmod(minutes, 60);
    format!(
        "{:02}:{:02}:{:02}{}{:03}",
        hours, minutes, seconds, millisecond_sep, milliseconds
    )
}
