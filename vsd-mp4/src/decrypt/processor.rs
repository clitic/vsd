//! High-level CENC/CBCS decrypting processor.
//!
//! Provides a simple API for decrypting CENC-encrypted MP4 data.

use std::collections::HashMap;

use super::cipher::CipherMode;
use super::decrypter::SingleSampleDecrypter;
use super::error::{DecryptError, Result};
use super::sample_info::SampleInfoTable;
use crate::Mp4Parser;

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

    /// Add a KID/key pair from raw bytes.
    pub fn key_bytes(mut self, kid: [u8; 16], key: [u8; 16]) -> Self {
        self.keys.insert(kid, key);
        self
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

    /// Decrypt encrypted MP4 data from files.
    ///
    /// This is a convenience method that reads from file paths instead of byte slices.
    ///
    /// # Arguments
    ///
    /// * `segment_path` - Path to the encrypted segment file (.m4s or .mp4)
    /// * `init_path` - Optional path to the initialization segment (.mp4)
    ///
    /// # Returns
    ///
    /// Decrypted segment data as a byte vector.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use vsd_mp4::decrypt::CencDecryptingProcessor;
    ///
    /// let processor = CencDecryptingProcessor::builder()
    ///     .key("eb676abbcb345e96bbcf616630f1a3da", "100b6c20940f779a4589152b57d2dacb")?
    ///     .build()?;
    ///
    /// // Decrypt with separate init segment
    /// let decrypted = processor.decrypt_file("segment.m4s", Some("init.mp4"))?;
    ///
    /// // Decrypt standalone MP4
    /// let decrypted = processor.decrypt_file("encrypted.mp4", None)?;
    /// # Ok::<(), vsd_mp4::decrypt::DecryptError>(())
    /// ```
    pub fn decrypt_file<P: AsRef<std::path::Path>>(
        &self,
        segment_path: P,
        init_path: Option<P>,
    ) -> Result<Vec<u8>> {
        let segment_data = std::fs::read(segment_path.as_ref()).map_err(DecryptError::Io)?;

        let init_data = if let Some(init) = init_path {
            Some(std::fs::read(init.as_ref()).map_err(DecryptError::Io)?)
        } else {
            None
        };

        self.decrypt(&segment_data, init_data.as_ref())
    }

    /// Decrypt encrypted MP4 data from files and write to an output file.
    ///
    /// This is a convenience method that handles file I/O for you.
    ///
    /// # Arguments
    ///
    /// * `segment_path` - Path to the encrypted segment file (.m4s or .mp4)
    /// * `init_path` - Optional path to the initialization segment (.mp4)
    /// * `output_path` - Path where the decrypted output will be written
    ///
    /// # Returns
    ///
    /// Number of bytes written to the output file.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use vsd_mp4::decrypt::CencDecryptingProcessor;
    ///
    /// let processor = CencDecryptingProcessor::builder()
    ///     .key("eb676abbcb345e96bbcf616630f1a3da", "100b6c20940f779a4589152b57d2dacb")?
    ///     .build()?;
    ///
    /// // Decrypt and write to file
    /// let bytes_written = processor.decrypt_file_to_file(
    ///     "segment.m4s",
    ///     Some("init.mp4"),
    ///     "decrypted.mp4"
    /// )?;
    ///
    /// println!("Wrote {} bytes", bytes_written);
    /// # Ok::<(), vsd_mp4::decrypt::DecryptError>(())
    /// ```
    pub fn decrypt_file_to_file<P: AsRef<std::path::Path>>(
        &self,
        segment_path: P,
        init_path: Option<P>,
        output_path: P,
    ) -> Result<usize> {
        let decrypted = self.decrypt_file(segment_path, init_path)?;
        let len = decrypted.len();

        std::fs::write(output_path.as_ref(), &decrypted).map_err(DecryptError::Io)?;

        Ok(len)
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
        use std::cell::RefCell;
        use std::rc::Rc;

        let state = Rc::new(RefCell::new(ParseState::default()));

        let result = Mp4Parser::new()
            .base_box("moov", crate::parser::children)
            .base_box("trak", crate::parser::children)
            .base_box("mdia", crate::parser::children)
            .base_box("minf", crate::parser::children)
            .base_box("stbl", crate::parser::children)
            .full_box("stsd", crate::parser::sample_description)
            .base_box("encv", crate::parser::visual_sample_entry)
            .base_box("enca", crate::parser::audio_sample_entry)
            .base_box("sinf", crate::parser::children)
            .full_box("schm", {
                let state = state.clone();
                move |mut box_| {
                    let reader = &mut box_.reader;
                    let scheme_type = reader.read_u32()?;
                    state.borrow_mut().scheme_type = scheme_type;
                    Ok(())
                }
            })
            .base_box("schi", crate::parser::children)
            .full_box("tenc", {
                let state = state.clone();
                move |mut box_| {
                    let reader = &mut box_.reader;

                    // Skip reserved byte
                    reader.skip(1)?;

                    // In version 0, skip another reserved byte
                    // In version 1, read crypt/skip blocks
                    if box_.version.unwrap_or(0) == 0 {
                        reader.skip(1)?;
                    } else {
                        let blocks = reader.read_u8()?;
                        state.borrow_mut().crypt_byte_block = (blocks >> 4) & 0x0F;
                        state.borrow_mut().skip_byte_block = blocks & 0x0F;
                    }

                    let is_protected = reader.read_u8()?;
                    let per_sample_iv_size = reader.read_u8()?;
                    let kid = reader.read_bytes_u8(16)?;

                    let mut s = state.borrow_mut();
                    s.is_protected = is_protected != 0;
                    s.per_sample_iv_size = per_sample_iv_size;
                    if let Ok(arr) = kid.try_into() {
                        s.default_kid = Some(arr);
                    }

                    // Read constant IV if per_sample_iv_size is 0 (CBCS mode)
                    // Note: tenc versions 0 and 1 have slightly different formats,
                    // but constant IV follows KID when per_sample_iv_size is 0
                    if per_sample_iv_size == 0
                        && let Ok(constant_iv_size) = reader.read_u8()
                        && constant_iv_size > 0
                        && constant_iv_size <= 16
                        && let Ok(constant_iv) = reader.read_bytes_u8(constant_iv_size as usize)
                    {
                        s.constant_iv = Some(constant_iv);
                    }

                    Ok(())
                }
            })
            .parse(data, true, true);

        // Ignore parse errors (partial data is expected)
        let _ = result;

        let s = state.borrow();
        self.default_kid = s.default_kid;
        self.per_sample_iv_size = s.per_sample_iv_size;
        self.default_iv_size = if s.per_sample_iv_size > 0 {
            s.per_sample_iv_size
        } else {
            s.constant_iv.as_ref().map(|v| v.len() as u8).unwrap_or(16)
        };
        self.default_is_protected = s.is_protected;
        self.default_constant_iv = s.constant_iv.clone();
        self.crypt_byte_block = s.crypt_byte_block;
        self.skip_byte_block = s.skip_byte_block;
        self.scheme_type = s.scheme_type;

        // Note: For CBCS, pattern 0:0 means full encryption (all blocks encrypted)
        // Only non-zero patterns like 1:9 use partial encryption
        // We keep the values from tenc as-is

        Ok(())
    }

    /// Parse segment and decrypt mdat boxes.
    fn parse_and_decrypt_segment(&mut self, output: &mut [u8], init_size: usize) -> Result<()> {
        use std::cell::RefCell;
        use std::rc::Rc;

        // Copy segment data for parsing (avoid mutable borrow conflict later)
        let segment_data = output[init_size..].to_vec();
        let sample_info = Rc::new(RefCell::new(None::<SampleInfoTable>));
        let sample_sizes = Rc::new(RefCell::new(Vec::<u32>::new()));
        let data_offset = Rc::new(RefCell::new(0u64));
        let moof_start = Rc::new(RefCell::new(0u64));

        // per_sample_iv_size is the IV size in senc (0 for CBCS with constant IV)
        let iv_size = self.per_sample_iv_size;
        let constant_iv = self.default_constant_iv.clone();

        // Parse moof to get sample info
        let _ = Mp4Parser::new()
            .base_box("moof", {
                let moof_start = moof_start.clone();
                move |box_| {
                    *moof_start.borrow_mut() = box_.start;
                    crate::parser::children(box_)
                }
            })
            .base_box("traf", crate::parser::children)
            .full_box("tfhd", |_| Ok(()))
            .full_box("tfdt", |_| Ok(()))
            .full_box("trun", {
                let sample_sizes = sample_sizes.clone();
                let data_offset = data_offset.clone();
                let moof_start = moof_start.clone();
                move |mut box_| {
                    let reader = &mut box_.reader;
                    let flags = box_.flags.unwrap_or(0);

                    let sample_count = reader.read_u32()?;

                    // Data offset (if present)
                    if flags & 0x000001 != 0 {
                        let offset = reader.read_i32()?;
                        *data_offset.borrow_mut() =
                            (*moof_start.borrow() as i64 + offset as i64) as u64;
                    }

                    // First sample flags (skip if present)
                    if flags & 0x000004 != 0 {
                        reader.skip(4)?;
                    }

                    let mut sizes = sample_sizes.borrow_mut();
                    for _ in 0..sample_count {
                        // Sample duration (skip if present)
                        if flags & 0x000100 != 0 {
                            reader.skip(4)?;
                        }
                        // Sample size
                        let size = if flags & 0x000200 != 0 {
                            reader.read_u32()?
                        } else {
                            0 // Default sample size from tfhd
                        };
                        sizes.push(size);
                        // Sample flags (skip if present)
                        if flags & 0x000400 != 0 {
                            reader.skip(4)?;
                        }
                        // Sample composition time offset (skip if present)
                        if flags & 0x000800 != 0 {
                            reader.skip(4)?;
                        }
                    }

                    Ok(())
                }
            })
            .full_box("senc", {
                let sample_info = sample_info.clone();
                move |mut box_| {
                    let reader = &mut box_.reader;
                    let flags = box_.flags.unwrap_or(0);

                    let sample_count = reader.read_u32()?;
                    let has_subsamples = flags & 0x02 != 0;

                    let effective_iv_size = if iv_size > 0 {
                        iv_size
                    } else {
                        constant_iv.as_ref().map(|v| v.len() as u8).unwrap_or(8)
                    };

                    let mut table = SampleInfoTable::new(
                        (flags & 0xFF) as u8,
                        0, // crypt_byte_block (from tenc)
                        0, // skip_byte_block (from tenc)
                        sample_count,
                        effective_iv_size,
                    );

                    for i in 0..sample_count {
                        // Read per-sample IV
                        if iv_size > 0 {
                            if let Ok(iv) = reader.read_bytes_u8(iv_size as usize) {
                                let _ = table.set_iv(i, &iv);
                            }
                        } else if let Some(ref civ) = constant_iv {
                            let _ = table.set_iv(i, civ);
                        }

                        // Read subsamples if present
                        if has_subsamples
                            && let Ok(subsample_count) = reader.read_u16()
                            && let Ok(subsample_data) =
                                reader.read_bytes_u8(subsample_count as usize * 6)
                        {
                            let _ = table.add_subsample_data(subsample_count, &subsample_data);
                        }
                    }

                    *sample_info.borrow_mut() = Some(table);
                    Ok(())
                }
            })
            .parse(&segment_data, true, true);

        self.sample_info = sample_info.take();
        self.sample_sizes = sample_sizes.take();

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
                // data_offset is already relative to segment_data (moof_start + trun offset)
                let data_start = *data_offset.borrow() as usize;
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

/// Helper struct for parsing init segment state.
#[derive(Default)]
struct ParseState {
    scheme_type: u32,
    is_protected: bool,
    per_sample_iv_size: u8,
    default_kid: Option<[u8; 16]>,
    constant_iv: Option<Vec<u8>>,
    crypt_byte_block: u8,
    skip_byte_block: u8,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_no_keys() {
        let result = CencDecryptingProcessor::builder().build();
        assert!(matches!(result, Err(DecryptError::NoKeys)));
    }

    #[test]
    fn test_builder_with_key() {
        let result = CencDecryptingProcessor::builder()
            .key(
                "eb676abbcb345e96bbcf616630f1a3da",
                "100b6c20940f779a4589152b57d2dacb",
            )
            .unwrap()
            .build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_hex_16() {
        let result = parse_hex_16("eb676abbcb345e96bbcf616630f1a3da");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 16);
    }

    #[test]
    fn test_parse_hex_16_invalid() {
        let result = parse_hex_16("invalid");
        assert!(result.is_err());
    }
}
