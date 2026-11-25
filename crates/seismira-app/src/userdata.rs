//! Kullanıcının kendi verisi (README "Kendi verini yükle"): fay izi
//! (GeoJSON), Vs30 zemin haritası (GeoTIFF), bina envanteri (CSV).
//!
//! Okuyucular arayüzden bağımsızdır; büyük dosyalar arka planda okunur
//! (Bölüm 11). Hata metinleri kullanıcıya gösterilir (çeviri anahtarı +
//! ayrıntı).
//!
//! Kurallar (Bölüm 6: sessiz ekstrapolasyon yok):
//! - Vs30: yalnızca coğrafi koordinatlı (boylam/enlem, ör. EPSG:4326) tek
//!   bantlı GeoTIFF; değerler 100–3000 m/s dışında olan hücreler veri yok
//!   sayılır. Dosyanın kapsadığı yerde uygulamanın Vs30'u yerine kullanılır.
//! - Fay izi: LineString / MultiLineString (en uzun çizgi) / Feature /
//!   FeatureCollection; 32 noktadan fazlaysa en az şekil kaybıyla 32'ye
//!   indirgenir (Visvalingam).
//! - Envanter: CSV, başlıklı; boylam, enlem, bina sayısı sütunları.

use std::path::Path;

use seismira_core::geo;

/// Okuma hatası: çeviri anahtarı ve ayrıntı (dosya, satır vb.).
#[derive(Clone, Debug, PartialEq)]
pub struct LoadError {
    pub key: &'static str,
    pub detail: String,
}

fn err(key: &'static str, detail: impl Into<String>) -> LoadError {
    LoadError {
        key,
        detail: detail.into(),
    }
}

/// Dosya türü (uzantıdan).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Fault,
    Vs30,
    Inventory,
}

impl Kind {
    pub fn of(path: &Path) -> Option<Self> {
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        Some(match ext.as_str() {
            "geojson" | "json" => Self::Fault,
            "tif" | "tiff" => Self::Vs30,
            "csv" | "txt" => Self::Inventory,
            _ => return None,
        })
    }
}

/// Yüklenmiş dosya.
pub enum Loaded {
    Fault(Vec<(f64, f64)>),
    Vs30(UserVs30),
    Inventory(Inventory),
}

/// Dosyayı uzantısına göre okur (arka plan iş parçacığında çağrılır).
pub fn load(path: &Path) -> Result<Loaded, LoadError> {
    match Kind::of(path) {
        Some(Kind::Fault) => load_fault(path).map(Loaded::Fault),
        Some(Kind::Vs30) => load_vs30(path).map(Loaded::Vs30),
        Some(Kind::Inventory) => load_inventory(path).map(Loaded::Inventory),
        None => Err(err("user-err-ext", file_name(path))),
    }
}

// ---------------------------------------------------------------- fay izi

/// GeoJSON'dan fay izi noktaları (boylam, enlem), en çok
/// [`seismira_core::quake::MAX_TRACE_POINTS`].
pub fn load_fault(path: &Path) -> Result<Vec<(f64, f64)>, LoadError> {
    let text = std::fs::read_to_string(path).map_err(|e| err("user-err-read", e.to_string()))?;
    parse_fault(&text)
}

pub fn parse_fault(text: &str) -> Result<Vec<(f64, f64)>, LoadError> {
    let v: serde_json::Value =
        serde_json::from_str(text).map_err(|e| err("user-err-json", e.to_string()))?;
    let mut lines = Vec::new();
    collect_lines(&v, &mut lines);
    let longest = lines
        .into_iter()
        .max_by(|a, b| line_km(a).total_cmp(&line_km(b)))
        .ok_or_else(|| err("user-err-no-line", ""))?;
    // Önce koordinat denetimi (metre koordinatında mesafe anlamsız).
    if longest
        .iter()
        .any(|p| !(-180.0..=360.0).contains(&p.0) || !(-90.0..=90.0).contains(&p.1))
    {
        return Err(err("user-err-not-lonlat", ""));
    }
    // Ardışık tekrar noktalarını at.
    let mut pts: Vec<(f64, f64)> = Vec::with_capacity(longest.len());
    for p in longest {
        if pts.last().is_none_or(|&q| geo::distance_km(q, p) > 1e-3) {
            pts.push(p);
        }
    }
    if pts.len() < 2 {
        return Err(err("user-err-no-line", ""));
    }
    Ok(simplify(pts, seismira_core::quake::MAX_TRACE_POINTS))
}

fn collect_lines(v: &serde_json::Value, out: &mut Vec<Vec<(f64, f64)>>) {
    let line = |c: &serde_json::Value| -> Option<Vec<(f64, f64)>> {
        c.as_array()?
            .iter()
            .map(|p| Some((p.get(0)?.as_f64()?, p.get(1)?.as_f64()?)))
            .collect()
    };
    match v["type"].as_str() {
        Some("FeatureCollection") => {
            for f in v["features"].as_array().into_iter().flatten() {
                collect_lines(f, out);
            }
        }
        Some("Feature") => collect_lines(&v["geometry"], out),
        Some("GeometryCollection") => {
            for g in v["geometries"].as_array().into_iter().flatten() {
                collect_lines(g, out);
            }
        }
        Some("LineString") => out.extend(line(&v["coordinates"])),
        Some("MultiLineString") => {
            for c in v["coordinates"].as_array().into_iter().flatten() {
                out.extend(line(c));
            }
        }
        _ => {}
    }
}

fn line_km(l: &[(f64, f64)]) -> f64 {
    l.windows(2).map(|w| geo::distance_km(w[0], w[1])).sum()
}

/// Visvalingam–Whyatt: en küçük üçgen alanlı iç noktayı atarak `max`
/// noktaya indirger (uçlar korunur). Alan yerel düzlemde (km²).
fn simplify(mut pts: Vec<(f64, f64)>, max: usize) -> Vec<(f64, f64)> {
    while pts.len() > max.max(2) {
        let area = |i: usize| {
            let (a, b, c) = (pts[i - 1], pts[i], pts[i + 1]);
            let k = a.1.to_radians().cos() * 111.195;
            let (bx, by) = ((b.0 - a.0) * k, (b.1 - a.1) * 111.195);
            let (cx, cy) = ((c.0 - a.0) * k, (c.1 - a.1) * 111.195);
            (bx * cy - by * cx).abs() / 2.0
        };
        let i = (1..pts.len() - 1)
            .min_by(|&i, &j| area(i).total_cmp(&area(j)))
            .expect("en az 3 nokta");
        pts.remove(i);
    }
    pts
}

// ---------------------------------------------------------------- Vs30

/// Kullanıcının Vs30 haritası (bellekte, satır satır kuzeyden güneye).
#[derive(Clone, Debug)]
pub struct UserVs30 {
    pub name: String,
    pub west: f64,
    pub north: f64,
    /// Hücre boyutu (derece): boylam, enlem.
    pub cell: (f64, f64),
    pub width: usize,
    pub height: usize,
    /// m/s; veri yoksa NaN.
    pub data: Vec<f32>,
    /// Geçerli hücre sayısı.
    pub valid: usize,
}

/// Geçerli Vs30 aralığı (m/s): dışı veri yok sayılır (ör. nodata −9999, 0).
pub const VS30_RANGE: (f32, f32) = (100.0, 3000.0);
/// Belleğe alınacak en çok hücre (~400 MB f32).
pub const MAX_CELLS: usize = 100_000_000;

impl UserVs30 {
    /// Noktadaki değer (en yakın hücre); kapsam dışı ya da veri yoksa `None`.
    pub fn sample(&self, lon: f64, lat: f64) -> Option<f64> {
        let mut dx = lon - self.west;
        dx -= 360.0 * (dx / 360.0).floor();
        let c = (dx / self.cell.0).floor();
        let r = ((self.north - lat) / self.cell.1).floor();
        if c < 0.0 || r < 0.0 || c >= self.width as f64 || r >= self.height as f64 {
            return None;
        }
        let v = self.data[r as usize * self.width + c as usize];
        v.is_finite().then_some(f64::from(v))
    }

    /// Kapsam (batı, güney, doğu, kuzey).
    pub fn bounds(&self) -> [f64; 4] {
        [
            self.west,
            self.north - self.cell.1 * self.height as f64,
            self.west + self.cell.0 * self.width as f64,
            self.north,
        ]
    }
}

pub fn load_vs30(path: &Path) -> Result<UserVs30, LoadError> {
    use tiff::decoder::{Decoder, DecodingResult, Limits};
    use tiff::tags::Tag;
    let f = std::fs::File::open(path).map_err(|e| err("user-err-read", e.to_string()))?;
    let mut limits = Limits::default();
    limits.ifd_value_size = 64 << 20;
    limits.decoding_buffer_size = MAX_CELLS * 8;
    limits.intermediate_buffer_size = MAX_CELLS * 8;
    let tiff_err = |e: tiff::TiffError| err("user-err-tiff", e.to_string());
    let mut dec = Decoder::new(std::io::BufReader::new(f))
        .map_err(tiff_err)?
        .with_limits(limits);
    let (w, h) = dec.dimensions().map_err(tiff_err)?;
    let (w, h) = (w as usize, h as usize);
    if w.saturating_mul(h) > MAX_CELLS {
        return Err(err("user-err-too-big", format!("{w} × {h}")));
    }
    let scale = dec
        .get_tag_f64_vec(Tag::ModelPixelScaleTag)
        .map_err(|_| err("user-err-no-geo", ""))?;
    let tie = dec
        .get_tag_f64_vec(Tag::ModelTiepointTag)
        .map_err(|_| err("user-err-no-geo", ""))?;
    if scale.len() < 2 || tie.len() < 6 || tie[0] != 0.0 || tie[1] != 0.0 {
        return Err(err("user-err-no-geo", ""));
    }
    let keys = dec
        .get_tag_u16_vec(Tag::GeoKeyDirectoryTag)
        .unwrap_or_default();
    let key = |id: u16| {
        keys.as_chunks::<4>()
            .0
            .iter()
            .skip(1)
            .find(|k| k[0] == id && k[1] == 0)
            .map(|k| k[3])
    };
    // GTModelTypeGeoKey (1024): 2 = coğrafi. Yoksa kapsam derece aralığında
    // olmalı (projeksiyonlu metre koordinatları reddedilir).
    let (cx, cy) = (scale[0], scale[1]);
    let half = if key(1025) == Some(2) { 0.5 } else { 0.0 }; // PixelIsPoint
    let west = tie[3] - half * cx;
    let north = tie[4] + half * cy;
    let east = west + cx * w as f64;
    let south = north - cy * h as f64;
    let degrees = (-180.0..=360.0).contains(&west)
        && (-180.0..=360.0).contains(&east)
        && (-90.0..=90.0).contains(&north)
        && (-90.0..=90.0).contains(&south);
    if key(1024).is_some_and(|m| m != 2) || !degrees || cx <= 0.0 || cy <= 0.0 {
        return Err(err("user-err-projected", ""));
    }
    let nodata = dec
        .get_tag_ascii_string(Tag::GdalNodata)
        .ok()
        .and_then(|s| s.trim_matches(char::from(0)).trim().parse::<f64>().ok());
    let img = dec.read_image().map_err(tiff_err)?;
    let to_f64: Vec<f64> = match img {
        DecodingResult::U8(v) => v.into_iter().map(f64::from).collect(),
        DecodingResult::U16(v) => v.into_iter().map(f64::from).collect(),
        DecodingResult::U32(v) => v.into_iter().map(f64::from).collect(),
        DecodingResult::I8(v) => v.into_iter().map(f64::from).collect(),
        DecodingResult::I16(v) => v.into_iter().map(f64::from).collect(),
        DecodingResult::I32(v) => v.into_iter().map(f64::from).collect(),
        DecodingResult::F32(v) => v.into_iter().map(f64::from).collect(),
        DecodingResult::F64(v) => v,
        _ => return Err(err("user-err-tiff", "desteklenmeyen piksel türü")),
    };
    if to_f64.len() != w * h {
        // Çok bantlı (ör. RGB) dosya.
        return Err(err(
            "user-err-bands",
            format!("{}", to_f64.len() / (w * h).max(1)),
        ));
    }
    let (lo, hi) = VS30_RANGE;
    let mut valid = 0;
    let data: Vec<f32> = to_f64
        .into_iter()
        .map(|x| {
            let v = x as f32;
            if nodata.is_some_and(|n| x == n) || !(lo..=hi).contains(&v) {
                f32::NAN
            } else {
                valid += 1;
                v
            }
        })
        .collect();
    if valid == 0 {
        return Err(err("user-err-no-vs30", ""));
    }
    Ok(UserVs30 {
        name: file_name(path),
        west,
        north,
        cell: (cx, cy),
        width: w,
        height: h,
        data,
        valid,
    })
}

// ---------------------------------------------------------------- envanter

/// Bina envanteri: noktalar (boylam, enlem, bina sayısı, isteğe bağlı ad).
#[derive(Clone, Debug, Default)]
pub struct Inventory {
    pub name: String,
    pub points: Vec<InventoryPoint>,
    /// Okunamayıp atlanan satır sayısı.
    pub skipped: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InventoryPoint {
    pub lon: f64,
    pub lat: f64,
    pub buildings: f64,
    pub label: String,
}

impl Inventory {
    pub fn total(&self) -> f64 {
        self.points.iter().map(|p| p.buildings).sum()
    }
}

pub fn load_inventory(path: &Path) -> Result<Inventory, LoadError> {
    let text = std::fs::read_to_string(path).map_err(|e| err("user-err-read", e.to_string()))?;
    let mut inv = parse_inventory(&text)?;
    inv.name = file_name(path);
    Ok(inv)
}

/// CSV: ilk satır başlık. Ayraç virgül ya da noktalı virgül (noktalı
/// virgülde ondalık virgül kabul edilir). Sütun adları (büyük/küçük harf
/// duyarsız): boylam = lon / lng / longitude / boylam / x; enlem = lat /
/// latitude / enlem / y; sayı = buildings / count / n / bina / bina_sayisi;
/// ad (isteğe bağlı) = name / ad / label.
pub fn parse_inventory(text: &str) -> Result<Inventory, LoadError> {
    let mut lines = text.lines().filter(|l| !l.trim().is_empty());
    let header = lines.next().ok_or_else(|| err("user-err-csv-empty", ""))?;
    let header = header.trim_start_matches('\u{feff}');
    let sep = if header.matches(';').count() > header.matches(',').count() {
        ';'
    } else {
        ','
    };
    let cols: Vec<String> = header
        .split(sep)
        .map(|c| c.trim().trim_matches('"').to_lowercase())
        .collect();
    let find = |names: &[&str]| cols.iter().position(|c| names.contains(&c.as_str()));
    let (Some(ix), Some(iy), Some(ic)) = (
        find(&["lon", "lng", "long", "longitude", "boylam", "x"]),
        find(&["lat", "latitude", "enlem", "y"]),
        find(&[
            "buildings",
            "count",
            "n",
            "bina",
            "bina_sayisi",
            "bina_sayısı",
            "number",
        ]),
    ) else {
        return Err(err("user-err-csv-columns", cols.join(", ")));
    };
    let il = find(&["name", "ad", "label", "isim"]);
    let num = |s: &str| -> Option<f64> {
        let s = s.trim().trim_matches('"');
        let s = if sep == ';' {
            s.replace(',', ".")
        } else {
            s.to_string()
        };
        s.parse::<f64>().ok().filter(|v| v.is_finite())
    };
    let mut inv = Inventory::default();
    for line in lines {
        let f: Vec<&str> = line.split(sep).collect();
        let get = |i: usize| f.get(i).copied();
        match (
            get(ix).and_then(num),
            get(iy).and_then(num),
            get(ic).and_then(num),
        ) {
            (Some(lon), Some(lat), Some(n))
                if (-180.0..=360.0).contains(&lon) && (-90.0..=90.0).contains(&lat) && n > 0.0 =>
            {
                inv.points.push(InventoryPoint {
                    lon,
                    lat,
                    buildings: n,
                    label: il
                        .and_then(get)
                        .map(|s| s.trim().trim_matches('"').to_string())
                        .unwrap_or_default(),
                });
            }
            _ => inv.skipped += 1,
        }
    }
    if inv.points.is_empty() {
        return Err(err(
            "user-err-csv-empty",
            format!("{} satır atlandı", inv.skipped),
        ));
    }
    Ok(inv)
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geojson_fay_izi() {
        let fc = r#"{"type":"FeatureCollection","features":[
          {"type":"Feature","geometry":{"type":"LineString","coordinates":[[36.0,36.5],[36.5,37.0]]}},
          {"type":"Feature","geometry":{"type":"MultiLineString","coordinates":[
             [[36.2,36.4],[36.6,36.9],[37.2,37.4],[38.0,37.9]]]}}]}"#;
        let p = parse_fault(fc).unwrap();
        // En uzun çizgi seçilir.
        assert_eq!(p.len(), 4);
        assert_eq!(p[3], (38.0, 37.9));
        assert_eq!(
            parse_fault(r#"{"type":"Point","coordinates":[1,2]}"#)
                .unwrap_err()
                .key,
            "user-err-no-line"
        );
        assert_eq!(parse_fault("{").unwrap_err().key, "user-err-json");
        // Metre koordinatlı (projeksiyonlu) dosya.
        let utm = r#"{"type":"LineString","coordinates":[[500000,4100000],[510000,4110000]]}"#;
        assert_eq!(parse_fault(utm).unwrap_err().key, "user-err-not-lonlat");
    }

    #[test]
    fn cok_noktali_iz_indirgenir() {
        // 100 noktalı hafif kıvrımlı çizgi → 32 nokta, uçlar korunur.
        let pts: Vec<(f64, f64)> = (0..100)
            .map(|i| {
                let x = f64::from(i) * 0.02;
                (30.0 + x, 40.0 + 0.05 * (x * 3.0).sin())
            })
            .collect();
        let s = simplify(pts.clone(), 32);
        assert_eq!(s.len(), 32);
        assert_eq!(s[0], pts[0]);
        assert_eq!(s[31], pts[99]);
    }

    #[test]
    fn envanter_csv() {
        let csv =
            "lon,lat,buildings,name\n36.2,36.2,120,Okul\n36.3,36.3,x,Bozuk\n\n36.4,36.4,80,\n";
        let inv = parse_inventory(csv).unwrap();
        assert_eq!(inv.points.len(), 2);
        assert_eq!(inv.skipped, 1);
        assert_eq!(inv.total(), 200.0);
        assert_eq!(inv.points[0].label, "Okul");
        // Noktalı virgül + ondalık virgül + Türkçe başlık.
        let tr = "Boylam;Enlem;Bina\n36,25;36,75;12,5\n";
        let inv = parse_inventory(tr).unwrap();
        assert_eq!(inv.points[0].lon, 36.25);
        assert_eq!(inv.points[0].buildings, 12.5);
        assert_eq!(
            parse_inventory("a,b,c\n1,2,3").unwrap_err().key,
            "user-err-csv-columns"
        );
    }

    /// Dış GeoTIFF denetimi (GDAL/QGIS çıktısı; LZW, DEFLATE, döşemeli):
    /// `SEISMIRA_TEST_VS30=<yol>` verilirse okunur ve özeti yazılır.
    #[test]
    fn dis_geotiff_ortam_degiskeniyle() {
        let Some(p) = std::env::var_os("SEISMIRA_TEST_VS30") else {
            return;
        };
        let v = load_vs30(Path::new(&p)).unwrap();
        println!(
            "OKUNDU {} × {}, geçerli {}, sınırlar {:?}, Antakya {:?}",
            v.width,
            v.height,
            v.valid,
            v.bounds(),
            v.sample(36.16, 36.20)
        );
    }

    /// Elle yazılmış küçük GeoTIFF'in okunması ([`crate::export`] yazıcısıyla).
    #[test]
    fn geotiff_vs30_okunur() {
        let dir = std::env::temp_dir().join(format!("seismira-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("vs30.tif");
        // 3 × 2, 0,5° hücre, batı 30, kuzey 41; bir hücre nodata, biri aralık dışı.
        let data = [400.0f32, 760.0, -9999.0, 250.0, 50.0, 1200.0];
        crate::export::write_geotiff(&path, 30.0, 41.0, 0.5, 3, 2, &data, Some(-9999.0)).unwrap();
        let v = load_vs30(&path).unwrap();
        assert_eq!((v.width, v.height), (3, 2));
        assert_eq!(v.valid, 4);
        assert_eq!(v.sample(30.25, 40.75), Some(400.0));
        assert_eq!(v.sample(31.25, 40.75), None); // nodata
        assert_eq!(v.sample(30.75, 40.25), None); // 50 m/s aralık dışı
        assert_eq!(v.sample(31.25, 40.25), Some(1200.0));
        assert_eq!(v.sample(29.0, 40.5), None); // kapsam dışı
        let _ = std::fs::remove_dir_all(&dir);
    }
}
