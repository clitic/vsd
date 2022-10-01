use crate::dash;

pub fn autoselect(
    master: &mut m3u8_rs::MasterPlaylist,
    audio_lang: Option<String>,
    subtitles_lang: Option<String>,
) {
    let mut alternative_audio = vec![];
    let mut alternative_subtitles = vec![];

    for (i, alternative) in master.alternatives.iter().enumerate() {
        if !matches!(
            alternative.media_type,
            m3u8_rs::AlternativeMediaType::Audio
                | m3u8_rs::AlternativeMediaType::Subtitles
                | m3u8_rs::AlternativeMediaType::ClosedCaptions
        ) {
            continue;
        }

        let mut quality_factor = 0;
        let mut language_factor = 0;

        let dash_playlist_tags = dash::PlaylistTag::from(&alternative.other_attributes);

        if let Some(bandwidth) = dash_playlist_tags.bandwidth {
            quality_factor += bandwidth;
        }

        if let Some(channels) = &alternative.channels {
            quality_factor += channels.parse::<usize>().unwrap();
        }

        if let Some(language) = alternative.language.as_ref().map(|x| x.to_lowercase()) {
            match &alternative.media_type {
                m3u8_rs::AlternativeMediaType::Audio => {
                    if let Some(audio_language) = audio_lang.as_ref().map(|x| x.to_lowercase()) {
                        if language == audio_language {
                            language_factor = 2;
                        } else if language.get(0..2) == audio_language.get(0..2) {
                            language_factor = 1;
                        }
                    }
                }
                m3u8_rs::AlternativeMediaType::Subtitles
                | m3u8_rs::AlternativeMediaType::ClosedCaptions => {
                    if let Some(subtitles_language) =
                        subtitles_lang.as_ref().map(|x| x.to_lowercase())
                    {
                        if language == subtitles_language {
                            language_factor = 2;
                        } else if language.get(0..2) == subtitles_language.get(0..2) {
                            language_factor = 1;
                        }
                    }
                }
                _ => (),
            }
        }

        if alternative.media_type == m3u8_rs::AlternativeMediaType::Audio {
            alternative_audio.push((i, quality_factor, language_factor));
        } else if alternative.media_type == m3u8_rs::AlternativeMediaType::Subtitles {
            alternative_subtitles.push((i, quality_factor, language_factor));
        }
    }

    if !alternative_audio.is_empty() {
        alternative_audio.sort_by(|x, y| y.1.cmp(&x.1));
        alternative_audio.sort_by(|x, y| y.2.cmp(&x.2));
        master
            .alternatives
            .get_mut(alternative_audio[0].0)
            .unwrap()
            .autoselect = true;
    }

    if !alternative_subtitles.is_empty() {
        alternative_subtitles.sort_by(|x, y| y.2.cmp(&x.2));
        master
            .alternatives
            .get_mut(alternative_subtitles[0].0)
            .unwrap()
            .autoselect = true;
    }
}
