# minipix

**minipix** is an image compression and conversion SDK supporting PNG, JPEG, WebP, and AVIF. A single Rust core (`minipix-core`) is published to npm, PyPI, and crates.io, giving JavaScript, Python, and Rust users the same API, the same encoding engine, and byte-identical output across all three languages — verified by SHA-256 conformance goldens in CI. The dependency tree is permissive-licensed (one documented MPL-2.0 file-level-copyleft exception, see [Codecs & licenses](#codecs--licenses)); compliance is enforced by `cargo-deny` in CI.

> **Warning: Pre-release (v0.1.0, milestone M1).** APIs are stabilising. Breaking changes may occur before the stable tag.

---

## Install

Publishing lands with the `v0.1.0` tag. The package names below are reserved:

```sh
npm install minipix
```

```sh
pip install minipix
```

```sh
cargo add minipix-core
```

npm and PyPI users receive prebuilt binaries/wheels. crates.io users build from source (see [Building from source](#building-from-source)).

---

## Quickstart

**JavaScript / Node**

```js
import { compress, convert } from 'minipix';
const out = await convert(buf, { format: 'avif', quality: 60 });
console.log(`${out.bytesIn} -> ${out.bytesOut} bytes (${(out.ratio * 100).toFixed(1)}%)`);
```

**Python**

```python
import minipix
out = minipix.convert(data, format="avif", quality=60)
print(f"{out.bytes_in} -> {out.bytes_out} bytes ({out.ratio:.1%})")
```

**Rust**

```rust
use minipix_core::{convert, Format, Options};
let out = convert(&data, &Options::default().with_format(Format::Avif).with_quality(60))?;
println!("{} -> {} bytes", out.bytes_in, out.bytes_out);
```

`compress` re-encodes in the same format; `convert` transcodes to the format specified in options.

---

## Options

| Option | JS/Python name | Rust field | Default | Description |
|---|---|---|---|---|
| format | `format` | `format` | *(input format for compress; required for convert)* | Output format: `png`, `jpeg`, `webp`, `avif` |
| quality | `quality` | `quality: u8` | `75` | Lossy quality 1–100 (higher = better) |
| effort | `effort` | `effort: u8` | `4` | Encoder effort 0–9 (higher = slower, smaller) |
| lossless | `lossless` | `lossless: bool` | `false` | Force lossless path |
| alphaQuality / alpha_quality | `alphaQuality` / `alpha_quality` | `alpha_quality: u8` | `100` | Alpha channel quality for WebP/AVIF, 1–100 |
| jpegProgressive / jpeg_progressive | `jpegProgressive` / `jpeg_progressive` | `jpeg_progressive: bool` | `true` | Emit progressive JPEG scans |
| maxPixels / max_pixels | `maxPixels` / `max_pixels` | `max_pixels: u64` | `268435456` | Anti-decompression-bomb pixel limit |

### Behavioral notes

- **PNG lossy** uses palette quantisation (quantette, Floyd-Steinberg dithering) for opaque images. Images with any transparent pixel fall back to lossless RGBA8 automatically.
- **JPEG** rejects `lossless: true` with `InvalidOptions`.
- **AVIF** lossless is not supported in v1 and is rejected with `InvalidOptions`.
- **Animated WebP** inputs are rejected with an explicit `Decode` error rather than silently flattening to the first frame.
- Metadata is always stripped. Embedded ICC profiles are **applied** during decode: output pixels are always in sRGB. Exception: AVIF images with an embedded ICC (`colr` box of type `prof`) are not colour-transformed in v1 (backlog; nclx/CICP is handled correctly).

---

## Errors

### Rust

```rust
pub enum Error {
    UnsupportedFormat,
    Decode { format: Format, detail: String },
    Encode { format: Format, detail: String },
    InvalidOptions(String),
    LimitExceeded { pixels: u64, limit: u64 },
    IccTransform(String),
}
```

### Node (JavaScript)

Errors carry a `[CODE] message` prefix, e.g. `[UnsupportedFormat] …`. Invalid option values surface as a dedicated error with an `InvalidArg` status code (distinct from codec errors).

### Python

| Exception class | When raised |
|---|---|
| `MinipixError` | Base class for all minipix errors |
| `UnsupportedFormatError` | Unknown or unsupported input format |
| `CodecError` | Decode or encode failure |
| `LimitExceededError` | Image exceeds `max_pixels` limit |
| `ValueError` | Invalid option values (quality/effort out of range, etc.) |

---

## Codecs & licenses

| Format | Decode | Encode | License |
|---|---|---|---|
| PNG | `png` | `png` + `oxipng` (optimiser) + `quantette` (palette quantisation) | MIT / Apache-2.0 |
| JPEG | `zune-jpeg` | `mozjpeg` | zune: MIT/Apache; mozjpeg: BSD-3 / IJG |
| WebP | `image-webp` | `libwebp` (`libwebp-sys2`) | BSD-3 |
| AVIF | `avif-decode` / `libaom` | `ravif` / `rav1e` | BSD-2 + patent grant (libaom); BSD-2 (rav1e) |
| Color | `moxcms` (ICC → sRGB) | — | BSD-3 / Apache-2.0 |

All dependencies are permissive (MIT/Apache/BSD/Zlib/IJG/NCSA), with one documented exception: `avif-parse` (the AVIF container parser) is MPL-2.0 — weak file-level copyleft that imposes no obligations on minipix or its users since the crate is consumed unmodified. No GPL/LGPL/AGPL anywhere in the tree. Compliance enforced by `cargo-deny` in CI (see `deny.toml` for the documented rationale).

---

## Building from source

crates.io users (and contributors) need:

- **CMake** (any recent version)
- **nasm 2.16.x** — nasm 3.x breaks libaom's cmake help-text check; install 2.16.x specifically
- **Perl** — required by mozjpeg's build scripts. On Windows, use [Strawberry Perl](https://strawberryperl.com/)

npm and PyPI users receive prebuilt binaries and do not need these tools.

---

## Architecture

The repository is a Cargo workspace with three crates:

- `crates/core` — pure Rust codec logic, all public API
- `crates/node` — Napi-rs bindings (npm package)
- `crates/python` — PyO3 bindings (PyPI wheel)

Codecs are kept behind internal `ImageDecoder` / `ImageEncoder` traits. Cross-language conformance is verified by SHA-256 goldens stored in `tests/conformance/goldens.json` and checked in CI against all three language bindings.

For design rationale see [`docs/superpowers/specs/2026-06-11-minipix-design.md`](docs/superpowers/specs/2026-06-11-minipix-design.md) and [`CLAUDE.md`](CLAUDE.md).

---

## Playground (M2)

A 100% client-side WebAssembly playground is coming in milestone M2. No server required — compress and convert images entirely in the browser.

---

## License

Licensed under either of:

- [MIT license](LICENSE-MIT)
- [Apache License, Version 2.0](LICENSE-APACHE)

at your option.
