#![warn(clippy::nursery)]

pub mod heic;
pub mod heif;
mod impl_read;

pub use heic::HeicDecoder;
pub use heif::HeifReader;
