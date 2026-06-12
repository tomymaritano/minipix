// crates/core/src/api.rs
//! API pública: `compress` (mismo formato) y `convert` (transcodificación).
use crate::codecs::{ImageDecoder, ImageEncoder};
use crate::codecs::{avif::AvifCodec, jpeg::JpegCodec, png::PngCodec, webp::WebpCodec};
use crate::error::Error;
use crate::format::Format;
use crate::image::DecodedImage;
use crate::options::{Options, Output};
use crate::peek::peek_dimensions;
use crate::sniff::sniff;

/// Re-encodea optimizando en el MISMO formato de entrada.
///
/// # Errors
/// `UnsupportedFormat`, `InvalidOptions`, `LimitExceeded`, `Decode`, `Encode`.
pub fn compress(data: &[u8], opts: &Options) -> Result<Output, Error> {
    let format = sniff(data).ok_or(Error::UnsupportedFormat)?;
    run(data, format, opts)
}

/// Transcodea al formato de `opts.format` (requerido).
///
/// # Errors
/// Como `compress`; además `InvalidOptions` si falta `format`.
pub fn convert(data: &[u8], opts: &Options) -> Result<Output, Error> {
    let target = opts
        .format
        .ok_or_else(|| Error::InvalidOptions("convert requires options.format".into()))?;
    sniff(data).ok_or(Error::UnsupportedFormat)?;
    run(data, target, opts)
}

fn run(data: &[u8], target: Format, opts: &Options) -> Result<Output, Error> {
    opts.validate()?; // SIEMPRE antes de trabajar (ravif hace assert sobre rangos)
    let source = sniff(data).ok_or(Error::UnsupportedFormat)?;
    // Capa rápida: dims del header sin tocar el decoder.
    if let Some((w, h)) = peek_dimensions(source, data) {
        let pixels = u64::from(w) * u64::from(h);
        if pixels > opts.max_pixels {
            return Err(Error::LimitExceeded {
                pixels,
                limit: opts.max_pixels,
            });
        }
    }
    let img = decode(source, data, opts.max_pixels)?;
    let encoded = encode(target, &img, opts)?;
    Ok(Output {
        format: target,
        width: img.width,
        height: img.height,
        bytes_in: data.len() as u64,
        bytes_out: encoded.len() as u64,
        data: encoded,
    })
}

fn decode(format: Format, data: &[u8], max_pixels: u64) -> Result<DecodedImage, Error> {
    match format {
        Format::Png => PngCodec.decode(data, max_pixels),
        Format::Jpeg => JpegCodec.decode(data, max_pixels),
        Format::WebP => WebpCodec.decode(data, max_pixels),
        Format::Avif => AvifCodec.decode(data, max_pixels),
    }
}

fn encode(format: Format, img: &DecodedImage, opts: &Options) -> Result<Vec<u8>, Error> {
    match format {
        Format::Png => PngCodec.encode(img, opts),
        Format::Jpeg => JpegCodec.encode(img, opts),
        Format::WebP => WebpCodec.encode(img, opts),
        Format::Avif => AvifCodec.encode(img, opts),
    }
}
