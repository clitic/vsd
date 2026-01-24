use crate::{
    automation::{self, InteractionType, SelectOptions, VideoPreference},
    playlist::{MediaPlaylist, MediaType},
};
use anyhow::Result;
use colored::Colorize;
use log::info;
use requestty::{Question, question::Choice};
use std::{
    collections::HashSet,
    io::{self, Write},
};

struct Streams {
    vid_streams: Vec<(usize, MediaPlaylist)>,
    aud_streams: Vec<(usize, MediaPlaylist)>,
    sub_streams: Vec<(usize, MediaPlaylist)>,
}

impl Streams {
    fn new(streams: Vec<MediaPlaylist>) -> Self {
        let mut vid_streams = Vec::new();
        let mut aud_streams = Vec::new();
        let mut sub_streams = Vec::new();

        // TODO - Add support for und streams
        for stream in streams.into_iter().enumerate() {
            match stream.1.media_type {
                MediaType::Audio => aud_streams.push(stream),
                MediaType::Subtitles => sub_streams.push(stream),
                MediaType::Undefined => (),
                MediaType::Video => vid_streams.push(stream),
            }
        }

        Self {
            vid_streams,
            aud_streams,
            sub_streams,
        }
    }
}

pub struct StreamSelector {
    interaction_type: InteractionType,
    selected_indices: HashSet<usize>,
    streams: Streams,
}

impl StreamSelector {
    pub fn new(streams: Vec<MediaPlaylist>) -> Self {
        Self {
            selected_indices: HashSet::new(),
            interaction_type: automation::load_interaction_type(),
            streams: Streams::new(streams),
        }
    }

    pub fn select(mut self, select_opts: &mut SelectOptions) -> Result<Vec<MediaPlaylist>> {
        self.select_vid(select_opts);
        self.select_aud(select_opts);
        self.select_sub(select_opts);

        match self.interaction_type {
            InteractionType::Modern => self.interact_modern(),
            InteractionType::None => self.interact_none(),
            InteractionType::Raw => self.interact_raw(),
        }
    }

    fn select_vid(&mut self, opts: &SelectOptions) {
        // Select all
        if opts.video.all {
            for (i, _) in &self.streams.vid_streams {
                self.selected_indices.insert(*i);
            }
            return;
        }

        let mut indices = HashSet::new();

        // Select by stream number
        for (i, _) in &self.streams.vid_streams {
            if opts.stream_numbers.iter().any(|x| (*x - 1) == *i) {
                indices.insert(*i);
            }
        }

        // Select by quality preference
        match &opts.video.preference {
            VideoPreference::Best => {
                if let Some((i, _)) = self.streams.vid_streams.first() {
                    indices.insert(*i);
                }
            }
            VideoPreference::None => (),
            VideoPreference::Worst => {
                if let Some((i, _)) = self.streams.vid_streams.last() {
                    indices.insert(*i);
                }
            }
        }

        // Select by resolution preference
        for (i, stream) in &self.streams.vid_streams {
            if let Some((w, h)) = &stream.resolution
                && opts.video.resolutions.contains(&(*w as u16, *h as u16))
            {
                indices.insert(*i);
            }
        }

        // Select inverted when skip is enabled
        if opts.video.skip && !indices.is_empty() {
            for (i, _) in &self.streams.vid_streams {
                if !indices.contains(i) {
                    self.selected_indices.insert(*i);
                }
            }
        } else if !opts.video.skip {
            if indices.is_empty() {
                if let Some((i, _)) = self.streams.vid_streams.first() {
                    indices.insert(*i);
                }
            }
            for i in indices {
                self.selected_indices.insert(i);
            }
        } // Skipped
    }

    fn select_aud(&mut self, opts: &mut SelectOptions) {
        if opts.audio.all {
            for (i, _) in &self.streams.aud_streams {
                self.selected_indices.insert(*i);
            }
            return;
        }

        let mut indices = HashSet::new();

        for (i, _) in &self.streams.aud_streams {
            if opts.stream_numbers.iter().any(|x| (*x - 1) == *i) {
                indices.insert(*i);
            }
        }

        for (i, stream) in &self.streams.aud_streams {
            if let Some(stream_lang) = &stream.language
                && opts.audio.contains_exact_lang(stream_lang)
            {
                indices.insert(*i);
            }
        }

        for (i, stream) in &self.streams.aud_streams {
            if let Some(stream_lang) = &stream.language
                && opts.audio.contains_siml_lang(stream_lang)
            {
                indices.insert(*i);
            }
        }

        if opts.audio.skip && !indices.is_empty() {
            for (i, _) in &self.streams.aud_streams {
                if !indices.contains(i) {
                    self.selected_indices.insert(*i);
                }
            }
        } else if !opts.audio.skip {
            if indices.is_empty() {
                if let Some((i, _)) = self.streams.aud_streams.first() {
                    indices.insert(*i);
                }
            }
            for i in indices {
                self.selected_indices.insert(i);
            }
        }
    }

    fn select_sub(&mut self, opts: &mut SelectOptions) {
        if opts.subs.all {
            for (i, _) in &self.streams.sub_streams {
                self.selected_indices.insert(*i);
            }
            return;
        }

        let mut selected_sstreams = HashSet::new();

        for (i, _) in &self.streams.sub_streams {
            if opts.stream_numbers.iter().any(|x| (*x - 1) == *i) {
                selected_sstreams.insert(*i);
            }
        }

        for (i, stream) in &self.streams.sub_streams {
            if let Some(stream_lang) = &stream.language
                && opts.subs.contains_exact_lang(stream_lang)
            {
                selected_sstreams.insert(*i);
            }
        }

        for (i, stream) in &self.streams.sub_streams {
            if let Some(stream_lang) = &stream.language
                && opts.subs.contains_siml_lang(stream_lang)
            {
                selected_sstreams.insert(*i);
            }
        }

        if opts.subs.skip && !selected_sstreams.is_empty() {
            for (i, _) in &self.streams.sub_streams {
                if !selected_sstreams.contains(i) {
                    self.selected_indices.insert(*i);
                }
            }
        } else if !opts.subs.skip {
            if selected_sstreams.is_empty() {
                if let Some((i, _)) = self.streams.sub_streams.first() {
                    selected_sstreams.insert(*i);
                }
            }
            for i in selected_sstreams {
                self.selected_indices.insert(i);
            }
        }
    }

    fn into_all_streams(self) -> Vec<(usize, MediaPlaylist)> {
        self.streams
            .vid_streams
            .into_iter()
            .chain(self.streams.aud_streams)
            .chain(self.streams.sub_streams)
            .collect()
    }

    fn build_choices(&self) -> Vec<Choice<(String, bool)>> {
        let mut choices = Vec::new();

        choices.push(requestty::Separator(
            "─────── Video Streams ────────".into(),
        ));
        for (i, stream) in &self.streams.vid_streams {
            choices.push(requestty::Choice((
                stream.to_string(),
                self.selected_indices.contains(i),
            )));
        }

        choices.push(requestty::Separator(
            "─────── Audio Streams ────────".into(),
        ));
        for (i, stream) in &self.streams.aud_streams {
            choices.push(requestty::Choice((
                stream.to_string(),
                self.selected_indices.contains(i),
            )));
        }

        choices.push(requestty::Separator(
            "────── Subtitle Streams ──────".into(),
        ));
        for (i, stream) in &self.streams.sub_streams {
            choices.push(requestty::Choice((
                stream.to_string(),
                self.selected_indices.contains(i),
            )));
        }

        choices
    }

    fn interact_modern(self) -> Result<Vec<MediaPlaylist>> {
        let choices = self.build_choices();
        let vid_len = self.streams.vid_streams.len();
        let aud_len = self.streams.aud_streams.len();
        let all_streams = self.into_all_streams();

        let question = Question::multi_select("streams")
            .should_loop(false)
            .message("Select streams to download")
            .choices_with_default(choices)
            .transform(|choices, _, backend| {
                let summary = choices
                    .iter()
                    .map(|x| {
                        x.text
                            .split('|')
                            .map(|s| s.trim())
                            .collect::<Vec<_>>()
                            .join(" ")
                    })
                    .collect::<Vec<_>>()
                    .join(" | ");
                backend.write_styled(&requestty::prompt::style::Stylize::cyan(&summary))
            })
            .build();

        let answer = requestty::prompt_one(question)?;

        let selected = answer
            .as_list_items()
            .unwrap()
            .iter()
            .filter_map(|item| {
                // Subtract separator count: 1 before video, 2 before audio, 3 before subs
                let idx = item.index;
                if idx <= vid_len {
                    Some(idx - 1)
                } else if idx <= vid_len + 1 + aud_len {
                    Some(idx - 2)
                } else {
                    Some(idx - 3)
                }
            })
            .collect::<HashSet<usize>>();

        Ok(all_streams
            .into_iter()
            .enumerate()
            .filter(|(pos, _)| selected.contains(pos))
            .map(|(_, (_, stream))| stream)
            .collect())
    }

    fn interact_none(self) -> Result<Vec<MediaPlaylist>> {
        let selected_indices = self.selected_indices;
        let all_streams = self
            .streams
            .vid_streams
            .into_iter()
            .chain(self.streams.aud_streams)
            .chain(self.streams.sub_streams);

        let mut streams = Vec::new();

        for (i, stream) in all_streams {
            let selected = selected_indices.contains(&i);
            info!(
                "Stream [{}] {}",
                stream.media_type.to_string().yellow(),
                if selected {
                    stream.to_string().cyan()
                } else {
                    stream.to_string().dimmed()
                }
            );
            if selected {
                streams.push(stream);
            }
        }

        Ok(streams)
    }

    fn interact_raw(self) -> Result<Vec<MediaPlaylist>> {
        let choices = self.build_choices();
        let all_streams = self.into_all_streams();

        info!("Select streams to download:");

        let mut choice_idx = 0;
        let mut defaults = Vec::new();

        for choice in &choices {
            match choice {
                requestty::Separator(sep) => info!("{}", sep.replace('─', "-").cyan()),
                requestty::Choice((msg, selected)) => {
                    choice_idx += 1;
                    if *selected {
                        defaults.push(choice_idx);
                    }
                    info!(
                        "{:2}) [{}] {}",
                        choice_idx,
                        if *selected { "x".green() } else { " ".normal() },
                        msg
                    );
                }
                _ => (),
            }
        }

        info!("{}", "------------------------------".cyan());
        print!("Press enter to proceed with defaults.\nOr select streams (1, 2, etc.): ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        info!("{}", "------------------------------".cyan());

        let user_selection: Vec<usize> = if input.trim().is_empty() {
            defaults
        } else {
            input
                .trim()
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect()
        };

        // Convert 1-based user indices to 0-based flat list positions
        let selected_positions: HashSet<usize> = user_selection
            .into_iter()
            .filter_map(|n| n.checked_sub(1)) // 1-based to 0-based
            .collect();

        let streams: Vec<MediaPlaylist> = all_streams
            .into_iter()
            .enumerate()
            .filter(|(pos, _)| selected_positions.contains(pos))
            .map(|(_, (_, stream))| stream)
            .collect();

        for stream in &streams {
            info!(
                "Stream [{}] {}",
                stream.media_type.to_string().yellow(),
                stream.to_string().cyan()
            );
        }

        info!("{}", "------------------------------".cyan());
        Ok(streams)
    }
}
