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
