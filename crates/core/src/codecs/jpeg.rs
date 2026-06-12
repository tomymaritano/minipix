//! JPEG: decode con zune-jpeg normalizado a RGBA8; encode via mozjpeg.
use crate::codecs::{ImageDecoder, ImageEncoder};
use crate::error::Error;
use crate::format::Format;
use crate::image::DecodedImage;
use crate::options::Options;

/// Codec for decoding JPEG images.
#[allow(dead_code)] // instanciado sólo en tests hasta que se conecte al dispatcher (Task 15)
pub(crate) struct JpegCodec;

fn decode_err(e: impl std::fmt::Display) -> Error {
    Error::Decode {
        format: Format::Jpeg,
        detail: e.to_string(),
    }
}

fn encode_err(e: impl std::fmt::Display) -> Error {
    Error::Encode {
        format: Format::Jpeg,
        detail: e.to_string(),
    }
}

impl ImageDecoder for JpegCodec {
    fn decode(&self, data: &[u8], max_pixels: u64) -> Result<DecodedImage, Error> {
        use zune_core::colorspace::ColorSpace;
        use zune_core::options::DecoderOptions;
        use zune_jpeg::JpegDecoder;

        // max_pixels gobierna el área total; cada eje puede ser a lo sumo max_pixels.
        // max_scans/deflate_limit se dejan en los defaults anti-DoS de zune a propósito.
        let axis_cap = usize::try_from(max_pixels).unwrap_or(usize::MAX);
        let opts = DecoderOptions::default()
            .jpeg_set_out_colorspace(ColorSpace::RGBA)
            .set_max_width(axis_cap)
            .set_max_height(axis_cap);
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

impl ImageEncoder for JpegCodec {
    fn encode(&self, img: &DecodedImage, opts: &Options) -> Result<Vec<u8>, Error> {
        if opts.lossless {
            return Err(Error::InvalidOptions(
                "JPEG does not support lossless".into(),
            ));
        }
        let rgb = img.to_rgb_over_white();
        // u32 → usize: images larger than 4 Gpx are impossible in practice.
        #[allow(clippy::cast_possible_truncation)]
        let (w, h) = (img.width as usize, img.height as usize);
        let q = f32::from(opts.quality);
        let progressive = opts.jpeg_progressive;

        // mozjpeg signals errors via resume_unwind (longjmp wrapper).
        // The ENTIRE mozjpeg flow must stay inside catch_unwind so no panic
        // escapes the core boundary (see CLAUDE.md: no panic crosses the boundary).
        std::panic::catch_unwind(move || -> Result<Vec<u8>, String> {
            let mut comp = mozjpeg::Compress::new(mozjpeg::ColorSpace::JCS_RGB);
            comp.set_size(w, h);
            comp.set_quality(q);
            if progressive {
                comp.set_progressive_mode();
            }
            let mut started = comp.start_compress(Vec::new()).map_err(|e| e.to_string())?;
            started.write_scanlines(&rgb).map_err(|e| e.to_string())?;
            started.finish().map_err(|e| e.to_string())
        })
        .map_err(|p| {
            encode_err(
                p.downcast_ref::<String>()
                    .map_or("mozjpeg aborted (internal libjpeg error)", |s| s.as_str()),
            )
        })?
        .map_err(encode_err)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::codecs::{ImageDecoder, ImageEncoder};
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

    #[test]
    fn encode_decode_roundtrip_aproximado() {
        let img = crate::testutil::vector_gradient_circle(32, 24);
        let bytes = JpegCodec
            .encode(&img, &crate::Options::default().with_quality(90))
            .unwrap();
        let back = JpegCodec.decode(&bytes, u64::MAX).unwrap();
        assert_eq!((back.width, back.height), (32, 24));
    }

    #[test]
    fn quality_menor_da_menos_bytes() {
        let img = crate::testutil::vector_gradient_circle(64, 64);
        let q90 = JpegCodec
            .encode(&img, &crate::Options::default().with_quality(90))
            .unwrap();
        let q40 = JpegCodec
            .encode(&img, &crate::Options::default().with_quality(40))
            .unwrap();
        assert!(q40.len() < q90.len());
    }

    #[test]
    fn lossless_es_rechazado() {
        let img = crate::testutil::vector_flat_colors(8, 8);
        let err = JpegCodec
            .encode(&img, &crate::Options::default().with_lossless(true))
            .unwrap_err();
        assert!(matches!(err, crate::Error::InvalidOptions(_)));
    }

    #[test]
    fn ancho_mayor_a_16384_es_valido_si_max_pixels_lo_permite() {
        // 19000x2 = 38000 px: supera el eje default de zune (16384) pero no nuestro límite.
        let w = 19000u16;
        let rgb = vec![128u8; usize::from(w) * 2 * 3];
        let mut bytes = Vec::new();
        jpeg_encoder::Encoder::new(&mut bytes, 90)
            .encode(&rgb, w, 2, jpeg_encoder::ColorType::Rgb)
            .unwrap();
        let out = JpegCodec.decode(&bytes, u64::MAX).unwrap();
        assert_eq!((out.width, out.height), (u32::from(w), 2));
    }
}
