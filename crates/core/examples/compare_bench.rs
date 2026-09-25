//! JSON timing probe for `scripts/compare-bench.mjs`.
//!
//! Prints one object per format. Criterion benches stay in `benches/`.
//! Settings match the README table: quality 75, effort 4 (the library defaults).

use std::env;
use std::fs;
use std::process::ExitCode;
use std::time::Instant;

use minipix_core::{Format, Options, convert};

const RUNS: usize = 3;

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: compare_bench <image> [out-dir]");
        return ExitCode::from(2);
    };
    let out_dir = args.next();
    let input = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(err) => {
            eprintln!("read {path}: {err}");
            return ExitCode::from(1);
        }
    };

    for format in [Format::Jpeg, Format::WebP, Format::Avif] {
        let opts = Options::default()
            .with_format(format)
            .with_quality(75)
            .with_effort(4);
        if let Err(err) = convert(&input, &opts) {
            eprintln!("warmup {format:?}: {err}");
            return ExitCode::from(1);
        }
        let mut runs_ms = Vec::with_capacity(RUNS);
        let mut encoded: Vec<u8> = Vec::new();
        for _ in 0..RUNS {
            let started = Instant::now();
            let out = match convert(&input, &opts) {
                Ok(out) => out,
                Err(err) => {
                    eprintln!("convert {format:?}: {err}");
                    return ExitCode::from(1);
                }
            };
            runs_ms.push(started.elapsed().as_secs_f64() * 1000.0);
            encoded = out.data;
        }
        let bytes_out = encoded.len();
        let name = match format {
            Format::Jpeg => "jpeg",
            Format::WebP => "webp",
            Format::Avif => "avif",
            Format::Png => "png",
        };
        let runs = runs_ms
            .iter()
            .map(|ms| format!("{ms:.3}"))
            .collect::<Vec<_>>()
            .join(",");
        println!(
            "{{\"impl\":\"minipix\",\"version\":\"{}\",\"format\":\"{name}\",\"quality\":75,\"effort\":4,\"bytes\":{bytes_out},\"runs_ms\":[{runs}]}}",
            env!("CARGO_PKG_VERSION"),
        );
        if let Some(dir) = &out_dir {
            let file = std::path::Path::new(dir).join(format!("minipix-{name}"));
            if let Err(err) = fs::write(&file, &encoded) {
                eprintln!("write {}: {err}", file.display());
                return ExitCode::from(1);
            }
        }
    }
    ExitCode::SUCCESS
}
