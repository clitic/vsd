use crate::{
    automation::{self, InteractionType, SelectOptions, VideoPreference},
    playlist::{MediaPlaylist, MediaType},
};
use anyhow::Result;
use colored::Colorize;
use log::info;
use std::{
    collections::HashSet,
    io::{self, Write},
};

struct CategorizedStreams {
    vid_streams: Vec<(usize, MediaPlaylist)>,
    aud_streams: Vec<(usize, MediaPlaylist)>,
    sub_streams: Vec<(usize, MediaPlaylist)>,
}

impl CategorizedStreams {
    fn from_streams(streams: Vec<MediaPlaylist>) -> Self {
        let mut vid_streams = Vec::new();
        let mut aud_streams = Vec::new();
        let mut sub_streams = Vec::new();

        // TODO - Add support for downloading und streams
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
    streams: CategorizedStreams,
    selected_indices: HashSet<usize>,
    interaction_type: InteractionType,
}

impl StreamSelector {
    pub fn new(streams: Vec<MediaPlaylist>) -> Self {
        Self {
            streams: CategorizedStreams::from_streams(streams),
            selected_indices: HashSet::new(),
            interaction_type: automation::load_interaction_type(),
        }
    }

    pub fn select(mut self, select_opts: &mut SelectOptions) -> Result<Vec<MediaPlaylist>> {
        self.select_vid(select_opts);
        self.select_aud(select_opts);
        self.select_sub(select_opts);

        let (choices, ranges) = self.build_stream_choices();

        match self.interaction_type {
            InteractionType::Modern => self.handle_modern_interaction(choices, ranges),
            InteractionType::None => self.handle_none_interaction(),
            InteractionType::Raw => self.handle_raw_interaction(choices, ranges),
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

        // Select by exact language match
        for (i, stream) in &self.streams.aud_streams {
            if let Some(stream_lang) = &stream.language
                && opts.audio.contains_exact_lang(stream_lang)
            {
                indices.insert(*i);
            }
        }

        // Select by similar language match
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

        // Select by exact language match
        for (i, stream) in &self.streams.sub_streams {
            if let Some(stream_lang) = &stream.language
                && opts.subs.contains_exact_lang(stream_lang)
            {
                selected_sstreams.insert(*i);
            }
        }

        // Select by similar language match
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

    /// Builds the UI choices list and computes ranges for each stream type.
    fn build_stream_choices(
        &self,
    ) -> (
        Vec<requestty::question::Choice<(String, bool)>>,
        [std::ops::Range<usize>; 4],
    ) {
        let mut choices = vec![];
        let mut ranges: [std::ops::Range<usize>; 4] = [(0..0), (0..0), (0..0), (0..0)];

        // Video streams
        choices.push(requestty::Separator(
            "─────── Video Streams ────────".to_owned(),
        ));
        choices.extend(self.streams.vid_streams.iter().map(|(i, x)| {
            requestty::Choice((x.display_stream(), self.selected_indices.contains(i)))
        }));
        ranges[0] = 1..choices.len();

        // Audio streams
        choices.push(requestty::Separator(
            "─────── Audio Streams ────────".to_owned(),
        ));
        choices.extend(self.streams.aud_streams.iter().map(|(i, x)| {
            requestty::Choice((x.display_stream(), self.selected_indices.contains(i)))
        }));

        if let InteractionType::Modern = self.interaction_type {
            ranges[1] = (ranges[0].end + 1)..choices.len();
        } else {
            ranges[1] = ranges[0].end..(choices.len() - 1);
        }

        // Subtitle streams
        choices.push(requestty::Separator(
            "────── Subtitle Streams ──────".to_owned(),
        ));
        choices.extend(self.streams.sub_streams.iter().map(|(i, x)| {
            requestty::Choice((x.display_stream(), self.selected_indices.contains(i)))
        }));

        if let InteractionType::Modern = self.interaction_type {
            ranges[2] = (ranges[1].end + 1)..choices.len();
        } else {
            ranges[2] = ranges[1].end..(choices.len() - 2);
        }

        (choices, ranges)
    }

    /// Handles modern interactive mode using requestty multi-select.
    fn handle_modern_interaction(
        mut self,
        choices_with_default: Vec<requestty::question::Choice<(String, bool)>>,
        ranges: [std::ops::Range<usize>; 4],
    ) -> Result<Vec<MediaPlaylist>> {
        let question = requestty::Question::multi_select("streams")
            .should_loop(false)
            .message("Select streams to download")
            .choices_with_default(choices_with_default)
            .transform(|choices, _, backend| {
                backend.write_styled(&requestty::prompt::style::Stylize::cyan(
                    &choices
                        .iter()
                        .map(|x| {
                            x.text
                                .split('|')
                                .map(|x| x.replace(" ", ""))
                                .collect::<Vec<_>>()
                                .join(" ")
                        })
                        .collect::<Vec<_>>()
                        .join(" | "),
                ))
            })
            .build();

        let answer = requestty::prompt_one(question)?;

        let mut selected_streams = vec![];
        let mut video_offset = 1;
        let mut audio_offset = video_offset + self.streams.vid_streams.len() + 1;
        let mut subtitle_offset = audio_offset + self.streams.aud_streams.len() + 1;

        for selected_item in answer.as_list_items().unwrap() {
            if ranges[0].contains(&selected_item.index) {
                selected_streams.push(
                    self.streams
                        .vid_streams
                        .remove(selected_item.index - video_offset)
                        .1,
                );
                video_offset += 1;
            } else if ranges[1].contains(&selected_item.index) {
                selected_streams.push(
                    self.streams
                        .aud_streams
                        .remove(selected_item.index - audio_offset)
                        .1,
                );
                audio_offset += 1;
            } else if ranges[2].contains(&selected_item.index) {
                selected_streams.push(
                    self.streams
                        .sub_streams
                        .remove(selected_item.index - subtitle_offset)
                        .1,
                );
                subtitle_offset += 1;
            }
        }

        Ok(selected_streams)
    }

    fn handle_none_interaction(self) -> Result<Vec<MediaPlaylist>> {
        let mut selected_streams = vec![];

        for (i, stream) in self
            .streams
            .vid_streams
            .into_iter()
            .chain(self.streams.aud_streams.into_iter())
            .chain(self.streams.sub_streams.into_iter())
        {
            if self.selected_indices.contains(&i) {
                info!(
                    "Stream [{}] {}",
                    stream.media_type.to_string().yellow(),
                    stream.display_stream().cyan()
                );
                selected_streams.push(stream);
            } else {
                info!(
                    "Stream [{}] {}",
                    stream.media_type.to_string().yellow(),
                    stream.display_stream().dimmed()
                );
            }
        }

        Ok(selected_streams)
    }

    fn handle_raw_interaction(
        mut self,
        choices_with_default: Vec<requestty::question::Choice<(String, bool)>>,
        ranges: [std::ops::Range<usize>; 4],
    ) -> Result<Vec<MediaPlaylist>> {
        info!("Select streams to download:");

        let mut selected_choices_index = vec![];
        let mut index = 1;

        for choice in &choices_with_default {
            if let requestty::Separator(separator) = choice {
                if let InteractionType::Raw = self.interaction_type {
                    info!("{}", separator.replace('─', "-").cyan());
                }
            } else if let requestty::Choice((message, selected)) = choice {
                if *selected {
                    selected_choices_index.push(index);
                }

                if let InteractionType::Raw = self.interaction_type {
                    info!(
                        "{:2}) [{}] {}",
                        index,
                        if *selected { "x".green() } else { " ".normal() },
                        message
                    );
                }
                index += 1;
            }
        }

        info!("{}", "------------------------------".cyan());
        print!(
            "Press enter to proceed with defaults.\n\
                    Or select streams to download (1, 2, etc.): "
        );
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        info!("{}", "------------------------------".cyan());

        let input = input.trim();

        if !input.is_empty() {
            selected_choices_index = input
                .split(',')
                .filter_map(|x| x.trim().parse::<usize>().ok())
                .collect::<Vec<usize>>();
        }

        let mut selected_streams = vec![];
        let mut video_offset = 1;
        let mut audio_offset = video_offset + self.streams.vid_streams.len();
        let mut subtitle_offset = audio_offset + self.streams.aud_streams.len();

        for i in selected_choices_index {
            if ranges[0].contains(&i) {
                let stream = self.streams.vid_streams.remove(i - video_offset).1;
                info!(
                    "Stream [{}] {}",
                    stream.media_type.to_string().yellow(),
                    stream.display_stream().cyan()
                );
                selected_streams.push(stream);
                video_offset += 1;
            } else if ranges[1].contains(&i) {
                let stream = self.streams.aud_streams.remove(i - audio_offset).1;
                info!(
                    "Stream [{}] {}",
                    stream.media_type.to_string().yellow(),
                    stream.display_stream().cyan()
                );
                selected_streams.push(stream);
                audio_offset += 1;
            } else if ranges[2].contains(&i) {
                let stream = self.streams.sub_streams.remove(i - subtitle_offset).1;
                info!(
                    "Stream [{}] {}",
                    stream.media_type.to_string().yellow(),
                    stream.display_stream().cyan()
                );
                selected_streams.push(stream);
                subtitle_offset += 1;
            }
        }

        Ok(selected_streams)
    }
}
