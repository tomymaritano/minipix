//! minipix core: compresión y conversión de imágenes (PNG, JPEG, WebP, AVIF).
//!
//! # Example
//!
//! ```
//! use minipix_core::{compress, Options};
//! # let mut png_bytes = Vec::new();
//! # {
//! #     let mut enc = png::Encoder::new(&mut png_bytes, 1, 1);
//! #     enc.set_color(png::ColorType::Rgba);
//! #     enc.set_depth(png::BitDepth::Eight);
//! #     let mut w = enc.write_header().unwrap();
//! #     w.write_image_data(&[255, 0, 0, 255]).unwrap();
//! #     w.finish().unwrap();
//! # }
//! # let png: &[u8] = &png_bytes;
//! let out = compress(png, &Options::default())?;
//! assert_eq!(out.format, minipix_core::Format::Png);
//! # Ok::<(), minipix_core::Error>(())
//! ```

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
