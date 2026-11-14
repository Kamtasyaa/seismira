//! Birinci düzey idari bölgeler (il / eyalet / prefektörlük): kimlik
//! rasterı ve bölge bilgileri (CLAUDE.md Bölüm 9).
//!
//! - Kimlik rasterı: Vs30 haritasıyla aynı ızgarada (30″) u16 bölge
//!   kimliği; `.sras` içinde iki bayt katmanı (`region_lo`, `region_hi`)
//!   olarak saklanır (kodlar ham bayttır, değer eşlemesi kullanılmaz).
//!   0 = bölge yok (deniz). En üst bit ([`BUILT_BIT`]) hücrenin yerleşim
//!   olduğunu gösterir (GHSL yapılaşma alanı ≥ hücrenin %1'i); kimlik alt
//!   15 bittir.
//! - Bölge bilgileri (`.sreg`): kimlik `i` için `regions[i - 1]`.
//!
//! Kaynak: Natural Earth 1:10m Admin-1 (kamu malı). Üretim:
//! `seismira-data regions`.

use crate::grid::{Domain, NO_DATA, ShakeGrid};
use crate::intensity::Shindo;
use crate::raster::Raster;
use std::io::{self, Read, Write};

/// Bölge kimliği; 0 = yok.
pub type RegionId = u16;

/// Raster değerinde "yerleşim hücresi" biti.
pub const BUILT_BIT: u16 = 0x8000;
/// Raster değerinden kimlik.
pub const ID_MASK: u16 = 0x7FFF;

#[derive(Clone, Debug, PartialEq)]
pub struct RegionInfo {
    /// ISO 3166-1 alfa-2 (Natural Earth `iso_a2`; bazı bölgelerde "-9" → boş).
    pub country: [u8; 2],
    /// ISO 3166-2 (ör. "TR-41").
    pub iso: String,
    pub name_en: String,
    pub name_tr: String,
}

const MAGIC: &[u8; 4] = b"SREG";
const VERSION: u32 = 1;

pub fn write_infos(regions: &[RegionInfo], w: &mut impl Write) -> io::Result<()> {
    w.write_all(MAGIC)?;
    w.write_all(&VERSION.to_le_bytes())?;
    w.write_all(&(regions.len() as u32).to_le_bytes())?;
    for r in regions {
        w.write_all(&r.country)?;
        for s in [&r.iso, &r.name_en, &r.name_tr] {
            let b = s.as_bytes();
            let n = u16::try_from(b.len()).map_err(|_| bad("ad çok uzun"))?;
            w.write_all(&n.to_le_bytes())?;
            w.write_all(b)?;
        }
    }
    Ok(())
}

pub fn read_infos(r: &mut impl Read) -> io::Result<Vec<RegionInfo>> {
    let mut head = [0u8; 12];
    r.read_exact(&mut head)?;
    if &head[..4] != MAGIC {
        return Err(bad("SREG dosyası değil"));
    }
    if u32::from_le_bytes(head[4..8].try_into().unwrap_or_default()) != VERSION {
        return Err(bad("desteklenmeyen SREG sürümü"));
    }
    let n = u32::from_le_bytes(head[8..12].try_into().unwrap_or_default());
    let mut out = Vec::with_capacity(n as usize);
    for _ in 0..n {
        let mut country = [0u8; 2];
        r.read_exact(&mut country)?;
        let mut s = || -> io::Result<String> {
            let mut l = [0u8; 2];
            r.read_exact(&mut l)?;
            let mut b = vec![0u8; u16::from_le_bytes(l) as usize];
            r.read_exact(&mut b)?;
            String::from_utf8(b).map_err(|_| bad("ad UTF-8 değil"))
        };
        let (iso, name_en, name_tr) = (s()?, s()?, s()?);
        out.push(RegionInfo {
            country,
            iso,
            name_en,
            name_tr,
        });
    }
    Ok(out)
}

/// Kimlik rasterı okuyucu.
pub struct RegionMap<B> {
    raster: Raster<B>,
    lo: usize,
    hi: usize,
}

impl<B: AsRef<[u8]>> RegionMap<B> {
    pub fn open(bytes: B) -> io::Result<Self> {
        let raster = Raster::open(bytes)?;
        let lo = raster
            .layer("region_lo")
            .ok_or_else(|| bad("region_lo yok"))?;
        let hi = raster
            .layer("region_hi")
            .ok_or_else(|| bad("region_hi yok"))?;
        Ok(Self { raster, lo, hi })
    }

    pub fn grid(&self) -> crate::raster::Grid {
        self.raster.grid
    }

    /// Noktadaki bölge (0: yok).
    pub fn id_at(&self, lon: f64, lat: f64) -> io::Result<RegionId> {
        let lo = self.raster.code_at(self.lo, lon, lat)?.unwrap_or(0);
        let hi = self.raster.code_at(self.hi, lon, lat)?.unwrap_or(0);
        Ok((u16::from(lo) | (u16::from(hi) << 8)) & ID_MASK)
    }

    /// Dikdörtgen pencere (bkz. [`Raster::window`]); dışı 0. Ham değer:
    /// kimlik için `& ID_MASK`, yerleşim için `& BUILT_BIT`.
    pub fn window(&self, col0: i64, row0: i64, w: u32, h: u32) -> io::Result<Vec<RegionId>> {
        let lo = self.raster.window(self.lo, col0, row0, w, h, 0)?;
        let hi = self.raster.window(self.hi, col0, row0, w, h, 0)?;
        Ok(lo
            .into_iter()
            .zip(hi)
            .map(|(l, h)| u16::from(l) | (u16::from(h) << 8))
            .collect())
    }
}

/// Bir bölgenin ızgaradan toplanan özeti (zamandan bağımsız; görünüm
/// zamanın saf fonksiyonu olarak bunlardan türetilir).
///
/// En yüksek Shindo (ve tepe anı, konum) YERLEŞİM hücrelerinden alınır;
/// bölgede alan içinde yerleşim hücresi yoksa tüm hücrelerden. Gerekçe:
/// JMA'nın bölge değeri istasyonlardan gelir ve istasyonlar yerleşim
/// yerlerindedir; boş dağ / tarla hücrelerindeki birkaç yumuşak zemin
/// hücresi bütün ili yükseltmesin (ölçüm ve karar: docs/arayuz.md).
/// İlk S varışı tüm hücrelerden (sarsıntı bölgede başladı).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RegionSummary {
    pub id: RegionId,
    /// Bölgedeki en yüksek son Shindo.
    pub max: Shindo,
    /// Bölgedeki ilk S varışı (s).
    pub t_first_s: f64,
    /// En yüksek Shindo'ya ilk ulaşılan an (kademeli yükselişle): en
    /// yüksek sınıftaki hücrelerin en erken `t_peak`'i.
    pub t_peak: f64,
    /// Veri olan hücre sayısı.
    pub cells: u32,
    /// Veri olan yerleşim hücresi sayısı (0 ise değerler tüm hücrelerden).
    pub built_cells: u32,
    /// En yüksek Shindo'ya ilk ulaşan hücrenin merkezi (boylam, enlem).
    pub at: (f64, f64),
}

/// Izgarayı bölgelere göre toplar. `ids`: alanın taban çözünürlükte
/// kimlik penceresi (`domain.w × domain.h`); her ızgara hücresi, kapsadığı
/// taban hücrelerin ortadakinin bölgesine sayılır (ızgaradaki Vs30
/// seçimiyle aynı kural). Sonuç kimliğe göre sıralı.
pub fn summarize(
    grid: &ShakeGrid,
    domain: &Domain,
    step: u32,
    ids: &[RegionId],
) -> Vec<RegionSummary> {
    use std::collections::HashMap;
    let rise = crate::shaking::RISE_STEP_S;
    // Bölge başına: (tüm hücreler, yerleşim hücreleri) özetleri.
    let mut acc: HashMap<RegionId, (RegionSummary, Option<RegionSummary>)> = HashMap::new();
    let update = |e: &mut RegionSummary, s: Shindo, ts: f64, peak: f64, at: (f64, f64)| {
        e.cells += 1;
        e.t_first_s = e.t_first_s.min(ts);
        if s > e.max || (s == e.max && peak < e.t_peak) {
            e.max = s;
            e.t_peak = peak;
            e.at = at;
        }
    };
    for row in 0..grid.spec.height {
        for col in 0..grid.spec.width {
            let i = (row * grid.spec.width + col) as usize;
            let class = grid.class[i];
            if class == NO_DATA {
                continue;
            }
            let bx = (col * step + step / 2).min(domain.w - 1) as usize;
            let by = (row * step + step / 2).min(domain.h - 1) as usize;
            let raw = ids[by * domain.w as usize + bx];
            let id = raw & ID_MASK;
            if id == 0 {
                continue;
            }
            let ts = f64::from(grid.t_s[i]);
            let s = Shindo::ALL[class as usize];
            let peak = ts + f64::from(class.saturating_sub(Shindo::S1 as u8)) * rise;
            let at = grid.spec.center(col, row);
            let fresh = RegionSummary {
                id,
                max: s,
                t_first_s: ts,
                t_peak: peak,
                cells: 0,
                built_cells: 0,
                at,
            };
            let e = acc.entry(id).or_insert((fresh, None));
            update(&mut e.0, s, ts, peak, at);
            if raw & BUILT_BIT != 0 {
                update(e.1.get_or_insert(fresh), s, ts, peak, at);
            }
        }
    }
    let mut out: Vec<_> = acc
        .into_values()
        .map(|(all, built)| match built {
            Some(b) => RegionSummary {
                t_first_s: all.t_first_s,
                cells: all.cells,
                built_cells: b.cells,
                ..b
            },
            None => all,
        })
        .collect();
    out.sort_by_key(|r| r.id);
    out
}

fn bad(msg: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::{Base, GridSpec};

    #[test]
    fn bilgi_dosyasi_gidis_donus() {
        let r = vec![
            RegionInfo {
                country: *b"TR",
                iso: "TR-41".into(),
                name_en: "Kocaeli".into(),
                name_tr: "Kocaeli".into(),
            },
            RegionInfo {
                country: *b"JP",
                iso: "JP-13".into(),
                name_en: "Tokyo".into(),
                name_tr: "Tokyo".into(),
            },
        ];
        let mut buf = Vec::new();
        write_infos(&r, &mut buf).unwrap();
        assert_eq!(read_infos(&mut buf.as_slice()).unwrap(), r);
    }

    #[test]
    fn ozet_en_yuksek_ve_ilk_varis() {
        // 2×2 ızgara, adım 1: sol sütun bölge 1, sağ sütun bölge 2.
        let domain = Domain {
            base: Base::GLOBAL_30S,
            col0: 0,
            row0: 0,
            w: 2,
            h: 2,
        };
        let grid = ShakeGrid {
            spec: GridSpec {
                west: 0.0,
                north: 0.0,
                cell: 1.0,
                width: 2,
                height: 2,
            },
            class: vec![
                Shindo::S5Lower as u8,
                Shindo::S3 as u8,
                Shindo::S6Lower as u8,
                NO_DATA,
            ],
            jma: vec![4.6, 3.0, 5.6, f32::NAN],
            t_s: vec![10.0, 12.0, 8.0, f32::NAN],
        };
        let ids = vec![1, 2, 1, 2];
        let s = summarize(&grid, &domain, 1, &ids);
        assert_eq!(s[0].built_cells, 0);
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].id, 1);
        assert_eq!(s[0].max, Shindo::S6Lower);
        assert_eq!(s[0].t_first_s, 8.0);
        assert_eq!(s[0].cells, 2);
        // 6- = S1'den 6 adım: 8 + 6·0,4.
        assert!((s[0].t_peak - (8.0 + 6.0 * crate::shaking::RISE_STEP_S)).abs() < 1e-9);
        assert_eq!(s[1].max, Shindo::S3);
        assert_eq!(s[1].cells, 1);

        // Yalnızca 5- hücresi yerleşim: bölge 1'in değeri 5- olur (6- boş
        // hücrede), ilk S varışı yine tüm hücrelerden (8 s).
        let ids = vec![1 | BUILT_BIT, 2, 1, 2];
        let s = summarize(&grid, &domain, 1, &ids);
        assert_eq!(s[0].max, Shindo::S5Lower);
        assert_eq!(s[0].built_cells, 1);
        assert_eq!(s[0].cells, 2);
        assert_eq!(s[0].t_first_s, 8.0);
    }
}
