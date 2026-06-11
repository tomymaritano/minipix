# CLAUDE.md — minipix

SDK de compresión/conversión de imágenes (PNG, JPEG, WebP, AVIF) con core en Rust publicado en npm + PyPI + crates.io, más un playground web 100% client-side (WASM). El diseño completo y sus porqués viven en [el spec](docs/superpowers/specs/2026-06-11-minipix-design.md) — leerlo antes de tocar arquitectura o alcance.

## Reglas de arquitectura (duras)

1. **Toda la lógica vive en `crates/core`.** Los bindings (`node`, `python`, `wasm`) solo convierten tipos, errores y manejan async. Si un binding necesita un `if` de negocio, ese `if` va al core.
2. **Códecs solo detrás de los traits `Decoder`/`Encoder`** contra `DecodedImage`. Nada fuera de `crates/core/src/codecs/` llama a un crate de códec directamente.
3. **Paridad byte a byte entre bindings nativos** es promesa contractual. Cualquier cambio que altere bytes de salida regenera los goldens de forma explícita y se justifica en el PR (tolerancias: SSIM −0.005 / tamaño +3%, spec §9).
4. **Ningún pánico cruza la FFI**: `catch_unwind` en el borde de cada binding. Un pánico se reporta como error interno, jamás aborta el proceso anfitrión.
5. **Árbol de licencias permisivo** (MIT/Apache/BSD/Zlib/IJG). Nada GPL/LGPL/AGPL — lo aplica `cargo deny check` en CI; no agregar excepciones a `deny.toml` sin discutirlo primero.

## Principios

- **KISS**: la implementación simple y obvia primero. La complejidad se justifica con un problema real medido, no anticipado.
- **DRY con regla de tres**: extraer cuando se duplica *conocimiento* (mapeo de `quality`, validación de opciones, manejo de ICC), no cuando el código se *parece*. Los módulos de códec van a parecerse entre sí — está bien; no crear abstracciones más allá de los traits existentes.
- **YAGNI**: el alcance de v1 está cerrado en el spec. No agregar opciones, formatos ni features especulativas "ya que estamos". Las ideas van a la sección v2+ del spec.
- **Errores tipados siempre**: prohibido `unwrap()`, `expect()` y `panic!` en código de librería (en tests sí se permiten). Todo camino de error devuelve `Result<_, Error>` con el enum del core.
- **`unsafe` prohibido** salvo en el borde FFI inevitable, y cada bloque lleva un comentario `// SAFETY:` que documente la invariante que lo hace correcto.
- **API pública documentada** (rustdoc con ejemplo ejecutable) antes de mergear.

## Lint y calidad — estricto, bloquea CI

Cero warnings en `main`. Un warning nuevo es un fallo de CI, no una advertencia.

- **Rust**: `cargo fmt --check` · `cargo clippy --workspace --all-targets -- -D warnings` con `clippy::pedantic` activado a nivel workspace (`[workspace.lints]`); los `allow` puntuales se declaran ahí con justificación en comentario · `cargo deny check` (licencias + advisories de seguridad).
- **TS/Svelte** (playground): `tsconfig` con `strict` + `noUncheckedIndexedAccess` · eslint flat config con `typescript-eslint` *strictTypeChecked* + `eslint-plugin-svelte` · prettier.
- **Python** (tests y stubs `.pyi`): ruff en modo estricto · `mypy --strict` sobre los stubs.
- **Markdown**: markdownlint.

## Comandos

(Actualizar cuando exista el workspace — el scaffolding es parte de M1.)

- Build: `cargo build --workspace`
- Tests: `cargo test --workspace`
- Lint completo: `cargo fmt --check; cargo clippy --workspace --all-targets -- -D warnings; cargo deny check`
- Playground (M2): `npm run dev` / `npm run lint` en `playground/`

## Testing

- TDD: test primero, implementación después (skill `superpowers:test-driven-development`).
- Todo cambio en `crates/core/src/codecs/` corre la suite de conformance contra los vectores de `tests/vectors/`.
- Los benchmarks (criterion) no bloquean CI pero acompañan cualquier PR que afirme una mejora de rendimiento.
