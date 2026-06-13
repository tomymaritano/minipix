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
use moxcms::{ColorProfile, Layout, Transform8BitExecutor, TransformOptions};
use std::sync::Arc;

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

    // Short-circuit: si el transform es identidad (perfil ya sRGB o equivalente),
    // saltear la transformación completa — los píxeles ya están en sRGB.
    // Esto evita 2 copias O(n) + el transform per-píxel completo.
    if transform_is_identity(&transform) {
        return Ok(());
    }

    let src = rgba.to_vec();
    let mut dst = vec![0u8; rgba.len()];
    transform
        .transform(&src, &mut dst)
        .map_err(|e| Error::IccTransform(e.to_string()))?;
    rgba.copy_from_slice(&dst);
    Ok(())
}

/// Devuelve `true` si el transform deja sin cambios (±1 LSB) un conjunto de
/// colores de prueba que cubren los extremos de cada canal y grises.
///
/// Para un transform matrix-shaper RGB→sRGB, identidad en estos probes implica
/// identidad en todo el dominio (una matriz 3×3 queda determinada por su acción
/// sobre los 3 primarios). Conservador: ante cualquier desviación >1 devuelve
/// `false` y se realiza la transformación real — nunca se salta un transform válido.
fn transform_is_identity(transform: &Arc<Transform8BitExecutor>) -> bool {
    const PROBES: [[u8; 4]; 12] = [
        [0, 0, 0, 255],
        [255, 255, 255, 255],
        [255, 0, 0, 255],
        [0, 255, 0, 255],
        [0, 0, 255, 255],
        [64, 64, 64, 255],
        [128, 128, 128, 255],
        [192, 192, 192, 255],
        [255, 128, 0, 255],
        [0, 128, 255, 255],
        [128, 0, 255, 255],
        [33, 177, 99, 128],
    ];
    let src: Vec<u8> = PROBES.iter().flatten().copied().collect();
    let mut dst = vec![0u8; src.len()];
    if transform.transform(&src, &mut dst).is_err() {
        return false; // ante error, no saltear (camino seguro)
    }
    src.iter().zip(&dst).all(|(a, b)| a.abs_diff(*b) <= 1)
}

/// Parsea un blob ICC crudo y aplica la conversión a sRGB sobre `rgba` (RGBA8).
///
/// Si el blob no es un perfil ICC válido devuelve `Err` sin tocar los píxeles.
/// Solo se aplican perfiles RGB: un perfil CMYK (u otro espacio no-RGB) haría que
/// moxcms interprete los 4 bytes RGBA como CMYK → colores basura silenciosos.
pub(crate) fn apply_icc_to_srgb(rgba: &mut [u8], icc: &[u8]) -> Result<(), Error> {
    let profile =
        ColorProfile::new_from_slice(icc).map_err(|e| Error::IccTransform(e.to_string()))?;

    // Solo perfiles RGB (o Gray con TRC) tienen sentido sobre nuestros pixels RGBA.
    // CMYK/otros: moxcms crearía un transform "exitoso" que interpreta RGBA como
    // CMYK → colores basura silenciosos. Se deja la imagen como decodificó.
    if !matches!(profile.color_space, moxcms::DataColorSpace::Rgb) {
        return Err(Error::IccTransform(format!(
            "unsupported ICC color space {:?} (only RGB profiles are applied)",
            profile.color_space
        )));
    }

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
    fn colorspace_no_rgb_devuelve_error() {
        // Un perfil Gray (color_space = DataColorSpace::Gray) no es RGB.
        // La guard debe rechazarlo con Err sin tocar los píxeles.
        // Usamos new_gray_with_gamma (disponible en moxcms 0.8) para construir
        // un perfil con color_space::Gray serializable como ICC blob real.
        let gray_profile = moxcms::ColorProfile::new_gray_with_gamma(2.2);
        let gray_bytes = encode_profile(&gray_profile);
        let mut pixels = vec![1u8, 2, 3, 255];
        let result = apply_icc_to_srgb(&mut pixels, &gray_bytes);
        assert!(
            result.is_err(),
            "perfil no-RGB debe devolver Err; color_space={:?}",
            gray_profile.color_space
        );
        // Los píxeles NO deben haberse modificado.
        assert_eq!(pixels, vec![1, 2, 3, 255], "pixels deben quedar intactos");
    }

    #[test]
    fn srgb_es_noop_exacto() {
        // El perfil sRGB dispara el short-circuit: pixels EXACTAMENTE iguales (no ±2).
        let srgb = ColorProfile::new_srgb();
        let srgb_bytes = encode_profile(&srgb);
        let mut pixels = vec![10u8, 128, 250, 200, 0, 0, 0, 255, 255, 255, 255, 255];
        let orig = pixels.clone();
        apply_icc_to_srgb(&mut pixels, &srgb_bytes).unwrap();
        assert_eq!(pixels, orig, "sRGB debe ser short-circuit byte-exacto");
    }
}
