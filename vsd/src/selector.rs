use crate::{
    automation::{self, InteractionType, Quality, SelectOptions},
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
            interaction_type: automation::get_interaction_type(),
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

        if opts.vid.all {
            for (i, _) in &vid_data {
                self.selected_indices.insert(*i);
            }
            return;
        }

        let mut indices = HashSet::new();

        for (i, _) in &vid_data {
            if opts.stream_indices.iter().any(|x| (*x - 1) == *i) {
                indices.insert(*i);
            }
        }

        match &opts.vid.quality {
            Quality::Best => {
                if let Some((i, _)) = vid_data.first() {
                    indices.insert(*i);
                }
            }
            Quality::None => (),
            Quality::Worst => {
                if let Some((i, _)) = vid_data.last() {
                    indices.insert(*i);
                }
            }
        }

        for (i, resolution) in &vid_data {
            if let Some((w, h)) = resolution {
                if opts.vid.resolutions.contains(&(*w as u16, *h as u16)) {
                    indices.insert(*i);
                }
            }
        }

        if opts.vid.skip && !indices.is_empty() {
            for (i, _) in &vid_data {
                if !indices.contains(i) {
                    self.selected_indices.insert(*i);
                }
            }
        } else if !opts.vid.skip {
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

        if opts.aud.all {
            for (i, _) in &aud_data {
                self.selected_indices.insert(*i);
            }
            return;
        }

        let mut indices = HashSet::new();

        for (i, _) in &aud_data {
            if opts.stream_indices.iter().any(|x| (*x - 1) == *i) {
                indices.insert(*i);
            }
        }

        for (i, lang) in &aud_data {
            if let Some(lang) = lang {
                if opts.aud.contains_exact_lang(lang) {
                    indices.insert(*i);
                }
            }
        }

        for (i, lang) in &aud_data {
            if let Some(lang) = lang {
                if opts.aud.contains_siml_lang(lang) {
                    indices.insert(*i);
                }
            }
        }

        if opts.aud.skip && !indices.is_empty() {
            for (i, _) in &aud_data {
                if !indices.contains(i) {
                    self.selected_indices.insert(*i);
                }
            }
        } else if !opts.aud.skip {
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

        if opts.sub.all {
            for (i, _) in &sub_data {
                self.selected_indices.insert(*i);
            }
            return;
        }

        let mut indices = HashSet::new();

        for (i, _) in &sub_data {
            if opts.stream_indices.iter().any(|x| (*x - 1) == *i) {
                indices.insert(*i);
            }
        }

        for (i, lang) in &sub_data {
            if let Some(lang) = lang {
                if opts.sub.contains_exact_lang(lang) {
                    indices.insert(*i);
                }
            }
        }

        for (i, lang) in &sub_data {
            if let Some(lang) = lang {
                if opts.sub.contains_siml_lang(lang) {
                    indices.insert(*i);
                }
            }
        }

        if opts.sub.skip && !indices.is_empty() {
            for (i, _) in &sub_data {
                if !indices.contains(i) {
                    self.selected_indices.insert(*i);
                }
            }
        } else if !opts.sub.skip {
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
        let selected_indices = answer
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
                if selected_indices.contains(&i) {
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

        let mut choice_idx = 1_usize;
        let mut indices = HashSet::new();

        for choice in &choices {
            match choice {
                requestty::Separator(header) => info!("{}", header.replace('─', "-").cyan()),
                requestty::Choice((msg, selected)) => {
                    if *selected {
                        indices.insert(choice_idx);
                    }
                    info!(
                        "{:>2}) [{}] {}",
                        choice_idx,
                        if *selected { "x".green() } else { " ".normal() },
                        msg
                    );
                    choice_idx += 1;
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

        if !input.trim().is_empty() {
            indices = input
                .trim()
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect();
        }

        let selected_indices = indices
            .into_iter()
            .filter_map(|n| n.checked_sub(1))
            .collect::<HashSet<_>>();

        let streams: Vec<MediaPlaylist> = self
            .streams
            .into_iter()
            .filter_map(|(i, stream)| {
                if selected_indices.contains(&i) {
                    info!(
                        "Stream [{}] {}",
                        stream.media_type.to_string().yellow(),
                        stream.to_string().cyan()
                    );
                    Some(stream)
                } else {
                    None
                }
            })
            .collect();

        info!("{}", "------------------------------".cyan());
        Ok(streams)
    }
}
