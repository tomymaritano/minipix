# minipix — Design Doc

- **Fecha**: 2026-06-11
- **Estado**: Aprobado en brainstorming; pendiente de plan de implementación
- **Inspiración**: [leerob/pixo](https://github.com/leerob/pixo) — pero como SDK práctico multi-lenguaje, no como ejercicio educativo

## 1. Resumen

**minipix** es un SDK de compresión y conversión de imágenes para desarrolladores, con un único core en Rust publicado en tres registros: **npm** (Node.js/Bun, addon nativo vía napi-rs), **PyPI** (wheels nativos vía PyO3/maturin) y **crates.io** (el core directamente). Soporta **PNG, JPEG, WebP y AVIF** en v1, con dos operaciones (`compress` y `convert`), opciones unificadas entre lenguajes y árbol de dependencias 100% permisivo (MIT/Apache/BSD).

El proyecto incluye además un **playground web** (como el de pixo/Squoosh): una SPA donde se arrastran imágenes y se comprimen **100% en el navegador** — el core compilado a WASM, la imagen nunca sale de la máquina del usuario.

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
| Runtimes v1 | Node.js/Bun nativo + CPython nativo (server-side) + browser vía WASM (solo playground) |
| Frontend | Playground web 100% client-side (WASM), stack Vite + Svelte |
| Licencias | Árbol completo MIT/Apache/BSD/IJG/zlib — sin GPL/LGPL/AGPL |
| Fuera de alcance v1 | Edge runtimes, paquete npm WASM publicado, resize, streaming, CLI, GIF/JPEG XL |

**Hitos de entrega**:

1. **M1 — SDK**: core + bindings + publicación en npm/PyPI/crates.io.
2. **M2 — Playground**: build WASM del core + SPA de compresión, desplegada como sitio estático.

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

```text
compressor/                  (repo)
├── crates/
│   ├── core/                # minipix-core → crates.io
│   │   ├── src/
│   │   │   ├── lib.rs       # API pública: compress(), convert(), Options, Output
│   │   │   ├── sniff.rs     # detección de formato por magic bytes
│   │   │   ├── image.rs     # DecodedImage: píxeles + alpha + ICC + metadata
│   │   │   ├── error.rs     # enum de errores tipados
│   │   │   └── codecs/      # un módulo por formato, contra traits ImageDecoder/ImageEncoder
│   │   │       ├── png.rs
│   │   │       ├── jpeg.rs
│   │   │       ├── webp.rs
│   │   │       └── avif.rs
│   ├── node/                # binding napi-rs → npm "minipix"
│   ├── python/              # binding PyO3 → PyPI "minipix" (maturin, abi3)
│   └── wasm/                # binding wasm-bindgen → playground (M2; no se publica en npm en v1)
├── playground/              # SPA Vite + Svelte (M2) → hosting estático
├── tests/
│   ├── vectors/             # imágenes de prueba compartidas
│   └── conformance/         # suite de paridad cross-lenguaje
└── docs/
```

Principios:

- **Toda la lógica vive en `core`**. Los bindings solo convierten tipos, errores y manejan async. Con la misma versión del core, los tres lenguajes producen bytes idénticos — y eso se prueba en CI.
- **Códecs como plugins**: cada formato implementa los traits `ImageDecoder`/`ImageEncoder` contra la representación intermedia `DecodedImage`. Agregar JPEG XL o un backend AVIF alternativo en v2 no toca la API.

### Matriz de códecs v1 (verificada jun 2026)

| Formato | Decode | Encode | Licencias |
| --- | --- | --- | --- |
| PNG | `png` 0.18 (Rust puro) | `png` + optimización `oxipng` 10.x; lossy: cuantización con `quantette` 0.6 | MIT/Apache |
| JPEG | `zune-jpeg` (Rust puro, velocidad clase libjpeg-turbo) | `mozjpeg` crate (C estático; trellis + progressive — referencia de calidad) | MIT/Apache/Zlib + IJG/BSD |
| WebP | `image-webp` 0.2 (Rust puro, decodifica todo el formato) | `libwebp` vía **`libwebp-sys2` directo** (el wrapper `webp` está poco mantenido — verificado) | MIT/Apache + BSD-3 |
| AVIF | `avif-decode` 1.0 (envuelve aom-decode/libaom; verificado en docs.rs) | `ravif` 0.13 (Rust puro, sobre rav1e) | BSD-2/BSD-3 |

Notas obligatorias de la verificación:

- **No existe encoder WebP lossy permisivo en Rust puro** (image-webp es lossless-only; zenwebp es AGPL) → libwebp es obligatorio.
- **`imagequant` (pngquant) es GPL-3/comercial** → se usa `quantette` (MIT, calidad levemente inferior). Si la calidad de PNG lossy resulta insuficiente en benchmarks, evaluar licencia comercial de imagequant como decisión de negocio separada.
- **rav1e está dormido upstream** (último commit dic 2025) pero es estable y completo; densidad apenas detrás de libaom. Mitigación: la interfaz de códecs permite un backend `libaom`/`SVT-AV1` opcional en v2.
- **Evitar `libavif-rs`** (sin mantenimiento desde jul 2024); el contenedor AVIF se arma con `avif-serialize`/`avif-parse`.

**Decisión AVIF decode (resuelta, verificada contra docs.rs en el plan M1)**: `avif-decode` (kornelski, BSD-3) — API de alto nivel "bytes AVIF → píxeles" que envuelve **aom-decode/libaom**, no dav1d como se asumió inicialmente. Ventaja decisiva: reutiliza el toolchain cmake+nasm que ya exigen mozjpeg/libwebp (no suma meson), y libaom compila a WASM vía emscripten (precedente: el AVIF de Squoosh usaba exactamente libaom). `rav1d` quedó descartado para v1: es un port orientado a C-API sin API Rust de librería usable.

**Códecs por target**: en los targets nativos la matriz aplica completa. En wasm32 (playground), los tres códecs C (`mozjpeg`, `libwebp`, `libaom`) se compilan con **emscripten** — precedente directo: Squoosh distribuyó exactamente esos códecs como WASM durante años. El resto de la matriz es Rust puro y compila a wasm32 sin toolchain adicional. Fallbacks feature-gated si un códec C resistiera el build WASM: `jpeg-encoder` (JPEG, menor densidad) y WebP lossless-only — documentados como degradación, no silenciosos.

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
| `lossless` | bool | false | Fuerza camino sin pérdida (PNG siempre; WebP soporta; JPEG lo rechaza con error; **AVIF lo rechaza en v1** — ravif no expone lossless, verificado en implementación) |
| `keepMetadata` | — | — | **Diferido a v2** (verificado: png 0.18 no escribe iCCP y ravif no embebe ICC). En v1: EXIF/XMP siempre se elimina, y el ICC **se aplica** convirtiendo los píxeles a sRGB al decodificar — los colores nunca se rompen y todo output es sRGB |
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
let out = minipix_core::convert(&buf, &Options::default().with_format(Format::Avif).with_quality(60))?;
```

### Flujo de datos

```text
bytes → sniff (magic bytes) → Decoder del formato
      → DecodedImage { píxeles RGBA8/Gray, alpha, ICC, dimensiones }
      → Encoder(opciones) → bytes + reporte
```

## 7. Playground web (M2)

SPA estática donde se arrastran imágenes y se comprimen **enteramente en el navegador**: el core compilado a WASM, cero backend, cero telemetría sobre el contenido — el argumento de privacidad contra TinyPNG hecho producto.

### 7.1 Stack frontend

Vite 8 + Svelte 5 runes + TypeScript. El core entra vía `crates/wasm` (wasm-bindgen 0.2.123). Protocolo de comunicación: postMessage propio (sin Comlink — el overhead de Comlink no justifica la abstracción para un protocolo de dos mensajes). Librerías adicionales: `img-comparison-slider` (comparador antes/después), `fflate` (zip client-side). Deploy: `wrangler-action@v4` a Cloudflare Pages (`wrangler-action/pages-action` está archivado — se usa el action general con `command: pages deploy`).

### 7.2 Build WASM — perfil puro-Rust (M2 entregado)

El binding `crates/wasm` compila con **perfil 100% Rust estable**, target `wasm32-unknown-unknown`, activando `features = ["wasm"]` en el core. No requiere emscripten ni toolchain C adicional.

**Códecs en wasm (real, M2)**:

| Formato | wasm (M2) | Diferencia vs nativo |
| --- | --- | --- |
| PNG | `png` + `oxipng` (Rust puro) | **Idéntico byte a byte** — paridad verificada |
| JPEG | `jpeg-encoder` (Rust puro) | Menor densidad que mozjpeg — diferencia esperada y documentada |
| WebP | `image-webp` lossless-only (Rust puro) | Solo lossless; encoder distinto → goldens separados (`goldens-wasm.json`) |
| AVIF decode | `createImageBitmap` del navegador → `encodeRgba` | Delegado al navegador; sin libaom en wasm |
| AVIF encode | `ravif` / `rav1e` (Rust puro) | **Idéntico byte a byte** — paridad verificada; single-thread (lento en browser) |

**Paridad medida**: PNG y AVIF son byte-idénticos entre la ruta wasm y la nativa. Esta paridad la afirman los smoke tests y los E2E de Playwright contra los goldens en `tests/conformance/goldens.json`. El assert es duro — una regresión rompe CI. WebP lossless difiere por encoder (goldens separados en `goldens-wasm.json`); JPEG difiere por diseño.

**Ruta emscripten (spike M3)**: `wasm-bindgen` soporta targets emscripten desde 0.2.115/0.2.122 (semanas de madurez en junio 2026). La ruta queda como spike M3 con criterios de salida explícitos: los tres crates `-sys` (`mozjpeg-sys`, `libwebp-sys2`, `avif-decode`) linkan bajo `emcc` vanilla sin parchear, los pánicos de JPEG unwinden sin matar la instancia wasm, el output con `MODULARIZE=1` carga correctamente en un worker de Vite, y el tamaño y determinismo son aceptables (≤ 4 MB gzip, mismos bytes en runs consecutivos). Si los criterios se cumplen: JPEG y WebP lossy alcanzan paridad de densidad con nativo, y AVIF encode recupera libaom.

### 7.3 Paralelismo — worker pool (sin COOP/COEP)

La compresión corre en un **pool de Web Workers** (capacidad 3, una imagen por worker, buffers transferidos via `Transferable`). La UI nunca se congela. **No se requieren headers COOP/COEP ni SharedArrayBuffer** — el pool no usa memoria compartida.

Threads WASM vía `wasm-bindgen-rayon` siguen siendo nightly-only (junio 2026) y quedan para M3. Si llegaran: son 2 líneas de `_headers` en Cloudflare Pages (COOP/COEP), sin cambios de código.

### 7.4 Pánicos en wasm — carve-out de la regla 4

En Rust estable con `panic=abort`, no hay `catch_unwind` disponible — un pánico produce un trap de wasm (`RuntimeError: unreachable` en V8/SpiderMonkey). Contrato: el pool detecta el `RuntimeError` del worker, lo reporta como `InternalPanic` o `WasmInitError`, y respawnea el worker antes de continuar. Ver implementación: `crates/wasm/src/lib.rs` y `playground/src/lib/pool.ts`.

### 7.5 Artefacto y hosting

- **Tamaño**: 1.46 MB raw / ~0.62 MB gzip (perfil `wasm-release` + `wasm-opt -Oz`).
- **Hosting**: Cloudflare Pages. GitHub Pages descartado (no permite headers custom — relevante si M3 habilita threads). M2 NO incluye `_headers`: no se usa SharedArrayBuffer; cuando M3 habilite threads wasm son 2 líneas de COOP/COEP en Pages.
- **Deploy**: automático vía `wrangler-action@v4` en cada push a `master` que toque `playground/`, `crates/wasm/`, `crates/core/`, o el workflow de deploy.

**Funcionalidad**:

- Drag & drop + selector de archivos; múltiples imágenes a la vez.
- Controles por imagen (con defaults globales): formato destino, `quality`, `effort`, `lossless` — los mismos nombres y semántica que el SDK; el playground ES la demo de la API.
- Comparador antes/después con slider y zoom (estilo Squoosh, vía `img-comparison-slider`).
- Bytes entrada/salida y % de ahorro por archivo y total.
- Descarga individual o todo junto (zip generado client-side vía `fflate`).

**Alcance**: el binding `crates/wasm` existe para servir al playground; **no** se publica como paquete npm en v1 (eso queda para v2 junto con edge runtimes).

**Alcance**: el binding `crates/wasm` existe para servir al playground; **no** se publica como paquete npm en v1 (eso queda para v2 junto con edge runtimes).

## 8. Manejo de errores y seguridad

- Enum tipado en el core: `UnsupportedFormat`, `DecodeError`, `EncodeError`, `InvalidOptions`, `LimitExceeded`.
- Cada binding traduce a lo idiomático: clases `Error` con propiedad `code` en JS; jerarquía de excepciones (`MinipixError` base) en Python.
- **Ningún pánico cruza la FFI**: `catch_unwind` en el borde de ambos bindings; un pánico se reporta como error interno, nunca aborta el proceso anfitrión.
- **Límite anti-bomba de descompresión**: tope configurable de píxeles totales (default ~268 MP, como sharp) y de dimensiones; un SDK de compresión procesa input no confiable por definición.
- Respuesta a CVEs: las deps C (mozjpeg, libwebp, dav1d) se monitorean (Dependabot + RUSTSEC); el pipeline permite re-publicar binarios parcheados en los tres registros el mismo día.

## 9. Testing

1. **Unit tests** en el core por camino de códec (cada formato × lossy/lossless × con/sin alpha × con/sin ICC).
2. **Conformance cross-lenguaje**: los mismos vectores de `tests/vectors/` corren contra los artefactos de npm, PyPI y crates.io en CI; se asserta **igualdad byte a byte entre lenguajes**. Entre *versiones* del core solo se exigen tolerancias: cada vector guarda valores golden (SSIM contra el original y tamaño en bytes) generados al crearlo, y una actualización de códec pasa si SSIM no cae más de 0.005 ni el tamaño crece más de 3% respecto del golden; superar eso exige regenerar los goldens de forma explícita y justificada en el PR.
3. **Property tests**: roundtrip `decode(encode(x)) == x` en caminos lossless (proptest).
4. **Fuzzing** (cargo-fuzz) sobre el sniffer y la capa de glue de decoders.
5. **Benchmarks** (criterion) + comparativa reproducible contra sharp y Pillow — no bloquea CI; alimenta README y sostiene el claim de calidad con números.
6. **Playground (M2)**: smoke E2E con Playwright — cargar la página, comprimir un vector en el browser, assertar reducción de tamaño y descarga. Paridad nativo↔WASM con tolerancias (SSIM/tamaño), **no** byte a byte: los caminos SIMD nativos vs WASM pueden divergir legítimamente; la promesa contractual de bytes idénticos aplica solo entre los tres bindings nativos.

## 10. CI y distribución

- **GitHub Actions** con plantillas oficiales: napi-rs CLI para npm, maturin-action para PyPI.
- **Matriz v1**: Linux x64/arm64 (glibc + musl), macOS x64/arm64, Windows x64. (win-arm64: stretch goal.)
- **npm**: binarios por plataforma como `optionalDependencies` (`@minipix/core-linux-x64-gnu`, etc.) — cero postinstall scripts.
- **PyPI**: wheels abi3 (abi3-py39) — un wheel por plataforma cubre todas las versiones de CPython ≥ 3.9.
- **crates.io**: `cargo publish` del core; los usuarios de Rust compilan de fuente (norma del ecosistema). Documentar requisitos: cmake + nasm (mozjpeg), y meson solo si queda dav1d.
- **Release**: un tag → publica a los tres registros con la misma versión.
- Toolchain de CI para deps C: nasm (mozjpeg), cmake (libwebp), meson (solo si la contingencia dav1d se activa).
- **M2**: jobs WASM (cargo wasm-release + wasm-bindgen + wasm-opt, perfil puro-Rust — sin emscripten) que compilan `crates/wasm`, smoke Node, E2E Playwright y deploy del playground a Cloudflare Pages (sin `_headers`; ver §7.5).

## 11. Riesgos y mitigaciones

| Riesgo | Mitigación |
| --- | --- |
| rav1e dormido upstream (AVIF encode) | Estable y funcional hoy; interfaz de códecs permite backend libaom/SVT-AV1 en v2; vigilar el fork activo del ecosistema |
| Calidad PNG lossy de quantette < imagequant (GPL) | Benchmark en implementación; si el gap es visible, decisión de negocio: licencia comercial de imagequant o aceptar el gap documentado |
| Bus factor kornelski (mozjpeg, ravif, avif-serialize) | Pin de versiones + vendoring posible; los crates son maduros y estables |
| Deps C multiplican la matriz de builds | Plantillas napi-rs/maturin-action ya resuelven esto (precedentes: sharp, polars); preferir rav1d sobre dav1d si es viable |
| sharp/Pillow cierran el gap | El diferenciador (paridad 3 lenguajes + licencias permisivas + un solo motor) requiere re-arquitectura para los incumbentes; ejecutar rápido |
| Build WASM de los códecs C (emscripten) es la parte más experimental del proyecto | Precedente directo (Squoosh distribuyó mozjpeg/libwebp como WASM años); está aislado en M2 (no bloquea el SDK); fallbacks Rust puros feature-gated documentados |
| AVIF encode en el browser es lento | Threads WASM vía COOP/COEP en Cloudflare Pages; `effort` bajo por defecto en el playground; indicador de progreso para que no parezca colgado |
| Sostenibilidad (@squoosh/lib murió por staffing) | Alcance v1 deliberadamente acotado; automatización máxima de release; decisión consciente antes de ampliar superficie |

## 12. Ideas para v2+ (explícitamente fuera de v1)

- Resize/variantes responsive; CLI fina sobre el core; publicar el build WASM como paquete npm (`@minipix/wasm`) para browser/edge runtimes (el build ya existe por el playground — falta empaquetado, docs y API pública estable); `keepMetadata` (preservación de EXIF/XMP/ICC — requiere escritura de iCCP/APP2/chunks por códec); JPEG XL y GIF; modo "target size" / calidad perceptual automática (butteraugli/SSIMULACRA); backend AVIF de alta densidad (libaom/SVT-AV1) como feature opcional; PWA/offline para el playground.
