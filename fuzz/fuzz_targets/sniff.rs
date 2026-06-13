//! Harness de fuzzing para `sniff::sniff`.
//!
//! Objetivo: sobre CUALQUIER input `sniff` debe retornar `Option<Format>` y
//! NUNCA paniquear. El tipo de retorno lo garantiza el compilador; libFuzzer
//! detecta panics/aborts.
//!
//! Ejecutar (Linux/macOS, toolchain nightly):
//!   cargo +nightly fuzz run sniff -- -max_total_time=60
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = minipix_core::sniff::sniff(data);
});
