//! Harness de fuzzing para `compress`.
//!
//! Objetivo: sobre CUALQUIER input `compress` debe retornar `Result<Output, Error>`
//! y NUNCA paniquear. max_pixels pequeño acota el tiempo de decode de imágenes
//! grandes que pasen el sniff/peek filter.
//!
//! Ejecutar (Linux/macOS, toolchain nightly):
//!   cargo +nightly fuzz run compress -- -max_total_time=120 -rss_limit_mb=4096
//!
//! El corpus inicial (`fuzz/corpus/compress/`) contiene los PNG de tests/vectors/
//! para que libFuzzer arranque con inputs conocidos y valide las rutas de decode
//! reales. La cobertura se extiende a los 4 formatos via mutaciones del corpus.
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // max_pixels pequeño: evita tiempos de decode largos en inputs que logran
    // pasar el guard de sniff/peek con dimensiones razonables.
    let opts = minipix_core::Options {
        max_pixels: 65536, // 256×256 máximo
        quality: 75,
        effort: 1, // mínimo esfuerzo → máxima velocidad del encode
        ..minipix_core::Options::default()
    };
    let _ = minipix_core::compress(data, &opts);
});
