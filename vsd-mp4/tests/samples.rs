use std::{
    error::Error,
    fs::{self, File},
    io::Write,
    path::PathBuf,
    sync::LazyLock,
};
use vsd_mp4::decrypt::CencDecryptingProcessor;

const VIDEO_KID: &str = "eb676abbcb345e96bbcf616630f1a3da";
const VIDEO_KEY: &str = "100b6c20940f779a4589152b57d2dacb";
const AUDIO_KID: &str = "63cb5f7184dd4b689a5c5ff11ee6a328";
const AUDIO_KEY: &str = "3bda3329158a4789880816a70e7e436d";

static SAMPLES_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("mp4decrypt/tests/samples")
});

static OUTPUT_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../target/vsd-mp4-samples");
    fs::create_dir_all(&dir).ok();
    dir
});

const REF_DIR: &str = "../target/mp4decrypt-samples";

fn find_mdat(data: &[u8]) -> Option<(usize, usize)> {
    for i in 0..data.len().saturating_sub(4) {
        if &data[i..i + 4] == b"mdat" {
            let start = i + 4;
            return Some((start, data.len() - start));
        }
    }
    None
}

macro_rules! sample {
    ($test_name: ident, $scheme: literal, $mode: literal, $track: literal) => {
        #[test]
        fn $test_name() -> Result<(), Box<dyn Error>> {
            let mut builder = CencDecryptingProcessor::builder().key(VIDEO_KID, VIDEO_KEY)?;

            if $mode == "multi" {
                builder = builder.key(AUDIO_KID, AUDIO_KEY)?;
            }

            let processor = builder.build()?;

            let init_data =
                fs::read(SAMPLES_DIR.join(concat!($scheme, "-", $mode, "/", $track, "_init.mp4")))?;
            let segment_data =
                fs::read(SAMPLES_DIR.join(concat!($scheme, "-", $mode, "/", $track, "_1.m4s")))?;

            let decrypted = processor.decrypt(&segment_data, Some(&init_data))?;

            fs::create_dir_all(OUTPUT_DIR.join(concat!($scheme, "-", $mode)))?;

            // Write output for inspection
            let mut f =
                File::create(OUTPUT_DIR.join(concat!($scheme, "-", $mode, "/", $track, ".mp4")))?;
            f.write_all(&init_data)?;
            f.write_all(&decrypted)?;

            // Verify against reference if it exists
            let ref_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join(REF_DIR)
                .join(concat!($scheme, "-", $mode, "/", $track, ".mp4"));

            if ref_path.exists() {
                let reference = fs::read(ref_path)?;

                // Compare mdat content
                let (our_start, our_len) = find_mdat(&decrypted).expect("decrypted mdat not found");
                let (ref_start, ref_len) = find_mdat(&reference).expect("reference mdat not found");

                let cmp_len = our_len.min(ref_len);
                let our_data = &decrypted[our_start..our_start + cmp_len];
                let ref_data = &reference[ref_start..ref_start + cmp_len];

                if our_data != ref_data {
                    // Find first diff for reporting
                    for i in 0..cmp_len {
                        if our_data[i] != ref_data[i] {
                            panic!(
                                "Mismatch at mdat offset {}: ours={:02X}, ref={:02X}",
                                i, our_data[i], ref_data[i]
                            );
                        }
                    }
                }
            }

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
