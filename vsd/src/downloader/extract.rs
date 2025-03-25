use crate::playlist::{Key, KeyMethod, MediaPlaylist, Segment};
use anyhow::{Result, anyhow, bail};
use kdam::term::Colorizer;
use reqwest::{Url, blocking::Client, header};
use std::collections::HashSet;
use vsd_mp4::pssh::Pssh;

pub fn check_unsupported_encryptions(streams: &Vec<MediaPlaylist>) -> Result<()> {
    for stream in streams {
        if let Some(Segment { key: Some(x), .. }) = stream.segments.get(0) {
            match &x.method {
                KeyMethod::Other(x) => bail!(
                    "{} (UNK) decryption is not supported. Use {} flag to download encrypted streams.",
                    x,
                    "--no-decrypt".colorize("bold green")
                ),
                KeyMethod::SampleAes => {
                    if stream.is_hls() {
                        bail!(
                            "sample-aes (HLS) decryption is not supported. Use {} flag to download encrypted streams.",
                            "--no-decrypt".colorize("bold green")
                        );
                    }
                }
                _ => (),
            }
        }
    }

    Ok(())
}

pub fn check_key_exists_for_kid(
    keys: &Vec<(Option<String>, String)>,
    kids: &HashSet<String>,
) -> Result<()> {
    let user_kids = keys.iter().flat_map(|x| x.0.as_ref());

    for kid in kids {
        if !user_kids.clone().any(|x| x == kid) {
            bail!(
                "use {} flag to specify content decryption keys for at least * pre-fixed key ids.",
                "--key".colorize("bold green")
            );
        }
    }

    Ok(())
}

pub fn extract_kids(
    base_url: &Option<Url>,
    client: &Client,
    streams: &Vec<MediaPlaylist>,
) -> Result<HashSet<String>> {
    let mut default_kids = HashSet::new();

    for stream in streams {
        if let Some(Segment {
            key: Some(Key {
                default_kid: Some(x),
                ..
            }),
            ..
        }) = stream.segments.get(0)
        {
            default_kids.insert(x.replace('-', ""));
        }
    }

    let mut parsed_kids = HashSet::new();

    for stream in streams {
        let stream_base_url = base_url
            .clone()
            .unwrap_or(stream.uri.parse::<Url>().unwrap());

        if let Some(Segment { map: Some(x), .. }) = stream.segments.get(0) {
            let url = stream_base_url.join(&x.uri)?;
            let mut request = client.get(url);

            if let Some(range) = &x.range {
                request = request.header(header::RANGE, range.as_header_value());
            }

            let response = request.send()?;
            let pssh = Pssh::new(&response.bytes()?).map_err(|x| anyhow!(x))?;

            for kid in pssh.key_ids {
                if !parsed_kids.contains(&kid.value) {
                    parsed_kids.insert(kid.value.clone());
                    println!(
                        "      {} {} {} ({})",
                        "KeyId".colorize("bold green"),
                        if default_kids.contains(&kid.value) {
                            "*"
                        } else {
                            " "
                        },
                        kid.uuid(),
                        kid.system_type,
                    );
                }
            }
        }
    }

    Ok(parsed_kids)
}
