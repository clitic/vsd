#[cfg(feature = "sidx")]
mod sidx;

#[cfg(feature = "sidx")]
#[cfg_attr(docsrs, doc(cfg(feature = "sidx")))]
pub use sidx::{SidxBox, SidxRange};

#[cfg(feature = "tenc")]
mod tenc;

#[cfg(feature = "tenc")]
#[cfg_attr(docsrs, doc(cfg(feature = "tenc")))]
pub use tenc::TencBox;

#[cfg(feature = "text-vtt")]
mod mdhd;

#[cfg(feature = "text-vtt")]
#[cfg_attr(docsrs, doc(cfg(feature = "text-vtt")))]
pub use mdhd::MdhdBox;

#[cfg(feature = "text-vtt")]
mod tfdt;

#[cfg(feature = "text-vtt")]
#[cfg_attr(docsrs, doc(cfg(feature = "text-vtt")))]
pub use tfdt::TfdtBox;

#[cfg(feature = "text-vtt")]
mod tfhd;

#[cfg(feature = "text-vtt")]
#[cfg_attr(docsrs, doc(cfg(feature = "text-vtt")))]
pub use tfhd::TfhdBox;

#[cfg(feature = "text-vtt")]
mod trun;

#[cfg(feature = "text-vtt")]
#[cfg_attr(docsrs, doc(cfg(feature = "text-vtt")))]
pub use trun::{TrunBox, TrunSample};
