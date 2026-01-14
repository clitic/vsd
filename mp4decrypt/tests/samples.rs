use mp4decrypt::{Error, Mp4Decrypter};
use std::{fs, path::PathBuf};

const VIDEO_KID: &str = "eb676abbcb345e96bbcf616630f1a3da";
const VIDEO_KEY: &str = "100b6c20940f779a4589152b57d2dacb";
const AUDIO_KID: &str = "63cb5f7184dd4b689a5c5ff11ee6a328";
const AUDIO_KEY: &str = "3bda3329158a4789880816a70e7e436d";

fn samples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("samples")
}

fn output_dir() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../target")
        .join("mp4decrypt-samples");
    fs::create_dir_all(&dir).ok();
    dir
}

#[test]
fn test_cenc_multi_video() -> Result<(), Error> {
    Mp4Decrypter::new()
        .key(VIDEO_KID, VIDEO_KEY)?
        .key(AUDIO_KID, AUDIO_KEY)?
        .init_file(samples_dir().join("cenc-multi/video_init.mp4"))?
        .input_file(samples_dir().join("cenc-multi/video_1.m4s"))?
        .decrypt_to_file(output_dir().join("cenc-multi-video.mp4"))?;
    Ok(())
}

#[test]
fn test_cenc_multi_audio() -> Result<(), Error> {
    Mp4Decrypter::new()
        .key(VIDEO_KID, VIDEO_KEY)?
        .key(AUDIO_KID, AUDIO_KEY)?
        .init_file(samples_dir().join("cenc-multi/audio_init.mp4"))?
        .input_file(samples_dir().join("cenc-multi/audio_1.m4s"))?
        .decrypt_to_file(output_dir().join("cenc-multi-audio.mp4"))?;
    Ok(())
}

#[test]
fn test_cenc_single_video() -> Result<(), Error> {
    Mp4Decrypter::new()
        .key(VIDEO_KID, VIDEO_KEY)?
        .init_file(samples_dir().join("cenc-single/video_init.mp4"))?
        .input_file(samples_dir().join("cenc-single/video_1.m4s"))?
        .decrypt_to_file(output_dir().join("cenc-single-video.mp4"))?;
    Ok(())
}

#[test]
fn test_cenc_single_audio() -> Result<(), Error> {
    Mp4Decrypter::new()
        .key(VIDEO_KID, VIDEO_KEY)?
        .init_file(samples_dir().join("cenc-single/audio_init.mp4"))?
        .input_file(samples_dir().join("cenc-single/audio_1.m4s"))?
        .decrypt_to_file(output_dir().join("cenc-single-audio.mp4"))?;
    Ok(())
}

#[test]
fn test_cbcs_multi_video() -> Result<(), Error> {
    Mp4Decrypter::new()
        .key(VIDEO_KID, VIDEO_KEY)?
        .key(AUDIO_KID, AUDIO_KEY)?
        .init_file(samples_dir().join("cbcs-multi/video_init.mp4"))?
        .input_file(samples_dir().join("cbcs-multi/video_1.m4s"))?
        .decrypt_to_file(output_dir().join("cbcs-multi-video.mp4"))?;
    Ok(())
}

#[test]
fn test_cbcs_multi_audio() -> Result<(), Error> {
    Mp4Decrypter::new()
        .key(VIDEO_KID, VIDEO_KEY)?
        .key(AUDIO_KID, AUDIO_KEY)?
        .init_file(samples_dir().join("cbcs-multi/audio_init.mp4"))?
        .input_file(samples_dir().join("cbcs-multi/audio_1.m4s"))?
        .decrypt_to_file(output_dir().join("cbcs-multi-audio.mp4"))?;
    Ok(())
}

#[test]
fn test_cbcs_single_video() -> Result<(), Error> {
    Mp4Decrypter::new()
        .key(VIDEO_KID, VIDEO_KEY)?
        .init_file(samples_dir().join("cbcs-single/video_init.mp4"))?
        .input_file(samples_dir().join("cbcs-single/video_1.m4s"))?
        .decrypt_to_file(output_dir().join("cbcs-single-video.mp4"))?;
    Ok(())
}

#[test]
fn test_cbcs_single_audio() -> Result<(), Error> {
    Mp4Decrypter::new()
        .key(VIDEO_KID, VIDEO_KEY)?
        .init_file(samples_dir().join("cbcs-single/audio_init.mp4"))?
        .input_file(samples_dir().join("cbcs-single/audio_1.m4s"))?
        .decrypt_to_file(output_dir().join("cbcs-single-audio.mp4"))?;
    Ok(())
}
