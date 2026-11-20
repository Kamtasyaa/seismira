//! Dalma-batma levha geometrisi: USGS Slab2 (Faz 5).
//!
//! Kaynak: Hayes, G. (2018). Slab2 - A Comprehensive Subduction Zone
//! Geometry Model. U.S. Geological Survey data release,
//! doi:10.5066/F7PV6JNV (kamu malı). Yöntem: Hayes vd. (2018), Science
//! 362, 58–61. Dosya `data/slab2/slab2.sslb`, `scripts/gen_slab2.py` ile
//! üretilir (biçim orada).
//!
//! Kullanım:
//! - Merkez üssündeki zon ve levha geometrisi ([`Slab2::locate`]): arayüz
//!   depremi için doğrultu / eğim / derinlik önerisi.
//! - Parker vd. (2020) bölgesi ([`parker_region`]).
//! - Yay önü / yay ardı ayrımı ([`Slab2::is_backarc`]): Parker modeli
//!   yalnızca yay önü sahalar için kurulmuştur (PEER 2020/03, 8.2.1, 8.3.2).
//!
//! Eğim ve doğrultu derinliğin merkezi farkından türetilir (iki düğüm
//! aralığı, küre yarıçapı 6371 km); Slab2'nin kendi dip / str ızgaraları
//! tam olarak budur (`gen_slab2.py --check`). Uzlaşım: doğrultu 0–360°,
//! eğim doğrultunun sağına (Aki & Richards), Slab2 ile aynı.

use crate::gmpe::psbah20::Region;
use crate::quake::Setting;
use std::io;
use std::path::Path;
use std::sync::OnceLock;

const MAGIC: &[u8; 4] = b"SSLB";
const VERSION: u32 = 1;
const HEADER: usize = 16;
const ZONE: usize = 52;
const OUTSIDE: u16 = 0x8000;
/// Slab2'nin türevde kullandığı küre yarıçapı (km), `gen_slab2.py`.
const EARTH_R_KM: f64 = 6371.0;

/// Volkanik yayın altındaki levha üst yüzeyi derinliği (km): Syracuse &
/// Abers (2006), G³ 7, Q05017, doi:10.1029/2005GC001045, özet: 500 km'lik
/// yay parçalarında 72–173 km, küresel ortalama 105 km. Levha bu derinlikten
/// daha derindeyse saha yay ardındadır. VARSAYIM: zon başına ayrı değer
/// yerine küresel ortalama (makalenin zon tablosu görülmedi).
pub const ARC_SLAB_DEPTH_KM: f64 = 105.0;

/// Bir Slab2 zonunun dosyadaki kaydı.
struct Zone {
    code: String,
    /// İlk sütun / satır düğümü (derece); `west` 180'i aşabilir.
    west: f64,
    south: f64,
    step: f64,
    w: usize,
    h: usize,
    frame: std::ops::Range<usize>,
    data: OnceLock<Option<Vec<u16>>>,
}

/// Bir noktadaki levha geometrisi.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Geometry {
    /// Levha üst yüzeyinin derinliği (km).
    pub depth_km: f64,
    /// Doğrultu (0–360°) ve eğim (0–90°).
    pub strike: f64,
    pub dip: f64,
}

/// [`Slab2::locate`] sonucu.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    /// Zon sırası ([`Slab2::code`] ile adı).
    pub zone: usize,
    pub geometry: Geometry,
}

pub struct Slab2 {
    bytes: Vec<u8>,
    unit_km: f64,
    zones: Vec<Zone>,
}

fn bad(msg: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, format!("slab2: {msg}"))
}

impl Slab2 {
    pub fn load(path: &Path) -> io::Result<Self> {
        Self::from_bytes(std::fs::read(path)?)
    }

    pub fn from_bytes(bytes: Vec<u8>) -> io::Result<Self> {
        let u32_at = |b: &[u8], p: usize| u32::from_le_bytes(b[p..p + 4].try_into().unwrap());
        let f64_at = |b: &[u8], p: usize| f64::from_le_bytes(b[p..p + 8].try_into().unwrap());
        if bytes.len() < HEADER || &bytes[..4] != MAGIC {
            return Err(bad("başlık yok"));
        }
        if u32_at(&bytes, 4) != VERSION {
            return Err(bad("desteklenmeyen sürüm"));
        }
        let n = u32_at(&bytes, 8) as usize;
        let unit_km = f64::from(f32::from_le_bytes(bytes[12..16].try_into().unwrap()));
        if bytes.len() < HEADER + n * ZONE {
            return Err(bad("zon dizini eksik"));
        }
        let mut zones = Vec::with_capacity(n);
        for i in 0..n {
            let p = HEADER + i * ZONE;
            let code = String::from_utf8_lossy(&bytes[p..p + 4])
                .trim_end_matches('\0')
                .to_string();
            let off = u64::from_le_bytes(bytes[p + 36..p + 44].try_into().unwrap()) as usize;
            let len = u32_at(&bytes, p + 44) as usize;
            if off + len > bytes.len() {
                return Err(bad("zon verisi dosya dışında"));
            }
            zones.push(Zone {
                code,
                west: f64_at(&bytes, p + 4),
                south: f64_at(&bytes, p + 12),
                step: f64_at(&bytes, p + 20),
                w: u32_at(&bytes, p + 28) as usize,
                h: u32_at(&bytes, p + 32) as usize,
                frame: off..off + len,
                data: OnceLock::new(),
            });
        }
        Ok(Self {
            bytes,
            unit_km,
            zones,
        })
    }

    pub fn len(&self) -> usize {
        self.zones.len()
    }

    pub fn is_empty(&self) -> bool {
        self.zones.is_empty()
    }

    /// Zonun Slab2 kodu (ör. "kur", "sam").
    pub fn code(&self, zone: usize) -> &str {
        &self.zones[zone].code
    }

    /// Zon verisi; ilk kullanımda açılır. Bozuksa `None` (günlüğe yazılır).
    fn cells(&self, zone: usize) -> Option<&[u16]> {
        let z = &self.zones[zone];
        z.data
            .get_or_init(|| {
                let raw =
                    crate::raster::decompress(&self.bytes[z.frame.clone()], z.w * z.h * 2).ok()?;
                Some(
                    raw.as_chunks::<2>()
                        .0
                        .iter()
                        .map(|c| u16::from_le_bytes(*c))
                        .collect(),
                )
            })
            .as_deref()
    }

    /// Noktanın zon ızgarasındaki kesirli (sütun, satır) konumu; dışındaysa
    /// `None`. Boylam zonun başlangıcına göre 360°'lik pencereye çekilir.
    fn frac(&self, zone: usize, lon: f64, lat: f64) -> Option<(f64, f64)> {
        let z = &self.zones[zone];
        let x = (lon - z.west).rem_euclid(360.0) / z.step;
        let y = (lat - z.south) / z.step;
        let (wmax, hmax) = ((z.w - 1) as f64, (z.h - 1) as f64);
        ((0.0..=wmax).contains(&x) && (0.0..=hmax).contains(&y)).then_some((x, y))
    }

    /// Çift doğrusal derinlik (km) ve en yakın düğümün levha içinde olup
    /// olmadığı. Zon (genişletilmiş) ızgarasının dışında `None`.
    fn sample(&self, zone: usize, lon: f64, lat: f64) -> Option<(f64, bool)> {
        let (x, y) = self.frac(zone, lon, lat)?;
        let cells = self.cells(zone)?;
        let w = self.zones[zone].w;
        let (x0, y0) = (x.floor() as usize, y.floor() as usize);
        let (x1, y1) = ((x0 + 1).min(w - 1), (y0 + 1).min(self.zones[zone].h - 1));
        let (fx, fy) = (x - x0 as f64, y - y0 as f64);
        let at = |c: usize, r: usize| f64::from(cells[r * w + c] & !OUTSIDE) * self.unit_km;
        let d = (at(x0, y0) * (1.0 - fx) + at(x1, y0) * fx) * (1.0 - fy)
            + (at(x0, y1) * (1.0 - fx) + at(x1, y1) * fx) * fy;
        let (nx, ny) = (x.round() as usize, y.round() as usize);
        Some((d, cells[ny * w + nx] & OUTSIDE == 0))
    }

    /// Levha üzerindeki noktanın geometrisi; nokta levha dışındaysa `None`.
    pub fn geometry(&self, zone: usize, lon: f64, lat: f64) -> Option<Geometry> {
        let (depth_km, inside) = self.sample(zone, lon, lat)?;
        if !inside {
            return None;
        }
        let s = self.zones[zone].step;
        // Merkezi fark, iki düğüm aralığı (Slab2 ile aynı); kenarda
        // tek yönlüye düşmek yerine kenar düğümleri kullanılmaz.
        let d = |dlon: f64, dlat: f64| self.sample(zone, lon + dlon, lat + dlat).map(|v| v.0);
        let (e, w, n, so) = (d(s, 0.0)?, d(-s, 0.0)?, d(0.0, s)?, d(0.0, -s)?);
        let step_km = s.to_radians() * EARTH_R_KM;
        let gx = (e - w) / (2.0 * step_km * lat.to_radians().cos());
        let gy = (n - so) / (2.0 * step_km);
        Some(Geometry {
            depth_km,
            dip: gx.hypot(gy).atan().to_degrees(),
            strike: (gx.atan2(gy).to_degrees() - 90.0).rem_euclid(360.0),
        })
    }

    /// Merkez üssünün altındaki levha. Birden çok zon üst üste biniyorsa
    /// (ör. Kanto: Pasifik ve Filipin Denizi levhaları) levha yüzeyi odak
    /// derinliğine en yakın olan seçilir. Levha içi depremde odak levha
    /// yüzeyinin altında olmalıdır (10 km tolerans: Slab2 belirsizliği).
    pub fn locate(&self, lon: f64, lat: f64, depth_km: f64, setting: Setting) -> Option<Hit> {
        (0..self.zones.len())
            .filter_map(|zone| {
                let g = self.geometry(zone, lon, lat)?;
                (setting != Setting::Intraslab || depth_km >= g.depth_km - 10.0)
                    .then_some(Hit { zone, geometry: g })
            })
            .min_by(|a, b| {
                let da = (a.geometry.depth_km - depth_km).abs();
                let db = (b.geometry.depth_km - depth_km).abs();
                da.total_cmp(&db)
            })
    }

    /// Saha, `zone` levhasına göre yay ardında mı? Sahanın altındaki levha
    /// yüzeyi (levha dışındaki sahada en yakın levha hücresininki)
    /// [`ARC_SLAB_DEPTH_KM`]'den derinse evet. Zonun 3° genişletilmiş
    /// ızgarası dışında `None` (bilinmiyor).
    pub fn is_backarc(&self, zone: usize, lon: f64, lat: f64) -> Option<bool> {
        let (d, _) = self.sample(zone, lon, lat)?;
        Some(d > ARC_SLAB_DEPTH_KM)
    }
}

/// Merkez üssünün Slab2 zonundan Parker vd. (2020) bölgesi.
///
/// Bölge adları rapor Tablo 4.1'den; alt bölge sınırları Campbell (2020,
/// Earthquake Spectra, kapalı erişim) tanımlıdır ve görülmedi. Sınırlar
/// bu yüzden VARSAYIMDIR (docs/bilim.md 7.4):
/// - Alaska / Aleut: Unimak Geçidi (164,5° B); doğusu Alaska.
/// - Güney Amerika kuzey / güney: Arica dirseği (18,3° G). Gerekçe: rapor
///   güneyde 13, kuzeyde 2 arayüz depremi sayar; veri tabanında Şili
///   depremleri çok, Peru'da iki büyük deprem var (2001 Arequipa, 2007
///   Pisco).
/// - Orta Amerika kuzey / güney: Meksika–Guatemala sınırı (92,2° B). İki
///   alt bölgenin katsayıları aynıdır; yalnızca levha içi Mc farklı.
/// - Tayvan: Ryukyu zonunun 123° D batısı doğu alt bölgesi, Manila
///   zonunun 21° K kuzeyi batı alt bölgesi; katsayılar ikisinde aynı.
/// - Izu-Bonin: Pasifik levhası (Japonya – Pasifik).
///
/// Listede olmayan zonlar (Yeni Zelanda, Hellen, Calabria, Makran,
/// Endonezya vb.) küresel modeli kullanır (rapor 4.3.4).
pub fn parker_region(code: &str, lon: f64, lat: f64) -> Region {
    let lon = (lon + 180.0).rem_euclid(360.0) - 180.0;
    match code {
        "alu" if (-164.5..0.0).contains(&lon) => Region::Alaska,
        "alu" => Region::Aleutian,
        "cas" => Region::Cascadia,
        "cam" if lon < -92.2 => Region::CamN,
        "cam" => Region::CamS,
        "kur" | "izu" => Region::JapanPac,
        "ryu" if lon < 123.0 => Region::TaiwanE,
        "ryu" => Region::JapanPhi,
        "man" if lat >= 21.0 => Region::TaiwanW,
        "sam" if lat >= -18.3 => Region::SaN,
        "sam" => Region::SaS,
        _ => Region::Global,
    }
}

#[cfg(test)]
pub(crate) fn test_model() -> Slab2 {
    let p = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/slab2/slab2.sslb");
    Slab2::load(Path::new(p)).expect("data/slab2/slab2.sslb (scripts/gen_slab2.py)")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zone(m: &Slab2, code: &str) -> usize {
        (0..m.len()).find(|&i| m.code(i) == code).unwrap()
    }

    /// Slab2'nin ham düğüm değerleri (derinlik, eğim, doğrultu) ile
    /// karşılaştırma. Tek fark kaynağı 0,05 km derinlik kuantalaması:
    /// derinlikte ≤ 0,025 km; eğimin tanjantında (gradyan) bileşen başına
    /// ≤ 0,05 km / iki düğüm aralığı. Doğrultu hatası bunun eğimin
    /// tanjantına oranı kadardır (yatık yerde ve 0,02° adımlı zonlarda büyür).
    #[test]
    fn slab2_dugumleriyle_ayni() {
        let m = test_model();
        let csv = include_str!("../../../data/test/slab2_ref.csv");
        let mut n = 0;
        for line in csv.lines().filter(|l| !l.starts_with('#')).skip(1) {
            let f: Vec<&str> = line.split(',').collect();
            let num = |i: usize| f[i].parse::<f64>().unwrap();
            let z = zone(&m, f[0]);
            let g = m.geometry(z, num(1), num(2)).expect(line);
            assert!((g.depth_km - num(3)).abs() < 0.026, "{line}: {g:?}");
            let step_km = m.zones[z].step.to_radians() * EARTH_R_KM * num(2).to_radians().cos();
            let g_err = std::f64::consts::SQRT_2 * m.unit_km / (2.0 * step_km);
            // Sumatra-Java: Slab2'nin kendi eğim / doğrultu ızgarası türevden
            // biraz farklı (gen_slab2.py --check: p95 1,2° / 1,7°).
            let extra = if f[0] == "sum" { 2.0 } else { 0.0 };
            assert!(
                (g.dip - num(4)).abs() < g_err.to_degrees() + 0.01 + extra,
                "{line}: {g:?}"
            );
            // Doğrultu yalnızca eğim belirginse anlamlı.
            let tan_dip = num(4).to_radians().tan();
            if g_err < 0.5 * tan_dip {
                let tol = (g_err / tan_dip).asin().to_degrees() + 0.05 + extra;
                let ds = (g.strike - num(5) + 180.0).rem_euclid(360.0) - 180.0;
                assert!(ds.abs() < tol, "{line}: {g:?} (tolerans {tol:.2}°)");
            }
            n += 1;
        }
        assert_eq!(n, 27 * 12);
    }

    #[test]
    fn tohoku_merkez_ussu() {
        // USGS: 2011 Tōhoku, 38.297 K 142.373 D, 29 km; W-faz çözümü
        // doğrultu 193°, eğim 14° (USGS olay sayfası, yalnızca kıyas).
        let m = test_model();
        let hit = m.locate(142.373, 38.297, 29.0, Setting::Interface).unwrap();
        assert_eq!(m.code(hit.zone), "kur");
        let g = hit.geometry;
        assert!((15.0..35.0).contains(&g.depth_km), "{g:?}");
        assert!((185.0..205.0).contains(&g.strike), "{g:?}");
        assert!((8.0..20.0).contains(&g.dip), "{g:?}");
        assert_eq!(
            parker_region(m.code(hit.zone), 142.373, 38.297),
            Region::JapanPac
        );
    }

    #[test]
    fn yay_onu_ve_ardi() {
        let m = test_model();
        let kur = zone(&m, "kur");
        // Sendai (yay önü, Pasifik kıyısı) / Akita (yay ardı, Japon Denizi
        // kıyısı); Tōhoku'da volkanik cephe Ou dağları boyunca.
        assert_eq!(m.is_backarc(kur, 140.87, 38.27), Some(false));
        assert_eq!(m.is_backarc(kur, 140.10, 39.72), Some(true));
        // Hendeğin okyanus tarafı: yay önü.
        assert_eq!(m.is_backarc(kur, 145.5, 38.0), Some(false));
        // Vladivostok ve Sahalin (Yujno-Sahalinsk): yay ardı.
        assert_eq!(m.is_backarc(kur, 131.9, 43.1), Some(true));
        assert_eq!(m.is_backarc(kur, 142.7, 46.96), Some(true));
        // Zonun genişletilmiş ızgarası dışı: bilinmiyor.
        assert_eq!(m.is_backarc(kur, 30.0, 40.0), None);
    }

    #[test]
    fn ust_uste_binen_zonlar_ve_antimeridyen() {
        let m = test_model();
        // Aleut zonu 180°'yi keser (161–226° D); Adak (176.6° B).
        let hit = m.locate(-176.6, 51.0, 30.0, Setting::Interface);
        assert!(hit.is_some_and(|h| m.code(h.zone) == "alu"), "{hit:?}");
        assert_eq!(parker_region("alu", -176.6, 51.0), Region::Aleutian);
        assert_eq!(parker_region("alu", 178.0, 51.6), Region::Aleutian);
        assert_eq!(parker_region("alu", -150.0, 59.0), Region::Alaska);
        // Açık deniz, hiçbir levha yok.
        assert!(m.locate(-30.0, 0.0, 20.0, Setting::Interface).is_none());
        // Levha içi: odak levha yüzeyinin çok üstündeyse o zon seçilmez.
        let deep = m.geometry(zone(&m, "kur"), 141.0, 40.0).unwrap();
        assert!(deep.depth_km > 60.0, "{deep:?}");
        assert!(m.locate(141.0, 40.0, 10.0, Setting::Intraslab).is_none());
        assert!(
            m.locate(141.0, 40.0, deep.depth_km + 20.0, Setting::Intraslab)
                .is_some()
        );
    }
}
