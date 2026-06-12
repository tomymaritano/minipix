//! JPEG: decode con zune-jpeg normalizado a RGBA8.
use crate::codecs::ImageDecoder;
use crate::error::Error;
use crate::format::Format;
use crate::image::DecodedImage;

/// Codec for decoding JPEG images.
#[allow(dead_code)] // instanciado sólo en tests hasta que se conecte al dispatcher (Task 15)
pub(crate) struct JpegCodec;

fn decode_err(e: impl std::fmt::Display) -> Error {
    Error::Decode {
        format: Format::Jpeg,
        detail: e.to_string(),
    }
}

impl ImageDecoder for JpegCodec {
    fn decode(&self, data: &[u8], max_pixels: u64) -> Result<DecodedImage, Error> {
        use zune_core::colorspace::ColorSpace;
        use zune_core::options::DecoderOptions;
        use zune_jpeg::JpegDecoder;

        let opts = DecoderOptions::default().jpeg_set_out_colorspace(ColorSpace::RGBA);
        let mut dec =
            JpegDecoder::new_with_options(zune_core::bytestream::ZCursor::new(data), opts);

        // IMPORTANTE: validar dimensiones del header ANTES de decodificar (anti-bomba).
        dec.decode_headers().map_err(decode_err)?;
        let info = dec
            .info()
            .ok_or_else(|| decode_err("missing header info"))?;
        let (w, h) = (u32::from(info.width), u32::from(info.height));
        let pixels = u64::from(w) * u64::from(h);
        if pixels > max_pixels {
            return Err(Error::LimitExceeded {
                pixels,
                limit: max_pixels,
            });
        }

        let buf = dec.decode().map_err(decode_err)?;
        DecodedImage::new(w, h, buf).map_err(decode_err)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::codecs::ImageDecoder;
    use crate::testutil::vector_gradient_circle;

    #[test]
    fn decodea_jpeg_a_rgba() {
        let img = vector_gradient_circle(32, 24);
        let rgb = img.to_rgb_over_white();
        let mut bytes = Vec::new();
        jpeg_encoder::Encoder::new(&mut bytes, 90)
            .encode(&rgb, 32, 24, jpeg_encoder::ColorType::Rgb)
            .unwrap();
        let out = JpegCodec.decode(&bytes, u64::MAX).unwrap();
        assert_eq!((out.width, out.height), (32, 24));
        assert_eq!(out.pixels.len(), 32 * 24 * 4);
        assert!(
            out.pixels.chunks_exact(4).all(|p| p[3] == 255),
            "JPEG no tiene alpha"
        );
        // calidad 90: el primer píxel debe quedar cerca del original
        assert!((i16::from(out.pixels[0]) - i16::from(rgb[0])).unsigned_abs() < 24);
    }

    #[test]
    fn error_tipado_con_basura() {
        let err = JpegCodec
            .decode(&[0xFF, 0xD8, 0xFF, 0x00, 0x00], u64::MAX)
            .unwrap_err();
        assert!(matches!(err, crate::Error::Decode { .. }));
    }

    #[test]
    fn limite_de_pixeles_se_aplica() {
        let img = vector_gradient_circle(32, 24); // 768 px
        let rgb = img.to_rgb_over_white();
        let mut bytes = Vec::new();
        jpeg_encoder::Encoder::new(&mut bytes, 90)
            .encode(&rgb, 32, 24, jpeg_encoder::ColorType::Rgb)
            .unwrap();
        let err = JpegCodec.decode(&bytes, 100).unwrap_err();
        assert!(matches!(
            err,
            crate::Error::LimitExceeded {
                pixels: 768,
                limit: 100
            }
        ));
    }
}
