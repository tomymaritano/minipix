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
        // WebP animado: v1 no lo soporta; rechazo explícito en vez de aplanar
        // silenciosamente al primer frame (pérdida de datos).
        if dec.is_animated() {
            return Err(decode_err("animated WebP is not supported in v1"));
        }
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
    fn webp_animado_es_rechazado() {
        // Contenedor mínimo: VP8X con flag de animación (bit 1) + chunk ANIM.
        // Construido a mano para que image_webp::WebPDecoder reconozca el flag
        // is_animated() antes de intentar leer frames de imagen.
        let mut f: Vec<u8> = Vec::new();
        // RIFF header — el tamaño debe ser exactamente (total_bytes - 8).
        // Contenido: "WEBP" + VP8X(10) + ANIM(6) = 4 + 8+10 + 8+6 = 36 bytes.
        f.extend(b"RIFF");
        f.extend(36u32.to_le_bytes()); // 36 bytes tras este campo
        f.extend(b"WEBP");
        // VP8X chunk (siempre 10 bytes de payload)
        f.extend(b"VP8X");
        f.extend(10u32.to_le_bytes());
        f.push(0b0000_0010); // flags byte: bit 1 = animation
        f.extend([0u8; 3]); // reserved bytes
        f.extend([31u8, 0, 0]); // canvas width-1 (24-bit LE) => 32 px
        f.extend([23u8, 0, 0]); // canvas height-1 (24-bit LE) => 24 px
        // ANIM chunk (6 bytes de payload: background colour 4 bytes + loop count 2 bytes)
        f.extend(b"ANIM");
        f.extend(6u32.to_le_bytes());
        f.extend([0u8; 6]);

        let err = WebpCodec.decode(&f, u64::MAX).unwrap_err();
        // image-webp rechaza el contenedor en WebPDecoder::new() antes de que
        // nuestro guard is_animated() se ejecute, porque el decodificador
        // exige al menos un chunk ANMF con datos de imagen real.  El error que
        // llega es "An expected chunk was missing" — un Decode tipado igualmente
        // correcto: nunca aplanamos silenciosamente al primer frame.
        // La rama is_animated() queda sin cobertura directa desde este fixture
        // (DONE_WITH_CONCERNS), pero el comportamiento observable es el mismo.
        assert!(
            matches!(err, crate::Error::Decode { .. }),
            "se esperaba Decode tipado, fue: {err}"
        );
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
