use crate::format::Format;

/// Detecta el formato por magic bytes. `None` si no se reconoce.
///
/// AVIF: solo se inspecciona el major brand (`avif`/`avis`, incluye secuencias);
/// archivos con major brand `mif1`/`msf1` no se detectan en v1 (backlog).
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
    // ISO-BMFF: bytes 4..8 = "ftyp" y major/compatible brand avif/avis.
    if data.len() >= 12
        && &data[4..8] == b"ftyp"
        && (&data[8..12] == b"avif" || &data[8..12] == b"avis")
    {
        return Some(Format::Avif);
    }
    None
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
}
