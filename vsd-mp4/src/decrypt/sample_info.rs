//! Sample encryption information for CENC/CBCS decryption.
//!
//! Tracks per-sample IVs and subsample encryption maps.

use super::error::{DecryptError, Result};

/// Entry describing a subsample's cleartext and encrypted portions.
#[derive(Debug, Clone, Copy)]
pub struct SubsampleEntry {
    /// Number of cleartext bytes at the start of the subsample.
    pub bytes_of_cleartext_data: u16,
    /// Number of encrypted bytes following the cleartext.
    pub bytes_of_encrypted_data: u32,
}

/// Sample encryption information table.
///
/// Stores per-sample IVs and subsample encryption maps needed for decryption.
#[derive(Debug, Clone)]
pub struct SampleInfoTable {
    /// Number of samples.
    sample_count: u32,
    /// Flags from the sample encryption box.
    flags: u8,
    /// Number of encrypted 16-byte blocks in pattern encryption.
    crypt_byte_block: u8,
    /// Number of unencrypted 16-byte blocks in pattern encryption.
    skip_byte_block: u8,
    /// Size of the IV in bytes (8 or 16).
    iv_size: u8,
    /// IV data for all samples (iv_size * sample_count bytes).
    iv_data: Vec<u8>,
    /// Subsample cleartext byte counts.
    bytes_of_cleartext_data: Vec<u16>,
    /// Subsample encrypted byte counts.
    bytes_of_encrypted_data: Vec<u32>,
    /// Start indices into subsample arrays for each sample.
    subsample_map_starts: Vec<usize>,
    /// Number of subsamples for each sample.
    subsample_map_lengths: Vec<usize>,
}

impl SampleInfoTable {
    /// Create a new sample info table.
    pub fn new(
        flags: u8,
        crypt_byte_block: u8,
        skip_byte_block: u8,
        sample_count: u32,
        iv_size: u8,
    ) -> Self {
        let effective_count = if sample_count == 0 { 1 } else { sample_count };
        let iv_data = vec![0u8; (iv_size as usize) * (effective_count as usize)];

        Self {
            sample_count,
            flags,
            crypt_byte_block,
            skip_byte_block,
            iv_size,
            iv_data,
            bytes_of_cleartext_data: Vec::new(),
            bytes_of_encrypted_data: Vec::new(),
            subsample_map_starts: Vec::new(),
            subsample_map_lengths: Vec::new(),
        }
    }

    /// Get the number of samples.
    pub fn sample_count(&self) -> u32 {
        self.sample_count
    }

    /// Get the IV size in bytes.
    pub fn iv_size(&self) -> u8 {
        self.iv_size
    }

    /// Get the crypt byte block count for pattern encryption.
    pub fn crypt_byte_block(&self) -> u8 {
        self.crypt_byte_block
    }

    /// Get the skip byte block count for pattern encryption.
    pub fn skip_byte_block(&self) -> u8 {
        self.skip_byte_block
    }

    /// Get the flags.
    pub fn flags(&self) -> u8 {
        self.flags
    }

    /// Set the IV for a sample.
    pub fn set_iv(&mut self, sample_index: u32, iv: &[u8]) -> Result<()> {
        let effective_index = if self.sample_count == 0 {
            if sample_index != 0 {
                return Err(DecryptError::SampleIndexOutOfRange {
                    index: sample_index as usize,
                    count: 1,
                });
            }
            0
        } else {
            if sample_index >= self.sample_count {
                return Err(DecryptError::SampleIndexOutOfRange {
                    index: sample_index as usize,
                    count: self.sample_count as usize,
                });
            }
            sample_index as usize
        };

        let iv_size = self.iv_size as usize;
        let offset = effective_index * iv_size;
        let copy_len = iv.len().min(iv_size);
        self.iv_data[offset..offset + copy_len].copy_from_slice(&iv[..copy_len]);

        Ok(())
    }

    /// Get the IV for a sample.
    pub fn get_iv(&self, sample_index: u32) -> Option<&[u8]> {
        let iv_size = self.iv_size as usize;

        if self.sample_count == 0 {
            // Constant IV for all samples
            return Some(&self.iv_data[..iv_size]);
        }

        if sample_index >= self.sample_count {
            return None;
        }

        let offset = (sample_index as usize) * iv_size;
        Some(&self.iv_data[offset..offset + iv_size])
    }

    /// Add subsample data for a sample.
    pub fn add_subsample_data(&mut self, subsample_count: u16, data: &[u8]) -> Result<()> {
        let current = self.subsample_map_starts.len();
        let start = if current == 0 {
            0
        } else {
            self.subsample_map_starts[current - 1] + self.subsample_map_lengths[current - 1]
        };

        self.subsample_map_starts.push(start);
        self.subsample_map_lengths.push(subsample_count as usize);

        if data.len() < (subsample_count as usize) * 6 {
            return Err(DecryptError::SubsampleError(format!(
                "insufficient subsample data: need {} bytes, got {}",
                subsample_count * 6,
                data.len()
            )));
        }

        for i in 0..subsample_count as usize {
            let offset = i * 6;
            let cleartext = u16::from_be_bytes([data[offset], data[offset + 1]]);
            let encrypted = u32::from_be_bytes([
                data[offset + 2],
                data[offset + 3],
                data[offset + 4],
                data[offset + 5],
            ]);
            self.bytes_of_cleartext_data.push(cleartext);
            self.bytes_of_encrypted_data.push(encrypted);
        }

        Ok(())
    }

    /// Check if the table has subsample information.
    pub fn has_subsample_info(&self) -> bool {
        !self.subsample_map_starts.is_empty()
    }

    /// Get the number of subsamples for a sample.
    pub fn get_subsample_count(&self, sample_index: u32) -> usize {
        if sample_index as usize >= self.subsample_map_lengths.len() {
            return 0;
        }
        self.subsample_map_lengths[sample_index as usize]
    }

    /// Get sample info (subsample count and pointers to subsample data).
    pub fn get_sample_info(&self, sample_index: u32) -> Result<(usize, &[u16], &[u32])> {
        if self.sample_count == 0 {
            // All samples fully encrypted, no subsamples
            return Ok((0, &[], &[]));
        }

        if sample_index >= self.sample_count {
            return Err(DecryptError::SampleIndexOutOfRange {
                index: sample_index as usize,
                count: self.sample_count as usize,
            });
        }

        if self.subsample_map_starts.is_empty() {
            // No subsamples
            return Ok((0, &[], &[]));
        }

        let idx = sample_index as usize;

        // Check if we have subsample info for this sample
        if idx >= self.subsample_map_lengths.len() {
            // No subsample info for this sample (full sample encryption)
            return Ok((0, &[], &[]));
        }

        let subsample_count = self.subsample_map_lengths[idx];
        if subsample_count == 0 {
            return Ok((0, &[], &[]));
        }

        let start = self.subsample_map_starts[idx];
        let end = start + subsample_count;

        Ok((
            subsample_count,
            &self.bytes_of_cleartext_data[start..end],
            &self.bytes_of_encrypted_data[start..end],
        ))
    }

    /// Get a specific subsample entry.
    pub fn get_subsample_entry(
        &self,
        sample_index: u32,
        subsample_index: u32,
    ) -> Result<SubsampleEntry> {
        let (count, cleartext, encrypted) = self.get_sample_info(sample_index)?;

        if subsample_index as usize >= count {
            return Err(DecryptError::SampleIndexOutOfRange {
                index: subsample_index as usize,
                count,
            });
        }

        let idx = subsample_index as usize;
        Ok(SubsampleEntry {
            bytes_of_cleartext_data: cleartext[idx],
            bytes_of_encrypted_data: encrypted[idx],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sample_info_table_basic() {
        let mut table = SampleInfoTable::new(0, 0, 0, 2, 8);

        // Set IVs
        table.set_iv(0, &[1, 2, 3, 4, 5, 6, 7, 8]).unwrap();
        table.set_iv(1, &[9, 10, 11, 12, 13, 14, 15, 16]).unwrap();

        // Get IVs
        assert_eq!(table.get_iv(0), Some(&[1, 2, 3, 4, 5, 6, 7, 8][..]));
        assert_eq!(table.get_iv(1), Some(&[9, 10, 11, 12, 13, 14, 15, 16][..]));
        assert_eq!(table.get_iv(2), None);
    }

    #[test]
    fn test_sample_info_table_subsamples() {
        let mut table = SampleInfoTable::new(0, 0, 0, 1, 8);

        // Add subsample data (2 subsamples)
        // Subsample 1: 100 clear, 200 encrypted
        // Subsample 2: 50 clear, 150 encrypted
        let data = [
            0, 100, // cleartext 1
            0, 0, 0, 200, // encrypted 1
            0, 50, // cleartext 2
            0, 0, 0, 150, // encrypted 2
        ];
        table.add_subsample_data(2, &data).unwrap();

        let (count, cleartext, encrypted) = table.get_sample_info(0).unwrap();
        assert_eq!(count, 2);
        assert_eq!(cleartext, &[100, 50]);
        assert_eq!(encrypted, &[200, 150]);
    }

    #[test]
    fn test_constant_iv() {
        let mut table = SampleInfoTable::new(0, 0, 0, 0, 16);

        // sample_count == 0 means constant IV
        table
            .set_iv(0, &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16])
            .unwrap();

        // Should return the same IV for any sample index check
        let iv = table.get_iv(0).unwrap();
        assert_eq!(iv, &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
    }
}
