//! Vectores de prueba deterministas generados en código (sin binarios en el repo).
#![cfg(test)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::cast_possible_truncation)]

use crate::image::DecodedImage;

/// Gradiente RGB con un círculo semi-transparente en el centro. Determinista.
#[allow(dead_code)] // usado por los códecs (Tasks 6-13)
pub fn vector_gradient_circle(w: u32, h: u32) -> DecodedImage {
    let mut px = Vec::with_capacity((w * h * 4) as usize);
    let (cx, cy) = (f64::from(w) / 2.0, f64::from(h) / 2.0);
    let r = f64::from(w.min(h)) / 3.0;
    for y in 0..h {
        for x in 0..w {
            let dist = ((f64::from(x) - cx).powi(2) + (f64::from(y) - cy).powi(2)).sqrt();
            let alpha = if dist < r { 128 } else { 255 };
            px.extend([
                (x * 255 / w.max(1)) as u8,
                (y * 255 / h.max(1)) as u8,
                ((x + y) * 127 / (w + h).max(1)) as u8,
                alpha,
            ]);
        }
    }
    DecodedImage::new(w, h, px).unwrap()
}

/// Imagen plana de pocos colores (caso ideal para PNG indexado).
#[allow(dead_code)] // usado por los códecs (Tasks 6-13)
pub fn vector_flat_colors(w: u32, h: u32) -> DecodedImage {
    let palette: [[u8; 4]; 4] = [
        [255, 0, 0, 255],
        [0, 255, 0, 255],
        [0, 0, 255, 255],
        [255, 255, 0, 255],
    ];
    let mut px = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            px.extend(palette[((x / 8 + y / 8) % 4) as usize]);
        }
    }
    DecodedImage::new(w, h, px).unwrap()
}
