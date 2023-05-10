/*
    REFERENCES
    ----------

    1. https://github.com/shaka-project/shaka-player/blob/9ce2f675d88d5de6f779f2a62a4f4af2bcc14611/lib/text/cue.js
    2. https://w3c.github.io/webvtt
    3. https://developer.mozilla.org/en-US/docs/Web/API/WebVTT_API

*/

use std::fmt::Write;

#[allow(dead_code)]
#[derive(Clone)]
pub struct Cue {
    pub(super) end_time: f32,
    pub(super) id: String,
    pub(super) payload: String,
    pub(super) settings: String,
    pub(super) start_time: f32,
}

pub struct Subtitles {
    cues: Vec<Cue>,
}

impl Subtitles {
    pub fn new(cues: Vec<Cue>) -> Self {
        let mut trimmed_cues: Vec<Cue> = vec![];

        for current_cue in cues {
            if !(current_cue.payload.is_empty() || (current_cue.start_time == current_cue.end_time))
            {
                if let Some(last_cue) = trimmed_cues.last() {
                    if last_cue.end_time == current_cue.start_time
                        && last_cue.settings == current_cue.settings
                        && last_cue.payload == current_cue.payload
                    {
                        let last_cue_index = trimmed_cues.len() - 1;
                        trimmed_cues.get_mut(last_cue_index).unwrap().end_time =
                            current_cue.end_time;
                        continue;
                    }
                }

                trimmed_cues.push(current_cue);
            }
        }

        Self { cues: trimmed_cues }
    }

    pub fn to_vtt(&self) -> String {
        let mut subtitles = "WEBVTT\n\n".to_owned();

        for cue in &self.cues {
            let _ = write!(
                subtitles,
                "{} --> {} {}\n{}\n\n",
                seconds_to_timestamp(cue.start_time, "."),
                seconds_to_timestamp(cue.end_time, "."),
                cue.settings,
                cue.payload
            );
        }

        subtitles
    }

    pub fn to_srt(&self) -> String {
        let mut subtitles = String::new();

        for (i, cue) in self.cues.iter().enumerate() {
            let _ = write!(
                subtitles,
                "{}\n{} --> {}\n{}\n\n",
                i + 1,
                seconds_to_timestamp(cue.start_time, ","),
                seconds_to_timestamp(cue.end_time, ","),
                cue.payload
            );
        }

        subtitles
    }
}

fn seconds_to_timestamp(seconds: f32, millisecond_sep: &str) -> String {
    let divmod = |x: usize, y: usize| (x / y, x % y);

    let (seconds, milliseconds) = divmod((seconds * 1000.0) as usize, 1000);
    let (minutes, seconds) = divmod(seconds, 60);
    let (hours, minutes) = divmod(minutes, 60);
    format!(
        "{:02}:{:02}:{:02}{}{:03}",
        hours, minutes, seconds, millisecond_sep, milliseconds
    )
}
