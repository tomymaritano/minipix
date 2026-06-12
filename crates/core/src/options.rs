use crate::error::Error;
use crate::format::Format;

/// Opciones unificadas (mismo vocabulario en Rust, Node y Python).
#[derive(Debug, Clone)]
pub struct Options {
    /// Formato destino. `None` = mismo formato de entrada (`compress`).
    pub format: Option<Format>,
    /// 1–100. Mapeada por códec (tabla en `codecs/`).
    pub quality: u8,
    /// 0–9: CPU invertido en reducir bytes.
    pub effort: u8,
    /// Fuerza camino sin pérdida (JPEG lo rechaza).
    pub lossless: bool,
    /// Defensa anti-bomba de descompresión.
    pub max_pixels: u64,
    /// JPEG progresivo (default true).
    pub jpeg_progressive: bool,
    /// Calidad del canal alpha en WebP/AVIF (1–100, default 100).
    pub alpha_quality: u8,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            format: None,
            quality: 75,
            effort: 4,
            lossless: false,
            max_pixels: 268_435_456,
            jpeg_progressive: true,
            alpha_quality: 100,
        }
    }
}

impl Options {
    /// Set the output format.
    #[must_use]
    pub fn with_format(mut self, f: Format) -> Self {
        self.format = Some(f);
        self
    }

    /// Set the quality (1-100).
    #[must_use]
    pub fn with_quality(mut self, q: u8) -> Self {
        self.quality = q;
        self
    }

    /// Set the effort level (0-9).
    #[must_use]
    pub fn with_effort(mut self, e: u8) -> Self {
        self.effort = e;
        self
    }

    /// Enable or disable lossless mode.
    #[must_use]
    pub fn with_lossless(mut self, l: bool) -> Self {
        self.lossless = l;
        self
    }

    /// Valida rangos. Llamada por `compress`/`convert` antes de trabajar.
    ///
    /// # Errors
    /// Returns an error if quality, effort, or `alpha_quality` are out of valid ranges.
    pub fn validate(&self) -> Result<(), Error> {
        if !(1..=100).contains(&self.quality) {
            return Err(Error::InvalidOptions(format!(
                "quality must be 1-100, got {}",
                self.quality
            )));
        }
        if self.effort > 9 {
            return Err(Error::InvalidOptions(format!(
                "effort must be 0-9, got {}",
                self.effort
            )));
        }
        if !(1..=100).contains(&self.alpha_quality) {
            return Err(Error::InvalidOptions(format!(
                "alpha_quality must be 1-100, got {}",
                self.alpha_quality
            )));
        }
        Ok(())
    }
}

/// Resultado de `compress`/`convert`.
#[derive(Debug)]
pub struct Output {
    /// Bytes codificados del formato de salida.
    pub data: Vec<u8>,
    /// Formato del output.
    pub format: Format,
    /// Ancho en píxeles.
    pub width: u32,
    /// Alto en píxeles.
    pub height: u32,
    /// Tamaño del input en bytes.
    pub bytes_in: u64,
    /// Tamaño del output en bytes.
    pub bytes_out: u64,
}

impl Output {
    /// `bytes_out` / `bytes_in` (1.0 = sin cambio; < 1.0 = ahorro).
    #[must_use]
    pub fn ratio(&self) -> f64 {
        if self.bytes_in == 0 {
            return 1.0;
        }
        #[allow(clippy::cast_precision_loss)] // tamaños de imagen << 2^52
        {
            self.bytes_out as f64 / self.bytes_in as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_correctos() {
        let o = Options::default();
        assert_eq!(o.quality, 75);
        assert_eq!(o.effort, 4);
        assert!(!o.lossless);
        assert_eq!(o.max_pixels, 268_435_456);
        assert!(o.format.is_none());
    }

    #[test]
    fn builder_encadena() {
        let o = Options::default()
            .with_quality(60)
            .with_effort(9)
            .with_lossless(true);
        assert_eq!((o.quality, o.effort, o.lossless), (60, 9, true));
    }

    #[test]
    fn valida_rangos() {
        assert!(Options::default().with_quality(0).validate().is_err());
        assert!(Options::default().with_quality(101).validate().is_err());
        assert!(Options::default().with_effort(10).validate().is_err());
        assert!(Options::default().validate().is_ok());
    }
}
