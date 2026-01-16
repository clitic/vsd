use crate::{
    Result,
    pssh::{KeyId, KeyIdSystemType},
};
use prost::Message;

include!(concat!(env!("OUT_DIR"), "/widevine.rs"));

pub(super) fn parse(data: &[u8]) -> Result<impl IntoIterator<Item = KeyId>> {
    let wv = WidevinePsshData::decode(data)?;

    // let protection_scheme = String::from_utf8(wv.protection_scheme().to_be_bytes().to_vec())?;

    Ok(wv.key_ids.into_iter().map(|x| KeyId {
        system_type: KeyIdSystemType::WideVine,
        value: hex::encode(x),
    }))
}
