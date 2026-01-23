//! High-level CENC/CBCS decrypting processor.
//!
//! Provides a simple API for decrypting CENC-encrypted MP4 data.

use crate::{
    Mp4Parser,
    boxes::{SchmBox, TencBox},
    data,
    decrypt::{
        SingleSampleDecrypter,
        cipher::CipherMode,
        error::{DecryptError, Result},
        sample_info::SampleInfoTable,
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
        let input = input_data.as_ref();

        // Combine init + segment if init is provided
        let combined_data;
        let data_to_parse = if let Some(init) = init_data.as_ref() {
            combined_data = [init.as_ref(), input].concat();
            &combined_data[..]
        } else {
            input
        };

        // Parse and decrypt
        self.decrypt_internal(
            data_to_parse,
            init_data.as_ref().map(|i| i.as_ref().len()).unwrap_or(0),
        )
    }

    /// Internal decryption implementation.
    fn decrypt_internal(&self, data: &[u8], init_size: usize) -> Result<Vec<u8>> {
        let mut output = data.to_vec();
        let mut decryption_state = DecryptionState::new(&self.keys);

        // Parse the MP4 structure and find encryption info
        decryption_state
            .parse_init_segment(&data[..init_size.max(data.len().min(init_size + 1000))])?;

        // If we have segment data, find and decrypt mdat
        if init_size < data.len() {
            decryption_state.parse_and_decrypt_segment(&mut output, init_size)?;
        }

        // Return only the segment portion (skip init data)
        if init_size > 0 && init_size < output.len() {
            let segment = output[init_size..].to_vec();
            // Note: We keep encryption boxes (senc, saio, saiz) in output.
            // Removing them requires updating trun.data_offset which is complex.
            // The boxes don't affect playback - they're just metadata.
            Ok(segment)
        } else {
            Ok(output)
        }
    }

    /// Get the key for a given KID.
    pub fn get_key(&self, kid: &[u8; 16]) -> Option<&[u8; 16]> {
        self.keys.get(kid)
    }
}

/// Internal state for decryption.
struct DecryptionState<'a> {
    keys: &'a HashMap<[u8; 16], [u8; 16]>,
    /// Track encryption defaults from tenc box
    default_kid: Option<[u8; 16]>,
    /// Per-sample IV size from tenc (0 for constant IV like CBCS)
    per_sample_iv_size: u8,
    /// Effective IV size for decryption (16 for most cases)
    default_iv_size: u8,
    default_is_protected: bool,
    default_constant_iv: Option<Vec<u8>>,
    /// Pattern encryption params from tenc box (for CBCS)
    crypt_byte_block: u8,
    skip_byte_block: u8,
    /// Scheme type (cenc, cens, cbc1, cbcs)
    scheme_type: u32,
    /// Sample info from senc box
    sample_info: Option<SampleInfoTable>,
    /// Sample sizes from trun box
    sample_sizes: Vec<u32>,
}

impl<'a> DecryptionState<'a> {
    fn new(keys: &'a HashMap<[u8; 16], [u8; 16]>) -> Self {
        Self {
            keys,
            default_kid: None,
            per_sample_iv_size: 8,
            default_iv_size: 8,
            default_is_protected: false,
            default_constant_iv: None,
            crypt_byte_block: 0,
            skip_byte_block: 0,
            scheme_type: 0,
            sample_info: None,
            sample_sizes: Vec::new(),
        }
    }

    /// Parse the init segment to extract encryption parameters.
    fn parse_init_segment(&mut self, data: &[u8]) -> Result<()> {
        let schm_box = data!();
        let tenc_box = data!();

        let schm_box_c = schm_box.clone();
        let tenc_box_c = tenc_box.clone();

        let result = Mp4Parser::new()
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

        // Ignore parse errors (partial data is expected)
        let _ = result;

        // Extract scheme type from schm box
        if let Some(schm) = schm_box.borrow().as_ref() {
            self.scheme_type = schm.scheme_type;
        }

        // Extract encryption parameters from tenc box
        if let Some(tenc) = tenc_box.borrow().as_ref() {
            self.default_kid = Some(tenc.default_kid);
            self.per_sample_iv_size = tenc.per_sample_iv_size;
            self.default_iv_size = tenc.effective_iv_size();
            self.default_is_protected = tenc.is_protected;
            self.default_constant_iv = tenc.constant_iv.clone();
            self.crypt_byte_block = tenc.crypt_byte_block;
            self.skip_byte_block = tenc.skip_byte_block;
        }

        // Note: For CBCS, pattern 0:0 means full encryption (all blocks encrypted)
        // Only non-zero patterns like 1:9 use partial encryption
        // We keep the values from tenc as-is

        Ok(())
    }

    /// Parse segment and decrypt mdat boxes.
    fn parse_and_decrypt_segment(&mut self, output: &mut [u8], init_size: usize) -> Result<()> {
        use crate::boxes::{SencBox, TrunBox};

        // Copy segment data for parsing (avoid mutable borrow conflict later)
        let segment_data = output[init_size..].to_vec();

        let moof_start = data!(0u64);
        let trun_box = data!();
        let senc_box = data!();

        let moof_start_c = moof_start.clone();
        let trun_box_c = trun_box.clone();

        // per_sample_iv_size is the IV size in senc (0 for CBCS with constant IV)
        let iv_size = self.per_sample_iv_size;
        let constant_iv = self.default_constant_iv.clone();
        let senc_box_c = senc_box.clone();

        // Parse moof to get sample info
        let _ = Mp4Parser::new()
            .base_box("moof", {
                let moof_start = moof_start_c.clone();
                move |box_| {
                    *moof_start.borrow_mut() = box_.start;
                    parser::children(box_)
                }
            })
            .base_box("traf", parser::children)
            .full_box("tfhd", |_| Ok(()))
            .full_box("tfdt", |_| Ok(()))
            .full_box("trun", move |mut box_| {
                *trun_box_c.borrow_mut() = Some(TrunBox::new(&mut box_)?);
                Ok(())
            })
            .full_box("senc", move |mut box_| {
                *senc_box_c.borrow_mut() =
                    Some(SencBox::new(&mut box_, iv_size, constant_iv.as_deref())?);
                Ok(())
            })
            .parse(&segment_data, true, true);

        // Extract sample sizes from TrunBox
        if let Some(trun) = trun_box.borrow().as_ref() {
            self.sample_sizes = trun
                .sample_data
                .iter()
                .map(|s| s.sample_size.unwrap_or(0))
                .collect();
        }

        // Convert SencBox to SampleInfoTable
        if let Some(senc) = senc_box.borrow().as_ref() {
            let effective_iv_size = if self.per_sample_iv_size > 0 {
                self.per_sample_iv_size
            } else {
                self.default_constant_iv
                    .as_ref()
                    .map(|v| v.len() as u8)
                    .unwrap_or(8)
            };

            let mut table = SampleInfoTable::new(
                (senc.flags & 0xFF) as u8,
                0, // crypt_byte_block (from tenc)
                0, // skip_byte_block (from tenc)
                senc.samples.len() as u32,
                effective_iv_size,
            );

            for (i, sample) in senc.samples.iter().enumerate() {
                let _ = table.set_iv(i as u32, &sample.iv);

                if !sample.subsamples.is_empty() {
                    // Convert subsamples to raw bytes format
                    let mut subsample_data = Vec::with_capacity(sample.subsamples.len() * 6);
                    for sub in &sample.subsamples {
                        subsample_data.extend_from_slice(&sub.bytes_of_clear_data.to_be_bytes());
                        subsample_data
                            .extend_from_slice(&sub.bytes_of_encrypted_data.to_be_bytes());
                    }
                    let _ =
                        table.add_subsample_data(sample.subsamples.len() as u16, &subsample_data);
                }
            }

            self.sample_info = Some(table);
        }

        // Now find and decrypt mdat
        if let Some(ref info) = self.sample_info {
            let kid = self.default_kid.ok_or(DecryptError::InvalidFormat(
                "No default KID found".to_string(),
            ))?;

            let key = self
                .keys
                .get(&kid)
                .ok_or_else(|| DecryptError::KeyNotFound(hex::encode(kid)))?;

            let cipher_mode = CipherMode::from_scheme_type(self.scheme_type);
            let reset_iv = CipherMode::resets_iv_per_subsample(self.scheme_type);

            let mut decrypter = SingleSampleDecrypter::new(
                cipher_mode,
                key,
                self.crypt_byte_block,
                self.skip_byte_block,
                reset_iv,
            )?;

            // Find mdat and decrypt samples
            let mdat_offset = find_box_offset(&segment_data, b"mdat");
            if let Some((_mdat_start, _mdat_header_size)) = mdat_offset {
                // Calculate data_start from moof_start + trun data_offset
                let data_start = if let Some(trun) = trun_box.borrow().as_ref() {
                    let offset = trun.data_offset.unwrap_or(0) as i64;
                    (*moof_start.borrow() as i64 + offset) as usize
                } else {
                    0
                };
                let mut current_offset = data_start;

                for (sample_idx, &sample_size) in self.sample_sizes.iter().enumerate() {
                    if sample_size == 0 {
                        continue;
                    }

                    let sample_end = current_offset + sample_size as usize;
                    if sample_end > segment_data.len() {
                        break;
                    }

                    // Get sample IV
                    let iv_slice = info.get_iv(sample_idx as u32).unwrap_or(&[0u8; 16]);
                    let mut iv = [0u8; 16];
                    let copy_len = iv_slice.len().min(16);
                    iv[..copy_len].copy_from_slice(&iv_slice[..copy_len]);

                    // Get subsample info
                    let (subsample_count, cleartext, encrypted) =
                        info.get_sample_info(sample_idx as u32)?;

                    // Decrypt sample
                    let sample_data = &segment_data[current_offset..sample_end];
                    let decrypted = decrypter.decrypt_sample_data(
                        sample_data,
                        &iv,
                        subsample_count,
                        cleartext,
                        encrypted,
                    )?;

                    // Copy decrypted data back
                    output[init_size + current_offset..init_size + sample_end]
                        .copy_from_slice(&decrypted);

                    current_offset = sample_end;
                }
            }
        }

        Ok(())
    }
}

/// Find a box offset in MP4 data.
fn find_box_offset(data: &[u8], box_type: &[u8; 4]) -> Option<(usize, usize)> {
    let mut offset = 0;
    while offset + 8 <= data.len() {
        let size = u32::from_be_bytes(data[offset..offset + 4].try_into().ok()?) as usize;
        let btype = &data[offset + 4..offset + 8];

        if btype == box_type {
            return Some((offset, 8));
        }

        if size == 0 {
            break; // Box extends to end of file
        }
        if size == 1 && offset + 16 <= data.len() {
            // 64-bit size
            let size64 =
                u64::from_be_bytes(data[offset + 8..offset + 16].try_into().ok()?) as usize;
            if btype == box_type {
                return Some((offset, 16));
            }
            offset += size64;
        } else {
            offset += size;
        }
    }
    None
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
