//! Robustez sobre input no confiable: sniff/peek/compress nunca paniquean.
//! Determinista (PRNG sembrado) → un pánico encontrado es reproducible.
//!
//! Para ejecutar localmente:
//!   cargo test -p minipix-core --test robustness -- --nocapture
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::cast_possible_truncation)]

use minipix_core::{Format, Options, compress, convert};

// ── SplitMix64 inline ────────────────────────────────────────────────────────
// Sin dependencias, 100 % determinista dada la misma semilla.
struct Rng(u64);

impl Rng {
    const fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn byte(&mut self) -> u8 {
        (self.next() & 0xFF) as u8
    }

    /// Returns a value in [0, n).
    fn range(&mut self, n: usize) -> usize {
        if n == 0 {
            return 0;
        }
        (self.next() % n as u64) as usize
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Opciones con `max_pixels` pequeño para acotar cualquier decode accidental.
/// `Options` es `#[non_exhaustive]` → solo usamos los builder methods.
fn bounded_opts() -> Options {
    let mut o = Options::default();
    o.max_pixels = 4096;
    o.quality = 75;
    o.effort = 1;
    o
}

/// Genera un buffer de `len` bytes aleatorios con el PRNG dado.
fn random_buf(rng: &mut Rng, len: usize) -> Vec<u8> {
    (0..len).map(|_| rng.byte()).collect()
}

/// Construye un PNG mínimo válido de `w×h` px RGBA8 constante con los crates
/// disponibles en dev-deps (no depende de testutil que es pub(crate)).
fn tiny_png(w: u32, h: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut enc = png::Encoder::new(&mut bytes, w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut wr = enc.write_header().unwrap();
    wr.write_image_data(&vec![100u8; (w * h * 4) as usize])
        .unwrap();
    wr.finish().unwrap();
    bytes
}

/// Convierte el PNG semilla al formato pedido con effort bajo.
/// Devuelve `None` si la conversión falla (plataforma sin codec, etc.).
fn seed_for_format(png: &[u8], fmt: Format) -> Option<Vec<u8>> {
    let opts = Options::default()
        .with_quality(60)
        .with_effort(1)
        .with_format(fmt);
    convert(png, &opts).ok().map(|o| o.data)
}

// ── Tipos de mutación ─────────────────────────────────────────────────────────

/// Aplica una mutación aleatoria sobre `buf` y devuelve el buffer mutado.
fn mutate(rng: &mut Rng, buf: &[u8]) -> Vec<u8> {
    let mut out = buf.to_vec();
    if out.is_empty() {
        return out;
    }
    match rng.range(4) {
        0 => {
            // Flip 1-5 bits aleatorios.
            let n = 1 + rng.range(5);
            for _ in 0..n {
                let byte_idx = rng.range(out.len());
                let bit = rng.range(8);
                out[byte_idx] ^= 1 << bit;
            }
        }
        1 => {
            // Truncar en un offset aleatorio (puede producir vacío).
            let at = rng.range(out.len() + 1);
            out.truncate(at);
        }
        2 => {
            // Sobrescribir un tramo aleatorio con bytes random.
            let start = rng.range(out.len());
            let max_run = (out.len() - start).min(64);
            let run = if max_run > 0 {
                1 + rng.range(max_run)
            } else {
                0
            };
            for item in out.iter_mut().skip(start).take(run) {
                *item = rng.byte();
            }
        }
        _ => {
            // Corromper la región del header (primeros 32 bytes) con bytes random.
            let end = out.len().min(32);
            let n = 1 + rng.range(end.max(1));
            let len = out.len();
            for i in 0..n {
                let val = rng.byte();
                out[i % len] = val;
            }
        }
    }
    out
}

/// Mutaciones dirigidas a los campos de tamaño de boxes ISO-BMFF (AVIF/MP4).
/// Estas son las mutaciones que más fiablemente ejercen los assert! de avif-parse:
///   - Sobreescribir bytes 0..4 (size del primer box) con un valor enorme.
///   - Truncar el buffer a mitad de un box (triggering "bytes left" asserts).
///   - Corromper un size field en un offset profundo del container.
fn mutate_avif_box_sizes(rng: &mut Rng, buf: &[u8]) -> Vec<u8> {
    let mut out = buf.to_vec();
    if out.len() < 4 {
        return out;
    }
    match rng.range(3) {
        0 => {
            // Sobreescribir el size del primer box (bytes 0..4) con valor enorme (big-endian).
            // avif-parse 1.4 hace assert!(offset <= size) — un size declarado > datos disponibles
            // dispara ese assert en builds release, y debug_assert_eq! en debug/fuzz.
            out[0] = 0xFF;
            out[1] = 0xFF;
            out[2] = 0xFF;
            out[3] = rng.byte() | 0xF0; // siempre un valor muy grande
        }
        1 => {
            // Truncar a un offset mid-box aleatorio (8..len/2): deja el size field
            // intacto pero los datos son insuficientes → el parser ve "bytes left".
            let min_len = out.len().min(8);
            let max_len = (out.len() / 2).max(min_len);
            let trunc = min_len + rng.range((max_len - min_len).max(1));
            out.truncate(trunc);
        }
        _ => {
            // Corromper un size field en un offset múltiplo de 4 más profundo.
            // Los boxes BMFF están anidados; corromper un size interior es un vector
            // clásico de parser crashes.
            let len = out.len();
            if len >= 8 {
                // Elegir un offset alineado a 4 bytes después del ftyp inicial (≥8).
                let candidates = (len - 4) / 4;
                let idx = (1 + rng.range(candidates.max(1))) * 4;
                let idx = idx.min(len - 4);
                out[idx] = 0xFF;
                out[idx + 1] = 0xFF;
                out[idx + 2] = 0xFF;
                out[idx + 3] = 0xFF;
            }
        }
    }
    out
}

// ── Test 1: sniff nunca panica ────────────────────────────────────────────────

#[test]
fn sniff_nunca_panica() {
    let mut rng = Rng::new(0xDEAD_BEEF_CAFE_1234);

    // ~5000 buffers cortos (0-64 bytes, foco en la región de magic bytes).
    for _ in 0..4800 {
        let len = rng.range(65); // 0..=64
        let buf = random_buf(&mut rng, len);
        let _ = minipix_core::sniff::sniff(&buf);
    }

    // Unos buffers más largos para cubrir el parsing del ftyp AVIF.
    for _ in 0..200 {
        let len = 64 + rng.range(512);
        let buf = random_buf(&mut rng, len);
        let _ = minipix_core::sniff::sniff(&buf);
    }
}

// ── Test 2: compress sobre basura nunca panica ────────────────────────────────

#[test]
fn compress_sobre_basura_nunca_panica() {
    let mut rng = Rng::new(0x1234_5678_ABCD_EF00);
    let opts = bounded_opts();

    for _ in 0..2000 {
        let len = rng.range(512);
        let buf = random_buf(&mut rng, len);
        // La mayoría retorna Err(UnsupportedFormat); ninguna debe paniquear.
        let _ = compress(&buf, &opts);
    }
}

// ── Test 3: compress sobre vectores mutados nunca panica ─────────────────────
//
// Este es el test de mayor valor: cubre los paths de decode reconocido-pero-malformado,
// que es la superficie de ataque real para cada codec.

/// Muta un seed N veces y pasa cada mutación por `compress`.
fn run_mutations(seed: &[u8], n: usize, rng: &mut Rng) {
    let opts = bounded_opts();
    for _ in 0..n {
        let mutated = mutate(rng, seed);
        let _ = compress(&mutated, &opts);
    }
}

#[test]
fn compress_sobre_vectores_mutados_nunca_panica() {
    let mut rng = Rng::new(0xFEDC_BA98_7654_3210);

    // ── Semilla PNG ────────────────────────────────────────────────────────────
    // Leemos el vector del repo (gradient_circle.png, 128×96, ~1 KB).
    let png_path = {
        let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        manifest.join("../../tests/vectors/gradient_circle.png")
    };
    let png_seed = std::fs::read(&png_path).unwrap_or_else(|_| tiny_png(16, 16));

    run_mutations(&png_seed, 300, &mut rng);

    // ── Semilla JPEG ───────────────────────────────────────────────────────────
    // Generamos un JPEG 16×16 directamente con jpeg-encoder (dev-dep).
    let jpeg_seed: Vec<u8> = {
        let rgb = vec![128u8; 16 * 16 * 3];
        let mut out = Vec::new();
        jpeg_encoder::Encoder::new(&mut out, 75)
            .encode(&rgb, 16, 16, jpeg_encoder::ColorType::Rgb)
            .unwrap();
        out
    };
    run_mutations(&jpeg_seed, 300, &mut rng);

    // ── Semilla WebP ───────────────────────────────────────────────────────────
    // convert PNG → WebP (lossy) con effort 1.
    let webp_seed = seed_for_format(&tiny_png(16, 16), Format::WebP)
        .unwrap_or_else(|| random_buf(&mut rng, 64));
    run_mutations(&webp_seed, 300, &mut rng);

    // ── Semilla AVIF ───────────────────────────────────────────────────────────
    // AVIF: encode es lento (rav1e); usamos 4×4 con effort 1 para seed pequeño.
    // Si el codec no está disponible (wasm), se usa basura reconocible como AVIF.
    let avif_seed = seed_for_format(&tiny_png(4, 4), Format::Avif).unwrap_or_else(|| {
        // Construir un header ftyp/avif mínimo para que sniff lo reconozca.
        let mut h = vec![0x00u8, 0x00, 0x00, 0x1C];
        h.extend(b"ftypavif");
        h.extend([0u8; 20]);
        h
    });
    // 50 mutaciones aleatorias (baseline) + 200 mutaciones dirigidas a box sizes ISO-BMFF.
    // Las mutaciones de box size son las que más fiablemente ejercen el catch_unwind
    // (avif-parse assert! sobre size fields). Sin estas mutaciones dirigidas la cobertura
    // del camino de pánico es ~0 con flips aleatorios.
    run_mutations(&avif_seed, 50, &mut rng);
    let opts = bounded_opts();
    for _ in 0..200 {
        let mutated = mutate_avif_box_sizes(&mut rng, &avif_seed);
        let _ = compress(&mutated, &opts);
    }
}

// ── Test 4: convert sobre basura retorna Result ───────────────────────────────

#[test]
fn convert_sobre_basura_nunca_panica() {
    let mut rng = Rng::new(0xCAFE_BABE_0000_1111);
    let fmts = [Format::Png, Format::Jpeg, Format::WebP, Format::Avif];

    for _ in 0..500 {
        let len = rng.range(256);
        let buf = random_buf(&mut rng, len);
        for &fmt in &fmts {
            let opts = Options::default()
                .with_quality(60)
                .with_effort(1)
                .with_format(fmt);
            let _ = convert(&buf, &opts);
        }
    }
}

// ── Test 5 (regresión): AVIF con box size corrupto devuelve Err, no pánico ────
//
// Verifica que catch_unwind en avif.rs convierte el pánico de avif-parse en un
// Error::Decode tipado. El test completando sin abortar ya prueba no-panic;
// el assert! comprueba que también retorna Err (no Ok con datos inválidos).

#[test]
fn avif_con_box_size_corrupto_devuelve_error_no_panica() {
    // Construimos un AVIF válido (4×4, effort 1) y corrompemos el size del primer box.
    let png = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/vectors/gradient_circle.png"),
    )
    .unwrap_or_else(|_| tiny_png(16, 16));

    // Intentar encode AVIF; si no está disponible (wasm/sin codec), usar header mínimo.
    let avif = {
        let opts = Options::default().with_format(Format::Avif).with_effort(1);
        convert(&png, &opts).map_or_else(
            |_| {
                // Codec no disponible en esta plataforma: construir header ftyp mínimo
                // que sniff reconoce como AVIF para ejercitar el mismo camino de decode.
                let mut h = vec![0x00u8, 0x00, 0x00, 0x1C];
                h.extend(b"ftypavif");
                h.extend([0u8; 64]);
                h
            },
            |out| out.data,
        )
    };

    // Corromper el size del primer box ISO-BMFF (bytes 0..4) con un valor enorme
    // (big-endian 0xFFFFFFF0 >> tamaño real del buffer → avif-parse hace assert!).
    let mut bad = avif.clone();
    if bad.len() >= 4 {
        bad[0] = 0xFF;
        bad[1] = 0xFF;
        bad[2] = 0xFF;
        bad[3] = 0xF0;
    }

    let res = compress(&bad, &Options::default());
    // DEBE ser Err (no panic). El test completando sin abortar prueba no-panic;
    // el assert! adicional verifica que el contrato "malformed AVIF → Err" se cumple.
    assert!(
        res.is_err(),
        "AVIF con box size corrupto debe retornar Err, no Ok ni panic"
    );
}

// ── Test 7: options extremas + basura no paniquan ─────────────────────────────

#[test]
fn options_extremas_con_basura_nunca_paniquan() {
    let mut rng = Rng::new(0x1111_2222_3333_4444);

    // Opciones con valores boundary válidos.
    // (max_pixels no tiene builder; lo seteamos post-default.)
    let mut o_tiny = Options::default();
    o_tiny.max_pixels = 1;
    let opts_tiny = o_tiny.with_quality(1).with_effort(0);

    let mut o_max = Options::default();
    o_max.max_pixels = u64::MAX;
    let opts_max = o_max.with_quality(100).with_effort(9);

    let mut o_lossless = Options::default();
    o_lossless.max_pixels = 4096;
    let opts_lossless = o_lossless
        .with_quality(75)
        .with_effort(4)
        .with_lossless(true);

    for opts in [&opts_tiny, &opts_max, &opts_lossless] {
        for _ in 0..200 {
            let len = rng.range(128);
            let buf = random_buf(&mut rng, len);
            let _ = compress(&buf, opts);
        }
    }
}
