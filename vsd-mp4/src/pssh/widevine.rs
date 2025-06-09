use super::{KeyId, KeyIdSystemType};
use crate::{Error, Result};
use prost::Message;

include!(concat!(env!("OUT_DIR"), "/widevine.rs"));

pub(super) fn parse(data: &[u8]) -> Result<impl IntoIterator<Item = KeyId>> {
    let wv = WidevinePsshData::decode(data).map_err(|x| {
        Error::new_decode(format!("PSSH box data as valid widevine data.\n\n{:#?}", x))
    })?;

    // let protection_scheme = String::from_utf8(wv.protection_scheme().to_be_bytes().to_vec())
    //     .map_err(|_| {
    //         Error::new_decode_err(
    //             "PSSH box widevine protection_scheme as valid utf-8 data (big endian)",
    //         )
    //     })?;

    Ok(wv.key_ids.into_iter().map(|x| KeyId {
        system_type: KeyIdSystemType::WideVine,
        value: hex::encode(x),
    }))
}
