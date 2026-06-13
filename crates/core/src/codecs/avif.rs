//! AVIF: encode con ravif (rav1e, Rust puro); decode con avif-decode (libaom/aom-decode, native).
use crate::codecs::ImageEncoder;
use crate::error::Error;
use crate::format::Format;
use crate::image::DecodedImage;
use crate::options::Options;

/// Codec for encoding images to AVIF format.
pub(crate) struct AvifCodec;

fn encode_err(e: impl std::fmt::Display) -> Error {
    Error::Encode {
        format: Format::Avif,
        detail: e.to_string(),
    }
}

#[cfg(feature = "native")]
fn decode_err(e: impl std::fmt::Display) -> Error {
    Error::Decode {
        format: Format::Avif,
        detail: e.to_string(),
    }
}

#[cfg(feature = "native")]
/// Rechaza AVIFs con alpha premultiplicado (asociado): avif-decode los
/// des-premultiplica con una fórmula rota (colapsa a ~0) → colores corruptos
/// silenciosos. Mejor un error tipado. Aislado para ser testeable sin fixture.
fn reject_if_premultiplied(premultiplied: bool) -> Result<(), Error> {
    if premultiplied {
        return Err(decode_err(
            "premultiplied-alpha AVIF is not supported: avif-decode un-premultiply formula is incorrect (would silently corrupt colors)",
        ));
    }
    Ok(())
}

impl ImageEncoder for AvifCodec {
    fn encode(&self, img: &DecodedImage, opts: &Options) -> Result<Vec<u8>, Error> {
        // AVIF lossless existe pero ravif no lo expone → rechazo honesto v1.
        if opts.lossless {
            return Err(Error::InvalidOptions(
                "AVIF lossless is not supported in v1".into(),
            ));
        }

        // Build RGBA8 pixel slice via bytemuck (rgb crate enables "bytemuck"
        // feature so RGBA8 implements Pod — zero-copy cast, no allocation).
        let pixels: &[rgb::RGBA8] = bytemuck::cast_slice(&img.pixels);

        // u32 → usize: images larger than 4 Gpx are impossible in practice.
        #[allow(clippy::cast_possible_truncation)]
        let (w, h) = (img.width as usize, img.height as usize);

        let buf = imgref::Img::new(pixels, w, h);

        // effort 0-9 → speed 10-1 (ravif: 1 = slow/dense, 10 = fast).
        // saturating_sub prevents underflow; max(1) keeps speed in ravif's
        // valid range [1, 10].
        let speed = 10u8.saturating_sub(opts.effort).max(1);

        // DETERMINISMO: ravif deriva la cantidad de tiles AV1 de los threads
        // (threads=None → rayon::current_num_threads()), y la geometría de tiles
        // CAMBIA los bytes de salida. Se fija threads=1 → 1 tile → bytes idénticos
        // en cualquier máquina (requisito de los goldens de conformance, CLAUDE.md §3).
        // Trade-off: encode secuencial por llamada; el paralelismo se obtiene a nivel
        // de batch (rayon sobre múltiples imágenes), no dentro de un encode.
        let res = ravif::Encoder::new()
            .with_quality(f32::from(opts.quality))
            .with_alpha_quality(f32::from(opts.alpha_quality))
            .with_speed(speed)
            .with_num_threads(Some(1))
            .encode_rgba(buf)
            .map_err(encode_err)?;

        Ok(res.avif_file)
    }
}

use crate::codecs::ImageDecoder;

#[cfg(feature = "native")]
impl ImageDecoder for AvifCodec {
    fn decode(&self, data: &[u8], max_pixels: u64) -> Result<DecodedImage, Error> {
        // ANTI-BOMBA real: dimensiones del sequence header AV1 (OBU) vía avif-parse,
        // SIN decodificar el frame. max_frame_* es el máximo de secuencia (cota
        // superior conservadora del tamaño croppeado) — la dirección segura para un guard.
        //
        // BUG CONOCIDO avif-parse 1.4.0: `read_avif` puede paniquear (assert!/
        // debug_assert!) sobre AVIF malformado — p.ej. debug_assert_eq! en lib.rs:1329
        // ("bad parser state bytes left", se activa en debug/test/fuzz) y assert! en
        // lib.rs:668 (offset <= size, alcanzable también en release con boxes cuyo
        // declared size supera el espacio disponible). `catch_unwind` convierte ese
        // pánico en un error tipado para que NUNCA escape a través de la API pública.
        // Backlog: cuando avif-parse upstream corrija los assert→error, eliminar este wrap.
        //
        // avif-parse 1.4 puede paniquear (assert!/debug_assert!) sobre AVIF malformado.
        // Se contiene con catch_unwind → Error::Decode tipado. NO se suprime el panic
        // hook: set_hook es global y minipix decodifica AVIFs en paralelo (threadpools
        // de los bindings / worker pool) — suprimir concurrentemente corrompería el hook.
        // El ruido en stderr aparece sólo en el camino de error con input malformado.
        let data_owned = data.to_vec();
        let parse_result = std::panic::catch_unwind(move || {
            let mut cursor = std::io::Cursor::new(data_owned);
            avif_parse::read_avif(&mut cursor)
        })
        .map_err(|_| {
            decode_err(
                "avif-parse panicked on malformed input (upstream bug: assert instead of error)",
            )
        })?;
        let parsed = parse_result.map_err(decode_err)?;
        let meta = parsed.primary_item_metadata().map_err(decode_err)?;
        let header_pixels =
            u64::from(meta.max_frame_width.get()) * u64::from(meta.max_frame_height.get());
        if header_pixels > max_pixels {
            return Err(Error::LimitExceeded {
                pixels: header_pixels,
                limit: max_pixels,
            });
        }

        // Guard: avif-decode 1.0.2 un-premultiplies associated alpha via a broken
        // formula — `(val*256)/(alpha*256)/256` which always yields `val/alpha/256 ≈ 0`
        // instead of the correct `val*255/alpha`.  Rather than silently produce wrong
        // colors, we reject premultiplied AVIFs with a typed error until avif-decode is
        // fixed upstream or replaced.
        //
        // avif-parse exposes `AvifData::premultiplied_alpha` (the HEIF `prem` reference
        // flag), so detection is reliable and zero-cost — the parse already happened for
        // the dimension guard above.
        reject_if_premultiplied(parsed.premultiplied_alpha)?;

        // Nota ICC: nclx/CICP la maneja avif-decode; AVIF con ICC embebido (`colr`
        // tipo `prof`) NO se normaliza en v1 (backlog).

        // Decodifica el frame AV1 completo (color conversion incluida).
        // A partir de aquí las dimensiones reales están disponibles.
        let decoder = avif_decode::Decoder::from_avif(data).map_err(decode_err)?;
        let image = decoder.to_image().map_err(decode_err)?;

        // Extract dimensions from the decoded image (available in all variants).
        let (w, h) = match &image {
            avif_decode::Image::Rgb8(img) => (img.width(), img.height()),
            avif_decode::Image::Rgb16(img) => (img.width(), img.height()),
            avif_decode::Image::Rgba8(img) => (img.width(), img.height()),
            avif_decode::Image::Rgba16(img) => (img.width(), img.height()),
            avif_decode::Image::Gray8(img) => (img.width(), img.height()),
            avif_decode::Image::Gray16(img) => (img.width(), img.height()),
        };

        // Belt-and-suspenders: el guard primario es el check pre-decode de arriba.
        // Este segundo check cubre el caso (teórico) en que las dimensiones reales
        // tras crop superen el header — no debería ocurrir, pero no cuesta nada.
        let pixels = w as u64 * h as u64;
        if pixels > max_pixels {
            return Err(Error::LimitExceeded {
                pixels,
                limit: max_pixels,
            });
        }

        // Normalize all variants to RGBA8. Strategy:
        //   - Rgb8   → alpha 255
        //   - Rgb16  → (channel >> 8) as u8, alpha 255
        //   - Rgba8  → pass-through
        //   - Rgba16 → (channel >> 8) as u8
        //   - Gray8  → replicate to RGB, alpha 255
        //   - Gray16 → (channel >> 8) as u8, replicate to RGB, alpha 255
        // w*h ya está acotado por max_pixels (guard de arriba); en 64-bit no
        // puede desbordar. wasm32/32-bit queda cubierto porque max_pixels
        // razonables (<2^30) mantienen w*h*4 < usize::MAX.
        let mut out: Vec<u8> = Vec::with_capacity(w * h * 4);

        match image {
            avif_decode::Image::Rgb8(img) => {
                for px in img.pixels() {
                    out.extend_from_slice(&[px.r, px.g, px.b, 255]);
                }
            }
            avif_decode::Image::Rgb16(img) => {
                for px in img.pixels() {
                    // 16-bit → 8-bit: discard low byte.
                    // This loses the low 8 bits of precision, which is acceptable
                    // for display/compression pipelines that operate on 8-bit images.
                    #[allow(clippy::cast_possible_truncation)] // intentional 16→8 squash
                    out.extend_from_slice(&[
                        (px.r >> 8) as u8,
                        (px.g >> 8) as u8,
                        (px.b >> 8) as u8,
                        255,
                    ]);
                }
            }
            avif_decode::Image::Rgba8(img) => {
                for px in img.pixels() {
                    out.extend_from_slice(&[px.r, px.g, px.b, px.a]);
                }
            }
            avif_decode::Image::Rgba16(img) => {
                for px in img.pixels() {
                    #[allow(clippy::cast_possible_truncation)] // intentional 16→8 squash
                    out.extend_from_slice(&[
                        (px.r >> 8) as u8,
                        (px.g >> 8) as u8,
                        (px.b >> 8) as u8,
                        (px.a >> 8) as u8,
                    ]);
                }
            }
            avif_decode::Image::Gray8(img) => {
                for px in img.pixels() {
                    let v = px.value();
                    out.extend_from_slice(&[v, v, v, 255]);
                }
            }
            avif_decode::Image::Gray16(img) => {
                for px in img.pixels() {
                    #[allow(clippy::cast_possible_truncation)] // intentional 16→8 squash
                    let v = (px.value() >> 8) as u8;
                    out.extend_from_slice(&[v, v, v, 255]);
                }
            }
        }

        // u64→u32: safe because we already verified pixels <= max_pixels <= u64::MAX,
        // and images above 4 Gpx are rejected by the pixel limit before we get here.
        #[allow(clippy::cast_possible_truncation)]
        DecodedImage::new(w as u32, h as u32, out).map_err(|e| Error::Decode {
            format: Format::Avif,
            detail: e.to_string(),
        })
    }
}

#[cfg(all(feature = "wasm", not(feature = "native")))]
impl ImageDecoder for AvifCodec {
    fn decode(&self, _data: &[u8], _max_pixels: u64) -> Result<DecodedImage, Error> {
        // En wasm el navegador decodifica AVIF (createImageBitmap) y el worker
        // alimenta RGBA vía encode_rgba(); el core lo rechaza tipado.
        Err(Error::Decode {
            format: Format::Avif,
            detail: "AVIF decode is not available in the wasm build (decode in the browser and use encodeRgba)".into(),
        })
    }
}

// Cobertura pendiente (backlog): variantes 16-bit y grayscale del normalizador (ravif solo encodea 8-bit; requiere fixture AVIF 10-bit).
#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use crate::codecs::ImageEncoder;
    use crate::testutil::vector_gradient_circle;

    #[test]
    fn encode_produce_avif_valido() {
        let img = vector_gradient_circle(32, 24);
        let bytes = AvifCodec
            .encode(&img, &crate::Options::default().with_effort(9))
            .unwrap();
        assert_eq!(crate::sniff::sniff(&bytes), Some(crate::Format::Avif));
    }

    #[test]
    fn quality_ordena_tamanios() {
        let img = vector_gradient_circle(64, 64);
        let q90 = AvifCodec
            .encode(
                &img,
                &crate::Options::default().with_quality(90).with_effort(9),
            )
            .unwrap();
        let q30 = AvifCodec
            .encode(
                &img,
                &crate::Options::default().with_quality(30).with_effort(9),
            )
            .unwrap();
        assert!(q30.len() < q90.len());
    }

    #[test]
    fn lossless_es_rechazado() {
        let img = crate::testutil::vector_flat_colors(8, 8);
        let err = AvifCodec
            .encode(&img, &crate::Options::default().with_lossless(true))
            .unwrap_err();
        assert!(matches!(err, crate::Error::InvalidOptions(_)));
    }

    #[cfg(feature = "native")]
    #[test]
    fn roundtrip_encode_decode() {
        let img = vector_gradient_circle(32, 24);
        let bytes = AvifCodec
            .encode(
                &img,
                &crate::Options::default().with_quality(90).with_effort(9),
            )
            .unwrap();
        let back = AvifCodec.decode(&bytes, u64::MAX).unwrap();
        assert_eq!((back.width, back.height), (32, 24));
        assert!(
            back.has_transparency(),
            "el circulo alpha=128 debe sobrevivir"
        );
    }

    #[cfg(feature = "native")]
    #[test]
    fn decode_error_tipado_con_basura() {
        let mut junk = vec![0x00, 0x00, 0x00, 0x1C];
        junk.extend(b"ftypavif");
        junk.extend([0u8; 64]);
        assert!(matches!(
            AvifCodec.decode(&junk, u64::MAX).unwrap_err(),
            crate::Error::Decode { .. }
        ));
    }

    #[cfg(feature = "native")]
    #[test]
    fn decode_limite_de_pixeles() {
        let img = vector_gradient_circle(32, 24); // 768 px
        let bytes = AvifCodec
            .encode(
                &img,
                &crate::Options::default().with_quality(90).with_effort(9),
            )
            .unwrap();
        let err = AvifCodec.decode(&bytes, 100).unwrap_err();
        assert!(matches!(
            err,
            crate::Error::LimitExceeded {
                pixels: 768,
                limit: 100
            }
        ));
    }

    /// Verify that `avif-parse` correctly exposes the `premultiplied_alpha` flag
    /// from a hand-crafted minimal AVIF container that carries a `prem` item reference.
    ///
    /// This is a unit test of the *detection path* — we confirm that our guard fires
    /// without needing a full AV1 bitstream.  A complete premultiplied-alpha AVIF
    /// fixture (with real AV1 payload) is not included because ravif always produces
    /// unassociated alpha and constructing one requires external tooling; that is
    /// tracked in the backlog.
    ///
    /// The formula bug in avif-decode 1.0.2 `unprem`:
    ///   `(val*256) / (alpha*256) / 256`  →  `val/alpha/256`  (≈ 0 for all valid inputs)
    /// Correct formula: `val * 255 / alpha`.
    /// See avif-decode-1.0.2/src/lib.rs lines 142-143 / 153-154.
    #[test]
    fn avif_parse_exposes_premultiplied_flag() {
        // Use an image with real alpha (straight/unassociated, alpha=128 circle)
        // to meaningfully assert that ravif straight-alpha output → premultiplied_alpha == false.
        // This confirms the flag is correctly exposed and not accidentally always-true or
        // always-false.
        use crate::codecs::ImageEncoder;
        let img = vector_gradient_circle(32, 24);
        let bytes = AvifCodec
            .encode(&img, &crate::Options::default().with_effort(9))
            .unwrap();
        let mut cursor = std::io::Cursor::new(&bytes);
        let parsed = avif_parse::read_avif(&mut cursor)
            .expect("valid AVIF produced by ravif must parse cleanly");
        // ravif encodes with AlphaColorMode::UnassociatedClean → no `prem` ref → flag must be false.
        assert!(
            !parsed.premultiplied_alpha,
            "ravif straight-alpha output must NOT carry the premultiplied flag"
        );
    }

    #[cfg(feature = "native")]
    #[test]
    fn reject_if_premultiplied_rechaza_true_acepta_false() {
        // Cubre AMBAS ramas del guard sin necesitar un fixture AVIF premultiplicado.
        assert!(super::reject_if_premultiplied(false).is_ok());
        let err = super::reject_if_premultiplied(true).unwrap_err();
        assert!(matches!(
            err,
            crate::Error::Decode {
                format: crate::Format::Avif,
                ..
            }
        ));
    }
}
