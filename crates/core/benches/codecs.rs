//! Benchmarks informativos (no bloquean CI). cargo bench -p minipix-core
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]
use criterion::{Criterion, criterion_group, criterion_main};
use minipix_core::{Format, Options, convert};
use std::path::PathBuf;

fn vector() -> Vec<u8> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::read(root.join("tests/vectors/gradient_circle.png")).expect("vector")
}

fn bench_convert(c: &mut Criterion) {
    let input = vector();
    for (fmt, name) in [
        (Format::Jpeg, "jpeg"),
        (Format::WebP, "webp"),
        (Format::Avif, "avif"),
    ] {
        c.bench_function(&format!("convert_png_to_{name}_q75e4"), |b| {
            b.iter(|| convert(&input, &Options::default().with_format(fmt)).expect("convert"));
        });
    }
}

criterion_group!(benches, bench_convert);
criterion_main!(benches);
