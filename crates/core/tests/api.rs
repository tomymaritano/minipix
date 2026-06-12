//! Tests de integración de la API pública.
#![allow(clippy::unwrap_used)]

use minipix_core::{Error, Format, Options, compress, convert};

/// PNG válido de 4x4 generado con el propio crate png (dep ya presente).
fn tiny_png() -> Vec<u8> {
    let mut bytes = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut bytes, 4, 4);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut w = enc.write_header().unwrap();
        w.write_image_data(&[128u8; 64]).unwrap();
        w.finish().unwrap();
    }
    bytes
}

#[test]
fn compress_mantiene_formato() {
    let out = compress(&tiny_png(), &Options::default()).unwrap();
    assert_eq!(out.format, Format::Png);
    assert_eq!((out.width, out.height), (4, 4));
    assert_eq!(out.bytes_in, tiny_png().len() as u64);
    assert_eq!(out.bytes_out, out.data.len() as u64);
}

#[test]
fn convert_png_a_jpeg_webp_y_avif() {
    for fmt in [Format::Jpeg, Format::WebP, Format::Avif] {
        let out = convert(
            &tiny_png(),
            &Options::default().with_format(fmt).with_effort(9),
        )
        .unwrap();
        assert_eq!(out.format, fmt);
        assert_eq!(minipix_core::sniff::sniff(&out.data), Some(fmt));
    }
}

#[test]
fn convert_sin_format_es_error() {
    assert!(matches!(
        convert(&tiny_png(), &Options::default()).unwrap_err(),
        Error::InvalidOptions(_)
    ));
}

#[test]
fn basura_es_unsupported() {
    assert!(matches!(
        compress(b"garbage", &Options::default()).unwrap_err(),
        Error::UnsupportedFormat
    ));
}

#[test]
fn opciones_invalidas_se_rechazan_antes_de_trabajar() {
    let mut opts = Options::default();
    opts.quality = 0;
    assert!(matches!(
        compress(&tiny_png(), &opts).unwrap_err(),
        Error::InvalidOptions(_)
    ));
}

#[test]
fn limite_de_pixeles_corta_antes_de_decodear() {
    // PNG crafteado: solo firma + IHDR declarando 60000x60000 (sin data).
    let mut bytes = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    bytes.extend([0, 0, 0, 13]);
    bytes.extend(b"IHDR");
    bytes.extend(60000u32.to_be_bytes());
    bytes.extend(60000u32.to_be_bytes());
    bytes.extend([8, 6, 0, 0, 0]);
    bytes.extend([0, 0, 0, 0]); // CRC inválido: no llegamos a leerlo
    let err = compress(&bytes, &Options::default()).unwrap_err();
    assert!(matches!(err, Error::LimitExceeded { .. }));
}
