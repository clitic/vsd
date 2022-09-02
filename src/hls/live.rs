use crate::utils;
use reqwest::blocking::Client;
use std::iter::Iterator;
use std::sync::Arc;

pub struct LivePlaylist {
    uri: String,
    client: Arc<Client>,
    record_duration: Option<f32>,
    pub recorded_duration: f32,
    last_segment: m3u8_rs::MediaSegment,
    first: bool,
}

impl LivePlaylist {
    pub fn new(uri: &str, client: Arc<Client>, record_duration: Option<f32>) -> Self {
        Self {
            uri: uri.to_owned(),
            client,
            record_duration,
            recorded_duration: 0.0,
            last_segment: m3u8_rs::MediaSegment::default(),
            first: true,
        }
    }
}

impl Iterator for LivePlaylist {
    type Item = Result<m3u8_rs::MediaPlaylist, String>;

    fn next(&mut self) -> Option<Self::Item> {
        let fetch = || -> Result<Vec<u8>, reqwest::Error> {
            Ok(self.client.get(&self.uri).send()?.bytes()?.to_vec())
        };

        loop {
            match fetch() {
                Ok(bytes) => {
                    if let Ok(playlist) = m3u8_rs::parse_playlist_res(&bytes) {
                        if let m3u8_rs::Playlist::MediaPlaylist(media) = playlist {
                            if self.first {
                                if let Some(record_duration) = self.record_duration {
                                    for (i, segment) in media.segments.iter().enumerate() {
                                        let new_recorded_duration =
                                            self.recorded_duration + segment.duration;

                                        if new_recorded_duration > record_duration {
                                            if i == 0 {
                                                return None;
                                            } else {
                                                let mut media_clone = media.clone();
                                                media_clone.segments =
                                                    media.segments[0..i].to_vec();
                                                self.last_segment =
                                                    media.segments.last().unwrap().to_owned();
                                                return Some(Ok(media_clone));
                                            }
                                        } else {
                                            self.recorded_duration =
                                                self.recorded_duration + segment.duration;
                                        }
                                    }
                                }

                                if media.segments.is_empty() {
                                    return None;
                                }

                                self.last_segment = media.segments.last().unwrap().to_owned();
                                self.first = false;
                                return Some(Ok(media));
                            }

                            if media.segments.last() == Some(&self.last_segment) {
                                std::thread::sleep(std::time::Duration::from_secs_f32(
                                    media.target_duration,
                                ));
                                continue;
                            }

                            if let Some(new_list) = media
                                .segments
                                .split(|x| *x.uri == self.last_segment.uri)
                                .nth(1)
                            {
                                if new_list.is_empty() {
                                    std::thread::sleep(std::time::Duration::from_secs_f32(
                                        media.target_duration,
                                    ));
                                    continue;
                                }

                                let mut media_clone = media.clone();

                                if let Some(record_duration) = self.record_duration {
                                    for (i, segment) in new_list.iter().enumerate() {
                                        let new_recorded_duration =
                                            self.recorded_duration + segment.duration;

                                        if new_recorded_duration > record_duration {
                                            if i == 0 {
                                                return None;
                                            } else {
                                                media_clone.segments =
                                                    media.segments[0..i].to_vec();
                                                self.last_segment =
                                                    media_clone.segments.last().unwrap().to_owned();
                                                return Some(Ok(media_clone));
                                            }
                                        } else {
                                            self.recorded_duration =
                                                self.recorded_duration + segment.duration;
                                        }
                                    }
                                }

                                media_clone.segments =
                                    new_list.iter().map(|x| x.to_owned()).collect();

                                if media_clone.segments.is_empty() {
                                    return None;
                                }

                                self.last_segment = media_clone.segments.last().unwrap().to_owned();
                                return Some(Ok(media_clone));
                            } else {
                                std::thread::sleep(std::time::Duration::from_secs_f32(
                                    media.target_duration,
                                ));
                                continue;
                            }
                        } else {
                            return Some(Err("Media playlist not found.".to_owned()));
                        }
                    } else {
                        return Some(Err(format!("Couldn't parse {} playlist.", self.uri)));
                    }
                }
                Err(e) => {
                    if let Err(e) = utils::check_reqwest_error(&e) {
                        return Some(Err(format!("{}", e)));
                    } else {
                        continue;
                    }
                }
            }
        }
    }
}
