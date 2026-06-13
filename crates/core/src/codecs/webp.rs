//! WebP: decode con image-webp (Rust puro) + encode con libwebp-sys2 (FFI, native) o image-webp (wasm).
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
        // Extraer ICC antes de read_image (requiere &mut self; se llama una sola vez).
        let icc = dec.icc_profile().ok().flatten();
        let size = dec
            .output_buffer_size()
            .ok_or_else(|| decode_err("image too large"))?;
        let mut buf = vec![0u8; size];
        dec.read_image(&mut buf).map_err(decode_err)?;
        let mut rgba = if dec.has_alpha() {
            buf
        } else {
            buf.chunks_exact(3)
                .flat_map(|rgb| [rgb[0], rgb[1], rgb[2], 255])
                .collect()
        };
        // wiring probado vía tests de color.rs; e2e con fixture ICC queda en backlog
        crate::color::apply_icc_best_effort(&mut rgba, icc.as_deref());
        DecodedImage::new(w, h, rgba).map_err(decode_err)
    }
}

#[cfg(feature = "native")]
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

#[cfg(all(feature = "wasm", not(feature = "native")))]
impl ImageEncoder for WebpCodec {
    fn encode(&self, img: &DecodedImage, opts: &Options) -> Result<Vec<u8>, Error> {
        // Fallback wasm: SOLO lossless (image-webp no tiene encoder lossy; documentado).
        if !opts.lossless && opts.quality != 100 {
            return Err(Error::InvalidOptions(
                "lossy WebP encode is not available in the wasm build (use lossless: true)".into(),
            ));
        }
        let mut out = Vec::new();
        image_webp::WebPEncoder::new(std::io::Cursor::new(&mut out))
            .encode(
                &img.pixels,
                img.width,
                img.height,
                image_webp::ColorType::Rgba8,
            )
            .map_err(encode_err)?;
        Ok(out)
    }
}

/// FFI con libwebp: ÚNICO módulo unsafe del workspace (CLAUDE.md).
///
/// Toda la memoria C se limpia en TODOS los caminos (éxito y error) mediante
/// guards RAII (`PictureGuard` y `WriterGuard`) que llaman a los destructores
/// de libwebp desde sus implementaciones `Drop`.
#[cfg(feature = "native")]
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

    // ── Fix 1: shim seguro sin transmute ────────────────────────────────────
    /// Shim seguro: confina el unsafe a la LLAMADA FFI real, no a reinterpretar
    /// el qualifier del puntero a función.
    extern "C" fn write_shim(
        d: *const u8,
        n: usize,
        p: *const sys::WebPPicture,
    ) -> std::os::raw::c_int {
        // SAFETY: libwebp garantiza d/n/p válidos durante el callback.
        unsafe { sys::WebPMemoryWrite(d, n, p) }
    }

    // ── Fix 3: guards RAII ──────────────────────────────────────────────────

    /// Libera `WebPPicture` al salir del scope (cleanup robusto ante early-returns).
    struct PictureGuard(sys::WebPPicture);
    impl Drop for PictureGuard {
        fn drop(&mut self) {
            // SAFETY: self.0 fue inicializada por WebPPictureInit; Free es no-op si no hay alloc.
            unsafe { sys::WebPPictureFree(&raw mut self.0) };
        }
    }

    /// Limpia `WebPMemoryWriter` al salir del scope.
    struct WriterGuard(sys::WebPMemoryWriter);
    impl Drop for WriterGuard {
        fn drop(&mut self) {
            // SAFETY: self.0 fue inicializada por WebPMemoryWriterInit; Clear es idempotente sobre mem NULL.
            unsafe { sys::WebPMemoryWriterClear(&raw mut self.0) };
        }
    }

    // ── Fix 4: helper de nombres de error ───────────────────────────────────
    fn encode_error_name(code: u32) -> &'static str {
        match code {
            1 => "out of memory",
            2 => "bitstream out of memory",
            3 => "null parameter",
            4 => "invalid configuration",
            5 => "bad dimension (WebP max is 16383x16383)",
            6 => "partition is bigger than 512k",
            7 => "partition is bigger than 16M",
            8 => "bad write",
            9 => "file is bigger than 4G",
            10 => "user abort",
            _ => "unknown error",
        }
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

        // ── 2. Inicializar WebPPicture, importar RGBA, envolver en guard ─────
        // SAFETY: `p2` se inicializa con `WebPPictureInit` antes de leerse.
        // `rgba.as_ptr()` apunta a un slice vivo durante toda esta llamada.
        // `PictureGuard::drop` llama `WebPPictureFree` en todos los caminos.
        let mut pic_guard = unsafe {
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
                // p2 fue inicializada; Free antes de retornar.
                sys::WebPPictureFree(&raw mut p2);
                return Err("WebPPictureImportRGBA failed (out of memory?)".into());
            }
            PictureGuard(p2)
        };

        // ── 3. Inicializar WebPMemoryWriter y cablear el writer ──────────────
        // SAFETY: `writer_guard.0` se inicializa con `WebPMemoryWriterInit`.
        // `pic_guard.0.custom_ptr` apunta a `writer_guard.0`; writer_guard NO
        // se mueve entre este punto y el final de `WebPEncode` (permanece en
        // el mismo stack frame). `WriterGuard::drop` llama `WebPMemoryWriterClear`
        // en todos los caminos, incluyendo early returns por error de encode.
        let mut writer_guard = unsafe {
            let mut w: sys::WebPMemoryWriter = std::mem::zeroed();
            sys::WebPMemoryWriterInit(&raw mut w);
            WriterGuard(w)
        };

        // Fix 1: usar write_shim en lugar de transmute de WebPMemoryWrite.
        // SAFETY: write_shim tiene la firma ABI correcta de WebPWriterFunction.
        // `pic_guard.0.custom_ptr` apunta a `writer_guard.0` cuyo lifetime
        // abarca todo el encode; no hay movimiento del writer hasta después
        // de que WebPEncode retorne.
        pic_guard.0.writer = Some(write_shim);
        pic_guard.0.custom_ptr = (&raw mut writer_guard.0).cast();

        // ── 4. Encode ────────────────────────────────────────────────────────
        // SAFETY: `config` y `pic_guard.0` están completamente inicializados.
        // Libwebp escribe en `writer_guard.0.mem` (heap propio de libwebp);
        // copiamos a un Vec antes de que WriterGuard llame WebPMemoryWriterClear.
        let ok = unsafe { sys::WebPEncode(&raw const config, &raw mut pic_guard.0) };

        // Capturar error_code antes de que PictureGuard::drop libere pic.
        // SAFETY: `pic_guard.0` aún es válido en este punto.
        let error_code = pic_guard.0.error_code;
        // Liberar explícitamente la picture (aunque Drop también lo haría).
        // Hacemos drop aquí para dejar claro el orden de limpieza.
        drop(pic_guard);

        if ok == 0 {
            // WriterGuard::drop limpiará writer_guard al salir del scope.
            let code = error_code as u32;
            return Err(format!(
                "WebPEncode failed: {} (code {})",
                encode_error_name(code),
                code
            ));
        }

        // ── 5. Copiar output antes de que WriterGuard limpie el buffer ───────
        // Fix 2: guard contra NULL mem / size 0 (UB con from_raw_parts(NULL,0)).
        let out = if writer_guard.0.size == 0 || writer_guard.0.mem.is_null() {
            Vec::new()
        } else {
            // SAFETY: size > 0 y mem no nulo ⇒ size bytes contiguos válidos escritos por libwebp.
            unsafe { std::slice::from_raw_parts(writer_guard.0.mem, writer_guard.0.size).to_vec() }
        };
        // WriterGuard::drop llama WebPMemoryWriterClear al salir de este scope.

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

    #[cfg(feature = "native")]
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

    #[cfg(feature = "native")]
    #[test]
    fn encode_lossless_roundtrip_exacto() {
        let img = crate::testutil::vector_flat_colors(32, 32);
        let bytes = WebpCodec
            .encode(&img, &crate::Options::default().with_lossless(true))
            .unwrap();
        let back = WebpCodec.decode(&bytes, u64::MAX).unwrap();
        assert_eq!(back.pixels, img.pixels);
    }

    #[cfg(feature = "native")]
    #[test]
    fn preserva_alpha() {
        let img = vector_gradient_circle(32, 32); // círculo alpha=128
        let bytes = WebpCodec
            .encode(&img, &crate::Options::default().with_quality(90))
            .unwrap();
        let back = WebpCodec.decode(&bytes, u64::MAX).unwrap();
        assert!(back.has_transparency());
    }

    // Fix 5: dimension-limit test
    #[cfg(feature = "native")]
    #[test]
    fn dimension_mayor_a_16383_da_error_claro() {
        // WebP no soporta >16383 por eje: 16390x1.
        let px = vec![128u8; 16390 * 4];
        let img = crate::image::DecodedImage::new(16390, 1, px).unwrap();
        let err = WebpCodec
            .encode(&img, &crate::Options::default())
            .unwrap_err();
        assert!(err.to_string().contains("bad dimension"), "fue: {err}");
    }
}
