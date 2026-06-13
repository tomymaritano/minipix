//! PNG: decode con el crate `png` normalizado a RGBA8.
//! PNG encode: lossless via RGBA8 + oxipng; lossy via quantette indexed.
use crate::codecs::{ImageDecoder, ImageEncoder};
use crate::error::Error;
use crate::format::Format;
use crate::image::DecodedImage;
use crate::options::Options;

/// Codec for decoding and encoding PNG images.
pub(crate) struct PngCodec;

fn decode_err(e: impl std::fmt::Display) -> Error {
    Error::Decode {
        format: Format::Png,
        detail: e.to_string(),
    }
}

fn encode_err(e: impl std::fmt::Display) -> Error {
    Error::Encode {
        format: Format::Png,
        detail: e.to_string(),
    }
}

impl ImageDecoder for PngCodec {
    fn decode(&self, data: &[u8], max_pixels: u64) -> Result<DecodedImage, Error> {
        let mut decoder = png::Decoder::new(std::io::Cursor::new(data));
        // Normaliza: expande paleta/grises/16-bit y agrega alpha.
        decoder.set_transformations(
            png::Transformations::normalize_to_color8() | png::Transformations::ALPHA,
        );
        let mut reader = decoder.read_info().map_err(decode_err)?;
        // Defensa contra bombas de dimensiones: validar ANTES de asignar el buffer.
        let info = reader.info();
        let pixels = u64::from(info.width) * u64::from(info.height);
        if pixels > max_pixels {
            return Err(Error::LimitExceeded {
                pixels,
                limit: max_pixels,
            });
        }
        // Extraer ICC antes de next_frame para evitar conflictos de borrow.
        // icc_profile es Option<Cow<[u8]>>; lo clonamos a Vec<u8> propio.
        let icc = info.icc_profile.as_ref().map(|c| c.as_ref().to_vec());
        let buf_size = reader
            .output_buffer_size()
            .ok_or_else(|| decode_err("output too large"))?;
        let mut buf = vec![0u8; buf_size];
        let info = reader.next_frame(&mut buf).map_err(decode_err)?;
        buf.truncate(info.buffer_size());
        // Tras normalize_to_color8|ALPHA el output es RGBA8 o GrayscaleAlpha.
        let mut rgba = match info.color_type {
            png::ColorType::Rgba => buf,
            png::ColorType::GrayscaleAlpha => buf
                .chunks_exact(2)
                .flat_map(|ga| [ga[0], ga[0], ga[0], ga[1]])
                .collect(),
            other => return Err(decode_err(format!("unexpected color type {other:?}"))),
        };
        // wiring probado vía tests de color.rs; e2e con fixture ICC queda en backlog
        crate::color::apply_icc_best_effort(&mut rgba, icc.as_deref());
        DecodedImage::new(info.width, info.height, rgba).map_err(decode_err)
    }
}

/// `quality` 1-100 → tamaño de paleta 8..=256.
fn palette_size(quality: u8) -> u16 {
    ((u16::from(quality) * 256) / 100).clamp(8, 256)
}

/// `effort` 0-9 → preset oxipng 0..=6 (7+ activa zopfli).
// NUNCA setear Options.timeout: usa Instant::now() que trappea en wasm32 (plan M2).
fn oxipng_options(effort: u8) -> oxipng::Options {
    let mut o = oxipng::Options::from_preset(effort.min(6));
    if effort >= 7 {
        o.deflater = oxipng::Deflater::Zopfli(oxipng::ZopfliOptions::default());
    }
    o.strip = oxipng::StripChunks::Safe;
    o
}

impl ImageEncoder for PngCodec {
    fn encode(&self, img: &DecodedImage, opts: &Options) -> Result<Vec<u8>, Error> {
        // Lossy + transparente → fallback a lossless (quantette sólo RGB).
        let raw = if opts.lossless || opts.quality == 100 || img.has_transparency() {
            encode_rgba_png(img)?
        } else {
            encode_indexed_png(img, palette_size(opts.quality))?
        };
        oxipng::optimize_from_memory(&raw, &oxipng_options(opts.effort)).map_err(encode_err)
    }
}

fn encode_rgba_png(img: &DecodedImage) -> Result<Vec<u8>, Error> {
    let mut out = Vec::new();
    let mut enc = png::Encoder::new(&mut out, img.width, img.height);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.set_source_srgb(png::SrgbRenderingIntent::Perceptual);
    let mut w = enc.write_header().map_err(encode_err)?;
    w.write_image_data(&img.pixels).map_err(encode_err)?;
    w.finish().map_err(encode_err)?;
    Ok(out)
}

/// Cuantiza con quantette a paleta ≤256 + escribe PNG indexado opaco.
/// Sólo se llama para imágenes OPACAS (alpha=255 siempre); escribe PLTE sin tRNS.
fn encode_indexed_png(img: &DecodedImage, max_colors: u16) -> Result<Vec<u8>, Error> {
    let (palette, indices) = quantize_rgba(img, max_colors)?;
    let mut out = Vec::new();
    let mut enc = png::Encoder::new(&mut out, img.width, img.height);
    enc.set_color(png::ColorType::Indexed);
    enc.set_depth(png::BitDepth::Eight);
    enc.set_source_srgb(png::SrgbRenderingIntent::Perceptual);
    let plte: Vec<u8> = palette.iter().flat_map(|c| [c[0], c[1], c[2]]).collect();
    enc.set_palette(plte);
    let mut w = enc.write_header().map_err(encode_err)?;
    w.write_image_data(&indices).map_err(encode_err)?;
    w.finish().map_err(encode_err)?;
    Ok(out)
}

/// Cuantiza la imagen (RGBA opaca) a paleta RGB ≤`max_colors` + índices u8 por píxel.
/// Usa quantette 0.6 Pipeline con Wu quantization y Floyd-Steinberg dithering.
fn quantize_rgba(img: &DecodedImage, max_colors: u16) -> Result<(Vec<[u8; 4]>, Vec<u8>), Error> {
    use quantette::deps::palette::Srgb;
    use quantette::{ImageRef, Pipeline, dither::FloydSteinberg};

    // Extraer canales RGB desde píxeles RGBA (imagen opaca: alpha siempre 255).
    let rgb_pixels: Vec<Srgb<u8>> = img
        .pixels
        .chunks_exact(4)
        .map(|px| Srgb::new(px[0], px[1], px[2]))
        .collect();

    let palette_sz = quantette::PaletteSize::try_from(max_colors)
        .map_err(|e| encode_err(format!("invalid palette size {max_colors}: {e}")))?;

    let image_ref = ImageRef::new(img.width, img.height, &rgb_pixels)
        .map_err(|e| encode_err(format!("quantette image ref: {e}")))?;

    // parallel debe quedar en false: el dither serial es la garantía de determinismo byte a byte.
    let indexed: quantette::IndexedImage<Srgb<u8>> = Pipeline::new()
        .palette_size(palette_sz)
        .ditherer(FloydSteinberg::new())
        .input_image(image_ref)
        .output_srgb8_indexed_image();

    let (pal, idx) = indexed.into_parts();
    // Convertir paleta Srgb<u8> → [u8;4] (alpha 255 para imagen opaca).
    let palette_rgba: Vec<[u8; 4]> = pal
        .iter()
        .map(|c| [c.red, c.green, c.blue, 255u8])
        .collect();

    Ok((palette_rgba, idx))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::codecs::ImageDecoder;
    use crate::testutil::vector_gradient_circle;

    /// Encodea con el crate png "a mano" y decodea con nuestro `PngCodec`.
    #[test]
    fn decodea_rgba8_roundtrip() {
        let img = vector_gradient_circle(32, 24);
        let mut bytes = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut bytes, img.width, img.height);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            let mut w = enc.write_header().unwrap();
            w.write_image_data(&img.pixels).unwrap();
        }
        let out = PngCodec.decode(&bytes, u64::MAX).unwrap();
        assert_eq!((out.width, out.height), (32, 24));
        assert_eq!(out.pixels, img.pixels);
    }

    #[test]
    fn decodea_grayscale_a_rgba() {
        let mut bytes = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut bytes, 2, 1);
            enc.set_color(png::ColorType::Grayscale);
            enc.set_depth(png::BitDepth::Eight);
            let mut w = enc.write_header().unwrap();
            w.write_image_data(&[10, 200]).unwrap();
        }
        let out = PngCodec.decode(&bytes, u64::MAX).unwrap();
        assert_eq!(out.pixels, vec![10, 10, 10, 255, 200, 200, 200, 255]);
    }

    #[test]
    fn error_tipado_con_basura() {
        let err = PngCodec.decode(b"\x89PNGgarbage", u64::MAX).unwrap_err();
        assert!(matches!(err, crate::Error::Decode { .. }));
    }

    #[test]
    fn limite_de_pixeles_antes_de_asignar() {
        // PNG real de 100x100 (10k px), límite 99 px → LimitExceeded, no Decode.
        let img = vector_gradient_circle(100, 100);
        let mut bytes = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut bytes, 100, 100);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            let mut w = enc.write_header().unwrap();
            w.write_image_data(&img.pixels).unwrap();
        }
        let err = PngCodec.decode(&bytes, 99).unwrap_err();
        assert!(matches!(
            err,
            crate::Error::LimitExceeded {
                pixels: 10_000,
                limit: 99
            }
        ));
    }

    #[test]
    fn decodea_grayscale_alpha_preserva_alpha() {
        let mut bytes = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut bytes, 2, 1);
            enc.set_color(png::ColorType::GrayscaleAlpha);
            enc.set_depth(png::BitDepth::Eight);
            let mut w = enc.write_header().unwrap();
            w.write_image_data(&[10, 128, 200, 255]).unwrap();
        }
        let out = PngCodec.decode(&bytes, u64::MAX).unwrap();
        assert_eq!(out.pixels, vec![10, 10, 10, 128, 200, 200, 200, 255]);
    }

    #[test]
    fn encode_lossless_roundtrip_exacto() {
        let img = crate::testutil::vector_gradient_circle(48, 32);
        let opts = crate::Options::default().with_lossless(true);
        let bytes = PngCodec.encode(&img, &opts).unwrap();
        let back = PngCodec.decode(&bytes, u64::MAX).unwrap();
        assert_eq!(back.pixels, img.pixels, "lossless debe ser bit-exacto");
    }

    #[test]
    fn encode_lossy_reduce_y_decodea() {
        let img = crate::testutil::vector_flat_colors(64, 64);
        let lossless = PngCodec
            .encode(&img, &crate::Options::default().with_lossless(true))
            .unwrap();
        let lossy = PngCodec
            .encode(&img, &crate::Options::default().with_quality(60))
            .unwrap();
        assert!(PngCodec.decode(&lossy, u64::MAX).is_ok());
        assert!(
            lossy.len() <= lossless.len(),
            "paleta no debe ser mayor que lossless en imagen plana"
        );
    }

    #[test]
    fn effort_alto_no_es_mayor() {
        let img = crate::testutil::vector_gradient_circle(48, 32);
        let e1 = PngCodec
            .encode(
                &img,
                &crate::Options::default().with_lossless(true).with_effort(1),
            )
            .unwrap();
        let e9 = PngCodec
            .encode(
                &img,
                &crate::Options::default().with_lossless(true).with_effort(9),
            )
            .unwrap();
        assert!(e9.len() <= e1.len());
    }
}
