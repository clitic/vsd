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

pub struct StreamSelector {
    interaction_type: InteractionType,
    selected_indices: HashSet<usize>,
    streams: Vec<(usize, MediaPlaylist)>,
}

impl StreamSelector {
    pub fn new(streams: Vec<MediaPlaylist>) -> Self {
        Self {
            selected_indices: HashSet::new(),
            interaction_type: automation::load_interaction_type(),
            streams: streams.into_iter().enumerate().collect(),
        }
    }

    pub fn select(mut self, opts: &mut SelectOptions) -> Result<Vec<MediaPlaylist>> {
        self.select_vid_streams(opts);
        self.select_aud_streams(opts);
        self.select_sub_streams(opts);

        match self.interaction_type {
            InteractionType::Modern => self.interact_modern(),
            InteractionType::None => self.interact_none(),
            InteractionType::Raw => self.interact_raw(),
        }
    }

    fn select_vid_streams(&mut self, opts: &SelectOptions) {
        let vid_data = self
            .streams
            .iter()
            .filter(|(_, s)| s.media_type == MediaType::Video)
            .map(|(i, s)| (*i, s.resolution))
            .collect::<Vec<_>>();

        if opts.video.all {
            for (i, _) in &vid_data {
                self.selected_indices.insert(*i);
            }
            return;
        }

        let mut indices = HashSet::new();

        for (i, _) in &vid_data {
            if opts.stream_numbers.iter().any(|x| (*x - 1) == *i) {
                indices.insert(*i);
            }
        }

        match &opts.video.preference {
            VideoPreference::Best => {
                if let Some((i, _)) = vid_data.first() {
                    indices.insert(*i);
                }
            }
            VideoPreference::None => (),
            VideoPreference::Worst => {
                if let Some((i, _)) = vid_data.last() {
                    indices.insert(*i);
                }
            }
        }

        for (i, resolution) in &vid_data {
            if let Some((w, h)) = resolution {
                if opts.video.resolutions.contains(&(*w as u16, *h as u16)) {
                    indices.insert(*i);
                }
            }
        }

        if opts.video.skip && !indices.is_empty() {
            for (i, _) in &vid_data {
                if !indices.contains(i) {
                    self.selected_indices.insert(*i);
                }
            }
        } else if !opts.video.skip {
            if indices.is_empty() {
                if let Some((i, _)) = vid_data.first() {
                    indices.insert(*i);
                }
            }
            self.selected_indices.extend(indices);
        }
    }

    fn select_aud_streams(&mut self, opts: &mut SelectOptions) {
        let aud_data = self
            .streams
            .iter()
            .filter(|(_, s)| s.media_type == MediaType::Audio)
            .map(|(i, s)| (*i, s.language.clone()))
            .collect::<Vec<_>>();

        if opts.audio.all {
            for (i, _) in &aud_data {
                self.selected_indices.insert(*i);
            }
            return;
        }

        let mut indices = HashSet::new();

        for (i, _) in &aud_data {
            if opts.stream_numbers.iter().any(|x| (*x - 1) == *i) {
                indices.insert(*i);
            }
        }

        for (i, lang) in &aud_data {
            if let Some(lang) = lang {
                if opts.audio.contains_exact_lang(lang) {
                    indices.insert(*i);
                }
            }
        }

        for (i, lang) in &aud_data {
            if let Some(lang) = lang {
                if opts.audio.contains_siml_lang(lang) {
                    indices.insert(*i);
                }
            }
        }

        if opts.audio.skip && !indices.is_empty() {
            for (i, _) in &aud_data {
                if !indices.contains(i) {
                    self.selected_indices.insert(*i);
                }
            }
        } else if !opts.audio.skip {
            if indices.is_empty() {
                if let Some((i, _)) = aud_data.first() {
                    indices.insert(*i);
                }
            }
            self.selected_indices.extend(indices);
        }
    }

    fn select_sub_streams(&mut self, opts: &mut SelectOptions) {
        let sub_data = self
            .streams
            .iter()
            .filter(|(_, s)| s.media_type == MediaType::Subtitles)
            .map(|(i, s)| (*i, s.language.clone()))
            .collect::<Vec<_>>();

        if opts.subs.all {
            for (i, _) in &sub_data {
                self.selected_indices.insert(*i);
            }
            return;
        }

        let mut indices = HashSet::new();

        for (i, _) in &sub_data {
            if opts.stream_numbers.iter().any(|x| (*x - 1) == *i) {
                indices.insert(*i);
            }
        }

        for (i, lang) in &sub_data {
            if let Some(lang) = lang {
                if opts.subs.contains_exact_lang(lang) {
                    indices.insert(*i);
                }
            }
        }

        for (i, lang) in &sub_data {
            if let Some(lang) = lang {
                if opts.subs.contains_siml_lang(lang) {
                    indices.insert(*i);
                }
            }
        }

        if opts.subs.skip && !indices.is_empty() {
            for (i, _) in &sub_data {
                if !indices.contains(i) {
                    self.selected_indices.insert(*i);
                }
            }
        } else if !opts.subs.skip {
            if indices.is_empty() {
                if let Some((i, _)) = sub_data.first() {
                    indices.insert(*i);
                }
            }
            self.selected_indices.extend(indices);
        }
    }

    fn build_choices(&self) -> Vec<Choice<(String, bool)>> {
        let mut choices = Vec::new();

        for (media_type, header) in [
            (MediaType::Video, "─────── Video Streams ────────"),
            (MediaType::Audio, "─────── Audio Streams ────────"),
            (MediaType::Subtitles, "────── Subtitle Streams ──────"),
        ] {
            choices.push(requestty::Separator(header.into()));
            for (i, stream) in &self.streams {
                if stream.media_type == media_type {
                    choices.push(requestty::Choice((
                        stream.to_string(),
                        self.selected_indices.contains(i),
                    )));
                }
            }
        }

        choices
    }

    fn interact_modern(self) -> Result<Vec<MediaPlaylist>> {
        let vid_len = self
            .streams
            .iter()
            .filter(|(_, s)| s.media_type == MediaType::Video)
            .count();
        let aud_len = self
            .streams
            .iter()
            .filter(|(_, s)| s.media_type == MediaType::Audio)
            .count();

        let question = Question::multi_select("streams")
            .message("Select streams to download")
            .should_loop(false)
            .choices_with_default(self.build_choices())
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
            .map(|item| {
                let idx = item.index;
                if idx <= vid_len {
                    idx - 1
                } else if idx <= vid_len + 1 + aud_len {
                    idx - 2
                } else {
                    idx - 3
                }
            })
            .collect::<HashSet<_>>();

        Ok(self
            .streams
            .into_iter()
            .filter_map(|(i, stream)| {
                if selected.contains(&i) {
                    Some(stream)
                } else {
                    None
                }
            })
            .collect())
    }

    fn interact_none(self) -> Result<Vec<MediaPlaylist>> {
        let selected_indices = self.selected_indices;
        let mut result = Vec::new();

        for (i, stream) in self.streams {
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
                result.push(stream);
            }
        }

        Ok(result)
    }

    fn interact_raw(self) -> Result<Vec<MediaPlaylist>> {
        let choices = self.build_choices();

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

        let selected_positions: HashSet<usize> = user_selection
            .into_iter()
            .filter_map(|n| n.checked_sub(1))
            .collect();

        let streams: Vec<MediaPlaylist> = self
            .streams
            .into_iter()
            .filter_map(|(i, stream)| {
                if selected_positions.contains(&i) {
                    Some(stream)
                } else {
                    None
                }
            })
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
