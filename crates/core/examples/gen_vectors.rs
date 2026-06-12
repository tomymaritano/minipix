// example/tool: expect es aceptable aquí
#![allow(clippy::expect_used)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::doc_markdown)]
//! Genera los vectores de prueba compartidos por Rust/Node/Python. Determinista.
//! Uso: cargo run -p minipix-core --example gen_vectors

use std::fs;

fn write_png(path: &str, w: u32, h: u32, rgba: &[u8]) {
    let file = fs::File::create(path).expect("create vector file");
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut wr = enc.write_header().expect("png header");
    wr.write_image_data(rgba).expect("png data");
    wr.finish().expect("png finish");
}

fn gradient_circle(w: u32, h: u32) -> Vec<u8> {
    let (cx, cy) = (f64::from(w) / 2.0, f64::from(h) / 2.0);
    let r = f64::from(w.min(h)) / 3.0;
    let mut px = Vec::new();
    for y in 0..h {
        for x in 0..w {
            let dist = ((f64::from(x) - cx).powi(2) + (f64::from(y) - cy).powi(2)).sqrt();
            px.extend([
                (x * 255 / w.max(1)) as u8,
                (y * 255 / h.max(1)) as u8,
                ((x + y) * 127 / (w + h).max(1)) as u8,
                if dist < r { 128 } else { 255 },
            ]);
        }
    }
    px
}

fn flat_colors(w: u32, h: u32) -> Vec<u8> {
    let palette: [[u8; 4]; 4] = [
        [255, 0, 0, 255],
        [0, 255, 0, 255],
        [0, 0, 255, 255],
        [255, 255, 0, 255],
    ];
    let mut px = Vec::new();
    for y in 0..h {
        for x in 0..w {
            px.extend(palette[((x / 8 + y / 8) % 4) as usize]);
        }
    }
    px
}

fn main() {
    fs::create_dir_all("tests/vectors").expect("mkdir tests/vectors");
    write_png(
        "tests/vectors/gradient_circle.png",
        128,
        96,
        &gradient_circle(128, 96),
    );
    write_png(
        "tests/vectors/flat_colors.png",
        64,
        64,
        &flat_colors(64, 64),
    );

    // Verificar rutas canónicas escritas
    let gc = std::fs::canonicalize("tests/vectors/gradient_circle.png")
        .expect("canonicalize gradient_circle");
    let fc =
        std::fs::canonicalize("tests/vectors/flat_colors.png").expect("canonicalize flat_colors");
    println!("vectores escritos:");
    println!("  {}", gc.display());
    println!("  {}", fc.display());
}
