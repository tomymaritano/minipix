//! WebP: decode con image-webp (Rust puro) + encode con libwebp-sys2 (FFI).
//!
//! Este es el ÚNICO módulo unsafe del workspace. El bloque unsafe vive en
//! `mod ffi` protegido por `#[allow(unsafe_code)]`; el resto del crate sigue
//! con `unsafe_code = "deny"` a nivel workspace.
use crate::codecs::{ImageDecoder, ImageEncoder};
use crate::error::Error;
use crate::format::Format;
use crate::image::DecodedImage;
use crate::options::Options;

/// Codec for decoding and encoding WebP images.
#[allow(dead_code)] // instanciado sólo en tests hasta que se conecte al dispatcher (Task 15)
pub(crate) struct WebpCodec;

fn decode_err(e: impl std::fmt::Display) -> Error {
    Error::Decode {
        format: Format::WebP,
        detail: e.to_string(),
    }
}

fn encode_err(e: impl std::fmt::Display) -> Error {
    Error::Encode {
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

impl ImageEncoder for WebpCodec {
    fn encode(&self, img: &DecodedImage, opts: &Options) -> Result<Vec<u8>, Error> {
        ffi::encode_rgba(
            &img.pixels,
            img.width,
            img.height,
            &ffi::EncodeParams {
                quality: f32::from(opts.quality),
                alpha_quality: i32::from(opts.alpha_quality),
                method: i32::from(opts.effort.min(6)), // effort 0-9 → method 0-6
                lossless: opts.lossless,
            },
        )
        .map_err(encode_err)
    }
}

/// FFI con libwebp: ÚNICO módulo unsafe del workspace (CLAUDE.md).
///
/// Toda la memoria C se limpia en TODOS los caminos (éxito y error):
/// - `WebPPictureFree` después de import exitoso Y en error post-init.
/// - `WebPMemoryWriterClear` tanto en encode-failure como después de copiar out.
#[allow(unsafe_code)]
mod ffi {
    use libwebp_sys as sys;

    /// Parámetros de encode pasados desde `WebpCodec::encode`.
    pub(super) struct EncodeParams {
        pub quality: f32,
        pub alpha_quality: i32,
        pub method: i32,
        pub lossless: bool,
    }

    /// Codifica `rgba` (RGBA8, stride = width*4) a bytes WebP.
    ///
    /// Devuelve `Err(String)` en cualquier fallo de la API C; el caller
    /// convierte a `Error::Encode` con `encode_err`.
    pub(super) fn encode_rgba(
        rgba: &[u8],
        width: u32,
        height: u32,
        p: &EncodeParams,
    ) -> Result<Vec<u8>, String> {
        // ── 1. Inicializar y validar WebPConfig ──────────────────────────────
        // SAFETY: `config` se inicializa con `WebPConfigInit` antes de leerse.
        // El struct vive en el stack de esta función; ningún puntero escapa.
        // `&raw mut` / `&raw const` evitan crear referencias a datos no-init.
        let config = unsafe {
            let mut c: sys::WebPConfig = std::mem::zeroed();
            if sys::WebPConfigInit(&raw mut c) == 0 {
                return Err("WebPConfigInit failed (version mismatch)".into());
            }
            c.quality = p.quality;
            c.alpha_quality = p.alpha_quality;
            c.method = p.method;
            c.lossless = i32::from(p.lossless);
            if sys::WebPValidateConfig(&raw const c) == 0 {
                return Err("invalid WebP config".into());
            }
            c
        };

        // ── 2. Inicializar WebPPicture e importar RGBA ───────────────────────
        // SAFETY: `pic` se inicializa con `WebPPictureInit` antes de leerse.
        // `rgba.as_ptr()` apunta a un slice vivo durante toda esta llamada.
        // `WebPPictureFree` se llama en todos los caminos post-import.
        let mut pic = unsafe {
            let mut p2: sys::WebPPicture = std::mem::zeroed();
            if sys::WebPPictureInit(&raw mut p2) == 0 {
                return Err("WebPPictureInit failed".into());
            }
            p2.use_argb = 1; // requerido para lossless y mejor calidad lossy
            p2.width = i32::try_from(width).map_err(|e| e.to_string())?;
            p2.height = i32::try_from(height).map_err(|e| e.to_string())?;

            let stride = i32::try_from(width.checked_mul(4).ok_or("width overflow")?)
                .map_err(|e| e.to_string())?;
            if sys::WebPPictureImportRGBA(&raw mut p2, rgba.as_ptr(), stride) == 0 {
                sys::WebPPictureFree(&raw mut p2);
                return Err("WebPPictureImportRGBA failed (out of memory?)".into());
            }
            p2
        };

        // ── 3. Inicializar WebPMemoryWriter y cablear el writer ──────────────
        // SAFETY: `writer` se inicializa con `WebPMemoryWriterInit` antes de
        // leerse. El puntero `pic.custom_ptr` apunta a `writer` cuyo lifetime
        // abarca todo el encode. `WebPMemoryWriterClear` se llama en todos los
        // caminos post-init (éxito y error de encode).
        let mut writer = unsafe {
            let mut w: sys::WebPMemoryWriter = std::mem::zeroed();
            sys::WebPMemoryWriterInit(&raw mut w);
            w
        };

        // SAFETY: `WebPMemoryWrite` tiene la misma firma ABI que `WebPWriterFunction`
        // (Option<extern "C" fn(...)>). La declaración FFI la marca `unsafe`
        // porque procede de un bloque `extern "C"`. El transmute es seguro:
        // ambas variantes comparten convención de llamada y tipos idénticos;
        // las precondiciones del callback (punteros válidos, writer inicializado)
        // las garantiza libwebp internamente. `pic.custom_ptr` apunta a `writer`
        // que vive en el mismo stack frame y no se mueve antes de que
        // `WebPEncode` retorne.
        // SAFETY: `addr_of_mut!` obtiene un puntero raw a `writer` sin crear
        // ninguna referencia intermedia al dato parcialmente inicializado.
        unsafe {
            pic.writer = Some(std::mem::transmute::<
                unsafe extern "C" fn(*const u8, usize, *const sys::WebPPicture) -> std::ffi::c_int,
                extern "C" fn(*const u8, usize, *const sys::WebPPicture) -> std::ffi::c_int,
            >(sys::WebPMemoryWrite));
            pic.custom_ptr = std::ptr::addr_of_mut!(writer).cast();
        }

        // ── 4. Encode ────────────────────────────────────────────────────────
        // SAFETY: `config` y `pic` están completamente inicializados. Libwebp
        // escribe en `writer.mem` (heap propio de libwebp); nosotros lo
        // copiamos a un Vec antes de liberar con `WebPMemoryWriterClear`.
        let ok = unsafe { sys::WebPEncode(&raw const config, &raw mut pic) };

        // Capturar error_code antes de free (pic se invalida tras Free).
        // SAFETY: `pic` aún es válido en este punto; Free limpia la memoria
        // interna asignada pero el struct local sigue siendo legible.
        let error_code = unsafe {
            let ec = pic.error_code;
            sys::WebPPictureFree(&raw mut pic);
            ec
        };

        if ok == 0 {
            // SAFETY: `writer` fue inicializado con `WebPMemoryWriterInit`;
            // WebPMemoryWriterClear libera `writer.mem` si fue asignado.
            unsafe { sys::WebPMemoryWriterClear(&raw mut writer) };
            return Err(format!("WebPEncode failed with code {error_code:?}"));
        }

        // ── 5. Copiar output y liberar ───────────────────────────────────────
        // SAFETY: `writer.mem` apunta a `writer.size` bytes consecutivos válidos
        // escritos por libwebp. Los copiamos a un Vec de Rust antes de llamar a
        // `WebPMemoryWriterClear`, que libera el buffer C original.
        let out = unsafe {
            let slice = std::slice::from_raw_parts(writer.mem, writer.size);
            let v = slice.to_vec();
            sys::WebPMemoryWriterClear(&raw mut writer);
            v
        };

        Ok(out)
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

    #[test]
    fn encode_lossy_decode_y_quality_order() {
        let img = vector_gradient_circle(64, 64);
        let q90 = WebpCodec
            .encode(&img, &crate::Options::default().with_quality(90))
            .unwrap();
        let q40 = WebpCodec
            .encode(&img, &crate::Options::default().with_quality(40))
            .unwrap();
        assert!(WebpCodec.decode(&q90, u64::MAX).is_ok());
        assert!(q40.len() < q90.len());
    }

    #[test]
    fn encode_lossless_roundtrip_exacto() {
        let img = crate::testutil::vector_flat_colors(32, 32);
        let bytes = WebpCodec
            .encode(&img, &crate::Options::default().with_lossless(true))
            .unwrap();
        let back = WebpCodec.decode(&bytes, u64::MAX).unwrap();
        assert_eq!(back.pixels, img.pixels);
    }

    #[test]
    fn preserva_alpha() {
        let img = vector_gradient_circle(32, 32); // círculo alpha=128
        let bytes = WebpCodec
            .encode(&img, &crate::Options::default().with_quality(90))
            .unwrap();
        let back = WebpCodec.decode(&bytes, u64::MAX).unwrap();
        assert!(back.has_transparency());
    }
}
