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

#[derive(Default)]
pub struct CencDecryptingProcessorBuilder {
    keys: HashMap<[u8; 16], [u8; 16]>,
}

impl CencDecryptingProcessorBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn key(mut self, kid: &str, key: &str) -> Result<Self> {
        self.keys.insert(parse_hex_16(kid)?, parse_hex_16(key)?);
        Ok(self)
    }

    pub fn keys(mut self, keys: &HashMap<String, String>) -> Result<Self> {
        for (kid, key) in keys {
            self.keys.insert(parse_hex_16(kid)?, parse_hex_16(key)?);
        }
        Ok(self)
    }

    pub fn build(self) -> Result<CencDecryptingProcessor> {
        if self.keys.is_empty() {
            return Err(DecryptError::NoKeys);
        }

        Ok(CencDecryptingProcessor { keys: self.keys })
    }
}

pub struct CencDecryptingProcessor {
    keys: HashMap<[u8; 16], [u8; 16]>,
}

impl CencDecryptingProcessor {
    pub fn builder() -> CencDecryptingProcessorBuilder {
        CencDecryptingProcessorBuilder::new()
    }

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

struct DecryptionState<'a> {
    keys: &'a HashMap<[u8; 16], [u8; 16]>,
    scheme_type: u32,
    tenc: Option<TencBox>,
}

impl<'a> DecryptionState<'a> {
    fn new(keys: &'a HashMap<[u8; 16], [u8; 16]>) -> Self {
        Self {
            keys,
            scheme_type: 0,
            tenc: None,
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

        self.tenc = tenc_box.take();
        Ok(())
    }

    fn parse_and_decrypt_segment(&mut self, output: &mut [u8]) -> Result<()> {
        let tenc = match &self.tenc {
            Some(t) => t,
            None => return Ok(()),
        };

        let moof_start = data!(0u64);
        let trun_box = data!();
        let senc_box = data!();

        let iv_size = tenc.per_sample_iv_size;
        let constant_iv = tenc.constant_iv.clone();

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
            .parse(output, true, true);

        let trun_ref = trun_box.borrow();
        let senc_ref = senc_box.borrow();

        let trun = match trun_ref.as_ref() {
            Some(t) => t,
            None => return Ok(()),
        };

        let senc = match senc_ref.as_ref() {
            Some(s) => s,
            None => return Ok(()),
        };

        let kid = tenc.default_kid;
        let key = self
            .keys
            .get(&kid)
            .ok_or_else(|| DecryptError::KeyNotFound(hex::encode(kid)))?;

        let cipher_mode = CipherMode::from_scheme_type(self.scheme_type);
        let reset_iv = CipherMode::resets_iv_per_subsample(self.scheme_type);
        let mut decrypter = SingleSampleDecrypter::new(
            cipher_mode,
            key,
            tenc.crypt_byte_block,
            tenc.skip_byte_block,
            reset_iv,
        )?;

        let data_start = {
            let offset = trun.data_offset.unwrap_or(0) as i64;
            (*moof_start.borrow() as i64 + offset) as usize
        };

        let mut offset = data_start;
        let output_len = output.len();

        for (trun_sample, senc_sample) in trun.sample_data.iter().zip(senc.samples.iter()) {
            let size = trun_sample.sample_size.unwrap_or(0) as usize;
            if size == 0 {
                continue;
            }

            let end = offset + size;
            if end > output_len {
                break;
            }

            let iv = senc_sample.iv_as_array();
            let (subsample_count, clear, encrypted) = senc_sample.subsample_info();

            let encrypted_sample = output[offset..end].to_vec();
            let decrypted = decrypter.decrypt_sample_data(
                &encrypted_sample,
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

fn parse_hex_16(input: &str) -> Result<[u8; 16]> {
    let bytes = hex::decode(input)?;

    if bytes.len() != 16 {
        return Err(DecryptError::HexWrongLength(bytes.len()));
    }

    let mut arr = [0u8; 16];
    arr.copy_from_slice(&bytes);
    Ok(arr)
}
