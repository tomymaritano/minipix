//! minipix core: compresión y conversión de imágenes (PNG, JPEG, WebP, AVIF).

/// Tipos de formato de imagen soportados.
pub mod format;
/// Detección de formato de imagen por magic bytes.
pub mod sniff;
pub use format::Format;
