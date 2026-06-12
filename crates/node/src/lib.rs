//! Node.js binding: SOLO conversión de tipos/errores. La lógica vive en minipix-core.
#![allow(missing_docs)] // napi macros generan ítems sin documentar

use napi::bindgen_prelude::*;
use napi_derive::napi;

/// Opciones de compresión/conversión (espejo de `minipix_core::Options`).
#[napi(object)]
#[derive(Default)]
pub struct MinipixOptions {
    /// Formato destino: "png", "jpeg", "webp", "avif". None = mismo formato.
    pub format: Option<String>,
    /// Calidad 1–100.
    pub quality: Option<u32>,
    /// Esfuerzo CPU 0–9.
    pub effort: Option<u32>,
    /// Modo sin pérdida.
    pub lossless: Option<bool>,
    /// Calidad alpha 1–100 (WebP/AVIF).
    pub alpha_quality: Option<u32>,
    /// JPEG progresivo.
    pub jpeg_progressive: Option<bool>,
    /// Límite de píxeles (anti-bomba).
    pub max_pixels: Option<f64>,
}

/// Resultado con bytes + reporte.
#[napi(object)]
pub struct MinipixOutput {
    /// Bytes codificados del formato de salida.
    pub data: Buffer,
    /// Nombre del formato de salida.
    pub format: String,
    /// Ancho en píxeles.
    pub width: u32,
    /// Alto en píxeles.
    pub height: u32,
    /// Tamaño del input en bytes.
    pub bytes_in: f64,
    /// Tamaño del output en bytes.
    pub bytes_out: f64,
    /// `bytes_out` / `bytes_in`.
    pub ratio: f64,
}

fn parse_format(s: &str) -> Result<minipix_core::Format> {
    match s {
        "png" => Ok(minipix_core::Format::Png),
        "jpeg" | "jpg" => Ok(minipix_core::Format::Jpeg),
        "webp" => Ok(minipix_core::Format::WebP),
        "avif" => Ok(minipix_core::Format::Avif),
        other => Err(Error::new(
            Status::InvalidArg,
            format!("unknown format: {other}"),
        )),
    }
}

fn to_core_options(o: &MinipixOptions) -> Result<minipix_core::Options> {
    let mut opts = minipix_core::Options::default();
    if let Some(f) = &o.format {
        opts.format = Some(parse_format(f)?);
    }
    if let Some(q) = o.quality {
        opts.quality = u8::try_from(q).map_err(invalid)?;
    }
    if let Some(e) = o.effort {
        opts.effort = u8::try_from(e).map_err(invalid)?;
    }
    if let Some(l) = o.lossless {
        opts.lossless = l;
    }
    if let Some(a) = o.alpha_quality {
        opts.alpha_quality = u8::try_from(a).map_err(invalid)?;
    }
    if let Some(p) = o.jpeg_progressive {
        opts.jpeg_progressive = p;
    }
    if let Some(m) = o.max_pixels {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // límite práctico < 2^53
        {
            opts.max_pixels = m as u64;
        }
    }
    Ok(opts)
}

fn invalid(e: impl std::fmt::Display) -> Error {
    Error::new(Status::InvalidArg, e.to_string())
}

fn map_err(e: &minipix_core::Error) -> Error {
    let status = match e {
        minipix_core::Error::InvalidOptions(_) => Status::InvalidArg,
        _ => Status::GenericFailure,
    };
    Error::new(status, e.to_string())
}

fn format_name(f: minipix_core::Format) -> &'static str {
    match f {
        minipix_core::Format::Png => "png",
        minipix_core::Format::Jpeg => "jpeg",
        minipix_core::Format::WebP => "webp",
        minipix_core::Format::Avif => "avif",
    }
}

/// Trabajo CPU-bound ejecutado en el threadpool de libuv.
pub struct Work {
    input: Vec<u8>,
    opts: minipix_core::Options,
    is_convert: bool,
}

impl Task for Work {
    type Output = minipix_core::Output;
    type JsValue = MinipixOutput;

    fn compute(&mut self) -> Result<Self::Output> {
        let run = if self.is_convert {
            minipix_core::convert
        } else {
            minipix_core::compress
        };
        run(&self.input, &self.opts).map_err(|e| map_err(&e))
    }

    fn resolve(&mut self, _env: Env, out: Self::Output) -> Result<Self::JsValue> {
        #[allow(clippy::cast_precision_loss)] // tamaños de imagen << 2^52
        Ok(MinipixOutput {
            ratio: out.ratio(),
            format: format_name(out.format).into(),
            width: out.width,
            height: out.height,
            bytes_in: out.bytes_in as f64,
            bytes_out: out.bytes_out as f64,
            data: out.data.into(),
        })
    }
}

/// Re-encodea optimizando en el mismo formato (async).
///
/// # Errors
/// Returns an error if options are invalid or if the input cannot be decoded.
#[napi(ts_return_type = "Promise<MinipixOutput>")]
#[allow(clippy::needless_pass_by_value)] // napi-rs requiere Buffer/Option<T> por valor
pub fn compress(input: Buffer, options: Option<MinipixOptions>) -> Result<AsyncTask<Work>> {
    let opts = to_core_options(&options.unwrap_or_default())?;
    Ok(AsyncTask::new(Work {
        input: input.to_vec(),
        opts,
        is_convert: false,
    }))
}

/// Transcodea al formato de options.format (async).
///
/// # Errors
/// Returns an error if options are invalid, the format is unrecognized, or the input cannot be decoded.
#[napi(ts_return_type = "Promise<MinipixOutput>")]
#[allow(clippy::needless_pass_by_value)] // napi-rs requiere Buffer/MinipixOptions por valor
pub fn convert(input: Buffer, options: MinipixOptions) -> Result<AsyncTask<Work>> {
    let opts = to_core_options(&options)?;
    Ok(AsyncTask::new(Work {
        input: input.to_vec(),
        opts,
        is_convert: true,
    }))
}
