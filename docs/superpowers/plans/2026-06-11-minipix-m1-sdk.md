# minipix M1 (SDK) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rust core que comprime/convierte PNG, JPEG, WebP y AVIF, publicado en crates.io, npm (napi-rs) y PyPI (PyO3/maturin), con paridad byte a byte entre los tres bindings.

**Architecture:** Workspace Cargo con `crates/core` (toda la lógica: sniff → Decoder → `DecodedImage` RGBA8 sRGB → Encoder), `crates/node` y `crates/python` como bindings finos. Códecs detrás de traits `ImageDecoder`/`ImageEncoder`. Ver spec: `docs/superpowers/specs/2026-06-11-minipix-design.md` y reglas en `CLAUDE.md` (lint estricto, TDD, sin `unwrap()` en código de librería, `unsafe` solo en FFI con `// SAFETY:`).

**Tech Stack:** Rust (edition 2024) · png 0.18 + oxipng 10 + quantette 0.6 · zune-jpeg 0.5 + mozjpeg 0.10 · image-webp 0.2 + libwebp-sys2 · ravif 0.13 + avif-decode 1.0 · moxcms (ICC→sRGB) · napi-rs v3 · PyO3 + maturin (abi3-py39).

**Desviaciones del spec (verificadas contra docs.rs, requieren amend del spec):**

1. `keepMetadata` se difiere a v2: png 0.18 no expone escritura de iCCP y ravif no embebe ICC. En v1 el color se resuelve convirtiendo a sRGB al decodificar (ICC se *aplica*, nunca se rompe) y EXIF/XMP siempre se elimina.
2. AVIF decode: `avif-decode` (kornelski) envuelve **aom-decode/libaom**, no dav1d — reutiliza el toolchain cmake+nasm que ya exigen mozjpeg/libwebp (no suma meson). Es el camino elegido; `rav1d` queda como swap futuro.
3. **Trait `ImageDecoder` recibe `max_pixels`** (review de Task 6): `decode(&self, data: &[u8], max_pixels: u64)`. Cada decoder valida dimensiones tan temprano como su API lo permite y devuelve `LimitExceeded` ANTES de asignar el buffer de salida (defensa contra bombas de dimensiones aun si un caller futuro saltea el `peek` del Task 15, p.ej. fuzz targets). El `peek` del Task 15 se mantiene como capa rápida. Los snippets de las Tasks 7-13 y 15 se adaptan a la nueva firma.

**Convención de cada tarea:** TDD — test primero, ver que falla, implementar mínimo, ver que pasa, commit. Comandos desde la raíz del repo. `cargo test -p minipix-core` corre los tests del core.

---

## Fase A — Esqueleto y tipos

### Task 1: Workspace + crate core + lints + CI básico

**Files:**

- Create: `Cargo.toml` (raíz, workspace)
- Create: `crates/core/Cargo.toml`
- Create: `crates/core/src/lib.rs`
- Create: `.github/workflows/ci.yml`

- [ ] **Step 1: Workspace raíz**

```toml
# Cargo.toml (raíz)
[workspace]
resolver = "2"
members = ["crates/core"]

[workspace.package]
version = "0.1.0"
edition = "2024"
license = "MIT OR Apache-2.0"
repository = "https://github.com/tomymaritano/minipix"

[workspace.lints.rust]
unsafe_code = "deny"          # crates/webp lo re-permite por módulo con justificación
missing_docs = "warn"

[workspace.lints.clippy]
all = { level = "deny", priority = -1 }
pedantic = { level = "warn", priority = -1 }
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
```

- [ ] **Step 2: Crate core mínimo**

```toml
# crates/core/Cargo.toml
[package]
name = "minipix-core"
description = "Image compression/conversion core: PNG, JPEG, WebP, AVIF"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true

[dependencies]
thiserror = "2"

[lints]
workspace = true
```

```rust
// crates/core/src/lib.rs
//! minipix core: compresión y conversión de imágenes (PNG, JPEG, WebP, AVIF).
```

- [ ] **Step 3: Verificar que compila y el lint pasa**

Run: `cargo build --workspace; cargo clippy --workspace --all-targets -- -D warnings; cargo fmt --check`
Expected: éxito sin warnings.

- [ ] **Step 4: CI de lint+test**

```yaml
# .github/workflows/ci.yml
name: ci
on:
  push: { branches: [master] }
  pull_request:
jobs:
  lint-test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { components: "clippy, rustfmt" }
      - uses: ilammy/setup-nasm@v1          # mozjpeg (tareas posteriores)
      - run: cargo fmt --check
      - run: cargo clippy --workspace --all-targets -- -D warnings
      - run: cargo test --workspace
      - uses: EmbarkStudios/cargo-deny-action@v2
```

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat: cargo workspace + minipix-core skeleton + strict lints + CI"
```

### Task 2: `Format` + sniff por magic bytes

**Files:**

- Create: `crates/core/src/format.rs`
- Create: `crates/core/src/sniff.rs`
- Modify: `crates/core/src/lib.rs`

- [ ] **Step 1: Test que falla**

```rust
// crates/core/src/sniff.rs  (módulo con sus tests abajo)
#[cfg(test)]
mod tests {
    use super::sniff;
    use crate::format::Format;

    #[test]
    fn detecta_png() {
        let mut data = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        data.extend([0u8; 16]);
        assert_eq!(sniff(&data), Some(Format::Png));
    }
    #[test]
    fn detecta_jpeg() {
        let mut data = vec![0xFF, 0xD8, 0xFF, 0xE0];
        data.extend([0u8; 16]);
        assert_eq!(sniff(&data), Some(Format::Jpeg));
    }
    #[test]
    fn detecta_webp() {
        let mut data = b"RIFF\x00\x00\x00\x00WEBP".to_vec();
        data.extend([0u8; 16]);
        assert_eq!(sniff(&data), Some(Format::WebP));
    }
    #[test]
    fn detecta_avif() {
        // caja ftyp ISO-BMFF: [size:4]["ftyp"][brand:4]
        let mut data = vec![0x00, 0x00, 0x00, 0x1C];
        data.extend(b"ftypavif");
        data.extend([0u8; 20]);
        assert_eq!(sniff(&data), Some(Format::Avif));
    }
    #[test]
    fn rechaza_basura_y_vacio() {
        assert_eq!(sniff(b"hola mundo!!"), None);
        assert_eq!(sniff(&[]), None);
    }
}
```

- [ ] **Step 2: Verificar que falla** — Run: `cargo test -p minipix-core` → FAIL (módulos inexistentes).

- [ ] **Step 3: Implementación mínima**

```rust
// crates/core/src/format.rs
/// Formato de imagen soportado.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Format {
    Png,
    Jpeg,
    WebP,
    Avif,
}
```

```rust
// crates/core/src/sniff.rs (encima de los tests)
use crate::format::Format;

/// Detecta el formato por magic bytes. `None` si no se reconoce.
pub fn sniff(data: &[u8]) -> Option<Format> {
    if data.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        return Some(Format::Png);
    }
    if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some(Format::Jpeg);
    }
    if data.len() >= 12 && &data[0..4] == b"RIFF" && &data[8..12] == b"WEBP" {
        return Some(Format::WebP);
    }
    // ISO-BMFF: bytes 4..8 = "ftyp" y major/compatible brand avif/avis.
    if data.len() >= 12 && &data[4..8] == b"ftyp" && (&data[8..12] == b"avif" || &data[8..12] == b"avis") {
        return Some(Format::Avif);
    }
    None
}
```

```rust
// crates/core/src/lib.rs — agregar
pub mod format;
pub mod sniff;
pub use format::Format;
```

- [ ] **Step 4: Verificar que pasa** — Run: `cargo test -p minipix-core` → PASS (5 tests).
- [ ] **Step 5: Commit** — `git add -A && git commit -m "feat(core): Format enum + sniff por magic bytes"`

### Task 3: `Error`, `Options`, `Output`

**Files:**

- Create: `crates/core/src/error.rs`
- Create: `crates/core/src/options.rs`
- Modify: `crates/core/src/lib.rs`

- [ ] **Step 1: Tests que fallan (validación de opciones)**

```rust
// crates/core/src/options.rs — tests al pie del módulo
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_correctos() {
        let o = Options::default();
        assert_eq!(o.quality, 75);
        assert_eq!(o.effort, 4);
        assert!(!o.lossless);
        assert_eq!(o.max_pixels, 268_435_456);
        assert!(o.format.is_none());
    }
    #[test]
    fn builder_encadena() {
        let o = Options::default().with_quality(60).with_effort(9).with_lossless(true);
        assert_eq!((o.quality, o.effort, o.lossless), (60, 9, true));
    }
    #[test]
    fn valida_rangos() {
        assert!(Options::default().with_quality(0).validate().is_err());
        assert!(Options::default().with_quality(101).validate().is_err());
        assert!(Options::default().with_effort(10).validate().is_err());
        assert!(Options::default().validate().is_ok());
    }
}
```

- [ ] **Step 2: Verificar que falla** — `cargo test -p minipix-core` → FAIL.

- [ ] **Step 3: Implementación**

```rust
// crates/core/src/error.rs
use crate::format::Format;

/// Error del core. Los bindings lo mapean a errores idiomáticos.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("unsupported or unrecognized image format")]
    UnsupportedFormat,
    #[error("decode failed ({format:?}): {detail}")]
    Decode { format: Format, detail: String },
    #[error("encode failed ({format:?}): {detail}")]
    Encode { format: Format, detail: String },
    #[error("invalid options: {0}")]
    InvalidOptions(String),
    #[error("image exceeds the configured pixel limit ({0} pixels)")]
    LimitExceeded(u64),
}
```

```rust
// crates/core/src/options.rs
use crate::error::Error;
use crate::format::Format;

/// Opciones unificadas (mismo vocabulario en Rust, Node y Python).
#[derive(Debug, Clone)]
pub struct Options {
    /// Formato destino. `None` = mismo formato de entrada (`compress`).
    pub format: Option<Format>,
    /// 1–100. Mapeada por códec (tabla en `codecs/`).
    pub quality: u8,
    /// 0–9: CPU invertido en reducir bytes.
    pub effort: u8,
    /// Fuerza camino sin pérdida (JPEG lo rechaza).
    pub lossless: bool,
    /// Defensa anti-bomba de descompresión.
    pub max_pixels: u64,
    /// JPEG progresivo (default true).
    pub jpeg_progressive: bool,
    /// Calidad del canal alpha en WebP/AVIF (1–100, default 100).
    pub alpha_quality: u8,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            format: None,
            quality: 75,
            effort: 4,
            lossless: false,
            max_pixels: 268_435_456,
            jpeg_progressive: true,
            alpha_quality: 100,
        }
    }
}

impl Options {
    #[must_use] pub fn with_format(mut self, f: Format) -> Self { self.format = Some(f); self }
    #[must_use] pub fn with_quality(mut self, q: u8) -> Self { self.quality = q; self }
    #[must_use] pub fn with_effort(mut self, e: u8) -> Self { self.effort = e; self }
    #[must_use] pub fn with_lossless(mut self, l: bool) -> Self { self.lossless = l; self }

    /// Valida rangos. Llamada por `compress`/`convert` antes de trabajar.
    pub fn validate(&self) -> Result<(), Error> {
        if !(1..=100).contains(&self.quality) {
            return Err(Error::InvalidOptions(format!("quality must be 1-100, got {}", self.quality)));
        }
        if self.effort > 9 {
            return Err(Error::InvalidOptions(format!("effort must be 0-9, got {}", self.effort)));
        }
        if !(1..=100).contains(&self.alpha_quality) {
            return Err(Error::InvalidOptions(format!("alpha_quality must be 1-100, got {}", self.alpha_quality)));
        }
        Ok(())
    }
}

/// Resultado de `compress`/`convert`.
#[derive(Debug)]
pub struct Output {
    pub data: Vec<u8>,
    pub format: Format,
    pub width: u32,
    pub height: u32,
    pub bytes_in: u64,
    pub bytes_out: u64,
}

impl Output {
    /// bytes_out / bytes_in (1.0 = sin cambio; < 1.0 = ahorro).
    #[must_use]
    pub fn ratio(&self) -> f64 {
        if self.bytes_in == 0 { return 1.0; }
        #[allow(clippy::cast_precision_loss)] // tamaños de imagen << 2^52
        { self.bytes_out as f64 / self.bytes_in as f64 }
    }
}
```

```rust
// crates/core/src/lib.rs — agregar
pub mod error;
pub mod options;
pub use error::Error;
pub use options::{Options, Output};
```

- [ ] **Step 4: Verificar que pasa** — `cargo test -p minipix-core` → PASS.
- [ ] **Step 5: Commit** — `git commit -am "feat(core): Error, Options con validacion, Output"`

### Task 4: `DecodedImage` + helpers RGBA

**Files:**

- Create: `crates/core/src/image.rs`
- Modify: `crates/core/src/lib.rs`

- [ ] **Step 1: Tests que fallan**

```rust
// crates/core/src/image.rs — tests al pie
#[cfg(test)]
mod tests {
    use super::*;

    fn img_2x1(px: [[u8; 4]; 2]) -> DecodedImage {
        DecodedImage::new(2, 1, px.concat()).unwrap()
    }

    #[test]
    fn valida_largo_del_buffer() {
        assert!(DecodedImage::new(2, 2, vec![0; 16]).is_ok());
        assert!(DecodedImage::new(2, 2, vec![0; 15]).is_err());
    }
    #[test]
    fn to_rgb_compone_sobre_blanco() {
        // alpha 0 => blanco puro; alpha 255 => color intacto
        let img = img_2x1([[200, 0, 0, 255], [200, 0, 0, 0]]);
        assert_eq!(img.to_rgb_over_white(), vec![200, 0, 0, 255, 255, 255]);
    }
    #[test]
    fn detecta_alpha_real() {
        assert!(!img_2x1([[1, 2, 3, 255], [4, 5, 6, 255]]).has_transparency());
        assert!(img_2x1([[1, 2, 3, 255], [4, 5, 6, 128]]).has_transparency());
    }
}
```

- [ ] **Step 2: Verificar que falla** — `cargo test -p minipix-core` → FAIL.

- [ ] **Step 3: Implementación**

```rust
// crates/core/src/image.rs
use crate::error::Error;

/// Imagen decodificada: SIEMPRE RGBA8 en sRGB (los decoders normalizan).
#[derive(Debug, Clone)]
pub struct DecodedImage {
    pub width: u32,
    pub height: u32,
    /// len == width * height * 4
    pub pixels: Vec<u8>,
}

impl DecodedImage {
    pub fn new(width: u32, height: u32, pixels: Vec<u8>) -> Result<Self, Error> {
        let expected = (u64::from(width)) * (u64::from(height)) * 4;
        if pixels.len() as u64 != expected {
            return Err(Error::InvalidOptions(format!(
                "pixel buffer length {} != {expected} for {width}x{height} RGBA8",
                pixels.len()
            )));
        }
        Ok(Self { width, height, pixels })
    }

    #[must_use]
    pub fn pixel_count(&self) -> u64 {
        u64::from(self.width) * u64::from(self.height)
    }

    /// JPEG no tiene alpha: compone sobre blanco (comportamiento TinyPNG/Squoosh).
    #[must_use]
    pub fn to_rgb_over_white(&self) -> Vec<u8> {
        let mut rgb = Vec::with_capacity(self.pixels.len() / 4 * 3);
        for px in self.pixels.chunks_exact(4) {
            let a = u16::from(px[3]);
            for c in &px[0..3] {
                let v = (u16::from(*c) * a + 255 * (255 - a)) / 255;
                #[allow(clippy::cast_possible_truncation)] // v <= 255 por construcción
                rgb.push(v as u8);
            }
        }
        rgb
    }

    /// true si algún píxel tiene alpha < 255 (decide RGB vs RGBA en encoders).
    #[must_use]
    pub fn has_transparency(&self) -> bool {
        self.pixels.chunks_exact(4).any(|px| px[3] != 255)
    }
}
```

```rust
// crates/core/src/lib.rs — agregar
pub mod image;
pub use image::DecodedImage;
```

- [ ] **Step 4: Verificar que pasa** — `cargo test -p minipix-core` → PASS.
- [ ] **Step 5: Commit** — `git commit -am "feat(core): DecodedImage RGBA8 + composicion sobre blanco"`

### Task 5: Traits de códec + vectores de prueba programáticos

**Files:**

- Create: `crates/core/src/codecs/mod.rs`
- Create: `crates/core/src/testutil.rs`
- Modify: `crates/core/src/lib.rs`

- [ ] **Step 1: Definir traits y helper de vectores (sin test propio: lo ejercitan los códecs)**

```rust
// crates/core/src/codecs/mod.rs
use crate::error::Error;
use crate::image::DecodedImage;
use crate::options::Options;

/// Decodifica bytes del formato a RGBA8 sRGB.
pub(crate) trait ImageDecoder {
    fn decode(&self, data: &[u8]) -> Result<DecodedImage, Error>;
}

/// Codifica RGBA8 sRGB a bytes del formato según `Options`.
pub(crate) trait ImageEncoder {
    fn encode(&self, img: &DecodedImage, opts: &Options) -> Result<Vec<u8>, Error>;
}
```

```rust
// crates/core/src/testutil.rs
//! Vectores de prueba deterministas generados en código (sin binarios en el repo).
#![cfg(test)]

use crate::image::DecodedImage;

/// Gradiente RGB con un círculo semi-transparente en el centro. Determinista.
pub fn vector_gradient_circle(w: u32, h: u32) -> DecodedImage {
    let mut px = Vec::with_capacity((w * h * 4) as usize);
    let (cx, cy) = (f64::from(w) / 2.0, f64::from(h) / 2.0);
    let r = f64::from(w.min(h)) / 3.0;
    for y in 0..h {
        for x in 0..w {
            let dist = ((f64::from(x) - cx).powi(2) + (f64::from(y) - cy).powi(2)).sqrt();
            let alpha = if dist < r { 128 } else { 255 };
            px.extend([
                (x * 255 / w.max(1)) as u8,
                (y * 255 / h.max(1)) as u8,
                ((x + y) * 127 / (w + h).max(1)) as u8,
                alpha,
            ]);
        }
    }
    DecodedImage::new(w, h, px).unwrap()
}

/// Imagen plana de pocos colores (caso ideal para PNG indexado).
pub fn vector_flat_colors(w: u32, h: u32) -> DecodedImage {
    let palette: [[u8; 4]; 4] =
        [[255, 0, 0, 255], [0, 255, 0, 255], [0, 0, 255, 255], [255, 255, 0, 255]];
    let mut px = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            px.extend(palette[((x / 8 + y / 8) % 4) as usize]);
        }
    }
    DecodedImage::new(w, h, px).unwrap()
}
```

```rust
// crates/core/src/lib.rs — agregar
pub(crate) mod codecs;
#[cfg(test)]
pub(crate) mod testutil;
```

- [ ] **Step 2: Compila y lint limpio** — `cargo build -p minipix-core; cargo clippy -p minipix-core --all-targets -- -D warnings` → OK.
- [ ] **Step 3: Commit** — `git commit -am "feat(core): traits ImageDecoder/ImageEncoder + vectores de test programaticos"`

## Fase B — Códecs (un task por dirección, TDD con roundtrips)

### Task 6: PNG decode

**Files:**

- Create: `crates/core/src/codecs/png.rs`
- Modify: `crates/core/src/codecs/mod.rs`, `crates/core/Cargo.toml`

- [ ] **Step 1: Deps**

```toml
# crates/core/Cargo.toml [dependencies] — agregar
png = "0.18"
```

- [ ] **Step 2: Test que falla (roundtrip contra el encoder de referencia del crate png)**

```rust
// crates/core/src/codecs/png.rs — tests al pie
#[cfg(test)]
mod tests {
    use super::*;
    use crate::codecs::ImageDecoder;
    use crate::testutil::vector_gradient_circle;

    /// Encodea con el crate png "a mano" y decodea con nuestro PngCodec.
    #[test]
    fn decodea_rgba8_roundtrip() {
        let img = vector_gradient_circle(32, 24);
        let mut bytes = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut bytes, img.width, img.height);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            let mut w = enc.write_header().unwrap();
            w.write_image_data(&img.pixels).unwrap();
        }
        let out = PngCodec.decode(&bytes).unwrap();
        assert_eq!((out.width, out.height), (32, 24));
        assert_eq!(out.pixels, img.pixels);
    }

    #[test]
    fn decodea_grayscale_a_rgba() {
        let mut bytes = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut bytes, 2, 1);
            enc.set_color(png::ColorType::Grayscale);
            enc.set_depth(png::BitDepth::Eight);
            let mut w = enc.write_header().unwrap();
            w.write_image_data(&[10, 200]).unwrap();
        }
        let out = PngCodec.decode(&bytes).unwrap();
        assert_eq!(out.pixels, vec![10, 10, 10, 255, 200, 200, 200, 255]);
    }

    #[test]
    fn error_tipado_con_basura() {
        let err = PngCodec.decode(b"\x89PNGgarbage").unwrap_err();
        assert!(matches!(err, crate::Error::Decode { .. }));
    }
}
```

- [ ] **Step 3: Verificar que falla** — `cargo test -p minipix-core png` → FAIL.

- [ ] **Step 4: Implementación**

```rust
// crates/core/src/codecs/png.rs
use crate::codecs::ImageDecoder;
use crate::error::Error;
use crate::format::Format;
use crate::image::DecodedImage;

pub(crate) struct PngCodec;

fn decode_err(e: impl std::fmt::Display) -> Error {
    Error::Decode { format: Format::Png, detail: e.to_string() }
}

impl ImageDecoder for PngCodec {
    fn decode(&self, data: &[u8]) -> Result<DecodedImage, Error> {
        let mut decoder = png::Decoder::new(std::io::Cursor::new(data));
        // Normaliza: expande paleta/grises/16-bit y agrega alpha.
        decoder.set_transformations(
            png::Transformations::normalize_to_color8() | png::Transformations::ALPHA,
        );
        let mut reader = decoder.read_info().map_err(decode_err)?;
        let mut buf = vec![0u8; reader.output_buffer_size().ok_or_else(|| decode_err("output too large"))?];
        let info = reader.next_frame(&mut buf).map_err(decode_err)?;
        buf.truncate(info.buffer_size());
        // Tras normalize_to_color8|ALPHA el output es RGBA8 o GrayA8.
        let rgba = match info.color_type {
            png::ColorType::Rgba => buf,
            png::ColorType::GrayscaleAlpha => buf
                .chunks_exact(2)
                .flat_map(|ga| [ga[0], ga[0], ga[0], ga[1]])
                .collect(),
            other => return Err(decode_err(format!("unexpected color type {other:?}"))),
        };
        DecodedImage::new(info.width, info.height, rgba)
    }
}
```

```rust
// crates/core/src/codecs/mod.rs — agregar
pub(crate) mod png;
```

- [ ] **Step 5: Verificar que pasa** — `cargo test -p minipix-core png` → PASS. Nota: si `normalize_to_color8`/`output_buffer_size` difieren en la versión instalada, ajustar contra `cargo doc -p png --open` — la intención (expandir a 8-bit + alpha) es lo contractual.
- [ ] **Step 6: Commit** — `git commit -am "feat(core): PNG decode normalizado a RGBA8"`

### Task 7: PNG encode (lossless oxipng + lossy quantette)

**Files:**

- Modify: `crates/core/src/codecs/png.rs`, `crates/core/Cargo.toml`

- [ ] **Step 1: Deps**

```toml
# crates/core/Cargo.toml [dependencies] — agregar
oxipng = { version = "10", default-features = false, features = ["parallel", "zopfli"] }
quantette = { version = "0.6", features = ["threads"] }
```

- [ ] **Step 2: Tests que fallan**

```rust
// crates/core/src/codecs/png.rs — agregar a tests
#[test]
fn encode_lossless_roundtrip_exacto() {
    let img = crate::testutil::vector_gradient_circle(48, 32);
    let opts = crate::Options::default().with_lossless(true);
    let bytes = PngCodec.encode(&img, &opts).unwrap();
    let back = PngCodec.decode(&bytes).unwrap();
    assert_eq!(back.pixels, img.pixels, "lossless debe ser bit-exacto");
}

#[test]
fn encode_lossy_reduce_y_decodea() {
    let img = crate::testutil::vector_flat_colors(64, 64);
    let lossless = PngCodec.encode(&img, &crate::Options::default().with_lossless(true)).unwrap();
    let lossy = PngCodec.encode(&img, &crate::Options::default().with_quality(60)).unwrap();
    assert!(PngCodec.decode(&lossy).is_ok());
    assert!(lossy.len() <= lossless.len(), "paleta no debe ser mayor que lossless en imagen plana");
}

#[test]
fn effort_alto_no_es_mayor() {
    let img = crate::testutil::vector_gradient_circle(48, 32);
    let e1 = PngCodec.encode(&img, &crate::Options::default().with_lossless(true).with_effort(1)).unwrap();
    let e9 = PngCodec.encode(&img, &crate::Options::default().with_lossless(true).with_effort(9)).unwrap();
    assert!(e9.len() <= e1.len());
}
```

- [ ] **Step 3: Verificar que falla** — `cargo test -p minipix-core png` → FAIL (no hay `encode`).

- [ ] **Step 4: Implementación**

```rust
// crates/core/src/codecs/png.rs — agregar
use crate::codecs::ImageEncoder;
use crate::options::Options;

fn encode_err(e: impl std::fmt::Display) -> Error {
    Error::Encode { format: Format::Png, detail: e.to_string() }
}

/// quality 1-100 → tamaño de paleta 8..=256 (mapa documentado en rustdoc).
fn palette_size(quality: u8) -> u16 {
    ((u16::from(quality) * 256) / 100).clamp(8, 256)
}

/// effort 0-9 → preset oxipng 0..=6 (7+ activa zopfli vía Deflaters).
fn oxipng_options(effort: u8) -> oxipng::Options {
    let mut o = oxipng::Options::from_preset(effort.min(6));
    if effort >= 7 {
        o.deflater = oxipng::Deflaters::Zopfli { iterations: std::num::NonZeroU8::new(15).unwrap_or(std::num::NonZeroU8::MIN) };
    }
    o.strip = oxipng::StripChunks::Safe;
    o
}

impl ImageEncoder for PngCodec {
    fn encode(&self, img: &DecodedImage, opts: &Options) -> Result<Vec<u8>, Error> {
        let raw = if opts.lossless || opts.quality == 100 {
            encode_rgba_png(img)?
        } else {
            encode_indexed_png(img, palette_size(opts.quality))?
        };
        oxipng::optimize_from_memory(&raw, &oxipng_options(opts.effort)).map_err(encode_err)
    }
}

fn encode_rgba_png(img: &DecodedImage) -> Result<Vec<u8>, Error> {
    let mut out = Vec::new();
    let mut enc = png::Encoder::new(&mut out, img.width, img.height);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.set_source_srgb(png::SrgbRenderingIntent::Perceptual);
    let mut w = enc.write_header().map_err(encode_err)?;
    w.write_image_data(&img.pixels).map_err(encode_err)?;
    w.finish().map_err(encode_err)?;
    Ok(out)
}

/// Cuantiza con quantette a paleta ≤256 + escribe PNG indexado con tRNS.
fn encode_indexed_png(img: &DecodedImage, max_colors: u16) -> Result<Vec<u8>, Error> {
    // quantette: pipeline sobre sRGB con dithering (ver docs del Pipeline 0.6).
    // Devuelve (paleta RGBA, índices por píxel).
    let (palette, indices) = quantize_rgba(img, max_colors)?;
    let mut out = Vec::new();
    let mut enc = png::Encoder::new(&mut out, img.width, img.height);
    enc.set_color(png::ColorType::Indexed);
    enc.set_depth(png::BitDepth::Eight);
    enc.set_source_srgb(png::SrgbRenderingIntent::Perceptual);
    let plte: Vec<u8> = palette.iter().flat_map(|c| [c[0], c[1], c[2]]).collect();
    let trns: Vec<u8> = palette.iter().map(|c| c[3]).collect();
    enc.set_palette(plte);
    enc.set_trns(trns);
    let mut w = enc.write_header().map_err(encode_err)?;
    w.write_image_data(&indices).map_err(encode_err)?;
    w.finish().map_err(encode_err)?;
    Ok(out)
}

/// Aísla la API de quantette (único punto a ajustar si la firma difiere).
fn quantize_rgba(img: &DecodedImage, max_colors: u16) -> Result<(Vec<[u8; 4]>, Vec<u8>), Error> {
    use quantette::{ImageRef, Pipeline, PaletteSize};
    let pixels: &[[u8; 4]] = bytemuck_cast_rgba(&img.pixels);
    let image = ImageRef::new(img.width, img.height, pixels).map_err(encode_err)?;
    let indexed = Pipeline::new(image)
        .palette_size(PaletteSize::try_from(max_colors).map_err(encode_err)?)
        .dither(true)
        .indexed_image();
    let palette = indexed.palette.iter().map(|c| [c.red, c.green, c.blue, c.alpha]).collect();
    let indices = indexed.indices;
    Ok((palette, indices))
}

fn bytemuck_cast_rgba(bytes: &[u8]) -> &[[u8; 4]] {
    // chunks_exact sin copia: layout [u8;4] garantizado por construcción de DecodedImage.
    unsafe { std::slice::from_raw_parts(bytes.as_ptr().cast(), bytes.len() / 4) }
}
```

**Nota para el implementador:** `quantize_rgba` es el único lugar acoplado a la firma exacta de quantette 0.6 (`Pipeline`/`ImageRef`/`IndexedImage` verificados en docs.rs, pero los nombres de método pueden variar levemente) — ajustar SOLO dentro de esa función hasta que compile; los tests de Step 2 son el contrato. Si quantette no soporta alpha en paleta, fallback documentado: cuantizar RGB compuesto sobre blanco cuando `!img.has_transparency()`, y usar RGBA lossless cuando sí hay transparencia. El `unsafe` de `bytemuck_cast_rgba` requiere `#![allow(unsafe_code)]` a nivel módulo con comentario, o reemplazarlo por el crate `bytemuck` (preferido: `bytemuck::cast_slice`, sin unsafe).

- [ ] **Step 5: Verificar que pasa** — `cargo test -p minipix-core png` → PASS.
- [ ] **Step 6: Commit** — `git commit -am "feat(core): PNG encode lossless(oxipng)+lossy(quantette indexado)"`

### Task 8: JPEG decode (zune-jpeg)

**Files:**

- Create: `crates/core/src/codecs/jpeg.rs`
- Modify: `crates/core/src/codecs/mod.rs`, `crates/core/Cargo.toml`

- [ ] **Step 1: Deps**

```toml
# crates/core/Cargo.toml [dependencies] — agregar
zune-jpeg = "0.5"
zune-core = "0.5"
```

- [ ] **Step 2: Test que falla** (usa mozjpeg del Task 9 aún no escrito → para no invertir el orden, el test encodea con `jpeg-encoder` de dev-deps)

```toml
# crates/core/Cargo.toml [dev-dependencies] — agregar
jpeg-encoder = "0.7"
```

```rust
// crates/core/src/codecs/jpeg.rs — tests al pie
#[cfg(test)]
mod tests {
    use super::*;
    use crate::codecs::ImageDecoder;
    use crate::testutil::vector_gradient_circle;

    #[test]
    fn decodea_jpeg_a_rgba() {
        let img = vector_gradient_circle(32, 24);
        let rgb = img.to_rgb_over_white();
        let mut bytes = Vec::new();
        jpeg_encoder::Encoder::new(&mut bytes, 90)
            .encode(&rgb, 32, 24, jpeg_encoder::ColorType::Rgb)
            .unwrap();
        let out = JpegCodec.decode(&bytes).unwrap();
        assert_eq!((out.width, out.height), (32, 24));
        assert_eq!(out.pixels.len(), 32 * 24 * 4);
        assert!(out.pixels.chunks_exact(4).all(|p| p[3] == 255), "JPEG no tiene alpha");
        // calidad 90: el primer píxel debe quedar cerca del original
        assert!(i16::from(out.pixels[0]).abs_diff(i16::from(rgb[0])) < 24);
    }

    #[test]
    fn error_tipado_con_basura() {
        let err = JpegCodec.decode(&[0xFF, 0xD8, 0xFF, 0x00, 0x00]).unwrap_err();
        assert!(matches!(err, crate::Error::Decode { .. }));
    }
}
```

- [ ] **Step 3: Verificar que falla** — `cargo test -p minipix-core jpeg` → FAIL.

- [ ] **Step 4: Implementación**

```rust
// crates/core/src/codecs/jpeg.rs
use crate::codecs::ImageDecoder;
use crate::error::Error;
use crate::format::Format;
use crate::image::DecodedImage;

pub(crate) struct JpegCodec;

fn decode_err(e: impl std::fmt::Display) -> Error {
    Error::Decode { format: Format::Jpeg, detail: e.to_string() }
}

impl ImageDecoder for JpegCodec {
    fn decode(&self, data: &[u8]) -> Result<DecodedImage, Error> {
        use zune_core::colorspace::ColorSpace;
        use zune_core::options::DecoderOptions;
        use zune_jpeg::JpegDecoder;
        let opts = DecoderOptions::default().jpeg_set_out_colorspace(ColorSpace::RGBA);
        let mut dec = JpegDecoder::new_with_options(zune_core::bytestream::ZCursor::new(data), opts);
        let pixels = dec.decode().map_err(decode_err)?;
        let info = dec.info().ok_or_else(|| decode_err("missing header info"))?;
        DecodedImage::new(u32::from(info.width), u32::from(info.height), pixels)
    }
}
```

```rust
// crates/core/src/codecs/mod.rs — agregar
pub(crate) mod jpeg;
```

- [ ] **Step 5: Verificar que pasa** — `cargo test -p minipix-core jpeg` → PASS (ajustar import exacto de `ZCursor` según docs.rs zune-core 0.5 si difiere).
- [ ] **Step 6: Commit** — `git commit -am "feat(core): JPEG decode (zune-jpeg) a RGBA8"`

### Task 9: JPEG encode (mozjpeg, con catch_unwind)

**Files:**

- Modify: `crates/core/src/codecs/jpeg.rs`, `crates/core/Cargo.toml`

- [ ] **Step 1: Deps** (requiere `nasm` instalado localmente y en CI — ya está en ci.yml Task 1)

```toml
# crates/core/Cargo.toml [dependencies] — agregar
mozjpeg = "0.10"
```

- [ ] **Step 2: Tests que fallan**

```rust
// crates/core/src/codecs/jpeg.rs — agregar a tests
#[test]
fn encode_decode_roundtrip_aproximado() {
    let img = crate::testutil::vector_gradient_circle(32, 24);
    let bytes = JpegCodec.encode(&img, &crate::Options::default().with_quality(90)).unwrap();
    let back = JpegCodec.decode(&bytes).unwrap();
    assert_eq!((back.width, back.height), (32, 24));
}

#[test]
fn quality_menor_da_menos_bytes() {
    let img = crate::testutil::vector_gradient_circle(64, 64);
    let q90 = JpegCodec.encode(&img, &crate::Options::default().with_quality(90)).unwrap();
    let q40 = JpegCodec.encode(&img, &crate::Options::default().with_quality(40)).unwrap();
    assert!(q40.len() < q90.len());
}

#[test]
fn lossless_es_rechazado() {
    let img = crate::testutil::vector_flat_colors(8, 8);
    let err = JpegCodec.encode(&img, &crate::Options::default().with_lossless(true)).unwrap_err();
    assert!(matches!(err, crate::Error::InvalidOptions(_)));
}
```

- [ ] **Step 3: Verificar que falla** — `cargo test -p minipix-core jpeg` → FAIL.

- [ ] **Step 4: Implementación** — mozjpeg reporta errores con unwind: TODO el flujo va dentro de `catch_unwind` (regla CLAUDE.md: el pánico no sale del core).

```rust
// crates/core/src/codecs/jpeg.rs — agregar
use crate::codecs::ImageEncoder;
use crate::options::Options;

fn encode_err(e: impl std::fmt::Display) -> Error {
    Error::Encode { format: Format::Jpeg, detail: e.to_string() }
}

impl ImageEncoder for JpegCodec {
    fn encode(&self, img: &DecodedImage, opts: &Options) -> Result<Vec<u8>, Error> {
        if opts.lossless {
            return Err(Error::InvalidOptions("JPEG does not support lossless".into()));
        }
        let rgb = img.to_rgb_over_white();
        let (w, h, q, progressive) =
            (img.width as usize, img.height as usize, f32::from(opts.quality), opts.jpeg_progressive);
        // mozjpeg señala errores haciendo unwind (resume_unwind): contener SIEMPRE.
        std::panic::catch_unwind(move || -> Result<Vec<u8>, String> {
            let mut comp = mozjpeg::Compress::new(mozjpeg::ColorSpace::JCS_RGB);
            comp.set_size(w, h);
            comp.set_quality(q);
            if progressive {
                comp.set_progressive_mode();
            }
            let mut started = comp.start_compress(Vec::new()).map_err(|e| e.to_string())?;
            started.write_scanlines(&rgb).map_err(|e| e.to_string())?;
            started.finish().map_err(|e| e.to_string())
        })
        .map_err(|_| encode_err("mozjpeg aborted (internal libjpeg error)"))?
        .map_err(encode_err)
    }
}
```

- [ ] **Step 5: Verificar que pasa** — `cargo test -p minipix-core jpeg` → PASS.
- [ ] **Step 6: Commit** — `git commit -am "feat(core): JPEG encode mozjpeg (progressive, catch_unwind)"`

### Task 10: WebP decode (image-webp)

**Files:**

- Create: `crates/core/src/codecs/webp.rs`
- Modify: `crates/core/src/codecs/mod.rs`, `crates/core/Cargo.toml`

- [ ] **Step 1: Deps**

```toml
# crates/core/Cargo.toml [dependencies] — agregar
image-webp = "0.2"
```

- [ ] **Step 2: Test que falla** (el vector WebP se genera con el encoder del Task 11; para mantener orden, este test usa un WebP lossless mínimo generado con `image-webp`'s propio `WebPEncoder` lossless)

```rust
// crates/core/src/codecs/webp.rs — tests al pie
#[cfg(test)]
mod tests {
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
        let out = WebpCodec.decode(&bytes).unwrap();
        assert_eq!((out.width, out.height), (32, 24));
        assert_eq!(out.pixels, img.pixels, "lossless roundtrip exacto");
    }

    #[test]
    fn error_tipado_con_basura() {
        let err = WebpCodec.decode(b"RIFF\x00\x00\x00\x00WEBPgarbage").unwrap_err();
        assert!(matches!(err, crate::Error::Decode { .. }));
    }
}
```

- [ ] **Step 3: Verificar que falla** — `cargo test -p minipix-core webp` → FAIL.

- [ ] **Step 4: Implementación** (API verificada: `WebPDecoder::new(BufRead+Seek)`, `dimensions()`, `has_alpha()`, `output_buffer_size() -> Option<usize>`, `read_image(&mut [u8])`; RGBA8 si hay alpha, RGB8 si no)

```rust
// crates/core/src/codecs/webp.rs
use crate::codecs::ImageDecoder;
use crate::error::Error;
use crate::format::Format;
use crate::image::DecodedImage;

pub(crate) struct WebpCodec;

fn decode_err(e: impl std::fmt::Display) -> Error {
    Error::Decode { format: Format::WebP, detail: e.to_string() }
}

impl ImageDecoder for WebpCodec {
    fn decode(&self, data: &[u8]) -> Result<DecodedImage, Error> {
        let mut dec = image_webp::WebPDecoder::new(std::io::Cursor::new(data)).map_err(decode_err)?;
        let (w, h) = dec.dimensions();
        let size = dec.output_buffer_size().ok_or_else(|| decode_err("image too large"))?;
        let mut buf = vec![0u8; size];
        dec.read_image(&mut buf).map_err(decode_err)?;
        let rgba = if dec.has_alpha() {
            buf
        } else {
            buf.chunks_exact(3).flat_map(|rgb| [rgb[0], rgb[1], rgb[2], 255]).collect()
        };
        DecodedImage::new(w, h, rgba)
    }
}
```

```rust
// crates/core/src/codecs/mod.rs — agregar
pub(crate) mod webp;
```

- [ ] **Step 5: Verificar que pasa** — `cargo test -p minipix-core webp` → PASS.
- [ ] **Step 6: Commit** — `git commit -am "feat(core): WebP decode (image-webp) a RGBA8"`

### Task 11: WebP encode (libwebp-sys2 — el único módulo `unsafe`)

**Files:**

- Modify: `crates/core/src/codecs/webp.rs`, `crates/core/Cargo.toml`

- [ ] **Step 1: Deps** (requiere `cmake` local y en CI; libwebp se compila estático)

```toml
# crates/core/Cargo.toml [dependencies] — agregar
libwebp-sys2 = { version = "0.2", features = ["1_2", "static"] }
```

- [ ] **Step 2: Tests que fallan**

```rust
// crates/core/src/codecs/webp.rs — agregar a tests
#[test]
fn encode_lossy_decodea_y_quality_ordena() {
    let img = crate::testutil::vector_gradient_circle(64, 64);
    let q90 = WebpCodec.encode(&img, &crate::Options::default().with_quality(90)).unwrap();
    let q40 = WebpCodec.encode(&img, &crate::Options::default().with_quality(40)).unwrap();
    assert!(WebpCodec.decode(&q90).is_ok());
    assert!(q40.len() < q90.len());
}

#[test]
fn encode_lossless_roundtrip_exacto() {
    let img = crate::testutil::vector_flat_colors(32, 32);
    let bytes = WebpCodec.encode(&img, &crate::Options::default().with_lossless(true)).unwrap();
    let back = WebpCodec.decode(&bytes).unwrap();
    assert_eq!(back.pixels, img.pixels);
}

#[test]
fn preserva_alpha() {
    let img = crate::testutil::vector_gradient_circle(32, 32); // tiene círculo alpha=128
    let bytes = WebpCodec.encode(&img, &crate::Options::default().with_quality(90)).unwrap();
    let back = WebpCodec.decode(&bytes).unwrap();
    assert!(back.has_transparency());
}
```

- [ ] **Step 3: Verificar que falla** — `cargo test -p minipix-core webp` → FAIL.

- [ ] **Step 4: Implementación** — flujo C estándar `WebPConfig` + `WebPPicture` + `WebPMemoryWriter`. Único `unsafe` del workspace: módulo con `#![allow(unsafe_code)]` y `// SAFETY:` por invariante.

```rust
// crates/core/src/codecs/webp.rs — agregar
use crate::codecs::ImageEncoder;
use crate::options::Options;

fn encode_err(e: impl std::fmt::Display) -> Error {
    Error::Encode { format: Format::WebP, detail: e.to_string() }
}

impl ImageEncoder for WebpCodec {
    fn encode(&self, img: &DecodedImage, opts: &Options) -> Result<Vec<u8>, Error> {
        ffi::encode_rgba(
            &img.pixels,
            img.width,
            img.height,
            ffi::EncodeParams {
                quality: f32::from(opts.quality),
                alpha_quality: i32::from(opts.alpha_quality),
                method: i32::from(opts.effort.min(6)), // effort 0-9 → method 0-6
                lossless: opts.lossless,
            },
        )
        .map_err(encode_err)
    }
}

#[allow(unsafe_code)] // FFI con libwebp: el resto del workspace deniega unsafe.
mod ffi {
    use libwebp_sys2 as sys;

    pub(super) struct EncodeParams {
        pub quality: f32,
        pub alpha_quality: i32,
        pub method: i32,
        pub lossless: bool,
    }

    pub(super) fn encode_rgba(
        rgba: &[u8],
        width: u32,
        height: u32,
        p: EncodeParams,
    ) -> Result<Vec<u8>, String> {
        // SAFETY: config/picture/writer son structs C inicializados por las
        // funciones Init de libwebp antes de cualquier lectura; rgba vive
        // durante todo el encode; los punteros no escapan de esta función.
        unsafe {
            let mut config: sys::WebPConfig = std::mem::zeroed();
            if sys::WebPConfigInit(&mut config) == 0 {
                return Err("WebPConfigInit failed (version mismatch)".into());
            }
            config.quality = p.quality;
            config.alpha_quality = p.alpha_quality;
            config.method = p.method;
            config.lossless = i32::from(p.lossless);
            if sys::WebPValidateConfig(&config) == 0 {
                return Err("invalid WebP config".into());
            }

            let mut pic: sys::WebPPicture = std::mem::zeroed();
            if sys::WebPPictureInit(&mut pic) == 0 {
                return Err("WebPPictureInit failed".into());
            }
            pic.use_argb = 1; // requerido para lossless y mejor calidad lossy
            pic.width = i32::try_from(width).map_err(|e| e.to_string())?;
            pic.height = i32::try_from(height).map_err(|e| e.to_string())?;

            let stride = i32::try_from(width * 4).map_err(|e| e.to_string())?;
            if sys::WebPPictureImportRGBA(&mut pic, rgba.as_ptr(), stride) == 0 {
                sys::WebPPictureFree(&mut pic);
                return Err("WebPPictureImportRGBA failed (out of memory?)".into());
            }

            let mut writer: sys::WebPMemoryWriter = std::mem::zeroed();
            sys::WebPMemoryWriterInit(&mut writer);
            pic.writer = Some(sys::WebPMemoryWrite);
            pic.custom_ptr = std::ptr::addr_of_mut!(writer).cast();

            let ok = sys::WebPEncode(&config, &mut pic);
            let error_code = pic.error_code;
            sys::WebPPictureFree(&mut pic);

            if ok == 0 {
                sys::WebPMemoryWriterClear(&mut writer);
                return Err(format!("WebPEncode failed with code {error_code:?}"));
            }
            let out = std::slice::from_raw_parts(writer.mem, writer.size).to_vec();
            sys::WebPMemoryWriterClear(&mut writer);
            Ok(out)
        }
    }
}
```

**Nota:** los nombres `WebPConfigInit`/`WebPPictureInit` pueden estar expuestos como wrappers seguros o macros en libwebp-sys2 0.2 — verificar con `cargo doc -p libwebp-sys2 --open` y ajustar SOLO dentro de `mod ffi`. Los tests de Step 2 son el contrato.

- [ ] **Step 5: Verificar que pasa** — `cargo test -p minipix-core webp` → PASS. Verificar también `cargo clippy -p minipix-core --all-targets -- -D warnings` (el allow de unsafe es por módulo, no global).
- [ ] **Step 6: Commit** — `git commit -am "feat(core): WebP encode libwebp (lossy+lossless+alpha), unico modulo unsafe"`

### Task 12: AVIF encode (ravif)

**Files:**

- Create: `crates/core/src/codecs/avif.rs`
- Modify: `crates/core/src/codecs/mod.rs`, `crates/core/Cargo.toml`

- [ ] **Step 1: Deps**

```toml
# crates/core/Cargo.toml [dependencies] — agregar
ravif = "0.13"
rgb = "0.8"
imgref = "1"
```

- [ ] **Step 2: Tests que fallan**

```rust
// crates/core/src/codecs/avif.rs — tests al pie
#[cfg(test)]
mod tests {
    use super::*;
    use crate::codecs::ImageEncoder;
    use crate::testutil::vector_gradient_circle;

    #[test]
    fn encode_produce_avif_valido() {
        let img = vector_gradient_circle(32, 24);
        let bytes = AvifCodec.encode(&img, &crate::Options::default().with_effort(9)).unwrap();
        assert_eq!(crate::sniff::sniff(&bytes), Some(crate::Format::Avif));
    }

    #[test]
    fn quality_ordena_tamanios() {
        let img = vector_gradient_circle(64, 64);
        let q90 = AvifCodec.encode(&img, &crate::Options::default().with_quality(90).with_effort(9)).unwrap();
        let q30 = AvifCodec.encode(&img, &crate::Options::default().with_quality(30).with_effort(9)).unwrap();
        assert!(q30.len() < q90.len());
    }
}
```

- [ ] **Step 3: Verificar que falla** — `cargo test -p minipix-core avif` → FAIL.

- [ ] **Step 4: Implementación** (API verificada ravif 0.13: `Encoder::new().with_quality(f32).with_alpha_quality(f32).with_speed(u8)`, `encode_rgba(Img<&[RGBA8]>) -> EncodedImage { avif_file, .. }`)

```rust
// crates/core/src/codecs/avif.rs
use crate::codecs::{ImageDecoder, ImageEncoder};
use crate::error::Error;
use crate::format::Format;
use crate::image::DecodedImage;
use crate::options::Options;

pub(crate) struct AvifCodec;

fn encode_err(e: impl std::fmt::Display) -> Error {
    Error::Encode { format: Format::Avif, detail: e.to_string() }
}

impl ImageEncoder for AvifCodec {
    fn encode(&self, img: &DecodedImage, opts: &Options) -> Result<Vec<u8>, Error> {
        // lossless=true: AVIF lossless existe pero ravif no lo expone → rechazo honesto v1.
        if opts.lossless {
            return Err(Error::InvalidOptions("AVIF lossless is not supported in v1".into()));
        }
        let pixels: &[rgb::RGBA8] = bytemuck::cast_slice(&img.pixels);
        let buf = imgref::Img::new(pixels, img.width as usize, img.height as usize);
        // effort 0-9 → speed 10-1 (ravif: 1 lento/denso, 10 rápido).
        let speed = 10u8.saturating_sub(opts.effort);
        let res = ravif::Encoder::new()
            .with_quality(f32::from(opts.quality))
            .with_alpha_quality(f32::from(opts.alpha_quality))
            .with_speed(speed.max(1))
            .encode_rgba(buf)
            .map_err(encode_err)?;
        Ok(res.avif_file)
    }
}
```

```toml
# crates/core/Cargo.toml [dependencies] — agregar (elimina el unsafe casero del Task 7)
bytemuck = "1"
```

```rust
// crates/core/src/codecs/mod.rs — agregar
pub(crate) mod avif;
```

(También reemplazar `bytemuck_cast_rgba` del Task 7 por `bytemuck::cast_slice` ahora que la dep existe.)

- [ ] **Step 5: Verificar que pasa** — `cargo test -p minipix-core avif` → PASS (estos tests son lentos en debug: si superan ~60 s, agregar `[profile.dev.package.rav1e] opt-level = 3` al Cargo.toml raíz).
- [ ] **Step 6: Commit** — `git commit -am "feat(core): AVIF encode (ravif/rav1e)"`

### Task 13: AVIF decode (avif-decode / libaom)

**Files:**

- Modify: `crates/core/src/codecs/avif.rs`, `crates/core/Cargo.toml`

- [ ] **Step 1: Deps** (avif-decode usa aom-decode/libaom: build con cmake+nasm, ya presentes por libwebp/mozjpeg; en Windows puede requerir `perl` para el build de aom — documentar en README si aparece)

```toml
# crates/core/Cargo.toml [dependencies] — agregar
avif-decode = "1"
```

- [ ] **Step 2: Test que falla (roundtrip contra nuestro encoder del Task 12)**

```rust
// crates/core/src/codecs/avif.rs — agregar a tests
#[test]
fn roundtrip_encode_decode() {
    let img = vector_gradient_circle(32, 24);
    let bytes = AvifCodec
        .encode(&img, &crate::Options::default().with_quality(90).with_effort(9))
        .unwrap();
    let back = AvifCodec.decode(&bytes).unwrap();
    assert_eq!((back.width, back.height), (32, 24));
    assert!(back.has_transparency(), "el circulo alpha=128 debe sobrevivir");
}

#[test]
fn error_tipado_con_basura() {
    let mut junk = vec![0x00, 0x00, 0x00, 0x1C];
    junk.extend(b"ftypavif");
    junk.extend([0u8; 64]);
    assert!(matches!(AvifCodec.decode(&junk).unwrap_err(), crate::Error::Decode { .. }));
}
```

- [ ] **Step 3: Verificar que falla** — `cargo test -p minipix-core avif` → FAIL.

- [ ] **Step 4: Implementación** (avif-decode 1.0: `Decoder::from_avif(&[u8])?.to_image()? -> Image` enum por profundidad/canales; variantes `Rgb8/Rgba8/Rgb16/Rgba16/Gray8/Gray16`)

```rust
// crates/core/src/codecs/avif.rs — agregar
fn decode_err(e: impl std::fmt::Display) -> Error {
    Error::Decode { format: Format::Avif, detail: e.to_string() }
}

impl ImageDecoder for AvifCodec {
    fn decode(&self, data: &[u8]) -> Result<DecodedImage, Error> {
        let decoder = avif_decode::Decoder::from_avif(data).map_err(decode_err)?;
        match decoder.to_image().map_err(decode_err)? {
            avif_decode::Image::Rgba8(img) => {
                let (buf, w, h) = img.into_contiguous_buf();
                DecodedImage::new(w as u32, h as u32, bytemuck::cast_slice(&buf).to_vec())
            }
            avif_decode::Image::Rgb8(img) => {
                let (buf, w, h) = img.into_contiguous_buf();
                let rgba = buf.iter().flat_map(|p| [p.r, p.g, p.b, 255]).collect();
                DecodedImage::new(w as u32, h as u32, rgba)
            }
            // 10/12-bit y gray: reducir a 8-bit (v1 emite 8-bit siempre).
            other => decode_16bit_or_gray(other),
        }
    }
}

/// Normaliza variantes de 16-bit y grayscale a RGBA8.
fn decode_16bit_or_gray(img: avif_decode::Image) -> Result<DecodedImage, Error> {
    fn squash(v: u16) -> u8 {
        #[allow(clippy::cast_possible_truncation)] // >>8 garantiza <=255
        { (v >> 8) as u8 }
    }
    match img {
        avif_decode::Image::Rgba16(i) => {
            let (buf, w, h) = i.into_contiguous_buf();
            let rgba = buf.iter().flat_map(|p| [squash(p.r), squash(p.g), squash(p.b), squash(p.a)]).collect();
            DecodedImage::new(w as u32, h as u32, rgba)
        }
        avif_decode::Image::Rgb16(i) => {
            let (buf, w, h) = i.into_contiguous_buf();
            let rgba = buf.iter().flat_map(|p| [squash(p.r), squash(p.g), squash(p.b), 255]).collect();
            DecodedImage::new(w as u32, h as u32, rgba)
        }
        avif_decode::Image::Gray8(i) => {
            let (buf, w, h) = i.into_contiguous_buf();
            let rgba = buf.iter().flat_map(|g| { let v = g.0; [v, v, v, 255] }).collect();
            DecodedImage::new(w as u32, h as u32, rgba)
        }
        avif_decode::Image::Gray16(i) => {
            let (buf, w, h) = i.into_contiguous_buf();
            let rgba = buf.iter().flat_map(|g| { let v = squash(g.0); [v, v, v, 255] }).collect();
            DecodedImage::new(w as u32, h as u32, rgba)
        }
        _ => Err(Error::Decode { format: Format::Avif, detail: "unsupported AVIF pixel layout".into() }),
    }
}
```

**Nota:** los nombres exactos de las variantes/`into_contiguous_buf` se ajustan contra `cargo doc -p avif-decode --open` (la doc inline del crate es escasa; el repo kornelski/avif-decode tiene ejemplos). Ajustar SOLO dentro de este módulo; los tests de Step 2 son el contrato.

- [ ] **Step 5: Verificar que pasa** — `cargo test -p minipix-core avif` → PASS.
- [ ] **Step 6: Commit** — `git commit -am "feat(core): AVIF decode (avif-decode/libaom) normalizado a RGBA8"`

## Fase C — Color, API pública y conformance

### Task 14: ICC → sRGB (moxcms) en los decoders

**Files:**

- Create: `crates/core/src/color.rs`
- Modify: `crates/core/src/codecs/{png.rs,jpeg.rs,webp.rs}`, `crates/core/Cargo.toml`, `crates/core/src/lib.rs`

- [ ] **Step 1: Deps**

```toml
# crates/core/Cargo.toml [dependencies] — agregar
moxcms = "0.7"
```

(Si moxcms no expone lo necesario al compilar, fallback aprobado: `lcms2 = "6"` — MIT, binding C de kornelski. Cambiar solo dentro de `color.rs`.)

- [ ] **Step 2: Test que falla** — sin fixtures binarios: moxcms genera el perfil Display P3 en el propio test.

```rust
// crates/core/src/color.rs — tests al pie
#[cfg(test)]
mod tests {
    use super::apply_icc_to_srgb;

    #[test]
    fn p3_rojo_saturado_se_desplaza_en_srgb() {
        // Rojo (255,0,0) etiquetado Display-P3 ≈ (250, 60..110, 50..80) en sRGB.
        let p3_profile = moxcms::ColorProfile::new_display_p3().to_bytes().unwrap();
        let mut pixels = vec![255u8, 0, 0, 255];
        apply_icc_to_srgb(&mut pixels, &p3_profile).unwrap();
        assert!(pixels[0] > 240, "R se mantiene alto: {}", pixels[0]);
        assert!(pixels[1] > 30, "G sube al mapear P3->sRGB: {}", pixels[1]);
        assert_eq!(pixels[3], 255, "alpha intacto");
    }

    #[test]
    fn icc_corrupto_no_rompe_devuelve_error() {
        let mut pixels = vec![1u8, 2, 3, 255];
        assert!(apply_icc_to_srgb(&mut pixels, b"not an icc profile").is_err());
        assert_eq!(pixels, vec![1, 2, 3, 255], "pixels intactos ante ICC invalido");
    }
}
```

- [ ] **Step 3: Verificar que falla** — `cargo test -p minipix-core color` → FAIL.

- [ ] **Step 4: Implementación**

```rust
// crates/core/src/color.rs
//! Aplica el perfil ICC embebido convirtiendo los píxeles a sRGB.
//! Política v1: el ICC se APLICA (nunca se re-embebe); todo output es sRGB.
use crate::error::Error;

pub(crate) fn apply_icc_to_srgb(rgba: &mut [u8], icc: &[u8]) -> Result<(), Error> {
    let src = moxcms::ColorProfile::new_from_slice(icc)
        .map_err(|e| Error::InvalidOptions(format!("invalid ICC profile: {e}")))?;
    let dst = moxcms::ColorProfile::new_srgb();
    let transform = src
        .create_transform_8bit(
            moxcms::Layout::Rgba,
            &dst,
            moxcms::Layout::Rgba,
            moxcms::TransformOptions::default(),
        )
        .map_err(|e| Error::InvalidOptions(format!("ICC transform: {e}")))?;
    let copy = rgba.to_vec();
    transform
        .transform(&copy, rgba)
        .map_err(|e| Error::InvalidOptions(format!("ICC transform: {e}")))?;
    Ok(())
}

/// Best-effort en decoders: ICC inválido se ignora (la imagen ya decodeó bien).
pub(crate) fn apply_icc_best_effort(rgba: &mut [u8], icc: Option<&[u8]>) {
    if let Some(icc) = icc {
        let _ = apply_icc_to_srgb(rgba, icc);
    }
}
```

Wiring (un cambio puntual por decoder, tras obtener los píxeles):

```rust
// png.rs (decode): el crate png expone el iCCP en reader.info()
let icc = reader.info().icc_profile.as_ref().map(|c| c.to_vec());
// ... tras armar `rgba`:
crate::color::apply_icc_best_effort(&mut rgba, icc.as_deref());

// jpeg.rs (decode): zune-jpeg expone el perfil tras decode
let icc = dec.icc_profile();
crate::color::apply_icc_best_effort(&mut pixels, icc.as_deref());

// webp.rs (decode): verificado icc_profile() -> Result<Option<Vec<u8>>, _>
let icc = dec.icc_profile().ok().flatten();
crate::color::apply_icc_best_effort(&mut rgba, icc.as_deref());
```

AVIF: el color viene en cajas `nclx`/`colr` que avif-decode ya interpreta a su modo; v1 no aplica ICC extra en AVIF (documentado en rustdoc del módulo).

**Nota:** los nombres exactos de moxcms (`new_from_slice`, `create_transform_8bit`, `Layout`) y `icc_profile()` de zune-jpeg se ajustan contra docs.rs al compilar — los tests de Step 2 son el contrato. Si `to_bytes()` no existe en moxcms para el test, alternativa: commitear `tests/vectors/DisplayP3.icc` generado una única vez con `lcms2`.

- [ ] **Step 5: Verificar que pasa** — `cargo test -p minipix-core` → PASS (todos los suites).
- [ ] **Step 6: Commit** — `git commit -am "feat(core): ICC->sRGB (moxcms) aplicado en decode PNG/JPEG/WebP"`

### Task 15: API pública `compress`/`convert` + límite anti-bomba

**Files:**

- Create: `crates/core/src/api.rs`, `crates/core/src/peek.rs`
- Modify: `crates/core/src/lib.rs`
- Test: `crates/core/tests/api.rs` (integración)

- [ ] **Step 1: Tests de integración que fallan**

```rust
// crates/core/tests/api.rs
use minipix_core::{compress, convert, Error, Format, Options};

/// PNG válido de 4x4 generado con el propio crate png (dev-dep ya presente).
fn tiny_png() -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut enc = png::Encoder::new(&mut bytes, 4, 4);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut w = enc.write_header().unwrap();
    w.write_image_data(&[128u8; 64]).unwrap();
    w.finish().unwrap();
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
fn convert_png_a_webp_y_avif() {
    for (fmt, magic_check) in [(Format::WebP, Format::WebP), (Format::Avif, Format::Avif)] {
        let out = convert(&tiny_png(), &Options::default().with_format(fmt).with_effort(9)).unwrap();
        assert_eq!(out.format, magic_check);
        assert_eq!(minipix_core::sniff::sniff(&out.data), Some(magic_check));
    }
}

#[test]
fn convert_sin_format_es_error() {
    assert!(matches!(convert(&tiny_png(), &Options::default()).unwrap_err(), Error::InvalidOptions(_)));
}

#[test]
fn basura_es_unsupported() {
    assert!(matches!(compress(b"garbage", &Options::default()).unwrap_err(), Error::UnsupportedFormat));
}

#[test]
fn limite_de_pixeles_corta_antes_de_decodear() {
    // PNG crafteado: solo firma + IHDR declarando 60000x60000 (sin data).
    let mut bytes = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    bytes.extend([0, 0, 0, 13]); // largo IHDR
    bytes.extend(b"IHDR");
    bytes.extend(60000u32.to_be_bytes());
    bytes.extend(60000u32.to_be_bytes());
    bytes.extend([8, 6, 0, 0, 0]); // depth 8, RGBA
    bytes.extend([0, 0, 0, 0]);    // CRC inválido: no importa, no llegamos a leerlo
    let err = compress(&bytes, &Options::default()).unwrap_err();
    assert!(matches!(err, Error::LimitExceeded(_)));
}
```

- [ ] **Step 2: Verificar que falla** — `cargo test -p minipix-core --test api` → FAIL.

- [ ] **Step 3: Implementación**

```rust
// crates/core/src/peek.rs
//! Dimensiones desde el header SIN decodificar (defensa anti-bomba).
use crate::format::Format;

/// `None` = este formato no permite peek barato (AVIF: se chequea post-decode).
pub(crate) fn peek_dimensions(format: Format, data: &[u8]) -> Option<(u32, u32)> {
    match format {
        Format::Png => {
            // IHDR es siempre el primer chunk: width/height en bytes 16..24.
            if data.len() < 24 || &data[12..16] != b"IHDR" { return None; }
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
    // Recorre marcadores hasta un SOFn (C0-CF salvo C4/C8/CC): height/width big-endian.
    let mut i = 2;
    while i + 9 < data.len() {
        if data[i] != 0xFF { return None; }
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
    if data.len() < 30 { return None; }
    match &data[12..16] {
        b"VP8X" => {
            // canvas: 24-bit little-endian menos uno, bytes 24..30.
            let w = 1 + u32::from(data[24]) + (u32::from(data[25]) << 8) + (u32::from(data[26]) << 16);
            let h = 1 + u32::from(data[27]) + (u32::from(data[28]) << 8) + (u32::from(data[29]) << 16);
            Some((w, h))
        }
        b"VP8 " => {
            // keyframe header: dims en bytes 26..30, 14 bits cada una.
            let w = u32::from(u16::from_le_bytes([data[26], data[27]]) & 0x3FFF);
            let h = u32::from(u16::from_le_bytes([data[28], data[29]]) & 0x3FFF);
            Some((w, h))
        }
        b"VP8L" => {
            // bytes 21..25: 14 bits width-1, 14 bits height-1.
            let b = [data[21], data[22], data[23], data[24]];
            let bits = u32::from_le_bytes(b);
            Some(((bits & 0x3FFF) + 1, ((bits >> 14) & 0x3FFF) + 1))
        }
        _ => None,
    }
}
```

```rust
// crates/core/src/api.rs
use crate::codecs::{avif::AvifCodec, jpeg::JpegCodec, png::PngCodec, webp::WebpCodec};
use crate::codecs::{ImageDecoder, ImageEncoder};
use crate::error::Error;
use crate::format::Format;
use crate::image::DecodedImage;
use crate::options::{Options, Output};
use crate::peek::peek_dimensions;
use crate::sniff::sniff;

/// Re-encodea optimizando en el MISMO formato de entrada.
pub fn compress(data: &[u8], opts: &Options) -> Result<Output, Error> {
    let format = sniff(data).ok_or(Error::UnsupportedFormat)?;
    run(data, format, opts)
}

/// Transcodea al formato de `opts.format` (requerido).
pub fn convert(data: &[u8], opts: &Options) -> Result<Output, Error> {
    let target = opts.format.ok_or_else(|| Error::InvalidOptions("convert requires options.format".into()))?;
    sniff(data).ok_or(Error::UnsupportedFormat)?;
    run(data, target, opts)
}

fn run(data: &[u8], target: Format, opts: &Options) -> Result<Output, Error> {
    opts.validate()?;
    let source = sniff(data).ok_or(Error::UnsupportedFormat)?;
    if let Some((w, h)) = peek_dimensions(source, data) {
        let px = u64::from(w) * u64::from(h);
        if px > opts.max_pixels {
            return Err(Error::LimitExceeded(px));
        }
    }
    let img = decode(source, data)?;
    if img.pixel_count() > opts.max_pixels {
        return Err(Error::LimitExceeded(img.pixel_count())); // AVIF y headers mentirosos
    }
    let encoded = encode(target, &img, opts)?;
    Ok(Output {
        format: target,
        width: img.width,
        height: img.height,
        bytes_in: data.len() as u64,
        bytes_out: encoded.len() as u64,
        data: encoded,
    })
}

fn decode(format: Format, data: &[u8]) -> Result<DecodedImage, Error> {
    match format {
        Format::Png => PngCodec.decode(data),
        Format::Jpeg => JpegCodec.decode(data),
        Format::WebP => WebpCodec.decode(data),
        Format::Avif => AvifCodec.decode(data),
    }
}

fn encode(format: Format, img: &DecodedImage, opts: &Options) -> Result<Vec<u8>, Error> {
    match format {
        Format::Png => PngCodec.encode(img, opts),
        Format::Jpeg => JpegCodec.encode(img, opts),
        Format::WebP => WebpCodec.encode(img, opts),
        Format::Avif => AvifCodec.encode(img, opts),
    }
}
```

```rust
// crates/core/src/lib.rs — agregar
pub mod api;
pub(crate) mod peek;
pub use api::{compress, convert};
```

(El test de integración usa `png` como dev-dependency: moverla a `[dev-dependencies]` si clippy se queja de uso solo-test; ya es dependencia normal por el códec.)

- [ ] **Step 4: Verificar que pasa** — `cargo test -p minipix-core` → PASS completo.
- [ ] **Step 5: Commit** — `git commit -am "feat(core): API publica compress/convert + peek anti-bomba"`

### Task 16: Vectores en disco + goldens de conformance

**Files:**

- Create: `crates/core/examples/gen_vectors.rs`
- Create: `crates/core/tests/conformance.rs`
- Create (generados y commiteados): `tests/vectors/gradient_circle.png`, `tests/vectors/flat_colors.png`, `tests/conformance/goldens.json`

- [ ] **Step 1: Generador determinista de vectores** (mismas funciones que `testutil`, duplicadas a propósito: `testutil` es `#[cfg(test)]` y el example no puede verla — 30 líneas, no amerita un crate aparte)

```rust
// crates/core/examples/gen_vectors.rs
//! Genera los vectores de prueba compartidos por Rust/Node/Python. Determinista.
//! Uso: cargo run -p minipix-core --example gen_vectors
use std::fs;

fn write_png(path: &str, w: u32, h: u32, rgba: &[u8]) {
    let file = fs::File::create(path).expect("create vector file");
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut wr = enc.write_header().expect("png header");
    wr.write_image_data(rgba).expect("png data");
    wr.finish().expect("png finish");
}

fn main() {
    fs::create_dir_all("tests/vectors").expect("mkdir");
    // gradient_circle 128x96 — misma fórmula que testutil::vector_gradient_circle
    let (w, h) = (128u32, 96u32);
    let (cx, cy, r) = (f64::from(w) / 2.0, f64::from(h) / 2.0, f64::from(h.min(w)) / 3.0);
    let mut px = Vec::new();
    for y in 0..h {
        for x in 0..w {
            let dist = ((f64::from(x) - cx).powi(2) + (f64::from(y) - cy).powi(2)).sqrt();
            px.extend([
                (x * 255 / w) as u8,
                (y * 255 / h) as u8,
                ((x + y) * 127 / (w + h)) as u8,
                if dist < r { 128 } else { 255 },
            ]);
        }
    }
    write_png("tests/vectors/gradient_circle.png", w, h, &px);
    // flat_colors 64x64 — misma fórmula que testutil::vector_flat_colors
    let palette: [[u8; 4]; 4] = [[255, 0, 0, 255], [0, 255, 0, 255], [0, 0, 255, 255], [255, 255, 0, 255]];
    let mut px = Vec::new();
    for y in 0..64u32 {
        for x in 0..64u32 {
            px.extend(palette[((x / 8 + y / 8) % 4) as usize]);
        }
    }
    write_png("tests/vectors/flat_colors.png", 64, 64, &px);
    println!("vectores escritos en tests/vectors/");
}
```

- [ ] **Step 2: Generar y commitear vectores** — Run: `cargo run -p minipix-core --example gen_vectors` → crea los 2 PNG. `git add tests/vectors`.

- [ ] **Step 3: Test de conformance con goldens (REGEN para crearlos la primera vez)**

```rust
// crates/core/tests/conformance.rs
//! Goldens compartidos: la matriz (vector × operación) produce hashes estables.
//! Los scripts de Node y Python (tasks 17/18) verifican los MISMOS hashes.
//! Regenerar: MINIPIX_REGEN_GOLDENS=1 cargo test -p minipix-core --test conformance
use minipix_core::{compress, convert, Format, Options};
use std::collections::BTreeMap;
use std::fs;

fn sha256_hex(data: &[u8]) -> String {
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

fn matrix() -> Vec<(String, Vec<u8>)> {
    let mut cases = Vec::new();
    for vector in ["gradient_circle", "flat_colors"] {
        let input = fs::read(format!("../../tests/vectors/{vector}.png")).expect("vector file");
        let compressed = compress(&input, &Options::default()).expect("compress png");
        cases.push((format!("{vector}.compress.png.q75e4"), compressed.data));
        for (fmt, tag) in [(Format::Jpeg, "jpeg"), (Format::WebP, "webp"), (Format::Avif, "avif")] {
            let out = convert(&input, &Options::default().with_format(fmt).with_effort(4))
                .expect("convert");
            cases.push((format!("{vector}.convert.{tag}.q75e4"), out.data));
        }
    }
    cases
}

#[test]
fn outputs_matchean_goldens() {
    let golden_path = "../../tests/conformance/goldens.json";
    let actual: BTreeMap<String, String> =
        matrix().into_iter().map(|(k, data)| (k, sha256_hex(&data))).collect();
    if std::env::var("MINIPIX_REGEN_GOLDENS").is_ok() {
        fs::create_dir_all("../../tests/conformance").expect("mkdir");
        fs::write(golden_path, serde_json::to_string_pretty(&actual).expect("json")).expect("write");
        return;
    }
    let golden: BTreeMap<String, String> =
        serde_json::from_str(&fs::read_to_string(golden_path).expect("goldens.json missing: regen first"))
            .expect("parse goldens");
    assert_eq!(actual, golden, "output cambió: si es intencional, regenerar goldens y justificar en el PR");
}
```

```toml
# crates/core/Cargo.toml [dev-dependencies] — agregar
sha2 = "0.10"
hex = "0.4"
serde_json = "1"
```

- [ ] **Step 4: Generar goldens y verificar** — Run: `$env:MINIPIX_REGEN_GOLDENS = "1"; cargo test -p minipix-core --test conformance; Remove-Item env:MINIPIX_REGEN_GOLDENS; cargo test -p minipix-core --test conformance` → segundo run PASS contra goldens.
- [ ] **Step 5: Commit** — `git add -A && git commit -m "test(core): vectores compartidos + goldens de conformance"`

## Fase D — Bindings

### Task 17: Binding Node (napi-rs v3)

**Files:**

- Create: `crates/node/Cargo.toml`, `crates/node/build.rs`, `crates/node/src/lib.rs`
- Create: `package.json` (raíz npm en `crates/node`), `crates/node/tests/smoke.mjs`
- Modify: `Cargo.toml` raíz (member), `.github/workflows/ci.yml`

- [ ] **Step 1: Crate + package**

```toml
# crates/node/Cargo.toml
[package]
name = "minipix-node"
version.workspace = true
edition.workspace = true
license.workspace = true
publish = false

[lib]
crate-type = ["cdylib"]

[dependencies]
minipix-core = { path = "../core" }
napi = { version = "3", default-features = false, features = ["napi8"] }
napi-derive = "3"

[build-dependencies]
napi-build = "2"

[lints]
workspace = true
```

```rust
// crates/node/build.rs
fn main() {
    napi_build::setup();
}
```

```json
// crates/node/package.json
{
  "name": "minipix",
  "version": "0.1.0",
  "description": "Image compression and conversion: PNG, JPEG, WebP, AVIF. One Rust core, same bytes everywhere.",
  "main": "index.js",
  "types": "index.d.ts",
  "license": "MIT OR Apache-2.0",
  "engines": { "node": ">= 18" },
  "napi": { "binaryName": "minipix" },
  "scripts": {
    "build": "napi build --platform --release",
    "test": "node tests/smoke.mjs"
  },
  "devDependencies": { "@napi-rs/cli": "^3.0.0" }
}
```

Y en `Cargo.toml` raíz: `members = ["crates/core", "crates/node"]`.

- [ ] **Step 2: Binding — AsyncTask para trabajo CPU-bound (no bloquea el event loop)**

```rust
// crates/node/src/lib.rs
//! Binding Node: SOLO conversión de tipos/errores. La lógica vive en minipix-core.
use napi::bindgen_prelude::*;
use napi_derive::napi;

#[napi(object)]
#[derive(Default)]
pub struct MinipixOptions {
    pub format: Option<String>,
    pub quality: Option<u32>,
    pub effort: Option<u32>,
    pub lossless: Option<bool>,
    pub alpha_quality: Option<u32>,
    pub jpeg_progressive: Option<bool>,
    pub max_pixels: Option<f64>,
}

#[napi(object)]
pub struct MinipixOutput {
    pub data: Buffer,
    pub format: String,
    pub width: u32,
    pub height: u32,
    pub bytes_in: f64,
    pub bytes_out: f64,
    pub ratio: f64,
}

fn parse_format(s: &str) -> Result<minipix_core::Format> {
    match s {
        "png" => Ok(minipix_core::Format::Png),
        "jpeg" | "jpg" => Ok(minipix_core::Format::Jpeg),
        "webp" => Ok(minipix_core::Format::WebP),
        "avif" => Ok(minipix_core::Format::Avif),
        other => Err(Error::new(Status::InvalidArg, format!("unknown format: {other}"))),
    }
}

fn to_core_options(o: &MinipixOptions) -> Result<minipix_core::Options> {
    let mut opts = minipix_core::Options::default();
    if let Some(f) = &o.format { opts.format = Some(parse_format(f)?); }
    if let Some(q) = o.quality { opts.quality = u8::try_from(q).map_err(invalid)?; }
    if let Some(e) = o.effort { opts.effort = u8::try_from(e).map_err(invalid)?; }
    if let Some(l) = o.lossless { opts.lossless = l; }
    if let Some(a) = o.alpha_quality { opts.alpha_quality = u8::try_from(a).map_err(invalid)?; }
    if let Some(p) = o.jpeg_progressive { opts.jpeg_progressive = p; }
    if let Some(m) = o.max_pixels { opts.max_pixels = m as u64; }
    Ok(opts)
}

fn invalid(e: impl std::fmt::Display) -> Error {
    Error::new(Status::InvalidArg, e.to_string())
}

fn map_err(e: minipix_core::Error) -> Error {
    let status = match &e {
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

pub struct Work {
    input: Vec<u8>,
    opts: minipix_core::Options,
    is_convert: bool,
}

impl Task for Work {
    type Output = minipix_core::Output;
    type JsValue = MinipixOutput;

    fn compute(&mut self) -> Result<Self::Output> {
        let run = if self.is_convert { minipix_core::convert } else { minipix_core::compress };
        run(&self.input, &self.opts).map_err(map_err)
    }

    fn resolve(&mut self, _env: Env, out: Self::Output) -> Result<Self::JsValue> {
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

#[napi(ts_return_type = "Promise<MinipixOutput>")]
pub fn compress(input: Buffer, options: Option<MinipixOptions>) -> Result<AsyncTask<Work>> {
    let opts = to_core_options(&options.unwrap_or_default())?;
    Ok(AsyncTask::new(Work { input: input.to_vec(), opts, is_convert: false }))
}

#[napi(ts_return_type = "Promise<MinipixOutput>")]
pub fn convert(input: Buffer, options: MinipixOptions) -> Result<AsyncTask<Work>> {
    let opts = to_core_options(&options)?;
    Ok(AsyncTask::new(Work { input: input.to_vec(), opts, is_convert: true }))
}
```

- [ ] **Step 3: Smoke test Node (corre la matriz de conformance contra los MISMOS goldens)**

```javascript
// crates/node/tests/smoke.mjs
import { compress, convert } from '../index.js';
import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import assert from 'node:assert/strict';

const goldens = JSON.parse(readFileSync('../../tests/conformance/goldens.json', 'utf8'));
const sha = (buf) => createHash('sha256').update(buf).digest('hex');

for (const vector of ['gradient_circle', 'flat_colors']) {
  const input = readFileSync(`../../tests/vectors/${vector}.png`);
  const c = await compress(input, {});
  assert.equal(sha(c.data), goldens[`${vector}.compress.png.q75e4`], `${vector} compress png`);
  for (const fmt of ['jpeg', 'webp', 'avif']) {
    const out = await convert(input, { format: fmt, effort: 4 });
    assert.equal(sha(out.data), goldens[`${vector}.convert.${fmt}.q75e4`], `${vector} -> ${fmt}`);
    assert.ok(out.bytesOut > 0 && out.width === c.width);
  }
}
console.log('node conformance: OK (bytes identicos al core Rust)');
```

- [ ] **Step 4: Build y test** — Run (en `crates/node/`): `npm install; npm run build; npm test` → "node conformance: OK". El `napi build` genera `index.js`/`index.d.ts` automáticamente (commitearlos NO: agregarlos a .gitignore junto a `*.node` — ya cubierto).
- [ ] **Step 5: CI** — agregar job a `ci.yml`:

```yaml
  node-binding:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: ilammy/setup-nasm@v1
      - uses: actions/setup-node@v4
        with: { node-version: 22 }
      - run: npm install && npm run build && npm test
        working-directory: crates/node
```

- [ ] **Step 6: Commit** — `git add -A && git commit -m "feat(node): binding napi-rs con AsyncTask + conformance vs goldens"`

### Task 18: Binding Python (PyO3 + maturin)

**Files:**

- Create: `crates/python/Cargo.toml`, `crates/python/pyproject.toml`, `crates/python/src/lib.rs`
- Create: `crates/python/minipix.pyi`, `crates/python/tests/test_smoke.py`
- Modify: `Cargo.toml` raíz (member), `.github/workflows/ci.yml`

- [ ] **Step 1: Crate + pyproject**

```toml
# crates/python/Cargo.toml
[package]
name = "minipix-python"
version.workspace = true
edition.workspace = true
license.workspace = true
publish = false

[lib]
name = "minipix"
crate-type = ["cdylib"]

[dependencies]
minipix-core = { path = "../core" }
pyo3 = { version = "0.25", features = ["extension-module", "abi3-py39"] }

[lints]
workspace = true
```

```toml
# crates/python/pyproject.toml
[build-system]
requires = ["maturin>=1.13,<2"]
build-backend = "maturin"

[project]
name = "minipix"
description = "Image compression and conversion: PNG, JPEG, WebP, AVIF. One Rust core, same bytes everywhere."
requires-python = ">=3.9"
license = "MIT OR Apache-2.0"
dynamic = ["version"]

[tool.maturin]
include = [{ path = "minipix.pyi", format = "sdist" }]
```

Y en `Cargo.toml` raíz: `members = ["crates/core", "crates/node", "crates/python"]`.

- [ ] **Step 2: Binding — `py.allow_threads` libera el GIL durante el trabajo CPU**

```rust
// crates/python/src/lib.rs
//! Binding Python: SOLO conversión de tipos/errores. La lógica vive en minipix-core.
use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use pyo3::types::PyBytes;

create_exception!(minipix, MinipixError, PyException, "Base error for minipix.");
create_exception!(minipix, UnsupportedFormatError, MinipixError, "Unrecognized input format.");
create_exception!(minipix, CodecError, MinipixError, "Decode or encode failure.");
create_exception!(minipix, LimitExceededError, MinipixError, "Pixel limit exceeded.");

fn map_err(e: minipix_core::Error) -> PyErr {
    use minipix_core::Error as E;
    match &e {
        E::UnsupportedFormat => UnsupportedFormatError::new_err(e.to_string()),
        E::Decode { .. } | E::Encode { .. } => CodecError::new_err(e.to_string()),
        E::InvalidOptions(_) => pyo3::exceptions::PyValueError::new_err(e.to_string()),
        E::LimitExceeded(_) => LimitExceededError::new_err(e.to_string()),
    }
}

fn parse_format(s: &str) -> PyResult<minipix_core::Format> {
    match s {
        "png" => Ok(minipix_core::Format::Png),
        "jpeg" | "jpg" => Ok(minipix_core::Format::Jpeg),
        "webp" => Ok(minipix_core::Format::WebP),
        "avif" => Ok(minipix_core::Format::Avif),
        other => Err(pyo3::exceptions::PyValueError::new_err(format!("unknown format: {other}"))),
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
    data: Py<PyBytes>,
    format: String,
    width: u32,
    height: u32,
    bytes_in: u64,
    bytes_out: u64,
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
    if let Some(f) = format { o.format = Some(parse_format(f)?); }
    if let Some(q) = quality { o.quality = q; }
    if let Some(e) = effort { o.effort = e; }
    if let Some(l) = lossless { o.lossless = l; }
    if let Some(a) = alpha_quality { o.alpha_quality = a; }
    if let Some(p) = jpeg_progressive { o.jpeg_progressive = p; }
    if let Some(m) = max_pixels { o.max_pixels = m; }
    Ok(o)
}

fn run(py: Python<'_>, data: &[u8], opts: &minipix_core::Options, is_convert: bool) -> PyResult<Output> {
    let f = if is_convert { minipix_core::convert } else { minipix_core::compress };
    let out = py.allow_threads(|| f(data, opts)).map_err(map_err)?;
    Ok(Output {
        ratio: out.ratio(),
        format: format_name(out.format).to_string(),
        width: out.width,
        height: out.height,
        bytes_in: out.bytes_in,
        bytes_out: out.bytes_out,
        data: PyBytes::new(py, &out.data).unbind(),
    })
}

#[pyfunction]
#[pyo3(signature = (data, *, quality=None, effort=None, lossless=None, alpha_quality=None, jpeg_progressive=None, max_pixels=None))]
#[allow(clippy::too_many_arguments)]
fn compress(
    py: Python<'_>, data: &[u8], quality: Option<u8>, effort: Option<u8>, lossless: Option<bool>,
    alpha_quality: Option<u8>, jpeg_progressive: Option<bool>, max_pixels: Option<u64>,
) -> PyResult<Output> {
    let opts = build_options(None, quality, effort, lossless, alpha_quality, jpeg_progressive, max_pixels)?;
    run(py, data, &opts, false)
}

#[pyfunction]
#[pyo3(signature = (data, *, format, quality=None, effort=None, lossless=None, alpha_quality=None, jpeg_progressive=None, max_pixels=None))]
#[allow(clippy::too_many_arguments)]
fn convert(
    py: Python<'_>, data: &[u8], format: &str, quality: Option<u8>, effort: Option<u8>, lossless: Option<bool>,
    alpha_quality: Option<u8>, jpeg_progressive: Option<bool>, max_pixels: Option<u64>,
) -> PyResult<Output> {
    let opts = build_options(Some(format), quality, effort, lossless, alpha_quality, jpeg_progressive, max_pixels)?;
    run(py, data, &opts, true)
}

#[pymodule]
fn minipix(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(compress, m)?)?;
    m.add_function(wrap_pyfunction!(convert, m)?)?;
    m.add_class::<Output>()?;
    m.add("MinipixError", py.get_type::<MinipixError>())?;
    m.add("UnsupportedFormatError", py.get_type::<UnsupportedFormatError>())?;
    m.add("CodecError", py.get_type::<CodecError>())?;
    m.add("LimitExceededError", py.get_type::<LimitExceededError>())?;
    Ok(())
}
```

```python
# crates/python/minipix.pyi
from typing import Optional

class Output:
    data: bytes
    format: str
    width: int
    height: int
    bytes_in: int
    bytes_out: int
    ratio: float

class MinipixError(Exception): ...
class UnsupportedFormatError(MinipixError): ...
class CodecError(MinipixError): ...
class LimitExceededError(MinipixError): ...

def compress(
    data: bytes, *, quality: Optional[int] = None, effort: Optional[int] = None,
    lossless: Optional[bool] = None, alpha_quality: Optional[int] = None,
    jpeg_progressive: Optional[bool] = None, max_pixels: Optional[int] = None,
) -> Output: ...

def convert(
    data: bytes, *, format: str, quality: Optional[int] = None, effort: Optional[int] = None,
    lossless: Optional[bool] = None, alpha_quality: Optional[int] = None,
    jpeg_progressive: Optional[bool] = None, max_pixels: Optional[int] = None,
) -> Output: ...
```

- [ ] **Step 3: Smoke test pytest (misma matriz, mismos goldens)**

```python
# crates/python/tests/test_smoke.py
import hashlib
import json
from pathlib import Path

import minipix
import pytest

ROOT = Path(__file__).resolve().parents[3]
GOLDENS = json.loads((ROOT / "tests/conformance/goldens.json").read_text())

def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()

@pytest.mark.parametrize("vector", ["gradient_circle", "flat_colors"])
def test_conformance(vector: str) -> None:
    data = (ROOT / f"tests/vectors/{vector}.png").read_bytes()
    out = minipix.compress(data)
    assert sha(out.data) == GOLDENS[f"{vector}.compress.png.q75e4"]
    for fmt in ["jpeg", "webp", "avif"]:
        res = minipix.convert(data, format=fmt, effort=4)
        assert sha(res.data) == GOLDENS[f"{vector}.convert.{fmt}.q75e4"], f"{vector} -> {fmt}"

def test_errores_tipados() -> None:
    with pytest.raises(minipix.UnsupportedFormatError):
        minipix.compress(b"garbage")
    with pytest.raises(ValueError):
        minipix.convert(b"garbage", format="bmp")
```

- [ ] **Step 4: Build y test** — Run (en `crates/python/`): `python -m venv .venv; .venv\Scripts\Activate.ps1; pip install maturin pytest; maturin develop --release; pytest tests/ -v` → PASS.
- [ ] **Step 5: CI** — job análogo al de Node con `PyO3/maturin-action` o venv+maturin develop:

```yaml
  python-binding:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: ilammy/setup-nasm@v1
      - uses: actions/setup-python@v5
        with: { python-version: "3.12" }
      - run: pip install maturin pytest && maturin develop --release && pytest tests/ -v
        working-directory: crates/python
```

- [ ] **Step 6: Commit** — `git add -A && git commit -m "feat(python): binding PyO3 (GIL liberado, excepciones tipadas) + conformance"`

## Fase E — Release y documentación

### Task 19: Pipeline de release (un tag → tres registros)

**Files:**

- Create: `.github/workflows/release.yml`

- [ ] **Step 1: Workflow de release** — prebuilds napi (matriz win/mac/linux ×64/arm64, glibc+musl), wheels maturin (manylinux/musllinux/mac/win, abi3) y `cargo publish`. Usar los templates oficiales como base: `napi-rs/napi-rs` docs "GitHub Actions" y `PyO3/maturin-action` README — el YAML completo se genera con `npx @napi-rs/cli new --dry-run` (sección workflows) y `maturin generate-ci github`, y se adapta:

```yaml
# .github/workflows/release.yml — esqueleto a completar con los templates generados
name: release
on:
  push:
    tags: ["v*"]
jobs:
  npm-prebuilds:
    # matriz generada por: npx napi create-npm-dirs && napi build --platform
    # targets v1: x86_64/aarch64 × (windows-msvc, apple-darwin, linux-gnu, linux-musl)
    # cada job: setup nasm+cmake, napi build --platform --release --target <t>, upload artifact
    strategy: { matrix: { settings: [] } } # ← completar desde template napi-rs
    runs-on: ${{ matrix.settings.host }}
    steps: [] # ← template
  npm-publish:
    needs: npm-prebuilds
    runs-on: ubuntu-latest
    steps: [] # napi prepublish + npm publish con NPM_TOKEN
  wheels:
    # maturin generate-ci github -m crates/python/Cargo.toml
    strategy: { matrix: { platform: [] } } # ← completar desde maturin generate-ci
    runs-on: ${{ matrix.platform.runner }}
    steps: [] # PyO3/maturin-action con manylinux + before-script-linux: instalar nasm/cmake
  pypi-publish:
    needs: wheels
    runs-on: ubuntu-latest
    steps: [] # maturin-action upload o pypa/gh-action-pypi-publish con trusted publishing
  crates-io:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo publish -p minipix-core --token ${{ secrets.CRATES_TOKEN }}
```

**Este task es el único donde "completar desde template" es aceptable**: los templates oficiales de napi-rs y maturin son generados por sus CLIs y cambian con cada versión — copiarlos a mano en este plan los desactualizaría. El criterio de done es el Step 2.

- [ ] **Step 2: Verificación en seco** — push de un tag `v0.1.0-rc.1` en un fork/branch: los 3 jobs de build deben terminar en verde y publicar a npm con `--dry-run`, TestPyPI y `cargo publish --dry-run`.
- [ ] **Step 3: Commit** — `git add -A && git commit -m "ci: release pipeline npm+PyPI+crates.io desde un tag"`

### Task 20: README + rustdoc + benchmark

**Files:**

- Create: `README.md`, `crates/core/benches/codecs.rs`
- Modify: `crates/core/Cargo.toml`

- [ ] **Step 1: README** con: pitch (1 párrafo: un core, tres lenguajes, bytes idénticos, licencias permisivas), quickstart en los 3 lenguajes (los snippets de la API del spec §6), tabla de opciones, tabla de códecs/licencias, requisitos de build para crates.io (cmake+nasm), link al spec y al playground (M2, "coming soon").
- [ ] **Step 2: rustdoc** — `cargo doc -p minipix-core --no-deps` sin warnings; `lib.rs` con ejemplo ejecutable (`compress` de un PNG mínimo embebido con `include_bytes!` del vector).
- [ ] **Step 3: Benchmark criterion (no bloquea CI)**

```toml
# crates/core/Cargo.toml — agregar
[dev-dependencies]
criterion = "0.5"

[[bench]]
name = "codecs"
harness = false
```

```rust
// crates/core/benches/codecs.rs
use criterion::{criterion_group, criterion_main, Criterion};
use minipix_core::{convert, Format, Options};

fn bench_convert(c: &mut Criterion) {
    let input = std::fs::read("../../tests/vectors/gradient_circle.png").expect("vector");
    for (fmt, name) in [(Format::Jpeg, "jpeg"), (Format::WebP, "webp"), (Format::Avif, "avif")] {
        c.bench_function(&format!("convert_png_to_{name}_q75"), |b| {
            b.iter(|| convert(&input, &Options::default().with_format(fmt)).expect("convert"));
        });
    }
}

criterion_group!(benches, bench_convert);
criterion_main!(benches);
```

Run: `cargo bench -p minipix-core` → reporta tiempos (informativo).

- [ ] **Step 4: Commit final M1** — `git add -A && git commit -m "docs: README multi-lenguaje + rustdoc + benchmarks criterion"`

---

## Verificación final de M1 (checklist de cierre)

- [ ] `cargo fmt --check; cargo clippy --workspace --all-targets -- -D warnings; cargo deny check; cargo test --workspace` → todo verde local y en CI.
- [ ] Conformance: los goldens pasan en Rust, Node y Python — **los mismos hashes** (la promesa del spec, probada).
- [ ] Release dry-run del Task 19 verde.
- [ ] `unsafe` solo en `crates/core/src/codecs/webp.rs::ffi` con `// SAFETY:` (verificar con `rg "unsafe" crates/`).
- [ ] Spec actualizado si hubo desviaciones adicionales durante la implementación.
- [ ] Pánicos en bindings: napi-rs y PyO3 ya convierten panics de Rust en Error/PanicException en el borde — verificado por los tests de errores tipados; el único productor interno de unwinds (mozjpeg) se contiene en el core (Task 9).

**Backlog inmediato post-M1** (spec §9.4, no bloquea el release): fuzzing con cargo-fuzz sobre `sniff`, `peek_dimensions` y los wrappers de decode (requiere nightly; job de CI separado).

**Backlog del review de Task 2**: ampliar detección AVIF a `compatible_brands` del ftyp (hoy solo major brand: AVIFs con major `mif1`/`msf1` se rechazan) — hacerlo antes de v1.0 final.
**Backlog del review de Task 13**: (a) fixture AVIF 10-bit/grayscale para cubrir las variantes 16-bit del normalizador; (b) [RESUELTO — Path B1] avif-decode 1.0.2 `unprem` tiene una fórmula incorrecta: `(val*256)/(alpha*256)/256` simplifica a `val/alpha/256 ≈ 0` en vez del correcto `val*255/alpha`. avif-parse 2.1.0 expone `AvifData::premultiplied_alpha: bool` (flag HEIF `prem`) y ya lo parseamos en el guard de dimensiones. Se añadió un guard explícito que rechaza AVIFs premultiplicados con `Error::Decode { format: Avif, detail: "premultiplied-alpha … not supported" }` en lugar de dejar que avif-decode corrompa colores silenciosamente. Cuando avif-decode corrija el upstream (o se reemplace), quitar el guard y agregar fixture binaria premultiplicada.
**Backlog del review de Task 14**: (a) moxcms despacha SIMD en runtime (AVX512/AVX2/SSE4.1) — si se agregan vectores golden CON ICC, validar estabilidad cross-CPU o fijar camino escalar; (b) AVIF con ICC embebido (colr prof) no se normaliza a sRGB en v1; (c) cache de transform para batch (no relevante con un decode por imagen); (d) short-circuit sRGB pendiente — `ColorProfile` no implementa `PartialEq` en moxcms 0.8 y comparar primaries/white-point dentro de epsilon serían >20 líneas de float frágil; diferir hasta que moxcms exponga `PartialEq` o un helper `is_srgb()`.
