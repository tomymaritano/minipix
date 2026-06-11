# minipix — Design Doc

- **Fecha**: 2026-06-11
- **Estado**: Aprobado en brainstorming; pendiente de plan de implementación
- **Inspiración**: [leerob/pixo](https://github.com/leerob/pixo) — pero como SDK práctico multi-lenguaje, no como ejercicio educativo

## 1. Resumen

**minipix** es un SDK de compresión y conversión de imágenes para desarrolladores, con un único core en Rust publicado en tres registros: **npm** (Node.js/Bun, addon nativo vía napi-rs), **PyPI** (wheels nativos vía PyO3/maturin) y **crates.io** (el core directamente). Soporta **PNG, JPEG, WebP y AVIF** en v1, con dos operaciones (`compress` y `convert`), opciones unificadas entre lenguajes y árbol de dependencias 100% permisivo (MIT/Apache/BSD).

Promesa diferencial: **misma API, mismo motor y bytes de salida idénticos en los tres lenguajes** para una misma versión del core.

Nombre verificado disponible en npm, PyPI y crates.io el 2026-06-11.

## 2. Contexto y motivación

Investigación verificada contra fuentes primarias (junio 2026):

- **El nicho está vacante**: ningún proyecto mantenido ofrece una API unificada de compresión PNG/JPEG/WebP/AVIF en npm + PyPI + crates.io. `@squoosh/lib` (el antecesor directo) fue abandonado por Google. Los más cercanos no llegan: imageflow es AGPL/comercial; libcaesium (Apache-2.0) no tiene AVIF ni bindings npm/PyPI; sharp es solo Node; Pillow/pyvips solo Python; imagemin está sin mantenimiento.
- **Dolores explotables**: Pillow es single-thread y sin encoders de máxima calidad (sin mozjpeg, sin oxipng, sin cuantización lossy de PNG); sharp/pyvips arrastran licencias LGPL (libvips/libheif) que bloquean adopción enterprise (precedente: vercel/next.js#72406); TinyPNG genera rechazo por precio y privacidad (las imágenes salen de tu infraestructura) — empuje hacia herramientas locales.
- **Matices**: sharp 0.35.0 (jun 2026) ya eliminó el postinstall script y Pillow 11.3+ trae AVIF nativo; el diferenciador NO es instalación ni cobertura de formatos, sino **paridad multi-lenguaje + licencias permisivas + calidad de encode**.

## 3. Requisitos (decisiones cerradas)

| Decisión | Valor |
| --- | --- |
| Propósito | Librería/SDK para developers |
| Ecosistemas | npm + PyPI + crates.io desde UN core compartido |
| Códecs | Reusar los mejores existentes; no reimplementar |
| Formatos v1 | PNG, JPEG, WebP, AVIF (decode + encode los cuatro) |
| Alcance funcional v1 | `compress` (re-encode mismo formato) + `convert` (transcodificación) |
| Runtimes v1 | Node.js/Bun nativo + CPython nativo (server-side) |
| Licencias | Árbol completo MIT/Apache/BSD/IJG/zlib — sin GPL/LGPL/AGPL |
| Fuera de alcance v1 | Browser/WASM, edge runtimes, resize, streaming, CLI, GIF/JPEG XL |

## 4. Análisis de lenguajes (resumen)

Investigado por agentes con verificación adversarial de afirmaciones clave (12 agentes, fuentes primarias: crates.io, GitHub, PyPI, npm, changelogs).

| Opción | Aptitud | Veredicto |
| --- | --- | --- |
| **Rust** | **8.5/10** | **Elegido.** Único lenguaje donde los tres registros son de primera clase: napi-rs v3 (activo, jun 2026) para npm, PyO3+maturin (wheels abi3) para PyPI, `cargo publish` gratis para crates.io. Precedentes a escala: polars, pydantic-core, SWC, rspack. |
| C/C++ | 7/10 | Acceso directo a jpegli/libaom, pero el costo de respuesta a CVEs de 5-8 deps C es propio (precedente libwebp CVE-2023-4863), jpegli no tiene releases etiquetados, y la pata crates.io queda de segunda clase. |
| Sin core compartido (sharp+pyvips+facade Rust) | 4.5/10 | Rápido a v1 en npm/PyPI pero con drift estructural entre lenguajes (libheif+aom vs ravif producen bytes distintos), 3x mantenimiento, y la pata Rust termina siendo ~60% del core evitado. Renuncia al diferenciador. |
| Zig | 4/10 | Interop C excelente, pero pre-1.0 (rompe código cada ~8 meses), ziggy-pydust atrasado 2 versiones de Zig, crates.io casi inviable desde un core Zig. |
| Go | 3/10 | Sin encoder WebP lossy ni AVIF puros; embeber el runtime Go en Node/CPython tiene footguns documentados (señales SA_ONSTACK, fork-unsafe, sin dlclose — issues golang/go #11100, #15538, #65050); crates.io inalcanzable. |

## 5. Arquitectura

Workspace de Cargo, tres crates, bindings sin lógica:

```
compressor/                  (repo)
├── crates/
│   ├── core/                # minipix-core → crates.io
│   │   ├── src/
│   │   │   ├── lib.rs       # API pública: compress(), convert(), Options, Output
│   │   │   ├── sniff.rs     # detección de formato por magic bytes
│   │   │   ├── image.rs     # DecodedImage: píxeles + alpha + ICC + metadata
│   │   │   ├── error.rs     # enum de errores tipados
│   │   │   └── codecs/      # un módulo por formato, contra traits Decoder/Encoder
│   │   │       ├── png.rs
│   │   │       ├── jpeg.rs
│   │   │       ├── webp.rs
│   │   │       └── avif.rs
│   ├── node/                # binding napi-rs → npm "minipix"
│   └── python/              # binding PyO3 → PyPI "minipix" (maturin, abi3)
├── tests/
│   ├── vectors/             # imágenes de prueba compartidas
│   └── conformance/         # suite de paridad cross-lenguaje
└── docs/
```

Principios:

- **Toda la lógica vive en `core`**. Los bindings solo convierten tipos, errores y manejan async. Con la misma versión del core, los tres lenguajes producen bytes idénticos — y eso se prueba en CI.
- **Códecs como plugins**: cada formato implementa los traits `Decoder`/`Encoder` contra la representación intermedia `DecodedImage`. Agregar JPEG XL o un backend AVIF alternativo en v2 no toca la API.

### Matriz de códecs v1 (verificada jun 2026)

| Formato | Decode | Encode | Licencias |
| --- | --- | --- | --- |
| PNG | `png` 0.18 (Rust puro) | `png` + optimización `oxipng` 10.x; lossy: cuantización con `quantette` 0.6 | MIT/Apache |
| JPEG | `zune-jpeg` (Rust puro, velocidad clase libjpeg-turbo) | `mozjpeg` crate (C estático; trellis + progressive — referencia de calidad) | MIT/Apache/Zlib + IJG/BSD |
| WebP | `image-webp` 0.2 (Rust puro, decodifica todo el formato) | `libwebp` vía **`libwebp-sys2` directo** (el wrapper `webp` está poco mantenido — verificado) | MIT/Apache + BSD-3 |
| AVIF | `dav1d` crate 0.11 (binding C, el más rápido) | `ravif` 0.13 (Rust puro, sobre rav1e) | MIT + BSD-2/BSD-3 |

Notas obligatorias de la verificación:

- **No existe encoder WebP lossy permisivo en Rust puro** (image-webp es lossless-only; zenwebp es AGPL) → libwebp es obligatorio.
- **`imagequant` (pngquant) es GPL-3/comercial** → se usa `quantette` (MIT, calidad levemente inferior). Si la calidad de PNG lossy resulta insuficiente en benchmarks, evaluar licencia comercial de imagequant como decisión de negocio separada.
- **rav1e está dormido upstream** (último commit dic 2025) pero es estable y completo; densidad apenas detrás de libaom. Mitigación: la interfaz de códecs permite un backend `libaom`/`SVT-AV1` opcional en v2.
- **Evitar `libavif-rs`** (sin mantenimiento desde jul 2024); el contenedor AVIF se arma con `avif-serialize`/`avif-parse`.

**Decisión abierta (resolver en implementación)**: AVIF decode con `dav1d` (C, requiere meson en CI, el más rápido) vs `rav1d` (Rust puro, ~5% más lento single-thread, elimina meson). Criterio: si la API del crate `rav1d` resulta usable directamente, preferirla para simplificar la matriz de builds.

## 6. API

Dos operaciones, mismos nombres, mismas opciones y misma semántica en los tres lenguajes:

- **`compress(input, options)`** — detecta el formato por magic bytes y re-encodea optimizado en el mismo formato.
- **`convert(input, options)`** — transcodea al formato de `options.format`.

### Opciones unificadas

| Opción | Tipo | Default | Semántica |
| --- | --- | --- | --- |
| `format` | enum | — (requerido en `convert`) | png \| jpeg \| webp \| avif |
| `quality` | 1–100 | 75 | Tabla de mapeo estática y documentada por códec (calibrada una vez con SSIM sobre los vectores de prueba) para que el mismo número dé calidad visual comparable entre formatos; auto-ajuste perceptual queda para v2 |
| `effort` | 0–9 | 4 | CPU invertido en reducir bytes (nivel oxipng / effort libwebp / speed rav1e invertido) |
| `lossless` | bool | false | Fuerza camino sin pérdida (PNG siempre; WebP/AVIF soportan; JPEG lo rechaza con error) |
| `keepMetadata` | bool | false | Por defecto se elimina EXIF/XMP. El perfil ICC **se respeta siempre** (se aplica o se preserva) para no romper colores |
| Por formato | namespace | — | `jpeg.progressive`, `jpeg.chromaSubsampling`, `png.interlace`, `avif.chromaSubsampling`, `webp.alphaQuality`, etc. |

### Resultado

`{ data, format, width, height, bytesIn, bytesOut, ratio }` — un pipeline de build puede loguear ahorros sin trabajo extra.

### Superficie por lenguaje

```ts
// npm — async por defecto (encode fuera del event loop, vía napi async tasks)
import { compress, convert } from 'minipix';
const out = await convert(buf, { format: 'avif', quality: 60 });
// también: compressSync / convertSync para scripts
```

```python
# PyPI — síncrono; libera el GIL durante decode/encode (threading real)
from minipix import compress, convert
out = convert(data, format="avif", quality=60)
```

```rust
// crates.io — síncrono; helper de batch paralelo con rayon como feature
let out = minipix_core::convert(&buf, Options::format(Format::Avif).quality(60))?;
```

### Flujo de datos

```
bytes → sniff (magic bytes) → Decoder del formato
      → DecodedImage { píxeles RGBA8/Gray, alpha, ICC, dimensiones }
      → Encoder(opciones) → bytes + reporte
```

## 7. Manejo de errores y seguridad

- Enum tipado en el core: `UnsupportedFormat`, `DecodeError`, `EncodeError`, `InvalidOptions`, `LimitExceeded`.
- Cada binding traduce a lo idiomático: clases `Error` con propiedad `code` en JS; jerarquía de excepciones (`MinipixError` base) en Python.
- **Ningún pánico cruza la FFI**: `catch_unwind` en el borde de ambos bindings; un pánico se reporta como error interno, nunca aborta el proceso anfitrión.
- **Límite anti-bomba de descompresión**: tope configurable de píxeles totales (default ~268 MP, como sharp) y de dimensiones; un SDK de compresión procesa input no confiable por definición.
- Respuesta a CVEs: las deps C (mozjpeg, libwebp, dav1d) se monitorean (Dependabot + RUSTSEC); el pipeline permite re-publicar binarios parcheados en los tres registros el mismo día.

## 8. Testing

1. **Unit tests** en el core por camino de códec (cada formato × lossy/lossless × con/sin alpha × con/sin ICC).
2. **Conformance cross-lenguaje**: los mismos vectores de `tests/vectors/` corren contra los artefactos de npm, PyPI y crates.io en CI; se asserta **igualdad byte a byte entre lenguajes**. Entre *versiones* del core solo se exigen tolerancias: cada vector guarda valores golden (SSIM contra el original y tamaño en bytes) generados al crearlo, y una actualización de códec pasa si SSIM no cae más de 0.005 ni el tamaño crece más de 3% respecto del golden; superar eso exige regenerar los goldens de forma explícita y justificada en el PR.
3. **Property tests**: roundtrip `decode(encode(x)) == x` en caminos lossless (proptest).
4. **Fuzzing** (cargo-fuzz) sobre el sniffer y la capa de glue de decoders.
5. **Benchmarks** (criterion) + comparativa reproducible contra sharp y Pillow — no bloquea CI; alimenta README y sostiene el claim de calidad con números.

## 9. CI y distribución

- **GitHub Actions** con plantillas oficiales: napi-rs CLI para npm, maturin-action para PyPI.
- **Matriz v1**: Linux x64/arm64 (glibc + musl), macOS x64/arm64, Windows x64. (win-arm64: stretch goal.)
- **npm**: binarios por plataforma como `optionalDependencies` (`@minipix/core-linux-x64-gnu`, etc.) — cero postinstall scripts.
- **PyPI**: wheels abi3 (abi3-py39) — un wheel por plataforma cubre todas las versiones de CPython ≥ 3.9.
- **crates.io**: `cargo publish` del core; los usuarios de Rust compilan de fuente (norma del ecosistema). Documentar requisitos: cmake + nasm (mozjpeg), y meson solo si queda dav1d.
- **Release**: un tag → publica a los tres registros con la misma versión.
- Toolchain de CI para deps C: nasm (mozjpeg), cmake (libwebp), meson (dav1d, si aplica).

## 10. Riesgos y mitigaciones

| Riesgo | Mitigación |
| --- | --- |
| rav1e dormido upstream (AVIF encode) | Estable y funcional hoy; interfaz de códecs permite backend libaom/SVT-AV1 en v2; vigilar el fork activo del ecosistema |
| Calidad PNG lossy de quantette < imagequant (GPL) | Benchmark en implementación; si el gap es visible, decisión de negocio: licencia comercial de imagequant o aceptar el gap documentado |
| Bus factor kornelski (mozjpeg, ravif, avif-serialize) | Pin de versiones + vendoring posible; los crates son maduros y estables |
| Deps C multiplican la matriz de builds | Plantillas napi-rs/maturin-action ya resuelven esto (precedentes: sharp, polars); preferir rav1d sobre dav1d si es viable |
| sharp/Pillow cierran el gap | El diferenciador (paridad 3 lenguajes + licencias permisivas + un solo motor) requiere re-arquitectura para los incumbentes; ejecutar rápido |
| Sostenibilidad (@squoosh/lib murió por staffing) | Alcance v1 deliberadamente acotado; automatización máxima de release; decisión consciente antes de ampliar superficie |

## 11. Ideas para v2+ (explícitamente fuera de v1)

- Resize/variantes responsive; CLI fina sobre el core; WASM para browser/edge (rav1d y los códecs Rust puros ya compilan a wasm32; mozjpeg/libwebp requerirían emscripten o reemplazo); JPEG XL y GIF; modo "target size" / calidad perceptual automática (butteraugli/SSIMULACRA); backend AVIF de alta densidad (libaom/SVT-AV1) como feature opcional.
