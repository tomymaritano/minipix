//! minipix core: compresión y conversión de imágenes (PNG, JPEG, WebP, AVIF).

/// Error types for core operations.
pub mod error;
/// Tipos de formato de imagen soportados.
pub mod format;
/// Compression and conversion options.
pub mod options;
/// Detección de formato de imagen por magic bytes.
pub mod sniff;

pub use error::Error;
pub use format::Format;
pub use options::{Options, Output};
