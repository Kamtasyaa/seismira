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

/// Noktanın ülkesi (ISO 3166-1 alfa-2, bölge kodunun ilk iki harfi); nokta
/// denizdeyse en yakın karanın ülkesi, `max_km` içinde 5 km'lik halkalarla
/// (24 yön) aranır. Bulunamazsa `None`. Kabuk depremlerinin bölgesel yer
/// hareketi düzeltmeleri için ([`crate::gmpe::CrustRegion`]).
pub fn country_near<B: AsRef<[u8]>>(
    map: &RegionMap<B>,
    infos: &[RegionInfo],
    lonlat: (f64, f64),
    max_km: f64,
) -> Option<String> {
    let at = |p: (f64, f64)| -> Option<String> {
        let id = map.id_at(p.0, p.1).ok()?;
        let info = infos.get(usize::from(id).checked_sub(1)?)?;
        let code = info.iso.get(..2)?;
        code.chars()
            .all(|c| c.is_ascii_uppercase())
            .then(|| code.to_string())
    };
    if let Some(c) = at(lonlat) {
        return Some(c);
    }
    let mut r = 5.0;
    while r <= max_km {
        for k in 0..24 {
            if let Some(c) = at(crate::geo::destination(lonlat, f64::from(k) * 15.0, r)) {
                return Some(c);
            }
        }
        r += 5.0;
    }
    None
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
    /// Bölgedeki ilk P varışı (s).
    pub t_first_p: f64,
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
    /// Yaygın Shindo: seçilen hücrelerin en az [`TYPICAL_FRACTION`]'ının
    /// ulaştığı en yüksek sınıf. Bölge dolgusu ve sınırı bununla boyanır
    /// (bildirim en yüksek değeri yazar; kullanıcı kararı 2026-11-15).
    pub typical: Shindo,
    /// `typical_times[k]`: yaygın Shindo'nun k'ya ulaştığı an (kademeli
    /// yükselişle; seçilen hücrelerin %10'u k'ya ulaştığında).
    pub typical_times: [f64; 10],
    /// `class_times[k]`: bölgede gösterilen en yüksek Shindo'nun k'ya
    /// ulaştığı an (kademeli yükselişle; hiç ulaşmazsa sonsuz).
    pub class_times: [f64; 10],
    /// Seçilen hücrelerdeki en yüksek medyan PGV (cm/s, büyük bileşen;
    /// [`ShakeGrid::pgv`]). Arayüz, MMI ölçeğinde bölgenin en yüksek
    /// değerini bundan hesaplar (Shindo ve MMI ikisi de PGV'den).
    pub pgv_max: f32,
    /// Yaygın PGV: seçilen hücrelerin en az [`TYPICAL_FRACTION`]'ının
    /// ulaştığı en yüksek PGV (cm/s); `typical`'ın MMI karşılığı için.
    pub pgv_typical: f32,
}

/// Yaygın Shindo eşiği: bölgedeki seçilen (yerleşim) hücrelerinin oranı.
/// Seçim (görsel): birkaç hücrelik yerel uç değer bütün ili boyamasın,
/// ama ilin onda birini kaplayan güçlü sarsıntı görünsün.
pub const TYPICAL_FRACTION: f64 = 0.10;

/// Bölgenin görünür durumu (Bölüm 9); simülasyon zamanının saf fonksiyonu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegionState {
    Quiet,
    /// Uyarı yayımlandı, P bölgeye ulaştı, S henüz ulaşmadı.
    Warning,
    /// Sarsıntı sürüyor; o ana kadar ulaşılan YAYGIN Shindo (en az 1).
    Shaking(Shindo),
    /// Sarsıntı bitti; nihai yaygın Shindo.
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

    /// `t` anında bölgenin yaygın Shindo'su (1 ve üstü).
    pub fn typical_at(&self, t: f64) -> Option<Shindo> {
        (1..10)
            .rev()
            .find(|&k| self.typical_times[k] <= t)
            .map(|k| Shindo::ALL[k])
    }

    /// Bölgenin durumu. Sarsıntı, bölgede herhangi bir hücrede başladığında
    /// başlar; gösterilen değer yaygın Shindo'dur. `t_alert`: uyarı
    /// yayımlandıysa ve bölge kapsamdaysa uyarı anı. `t_end`: bölgede
    /// sarsıntının bittiği an (bilinmiyorsa `None`: sarsıntı sürer).
    pub fn state(&self, t: f64, t_alert: Option<f64>, t_end: Option<f64>) -> RegionState {
        if self.shindo_at(t).is_some() {
            return if t_end.is_some_and(|e| t >= e) {
                RegionState::Settled(self.typical)
            } else {
                RegionState::Shaking(self.typical_at(t).unwrap_or(Shindo::S1))
            };
        }
        // Uyarı yalnızca P dalgası bölgeye ulaştıktan sonra görünür
        // (kullanıcı kararı 2026-11-15: uzak bölgeler gereksiz yanmasın).
        if t_alert.is_some_and(|a| t >= a) && t >= self.t_first_p {
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
    // Bölge başına: (tüm hücreler, yerleşim hücreleri) özetleri.
    let mut acc: HashMap<RegionId, (RegionSummary, Option<RegionSummary>)> = HashMap::new();
    let update = |e: &mut RegionSummary,
                  s: Shindo,
                  (tp, ts, rise): (f64, f64, f64),
                  peak: f64,
                  end: f64,
                  at: (f64, f64),
                  pgv: f32| {
        e.t_end = e.t_end.max(end);
        if pgv.is_finite() {
            e.pgv_max = e.pgv_max.max(pgv);
        }
        e.t_first_p = e.t_first_p.min(tp);
        for (k, slot) in e
            .class_times
            .iter_mut()
            .enumerate()
            .take(s as usize + 1)
            .skip(1)
        {
            *slot = slot.min(ts + (k - 1) as f64 * rise);
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
            let tp = f64::from(grid.t_p[i]);
            let rise = f64::from(grid.rise[i]);
            let s = Shindo::ALL[class as usize];
            let peak = ts + f64::from(class.saturating_sub(Shindo::S1 as u8)) * rise;
            let at = grid.spec.center(col, row);
            let end = f64::from(grid.t_end[i]);
            let pgv = grid.pgv[i];
            let fresh = RegionSummary {
                t_end: f64::NEG_INFINITY,
                t_first_p: tp,
                id,
                max: s,
                t_first_s: ts,
                t_peak: peak,
                cells: 0,
                built_cells: 0,
                at,
                class_times: [f64::INFINITY; 10],
                typical: Shindo::S0,
                typical_times: [f64::INFINITY; 10],
                pgv_max: 0.0,
                pgv_typical: 0.0,
            };
            let e = acc.entry(id).or_insert((fresh, None));
            update(&mut e.0, s, (tp, ts, rise), peak, end, at, pgv);
            if raw & BUILT_BIT != 0 {
                update(
                    e.1.get_or_insert(fresh),
                    s,
                    (tp, ts, rise),
                    peak,
                    end,
                    at,
                    pgv,
                );
            }
        }
    }
    let mut out: Vec<_> = acc
        .into_values()
        .map(|(all, built)| match built {
            Some(b) => RegionSummary {
                t_first_p: all.t_first_p,
                t_first_s: all.t_first_s,
                cells: all.cells,
                built_cells: b.cells,
                ..b
            },
            None => all,
        })
        .collect();
    out.sort_by_key(|r| r.id);

    // Yaygın Shindo: ikinci geçiş, her bölgenin seçilen hücre kümesinde
    // (yerleşim hücresi varsa yalnızca onlar) sınıf başına ulaşma anları.
    let index: HashMap<RegionId, usize> = out.iter().enumerate().map(|(k, r)| (r.id, k)).collect();
    let mut times: Vec<[Vec<f64>; 10]> = vec![Default::default(); out.len()];
    let mut pgvs: Vec<Vec<f32>> = vec![Vec::new(); out.len()];
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
            let Some(&k) = index.get(&(raw & ID_MASK)) else {
                continue;
            };
            if out[k].built_cells > 0 && raw & BUILT_BIT == 0 {
                continue;
            }
            let ts = f64::from(grid.t_s[i]);
            let rise = f64::from(grid.rise[i]);
            if grid.pgv[i].is_finite() {
                pgvs[k].push(grid.pgv[i]);
            }
            for (c, list) in times[k]
                .iter_mut()
                .enumerate()
                .take(class as usize + 1)
                .skip(1)
            {
                list.push(ts + (c - 1) as f64 * rise);
            }
        }
    }
    for ((r, mut t), mut pgv) in out.iter_mut().zip(times).zip(pgvs) {
        let n = if r.built_cells > 0 {
            r.built_cells
        } else {
            r.cells
        };
        let need = ((f64::from(n) * TYPICAL_FRACTION).ceil() as usize).max(1);
        for (c, list) in t.iter_mut().enumerate().skip(1) {
            if list.len() >= need {
                list.sort_by(f64::total_cmp);
                r.typical_times[c] = list[need - 1];
                r.typical = Shindo::ALL[c];
            }
        }
        if pgv.len() >= need {
            pgv.sort_by(|a, b| b.total_cmp(a));
            r.pgv_typical = pgv[need - 1];
        }
    }
    out
}

/// Bir sınır halkası (Web Mercator birim karesinde, kapalı).
#[derive(Clone, Debug, PartialEq)]
pub struct Ring {
    /// Dış halka değil, delik (ör. Lesotho'yu çevreleyen Güney Afrika
    /// bölgelerinde).
    pub hole: bool,
    pub points: Vec<[f32; 2]>,
}

/// Bölge sınır halkaları (`.sout`): bölge `i` için `regions[i - 1]`.
///
/// Biçim: `"SOUT" u32 sürüm u32 bölge sayısı`, ardından tek bir zstd
/// çerçevesi: her bölge için `u32 halka sayısı`, her halka için
/// `u32 nokta sayısı` (en üst bit: delik) ve `[f32; 2] × n`.
pub type Outlines = Vec<Vec<Ring>>;

const OUT_MAGIC: &[u8; 4] = b"SOUT";
const OUT_VERSION: u32 = 2;
const HOLE_FLAG: u32 = 1 << 31;

/// Bölge dolgusu için üçgen ağı: halkalar dış halka + içindeki delikler
/// olarak gruplanır (delik, ilk noktasını içeren dış halkaya bağlanır) ve
/// earcut ile üçgenlenir. Dönen: köşeler ve üçgen indeksleri.
pub fn fill_mesh(rings: &[Ring]) -> (Vec<[f32; 2]>, Vec<u32>) {
    let outers: Vec<&Ring> = rings
        .iter()
        .filter(|r| !r.hole && r.points.len() >= 3)
        .collect();
    let mut groups: Vec<Vec<&Ring>> = outers.iter().map(|o| vec![*o]).collect();
    for h in rings.iter().filter(|r| r.hole && r.points.len() >= 3) {
        let p = h.points[0];
        if let Some(i) = outers.iter().position(|o| point_in_ring(&o.points, p)) {
            groups[i].push(h);
        }
    }
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut earcut = earcut::Earcut::new();
    let mut tris: Vec<u32> = Vec::new();
    for g in groups {
        let mut flat: Vec<[f64; 2]> = Vec::new();
        let mut holes: Vec<u32> = Vec::new();
        for (k, r) in g.iter().enumerate() {
            if k > 0 {
                holes.push(flat.len() as u32);
            }
            // Kapalı halkanın tekrar eden son noktası earcut'a verilmez.
            let pts = match r.points.as_slice() {
                [first, .., last] if first == last => &r.points[..r.points.len() - 1],
                all => all,
            };
            flat.extend(pts.iter().map(|p| [f64::from(p[0]), f64::from(p[1])]));
        }
        earcut.earcut(flat.iter().copied(), &holes, &mut tris);
        let base = vertices.len() as u32;
        vertices.extend(flat.iter().map(|p| [p[0] as f32, p[1] as f32]));
        indices.extend(tris.iter().map(|&i| base + i));
    }
    (vertices, indices)
}

/// Çift-tek kuralıyla nokta halka içinde mi?
pub(crate) fn point_in_ring(ring: &[[f32; 2]], p: [f32; 2]) -> bool {
    let mut inside = false;
    let mut j = ring.len() - 1;
    for i in 0..ring.len() {
        let (a, b) = (ring[i], ring[j]);
        if (a[1] > p[1]) != (b[1] > p[1])
            && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0]
        {
            inside = !inside;
        }
        j = i;
    }
    inside
}

pub fn write_outlines(
    outlines: &Outlines,
    w: &mut impl Write,
    compress: impl FnOnce(&[u8]) -> io::Result<Vec<u8>>,
) -> io::Result<()> {
    let mut raw = Vec::new();
    for rings in outlines {
        raw.extend_from_slice(&(rings.len() as u32).to_le_bytes());
        for ring in rings {
            let n = ring.points.len() as u32 | if ring.hole { HOLE_FLAG } else { 0 };
            raw.extend_from_slice(&n.to_le_bytes());
            for p in &ring.points {
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
            let head = u32_at(&raw)?;
            let n_pts = (head & !HOLE_FLAG) as usize;
            let mut points = Vec::with_capacity(n_pts);
            for _ in 0..n_pts {
                let x = f32::from_bits(u32_at(&raw)?);
                let y = f32::from_bits(u32_at(&raw)?);
                points.push([x, y]);
            }
            rings.push(Ring {
                hole: head & HOLE_FLAG != 0,
                points,
            });
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
            t_p: vec![5.0, 6.0, 4.0, f32::NAN],
            t_s: vec![10.0, 12.0, 8.0, f32::NAN],
            t_end: vec![40.0, 30.0, 50.0, f32::NAN],
            rise: vec![crate::shaking::RISE_STEP_S as f32; 4],
            pgv: vec![f32::NAN; 4],
            pga: vec![f32::NAN; 4],
            pgv_tau: vec![f32::NAN; 4],
            pga_tau: vec![f32::NAN; 4],
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
        assert_eq!(s[0].t_first_p, 4.0);
        // 2 hücre: %10 → 1 hücre yeter; yaygın = en yüksek.
        assert_eq!(s[0].typical, Shindo::S6Lower);
        // 6- = S1'den 6 adım: 8 + 6·0,4.
        assert!((s[0].t_peak - (8.0 + 6.0 * crate::shaking::RISE_STEP_S)).abs() < 1e-6);
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
        let ring = |hole, points: Vec<[f32; 2]>| Ring { hole, points };
        let o: Outlines = vec![
            vec![ring(false, vec![[0.1, 0.2], [0.3, 0.4], [0.1, 0.2]])],
            vec![],
            vec![
                ring(false, vec![[0.5, 0.5]]),
                ring(true, vec![[0.6, 0.6], [0.7, 0.7]]),
            ],
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
    fn delikli_dolgu_agi() {
        // 4×4 kare, ortasında 2×2 delik: alan 16 − 4 = 12.
        let sq = |x0: f32, y0: f32, s: f32| {
            vec![
                [x0, y0],
                [x0 + s, y0],
                [x0 + s, y0 + s],
                [x0, y0 + s],
                [x0, y0],
            ]
        };
        let rings = vec![
            Ring {
                hole: false,
                points: sq(0.0, 0.0, 4.0),
            },
            Ring {
                hole: true,
                points: sq(1.0, 1.0, 2.0),
            },
            // Başka bir dış halka (ada): alan 1.
            Ring {
                hole: false,
                points: sq(10.0, 0.0, 1.0),
            },
        ];
        let (v, idx) = fill_mesh(&rings);
        let area: f32 = idx
            .chunks(3)
            .map(|t| {
                let (a, b, c) = (v[t[0] as usize], v[t[1] as usize], v[t[2] as usize]);
                ((b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1])).abs() / 2.0
            })
            .sum();
        assert!((area - 13.0).abs() < 1e-4, "{area}");
    }

    #[test]
    fn yaygin_deger_birkac_uc_hucreyi_yok_sayar() {
        // 1×20 ızgara, hepsi bölge 1 ve yerleşim: 19 hücre 4, 1 hücre 6-.
        let n = 20;
        let domain = Domain {
            base: Base::GLOBAL_30S,
            col0: 0,
            row0: 0,
            w: n,
            h: 1,
        };
        let mut class = vec![Shindo::S4 as u8; n as usize];
        class[0] = Shindo::S6Lower as u8;
        let grid = ShakeGrid {
            spec: GridSpec {
                west: 0.0,
                north: 0.0,
                cell: 1.0,
                width: n,
                height: 1,
            },
            class,
            jma: vec![4.0; n as usize],
            t_p: vec![1.0; n as usize],
            t_s: (0..n).map(|i| 10.0 + i as f32).collect(),
            t_end: vec![60.0; n as usize],
            rise: vec![crate::shaking::RISE_STEP_S as f32; n as usize],
            pgv: (0..n).map(|i| 100.0 - i as f32).collect(),
            pga: vec![f32::NAN; n as usize],
            pgv_tau: vec![f32::NAN; n as usize],
            pga_tau: vec![f32::NAN; n as usize],
        };
        let ids = vec![1 | BUILT_BIT; n as usize];
        let r = summarize(&grid, &domain, 1, &ids)[0];
        assert_eq!(r.max, Shindo::S6Lower);
        assert_eq!(r.typical, Shindo::S4);
        // PGV: en yüksek 100; yaygın = %10'un (2 hücre) ulaştığı 99.
        assert_eq!(r.pgv_max, 100.0);
        assert_eq!(r.pgv_typical, 99.0);
        // %10 = 2 hücre: 4'e ulaşan 2. hücre (S varışı 11 s) + 3 adım.
        let rise = crate::shaking::RISE_STEP_S;
        assert!((r.typical_times[Shindo::S4 as usize] - (11.0 + 3.0 * rise)).abs() < 1e-6);
        assert_eq!(
            r.state(100.0, None, Some(60.0)),
            RegionState::Settled(Shindo::S4)
        );
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
            t_first_p: 6.0,
            t_first_s: 10.0,
            t_peak: class_times[Shindo::S5Upper as usize],
            cells: 1,
            built_cells: 1,
            at: (0.0, 0.0),
            t_end: 25.0,
            class_times,
            typical: Shindo::S5Upper,
            typical_times: class_times,
            pgv_max: 0.0,
            pgv_typical: 0.0,
        };
        assert_eq!(r.state(5.0, Some(8.0), None), RegionState::Quiet);
        // Uyarı yayımlandı ama P henüz ulaşmadı → sessiz.
        assert_eq!(r.state(5.5, Some(4.0), None), RegionState::Quiet);
        assert_eq!(r.state(6.0, Some(4.0), None), RegionState::Warning);
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
