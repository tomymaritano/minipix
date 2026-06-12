use crate::format::Format;

/// Error del core. Los bindings lo mapean a errores idiomáticos.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// Unsupported or unrecognized image format.
    #[error("unsupported or unrecognized image format")]
    UnsupportedFormat,

    /// Decode error with format and detail.
    #[error("decode failed ({format:?}): {detail}")]
    Decode {
        /// The image format.
        format: Format,
        /// Error detail.
        detail: String,
    },

    /// Encode error with format and detail.
    #[error("encode failed ({format:?}): {detail}")]
    Encode {
        /// The image format.
        format: Format,
        /// Error detail.
        detail: String,
    },

    /// Invalid options error.
    #[error("invalid options: {0}")]
    InvalidOptions(String),

    /// Image exceeds the configured pixel limit.
    #[error("image has {pixels} pixels, exceeds configured limit of {limit}")]
    LimitExceeded {
        /// Píxeles reales de la imagen.
        pixels: u64,
        /// Límite configurado (`Options::max_pixels`).
        limit: u64,
    },
}
