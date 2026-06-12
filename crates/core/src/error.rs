use crate::format::Format;

/// Error del core. Los bindings lo mapean a errores idiomáticos.
#[derive(Debug, thiserror::Error)]
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
    #[error("image exceeds the configured pixel limit ({0} pixels)")]
    LimitExceeded(u64),
}
