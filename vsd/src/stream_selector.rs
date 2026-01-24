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
    ops::Range,
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

    fn stream_choices(&self) -> (Vec<Choice<(String, bool)>>, [Range<usize>; 3]) {
        let mut choices = Vec::new();
        let mut ranges: [Range<usize>; 3] = [(0..0), (0..0), (0..0)];

        choices.push(requestty::Separator(
            "─────── Video Streams ────────".to_owned(),
        ));
        choices.extend(self.streams.vid_streams.iter().map(|(i, x)| {
            requestty::Choice((x.display_stream(), self.selected_indices.contains(i)))
        }));
        ranges[0] = 1..choices.len();

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

    fn interact_modern(mut self) -> Result<Vec<MediaPlaylist>> {
        let (choices, ranges) = self.stream_choices();

        let question = Question::multi_select("streams")
            .should_loop(false)
            .message("Select streams to download")
            .choices_with_default(choices)
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

        let mut streams = Vec::new();
        let mut vid_offset = 1;
        let mut aud_offset = vid_offset + self.streams.vid_streams.len() + 1;
        let mut sub_offset = aud_offset + self.streams.aud_streams.len() + 1;

        for selected_item in answer.as_list_items().unwrap() {
            if ranges[0].contains(&selected_item.index) {
                streams.push(
                    self.streams
                        .vid_streams
                        .remove(selected_item.index - vid_offset)
                        .1,
                );
                vid_offset += 1;
            } else if ranges[1].contains(&selected_item.index) {
                streams.push(
                    self.streams
                        .aud_streams
                        .remove(selected_item.index - aud_offset)
                        .1,
                );
                aud_offset += 1;
            } else if ranges[2].contains(&selected_item.index) {
                streams.push(
                    self.streams
                        .sub_streams
                        .remove(selected_item.index - sub_offset)
                        .1,
                );
                sub_offset += 1;
            }
        }

        Ok(streams)
    }

    fn interact_none(self) -> Result<Vec<MediaPlaylist>> {
        let mut streams = Vec::new();

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
                streams.push(stream);
            } else {
                info!(
                    "Stream [{}] {}",
                    stream.media_type.to_string().yellow(),
                    stream.display_stream().dimmed()
                );
            }
        }

        Ok(streams)
    }

    fn interact_raw(mut self) -> Result<Vec<MediaPlaylist>> {
        let (choices, ranges) = self.stream_choices();

        info!("Select streams to download:");

        let mut index = 1;
        let mut selected_indices = Vec::new();

        for choice in &choices {
            if let requestty::Separator(separator) = choice {
                info!("{}", separator.replace('─', "-").cyan());
            } else if let requestty::Choice((message, selected)) = choice {
                if *selected {
                    selected_indices.push(index);
                }
                info!(
                    "{:2}) [{}] {}",
                    index,
                    if *selected { "x".green() } else { " ".normal() },
                    message
                );
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
            selected_indices = input
                .split(',')
                .filter_map(|x| x.trim().parse::<usize>().ok())
                .collect::<Vec<usize>>();
        }

        let mut streams = Vec::new();
        let mut vid_offset = 1;
        let mut aud_offset = vid_offset + self.streams.vid_streams.len();
        let mut sub_offset = aud_offset + self.streams.aud_streams.len();

        for i in selected_indices {
            if ranges[0].contains(&i) {
                let stream = self.streams.vid_streams.remove(i - vid_offset).1;
                streams.push(stream);
                vid_offset += 1;
            } else if ranges[1].contains(&i) {
                let stream = self.streams.aud_streams.remove(i - aud_offset).1;
                streams.push(stream);
                aud_offset += 1;
            } else if ranges[2].contains(&i) {
                let stream = self.streams.sub_streams.remove(i - sub_offset).1;
                streams.push(stream);
                sub_offset += 1;
            }
        }

        for stream in &streams {
            info!(
                "Stream [{}] {}",
                stream.media_type.to_string().yellow(),
                stream.display_stream().cyan()
            );
        }

        info!("{}", "------------------------------".cyan());
        Ok(streams)
    }
}
