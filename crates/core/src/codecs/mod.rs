//! Códecs por formato, detrás de traits contra `DecodedImage`.
use crate::error::Error;
use crate::image::DecodedImage;
use crate::options::Options;

pub(crate) mod png;

/// Decodifica bytes del formato a RGBA8 sRGB.
///
/// `max_pixels` DEBE validarse contra las dimensiones del header tan temprano
/// como la API del códec lo permita, ANTES de asignar el buffer de salida
/// (defensa contra bombas de dimensiones; ver review de Task 6).
#[allow(dead_code)] // usado por los códecs (Tasks 6-13)
pub(crate) trait ImageDecoder {
    fn decode(&self, data: &[u8], max_pixels: u64) -> Result<DecodedImage, Error>;
}

/// Codifica RGBA8 sRGB a bytes del formato según `Options`.
#[allow(dead_code)] // PngCodec impl exists; dispatcher wiring is a later task
pub(crate) trait ImageEncoder {
    fn encode(&self, img: &DecodedImage, opts: &Options) -> Result<Vec<u8>, Error>;
}
