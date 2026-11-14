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
    /// Bölgede (seçilen hücrelerde) sarsıntının bittiği en geç an.
    pub t_end: f64,
    /// `class_times[k]`: bölgede gösterilen en yüksek Shindo'nun k'ya
    /// ulaştığı an (kademeli yükselişle; hiç ulaşmazsa sonsuz).
    pub class_times: [f64; 10],
}

/// Bölgenin görünür durumu (Bölüm 9); simülasyon zamanının saf fonksiyonu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegionState {
    Quiet,
    /// Uyarı yayımlandı, S henüz ulaşmadı.
    Warning,
    /// Sarsıntı sürüyor; o ana kadar ulaşılan en yüksek Shindo.
    Shaking(Shindo),
    /// Sarsıntı bitti; nihai en yüksek Shindo.
    Settled(Shindo),
}

impl RegionSummary {
    /// `t` anında bölgede gösterilen en yüksek Shindo (1 ve üstü).
    pub fn shindo_at(&self, t: f64) -> Option<Shindo> {
        (1..10)
            .rev()
            .find(|&k| self.class_times[k] <= t)
            .map(|k| Shindo::ALL[k])
    }

    /// Bölgenin durumu. `t_alert`: uyarı yayımlandıysa ve bölge
    /// kapsamdaysa uyarı anı. `t_end`: bölgede sarsıntının bittiği an
    /// (bilinmiyorsa `None`: sarsıntı sürer).
    pub fn state(&self, t: f64, t_alert: Option<f64>, t_end: Option<f64>) -> RegionState {
        if let Some(s) = self.shindo_at(t) {
            return if t_end.is_some_and(|e| t >= e) {
                RegionState::Settled(self.max)
            } else {
                RegionState::Shaking(s)
            };
        }
        if t_alert.is_some_and(|a| t >= a) {
            RegionState::Warning
        } else {
            RegionState::Quiet
        }
    }
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
    let update =
        |e: &mut RegionSummary, s: Shindo, ts: f64, peak: f64, end: f64, at: (f64, f64)| {
            e.t_end = e.t_end.max(end);
            for k in 1..=s as usize {
                let tk = ts + (k - 1) as f64 * rise;
                e.class_times[k] = e.class_times[k].min(tk);
            }
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
            let end = f64::from(grid.t_end[i]);
            let fresh = RegionSummary {
                t_end: f64::NEG_INFINITY,
                id,
                max: s,
                t_first_s: ts,
                t_peak: peak,
                cells: 0,
                built_cells: 0,
                at,
                class_times: [f64::INFINITY; 10],
            };
            let e = acc.entry(id).or_insert((fresh, None));
            update(&mut e.0, s, ts, peak, end, at);
            if raw & BUILT_BIT != 0 {
                update(e.1.get_or_insert(fresh), s, ts, peak, end, at);
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

/// Bölge sınır halkaları (`.sout`): bölge `i` için `regions[i - 1]`,
/// her halka Web Mercator birim karesinde kapalı bir nokta dizisi.
///
/// Biçim: `"SOUT" u32 sürüm u32 bölge sayısı`, ardından tek bir zstd
/// çerçevesi: her bölge için `u32 halka sayısı`, her halka için
/// `u32 nokta sayısı` ve `[f32; 2] × n`.
pub type Outlines = Vec<Vec<Vec<[f32; 2]>>>;

const OUT_MAGIC: &[u8; 4] = b"SOUT";
const OUT_VERSION: u32 = 1;

pub fn write_outlines(
    outlines: &Outlines,
    w: &mut impl Write,
    compress: impl FnOnce(&[u8]) -> io::Result<Vec<u8>>,
) -> io::Result<()> {
    let mut raw = Vec::new();
    for rings in outlines {
        raw.extend_from_slice(&(rings.len() as u32).to_le_bytes());
        for ring in rings {
            raw.extend_from_slice(&(ring.len() as u32).to_le_bytes());
            for p in ring {
                raw.extend_from_slice(&p[0].to_le_bytes());
                raw.extend_from_slice(&p[1].to_le_bytes());
            }
        }
    }
    w.write_all(OUT_MAGIC)?;
    w.write_all(&OUT_VERSION.to_le_bytes())?;
    w.write_all(&(outlines.len() as u32).to_le_bytes())?;
    w.write_all(&compress(&raw)?)?;
    Ok(())
}

pub fn read_outlines(r: &mut impl Read) -> io::Result<Outlines> {
    use ruzstd::io::Read as _;
    let mut head = [0u8; 12];
    r.read_exact(&mut head)?;
    if &head[..4] != OUT_MAGIC {
        return Err(bad("SOUT dosyası değil"));
    }
    if u32::from_le_bytes(head[4..8].try_into().unwrap_or_default()) != OUT_VERSION {
        return Err(bad("desteklenmeyen SOUT sürümü"));
    }
    let n = u32::from_le_bytes(head[8..12].try_into().unwrap_or_default()) as usize;
    let mut rest = Vec::new();
    r.read_to_end(&mut rest)?;
    let mut src = rest.as_slice();
    let mut dec = ruzstd::decoding::StreamingDecoder::new(&mut src)
        .map_err(|e| bad(&format!("zstd: {e}")))?;
    let mut raw = Vec::new();
    dec.read_to_end(&mut raw)?;
    let mut pos = 0usize;
    let mut u32_at = |raw: &[u8]| -> io::Result<u32> {
        let b = raw.get(pos..pos + 4).ok_or_else(|| bad("SOUT kesik"))?;
        pos += 4;
        Ok(u32::from_le_bytes(b.try_into().unwrap_or_default()))
    };
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        let n_rings = u32_at(&raw)? as usize;
        let mut rings = Vec::with_capacity(n_rings);
        for _ in 0..n_rings {
            let n_pts = u32_at(&raw)? as usize;
            let mut ring = Vec::with_capacity(n_pts);
            for _ in 0..n_pts {
                let x = f32::from_bits(u32_at(&raw)?);
                let y = f32::from_bits(u32_at(&raw)?);
                ring.push([x, y]);
            }
            rings.push(ring);
        }
        out.push(rings);
    }
    Ok(out)
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
            t_end: vec![40.0, 30.0, 50.0, f32::NAN],
        };
        let ids = vec![1, 2, 1, 2];
        let s = summarize(&grid, &domain, 1, &ids);
        assert_eq!(s[0].built_cells, 0);
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].id, 1);
        assert_eq!(s[0].max, Shindo::S6Lower);
        assert_eq!(s[0].t_first_s, 8.0);
        assert_eq!(s[0].cells, 2);
        assert_eq!(s[0].t_end, 50.0);
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

    #[test]
    fn sinir_dosyasi_gidis_donus() {
        let o: Outlines = vec![
            vec![vec![[0.1, 0.2], [0.3, 0.4], [0.1, 0.2]]],
            vec![],
            vec![vec![[0.5, 0.5]], vec![[0.6, 0.6], [0.7, 0.7]]],
        ];
        let mut buf = Vec::new();
        write_outlines(&o, &mut buf, |b| {
            Ok(ruzstd::encoding::compress_to_vec(
                b,
                ruzstd::encoding::CompressionLevel::Fastest,
            ))
        })
        .unwrap();
        assert_eq!(read_outlines(&mut buf.as_slice()).unwrap(), o);
    }

    #[test]
    fn durum_zamanin_saf_fonksiyonu() {
        let rise = crate::shaking::RISE_STEP_S;
        let mut class_times = [f64::INFINITY; 10];
        for (k, tk) in class_times
            .iter_mut()
            .enumerate()
            .take(Shindo::S5Upper as usize + 1)
            .skip(1)
        {
            *tk = 10.0 + (k - 1) as f64 * rise;
        }
        let r = RegionSummary {
            id: 1,
            max: Shindo::S5Upper,
            t_first_s: 10.0,
            t_peak: class_times[Shindo::S5Upper as usize],
            cells: 1,
            built_cells: 1,
            at: (0.0, 0.0),
            t_end: 25.0,
            class_times,
        };
        assert_eq!(r.state(5.0, Some(8.0), None), RegionState::Quiet);
        assert_eq!(r.state(9.0, Some(8.0), None), RegionState::Warning);
        assert_eq!(r.state(9.0, None, None), RegionState::Quiet);
        assert_eq!(
            r.state(10.0, Some(8.0), None),
            RegionState::Shaking(Shindo::S1)
        );
        assert_eq!(
            r.state(10.0 + rise, Some(8.0), None),
            RegionState::Shaking(Shindo::S2)
        );
        assert_eq!(
            r.state(30.0, Some(8.0), None),
            RegionState::Shaking(Shindo::S5Upper)
        );
        assert_eq!(
            r.state(30.0, Some(8.0), Some(25.0)),
            RegionState::Settled(Shindo::S5Upper)
        );
    }
}
