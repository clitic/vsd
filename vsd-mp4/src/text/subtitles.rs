/*
    REFERENCES
    ----------

    1. https://github.com/shaka-project/shaka-player/blob/9ce2f675d88d5de6f779f2a62a4f4af2bcc14611/lib/text/cue.js
    2. https://www.w3.org/TR/webvtt1/

*/

use std::fmt::Write;

pub(crate) struct Cue {
    pub(crate) end_time: f32,
    pub(crate) payload: String,
    pub(crate) settings: String,
    pub(crate) start_time: f32,
}

/// Subtitles builder.
#[derive(Default)]
pub struct Subtitles {
    cues: Vec<Cue>,
}

impl Subtitles {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn extend_cues(&mut self, cues: Vec<Cue>) {
        self.cues.extend(cues);
    }

    pub(crate) fn fix_cues(self) -> Self {
        let mut cues: Vec<Cue> = Vec::new();

        for cue in self.cues {
            if !(cue.payload.is_empty() || (cue.start_time == cue.end_time)) {
                if let Some(last_cue) = cues.last()
                    && last_cue.end_time == cue.start_time
                    && last_cue.settings == cue.settings
                    && last_cue.payload == cue.payload
                {
                    cues.last_mut().unwrap().end_time = cue.end_time;
                    continue;
                }

                cues.push(cue);
            }
        }

        Self { cues }
    }

    /// Build subtitles in subrip format.
    pub fn as_srt(self) -> String {
        let cues = self.fix_cues().cues;
        let mut subtitles = String::new();

        for (i, cue) in cues.iter().enumerate() {
            let _ = write!(
                subtitles,
                "{}\n{} --> {}\n{}\n\n",
                i + 1,
                timestamp(cue.start_time, ','),
                timestamp(cue.end_time, ','),
                cue.payload
            );
        }

        subtitles
    }

    /// Build subtitles in webvtt format.
    pub fn as_vtt(self) -> String {
        let cues = self.fix_cues().cues;
        let mut subtitles = "WEBVTT\n\n".to_owned();

        for cue in cues {
            let _ = write!(
                subtitles,
                "{} --> {} {}\n{}\n\n",
                timestamp(cue.start_time, '.'),
                timestamp(cue.end_time, '.'),
                cue.settings,
                cue.payload
            );
        }

        subtitles
    }
}

fn timestamp(seconds: f32, sep: char) -> String {
    let divmod = |x, y| (x / y, x % y);
    let (s, ms) = divmod((seconds * 1000.0).round() as usize, 1000);
    let (m, s) = divmod(s, 60);
    let (h, m) = divmod(m, 60);
    format!("{h:02}:{m:02}:{s:02}{sep}{ms:03}")
}
