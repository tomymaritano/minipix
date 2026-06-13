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

    /// WebP VP8 lossy: header artesanal con dimensiones 640x480.
    #[test]
    fn webp_vp8_lossy_peek_dims() {
        // Header VP8 lossy crafteado: RIFF + "VP8 " + frame tag + start code + dims 14-bit LE.
        let mut f = Vec::new();
        f.extend(b"RIFF");
        f.extend(20u32.to_le_bytes());
        f.extend(b"WEBP");
        f.extend(b"VP8 ");
        f.extend(12u32.to_le_bytes());
        f.extend([0x00, 0x00, 0x00]); // frame tag (keyframe bits no importan para peek)
        f.extend([0x9D, 0x01, 0x2A]); // start code
        f.extend(640u16.to_le_bytes()); // width 14 bits
        f.extend(480u16.to_le_bytes()); // height 14 bits
        assert_eq!(peek_dimensions(Format::WebP, &f), Some((640, 480)));
    }

    /// AVIF siempre devuelve None (sin peek barato).
    #[test]
    fn avif_siempre_none() {
        assert_eq!(peek_dimensions(Format::Avif, &[0u8; 64]), None);
    }

    // ── Robustez: peek_dimensions nunca panica sobre input aleatorio ──────────
    // SplitMix64 inline — sin dependencias externas, determinista.
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        }
        #[allow(clippy::cast_possible_truncation)]
        fn byte(&mut self) -> u8 {
            (self.next() & 0xFF) as u8
        }
        #[allow(clippy::cast_possible_truncation)]
        fn range(&mut self, n: usize) -> usize {
            if n == 0 {
                return 0;
            }
            (self.next() % n as u64) as usize
        }
    }

    /// `peek_dimensions` nunca panica sobre bytes aleatorios para los 4 formatos.
    /// La función es `pub(crate)` → se cubre aquí dentro del módulo, no desde `tests/`.
    #[test]
    fn peek_dimensions_nunca_panica_sobre_input_aleatorio() {
        let mut rng = Rng(0xABCD_1234_5678_EF00);
        let formats = [Format::Png, Format::Jpeg, Format::WebP, Format::Avif];

        for _ in 0..3000 {
            let len = rng.range(256);
            let buf: Vec<u8> = (0..len).map(|_| rng.byte()).collect();
            for &fmt in &formats {
                // La única aserción es que no panica; el valor de retorno es irrelevante.
                let _ = peek_dimensions(fmt, &buf);
            }
        }

        // Buffers cortos (0-15 bytes) — cubren todos los guards de longitud mínima.
        for len in 0usize..16 {
            let buf: Vec<u8> = (0..len).map(|_| rng.byte()).collect();
            for &fmt in &formats {
                let _ = peek_dimensions(fmt, &buf);
            }
        }
    }

    /// Entradas diseñadas para tensar las ramas de longitud en cada parser.
    #[test]
    fn peek_dimensions_longitudes_boundary_nunca_paniquan() {
        // PNG: exactamente 23 bytes (un byte menos que el mínimo de 24).
        let short_png = vec![
            0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13, // chunk length
            b'I', b'H', b'D', b'R', // chunk type
            0, 0, 0, 1, // width
            0, 0, 0, // altura incompleta (23 bytes total)
        ];
        assert_eq!(peek_dimensions(Format::Png, &short_png), None);

        // WebP: exactamente 29 bytes (un byte menos que el mínimo de 30).
        let short_webp = vec![
            b'R', b'I', b'F', b'F', 25, 0, 0, 0, b'W', b'E', b'B', b'P', b'V', b'P', b'8', b'X', 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 29 bytes
        ];
        assert_eq!(peek_dimensions(Format::WebP, &short_webp), None);

        // JPEG: buffer de 9 bytes (justo en el límite del bucle `i + 9 < data.len()`).
        let short_jpeg = vec![0xFF, 0xD8, 0xFF, 0xC0, 0, 11, 0, 0, 1];
        assert_eq!(peek_dimensions(Format::Jpeg, &short_jpeg), None);
    }
}
