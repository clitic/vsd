use mp4decrypt::Ap4CencDecryptingProcessor;
use std::{
    error::Error,
    fs::{self, File},
    io::Write,
    path::PathBuf,
    sync::Arc,
    sync::LazyLock,
    thread,
};

const VIDEO_KID: &str = "eb676abbcb345e96bbcf616630f1a3da";
const VIDEO_KEY: &str = "100b6c20940f779a4589152b57d2dacb";
const AUDIO_KID: &str = "63cb5f7184dd4b689a5c5ff11ee6a328";
const AUDIO_KEY: &str = "3bda3329158a4789880816a70e7e436d";

static SAMPLES_DIR: LazyLock<PathBuf> =
    LazyLock::new(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/samples"));

static OUTPUT_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../target/mp4decrypt-samples");
    fs::create_dir_all(&dir).ok();
    dir
});

macro_rules! sample {
    ($test_name: ident, $scheme: literal, $mode: literal, $track: literal) => {
        #[test]
        fn $test_name() -> Result<(), Box<dyn Error>> {
            let mut builder = Ap4CencDecryptingProcessor::new().key(VIDEO_KID, VIDEO_KEY)?;

            if $mode == "multi" {
                builder = builder.key(AUDIO_KID, AUDIO_KEY)?;
            }

            let ctx = builder.build()?;

            let init_data =
                fs::read(SAMPLES_DIR.join(concat!($scheme, "-", $mode, "/", $track, "_init.mp4")))?;
            let segment_data =
                fs::read(SAMPLES_DIR.join(concat!($scheme, "-", $mode, "/", $track, "_1.m4s")))?;

            let decrypted = ctx.decrypt(&segment_data, Some(&init_data))?;

            fs::create_dir_all(OUTPUT_DIR.join(concat!($scheme, "-", $mode)))?;
            fs::write(
                OUTPUT_DIR.join(concat!($scheme, "-", $mode, "/", $track, ".mp4")),
                decrypted,
            )?;
            Ok(())
        }
    };
}

// ==========================================
// CENC Tests
// ==========================================

sample!(test_cenc_multi_video, "cenc", "multi", "video");
sample!(test_cenc_multi_audio, "cenc", "multi", "audio");
sample!(test_cenc_single_video, "cenc", "single", "video");
sample!(test_cenc_single_audio, "cenc", "single", "audio");

// ==========================================
// CENS Tests
// ==========================================

sample!(test_cens_multi_video, "cens", "multi", "video");
sample!(test_cens_multi_audio, "cens", "multi", "audio");
sample!(test_cens_single_video, "cens", "single", "video");
sample!(test_cens_single_audio, "cens", "single", "audio");

// ==========================================
// CBC1 Tests
// ==========================================

sample!(test_cbc1_multi_video, "cbc1", "multi", "video");
sample!(test_cbc1_multi_audio, "cbc1", "multi", "audio");
sample!(test_cbc1_single_video, "cbc1", "single", "video");
sample!(test_cbc1_single_audio, "cbc1", "single", "audio");

// ==========================================
// CBCS Tests
// ==========================================

sample!(test_cbcs_multi_video, "cbcs", "multi", "video");
sample!(test_cbcs_multi_audio, "cbcs", "multi", "audio");
sample!(test_cbcs_single_video, "cbcs", "single", "video");
sample!(test_cbcs_single_audio, "cbcs", "single", "audio");

#[test]
fn test_cenc_multi_video_file() -> Result<(), Box<dyn Error>> {
    let output_dir = OUTPUT_DIR.join("cenc-multi-video-file");
    let output_path = output_dir.join("video.mp4");

    fs::create_dir_all(&output_dir)?;

    Ap4CencDecryptingProcessor::new()
        .key(VIDEO_KID, VIDEO_KEY)?
        .key(AUDIO_KID, AUDIO_KEY)?
        .build()?
        .decrypt_file(
            SAMPLES_DIR.join("cenc-multi/video_1.m4s"),
            output_path.clone(),
            Some(SAMPLES_DIR.join("cenc-multi/video_init.mp4")),
        )?;

    assert!(output_path.exists());

    let mut f = File::create(output_dir.join("video-with-init.mp4"))?;
    f.write_all(&fs::read(SAMPLES_DIR.join("cenc-multi/video_init.mp4"))?)?;
    f.write_all(&fs::read(output_path).unwrap())?;
    Ok(())
}

#[test]
fn test_cenc_multi_video_threaded() -> Result<(), Box<dyn Error>> {
    let init_data = Arc::new(fs::read(SAMPLES_DIR.join("cenc-multi/video_init.mp4"))?);
    let segment_data = Arc::new(fs::read(SAMPLES_DIR.join("cenc-multi/video_1.m4s"))?);
    let out_dir = Arc::new(OUTPUT_DIR.join("cenc-multi-video-threaded"));

    fs::create_dir_all(&*out_dir)?;

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
