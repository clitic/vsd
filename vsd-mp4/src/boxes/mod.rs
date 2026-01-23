#[cfg(feature = "decrypt")]
mod schm;

#[cfg(feature = "decrypt")]
#[cfg_attr(docsrs, doc(cfg(feature = "decrypt")))]
pub use schm::SchmBox;

#[cfg(feature = "decrypt")]
mod senc;

#[cfg(feature = "decrypt")]
#[cfg_attr(docsrs, doc(cfg(feature = "decrypt")))]
pub use senc::{SencBox, SencSample, SencSubsample};

#[cfg(feature = "decrypt")]
mod tenc;

#[cfg(feature = "decrypt")]
#[cfg_attr(docsrs, doc(cfg(feature = "decrypt")))]
pub use tenc::TencBox;

#[cfg(feature = "sidx")]
mod sidx;

#[cfg(feature = "sidx")]
#[cfg_attr(docsrs, doc(cfg(feature = "sidx")))]
pub use sidx::{SidxBox, SidxRange};

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

#[cfg(any(feature = "decrypt", feature = "text-vtt"))]
mod trun;

#[cfg(any(feature = "decrypt", feature = "text-vtt"))]
#[cfg_attr(docsrs, doc(cfg(any(feature = "decrypt", feature = "text-vtt"))))]
pub use trun::{TrunBox, TrunSample};

#[macro_export]
macro_rules! data {
    () => {
        std::rc::Rc::new(std::cell::RefCell::new(None))
    };
    ($val:expr) => {
        std::rc::Rc::new(std::cell::RefCell::new($val))
    };
}
