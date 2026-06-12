//! WebP: decode con image-webp (Rust puro) normalizado a RGBA8.
use crate::codecs::ImageDecoder;
use crate::error::Error;
use crate::format::Format;
use crate::image::DecodedImage;

/// Codec for decoding WebP images.
#[allow(dead_code)] // instanciado sólo en tests hasta que se conecte al dispatcher (Task 15)
pub(crate) struct WebpCodec;

fn decode_err(e: impl std::fmt::Display) -> Error {
    Error::Decode {
        format: Format::WebP,
        detail: e.to_string(),
    }
}

impl ImageDecoder for WebpCodec {
    fn decode(&self, data: &[u8], max_pixels: u64) -> Result<DecodedImage, Error> {
        let mut dec =
            image_webp::WebPDecoder::new(std::io::Cursor::new(data)).map_err(decode_err)?;
        let (w, h) = dec.dimensions();
        // Anti-bomba: validar ANTES de asignar el buffer de salida.
        let pixels = u64::from(w) * u64::from(h);
        if pixels > max_pixels {
            return Err(Error::LimitExceeded {
                pixels,
                limit: max_pixels,
            });
        }
        let size = dec
            .output_buffer_size()
            .ok_or_else(|| decode_err("image too large"))?;
        let mut buf = vec![0u8; size];
        dec.read_image(&mut buf).map_err(decode_err)?;
        let rgba = if dec.has_alpha() {
            buf
        } else {
            buf.chunks_exact(3)
                .flat_map(|rgb| [rgb[0], rgb[1], rgb[2], 255])
                .collect()
        };
        DecodedImage::new(w, h, rgba).map_err(decode_err)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::codecs::ImageDecoder;
    use crate::testutil::vector_gradient_circle;

    #[test]
    fn decodea_webp_lossless_roundtrip() {
        let img = vector_gradient_circle(32, 24);
        let mut bytes = Vec::new();
        image_webp::WebPEncoder::new(std::io::Cursor::new(&mut bytes))
            .encode(&img.pixels, 32, 24, image_webp::ColorType::Rgba8)
            .unwrap();
        let out = WebpCodec.decode(&bytes, u64::MAX).unwrap();
        assert_eq!((out.width, out.height), (32, 24));
        assert_eq!(out.pixels, img.pixels, "lossless roundtrip exacto");
    }

    #[test]
    fn error_tipado_con_basura() {
        let err = WebpCodec
            .decode(b"RIFF\x00\x00\x00\x00WEBPgarbage", u64::MAX)
            .unwrap_err();
        assert!(matches!(err, crate::Error::Decode { .. }));
    }

    #[test]
    fn limite_de_pixeles_se_aplica() {
        let img = vector_gradient_circle(32, 24); // 768 px
        let mut bytes = Vec::new();
        image_webp::WebPEncoder::new(std::io::Cursor::new(&mut bytes))
            .encode(&img.pixels, 32, 24, image_webp::ColorType::Rgba8)
            .unwrap();
        let err = WebpCodec.decode(&bytes, 100).unwrap_err();
        assert!(matches!(
            err,
            crate::Error::LimitExceeded {
                pixels: 768,
                limit: 100
            }
        ));
    }
}
