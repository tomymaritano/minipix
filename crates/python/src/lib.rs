//! Binding Python: SOLO conversión de tipos/errores. La lógica vive en minipix-core.
//!
//! # Panic safety
//! `PyO3` installs a `catch_unwind` trampoline at every FFI boundary (the `#[pyfunction]`
//! and `#[pymodule]` macros wrap generated code in `std::panic::catch_unwind`). Any Rust
//! panic that would otherwise cross the boundary is converted automatically into a Python
//! `pyo3.PanicException`. This guarantee is documented in `PyO3`'s "Panics" section
//! (<https://pyo3.rs/v0.29.0/exception.html#panics>). No manual `catch_unwind` is needed
//! here — unlike the Node binding which wraps libuv threadpool work items manually.
#![allow(missing_docs)] // pyo3 macros generan items sin documentar

use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use pyo3::types::PyBytes;

create_exception!(
    minipix,
    MinipixError,
    PyException,
    "Base error for minipix."
);
create_exception!(
    minipix,
    UnsupportedFormatError,
    MinipixError,
    "Unrecognized input format."
);
create_exception!(
    minipix,
    CodecError,
    MinipixError,
    "Decode or encode failure."
);
create_exception!(
    minipix,
    LimitExceededError,
    MinipixError,
    "Pixel limit exceeded."
);

fn map_err(e: &minipix_core::Error) -> PyErr {
    use minipix_core::Error as E;
    match e {
        E::UnsupportedFormat => UnsupportedFormatError::new_err(e.to_string()),
        E::Decode { .. } | E::Encode { .. } | E::IccTransform(_) => {
            // IccTransform se agrupa como CodecError a propósito (falla a nivel códec).
            CodecError::new_err(e.to_string())
        }
        E::InvalidOptions(_) => pyo3::exceptions::PyValueError::new_err(e.to_string()),
        E::LimitExceeded { .. } => LimitExceededError::new_err(e.to_string()),
        _ => MinipixError::new_err(e.to_string()), // non_exhaustive guard
    }
}

fn parse_format(s: &str) -> PyResult<minipix_core::Format> {
    match s {
        "png" => Ok(minipix_core::Format::Png),
        "jpeg" | "jpg" => Ok(minipix_core::Format::Jpeg),
        "webp" => Ok(minipix_core::Format::WebP),
        "avif" => Ok(minipix_core::Format::Avif),
        other => Err(pyo3::exceptions::PyValueError::new_err(format!(
            "unknown format: {other}"
        ))),
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

/// Resultado de compress/convert.
#[pyclass(frozen, get_all)]
pub struct Output {
    /// Bytes codificados del formato de salida.
    data: Py<PyBytes>,
    /// Nombre del formato de salida.
    format: String,
    /// Ancho en píxeles.
    width: u32,
    /// Alto en píxeles.
    height: u32,
    /// Tamaño del input en bytes.
    bytes_in: u64,
    /// Tamaño del output en bytes.
    bytes_out: u64,
    /// `bytes_out / bytes_in`.
    ratio: f64,
}

#[allow(clippy::too_many_arguments)]
fn build_options(
    format: Option<&str>,
    quality: Option<u8>,
    effort: Option<u8>,
    lossless: Option<bool>,
    alpha_quality: Option<u8>,
    jpeg_progressive: Option<bool>,
    max_pixels: Option<u64>,
) -> PyResult<minipix_core::Options> {
    let mut o = minipix_core::Options::default();
    if let Some(f) = format {
        o.format = Some(parse_format(f)?);
    }
    if let Some(q) = quality {
        o.quality = q;
    }
    if let Some(e) = effort {
        o.effort = e;
    }
    if let Some(l) = lossless {
        o.lossless = l;
    }
    if let Some(a) = alpha_quality {
        o.alpha_quality = a;
    }
    if let Some(p) = jpeg_progressive {
        o.jpeg_progressive = p;
    }
    if let Some(m) = max_pixels {
        o.max_pixels = m;
    }
    Ok(o)
}

fn run(
    py: Python<'_>,
    data: &[u8],
    opts: &minipix_core::Options,
    is_convert: bool,
) -> PyResult<Output> {
    let f = if is_convert {
        minipix_core::convert
    } else {
        minipix_core::compress
    };
    // GIL liberado durante el trabajo CPU (threading real en Python).
    // En PyO3 0.29 el método se llama `detach` (renombrado desde `allow_threads`).
    let out = py.detach(|| f(data, opts)).map_err(|e| map_err(&e))?;
    Ok(Output {
        ratio: out.ratio(),
        format: format_name(out.format).to_string(),
        width: out.width,
        height: out.height,
        bytes_in: out.bytes_in,
        bytes_out: out.bytes_out,
        // PyBytes::new returns Bound<'_, PyBytes> in PyO3 0.29; .unbind() gives Py<PyBytes>.
        data: PyBytes::new(py, &out.data).unbind(),
    })
}

/// Re-encodea optimizando en el mismo formato.
///
/// Raises `ValueError` for invalid options; `CodecError` for decode/encode failures; `LimitExceededError` if `max_pixels` exceeded.
#[pyfunction]
#[pyo3(signature = (data, *, quality=None, effort=None, lossless=None, alpha_quality=None, jpeg_progressive=None, max_pixels=None))]
#[allow(clippy::too_many_arguments)]
fn compress(
    py: Python<'_>,
    data: &[u8],
    quality: Option<u8>,
    effort: Option<u8>,
    lossless: Option<bool>,
    alpha_quality: Option<u8>,
    jpeg_progressive: Option<bool>,
    max_pixels: Option<u64>,
) -> PyResult<Output> {
    let opts = build_options(
        None,
        quality,
        effort,
        lossless,
        alpha_quality,
        jpeg_progressive,
        max_pixels,
    )?;
    run(py, data, &opts, false)
}

/// Transcodea al formato indicado.
///
/// Raises `ValueError` for invalid options or unknown format; `CodecError` for decode/encode failures; `LimitExceededError` if `max_pixels` exceeded.
#[pyfunction]
#[pyo3(signature = (data, *, format, quality=None, effort=None, lossless=None, alpha_quality=None, jpeg_progressive=None, max_pixels=None))]
#[allow(clippy::too_many_arguments)]
fn convert(
    py: Python<'_>,
    data: &[u8],
    format: &str,
    quality: Option<u8>,
    effort: Option<u8>,
    lossless: Option<bool>,
    alpha_quality: Option<u8>,
    jpeg_progressive: Option<bool>,
    max_pixels: Option<u64>,
) -> PyResult<Output> {
    let opts = build_options(
        Some(format),
        quality,
        effort,
        lossless,
        alpha_quality,
        jpeg_progressive,
        max_pixels,
    )?;
    run(py, data, &opts, true)
}

#[pymodule]
fn minipix(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(compress, m)?)?;
    m.add_function(wrap_pyfunction!(convert, m)?)?;
    m.add_class::<Output>()?;
    // Register typed exception classes so Python callers can catch them by name.
    // py.get_type::<T>() returns Bound<'_, PyType> which m.add() accepts.
    let py = m.py();
    m.add("MinipixError", py.get_type::<MinipixError>())?;
    m.add(
        "UnsupportedFormatError",
        py.get_type::<UnsupportedFormatError>(),
    )?;
    m.add("CodecError", py.get_type::<CodecError>())?;
    m.add("LimitExceededError", py.get_type::<LimitExceededError>())?;
    Ok(())
}
