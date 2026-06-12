//! PNG: decode con el crate `png` normalizado a RGBA8.
use crate::codecs::ImageDecoder;
use crate::error::Error;
use crate::format::Format;
use crate::image::DecodedImage;

/// Codec for decoding PNG images to RGBA8.
#[allow(dead_code)] // instanciado sólo en tests hasta que se conecte al dispatcher
pub(crate) struct PngCodec;

#[allow(dead_code)] // usado dentro del impl y en tests; silenciado hasta Task 8
fn decode_err(e: impl std::fmt::Display) -> Error {
    Error::Decode {
        format: Format::Png,
        detail: e.to_string(),
    }
}

impl ImageDecoder for PngCodec {
    fn decode(&self, data: &[u8]) -> Result<DecodedImage, Error> {
        let mut decoder = png::Decoder::new(std::io::Cursor::new(data));
        // Normaliza: expande paleta/grises/16-bit y agrega alpha.
        decoder.set_transformations(
            png::Transformations::normalize_to_color8() | png::Transformations::ALPHA,
        );
        let mut reader = decoder.read_info().map_err(decode_err)?;
        let buf_size = reader
            .output_buffer_size()
            .ok_or_else(|| decode_err("output too large"))?;
        let mut buf = vec![0u8; buf_size];
        let info = reader.next_frame(&mut buf).map_err(decode_err)?;
        buf.truncate(info.buffer_size());
        // Tras normalize_to_color8|ALPHA el output es RGBA8 o GrayscaleAlpha.
        let rgba = match info.color_type {
            png::ColorType::Rgba => buf,
            png::ColorType::GrayscaleAlpha => buf
                .chunks_exact(2)
                .flat_map(|ga| [ga[0], ga[0], ga[0], ga[1]])
                .collect(),
            other => return Err(decode_err(format!("unexpected color type {other:?}"))),
        };
        DecodedImage::new(info.width, info.height, rgba).map_err(decode_err)
    }
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
        let out = PngCodec.decode(&bytes).unwrap();
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
        let out = PngCodec.decode(&bytes).unwrap();
        assert_eq!(out.pixels, vec![10, 10, 10, 255, 200, 200, 200, 255]);
    }

    #[test]
    fn error_tipado_con_basura() {
        let err = PngCodec.decode(b"\x89PNGgarbage").unwrap_err();
        assert!(matches!(err, crate::Error::Decode { .. }));
    }
}
