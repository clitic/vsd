use mp4decrypt::Ap4Context;
use mp4decrypt::{Error, Mp4Decrypter};
use std::{fs, path::PathBuf, sync::Arc, thread};

const VIDEO_KID: &str = "eb676abbcb345e96bbcf616630f1a3da";
const VIDEO_KEY: &str = "100b6c20940f779a4589152b57d2dacb";
const AUDIO_KID: &str = "63cb5f7184dd4b689a5c5ff11ee6a328";
const AUDIO_KEY: &str = "3bda3329158a4789880816a70e7e436d";

fn samples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/samples")
}

fn output_dir() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../target/mp4decrypt-samples");
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

#[test]
fn test_multithreaded_decryption() -> Result<(), Error> {
    let init_data = Arc::new(fs::read(samples_dir().join("cenc-multi/video_init.mp4")).unwrap());
    let segment_data = Arc::new(fs::read(samples_dir().join("cenc-multi/video_1.m4s")).unwrap());
    let out_dir = Arc::new(output_dir().join("multi-threaded"));

    fs::create_dir_all(&*out_dir).unwrap();

    let ctx = Arc::new(
        Ap4Context::new()
            .key(VIDEO_KID, VIDEO_KEY)?
            .key(AUDIO_KID, AUDIO_KEY)?
            .build()?,
    );

    let handles = (0..5)
        .map(|i| {
            let ctx = Arc::clone(&ctx);
            let init = Arc::clone(&init_data);
            let segment = Arc::clone(&segment_data);
            let out_dir = Arc::clone(&out_dir);

            thread::spawn(move || {
                for j in 0..5 {
                    let result = ctx.decrypt(&init, &segment);
                    assert!(
                        result.is_ok(),
                        "Thread {} iteration {} failed: {:?}",
                        i,
                        j,
                        result.err()
                    );
                    let decrypted = result.unwrap();
                    assert!(!decrypted.is_empty(), "Decrypted data should not be empty");

                    fs::write(
                        out_dir.join(format!("thread-{}-iter-{}.mp4", i, j)),
                        &decrypted,
                    )
                    .unwrap();
                }
            })
        })
        .collect::<Vec<_>>();

    for handle in handles {
        handle.join().expect("Thread panicked");
    }

    Ok(())
}

#[test]
fn test_file_based_decryption() -> Result<(), Error> {
    let ctx = Ap4Context::new().key(VIDEO_KID, VIDEO_KEY)?.build()?;

    let out_dir = output_dir().join("file-based");
    fs::create_dir_all(&out_dir).unwrap();

    let decrypted_segment = out_dir.join("video_segment.m4s");
    ctx.decrypt_file(
        Some(&samples_dir().join("cenc-single/video_init.mp4")),
        &samples_dir().join("cenc-single/video_1.m4s"),
        &decrypted_segment,
    )?;

    // Merge init + decrypted segment for a playable file
    let init = fs::read(samples_dir().join("cenc-single/video_init.mp4")).unwrap();
    let segment = fs::read(&decrypted_segment).unwrap();
    let mut playable = init;
    playable.extend(segment);
    fs::write(out_dir.join("video.mp4"), playable).unwrap();

    assert!(out_dir.join("video.mp4").exists());
    Ok(())
}
