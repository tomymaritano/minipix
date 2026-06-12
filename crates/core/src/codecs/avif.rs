//! AVIF: encode con ravif (rav1e, Rust puro).
use crate::codecs::ImageEncoder;
use crate::error::Error;
use crate::format::Format;
use crate::image::DecodedImage;
use crate::options::Options;

/// Codec for encoding images to AVIF format.
#[allow(dead_code)] // instanciado sólo en tests hasta que se conecte al dispatcher (Task 15)
pub(crate) struct AvifCodec;

fn encode_err(e: impl std::fmt::Display) -> Error {
    Error::Encode {
        format: Format::Avif,
        detail: e.to_string(),
    }
}

impl ImageEncoder for AvifCodec {
    fn encode(&self, img: &DecodedImage, opts: &Options) -> Result<Vec<u8>, Error> {
        // AVIF lossless existe pero ravif no lo expone → rechazo honesto v1.
        if opts.lossless {
            return Err(Error::InvalidOptions(
                "AVIF lossless is not supported in v1".into(),
            ));
        }

        // Build RGBA8 pixel slice via bytemuck (rgb crate enables "bytemuck"
        // feature so RGBA8 implements Pod — zero-copy cast, no allocation).
        let pixels: &[rgb::RGBA8] = bytemuck::cast_slice(&img.pixels);

        // u32 → usize: images larger than 4 Gpx are impossible in practice.
        #[allow(clippy::cast_possible_truncation)]
        let (w, h) = (img.width as usize, img.height as usize);

        let buf = imgref::Img::new(pixels, w, h);

        // effort 0-9 → speed 10-1 (ravif: 1 = slow/dense, 10 = fast).
        // saturating_sub prevents underflow; max(1) keeps speed in ravif's
        // valid range [1, 10].
        let speed = 10u8.saturating_sub(opts.effort).max(1);

        // ravif's Encoder does not expose a thread-count option in 0.13;
        // rav1e uses a thread pool internally but tiles deterministically,
        // so output bytes are byte-identical regardless of thread count.
        let res = ravif::Encoder::new()
            .with_quality(f32::from(opts.quality))
            .with_alpha_quality(f32::from(opts.alpha_quality))
            .with_speed(speed)
            .encode_rgba(buf)
            .map_err(encode_err)?;

        Ok(res.avif_file)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::codecs::ImageEncoder;
    use crate::testutil::vector_gradient_circle;

    #[test]
    fn encode_produce_avif_valido() {
        let img = vector_gradient_circle(32, 24);
        let bytes = AvifCodec
            .encode(&img, &crate::Options::default().with_effort(9))
            .unwrap();
        assert_eq!(crate::sniff::sniff(&bytes), Some(crate::Format::Avif));
    }

    #[test]
    fn quality_ordena_tamanios() {
        let img = vector_gradient_circle(64, 64);
        let q90 = AvifCodec
            .encode(
                &img,
                &crate::Options::default().with_quality(90).with_effort(9),
            )
            .unwrap();
        let q30 = AvifCodec
            .encode(
                &img,
                &crate::Options::default().with_quality(30).with_effort(9),
            )
            .unwrap();
        assert!(q30.len() < q90.len());
    }

    #[test]
    fn lossless_es_rechazado() {
        let img = crate::testutil::vector_flat_colors(8, 8);
        let err = AvifCodec
            .encode(&img, &crate::Options::default().with_lossless(true))
            .unwrap_err();
        assert!(matches!(err, crate::Error::InvalidOptions(_)));
    }
}
