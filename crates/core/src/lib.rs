//! minipix core: compresión y conversión de imágenes (PNG, JPEG, WebP, AVIF).

pub mod api;
pub(crate) mod codecs;
/// Conversión de espacio de color ICC → sRGB para decoders.
pub(crate) mod color;
/// Error types for core operations.
pub mod error;
/// Tipos de formato de imagen soportados.
pub mod format;
/// Decoded RGBA8 image representation.
pub mod image;
/// Compression and conversion options.
pub mod options;
pub(crate) mod peek;
/// Detección de formato de imagen por magic bytes.
pub mod sniff;
#[cfg(test)]
pub(crate) mod testutil;

pub use api::{compress, convert};
pub use error::Error;
pub use format::Format;
pub use image::DecodedImage;
pub use options::{Options, Output};
