use crate::{
    Mp4Parser,
    boxes::{SchmBox, SencBox, TencBox, TrunBox},
    data,
    decrypt::{
        cipher::CipherMode,
        decrypter::SingleSampleDecrypter,
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

    pub fn session(&self, init_data: &[u8]) -> Result<DecryptionSession<'_>> {
        DecryptionSession::new(&self.keys, init_data)
    }

    pub fn decrypt<T: AsRef<[u8]>>(&self, input_data: T, init_data: Option<T>) -> Result<Vec<u8>> {
        let input = input_data.as_ref();
        let init = init_data.as_ref().map(|x| x.as_ref());

        if let Some(init) = init {
            self.session(init)?.decrypt(input)
        } else if !input.is_empty() {
            self.session(&input[..input.len().min(1000)])?
                .decrypt(input)
        } else {
            Ok(Vec::new())
        }
    }
}

pub struct DecryptionSession<'a> {
    keys: &'a HashMap<[u8; 16], [u8; 16]>,
    scheme_type: u32,
    tenc: TencBox,
}

impl<'a> DecryptionSession<'a> {
    fn new(keys: &'a HashMap<[u8; 16], [u8; 16]>, init_data: &[u8]) -> Result<Self> {
        let schm_box = data!();
        let tenc_box = data!();

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
            .full_box("schm", {
                let schm_box = schm_box.clone();
                move |mut box_| {
                    *schm_box.borrow_mut() = Some(SchmBox::new(&mut box_)?);
                    Ok(())
                }
            })
            .base_box("schi", parser::children)
            .full_box("tenc", {
                let tenc_box = tenc_box.clone();
                move |mut box_| {
                    *tenc_box.borrow_mut() = Some(TencBox::new(&mut box_)?);
                    Ok(())
                }
            })
            .parse(init_data, true, true);

        let scheme_type = schm_box
            .borrow()
            .as_ref()
            .map(|s| s.scheme_type)
            .unwrap_or_default();

        let tenc = tenc_box
            .take()
            .ok_or_else(|| DecryptError::InvalidFormat("No tenc box found".into()))?;

        Ok(Self {
            keys,
            scheme_type,
            tenc,
        })
    }

    pub fn decrypt(&self, segment_data: &[u8]) -> Result<Vec<u8>> {
        let mut output = segment_data.to_vec();

        let moof_start = data!(0u64);
        let trun_box = data!();
        let senc_box = data!();

        let iv_size = self.tenc.per_sample_iv_size;
        let constant_iv = self.tenc.constant_iv.clone();

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
            .parse(&output, true, true);

        let trun_ref = trun_box.borrow();
        let senc_ref = senc_box.borrow();

        let (trun, senc) = match (trun_ref.as_ref(), senc_ref.as_ref()) {
            (Some(t), Some(s)) => (t, s),
            _ => return Ok(output),
        };

        let kid = self.tenc.default_kid;
        let key = self
            .keys
            .get(&kid)
            .ok_or_else(|| DecryptError::KeyNotFound(hex::encode(kid)))?;

        let cipher_mode = CipherMode::from_scheme_type(self.scheme_type);
        let mut decrypter = SingleSampleDecrypter::new(
            cipher_mode,
            key,
            self.tenc.crypt_byte_block,
            self.tenc.skip_byte_block,
        )?;

        let data_start = {
            let offset = trun.data_offset.unwrap_or_default() as i64;
            (*moof_start.borrow() as i64 + offset) as usize
        };

        let mut offset = data_start;
        let output_len = output.len();

        for (trun_sample, senc_sample) in trun.sample_data.iter().zip(senc.samples.iter()) {
            let size = trun_sample.sample_size.unwrap_or_default() as usize;
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

        Ok(output)
    }
}

fn parse_hex_16(input: &str) -> Result<[u8; 16]> {
    let bytes = hex::decode(input)?;
    bytes
        .try_into()
        .map_err(|v: Vec<u8>| DecryptError::HexWrongLength(v.len()))
}
