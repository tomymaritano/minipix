//! Goldens compartidos: la matriz (vector × operación) produce hashes estables.
//! Los scripts de Node y Python (Tasks 17/18) verifican los MISMOS hashes.
//! Regenerar con: MINIPIX_REGEN_GOLDENS=1 cargo test -p minipix-core --test conformance
//!
//! RIESGO CONOCIDO cross-OS: los goldens se generaron en Windows/MSVC; rav1e (asm) es el sospechoso #1 si Linux CI diverge. Validar en el primer push; si AVIF diverge, calificar esas claves por plataforma SIN debilitar las claves puras (input/png).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::doc_markdown)]

use minipix_core::{Format, Options, compress, convert};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = crates/core → subir dos niveles.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root")
}

fn sha256_hex(data: &[u8]) -> String {
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

fn matrix() -> Vec<(String, Vec<u8>)> {
    let mut cases = Vec::new();
    for vector in ["gradient_circle", "flat_colors"] {
        let input =
            fs::read(repo_root().join(format!("tests/vectors/{vector}.png"))).expect("vector file");
        // hash del INPUT también: defiende la duplicación testutil/example
        cases.push((format!("{vector}.input.png"), input.clone()));
        let compressed = compress(&input, &Options::default()).expect("compress png");
        cases.push((format!("{vector}.compress.png.q75e4"), compressed.data));
        for (fmt, tag) in [
            (Format::Jpeg, "jpeg"),
            (Format::WebP, "webp"),
            (Format::Avif, "avif"),
        ] {
            let out = convert(&input, &Options::default().with_format(fmt).with_effort(4))
                .expect("convert");
            cases.push((format!("{vector}.convert.{tag}.q75e4"), out.data));
        }
    }
    cases
}

#[test]
fn outputs_matchean_goldens() {
    let golden_path = repo_root().join("tests/conformance/goldens.json");
    let actual: BTreeMap<String, String> = matrix()
        .into_iter()
        .map(|(k, data)| (k, sha256_hex(&data)))
        .collect();
    if std::env::var("MINIPIX_REGEN_GOLDENS").is_ok() {
        fs::create_dir_all(golden_path.parent().unwrap()).expect("mkdir");
        fs::write(
            &golden_path,
            serde_json::to_string_pretty(&actual).expect("json"),
        )
        .expect("write");
        println!("goldens escritos en: {}", golden_path.display());
        return;
    }
    let golden: BTreeMap<String, String> = serde_json::from_str(
        &fs::read_to_string(&golden_path).expect("goldens.json missing: regen first"),
    )
    .expect("parse goldens");
    assert_eq!(
        actual, golden,
        "output cambió: si es intencional, regenerar goldens y justificar en el PR"
    );
}

/// Defiende la duplicación testutil/example: el PNG en disco debe ser
/// EXACTAMENTE el que produce la fórmula (vía el hash del input en goldens).
#[test]
fn vectores_en_disco_decodifican_con_dims_correctas() {
    // gradient_circle 128x96 regenerado en memoria con la fórmula del example
    // (misma que testutil) y encodeado igual → bytes idénticos al archivo.
    // Implementar: regenerar los píxeles aquí (copiar la fórmula UNA tercera vez
    // sería peor: en su lugar, decodificar el PNG del disco y verificar
    // propiedades estructurales clave + el hash ya cubierto por goldens).
    let data = fs::read(repo_root().join("tests/vectors/gradient_circle.png")).expect("vector");
    let out = compress(&data, &Options::default()).expect("decode ok");
    assert_eq!((out.width, out.height), (128, 96));
}
