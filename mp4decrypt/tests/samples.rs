use mp4decrypt::{Ap4CencDecryptingProcessor, Error};
use std::{
    fs::{self, File},
    io::Write,
    path::PathBuf,
    sync::Arc,
    thread,
};

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

fn decrypt_and_save(
    ctx: &Ap4CencDecryptingProcessor,
    init_path: &PathBuf,
    segment_path: &PathBuf,
    output_path: &PathBuf,
) -> Result<(), Error> {
    let init_data = fs::read(init_path).unwrap();
    let segment_data = fs::read(segment_path).unwrap();

    let decrypted = ctx.decrypt(&segment_data, Some(&init_data))?;

    let mut playable = init_data;
    playable.extend(decrypted);
    fs::write(output_path, playable).unwrap();
    Ok(())
}

#[test]
fn test_cenc_multi_video() -> Result<(), Error> {
    let ctx = Ap4CencDecryptingProcessor::new()
        .key(VIDEO_KID, VIDEO_KEY)?
        .key(AUDIO_KID, AUDIO_KEY)?
        .build()?;

    decrypt_and_save(
        &ctx,
        &samples_dir().join("cenc-multi/video_init.mp4"),
        &samples_dir().join("cenc-multi/video_1.m4s"),
        &output_dir().join("cenc-multi-video.mp4"),
    )
}

#[test]
fn test_cenc_multi_audio() -> Result<(), Error> {
    let ctx = Ap4CencDecryptingProcessor::new()
        .key(VIDEO_KID, VIDEO_KEY)?
        .key(AUDIO_KID, AUDIO_KEY)?
        .build()?;

    decrypt_and_save(
        &ctx,
        &samples_dir().join("cenc-multi/audio_init.mp4"),
        &samples_dir().join("cenc-multi/audio_1.m4s"),
        &output_dir().join("cenc-multi-audio.mp4"),
    )
}

#[test]
fn test_cenc_single_video() -> Result<(), Error> {
    let ctx = Ap4CencDecryptingProcessor::new()
        .key(VIDEO_KID, VIDEO_KEY)?
        .build()?;

    decrypt_and_save(
        &ctx,
        &samples_dir().join("cenc-single/video_init.mp4"),
        &samples_dir().join("cenc-single/video_1.m4s"),
        &output_dir().join("cenc-single-video.mp4"),
    )
}

#[test]
fn test_cenc_single_audio() -> Result<(), Error> {
    let ctx = Ap4CencDecryptingProcessor::new()
        .key(VIDEO_KID, VIDEO_KEY)?
        .build()?;

    decrypt_and_save(
        &ctx,
        &samples_dir().join("cenc-single/audio_init.mp4"),
        &samples_dir().join("cenc-single/audio_1.m4s"),
        &output_dir().join("cenc-single-audio.mp4"),
    )
}

#[test]
fn test_cbcs_multi_video() -> Result<(), Error> {
    let ctx = Ap4CencDecryptingProcessor::new()
        .key(VIDEO_KID, VIDEO_KEY)?
        .key(AUDIO_KID, AUDIO_KEY)?
        .build()?;

    decrypt_and_save(
        &ctx,
        &samples_dir().join("cbcs-multi/video_init.mp4"),
        &samples_dir().join("cbcs-multi/video_1.m4s"),
        &output_dir().join("cbcs-multi-video.mp4"),
    )
}

#[test]
fn test_cbcs_multi_audio() -> Result<(), Error> {
    let ctx = Ap4CencDecryptingProcessor::new()
        .key(VIDEO_KID, VIDEO_KEY)?
        .key(AUDIO_KID, AUDIO_KEY)?
        .build()?;

    decrypt_and_save(
        &ctx,
        &samples_dir().join("cbcs-multi/audio_init.mp4"),
        &samples_dir().join("cbcs-multi/audio_1.m4s"),
        &output_dir().join("cbcs-multi-audio.mp4"),
    )
}

#[test]
fn test_cbcs_single_video() -> Result<(), Error> {
    let ctx = Ap4CencDecryptingProcessor::new()
        .key(VIDEO_KID, VIDEO_KEY)?
        .build()?;

    decrypt_and_save(
        &ctx,
        &samples_dir().join("cbcs-single/video_init.mp4"),
        &samples_dir().join("cbcs-single/video_1.m4s"),
        &output_dir().join("cbcs-single-video.mp4"),
    )
}

#[test]
fn test_cbcs_single_audio() -> Result<(), Error> {
    let ctx = Ap4CencDecryptingProcessor::new()
        .key(VIDEO_KID, VIDEO_KEY)?
        .build()?;

    decrypt_and_save(
        &ctx,
        &samples_dir().join("cbcs-single/audio_init.mp4"),
        &samples_dir().join("cbcs-single/audio_1.m4s"),
        &output_dir().join("cbcs-single-audio.mp4"),
    )
}

#[test]
fn test_multithreaded_decryption() -> Result<(), Error> {
    let init_data = Arc::new(fs::read(samples_dir().join("cenc-multi/video_init.mp4")).unwrap());
    let segment_data = Arc::new(fs::read(samples_dir().join("cenc-multi/video_1.m4s")).unwrap());
    let out_dir = Arc::new(output_dir().join("multi-threaded"));

    fs::create_dir_all(&*out_dir).unwrap();

    let ctx = Arc::new(
        Ap4CencDecryptingProcessor::new()
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
                    let result = ctx.decrypt(segment.as_slice(), Some(init.as_slice()));
                    assert!(
                        result.is_ok(),
                        "Thread {} iteration {} failed: {:?}",
                        i,
                        j,
                        result.err()
                    );
                    fs::write(
                        out_dir.join(format!("thread-{}-iter-{}.mp4", i, j)),
                        result.unwrap(),
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
    let init_path = samples_dir().join("cenc-single/video_init.mp4");
    let output_path = output_dir().join("cenc-single-video-file-based.mp4");

    Ap4CencDecryptingProcessor::new()
        .key(VIDEO_KID, VIDEO_KEY)?
        .build()?
        .decrypt_file(
            samples_dir().join("cenc-single/video_1.m4s"),
            output_path.clone(),
            Some(init_path.clone()),
        )?;

    assert!(output_path.exists());

    let mut f = File::create(output_dir().join("cenc-single-video-file-based-init.mp4")).unwrap();
    f.write_all(fs::read(init_path).unwrap().as_slice())
        .unwrap();
    f.write_all(fs::read(output_path).unwrap().as_slice())
        .unwrap();

    Ok(())
}
