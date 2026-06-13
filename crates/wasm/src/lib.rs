//! Binding WASM: SOLO conversión de tipos/errores. La lógica vive en minipix-core.
//!
//! PÁNICOS (carve-out de CLAUDE.md regla 4): en stable wasm32 `panic=abort`, un
//! pánico es un trap que mata la instancia — no hay `catch_unwind` posible. El
//! contrato con el worker JS: cualquier `RuntimeError` ⇒ instancia muerta ⇒
//! terminate + respawn del worker (ver playground worker pool).

#![allow(clippy::missing_errors_doc)] // wasm-bindgen fns use JsError, not doc-required
#![allow(clippy::missing_panics_doc)] // panic=abort en wasm32; carve-out documentado arriba

use serde::Deserialize;
use wasm_bindgen::prelude::*;

/// Espejo de las opciones del SDK (camelCase como en el binding Node).
#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct WasmOptions {
    /// Output format string: `"png"`, `"jpeg"`/`"jpg"`, `"webp"`, `"avif"`.
    pub format: Option<String>,
    /// Quality (1-100).
    pub quality: Option<u8>,
    /// Effort (0-9).
    pub effort: Option<u8>,
    /// Lossless mode.
    pub lossless: Option<bool>,
    /// Alpha channel quality (1-100).
    pub alpha_quality: Option<u8>,
    /// JPEG progressive encoding.
    pub jpeg_progressive: Option<bool>,
    /// Maximum pixel count (anti-bomb).
    pub max_pixels: Option<u64>,
}

fn parse_format(s: &str) -> Result<minipix_core::Format, JsError> {
    match s {
        "png" => Ok(minipix_core::Format::Png),
        "jpeg" | "jpg" => Ok(minipix_core::Format::Jpeg),
        "webp" => Ok(minipix_core::Format::WebP),
        "avif" => Ok(minipix_core::Format::Avif),
        other => Err(JsError::new(&format!(
            "[InvalidOptions] unknown format: {other}"
        ))),
    }
}

fn to_core(o: &WasmOptions) -> Result<minipix_core::Options, JsError> {
    let mut opts = minipix_core::Options::default();
    if let Some(f) = &o.format {
        opts.format = Some(parse_format(f)?);
    }
    if let Some(q) = o.quality {
        opts.quality = q;
    }
    if let Some(e) = o.effort {
        opts.effort = e;
    }
    if let Some(l) = o.lossless {
        opts.lossless = l;
    }
    if let Some(a) = o.alpha_quality {
        opts.alpha_quality = a;
    }
    if let Some(p) = o.jpeg_progressive {
        opts.jpeg_progressive = p;
    }
    if let Some(m) = o.max_pixels {
        opts.max_pixels = m;
    }
    Ok(opts)
}

fn map_err(e: &minipix_core::Error) -> JsError {
    use minipix_core::Error as E;
    // Mismo convenio [Code] message que el binding Node.
    let code = match e {
        E::UnsupportedFormat => "UnsupportedFormat",
        E::Decode { .. } => "DecodeError",
        E::Encode { .. } => "EncodeError",
        E::InvalidOptions(_) => "InvalidOptions",
        E::LimitExceeded { .. } => "LimitExceeded",
        E::IccTransform(_) => "IccTransform",
        _ => "Unknown",
    };
    JsError::new(&format!("[{code}] {e}"))
}

fn parse_options(options: JsValue) -> Result<WasmOptions, JsError> {
    if options.is_undefined() || options.is_null() {
        Ok(WasmOptions::default())
    } else {
        serde_wasm_bindgen::from_value(options)
            .map_err(|e| JsError::new(&format!("[InvalidOptions] failed to parse options: {e}")))
    }
}

// WasmOutput: wasm-bindgen 0.2.x NO permite `pub Vec<u8>` en structs (Vec no es Copy).
// Estrategia: todos los campos son privados; se exponen vía getters explícitos.
// Los getters con `js_name` garantizan que el .d.ts muestre `bytesIn`/`bytesOut`
// en camelCase. El campo `data` devuelve `Vec<u8>` (wasm-bindgen lo convierte a
// `Uint8Array` en el .d.ts generado).
/// Resultado de compresión/conversión para JS.
#[wasm_bindgen]
pub struct WasmOutput {
    data: Vec<u8>,
    format: String,
    width: u32,
    height: u32,
    bytes_in: f64,
    bytes_out: f64,
    ratio: f64,
}

#[wasm_bindgen]
impl WasmOutput {
    /// Encoded image bytes (returned as `Uint8Array` in JS).
    /// NOTA: cada lectura de .data copia el buffer — leer UNA vez y transferir.
    #[must_use]
    #[wasm_bindgen(getter)]
    pub fn data(&self) -> Vec<u8> {
        self.data.clone()
    }

    /// Output format name (`"png"`, `"jpeg"`, `"webp"`, or `"avif"`).
    #[must_use]
    #[wasm_bindgen(getter)]
    pub fn format(&self) -> String {
        self.format.clone()
    }

    /// Image width in pixels.
    #[must_use]
    #[wasm_bindgen(getter)]
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Image height in pixels.
    #[must_use]
    #[wasm_bindgen(getter)]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Input size in bytes.
    #[must_use]
    #[wasm_bindgen(getter, js_name = bytesIn)]
    pub fn bytes_in(&self) -> f64 {
        self.bytes_in
    }

    /// Output size in bytes.
    #[must_use]
    #[wasm_bindgen(getter, js_name = bytesOut)]
    pub fn bytes_out(&self) -> f64 {
        self.bytes_out
    }

    /// Compression ratio (`bytes_out` / `bytes_in`).
    #[must_use]
    #[wasm_bindgen(getter)]
    pub fn ratio(&self) -> f64 {
        self.ratio
    }
}

fn format_name(f: minipix_core::Format) -> &'static str {
    match f {
        minipix_core::Format::Png => "png",
        minipix_core::Format::Jpeg => "jpeg",
        minipix_core::Format::WebP => "webp",
        minipix_core::Format::Avif => "avif",
    }
}

#[allow(clippy::cast_precision_loss)] // image sizes << 2^52
fn to_output(out: minipix_core::Output) -> WasmOutput {
    let ratio = out.ratio();
    WasmOutput {
        data: out.data,
        format: format_name(out.format).to_owned(),
        width: out.width,
        height: out.height,
        bytes_in: out.bytes_in as f64,
        bytes_out: out.bytes_out as f64,
        ratio,
    }
}

/// Inicialización opcional de diagnóstico (solo build `debug-panics`).
#[wasm_bindgen(start)]
pub fn start() {
    #[cfg(feature = "debug-panics")]
    console_error_panic_hook::set_once();
}

/// Re-encodea en el mismo formato detectado.
#[wasm_bindgen]
pub fn compress(input: &[u8], options: JsValue) -> Result<WasmOutput, JsError> {
    let wasm_opts = parse_options(options)?;
    let core_opts = to_core(&wasm_opts)?;
    minipix_core::compress(input, &core_opts)
        .map(to_output)
        .map_err(|e| map_err(&e))
}

/// Transcodea al formato de `options.format`.
#[wasm_bindgen]
pub fn convert(input: &[u8], options: JsValue) -> Result<WasmOutput, JsError> {
    let wasm_opts = parse_options(options)?;
    let core_opts = to_core(&wasm_opts)?;
    minipix_core::convert(input, &core_opts)
        .map(to_output)
        .map_err(|e| map_err(&e))
}

/// Codifica RGBA8 crudo (el worker lo usa para inputs decodificados por el navegador).
#[wasm_bindgen(js_name = encodeRgba)]
pub fn encode_rgba(
    rgba: &[u8],
    width: u32,
    height: u32,
    format: &str,
    options: JsValue,
) -> Result<WasmOutput, JsError> {
    let fmt = parse_format(format)?;
    let wasm_opts = parse_options(options)?;
    let core_opts = to_core(&wasm_opts)?;
    minipix_core::encode_rgba(rgba, width, height, fmt, &core_opts)
        .map(to_output)
        .map_err(|e| map_err(&e))
}
