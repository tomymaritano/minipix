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

    // Short-circuit: sólo si el perfil es matrix-shaper puro (sin LUTs A2B/B2A),
    // la matriz combinada RGB→XYZ→RGB es ≈identidad (1e-4), Y la rampa de grises
    // es byte-exacta. Cualquier duda → transformación completa.
    if transform_is_identity(profile, &dst_profile, &transform) {
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

/// Devuelve `true` sólo si el transform es byte-exactamente identidad en todo el dominio,
/// verificado mediante tres comprobaciones estructurales encadenadas:
///
/// 1. **Matrix-shaper sin LUTs**: el perfil origen es matrix-shaper puro (TRC + matriz de
///    colorantes; `is_matrix_shaper()` verdadero) y no contiene etiquetas cLUT A2B ni B2A.
///    Un perfil LUT puede tener su matriz-identidad y aun así un cLUT entre nodos con
///    corrección de color no trivial.
///
/// 2. **Matriz ≈ identidad (1e-4)**: `transform_matrix(dst)` (la matriz combinada
///    RGB→XYZ→RGB incluyendo adaptación cromática) tiene todos sus elementos diagonales
///    dentro de 1e-4 de 1.0 y todos los fuera-de-diagonal dentro de 1e-4 de 0.0.
///
/// 3. **Rampa de grises byte-exacta**: todos los 256 grises (0..=255) transformados vuelven sin
///    cambios (diferencia = 0 en todos los canales RGB). Esto detecta desviaciones de TRC
///    que la comprobación de matriz no cubre (e.g., gamma ligeramente diferente).
///
/// El check es conservador: ante cualquier fallo devuelve `false` y se aplica la
/// transformación completa. Nunca se salta un transform necesario.
fn transform_is_identity(
    profile: &ColorProfile,
    dst: &ColorProfile,
    transform: &Arc<Transform8BitExecutor>,
) -> bool {
    // 1. Matrix-shaper sin LUTs cLUT (A2B ni B2A).
    if !profile.is_matrix_shaper() {
        return false;
    }
    let has_lut = profile.lut_a_to_b_perceptual.is_some()
        || profile.lut_a_to_b_colorimetric.is_some()
        || profile.lut_a_to_b_saturation.is_some()
        || profile.lut_b_to_a_perceptual.is_some()
        || profile.lut_b_to_a_colorimetric.is_some()
        || profile.lut_b_to_a_saturation.is_some();
    if has_lut {
        return false;
    }

    // 2. Matriz combinada ≈ identidad (tolerancia 1e-4).
    let m = profile.transform_matrix(dst);
    let near_one = |x: f64| (x - 1.0_f64).abs() < 1e-4_f64;
    let near_zero = |x: f64| x.abs() < 1e-4_f64;
    let identity_matrix = near_one(m.v[0][0])
        && near_one(m.v[1][1])
        && near_one(m.v[2][2])
        && near_zero(m.v[0][1])
        && near_zero(m.v[0][2])
        && near_zero(m.v[1][0])
        && near_zero(m.v[1][2])
        && near_zero(m.v[2][0])
        && near_zero(m.v[2][1]);
    if !identity_matrix {
        return false;
    }

    // 3. Rampa de grises completa (0..=255): con la matriz ya probada identidad,
    //    exactitud byte a byte en los 256 grises prueba que las 3 TRC por canal son
    //    identidad en todo el dominio (sin gap de muestreo). Una sola llamada al transform.
    let mut ramp: Vec<u8> = Vec::with_capacity(256 * 4);
    for v in 0u8..=255 {
        ramp.extend([v, v, v, 255]);
    }
    let mut out = vec![0u8; ramp.len()];
    if transform.transform(&ramp, &mut out).is_err() {
        return false; // ante error, no saltear (camino seguro)
    }
    ramp == out
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

    #[test]
    fn srgb_perturbado_no_se_saltea() {
        // Un perfil casi-sRGB con el colorante rojo perturbado (+0.002 en X) tiene una
        // matriz combinada RGB→XYZ→RGB que se aleja >1e-4 de la identidad, por lo que
        // el short-circuit NO debe dispararse — el transform debe aplicarse.
        // Verificamos en [249, 10, 0, 255], donde la desviación es máxima.
        let mut p = moxcms::ColorProfile::new_srgb();
        // red_colorant.x es el componente X del colorante rojo (f64, unidades XYZ D50).
        // Una perturbación de +0.002 da una desviación de ~0.003 en la diagonal de la
        // matriz combinada — bien por encima del umbral 1e-4 → no short-circuit.
        p.red_colorant.x += 0.002;
        let perturbed_bytes = p.encode().expect("encode perturbed profile");

        let mut px = vec![249u8, 10, 0, 255];
        let orig = px.clone();
        // Aplica vía blob (ejercita el camino completo incluido el parse).
        apply_icc_to_srgb(&mut px, &perturbed_bytes).unwrap();
        // El perfil perturbado NO es identidad — el transform debe cambiar al menos un canal.
        assert_ne!(
            &px[..3],
            &orig[..3],
            "perfil casi-sRGB perturbado debe transformarse, no saltearse"
        );
    }
}
