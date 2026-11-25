//! Çıktı dışa aktarma (README "Çıktı dışa aktarma"): QGIS / ArcGIS için
//! GeoTIFF (sarsıntı ızgarası), CSV (şehirler, bölgeler, envanter) ve
//! GeoJSON (fay izi, zemin yenilmesi kareleri).
//!
//! GeoTIFF elle yazılır (bağımlılık yok): tek bant, 32 bit kayan nokta,
//! sıkıştırmasız tek şerit, coğrafi koordinat (EPSG:4326, PixelIsArea).
//! Etiketler: TIFF 6.0 + GeoTIFF 1.0 (ModelPixelScale 33550, ModelTiepoint
//! 33922, GeoKeyDirectory 34735: GTModelType 1024 = 2 coğrafi,
//! GTRasterType 1025 = 1 alan, GeographicType 2048 = 4326, GeogAngularUnits
//! 2054 = 9102 derece) ve GDAL_NODATA 42113.

use std::io::Write;
use std::path::Path;

/// Tek bantlı f32 GeoTIFF yazar. `data` kuzeyden güneye satırlar
/// (`width × height`); NaN değerler `nodata` ile yazılır (verilmişse).
#[allow(clippy::too_many_arguments)]
pub fn write_geotiff(
    path: &Path,
    west: f64,
    north: f64,
    cell: f64,
    width: u32,
    height: u32,
    data: &[f32],
    nodata: Option<f32>,
) -> std::io::Result<()> {
    assert_eq!(data.len(), width as usize * height as usize);
    const SHORT: u16 = 3;
    const LONG: u16 = 4;
    const ASCII: u16 = 2;
    const DOUBLE: u16 = 12;
    // (etiket, tür, sayı, satır içi değer ya da dış veri)
    enum V {
        Inline(u32),
        Bytes(Vec<u8>),
    }
    let shorts = |v: &[u16]| V::Bytes(v.iter().flat_map(|x| x.to_le_bytes()).collect());
    let doubles = |v: &[f64]| V::Bytes(v.iter().flat_map(|x| x.to_le_bytes()).collect());
    let geokeys: [u16; 20] = [
        1, 1, 0, 4, //
        1024, 0, 1, 2, //
        1025, 0, 1, 1, //
        2048, 0, 1, 4326, //
        2054, 0, 1, 9102,
    ];
    let strip_bytes = data.len() as u32 * 4;
    let mut entries: Vec<(u16, u16, u32, V)> = vec![
        (256, LONG, 1, V::Inline(width)),
        (257, LONG, 1, V::Inline(height)),
        (258, SHORT, 1, V::Inline(32)),
        (259, SHORT, 1, V::Inline(1)),
        (262, SHORT, 1, V::Inline(1)),
        (273, LONG, 1, V::Inline(0)), // veri konumu sonra
        (277, SHORT, 1, V::Inline(1)),
        (278, LONG, 1, V::Inline(height)),
        (279, LONG, 1, V::Inline(strip_bytes)),
        (284, SHORT, 1, V::Inline(1)),
        (339, SHORT, 1, V::Inline(3)),
        (33550, DOUBLE, 3, doubles(&[cell, cell, 0.0])),
        (
            33922,
            DOUBLE,
            6,
            doubles(&[0.0, 0.0, 0.0, west, north, 0.0]),
        ),
        (34735, SHORT, geokeys.len() as u32, shorts(&geokeys)),
    ];
    if let Some(n) = nodata {
        let mut s = format!("{n}").into_bytes();
        s.push(0);
        entries.push((42113, ASCII, s.len() as u32, V::Bytes(s)));
    }
    // Yerleşim: başlık (8) + IFD + dış veriler + görüntü.
    let ifd_len = 2 + entries.len() * 12 + 4;
    let mut extra = Vec::new();
    let mut offsets = Vec::new();
    let mut pos = 8 + ifd_len;
    for (_, _, _, v) in &entries {
        if let V::Bytes(b) = v
            && b.len() > 4
        {
            offsets.push(Some(pos as u32));
            extra.extend_from_slice(b);
            pos += b.len();
            if pos % 2 == 1 {
                extra.push(0);
                pos += 1;
            }
            continue;
        }
        offsets.push(None);
    }
    let data_offset = pos as u32;
    let mut out = Vec::with_capacity(pos + strip_bytes as usize);
    out.extend_from_slice(b"II");
    out.extend_from_slice(&42u16.to_le_bytes());
    out.extend_from_slice(&8u32.to_le_bytes());
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    for ((tag, typ, count, v), off) in entries.iter().zip(&offsets) {
        out.extend_from_slice(&tag.to_le_bytes());
        out.extend_from_slice(&typ.to_le_bytes());
        out.extend_from_slice(&count.to_le_bytes());
        let mut field = [0u8; 4];
        match (v, off) {
            (_, Some(o)) => field = o.to_le_bytes(),
            (V::Inline(x), None) => {
                let x = if *tag == 273 { data_offset } else { *x };
                if *typ == SHORT {
                    field[..2].copy_from_slice(&(x as u16).to_le_bytes());
                } else {
                    field = x.to_le_bytes();
                }
            }
            (V::Bytes(b), None) => field[..b.len()].copy_from_slice(b),
        }
        out.extend_from_slice(&field);
    }
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&extra);
    debug_assert_eq!(out.len(), data_offset as usize);
    for &v in data {
        let v = match nodata {
            Some(n) if v.is_nan() => n,
            _ => v,
        };
        out.extend_from_slice(&v.to_le_bytes());
    }
    std::fs::File::create(path)?.write_all(&out)
}

/// CSV alanı: virgül, tırnak ya da satır sonu içeriyorsa tırnaklanır.
pub fn csv_field(s: &str) -> String {
    if s.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `tiff` crate'iyle geri okunur; konum etiketleri doğru.
    #[test]
    fn geotiff_geri_okunur() {
        use tiff::decoder::{Decoder, DecodingResult};
        use tiff::tags::Tag;
        let dir = std::env::temp_dir().join(format!("seismira-exp-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("t.tif");
        let data = [1.0f32, 2.5, f32::NAN, -4.0];
        write_geotiff(&path, 26.0, 42.0, 0.25, 2, 2, &data, Some(-9999.0)).unwrap();
        let mut d = Decoder::new(std::fs::File::open(&path).unwrap()).unwrap();
        assert_eq!(d.dimensions().unwrap(), (2, 2));
        assert_eq!(
            d.get_tag_f64_vec(Tag::ModelPixelScaleTag).unwrap()[..2],
            [0.25, 0.25]
        );
        let tie = d.get_tag_f64_vec(Tag::ModelTiepointTag).unwrap();
        assert_eq!((tie[3], tie[4]), (26.0, 42.0));
        let keys = d.get_tag_u16_vec(Tag::GeoKeyDirectoryTag).unwrap();
        assert!(keys.chunks(4).any(|k| k == [2048, 0, 1, 4326]));
        assert_eq!(
            d.get_tag_ascii_string(Tag::GdalNodata)
                .unwrap()
                .trim_matches('\0'),
            "-9999"
        );
        let DecodingResult::F32(v) = d.read_image().unwrap() else {
            panic!("f32 değil");
        };
        assert_eq!(v, vec![1.0, 2.5, -9999.0, -4.0]);
        // GDAL ile dış denetim için: SEISMIRA_KEEP_TIF=<yol> verilirse kopyalanır.
        if let Some(keep) = std::env::var_os("SEISMIRA_KEEP_TIF") {
            std::fs::copy(&path, keep).unwrap();
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn csv_tirnak() {
        assert_eq!(csv_field("Adana"), "Adana");
        assert_eq!(csv_field("Kafr Lāhā, Hama"), "\"Kafr Lāhā, Hama\"");
        assert_eq!(csv_field("a\"b"), "\"a\"\"b\"");
    }
}
