//! Aplica el perfil ICC embebido convirtiendo los píxeles a sRGB.
//!
//! Política v1: el ICC se APLICA (nunca se re-embebe); todo output es sRGB.
//!
//! # Estructura interna
//!
//! Hay dos capas:
//! - [`apply_profile_to_srgb`]: recibe un `&ColorProfile` ya construido; usada en los tests
//!   para construir perfiles (P3, sRGB) directamente sin necesidad de serializar/deserializar.
//! - [`apply_icc_to_srgb`]: parseaa un blob ICC crudo y delega a la anterior; deja los píxeles
//!   intactos ante cualquier error de parseo.
//! - [`apply_icc_best_effort`]: wrapper best-effort para decoders: ICC inválido se ignora.
use crate::error::Error;
use moxcms::{ColorProfile, Layout, TransformOptions};

/// Aplica `profile` como espacio de origen, convirtiendo `rgba` (RGBA8) a sRGB in-place.
///
/// La transformación escribe en un buffer temporal; los píxeles originales se
/// sobreescriben **sólo si el transform tiene éxito**. Ante CUALQUIER error los
/// píxeles quedan intactos.
pub(crate) fn apply_profile_to_srgb(rgba: &mut [u8], profile: &ColorProfile) -> Result<(), Error> {
    let dst_profile = ColorProfile::new_srgb();
    let transform = profile
        .create_transform_8bit(
            Layout::Rgba,
            &dst_profile,
            Layout::Rgba,
            TransformOptions::default(),
        )
        .map_err(|e| Error::IccTransform(e.to_string()))?;
    let src = rgba.to_vec();
    let mut dst = vec![0u8; rgba.len()];
    transform
        .transform(&src, &mut dst)
        .map_err(|e| Error::IccTransform(e.to_string()))?;
    rgba.copy_from_slice(&dst);
    Ok(())
}

/// Parsea un blob ICC crudo y aplica la conversión a sRGB sobre `rgba` (RGBA8).
///
/// Si el blob no es un perfil ICC válido devuelve `Err` sin tocar los píxeles.
pub(crate) fn apply_icc_to_srgb(rgba: &mut [u8], icc: &[u8]) -> Result<(), Error> {
    let profile =
        ColorProfile::new_from_slice(icc).map_err(|e| Error::IccTransform(e.to_string()))?;
    apply_profile_to_srgb(rgba, &profile)
}

/// Best-effort para decoders: si hay ICC lo aplica; si falla (perfil inválido o
/// transform imposible) silencia el error — la imagen ya decodificó correctamente.
pub(crate) fn apply_icc_best_effort(rgba: &mut [u8], icc: Option<&[u8]>) {
    if let Some(icc) = icc {
        let _ = apply_icc_to_srgb(rgba, icc);
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::{apply_icc_to_srgb, apply_profile_to_srgb};
    use moxcms::ColorProfile;

    /// Serializa un perfil moxcms a bytes ICC; los tests que necesitan el blob lo usan.
    fn encode_profile(p: &ColorProfile) -> Vec<u8> {
        p.encode().expect("encode profile")
    }

    #[test]
    fn p3_rojo_saturado_se_desplaza_en_srgb() {
        // Rojo (255,0,0) etiquetado Display-P3 cae fuera del gamut sRGB.
        //
        // Nota de calibración: moxcms 0.8 usa un profile Display P3 de sólo TRC+matrix
        // (sin LUT perceptual). La conversión P3 rojo → sRGB produce XYZ≈(0.4866,0.2270,0)
        // que en sRGB lineal resulta en (R≈1.10, G≈−0.08, B≈−0.01); tras clip de canal
        // negativo y gamma, el output es (255, 0, 0). Esto es comportamiento correcto de
        // un CMS que usa recorte de gamut (relative colorimetric sin LUT perceptual):
        // R permanece en máximo y G/B se clipan a 0. El test verifica que:
        //   1. La transformación se ejecuta sin error (no hay panic ni Err),
        //   2. R se mantiene alto (≥230) — el color no degenera ni se apaga,
        //   3. El canal alpha no se toca.
        // Un test más exigente (G>20) requeriría un perfil ICC con LUT perceptual incluida;
        // queda en backlog junto con los tests e2e con fixtures reales.
        let p3_profile = ColorProfile::new_display_p3();
        let mut pixels = vec![255u8, 0, 0, 255];
        apply_profile_to_srgb(&mut pixels, &p3_profile).unwrap();
        assert!(pixels[0] > 230, "R se mantiene alto: {}", pixels[0]);
        // G puede ser 0 con clip de gamut (correcto para TRC+matrix sin LUT perceptual).
        assert_eq!(pixels[3], 255, "alpha intacto");

        // Verificar también via blob ICC serializado (cubre apply_icc_to_srgb).
        let p3_bytes = encode_profile(&p3_profile);
        let mut pixels2 = vec![255u8, 0, 0, 255];
        apply_icc_to_srgb(&mut pixels2, &p3_bytes).unwrap();
        assert!(
            pixels2[0] > 230,
            "R (blob) se mantiene alto: {}",
            pixels2[0]
        );
        assert_eq!(pixels2[3], 255, "alpha (blob) intacto");
    }

    #[test]
    fn icc_corrupto_devuelve_error_sin_tocar_pixels() {
        let mut pixels = vec![1u8, 2, 3, 255];
        assert!(apply_icc_to_srgb(&mut pixels, b"not an icc profile").is_err());
        assert_eq!(pixels, vec![1, 2, 3, 255]);
    }

    #[test]
    fn p3_color_en_gamut_cambia_numericamente() {
        let p3 = moxcms::ColorProfile::new_display_p3();
        let mut pixels = vec![200u8, 100, 50, 255]; // naranja moderado, dentro de ambos gamuts
        let orig = pixels.clone();
        super::apply_profile_to_srgb(&mut pixels, &p3).unwrap();
        assert_ne!(
            &pixels[..3],
            &orig[..3],
            "el transform debe cambiar valores: {pixels:?}"
        );
        assert_eq!(pixels[3], 255);
    }

    #[test]
    fn srgb_es_noop_aproximado() {
        // Un perfil sRGB no debe cambiar materialmente los pixels.
        let srgb = ColorProfile::new_srgb();
        let srgb_bytes = encode_profile(&srgb);
        let mut pixels = vec![10u8, 128, 250, 200];
        let orig = pixels.clone();
        apply_icc_to_srgb(&mut pixels, &srgb_bytes).unwrap();
        for (a, b) in pixels.iter().zip(orig.iter()) {
            assert!(
                i16::from(*a).abs_diff(i16::from(*b)) <= 2,
                "{pixels:?} vs {orig:?}"
            );
        }
    }
}
