//! Saha koşulu: Vs30 haritasından (USGS Küresel Vs30 Mozaiği, 2026)
//! okuma. Biçim için bkz. [`crate::raster`]; dönüştürme `seismira-data vs30`.
//!
//! Kurallar (docs/bilim.md "Saha koşulu"):
//! - Hücre kara ise haritadaki değer kullanılır (`Vs30Source::Map`).
//! - Hücre su / buz / buzul ise (ör. kıyıdaki bir şehrin merkez noktası
//!   denize düşmüşse) en fazla [`SEARCH_CELLS`] hücre uzaklıkta en yakın
//!   kara hücresi aranır (`Nearby`).
//! - Hiç bulunamazsa ya da harita yoksa referans zemin kullanılır
//!   (`Reference`); arayüz bunu ayrıca belirtir.
//! - Şehirler için (veri hazırlamada, [`Vs30Map::built_weighted`]):
//!   şehir noktası çevresindeki pencerede Vs30'un yapılaşma yoğunluğuyla
//!   ağırlıklı medyanı (`Built`). ESRM20 "exposure2site" yaklaşımı
//!   (Weatherill vd. 2023, BEE 21, doi:10.1007/s10518-022-01526-5):
//!   saha değeri, insanların yaşadığı hücrelerden alınır; şehir noktası
//!   boş bir yamaca düşse bile yerleşik ova temsil edilir.

use crate::raster::Raster;
use std::io::{self, Read, Write};

/// Harita yokken ya da yakında kara yokken kullanılan zemin. 400 m/s:
/// J-SHIS'in "mühendislik ana kayası" (工学的基盤) tanımı.
pub const REFERENCE_VS30: f64 = 400.0;

/// Kara hücresi arama yarıçapı (hücre). 30″ ızgarada 3 hücre ≈ 2,8 km.
pub const SEARCH_CELLS: i32 = 3;

/// Şehir penceresi yarı genişliği (hücre): (2·2+1)² = 25 hücre, 30″
/// ızgarada K-G ≈ 4,6 km. Tipik bir şehir merkezini kapsayan, komşu
/// yerleşimlere taşmayan bir ölçek (seçim; ESRM20 benzer kaba hücreler
/// için 90″–120″ kullanıyor).
pub const CITY_WINDOW: i32 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Vs30Source {
    Map = 0,
    Nearby = 1,
    Reference = 2,
    /// Yapılaşma ağırlıklı (şehirler).
    Built = 3,
}

impl Vs30Source {
    fn from_u8(v: u8) -> Option<Self> {
        Some(match v {
            0 => Self::Map,
            1 => Self::Nearby,
            2 => Self::Reference,
            3 => Self::Built,
            _ => return None,
        })
    }
}

/// Ağırlıklı medyan: değere göre sıralı birikimli ağırlığın toplamın
/// yarısına ulaştığı ilk eleman. Ağırlık toplamı 0 ise `None`.
/// Döndürülen: o elemanın dizideki (sıralama sonrası) konumu.
pub fn weighted_median<T>(items: &mut [(f64, f64, T)]) -> Option<&(f64, f64, T)> {
    let total: f64 = items.iter().map(|i| i.1.max(0.0)).sum();
    if total.is_nan() || total <= 0.0 {
        return None;
    }
    items.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut acc = 0.0;
    for it in items.iter() {
        acc += it.1.max(0.0);
        if acc >= total * 0.5 {
            return Some(it);
        }
    }
    items.last()
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Site {
    /// Haritadaki Vs30 (m/s), kırpılmamış.
    pub vs30: f64,
    /// Vs30'un log standart sapması (haritanın belirsizlik katmanı).
    pub vs30_ln_sd: Option<f64>,
    pub source: Vs30Source,
}

impl Site {
    pub fn reference() -> Self {
        Self {
            vs30: REFERENCE_VS30,
            vs30_ln_sd: None,
            source: Vs30Source::Reference,
        }
    }
}

pub struct Vs30Map<B> {
    raster: Raster<B>,
    median: usize,
    sd: Option<usize>,
}

impl<B: AsRef<[u8]>> Vs30Map<B> {
    pub fn open(bytes: B) -> io::Result<Self> {
        let raster = Raster::open(bytes)?;
        let median = raster
            .layer("vs30")
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "vs30 katmanı yok"))?;
        let sd = raster.layer("vs30_sd");
        Ok(Self { raster, median, sd })
    }

    /// Tam hücredeki değer; su / buz / ızgara dışı ise `None`.
    pub fn value_at(&self, lon: f64, lat: f64) -> io::Result<Option<(f64, Option<f64>)>> {
        let Some(code) = self.raster.code_at(self.median, lon, lat)? else {
            return Ok(None);
        };
        let Some(v) = self.raster.layers[self.median].value(code) else {
            return Ok(None);
        };
        let sd = match self.sd {
            Some(l) => self
                .raster
                .code_at(l, lon, lat)?
                .and_then(|c| self.raster.layers[l].value(c)),
            None => None,
        };
        Ok(Some((f64::from(v), sd.map(f64::from))))
    }

    /// Saha koşulu (modül belgesindeki kurallarla).
    pub fn site(&self, lon: f64, lat: f64) -> io::Result<Site> {
        if let Some((vs30, sd)) = self.value_at(lon, lat)? {
            return Ok(Site {
                vs30,
                vs30_ln_sd: sd,
                source: Vs30Source::Map,
            });
        }
        let cell = self.raster.grid.cell;
        let kx = 1.0 / lat.to_radians().cos().max(0.05);
        for r in 1..=SEARCH_CELLS {
            // r halkasındaki kara hücreleri arasından (yaklaşık) en yakını.
            let mut best: Option<(f64, f64, Option<f64>)> = None;
            for dy in -r..=r {
                for dx in -r..=r {
                    if dx.abs().max(dy.abs()) != r {
                        continue;
                    }
                    let (dxf, dyf) = (f64::from(dx), f64::from(dy));
                    let Some((v, sd)) = self.value_at(lon + dxf * cell, lat - dyf * cell)? else {
                        continue;
                    };
                    // Boylam farkı km'de cos(enlem) ile küçülür; hücre
                    // sayısı olarak ölçtüğümüz için ters orantı uygulanır.
                    let d2 = (dxf / kx).powi(2) + dyf.powi(2);
                    if best.is_none_or(|b| d2 < b.0) {
                        best = Some((d2, v, sd));
                    }
                }
            }
            if let Some((_, vs30, sd)) = best {
                return Ok(Site {
                    vs30,
                    vs30_ln_sd: sd,
                    source: Vs30Source::Nearby,
                });
            }
        }
        Ok(Site::reference())
    }

    /// Şehir zemini: [`CITY_WINDOW`] penceresindeki kara hücrelerinin
    /// Vs30'unun `weight(boylam, enlem)` (yapılaşma alanı) ile ağırlıklı
    /// medyanı. Pencerede yapılaşma yoksa `None` (çağıran [`Self::site`]'a
    /// döner). Hücre merkezleri bu haritanın ızgarasından alınır.
    pub fn built_weighted(
        &self,
        lon: f64,
        lat: f64,
        mut weight: impl FnMut(f64, f64) -> io::Result<f64>,
    ) -> io::Result<Option<Site>> {
        let g = self.raster.grid;
        let Some((col, row)) = g.cell_of(lon, lat) else {
            return Ok(None);
        };
        let center_lon = g.west + (f64::from(col) + 0.5) * g.cell;
        let center_lat = g.north - (f64::from(row) + 0.5) * g.cell;
        let mut items = Vec::with_capacity(((2 * CITY_WINDOW + 1).pow(2)) as usize);
        for dy in -CITY_WINDOW..=CITY_WINDOW {
            for dx in -CITY_WINDOW..=CITY_WINDOW {
                let (x, y) = (
                    center_lon + f64::from(dx) * g.cell,
                    center_lat - f64::from(dy) * g.cell,
                );
                let Some((vs30, sd)) = self.value_at(x, y)? else {
                    continue;
                };
                let w = weight(x, y)?;
                if w.is_finite() && w > 0.0 {
                    items.push((vs30, w, sd));
                }
            }
        }
        Ok(weighted_median(&mut items).map(|&(vs30, _, sd)| Site {
            vs30,
            vs30_ln_sd: sd,
            source: Vs30Source::Built,
        }))
    }
}

/// Şehir başına zemin dosyası (`.ssit`), şehir listesiyle aynı sıra.
///
/// Biçim (little-endian): `"SSIT" u32 sürüm u32 sayı`, her şehir için
/// `f32 Vs30, f32 ln SD (NaN: yok), u8 kaynak, 3 bayt boşluk`.
const SITE_MAGIC: &[u8; 4] = b"SSIT";
const SITE_VERSION: u32 = 1;

pub fn write_sites(sites: &[Site], w: &mut impl Write) -> io::Result<()> {
    w.write_all(SITE_MAGIC)?;
    w.write_all(&SITE_VERSION.to_le_bytes())?;
    w.write_all(&(sites.len() as u32).to_le_bytes())?;
    for s in sites {
        w.write_all(&(s.vs30 as f32).to_le_bytes())?;
        w.write_all(&(s.vs30_ln_sd.unwrap_or(f64::NAN) as f32).to_le_bytes())?;
        w.write_all(&[s.source as u8, 0, 0, 0])?;
    }
    Ok(())
}

pub fn read_sites(r: &mut impl Read) -> io::Result<Vec<Site>> {
    let bad = |m: &str| io::Error::new(io::ErrorKind::InvalidData, m.to_owned());
    let mut head = [0u8; 12];
    r.read_exact(&mut head)?;
    if &head[..4] != SITE_MAGIC {
        return Err(bad("SSIT dosyası değil"));
    }
    if u32::from_le_bytes(head[4..8].try_into().unwrap_or_default()) != SITE_VERSION {
        return Err(bad("desteklenmeyen SSIT sürümü"));
    }
    let n = u32::from_le_bytes(head[8..12].try_into().unwrap_or_default()) as usize;
    let mut buf = vec![0u8; n * 12];
    r.read_exact(&mut buf)?;
    buf.as_chunks::<12>()
        .0
        .iter()
        .map(|b| {
            let f = |i: usize| f32::from_le_bytes(b[i..i + 4].try_into().unwrap_or_default());
            let sd = f(4);
            Ok(Site {
                vs30: f64::from(f(0)),
                vs30_ln_sd: (!sd.is_nan()).then_some(f64::from(sd)),
                source: Vs30Source::from_u8(b[8]).ok_or_else(|| bad("bilinmeyen kaynak"))?,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::raster::{Encoded, Grid, LayerInfo, Scale, encode, write};

    const OCEAN: u8 = 251;

    /// 8×8 hücrelik, 1°'lik ızgara: sol yarısı kara (Vs30 ≈ 300), sağ
    /// yarısı okyanus; (1,1) hücresi göl.
    fn test_map() -> Vs30Map<Vec<u8>> {
        let grid = Grid {
            west: 0.0,
            north: 8.0,
            cell: 1.0,
            width: 8,
            height: 8,
            tile: 8,
        };
        let lv = LayerInfo {
            name: "vs30".into(),
            scale: Scale::Log,
            n_valid: 251,
            lo: 80.0,
            hi: 2500.0,
        };
        let ls = LayerInfo {
            name: "vs30_sd".into(),
            scale: Scale::Linear,
            n_valid: 251,
            lo: 0.0,
            hi: 1.25,
        };
        let mut m = vec![OCEAN; 64];
        let mut s = vec![OCEAN; 64];
        for y in 0..8 {
            for x in 0..4 {
                m[y * 8 + x] = lv.quantize(300.0);
                s[y * 8 + x] = ls.quantize(0.43);
            }
        }
        m[9] = 254;
        s[9] = 254;
        let z = |t: &[u8]| {
            Ok(ruzstd::encoding::compress_to_vec(
                t,
                ruzstd::encoding::CompressionLevel::Fastest,
            ))
        };
        let tiles: Vec<Vec<Encoded>> = [m, s].iter().map(|t| vec![encode(t, z).unwrap()]).collect();
        let mut buf = Vec::new();
        write(&mut buf, &grid, &[lv, ls], &tiles).unwrap();
        Vs30Map::open(buf).unwrap()
    }

    #[test]
    fn kara_hucresi_dogrudan() {
        let s = test_map().site(2.5, 5.5).unwrap();
        assert_eq!(s.source, Vs30Source::Map);
        assert!((s.vs30 / 300.0 - 1.0).abs() < 0.01);
        assert!((s.vs30_ln_sd.unwrap() - 0.43).abs() < 0.005);
    }

    #[test]
    fn kiyida_en_yakin_kara() {
        let map = test_map();
        // Göl hücresi (1,1): komşu kara.
        assert_eq!(map.site(1.5, 6.5).unwrap().source, Vs30Source::Nearby);
        // Kıyıdan 2 hücre denizde: bulunur.
        assert_eq!(map.site(5.5, 4.5).unwrap().source, Vs30Source::Nearby);
        // Ölçek: 4 hücre uzakta yok → referans.
        let far = map.site(7.5, 4.5).unwrap();
        assert_eq!(far.source, Vs30Source::Reference);
        assert_eq!(far.vs30, REFERENCE_VS30);
    }

    #[test]
    fn agirlikli_medyan() {
        let mut v = vec![(500.0, 1.0, ()), (250.0, 5.0, ()), (300.0, 1.0, ())];
        assert_eq!(weighted_median(&mut v).unwrap().0, 250.0);
        let mut eq = vec![(1.0, 1.0, ()), (2.0, 1.0, ()), (3.0, 1.0, ())];
        assert_eq!(weighted_median(&mut eq).unwrap().0, 2.0);
        let mut none: Vec<(f64, f64, ())> = vec![(1.0, 0.0, ())];
        assert!(weighted_median(&mut none).is_none());
    }

    #[test]
    fn yapilasma_agirlikli_sehir_zemini() {
        let map = test_map();
        // Kara 0–4° boylam (Vs30 300). Yapılaşma yalnızca 3. sütunda
        // (2–3°) → o hücreler seçilir; değer 300 kalır ama kaynak Built.
        let s = map
            .built_weighted(2.5, 4.5, |lon, _| {
                Ok(if (2.0..3.0).contains(&lon) { 10.0 } else { 0.0 })
            })
            .unwrap()
            .unwrap();
        assert_eq!(s.source, Vs30Source::Built);
        assert!((s.vs30 / 300.0 - 1.0).abs() < 0.01);
        // Yapılaşma yok → None.
        assert!(
            map.built_weighted(2.5, 4.5, |_, _| Ok(0.0))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn sehir_zemin_dosyasi_gidis_donus() {
        let sites = vec![
            Site {
                vs30: 250.0,
                vs30_ln_sd: Some(0.43),
                source: Vs30Source::Built,
            },
            Site::reference(),
        ];
        let mut buf = Vec::new();
        write_sites(&sites, &mut buf).unwrap();
        let back = read_sites(&mut buf.as_slice()).unwrap();
        assert_eq!(back.len(), 2);
        assert_eq!(back[1], Site::reference());
        assert_eq!(back[0].source, Vs30Source::Built);
        assert!((back[0].vs30_ln_sd.unwrap() - 0.43).abs() < 1e-6);
    }

    #[test]
    fn izgara_disi_referans() {
        let s = test_map().site(20.0, 4.0).unwrap();
        assert_eq!(s, Site::reference());
    }
}
