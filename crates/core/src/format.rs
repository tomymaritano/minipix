/// Formato de imagen soportado.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Format {
    /// Portable Network Graphics.
    Png,
    /// Joint Photographic Experts Group.
    Jpeg,
    /// WebP format.
    WebP,
    /// AV1 Image File Format.
    Avif,
}
