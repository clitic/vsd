use crate::playlist;

pub fn parse_as_master(m3u8: &m3u8_rs::MasterPlaylist, uri: &str) -> playlist::MasterPlaylist {
    let mut streams = vec![];

    for video_stream in &m3u8.variants {
        streams.push(playlist::MediaPlaylist {
            bandwidth: Some(video_stream.bandwidth),
            channels: None,
            codecs: video_stream.codecs,
            extension: Some("ts".to_owned()), // cannot comment here
            frame_rate: video_stream.frame_rate.map(|x| x as f32),
            i_frame: video_stream.is_i_frame,
            language: None,
            live: false, // cannot comment here
            media_type: playlist::MediaType::Video,
            playlist_type: playlist::PlaylistType::Hls,
            resolution: if let Some(m3u8_rs::Resolution { width, height }) = video_stream.resolution
            {
                Some((width, height))
            } else {
                None
            },
            segments: vec![], // cannot comment here
            uri: video_stream.uri,
        });
    }

    for alternative_stream in &m3u8.alternatives {
        let language = alternative_stream.language.or(alternative_stream.assoc_language);

        if let Some(uri) = alternative_stream.uri {
            match &alternative_stream.media_type {
                m3u8_rs::AlternativeMediaType::Video => {
                    streams.push(playlist::MediaPlaylist {
                        bandwidth: None, // cannot comment here
                        channels: None,
                        codecs: None, // cannot comment here
                        extension: Some("ts".to_owned()), // cannot comment here
                        frame_rate: None, // cannot comment here
                        i_frame: false, // cannot comment here
                        language: None,
                        live: false, // cannot comment here
                        media_type: playlist::MediaType::Video,
                        playlist_type: playlist::PlaylistType::Hls,
                        resolution: None, // cannot comment here
                        segments: vec![], // cannot comment here
                        uri,
                    });
                }
                m3u8_rs::AlternativeMediaType::Audio => {}
                m3u8_rs::AlternativeMediaType::ClosedCaptions
                | m3u8_rs::AlternativeMediaType::Subtitles => {}
                m3u8_rs::AlternativeMediaType::Other(_) => {}
            }
        }
    }

    playlist::MasterPlaylist {
        playlist_type: playlist::PlaylistType::Hls,
        uri: uri.to_owned(),
        streams,
    }
}

// pub fn push_segments(
//     mpd: &MPD,
//     playlist: &mut playlist::MediaPlaylist,
//     baseurl: &str,
// ) -> Result<()> {
// }