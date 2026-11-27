//! Kritik tesisler (README "Önemli"): hastane, köprü / viyadük, baraj, trafo
//! merkezi, doğalgaz boru hattı ve her birinde tahmini yer hareketi.
//!
//! Veri: OpenStreetMap (© OpenStreetMap contributors, ODbL 1.0), AYRI dosya
//! (`data/infra/infra.sinf`; ODbL verisi başka verilerle tek veri tabanında
//! birleştirilmez, CLAUDE.md Bölüm 12), ana veri paketinin içinde (kullanıcı
//! kararı 2026-11-27). Üretim: `scripts/gen_infra.py` → `seismira-data infra`.
//!
//! Biçim (`.sinf`, küçük sonlu): "SINF", sürüm u32, döşeme boyu (derece)
//! f32; ardından 360/boy × 180/boy döşeme için dizin (ofset u64, sıkıştırılmış
//! uzunluk u32, açık uzunluk u32; satır satır, kuzeyden, batıdan); sonra
//! zstd ile sıkıştırılmış döşeme gövdeleri. Gövde: sayı u32; her tesis:
//! tür u8, boylam f32, enlem f32, öznitelik u16, ad uzunluğu u8, ad (UTF-8),
//! köşe sayısı u16, köşeler (boylam f32, enlem f32). Tesis, ilk köşesinin /
//! konumunun döşemesindedir; çizgiler ≤ 50 km parçalardır, sorgu alanı buna
//! göre genişletilir ([`LINE_MARGIN_DEG`]).

use crate::shaking::Scenario;
use crate::site::Site;
use std::io::{self, Write};

const MAGIC: &[u8; 4] = b"SINF";
const VERSION: u32 = 1;
/// Çizgi parçalarının en uzun boyu (≤ 50 km) için sorgu payı (derece).
pub const LINE_MARGIN_DEG: f64 = 0.6;

/// Tesis türü.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Kind {
    Hospital = 0,
    Bridge = 1,
    Viaduct = 2,
    Dam = 3,
    Substation = 4,
    GasPipeline = 5,
}

impl Kind {
    pub const ALL: [Kind; 6] = [
        Kind::Hospital,
        Kind::Bridge,
        Kind::Viaduct,
        Kind::Dam,
        Kind::Substation,
        Kind::GasPipeline,
    ];

    /// Ara TSV'deki ad (`scripts/gen_infra.py`).
    pub fn parse(s: &str) -> Option<Kind> {
        Some(match s {
            "hospital" => Kind::Hospital,
            "bridge" => Kind::Bridge,
            "viaduct" => Kind::Viaduct,
            "dam" => Kind::Dam,
            "substation" => Kind::Substation,
            "gas" => Kind::GasPipeline,
            _ => return None,
        })
    }

    /// Kısa kod (dışa aktarma; arayüz metni değil).
    pub fn code(self) -> &'static str {
        match self {
            Kind::Hospital => "hospital",
            Kind::Bridge => "bridge",
            Kind::Viaduct => "viaduct",
            Kind::Dam => "dam",
            Kind::Substation => "substation",
            Kind::GasPipeline => "gas",
        }
    }

    fn from_u8(v: u8) -> Option<Kind> {
        Kind::ALL.get(usize::from(v)).copied()
    }
}

/// Bir tesis.
#[derive(Clone, Debug, PartialEq)]
pub struct Facility {
    pub kind: Kind,
    /// Temsil noktası (boylam, enlem): alan / çizgide sınır kutusu merkezi,
    /// köprüde orta düğüm, boru hattı parçasında ilk köşe.
    pub lonlat: (f64, f64),
    /// Köprü / viyadük: uzunluk (m); trafo: en yüksek gerilim (kV); diğer: 0.
    pub attr: u16,
    pub name: String,
    /// Boru hattı parçasının köşeleri; noktasal tesislerde boş.
    pub line: Vec<(f32, f32)>,
}

/// Döşeme dizini kaydı.
#[derive(Clone, Copy, Debug, Default)]
struct Entry {
    offset: u64,
    len: u32,
    raw_len: u32,
}

/// Bellekteki ya da belleğe eşlenmiş `.sinf` dosyası; yalnızca sorulan
/// döşemeler açılır (CLAUDE.md Bölüm 11).
pub struct InfraFile<B> {
    bytes: B,
    tile_deg: f64,
    cols: usize,
    rows: usize,
    index: Vec<Entry>,
}

impl<B: AsRef<[u8]>> InfraFile<B> {
    pub fn open(bytes: B) -> io::Result<Self> {
        let b = bytes.as_ref();
        if b.len() < 12 || &b[..4] != MAGIC {
            return Err(bad("SINF dosyası değil"));
        }
        let version = u32::from_le_bytes(b[4..8].try_into().unwrap_or_default());
        if version != VERSION {
            return Err(bad("desteklenmeyen SINF sürümü"));
        }
        let tile_deg = f64::from(f32::from_le_bytes(b[8..12].try_into().unwrap_or_default()));
        if !(tile_deg > 0.0 && (360.0 / tile_deg).fract() == 0.0) {
            return Err(bad("döşeme boyu geçersiz"));
        }
        let cols = (360.0 / tile_deg) as usize;
        let rows = (180.0 / tile_deg) as usize;
        let mut index = Vec::with_capacity(cols * rows);
        let mut p = 12;
        for _ in 0..cols * rows {
            let e = b.get(p..p + 16).ok_or_else(|| bad("dizin kesik"))?;
            index.push(Entry {
                offset: u64::from_le_bytes(e[..8].try_into().unwrap_or_default()),
                len: u32::from_le_bytes(e[8..12].try_into().unwrap_or_default()),
                raw_len: u32::from_le_bytes(e[12..16].try_into().unwrap_or_default()),
            });
            p += 16;
        }
        Ok(Self {
            bytes,
            tile_deg,
            cols,
            rows,
            index,
        })
    }

    /// Toplam tesis sayısı tahmini için döşeme sayısı (bilgi).
    pub fn tile_count(&self) -> usize {
        self.index.iter().filter(|e| e.len > 0).count()
    }

    fn tile(&self, col: usize, row: usize) -> io::Result<Vec<Facility>> {
        let e = self.index[row * self.cols + col];
        if e.len == 0 {
            return Ok(Vec::new());
        }
        let b = self.bytes.as_ref();
        let src = b
            .get(e.offset as usize..e.offset as usize + e.len as usize)
            .ok_or_else(|| bad("döşeme dosya dışında"))?;
        let raw = crate::raster::decompress(src, e.raw_len as usize)?;
        parse_tile(&raw)
    }

    /// Kutudaki tesisler (batı, güney, doğu, kuzey; derece). Batı > doğu ise
    /// 180° meridyenini aşan kutu. Çizgi parçaları için kutu
    /// [`LINE_MARGIN_DEG`] genişletilerek okunur; dönen noktasal tesisler
    /// kutunun içindedir, çizgiler ise en az bir köşesi içeride olanlardır.
    pub fn query(&self, west: f64, south: f64, east: f64, north: f64) -> io::Result<Vec<Facility>> {
        let ranges: Vec<(f64, f64)> = if west <= east {
            vec![(west, east)]
        } else {
            vec![(west, 180.0), (-180.0, east)]
        };
        let m = LINE_MARGIN_DEG;
        let row_of = |lat: f64| {
            (((90.0 - lat) / self.tile_deg).floor().max(0.0) as usize).min(self.rows - 1)
        };
        let col_of = |lon: f64| {
            (((lon + 180.0) / self.tile_deg).floor().max(0.0) as usize).min(self.cols - 1)
        };
        let inside = |p: (f64, f64)| {
            p.1 >= south && p.1 <= north && ranges.iter().any(|&(w, e)| p.0 >= w && p.0 <= e)
        };
        let mut out = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for &(w, e) in &ranges {
            for row in row_of(north + m)..=row_of(south - m) {
                for col in col_of(w - m)..=col_of(e + m) {
                    if !seen.insert((col, row)) {
                        continue;
                    }
                    for f in self.tile(col, row)? {
                        let keep = if f.line.is_empty() {
                            inside(f.lonlat)
                        } else {
                            f.line
                                .iter()
                                .any(|&(x, y)| inside((f64::from(x), f64::from(y))))
                        };
                        if keep {
                            out.push(f);
                        }
                    }
                }
            }
        }
        Ok(out)
    }
}

fn parse_tile(b: &[u8]) -> io::Result<Vec<Facility>> {
    let mut p = 0usize;
    let mut take = |n: usize| -> io::Result<&[u8]> {
        let s = b.get(p..p + n).ok_or_else(|| bad("döşeme kesik"))?;
        p += n;
        Ok(s)
    };
    let n = u32::from_le_bytes(take(4)?.try_into().unwrap_or_default()) as usize;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        let kind = Kind::from_u8(take(1)?[0]).ok_or_else(|| bad("tesis türü"))?;
        let f32_ = |s: &[u8]| f32::from_le_bytes(s.try_into().unwrap_or_default());
        let lon = f32_(take(4)?);
        let lat = f32_(take(4)?);
        let attr = u16::from_le_bytes(take(2)?.try_into().unwrap_or_default());
        let nl = usize::from(take(1)?[0]);
        let name = String::from_utf8_lossy(take(nl)?).into_owned();
        let nv = usize::from(u16::from_le_bytes(take(2)?.try_into().unwrap_or_default()));
        let mut line = Vec::with_capacity(nv);
        for _ in 0..nv {
            let x = f32_(take(4)?);
            let y = f32_(take(4)?);
            line.push((x, y));
        }
        out.push(Facility {
            kind,
            lonlat: (f64::from(lon), f64::from(lat)),
            attr,
            name,
            line,
        });
    }
    Ok(out)
}

/// Dosyayı yazar. `compress`: döşeme gövdesini zstd ile sıkıştıran işlev
/// (veri aracı yüksek düzey, testler ruzstd kullanır). Ad 255 baytta, köşe
/// sayısı 65 535'te kesilir.
pub fn write(
    facilities: &[Facility],
    tile_deg: f64,
    compress: impl Fn(&[u8]) -> io::Result<Vec<u8>>,
    w: &mut impl Write,
) -> io::Result<()> {
    let cols = (360.0 / tile_deg) as usize;
    let rows = (180.0 / tile_deg) as usize;
    let mut tiles: Vec<Vec<&Facility>> = vec![Vec::new(); cols * rows];
    for f in facilities {
        let (lon, lat) = f.lonlat;
        let col = (((lon + 180.0) / tile_deg).floor().max(0.0) as usize).min(cols - 1);
        let row = (((90.0 - lat) / tile_deg).floor().max(0.0) as usize).min(rows - 1);
        tiles[row * cols + col].push(f);
    }
    let mut bodies = Vec::with_capacity(tiles.len());
    for t in &tiles {
        if t.is_empty() {
            bodies.push((Vec::new(), 0u32));
            continue;
        }
        let mut raw = Vec::new();
        raw.extend_from_slice(&(t.len() as u32).to_le_bytes());
        for f in t {
            raw.push(f.kind as u8);
            raw.extend_from_slice(&(f.lonlat.0 as f32).to_le_bytes());
            raw.extend_from_slice(&(f.lonlat.1 as f32).to_le_bytes());
            raw.extend_from_slice(&f.attr.to_le_bytes());
            let mut name = f.name.as_bytes();
            if name.len() > 255 {
                let mut cut = 255;
                while !f.name.is_char_boundary(cut) {
                    cut -= 1;
                }
                name = &name[..cut];
            }
            raw.push(name.len() as u8);
            raw.extend_from_slice(name);
            let nv = f.line.len().min(usize::from(u16::MAX));
            raw.extend_from_slice(&(nv as u16).to_le_bytes());
            for &(x, y) in &f.line[..nv] {
                raw.extend_from_slice(&x.to_le_bytes());
                raw.extend_from_slice(&y.to_le_bytes());
            }
        }
        let raw_len = raw.len() as u32;
        bodies.push((compress(&raw)?, raw_len));
    }
    w.write_all(MAGIC)?;
    w.write_all(&VERSION.to_le_bytes())?;
    w.write_all(&(tile_deg as f32).to_le_bytes())?;
    let mut offset = (12 + 16 * tiles.len()) as u64;
    for (body, raw_len) in &bodies {
        w.write_all(&offset.to_le_bytes())?;
        w.write_all(&(body.len() as u32).to_le_bytes())?;
        w.write_all(&raw_len.to_le_bytes())?;
        offset += body.len() as u64;
    }
    for (body, _) in &bodies {
        w.write_all(body)?;
    }
    Ok(())
}

/// Bir tesiste tahmini yer hareketi.
#[derive(Clone, Debug, PartialEq)]
pub struct Assessed {
    /// Sorgu sonucundaki tesisin sırası.
    pub index: usize,
    /// Değerlendirilen nokta (boru hattında en güçlü sarsılan köşe).
    pub at: (f64, f64),
    pub rrup: f64,
    /// Medyan PGA (g) ve PGV (cm/s), RotD50.
    pub pga_g: f64,
    pub pgv: f64,
    /// JMA aletsel şiddeti ve MMI (medyan).
    pub jma: f64,
    pub mmi: f64,
    /// Vs30 (hesapta kullanılan, modele çekilmiş).
    pub vs30: f64,
    /// İlk S varışı (s); haritada işaret bu anda belirir.
    pub t_s: Option<f64>,
}

/// Tesislerde tahmin: noktasal tesislerde temsil noktası, boru hattında
/// tüm köşeler (en yüksek PGV'li köşe raporlanır). Model dışı (mesafe,
/// ortam) tesisler atlanır. Aynı hattın komşu parçaları ortak köşelerinde
/// en güçlü sarsılırsa aynı ad ve noktayla tek kayıt kalır (2026-11-27,
/// CSV'de çift satır). `site`: zemin (uygulamada kullanıcı Vs30 → harita →
/// referans). Paralel (rayon).
pub fn assess(
    sc: &Scenario,
    facilities: &[Facility],
    site: impl Fn((f64, f64)) -> Site + Sync,
) -> Vec<Assessed> {
    use rayon::prelude::*;
    let mut seen = std::collections::HashSet::new();
    facilities
        .par_iter()
        .enumerate()
        .filter_map(|(index, f)| {
            let pts: Vec<(f64, f64)> = if f.line.is_empty() {
                vec![f.lonlat]
            } else {
                f.line
                    .iter()
                    .map(|&(x, y)| (f64::from(x), f64::from(y)))
                    .collect()
            };
            pts.into_iter()
                .filter_map(|p| {
                    let r = sc.site(p, || site(p));
                    let m = r.motion?;
                    Some(Assessed {
                        index,
                        at: p,
                        rrup: r.dist.rrup,
                        pga_g: m.pga.median(),
                        pgv: m.pgv.median(),
                        jma: m.jma,
                        mmi: m.mmi,
                        vs30: r.vs30_used,
                        t_s: r.t_s,
                    })
                })
                .max_by(|a, b| a.pgv.total_cmp(&b.pgv))
        })
        .collect::<Vec<_>>()
        .into_iter()
        .filter(|a| {
            let f = &facilities[a.index];
            f.line.is_empty() || seen.insert((f.name.clone(), a.at.0.to_bits(), a.at.1.to_bits()))
        })
        .collect()
}

/// Depremin tesis sorgu kutusu (batı, güney, doğu, kuzey): modelin mesafe
/// sınırı eğrisinin ([`crate::rupture::Rupture::rrup_contour`]) sınır
/// kutusu. 180° meridyenini aşarsa batı > doğu.
pub fn query_box(sc: &Scenario, max_rrup_km: f64) -> (f64, f64, f64, f64) {
    let pts = sc.fault.rrup_contour(max_rrup_km, 144);
    let lon0 = sc.quake.epicenter.0;
    // Boylamlar merkez üssüne göre açılır (meridyen aşımı).
    let rel = |x: f64| (x - lon0 + 540.0).rem_euclid(360.0) - 180.0;
    let (mut w, mut e, mut s, mut n) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
    for &(x, y) in &pts {
        let dx = rel(x);
        w = w.min(dx);
        e = e.max(dx);
        s = s.min(y);
        n = n.max(y);
    }
    let wrap = |x: f64| (x + 540.0).rem_euclid(360.0) - 180.0;
    (wrap(lon0 + w), s.max(-90.0), wrap(lon0 + e), n.min(90.0))
}

fn bad(msg: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zstd(b: &[u8]) -> io::Result<Vec<u8>> {
        Ok(ruzstd::encoding::compress_to_vec(
            b,
            ruzstd::encoding::CompressionLevel::Fastest,
        ))
    }

    fn sample() -> Vec<Facility> {
        let f = |kind, lon, lat, attr, name: &str| Facility {
            kind,
            lonlat: (lon, lat),
            attr,
            name: name.into(),
            line: Vec::new(),
        };
        vec![
            f(Kind::Hospital, 36.16, 36.20, 0, "Antakya Devlet Hastanesi"),
            f(Kind::Bridge, 37.10, 37.30, 120, "O-52"),
            f(Kind::Substation, 179.9, -17.0, 132, ""),
            f(Kind::Dam, -179.9, -17.1, 0, "Baraj"),
            Facility {
                kind: Kind::GasPipeline,
                lonlat: (35.95, 37.0),
                attr: 0,
                name: "BOTAŞ".into(),
                line: vec![(35.95, 37.0), (36.05, 37.0), (36.3, 37.05)],
            },
        ]
    }

    /// Yaz-oku; kutu sorgusu; 180° meridyeni; çizgi parçası kutunun
    /// dışında başlasa da içeri giren köşesiyle bulunur.
    #[test]
    fn yaz_oku_sorgu() {
        let mut buf = Vec::new();
        write(&sample(), 10.0, zstd, &mut buf).unwrap();
        let file = InfraFile::open(buf).unwrap();
        let all = file.query(-180.0, -90.0, 180.0, 90.0).unwrap();
        assert_eq!(all.len(), 5);
        let tr = file.query(36.0, 36.0, 38.0, 38.0).unwrap();
        let kinds: Vec<Kind> = tr.iter().map(|f| f.kind).collect();
        assert!(kinds.contains(&Kind::Hospital) && kinds.contains(&Kind::Bridge));
        assert!(kinds.contains(&Kind::GasPipeline), "{kinds:?}");
        let h = tr.iter().find(|f| f.kind == Kind::Hospital).unwrap();
        assert_eq!(h.name, "Antakya Devlet Hastanesi");
        assert!((h.lonlat.0 - 36.16).abs() < 1e-5);
        // 180° meridyenini aşan kutu.
        let fiji = file.query(179.0, -18.0, -179.0, -16.0).unwrap();
        assert_eq!(fiji.len(), 2);
        // Boş bölge.
        assert!(file.query(0.0, 0.0, 1.0, 1.0).unwrap().is_empty());
    }

    #[test]
    fn bozuk_dosya() {
        assert!(InfraFile::open(b"XXXX0000".to_vec()).is_err());
    }

    /// Değerlendirme: yakın tesis daha güçlü; boru hattında faya en yakın
    /// köşe seçilir; 300 km ötesi atlanır; sorgu kutusu fayı kapsar.
    #[test]
    fn degerlendirme() {
        let tt = crate::traveltime::TravelTimes::load(std::path::Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/traveltime/ak135.ttt"
        )))
        .unwrap();
        let q = crate::quake::Quake::new((36.5, 37.0));
        let sc = Scenario::new(q, &tt);
        let mk = |lon: f64, line: Vec<(f32, f32)>| Facility {
            kind: if line.is_empty() {
                Kind::Hospital
            } else {
                Kind::GasPipeline
            },
            lonlat: (lon, 37.0),
            attr: 0,
            name: String::new(),
            line,
        };
        let fs = vec![
            mk(36.6, vec![]),
            mk(37.5, vec![]),
            mk(42.0, vec![]), // ~490 km
            mk(37.2, vec![(37.2, 37.0), (36.55, 37.0), (37.4, 37.0)]),
            // Aynı hattın komşu parçası: en güçlü köşe ortak → tekrar sayılmaz.
            mk(36.55, vec![(36.55, 37.0), (36.0, 37.0)]),
        ];
        let a = assess(&sc, &fs, |_| Site::reference());
        assert_eq!(a.len(), 3, "{a:?}");
        let by = |i: usize| a.iter().find(|x| x.index == i).unwrap();
        assert!(by(0).pga_g > by(1).pga_g && by(0).jma > by(1).jma);
        assert!((by(3).at.0 - 36.55).abs() < 1e-6, "{:?}", by(3).at);
        let (w, s0, e, n) = query_box(&sc, 300.0);
        assert!(
            w < 33.5 && e > 39.5 && s0 < 34.5 && n > 39.5,
            "{w} {s0} {e} {n}"
        );
    }
}
