# minipix M2 (Playground WASM) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Playground web 100% client-side — el core de minipix compilado a WASM corriendo en Web Workers dentro de una SPA Vite + Svelte, desplegada en Cloudflare Pages.

**Architecture:** Perfil de features puro-Rust de `minipix-core` → `crates/wasm` (wasm-bindgen 0.2.123, target `wasm32-unknown-unknown`, Rust ESTABLE) → pool de Web Workers con protocolo postMessage tipado (buffers transferidos, sin COOP/COEP) → SPA Svelte 5 (runes). AVIF se decodifica con el navegador (createImageBitmap) y se alimenta como RGBA al core vía una nueva API `encode_rgba`.

**Tech Stack:** wasm-bindgen 0.2.123 + wasm-pack 0.15 + wasm-opt (binaryen 130) · Vite 8 + Svelte 5.56 (runes) + TypeScript strict · img-comparison-slider 8 · fflate 0.8 · wrangler-action@v4 → Cloudflare Pages.

**Decisiones verificadas que DESVÍAN del spec §7 (amendments en Task 11):**

1. **Códecs C fuera del build wasm en M2.** El soporte emscripten de wasm-bindgen existe (0.2.115+, vanilla-emsdk desde 2026-05-21) pero tiene ~3 semanas y docs stale → es un spike de paridad para M3, no la base de M2. Se activan los fallbacks que el spec ya sancionaba: JPEG encode vía `jpeg-encoder` (puro Rust, menor densidad — documentado), WebP encode solo lossless (`image-webp`), y AVIF decode vía navegador (93.4% soporte global; no existe decoder AV1 puro-Rust con licencia MIT/Apache en 2026 — rav1d sigue sin API Rust, rav1d-safe es AGPL).
2. **Sin threads WASM ni COOP/COEP en M2** (wasm-bindgen-rayon sigue siendo nightly-only): el paralelismo es a nivel de pool de workers (una imagen por worker, ArrayBuffers transferidos — cero headers especiales). Si M3 agrega threads, son 2 líneas de `_headers` en Cloudflare Pages.
3. **Carve-out wasm de la regla 4 de CLAUDE.md**: en stable, panic=abort → un pánico atrapa (trap) y la instancia queda muerta. Patrón documentado: `console_error_panic_hook` (debug), todo RuntimeError del worker = instancia muerta → terminate + respawn del worker, error tipado a la UI.

**Hechos medidos (workflow de research, 2026-06-12):** bundle puro-Rust completo con rav1e: 1.65 MB raw / 0.61 MB gzip (probe local compilado). rav1e wasm single-thread: ~11 s/MP a speed 10 (benchmark 2021, rav1e 0.4 — re-medir). oxipng necesita `clang` para su feature `freestanding` (libdeflate C → wasm); en Windows local instalar LLVM (`winget install LLVM.LLVM`), en CI ubuntu ya está.

---

## Fase A — Core wasm-ready

### Task 1: Matriz de features en minipix-core (native vs wasm)

**Files:**

- Modify: `crates/core/Cargo.toml`, `crates/core/src/codecs/{jpeg.rs,webp.rs,avif.rs,png.rs}`, `crates/core/src/api.rs`
- Modify: `.github/workflows/ci.yml` (gate `cargo check --target wasm32-unknown-unknown`)

- [ ] **Step 1: Reestructurar Cargo.toml con features**

```toml
# crates/core/Cargo.toml — [features] nuevo + deps condicionales
[features]
default = ["native"]
# Códecs C de máxima calidad + paralelismo (build nativo)
native = [
  "dep:mozjpeg",
  "dep:libwebp-sys2",
  "dep:avif-decode",
  "oxipng/parallel",
  "quantette/threads",
  "ravif/asm",
  "ravif/threading",
]
# Perfil puro-Rust para wasm32 (fallbacks documentados; ver plan M2)
wasm = ["dep:jpeg-encoder"]

[dependencies]
# ... deps existentes, con estos cambios:
mozjpeg = { version = "0.10", optional = true }
libwebp-sys2 = { version = "0.2", features = ["1_2", "static"], optional = true }
avif-decode = { version = "1", optional = true }
oxipng = { version = "10", default-features = false, features = ["zopfli", "freestanding"] }
quantette = { version = "0.6", default-features = false, features = ["kmeans", "std"] }
ravif = { version = "0.13", default-features = false }
jpeg-encoder = { version = "0.7", features = ["std"], optional = true }
# jpeg-encoder se QUITA de dev-dependencies (pasa a optional normal; los tests
# del decoder lo usan vía la feature wasm o se mantiene también en dev-deps — verificar
# que `cargo test` default siga compilando; si hace falta, dejarlo en ambos lados).
```

VERIFICAR: `oxipng/parallel` y `quantette/threads` hoy están hardcodeadas — al moverlas a la feature `native`, el build default (native) debe producir EXACTAMENTE los mismos bytes que antes (los goldens lo prueban). `ravif/asm`+`threading`: ídem — rav1e CI asegura asm==rust bit-exact, y threads ya está fijado en 1 por código; los goldens son el árbitro final: `cargo test -p minipix-core --test conformance` DEBE pasar sin regenerar.

`freestanding` en oxipng: agregar SIEMPRE (native incluido) solo si los goldens no cambian (usa rust-alloc en vez de malloc de libc — verificar; si cambia bytes en native, hacerla condicional del perfil wasm vía `[target.'cfg(target_family = "wasm")'.dependencies]` en su lugar — reportar cuál fue el caso).

- [ ] **Step 2: cfg-gates en los códecs**

```rust
// jpeg.rs — el encoder se divide:
#[cfg(feature = "native")]
impl ImageEncoder for JpegCodec {
    // ... impl mozjpeg existente sin cambios ...
}

#[cfg(all(feature = "wasm", not(feature = "native")))]
impl ImageEncoder for JpegCodec {
    fn encode(&self, img: &DecodedImage, opts: &Options) -> Result<Vec<u8>, Error> {
        if opts.lossless {
            return Err(Error::InvalidOptions("JPEG does not support lossless".into()));
        }
        // Fallback wasm documentado: jpeg-encoder (puro Rust, sin trellis →
        // menor densidad que mozjpeg; ver plan M2 / README).
        let rgb = img.to_rgb_over_white();
        let mut out = Vec::new();
        let mut enc = jpeg_encoder::Encoder::new(&mut out, opts.quality);
        if opts.jpeg_progressive {
            enc.set_progressive(true); // verificar API exacta de jpeg-encoder 0.7
        }
        enc.encode(
            &rgb,
            u16::try_from(img.width).map_err(|e| encode_err(e))?,
            u16::try_from(img.height).map_err(|e| encode_err(e))?,
            jpeg_encoder::ColorType::Rgb,
        )
        .map_err(encode_err)?;
        Ok(out)
    }
}
```

```rust
// webp.rs — encode con gate:
#[cfg(feature = "native")]  // impl libwebp existente + mod ffi entero bajo este cfg
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
            .encode(&img.pixels, img.width, img.height, image_webp::ColorType::Rgba8)
            .map_err(encode_err)?;
        Ok(out)
    }
}
```

```rust
// avif.rs — decode con gate (encode ravif queda SIN gate: es puro Rust, sirve en ambos):
#[cfg(feature = "native")]  // impl avif-decode existente bajo este cfg
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
```

png.rs: sin gate (puro Rust en ambos perfiles). OJO oxipng: NUNCA setear `Options.timeout` (usa `Instant::now()` → panic en wasm); ya no lo seteamos — agregar comentario `// NUNCA setear timeout: Instant::now() trappea en wasm32 (plan M2)` junto a `oxipng_options()`.

- [ ] **Step 3: CI gate** — agregar step al job lint-test de ci.yml:

```yaml
      - run: rustup target add wasm32-unknown-unknown
      - run: cargo check -p minipix-core --target wasm32-unknown-unknown --no-default-features --features wasm
```

- [ ] **Step 4: Verificar** — `cargo test --workspace` (63 tests, goldens SIN regenerar = la matriz no cambió bytes nativos), `cargo check -p minipix-core --target wasm32-unknown-unknown --no-default-features --features wasm` (requiere `rustup target add wasm32-unknown-unknown` y clang para oxipng/freestanding — si clang falta localmente: `winget install LLVM.LLVM` y refrescar PATH), clippy, fmt.
- [ ] **Step 5: Commit** — `git add -A; if ($?) { git commit -m "feat(core): matriz de features native/wasm con fallbacks puro-Rust" }`

### Task 2: API `encode_rgba` en el core

**Files:** Modify `crates/core/src/api.rs`, `crates/core/src/lib.rs`; Test `crates/core/tests/api.rs`.

- [ ] **Step 1: Tests que fallan** (en tests/api.rs):

```rust
#[test]
fn encode_rgba_directo_a_webp() {
    let rgba = vec![128u8; 4 * 4 * 4];
    let out = minipix_core::encode_rgba(&rgba, 4, 4, Format::WebP, &Options::default().with_lossless(true)).unwrap();
    assert_eq!(minipix_core::sniff::sniff(&out.data), Some(Format::WebP));
    assert_eq!((out.width, out.height), (4, 4));
    assert_eq!(out.bytes_in, (4 * 4 * 4) as u64);
}

#[test]
fn encode_rgba_valida_buffer_y_limite() {
    let rgba = vec![0u8; 10]; // largo inválido
    assert!(minipix_core::encode_rgba(&rgba, 4, 4, Format::Png, &Options::default()).is_err());
    let rgba = vec![0u8; 4 * 4 * 4];
    let mut opts = Options::default();
    opts.max_pixels = 8;
    assert!(matches!(
        minipix_core::encode_rgba(&rgba, 4, 4, Format::Png, &opts).unwrap_err(),
        Error::LimitExceeded { pixels: 16, limit: 8 }
    ));
}
```

- [ ] **Step 2: Implementación** (api.rs):

```rust
/// Codifica píxeles RGBA8 crudos (sRGB) al formato pedido, sin pasar por un decoder.
/// Caso de uso principal: el playground wasm alimenta imágenes decodificadas por el
/// navegador (p.ej. AVIF). `bytes_in` reporta el tamaño del buffer RGBA de entrada.
///
/// # Errors
/// `InvalidOptions` (buffer/opciones inválidas), `LimitExceeded`, `Encode`.
pub fn encode_rgba(
    rgba: &[u8],
    width: u32,
    height: u32,
    format: Format,
    opts: &Options,
) -> Result<Output, Error> {
    opts.validate()?;
    let pixels = u64::from(width) * u64::from(height);
    if pixels > opts.max_pixels {
        return Err(Error::LimitExceeded { pixels, limit: opts.max_pixels });
    }
    let img = DecodedImage::new(width, height, rgba.to_vec())?;
    let encoded = encode(format, &img, opts)?;
    Ok(Output {
        format,
        width,
        height,
        bytes_in: rgba.len() as u64,
        bytes_out: encoded.len() as u64,
        data: encoded,
    })
}
```

lib.rs: `pub use api::encode_rgba;`

- [ ] **Step 3: Verificar** (65 tests) + clippy + fmt. **Step 4: Commit** — `git commit -am "feat(core): encode_rgba para inputs decodificados por el navegador"`

### Task 3: Crate `crates/wasm` (binding wasm-bindgen)

**Files:**

- Create: `crates/wasm/Cargo.toml`, `crates/wasm/src/lib.rs`, `crates/wasm/build.ps1` (script local) — Modify: root `Cargo.toml` (member), root `[profile.wasm-release]`.

- [ ] **Step 1: Crate**

```toml
# crates/wasm/Cargo.toml
[package]
name = "minipix-wasm"
version.workspace = true
edition.workspace = true
license.workspace = true
publish = false

[lib]
crate-type = ["cdylib"]

[dependencies]
minipix-core = { path = "../core", version = "0.1.0", default-features = false, features = ["wasm"] }
wasm-bindgen = "0.2.123"
serde = { version = "1", features = ["derive"] }
serde-wasm-bindgen = "0.6"
console_error_panic_hook = { version = "0.1.7", optional = true }

[features]
default = []
debug-panics = ["dep:console_error_panic_hook"]

[lints]
workspace = true
```

Root Cargo.toml: members += "crates/wasm"; y perfil separado (NO tocar release nativo — los goldens M1 dependen de él):

```toml
[profile.wasm-release]
inherits = "release"
opt-level = "z"
lto = "fat"
codegen-units = 1
panic = "abort"
strip = "symbols"
```

- [ ] **Step 2: Binding** — solo conversión de tipos (CLAUDE.md regla 1). Opciones como objeto JS plano vía serde-wasm-bindgen; bytes como `&[u8]`/`Vec<u8>` (wasm-bindgen copia entre JS y wasm — sin vistas colgantes):

```rust
// crates/wasm/src/lib.rs
//! Binding WASM: SOLO conversión de tipos/errores. La lógica vive en minipix-core.
//!
//! PÁNICOS (carve-out de CLAUDE.md regla 4): en stable wasm32 panic=abort, un
//! pánico es un trap que mata la instancia — no hay catch_unwind posible. El
//! contrato con el worker JS: cualquier RuntimeError ⇒ instancia muerta ⇒
//! terminate + respawn del worker (ver playground/src/lib/worker-pool.ts).
use serde::Deserialize;
use wasm_bindgen::prelude::*;

/// Espejo de las opciones del SDK (camelCase como en el binding Node).
#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct WasmOptions {
    pub format: Option<String>,
    pub quality: Option<u8>,
    pub effort: Option<u8>,
    pub lossless: Option<bool>,
    pub alpha_quality: Option<u8>,
    pub jpeg_progressive: Option<bool>,
    pub max_pixels: Option<u64>,
}

fn parse_format(s: &str) -> Result<minipix_core::Format, JsError> {
    match s {
        "png" => Ok(minipix_core::Format::Png),
        "jpeg" | "jpg" => Ok(minipix_core::Format::Jpeg),
        "webp" => Ok(minipix_core::Format::WebP),
        "avif" => Ok(minipix_core::Format::Avif),
        other => Err(JsError::new(&format!("[InvalidOptions] unknown format: {other}"))),
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

/// Resultado serializable a un objeto JS plano.
#[wasm_bindgen(getter_with_clone)]
pub struct WasmOutput {
    pub data: Vec<u8>,
    pub format: String,
    pub width: u32,
    pub height: u32,
    pub bytes_in: f64,
    pub bytes_out: f64,
    pub ratio: f64,
}

fn to_output(out: minipix_core::Output) -> WasmOutput {
    let name = match out.format {
        minipix_core::Format::Png => "png",
        minipix_core::Format::Jpeg => "jpeg",
        minipix_core::Format::WebP => "webp",
        minipix_core::Format::Avif => "avif",
    };
    #[allow(clippy::cast_precision_loss)] // tamaños << 2^52
    WasmOutput {
        ratio: out.ratio(),
        format: name.to_string(),
        width: out.width,
        height: out.height,
        bytes_in: out.bytes_in as f64,
        bytes_out: out.bytes_out as f64,
        data: out.data,
    }
}

/// Inicialización opcional de diagnóstico (solo build debug-panics).
#[wasm_bindgen(start)]
pub fn start() {
    #[cfg(feature = "debug-panics")]
    console_error_panic_hook::set_once();
}

/// Re-encodea en el mismo formato detectado.
#[wasm_bindgen]
pub fn compress(input: &[u8], options: JsValue) -> Result<WasmOutput, JsError> {
    let o: WasmOptions = serde_wasm_bindgen::from_value(options).map_err(|e| JsError::new(&e.to_string()))?;
    let opts = to_core(&o)?;
    minipix_core::compress(input, &opts).map(to_output).map_err(|e| map_err(&e))
}

/// Transcodea al formato de options.format.
#[wasm_bindgen]
pub fn convert(input: &[u8], options: JsValue) -> Result<WasmOutput, JsError> {
    let o: WasmOptions = serde_wasm_bindgen::from_value(options).map_err(|e| JsError::new(&e.to_string()))?;
    let opts = to_core(&o)?;
    minipix_core::convert(input, &opts).map(to_output).map_err(|e| map_err(&e))
}

/// Codifica RGBA8 crudo (el worker lo usa para inputs decodificados por el navegador).
#[wasm_bindgen(js_name = encodeRgba)]
pub fn encode_rgba(rgba: &[u8], width: u32, height: u32, format: &str, options: JsValue) -> Result<WasmOutput, JsError> {
    let o: WasmOptions = serde_wasm_bindgen::from_value(options).map_err(|e| JsError::new(&e.to_string()))?;
    let opts = to_core(&o)?;
    let fmt = parse_format(format)?;
    minipix_core::encode_rgba(rgba, width, height, fmt, &opts).map(to_output).map_err(|e| map_err(&e))
}
```

API NOTE: nombres exactos de wasm-bindgen 0.2.123 / serde-wasm-bindgen 0.6 se ajustan al compilar; los tests del Task 4 son el contrato. `getter_with_clone` por el String/Vec — verificar que genera la clase JS esperada. **CamelCase NO es automático en wasm-bindgen** (a diferencia de napi-rs): los getters de campos de struct exportan el nombre Rust (`bytes_in`). Para que JS vea `bytesIn`/`bytesOut` como en el binding Node, usar `#[wasm_bindgen(js_name = ...)]` en getters explícitos o renombrar los campos del struct exportado — verificar el `.d.ts` generado y alinear con lo que consume el worker (protocol.ts usa camelCase).

- [ ] **Step 3: Pipeline de build** — `crates/wasm/build.ps1` (y equivalente en CI):

```powershell
# build.ps1 — wasm-pack con perfil wasm-release + wasm-opt con features de Rust 1.87+
wasm-pack build crates/wasm --target web --out-dir ../../playground/src/lib/wasm --profile wasm-release
```

Con en crates/wasm/Cargo.toml:

```toml
[package.metadata.wasm-pack.profile.wasm-release]
wasm-opt = ["-Oz", "--enable-bulk-memory", "--enable-nontrapping-float-to-int"]
```

(Si wasm-pack 0.15 no soporta el nombre de perfil custom en metadata, usar el bloque `profile.release` de metadata y `--profile wasm-release` solo en cargo — ajustar y reportar. Pin wasm-bindgen-cli 0.2.123 == crate.)

- [ ] **Step 4: Verificar** — `wasm-pack build` produce el paquete en playground/src/lib/wasm (gitignored: agregar `playground/src/lib/wasm/` a .gitignore — artefacto de build); tamaño del .wasm reportado (~1.6-1.8 MB raw esperado). `cargo clippy --workspace --all-targets -- -D warnings` (el crate wasm entra al workspace lint), fmt, y los 65 tests nativos intactos.
- [ ] **Step 5: Commit** — `git commit -am "feat(wasm): binding wasm-bindgen del core (perfil puro-Rust)"`

### Task 4: Smoke de conformance wasm (Node)

**Files:** Create `crates/wasm/tests/smoke.mjs`; Modify ci.yml (job wasm).

- [ ] **Step 1: Build paralelo a nodejs para test** — el target `web` no corre en Node puro; generar build de test: `wasm-pack build crates/wasm --target nodejs --out-dir ../../target/wasm-node-test --profile wasm-release` y un smoke que ejercita la matriz:

```javascript
// crates/wasm/tests/smoke.mjs — corre con: node crates/wasm/tests/smoke.mjs
import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const root = join(dirname(fileURLToPath(import.meta.url)), '..', '..', '..');
const { compress, convert, encodeRgba } = await import(
  new URL(`file://${join(root, 'target/wasm-node-test/minipix_wasm.js')}`).href
);
const goldens = JSON.parse(readFileSync(join(root, 'tests/conformance/goldens.json'), 'utf8'));
const sha = (buf) => createHash('sha256').update(Buffer.from(buf)).digest('hex');

for (const vector of ['gradient_circle', 'flat_colors']) {
  const input = readFileSync(join(root, `tests/vectors/${vector}.png`));
  // PNG: camino 100% puro-Rust idéntico al nativo → byte-igual a los goldens.
  const c = compress(input, {});
  assert.equal(sha(c.data), goldens[`${vector}.compress.png.q75e4`], `${vector} png parity wasm==native`);
  // AVIF encode: rav1e asm-off vs nativo asm-on — rav1e garantiza asm==rust.
  // VERIFICAR empíricamente: si coincide, assert duro; si no, registrar y comparar tamaño ±10%.
  const avif = convert(input, { format: 'avif', effort: 4 });
  console.log(`${vector} avif wasm hash: ${sha(avif.data)} (golden: ${goldens[`${vector}.convert.${'avif'}.q75e4`]})`);
  // JPEG: encoder DISTINTO por diseño (jpeg-encoder vs mozjpeg) — solo sanity:
  const jpg = convert(input, { format: 'jpeg' });
  assert.ok(jpg.bytesOut > 0 && jpg.format === 'jpeg');
  // WebP: solo lossless en wasm. La clave webp.lossless se agrega a goldens en
  // el Step 2 ANTES de correr este smoke — sin la clave el test DEBE fallar:
  const webp = convert(input, { format: 'webp', lossless: true });
  assert.equal(sha(webp.data), goldens[`${vector}.convert.webp.lossless`], `${vector} webp lossless`);
  // encodeRgba:
  const raw = new Uint8Array(4 * 4 * 4).fill(128);
  const er = encodeRgba(raw, 4, 4, 'png', {});
  assert.ok(er.bytesOut > 0);
}
// Errores tipados:
assert.throws(() => compress(new Uint8Array([1, 2, 3]), {}), /UnsupportedFormat/);
assert.throws(() => convert(readFileSync(join(root, 'tests/vectors/flat_colors.png')), { format: 'webp' }), /InvalidOptions/); // lossy webp no disponible en wasm
console.log('wasm conformance: OK');
```

- [ ] **Step 2: Decidir aserciones AVIF/WebP según la realidad**: (a) si el hash AVIF wasm == golden nativo → convertir el console.log en assert duro y documentar la paridad total; si difiere → assert de tamaño ±10% + nota en spec (paridad nativa↔wasm con tolerancias, como ya prevé el spec §9). (b) WebP lossless: agregar la clave `"{vector}.convert.webp.lossless"` a goldens.json regenerando con `MINIPIX_REGEN_GOLDENS=1` SOLO si el camino nativo produce lo mismo (image-webp lossless es puro Rust — debería); las claves existentes NO deben cambiar.
- [ ] **Step 3: CI** — job `wasm` en ci.yml: ubuntu, rustup target add, `cargo install wasm-pack --locked` (o binario), clang presente, build nodejs + `node crates/wasm/tests/smoke.mjs`, y el check del Task 1.
- [ ] **Step 4: Verificar local + Commit** — `git commit -am "test(wasm): smoke de conformance del build wasm + claves webp lossless"`

## Fase B — Playground

### Task 5: Scaffold del playground (Vite 8 + Svelte 5 + TS strict)

**Files:** Create `playground/` (scaffold), `playground/eslint.config.js`, `playground/.prettierrc`; Modify root `.gitignore`.

- [ ] **Step 1: Scaffold** — `npm create vite@latest playground -- --template svelte-ts` (create-vite 9; Node ≥20.19 — local hay 24). Limpiar el demo (Counter etc.). tsconfig: `"strict": true` + `"noUncheckedIndexedAccess": true` (CLAUDE.md).
- [ ] **Step 2: Lint estricto** — eslint flat config con typescript-eslint strictTypeChecked + eslint-plugin-svelte + prettier (CLAUDE.md §Lint). Scripts: `"lint": "eslint . && prettier --check ."`, `"check": "svelte-check"`. Verificar `npm run lint` y `npm run check` limpios sobre el scaffold.
- [ ] **Step 3: Estructura**:

```text
playground/src/
├── App.svelte            # layout: dropzone arriba, lista de jobs, footer
├── lib/
│   ├── wasm/             # output de wasm-pack (gitignored)
│   ├── worker/
│   │   ├── compress.worker.ts   # corre el wasm; protocolo tipado
│   │   ├── protocol.ts          # tipos de mensajes (request/response/progress)
│   │   └── pool.ts              # WorkerPool (N=min(hc-1,3), respawn)
│   ├── state.svelte.ts   # runes: jobs $state, stats $derived
│   └── components/
│       ├── DropZone.svelte
│       ├── JobCard.svelte       # controles + estado + resultado por imagen
│       ├── CompareSlider.svelte # wrapper de img-comparison-slider
│       └── GlobalControls.svelte
```

- [ ] **Step 4: Commit** — `git add -A; if ($?) { git commit -m "feat(playground): scaffold Vite 8 + Svelte 5 + TS strict + eslint" }`

### Task 6: Protocolo de workers + pool

**Files:** Create `playground/src/lib/worker/{protocol.ts,compress.worker.ts,pool.ts}` + unit tests `playground/src/lib/worker/pool.test.ts` (vitest).

- [ ] **Step 1: Protocolo tipado** (sin Comlink — 4.4.2 lleva 19 meses sin release):

```typescript
// protocol.ts
export interface CompressRequest {
  id: number;
  kind: 'compress' | 'convert';
  /// Transferido (no copiado): el ArrayBuffer del archivo.
  data: ArrayBuffer;
  fileName: string;
  options: {
    format?: 'png' | 'jpeg' | 'webp' | 'avif';
    quality?: number;
    effort?: number;
    lossless?: boolean;
  };
}

export type WorkerResponse =
  | { id: number; ok: true; data: ArrayBuffer; format: string; width: number; height: number; bytesIn: number; bytesOut: number; ratio: number }
  | { id: number; ok: false; code: string; message: string }
  | { id: number; progress: 'decoding' | 'encoding' };
```

- [ ] **Step 2: Worker** — inicializa el wasm una vez (`await init()` del glue `--target web`), procesa requests. Camino especial AVIF input (y fallback universal): si `sniff` del lado JS detecta AVIF (magic ftyp), decodificar con `createImageBitmap(new Blob([data]))` + OffscreenCanvas → `getImageData` → `encodeRgba(...)`. Errores: try/catch — un `RuntimeError` (trap wasm) se reporta con `code: 'InternalPanic'` y el pool DEBE descartar este worker (instancia muerta). El resto de errores vienen como `Error` con el convenio `[Code] message` → parsear code.

```typescript
// compress.worker.ts (esqueleto load-bearing)
import init, { compress, convert, encodeRgba } from '../wasm/minipix_wasm.js';
import type { CompressRequest, WorkerResponse } from './protocol';

const ready = init();

function isAvif(bytes: Uint8Array): boolean {
  return bytes.length >= 12 && bytes[4] === 0x66 && bytes[5] === 0x74 && bytes[6] === 0x79 && bytes[7] === 0x70; // 'ftyp'
}

async function browserDecodeToRgba(data: ArrayBuffer): Promise<{ rgba: Uint8Array; width: number; height: number }> {
  const bitmap = await createImageBitmap(new Blob([data]));
  const canvas = new OffscreenCanvas(bitmap.width, bitmap.height);
  const ctx = canvas.getContext('2d');
  if (!ctx) throw new Error('[DecodeError] OffscreenCanvas 2d context unavailable');
  ctx.drawImage(bitmap, 0, 0);
  const img = ctx.getImageData(0, 0, bitmap.width, bitmap.height);
  bitmap.close();
  return { rgba: new Uint8Array(img.data.buffer), width: img.width, height: img.height };
}

self.onmessage = async (ev: MessageEvent<CompressRequest>) => {
  const req = ev.data;
  const post = (msg: WorkerResponse, transfer?: Transferable[]) =>
    (self as unknown as Worker).postMessage(msg, transfer ?? []);
  try {
    await ready;
    const bytes = new Uint8Array(req.data);
    post({ id: req.id, progress: 'encoding' });
    let out;
    if (isAvif(bytes)) {
      // AVIF: el navegador decodifica (core wasm no trae decoder AV1 — plan M2).
      const { rgba, width, height } = await browserDecodeToRgba(req.data);
      const target = req.options.format ?? 'avif';
      out = encodeRgba(rgba, width, height, target, req.options);
    } else {
      out = req.kind === 'convert' ? convert(bytes, req.options) : compress(bytes, req.options);
    }
    const buf = (out.data as Uint8Array).buffer as ArrayBuffer;
    post({ id: req.id, ok: true, data: buf, format: out.format, width: out.width, height: out.height, bytesIn: out.bytesIn, bytesOut: out.bytesOut, ratio: out.ratio }, [buf]);
  } catch (e) {
    const m = e instanceof Error ? e.message : String(e);
    const code = e instanceof WebAssembly.RuntimeError ? 'InternalPanic' : (/^\[(\w+)\]/.exec(m)?.[1] ?? 'Unknown');
    post({ id: req.id, ok: false, code, message: m });
    if (code === 'InternalPanic') self.close(); // instancia muerta: el pool respawnea
  }
};
```

- [ ] **Step 3: Pool** — `pool.ts`: N = `Math.max(1, Math.min((navigator.hardwareConcurrency ?? 4) - 1, 3))` (cap 3: cada instancia wasm puede crecer cientos de MB en AVIF grandes y la memoria wasm nunca se achica). Cola FIFO, un job por worker, `new Worker(new URL('./compress.worker.ts', import.meta.url), { type: 'module' })`. Respawn: si un worker muere (`error`/close tras InternalPanic) se reemplaza y su job se reporta fallido (NO se reintenta automáticamente — un input que trappea volvería a trappear). API: `pool.run(request): Promise<SuccessResponse>` + callback de progreso.
- [ ] **Step 4: Tests vitest** del protocolo y la lógica de cola/respawn (worker mockeado — el wasm real se prueba en Task 9 E2E). `npm run lint` + `npm test` verdes.
- [ ] **Step 5: Commit** — `git commit -am "feat(playground): protocolo tipado + pool de workers con respawn"`

### Task 7: UI — ingreso y controles

**Files:** Create `DropZone.svelte`, `GlobalControls.svelte`, `JobCard.svelte` (parte de controles), `state.svelte.ts`; Modify `App.svelte`.

- [ ] **Step 1: Estado con runes** (`state.svelte.ts`): `jobs = $state<Job[]>([])` con Job = { id, file, status: 'queued'|'working'|'done'|'error', options, result?, error? }; `totals = $derived(...)` (bytes in/out, ahorro %). Defaults de opciones globales: formato destino 'auto' (= compress), quality 75, effort 4; para AVIF en el selector mostrar aviso de lentitud y default effort BAJO (effort 2 ⇒ speed 8) + cap de dimensiones: si width*height > 4MP mostrar advertencia y ofrecer continuar (el encode AVIF wasm es ~10-60 s/MP single-thread — honestidad de UX, plan M2).
- [ ] **Step 2: DropZone** — `<input type="file" multiple accept="image/png,image/jpeg,image/webp,image/avif">` + drag&drop con `DataTransfer.items` (baseline universal; NADA de showDirectoryPicker como dependencia — Chromium-only). Al soltar: crear jobs y encolar en el pool.
- [ ] **Step 3: Controles** — GlobalControls (formato/quality/effort/lossless aplicados a nuevos jobs) y por-job en JobCard (re-encolar al cambiar). Mismo vocabulario que el SDK — el playground ES la demo de la API.
- [ ] **Step 4: Lint + check verdes; commit** — `git commit -am "feat(playground): dropzone, estado runes y controles"`

### Task 8: UI — resultados, comparador y descargas

**Files:** Create `CompareSlider.svelte`; Modify `JobCard.svelte`, `App.svelte`; deps `img-comparison-slider@8`, `fflate@0.8`.

- [ ] **Step 1: Resultado por job** — bytes antes/después, % ahorro, tiempo; preview del output con `<img src={URL.createObjectURL(blob)}>` (AVIF nativo en 93.4% de navegadores; `onerror` ⇒ mensaje "descargá para ver" — Safari <16.4). Revocar object URLs en `$effect` cleanup.
- [ ] **Step 2: CompareSlider.svelte** — wrapper del web component `img-comparison-slider` (import directo, sin wrapper de framework; Svelte maneja custom elements). Original a la izquierda, resultado a la derecha, zoom CSS.
- [ ] **Step 3: Descargas** — individual (`<a download>`) y "Descargar todo (.zip)" con `fflate.zipSync` sobre los outputs (corre en el main thread con los buffers ya listos; si pesa, moverlo a un worker — decidir al medir).
- [ ] **Step 4: Totales** — banner con ahorro acumulado ($derived). Lint + check; commit — `git commit -am "feat(playground): comparador antes/despues, previews y zip"`

### Task 9: E2E con Playwright

**Files:** Create `playground/e2e/smoke.spec.ts`, `playground/playwright.config.ts`; Modify package.json.

- [ ] **Step 1: Setup** — `npm i -D @playwright/test`; config con `webServer: { command: 'npm run dev' }`.
- [ ] **Step 2: Smoke** — sube `tests/vectors/gradient_circle.png` (input file), espera el job done, asserts: ahorro mostrado, preview visible, descarga produce bytes (>0) con magic correcto; convierte a WebP lossless y verifica el SHA-256 contra `tests/conformance/goldens.json` (la promesa byte a byte llevada hasta el BROWSER). Un segundo test: input AVIF (generar con el CLI nativo o committear un fixture chico) pasa por el camino browser-decode y produce salida.
- [ ] **Step 3: CI** — job playground en ci.yml: build wasm (web target) + `npx playwright install --with-deps chromium` + `npm run e2e`. Verificar verde local; commit — `git commit -am "test(playground): e2e Playwright con paridad de goldens en browser"`

### Task 10: Deploy a Cloudflare Pages

**Files:** Create `.github/workflows/deploy-playground.yml`; Modify README.

- [ ] **Step 1: Workflow** — on push a master (el playground "estable") y workflow_dispatch:

```yaml
name: deploy-playground
on:
  push:
    branches: [master]
    paths: ["playground/**", "crates/wasm/**", "crates/core/**"]
  workflow_dispatch:
jobs:
  deploy:
    runs-on: ubuntu-latest
    permissions: { contents: read, deployments: write }
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { targets: wasm32-unknown-unknown }
      - uses: Swatinem/rust-cache@v2
      - run: cargo install wasm-pack --locked
      - run: wasm-pack build crates/wasm --target web --out-dir ../../playground/src/lib/wasm --profile wasm-release
      - uses: actions/setup-node@v4
        with: { node-version: 22 }
      - run: npm ci && npm run build
        working-directory: playground
      - uses: cloudflare/wrangler-action@v4   # pages-action está ARCHIVADO — no usar
        with:
          apiToken: ${{ secrets.CLOUDFLARE_API_TOKEN }}
          accountId: ${{ secrets.CLOUDFLARE_ACCOUNT_ID }}
          command: pages deploy playground/dist --project-name=minipix
```

NO se necesita `_headers` (sin SharedArrayBuffer en M2); dejar comentario en el workflow: threads futuros = 2 líneas COOP/COEP.

- [ ] **Step 2: Setup manual documentado** — README: crear el proyecto Pages "minipix" en el dashboard de Cloudflare + secretos `CLOUDFLARE_API_TOKEN`/`CLOUDFLARE_ACCOUNT_ID` (este paso lo hace el dueño del repo; el workflow queda listo).
- [ ] **Step 3: Commit** — `git commit -am "ci: deploy del playground a Cloudflare Pages (wrangler-action v4)"`

### Task 11: Docs + amendments

**Files:** Modify `README.md`, `CLAUDE.md`, `docs/superpowers/specs/2026-06-11-minipix-design.md`.

- [ ] **Step 1: Spec §7 amendments** (las 3 desviaciones del header de este plan): códecs C fuera del wasm M2 (emscripten = spike M3 con criterios de salida: los 3 -sys linkan bajo emcc vanilla, errores JPEG unwinden sin matar la instancia, output MODULARIZE carga en worker Vite, tamaño y determinismo aceptables), worker-pool en vez de threads (sin COOP/COEP), carve-out de pánicos wasm.
- [ ] **Step 2: CLAUDE.md regla 4** — agregar: "Excepción wasm: en stable panic=abort no hay catch_unwind; el contrato es respawn del worker ante RuntimeError (ver crates/wasm/src/lib.rs)".
- [ ] **Step 3: README** — sección Playground (URL de Pages cuando exista, limitaciones del build wasm: JPEG menor densidad, WebP solo lossless, AVIF decode por navegador, AVIF encode lento — tabla honesta wasm vs native) + comandos de build local (LLVM/clang en Windows).
- [ ] **Step 4: Commit final M2** — `git commit -am "docs: amendments M2 (wasm puro-Rust, worker pool, carve-out de panics)"`

---

## Verificación final de M2 (checklist de cierre)

- [ ] `cargo test --workspace` (65 tests nativos) con goldens M1 INTACTOS (la matriz de features no cambió bytes nativos) + `cargo check --target wasm32-unknown-unknown --no-default-features --features wasm` + clippy + fmt + deny — todo verde local y en CI.
- [ ] Smoke wasm (Node) verde: PNG byte-igual a goldens nativos; AVIF/WebP según lo decidido en Task 4 Step 2.
- [ ] E2E Playwright verde: compresión real en browser con paridad de goldens en el camino puro-Rust.
- [ ] Playground desplegado en Cloudflare Pages (o workflow listo + pasos manuales documentados si faltan los secretos).
- [ ] Tamaño del .wasm reportado en README (budget: ≤2 MB raw / ≤0.7 MB comprimido).
- [ ] Spec y CLAUDE.md actualizados con los 3 amendments.

**Backlog M3 (registrado, no bloquea M2):** spike emscripten para paridad total de códecs en wasm (criterios de salida en Task 11); threads wasm vía wasm-bindgen-rayon cuando salga de nightly (+COOP/COEP, 2 líneas en Pages); paquete npm `@minipix/wasm`; PWA/offline.
