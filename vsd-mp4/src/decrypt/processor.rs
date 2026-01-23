//! High-level CENC/CBCS decrypting processor.
//!
//! Provides a simple API for decrypting CENC-encrypted MP4 data.

use crate::{
    Mp4Parser,
    boxes::{SchmBox, SencBox, TencBox, TrunBox},
    data,
    decrypt::{
        SingleSampleDecrypter,
        cipher::CipherMode,
        error::{DecryptError, Result},
    },
    parser,
};
use std::collections::HashMap;

/// Builder for creating [`CencDecryptingProcessor`] instances.
///
/// Use this builder to configure decryption keys before creating the processor.
///
/// # Example
///
/// ```no_run
/// use vsd_mp4::decrypt::CencDecryptingProcessor;
///
/// let processor = CencDecryptingProcessor::builder()
///     .key("eb676abbcb345e96bbcf616630f1a3da", "100b6c20940f779a4589152b57d2dacb")?
///     .build()?;
/// # Ok::<(), vsd_mp4::decrypt::DecryptError>(())
/// ```
#[derive(Default)]
pub struct CencDecryptingProcessorBuilder {
    keys: HashMap<[u8; 16], [u8; 16]>,
}

impl CencDecryptingProcessorBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a KID/key pair for decryption.
    ///
    /// # Arguments
    ///
    /// * `kid` - The Key ID as a 32-character hexadecimal string (16 bytes)
    /// * `key` - The decryption key as a 32-character hexadecimal string (16 bytes)
    pub fn key(mut self, kid: &str, key: &str) -> Result<Self> {
        self.keys.insert(parse_hex_16(kid)?, parse_hex_16(key)?);
        Ok(self)
    }

    /// Add multiple KID/key pairs from a HashMap.
    pub fn keys(mut self, keys: &HashMap<String, String>) -> Result<Self> {
        for (kid, key) in keys {
            self.keys.insert(parse_hex_16(kid)?, parse_hex_16(key)?);
        }
        Ok(self)
    }

    /// Build the processor.
    pub fn build(self) -> Result<CencDecryptingProcessor> {
        if self.keys.is_empty() {
            return Err(DecryptError::NoKeys);
        }

        Ok(CencDecryptingProcessor { keys: self.keys })
    }
}

/// CENC/CBCS decrypting processor for MP4 data.
///
/// Provides a high-level API for decrypting CENC-encrypted MP4 segments.
///
/// # Example
///
/// ```no_run
/// use vsd_mp4::decrypt::CencDecryptingProcessor;
/// use std::fs;
///
/// let processor = CencDecryptingProcessor::builder()
///     .key("eb676abbcb345e96bbcf616630f1a3da", "100b6c20940f779a4589152b57d2dacb")?
///     .build()?;
///
/// // Decrypt in memory
/// let init_data = fs::read("init.mp4")?;
/// let segment_data = fs::read("segment.m4s")?;
/// let decrypted = processor.decrypt(&segment_data, Some(&init_data))?;
/// fs::write("output.mp4", decrypted)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Thread Safety
///
/// The processor can be shared across threads using `Arc<CencDecryptingProcessor>`.
/// Each call to `decrypt` creates its own internal state and does not require
/// mutable access.
pub struct CencDecryptingProcessor {
    keys: HashMap<[u8; 16], [u8; 16]>,
}

// SAFETY: The processor is thread-safe because decrypt() creates local state
unsafe impl Send for CencDecryptingProcessor {}
unsafe impl Sync for CencDecryptingProcessor {}

impl CencDecryptingProcessor {
    /// Create a new builder for configuring the processor.
    pub fn builder() -> CencDecryptingProcessorBuilder {
        CencDecryptingProcessorBuilder::new()
    }

    /// Get the key for a given KID.
    pub fn get_key(&self, kid: &[u8; 16]) -> Option<&[u8; 16]> {
        self.keys.get(kid)
    }

    /// Decrypt CENC-encrypted MP4 data.
    ///
    /// # Arguments
    ///
    /// * `input_data` - The encrypted MP4 segment data
    /// * `init_data` - Optional initialization segment data. When provided,
    ///   the init data is prepended to the input for parsing, and the output
    ///   contains only the decrypted segment (not the init portion).
    ///
    /// # Returns
    ///
    /// Decrypted MP4 data. If init_data was provided, this will be a complete
    /// playable MP4 (init + decrypted segment combined).
    pub fn decrypt<T: AsRef<[u8]>>(&self, input_data: T, init_data: Option<T>) -> Result<Vec<u8>> {
        let init = init_data.as_ref().map(|x| x.as_ref());
        let input = input_data.as_ref();

        let mut state = DecryptionState::new(&self.keys);

        if let Some(init) = init {
            state.parse_init(init)?;
        } else if !input.is_empty() {
            state.parse_init(&input[..input.len().min(1000)])?;
        }

        let mut output = input.to_vec();
        state.parse_and_decrypt_segment(&mut output)?;

        Ok(output)
    }
}

/// Internal state for decryption.
struct DecryptionState<'a> {
    keys: &'a HashMap<[u8; 16], [u8; 16]>,
    default_kid: Option<[u8; 16]>,
    per_sample_iv_size: u8,
    default_constant_iv: Option<Vec<u8>>,
    crypt_byte_block: u8,
    skip_byte_block: u8,
    scheme_type: u32,
}

impl<'a> DecryptionState<'a> {
    fn new(keys: &'a HashMap<[u8; 16], [u8; 16]>) -> Self {
        Self {
            keys,
            default_kid: None,
            per_sample_iv_size: 8,
            default_constant_iv: None,
            crypt_byte_block: 0,
            skip_byte_block: 0,
            scheme_type: 0,
        }
    }

    fn parse_init(&mut self, data: &[u8]) -> Result<()> {
        let schm_box = data!();
        let tenc_box = data!();

        let schm_box_c = schm_box.clone();
        let tenc_box_c = tenc_box.clone();

        let _ = Mp4Parser::new()
            .base_box("moov", parser::children)
            .base_box("trak", parser::children)
            .base_box("mdia", parser::children)
            .base_box("minf", parser::children)
            .base_box("stbl", parser::children)
            .full_box("stsd", parser::sample_description)
            .base_box("encv", parser::visual_sample_entry)
            .base_box("enca", parser::audio_sample_entry)
            .base_box("sinf", parser::children)
            .full_box("schm", move |mut box_| {
                *schm_box_c.borrow_mut() = Some(SchmBox::new(&mut box_)?);
                Ok(())
            })
            .base_box("schi", parser::children)
            .full_box("tenc", move |mut box_| {
                *tenc_box_c.borrow_mut() = Some(TencBox::new(&mut box_)?);
                Ok(())
            })
            .parse(data, true, true);

        if let Some(schm) = schm_box.borrow().as_ref() {
            self.scheme_type = schm.scheme_type;
        }

        if let Some(tenc) = tenc_box.borrow().as_ref() {
            self.default_kid = Some(tenc.default_kid);
            self.per_sample_iv_size = tenc.per_sample_iv_size;
            self.default_constant_iv = tenc.constant_iv.clone();
            self.crypt_byte_block = tenc.crypt_byte_block;
            self.skip_byte_block = tenc.skip_byte_block;
        }

        Ok(())
    }

    fn parse_and_decrypt_segment(&mut self, output: &mut [u8]) -> Result<()> {
        let segment_data = output.to_vec();

        let moof_start = data!(0u64);
        let trun_box = data!();
        let senc_box = data!();

        let iv_size = self.per_sample_iv_size;
        let constant_iv = self.default_constant_iv.clone();

        let _ = Mp4Parser::new()
            .base_box("moof", {
                let moof_start = moof_start.clone();
                move |box_| {
                    *moof_start.borrow_mut() = box_.start;
                    parser::children(box_)
                }
            })
            .base_box("traf", parser::children)
            .full_box("tfhd", |_| Ok(()))
            .full_box("tfdt", |_| Ok(()))
            .full_box("trun", {
                let trun_box = trun_box.clone();
                move |mut box_| {
                    *trun_box.borrow_mut() = Some(TrunBox::new(&mut box_)?);
                    Ok(())
                }
            })
            .full_box("senc", {
                let senc_box = senc_box.clone();
                move |mut box_| {
                    *senc_box.borrow_mut() =
                        Some(SencBox::new(&mut box_, iv_size, constant_iv.as_deref())?);
                    Ok(())
                }
            })
            .parse(&segment_data, true, true);

        let trun = trun_box.borrow();
        let senc = senc_box.borrow();

        let trun = match trun.as_ref() {
            Some(t) => t,
            None => return Ok(()),
        };

        let senc = match senc.as_ref() {
            Some(s) => s,
            None => return Ok(()),
        };

        // Get decryption key
        let kid = self
            .default_kid
            .ok_or_else(|| DecryptError::InvalidFormat("No default KID found".into()))?;

        let key = self
            .keys
            .get(&kid)
            .ok_or_else(|| DecryptError::KeyNotFound(hex::encode(kid)))?;

        // Create decrypter
        let cipher_mode = CipherMode::from_scheme_type(self.scheme_type);
        let reset_iv = CipherMode::resets_iv_per_subsample(self.scheme_type);
        let mut decrypter = SingleSampleDecrypter::new(
            cipher_mode,
            key,
            self.crypt_byte_block,
            self.skip_byte_block,
            reset_iv,
        )?;

        // Calculate sample data start offset
        let data_start = {
            let offset = trun.data_offset.unwrap_or(0) as i64;
            (*moof_start.borrow() as i64 + offset) as usize
        };

        // Decrypt each sample
        let mut offset = data_start;

        for (trun_sample, senc_sample) in trun.sample_data.iter().zip(senc.samples.iter()) {
            let size = trun_sample.sample_size.unwrap_or(0) as usize;
            if size == 0 {
                continue;
            }

            let end = offset + size;
            if end > segment_data.len() {
                break;
            }

            // Prepare IV (pad to 16 bytes)
            let mut iv = [0u8; 16];
            let len = senc_sample.iv.len().min(16);
            iv[..len].copy_from_slice(&senc_sample.iv[..len]);

            // Get subsample info
            let (subsample_count, clear, encrypted): (usize, Vec<u16>, Vec<u32>) =
                if senc_sample.subsamples.is_empty() {
                    (0, vec![], vec![])
                } else {
                    (
                        senc_sample.subsamples.len(),
                        senc_sample
                            .subsamples
                            .iter()
                            .map(|s| s.bytes_of_clear_data)
                            .collect(),
                        senc_sample
                            .subsamples
                            .iter()
                            .map(|s| s.bytes_of_encrypted_data)
                            .collect(),
                    )
                };

            let decrypted = decrypter.decrypt_sample_data(
                &segment_data[offset..end],
                &iv,
                subsample_count,
                &clear,
                &encrypted,
            )?;

            output[offset..end].copy_from_slice(&decrypted);
            offset = end;
        }

        Ok(())
    }
}

/// Parse a 16-byte hex string.
fn parse_hex_16(input: &str) -> Result<[u8; 16]> {
    let bytes = hex::decode(input)?;
    if bytes.len() != 16 {
        return Err(DecryptError::HexWrongLength {
            expected: 16,
            actual: bytes.len(),
        });
    }
    let mut arr = [0u8; 16];
    arr.copy_from_slice(&bytes);
    Ok(arr)
}
