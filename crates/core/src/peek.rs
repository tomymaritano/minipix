// crates/core/src/peek.rs
//! Dimensiones desde el header SIN decodificar (capa rápida anti-bomba;
//! cada códec valida además por su cuenta — defensa en profundidad).
use crate::format::Format;

/// `None` = este formato no permite peek barato (AVIF: avif-parse lo cubre en el códec).
pub(crate) fn peek_dimensions(format: Format, data: &[u8]) -> Option<(u32, u32)> {
    match format {
        Format::Png => {
            if data.len() < 24 || &data[12..16] != b"IHDR" {
                return None;
            }
            let w = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
            let h = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
            Some((w, h))
        }
        Format::Jpeg => peek_jpeg(data),
        Format::WebP => peek_webp(data),
        Format::Avif => None,
    }
}

fn peek_jpeg(data: &[u8]) -> Option<(u32, u32)> {
    // Recorre marcadores hasta un SOFn (C0-CF salvo C4/C8/CC).
    let mut i = 2;
    while i + 9 < data.len() {
        if data[i] != 0xFF {
            return None;
        }
        let marker = data[i + 1];
        if (0xC0..=0xCF).contains(&marker) && !matches!(marker, 0xC4 | 0xC8 | 0xCC) {
            let h = u32::from(u16::from_be_bytes([data[i + 5], data[i + 6]]));
            let w = u32::from(u16::from_be_bytes([data[i + 7], data[i + 8]]));
            return Some((w, h));
        }
        let len = usize::from(u16::from_be_bytes([data[i + 2], data[i + 3]]));
        i += 2 + len.max(2);
    }
    None
}

fn peek_webp(data: &[u8]) -> Option<(u32, u32)> {
    if data.len() < 30 {
        return None;
    }
    match &data[12..16] {
        b"VP8X" => {
            let w =
                1 + u32::from(data[24]) + (u32::from(data[25]) << 8) + (u32::from(data[26]) << 16);
            let h =
                1 + u32::from(data[27]) + (u32::from(data[28]) << 8) + (u32::from(data[29]) << 16);
            Some((w, h))
        }
        b"VP8 " => {
            let w = u32::from(u16::from_le_bytes([data[26], data[27]]) & 0x3FFF);
            let h = u32::from(u16::from_le_bytes([data[28], data[29]]) & 0x3FFF);
            Some((w, h))
        }
        b"VP8L" => {
            let b = [data[21], data[22], data[23], data[24]];
            let bits = u32::from_le_bytes(b);
            Some(((bits & 0x3FFF) + 1, ((bits >> 14) & 0x3FFF) + 1))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    /// PNG IHDR artesanal con dimensiones 60000x60000.
    #[test]
    fn png_ihdr_crafted_60k() {
        let mut data = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        data.extend([0, 0, 0, 13]);
        data.extend(b"IHDR");
        data.extend(60000u32.to_be_bytes());
        data.extend(60000u32.to_be_bytes());
        data.extend([8, 6, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(peek_dimensions(Format::Png, &data), Some((60000, 60000)));
    }

    /// Datos demasiado cortos → None.
    #[test]
    fn png_short_data_none() {
        assert_eq!(
            peek_dimensions(Format::Png, &[0x89, b'P', b'N', b'G']),
            None
        );
    }

    /// JPEG: roundtrip dims via jpeg-encoder (disponible en dev-deps in-crate).
    #[test]
    fn jpeg_peek_dims() {
        // Generar un JPEG 32x24 con jpeg-encoder.
        let rgb = vec![128u8; 32 * 24 * 3];
        let mut bytes = Vec::new();
        jpeg_encoder::Encoder::new(&mut bytes, 90)
            .encode(&rgb, 32, 24, jpeg_encoder::ColorType::Rgb)
            .unwrap();
        assert_eq!(peek_dimensions(Format::Jpeg, &bytes), Some((32, 24)));
    }

    /// WebP VP8L lossless: roundtrip dims via image-webp.
    #[test]
    fn webp_vp8l_peek_dims() {
        let pixels = vec![128u8; 32 * 24 * 4];
        let mut bytes = Vec::new();
        image_webp::WebPEncoder::new(std::io::Cursor::new(&mut bytes))
            .encode(&pixels, 32, 24, image_webp::ColorType::Rgba8)
            .unwrap();
        // image-webp lossless produce VP8L
        assert_eq!(peek_dimensions(Format::WebP, &bytes), Some((32, 24)));
    }

    /// AVIF siempre devuelve None (sin peek barato).
    #[test]
    fn avif_siempre_none() {
        assert_eq!(peek_dimensions(Format::Avif, &[0u8; 64]), None);
    }
}
