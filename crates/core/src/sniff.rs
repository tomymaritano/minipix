use crate::format::Format;

/// Detecta el formato por magic bytes. `None` si no se reconoce.
///
/// AVIF: se detecta si `avif` o `avis` aparece como major brand **o** en la lista
/// `compatible_brands` del ftyp box (ISO-BMFF, ISO 23000-22). Archivos HEIC/MP4 sin
/// ninguna brand `avif`/`avis` retornan `None`.
#[must_use]
pub fn sniff(data: &[u8]) -> Option<Format> {
    if data.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        return Some(Format::Png);
    }
    if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some(Format::Jpeg);
    }
    if data.len() >= 12 && &data[0..4] == b"RIFF" && &data[8..12] == b"WEBP" {
        return Some(Format::WebP);
    }
    // ISO-BMFF: "ftyp" en 4..8; avif/avis como major brand O en compatible_brands.
    if data.len() >= 16 && &data[4..8] == b"ftyp" && is_avif_ftyp(data) {
        return Some(Format::Avif);
    }
    None
}

/// Detecta `avif`/`avis` como major brand o en la lista `compatible_brands` del ftyp.
/// Robusto ante box size mentido/cero: nunca lee fuera de `data`.
fn is_avif_ftyp(data: &[u8]) -> bool {
    let is_avif_brand = |b: &[u8]| b == b"avif" || b == b"avis";
    // Major brand 8..12
    if is_avif_brand(&data[8..12]) {
        return true;
    }
    // box size en 0..4 (big-endian). 0 = hasta fin de archivo; clamp a data.len().
    let declared = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;
    let end = if declared == 0 || declared > data.len() {
        data.len()
    } else {
        declared
    };
    // compatible_brands empiezan en offset 16, de a 4 bytes.
    let mut i = 16;
    while i + 4 <= end {
        if is_avif_brand(&data[i..i + 4]) {
            return true;
        }
        i += 4;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::sniff;
    use crate::format::Format;

    #[test]
    fn detecta_png() {
        let mut data = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        data.extend([0u8; 16]);
        assert_eq!(sniff(&data), Some(Format::Png));
    }
    #[test]
    fn detecta_jpeg() {
        let mut data = vec![0xFF, 0xD8, 0xFF, 0xE0];
        data.extend([0u8; 16]);
        assert_eq!(sniff(&data), Some(Format::Jpeg));
    }
    #[test]
    fn detecta_webp() {
        let mut data = b"RIFF\x00\x00\x00\x00WEBP".to_vec();
        data.extend([0u8; 16]);
        assert_eq!(sniff(&data), Some(Format::WebP));
    }
    #[test]
    fn detecta_avif() {
        // caja ftyp ISO-BMFF: [size:4]["ftyp"][brand:4]
        let mut data = vec![0x00, 0x00, 0x00, 0x1C];
        data.extend(b"ftypavif");
        data.extend([0u8; 20]);
        assert_eq!(sniff(&data), Some(Format::Avif));
    }
    #[test]
    fn rechaza_basura_y_vacio() {
        assert_eq!(sniff(b"hola mundo!!"), None);
        assert_eq!(sniff(&[]), None);
    }
    #[test]
    fn riff_truncado_de_11_bytes_es_none() {
        assert_eq!(sniff(b"RIFF\x00\x00\x00\x00WEB"), None);
    }
    #[test]
    fn ftyp_con_brand_no_avif_es_none() {
        let mut data = vec![0x00, 0x00, 0x00, 0x1C];
        data.extend(b"ftypmif1");
        data.extend([0u8; 20]);
        assert_eq!(sniff(&data), None);
    }

    #[test]
    fn detecta_avif_via_compatible_brand() {
        // major brand mif1, avif en compatible_brands — debe detectarse.
        let mut data = vec![0x00, 0x00, 0x00, 0x1C]; // box size 28
        data.extend(b"ftyp"); // 4..8
        data.extend(b"mif1"); // major brand 8..12
        data.extend(b"\x00\x00\x00\x00"); // minor version 12..16
        data.extend(b"avif"); // compatible brand 16..20
        data.extend(b"mif1"); // compatible brand 20..24
        data.extend([0u8; 4]); // padding hasta 28
        assert_eq!(sniff(&data), Some(Format::Avif));
    }

    #[test]
    fn detecta_avis_via_compatible_brand() {
        let mut data = vec![0x00, 0x00, 0x00, 0x18];
        data.extend(b"ftypmsf1"); // major msf1
        data.extend(b"\x00\x00\x00\x00");
        data.extend(b"avis"); // compatible brand de secuencia
        assert_eq!(sniff(&data), Some(Format::Avif));
    }

    #[test]
    fn heic_sin_avif_brand_es_none() {
        // HEIC: major heic, compatible mif1/heic — NINGÚN avif → None (no rutear HEIC a AVIF).
        let mut data = vec![0x00, 0x00, 0x00, 0x18];
        data.extend(b"ftypheic");
        data.extend(b"\x00\x00\x00\x00");
        data.extend(b"mif1");
        assert_eq!(sniff(&data), None);
    }

    #[test]
    fn box_size_mentido_no_panica() {
        // box size enorme pero data corta: no debe leer fuera de límites.
        let mut data = vec![0xFF, 0xFF, 0xFF, 0xFF]; // size gigante
        data.extend(b"ftypmif1");
        data.extend(b"\x00\x00\x00\x00");
        data.extend(b"avif");
        assert_eq!(sniff(&data), Some(Format::Avif)); // encuentra avif sin leer de más
    }

    #[test]
    fn box_size_cero_escanea_hasta_fin_de_data() {
        // box size 0 = "hasta el fin del archivo" (ISO-BMFF).
        let mut data = vec![0x00, 0x00, 0x00, 0x00];
        data.extend(b"ftypmif1");
        data.extend(b"\x00\x00\x00\x00");
        data.extend(b"avif");
        assert_eq!(sniff(&data), Some(Format::Avif));
    }
}
