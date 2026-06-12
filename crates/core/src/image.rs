use crate::error::Error;

/// Imagen decodificada: SIEMPRE RGBA8 en sRGB (los decoders normalizan).
#[derive(Debug, Clone)]
pub struct DecodedImage {
    /// Ancho en píxeles.
    pub width: u32,
    /// Alto en píxeles.
    pub height: u32,
    /// Píxeles RGBA8; len == width * height * 4.
    pub pixels: Vec<u8>,
}

impl DecodedImage {
    /// Construye validando que el buffer tenga exactamente w*h*4 bytes.
    ///
    /// # Errors
    /// `Error::InvalidOptions` si el largo del buffer no coincide.
    pub fn new(width: u32, height: u32, pixels: Vec<u8>) -> Result<Self, Error> {
        let expected = (u64::from(width)) * (u64::from(height)) * 4;
        if pixels.len() as u64 != expected {
            return Err(Error::InvalidOptions(format!(
                "pixel buffer length {} != {expected} for {width}x{height} RGBA8",
                pixels.len()
            )));
        }
        Ok(Self {
            width,
            height,
            pixels,
        })
    }

    /// Cantidad total de píxeles (w*h).
    #[must_use]
    pub fn pixel_count(&self) -> u64 {
        u64::from(self.width) * u64::from(self.height)
    }

    /// JPEG no tiene alpha: compone sobre blanco (comportamiento TinyPNG/Squoosh).
    #[must_use]
    pub fn to_rgb_over_white(&self) -> Vec<u8> {
        let mut rgb = Vec::with_capacity(self.pixels.len() / 4 * 3);
        for px in self.pixels.chunks_exact(4) {
            let a = u16::from(px[3]);
            for c in &px[0..3] {
                let v = (u16::from(*c) * a + 255 * (255 - a)) / 255;
                #[allow(clippy::cast_possible_truncation)] // v <= 255 por construcción
                rgb.push(v as u8);
            }
        }
        rgb
    }

    /// true si algún píxel tiene alpha < 255 (decide RGB vs RGBA en encoders).
    #[must_use]
    pub fn has_transparency(&self) -> bool {
        self.pixels.chunks_exact(4).any(|px| px[3] != 255)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn img_2x1(px: [[u8; 4]; 2]) -> DecodedImage {
        #[allow(clippy::unwrap_used)]
        DecodedImage::new(2, 1, px.concat()).unwrap()
    }

    #[test]
    fn valida_largo_del_buffer() {
        assert!(DecodedImage::new(2, 2, vec![0; 16]).is_ok());
        assert!(DecodedImage::new(2, 2, vec![0; 15]).is_err());
    }

    #[test]
    fn to_rgb_compone_sobre_blanco() {
        // alpha 0 => blanco puro; alpha 255 => color intacto
        let img = img_2x1([[200, 0, 0, 255], [200, 0, 0, 0]]);
        assert_eq!(img.to_rgb_over_white(), vec![200, 0, 0, 255, 255, 255]);
        // alpha medio: fija el contrato de truncamiento (no round-half).
        let mid = img_2x1([[200, 0, 0, 128], [0, 0, 0, 255]]);
        assert_eq!(&mid.to_rgb_over_white()[0..3], &[227, 127, 127]);
    }

    #[test]
    fn detecta_alpha_real() {
        assert!(!img_2x1([[1, 2, 3, 255], [4, 5, 6, 255]]).has_transparency());
        assert!(img_2x1([[1, 2, 3, 255], [4, 5, 6, 128]]).has_transparency());
    }
}
