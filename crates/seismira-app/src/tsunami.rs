//! Tsunami hesabı (Faz 8; docs/bilim.md 7.10): veri yükleme, derinlik
//! pencereleri ve arka plan işi. Bilim seismira-core `tsunami`'de.
//!
//! - Bölgesel pencere (1′, merkez üssü çevresinde): yakın alan tırmanması ve
//!   bölgesel varış süreleri.
//! - Mw ≥ 7,9 (PTWC "okyanus ölçeği"): ek olarak küresel 4′ ızgarada varış
//!   süreleri (bölgesel pencere dışındaki kıyı noktaları için).

use crate::compute::MappedRaster;
use seismira_core::cities::City;
use seismira_core::quake::Quake;
use seismira_core::tsunami::{bathy, coast, forecast, trigger};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

/// İlk uyarının (ve kıyı vurgusunun) başladığı an: JMA hedefi "depremden
/// yaklaşık 3 dakika sonra" (気象庁「津波警報・注意報、津波情報、津波予報に
/// ついて」, 2026-11-23 okundu; PTWC ilk ürünü 3–10 dk, IOC TS 135 Tablo 1).
pub const ISSUE_S: f64 = 180.0;

/// JMA kategorileri (aynı sayfa): 大津波警報 > 3 m, 津波警報 1–3 m,
/// 津波注意報 0,2–1 m.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Category {
    Advisory,
    Warning,
    Major,
}

impl Category {
    /// Renkler: JMA HPColorGuide (2020) Tablo 1-2: 大津波警報 (200,0,255),
    /// 津波警報 (255,40,0), 津波注意報 (250,245,0) (docs/bilim.md 7.2 notu).
    pub fn color(self) -> eframe::egui::Color32 {
        use eframe::egui::Color32;
        match self {
            Self::Major => Color32::from_rgb(200, 0, 255),
            Self::Warning => Color32::from_rgb(255, 40, 0),
            Self::Advisory => Color32::from_rgb(250, 245, 0),
        }
    }

    /// Çeviri anahtarı.
    pub fn key(self) -> &'static str {
        match self {
            Self::Major => "tsunami-cat-major",
            Self::Warning => "tsunami-cat-warning",
            Self::Advisory => "tsunami-cat-advisory",
        }
    }

    pub fn from_height(h: f64) -> Option<Self> {
        if h > 3.0 {
            Some(Self::Major)
        } else if h > 1.0 {
            Some(Self::Warning)
        } else if h >= 0.2 {
            Some(Self::Advisory)
        } else {
            None
        }
    }
}

/// Uygulamayla gelen tsunami verisi.
pub struct TsunamiData {
    bathy: MappedRaster,
    layer: usize,
    pub coast: Vec<coast::CoastPoint>,
}

impl TsunamiData {
    pub fn load(dir: &std::path::Path) -> Result<Arc<Self>, String> {
        let bathy = crate::compute::map_raster(&dir.join(bathy::FILE))?;
        let layer = bathy
            .layer("depth_m")
            .ok_or_else(|| format!("{}: 'depth_m' yok", bathy::FILE))?;
        let path = dir.join(coast::FILE);
        let coast = std::fs::File::open(&path)
            .and_then(|f| coast::read(&mut std::io::BufReader::new(f)))
            .map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(Arc::new(Self {
            bathy,
            layer,
            coast,
        }))
    }

    /// 1′ pencereyi k × k birleştirerek derinlik (m; NaN kara). Boylamda
    /// sarmal okunur (pencere 180°'yi aşabilir).
    fn window(&self, west: f64, north: f64, w1: u32, h1: u32, k: u32) -> Vec<f32> {
        let g = self.bathy.grid;
        let col0 = ((west - g.west) / g.cell).round() as i64;
        let row0 = ((g.north - north) / g.cell).round() as i64;
        let info = &self.bathy.layers[self.layer];
        let codes = match self
            .bathy
            .window(self.layer, col0, row0, w1, h1, bathy::LAND)
        {
            Ok(c) => c,
            Err(e) => {
                log::error!("derinlik penceresi okunamadı: {e}");
                return Vec::new();
            }
        };
        let (w, h) = ((w1 / k) as usize, (h1 / k) as usize);
        let k = k as usize;
        let mut out = vec![f32::NAN; w * h];
        for y in 0..h {
            for x in 0..w {
                let (mut s, mut n) = (0.0f32, 0u32);
                for dy in 0..k {
                    for dx in 0..k {
                        if let Some(v) = info.value(codes[(y * k + dy) * w1 as usize + x * k + dx])
                        {
                            s += v;
                            n += 1;
                        }
                    }
                }
                if n > 0 {
                    out[y * w + x] = s / n as f32;
                }
            }
        }
        out
    }
}

/// Bir kıyı noktasının gösterilecek sonucu.
#[derive(Clone, Debug)]
pub struct CoastNote {
    pub lon: f64,
    pub lat: f64,
    pub category: Category,
    /// Tahmini en yüksek tırmanma (m); uzak alanda / doğrultu atımlıda `None`
    /// (PTWC tehdit bölgesi, yükseklik tahmin edilmedi).
    pub height: Option<f64>,
    pub eta_s: f64,
    /// En yakın şehir (≤ 50 km) ve ili.
    pub place: Option<(String, String)>,
}

impl TsunamiResult {
    /// Son varış (s; kategorili noktalar).
    pub fn last_eta(&self) -> Option<f64> {
        self.coast
            .iter()
            .map(|c| c.eta_s)
            .filter(|t| t.is_finite())
            .max_by(f64::total_cmp)
    }

    /// İl başına ilk varış (ana göre sıralı). Adsız noktalar atlanır.
    pub fn region_arrivals(&self) -> Vec<RegionArrival> {
        let mut by: std::collections::HashMap<&str, RegionArrival> =
            std::collections::HashMap::new();
        for c in &self.coast {
            let Some((_, region)) = &c.place else {
                continue;
            };
            let e = by.entry(region.as_str()).or_insert(RegionArrival {
                region: region.clone(),
                t: c.eta_s,
                height: c.height,
                category: c.category,
                at: (c.lon, c.lat),
            });
            if c.eta_s < e.t {
                e.t = c.eta_s;
                e.at = (c.lon, c.lat);
            }
            e.height = match (e.height, c.height) {
                (Some(a), Some(b)) => Some(a.max(b)),
                (a, b) => a.or(b),
            };
            e.category = e.category.max(c.category);
        }
        let mut v: Vec<RegionArrival> = by.into_values().collect();
        v.sort_by(|a, b| a.t.total_cmp(&b.t));
        v
    }
}

/// Bir ilin tsunami özeti: ilk varış (s), en yüksek tahmin (m), en yüksek
/// kategori, ilk varışın konumu.
#[derive(Clone, Debug)]
pub struct RegionArrival {
    pub region: String,
    pub t: f64,
    pub height: Option<f64>,
    pub category: Category,
    pub at: (f64, f64),
}

pub struct TsunamiResult {
    pub assessment: trigger::Assessment,
    /// Kategorisi olan kıyı noktaları (varış sırasıyla).
    pub coast: Vec<CoastNote>,
}

/// Bölgesel pencere yarıçapı (derece): PTWC tehdit yarıçapını kapsar.
fn regional_radius_deg(a: trigger::Assessment) -> f64 {
    match a {
        trigger::Assessment::Threat { radius_km: Some(r) } => (r / 111.0 + 2.0).min(14.0),
        _ => 8.0,
    }
}

pub fn run(
    q: &Quake,
    data: &TsunamiData,
    cities: &[City],
    cancel: &AtomicBool,
) -> Option<TsunamiResult> {
    let t0 = std::time::Instant::now();
    let g = data.bathy.grid;
    // Önce kaba değerlendirme (pencere boyutu için): Mw ve derinlikten; deniz
    // denetimi asıl hesapta.
    let pre = trigger::assess(q.mw, q.depth_km, true);
    let r = regional_radius_deg(pre);
    let cell = g.cell;
    let west =
        ((q.epicenter.0 - r / q.epicenter.1.to_radians().cos().max(0.2)) / cell).floor() * cell;
    let north = ((q.epicenter.1 + r).min(90.0) / cell).ceil() * cell;
    let south = ((q.epicenter.1 - r).max(-90.0) / cell).floor() * cell;
    let w1 = (((2.0 * (q.epicenter.0 - west)) / cell).ceil() as u32).min(g.width);
    let h1 = ((north - south) / cell).round() as u32;
    let depth = data.window(west, north, w1, h1, 1);
    if depth.is_empty() {
        return None;
    }
    let dw = forecast::Depth {
        west,
        north,
        cell,
        width: w1 as usize,
        height: h1 as usize,
        depth: &depth,
        wrap: false,
    };
    let regional = forecast::compute(q, &dw, &data.coast, 6.0 * 3600.0, cancel)?;
    log::info!(
        "tsunami: {:?}, H {:.2} m, d {:.0} m, eğim {:.4}, R0 {:.1} m; bölgesel {} × {} ({} ms)",
        regional.assessment,
        regional.h_max,
        regional.d,
        regional.slope,
        regional.r0,
        w1,
        h1,
        t0.elapsed().as_millis()
    );
    let mut results = regional.coast.clone();
    // Okyanus ölçeği: küresel 4′ varış süreleri, bölgesel pencere dışı.
    if matches!(
        regional.assessment,
        trigger::Assessment::Threat { radius_km: None }
    ) {
        let k = 4;
        let depth4 = data.window(g.west, g.north, g.width, g.height, k);
        if !depth4.is_empty() {
            let dg = forecast::Depth {
                west: g.west,
                north: g.north,
                cell: cell * f64::from(k),
                width: (g.width / k) as usize,
                height: (g.height / k) as usize,
                depth: &depth4,
                wrap: true,
            };
            let global = forecast::compute(q, &dg, &data.coast, 24.0 * 3600.0, cancel)?;
            let have: std::collections::HashSet<usize> = results.iter().map(|c| c.point).collect();
            for c in global.coast {
                if !have.contains(&c.point) {
                    // Uzak alanda yükseklik yok (yakın alan formülü yalnızca
                    // bölgesel pencerede).
                    results.push(forecast::CoastResult { runup_m: None, ..c });
                }
            }
            log::info!(
                "tsunami: küresel 4′ varış ({} ms)",
                t0.elapsed().as_millis()
            );
        }
    }
    let mut coast: Vec<CoastNote> = results
        .iter()
        .filter_map(|c| {
            let category = match c.runup_m {
                Some(h) => Category::from_height(h).or(c.threat.then_some(Category::Advisory)),
                None => c.threat.then_some(Category::Advisory),
            }?;
            if !c.threat && c.runup_m.is_none() {
                return None;
            }
            let p = &data.coast[c.point];
            let place = (p.city_km <= 50.0)
                .then(|| cities.get(p.city as usize))
                .flatten()
                .map(|city| (city.name.clone(), city.admin1.clone()));
            Some(CoastNote {
                lon: f64::from(p.lon),
                lat: f64::from(p.lat),
                category,
                height: c.runup_m.filter(|&h| h >= 0.2),
                eta_s: c.eta_s,
                place,
            })
        })
        .collect();
    coast.sort_by(|a, b| a.eta_s.total_cmp(&b.eta_s));
    Some(TsunamiResult {
        assessment: regional.assessment,
        coast,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jma_kategorileri() {
        assert_eq!(Category::from_height(0.19), None);
        assert_eq!(Category::from_height(0.2), Some(Category::Advisory));
        assert_eq!(Category::from_height(1.0), Some(Category::Advisory));
        assert_eq!(Category::from_height(1.01), Some(Category::Warning));
        assert_eq!(Category::from_height(3.0), Some(Category::Warning));
        assert_eq!(Category::from_height(3.1), Some(Category::Major));
    }
}
