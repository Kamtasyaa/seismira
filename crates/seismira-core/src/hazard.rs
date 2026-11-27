//! Olasılıksal sismik tehlike haritaları: ESHM20 hazır sonuçları (README
//! "Olasılıksal tehlike"; kullanıcı kararı 2026-11-27: önce toplu senaryo,
//! sonra ESHM20; modeli yeniden hesaplamak yerine resmi haritalar,
//! docs/bilim.md "ESHM20").
//!
//! Kaynak: Danciu vd. (2021), The 2020 update of the European Seismic Hazard
//! Model (ESHM20), EFEHR Technical Report 001, doi:10.12686/a15; CC BY 4.0.
//! Değerler: referans kaya (Vs30 800 m/s), dallar üzerinden aritmetik
//! ortalama; g. Üretim: `scripts/fetch_eshm20.py` → `seismira-data hazard`.
//!
//! Biçim (`.shaz`, küçük sonlu): "SHAZ", sürüm u32; ızgara: batı boylamı
//! f64, güney enlemi f64, adım f64, nx u32, ny u32 (satır 0 = güney);
//! katman sayısı u32; her katman: periyot f32 (0 = PGA), yıllık aşılma
//! olasılığı f64, ofset u64, sıkıştırılmış uzunluk u32; sonra zstd ile
//! sıkıştırılmış katmanlar: nx·ny u16, değer = kod · [`SCALE_G`] g, 0 = veri
//! yok (deniz, model dışı). Katmanlar ilk kullanımda açılır.

use std::io::{self, Write};
use std::sync::OnceLock;

const MAGIC: &[u8; 4] = b"SHAZ";
const VERSION: u32 = 1;
/// Kodlama adımı (g): 0,0001 g; en büyük ~6,55 g.
pub const SCALE_G: f64 = 1e-4;

/// Bir katman: şiddet ölçüsü ve olasılık.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayerInfo {
    /// Spektral periyot (s); PGA için 0.
    pub period: f64,
    /// Yıllık aşılma olasılığı.
    pub annual_poe: f64,
}

impl LayerInfo {
    /// `years` yıl içinde aşılma olasılığı (Poisson): 1 − exp(−λ·t),
    /// λ = −ln(1 − p_yıllık).
    pub fn poe_in(&self, years: f64) -> f64 {
        let rate = -(1.0 - self.annual_poe).ln();
        1.0 - (-rate * years).exp()
    }

    /// Ortalama dönüş periyodu (yıl) ≈ 1 / λ.
    pub fn return_period(&self) -> f64 {
        1.0 / -(1.0 - self.annual_poe).ln()
    }
}

/// Izgara geometrisi.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grid {
    pub west: f64,
    pub south: f64,
    pub step: f64,
    pub nx: u32,
    pub ny: u32,
}

struct Layer {
    info: LayerInfo,
    offset: u64,
    len: u32,
    data: OnceLock<Vec<u16>>,
}

/// Tehlike haritası dosyası.
pub struct HazardMap<B> {
    bytes: B,
    pub grid: Grid,
    layers: Vec<Layer>,
}

impl<B: AsRef<[u8]>> HazardMap<B> {
    pub fn open(bytes: B) -> io::Result<Self> {
        let b = bytes.as_ref();
        let mut p = 0usize;
        let mut take = |n: usize| -> io::Result<&[u8]> {
            let s = b.get(p..p + n).ok_or_else(|| bad("SHAZ dosyası kesik"))?;
            p += n;
            Ok(s)
        };
        if take(4)? != MAGIC {
            return Err(bad("SHAZ dosyası değil"));
        }
        let u32_ = |s: &[u8]| u32::from_le_bytes(s.try_into().unwrap_or_default());
        let f64_ = |s: &[u8]| f64::from_le_bytes(s.try_into().unwrap_or_default());
        if u32_(take(4)?) != VERSION {
            return Err(bad("desteklenmeyen SHAZ sürümü"));
        }
        let grid = Grid {
            west: f64_(take(8)?),
            south: f64_(take(8)?),
            step: f64_(take(8)?),
            nx: u32_(take(4)?),
            ny: u32_(take(4)?),
        };
        if !(grid.step > 0.0 && grid.nx > 0 && grid.ny > 0) {
            return Err(bad("SHAZ ızgarası geçersiz"));
        }
        let n = u32_(take(4)?) as usize;
        let mut layers = Vec::with_capacity(n);
        for _ in 0..n {
            let f32_ = |s: &[u8]| f64::from(f32::from_le_bytes(s.try_into().unwrap_or_default()));
            let period = f32_(take(4)?);
            let annual_poe = f64_(take(8)?);
            let offset = u64::from_le_bytes(take(8)?.try_into().unwrap_or_default());
            let len = u32_(take(4)?);
            layers.push(Layer {
                info: LayerInfo { period, annual_poe },
                offset,
                len,
                data: OnceLock::new(),
            });
        }
        Ok(Self {
            bytes,
            grid,
            layers,
        })
    }

    pub fn layers(&self) -> Vec<LayerInfo> {
        self.layers.iter().map(|l| l.info).collect()
    }

    /// Katmanın açılmış kodları (ilk çağrıda açılır).
    pub fn codes(&self, layer: usize) -> io::Result<&[u16]> {
        let l = self.layers.get(layer).ok_or_else(|| bad("katman yok"))?;
        if let Some(d) = l.data.get() {
            return Ok(d);
        }
        let b = self.bytes.as_ref();
        let src = b
            .get(l.offset as usize..l.offset as usize + l.len as usize)
            .ok_or_else(|| bad("katman dosya dışında"))?;
        let n = self.grid.nx as usize * self.grid.ny as usize;
        let raw = crate::raster::decompress(src, n * 2)?;
        let codes: Vec<u16> = raw
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| u16::from_le_bytes(*c))
            .collect();
        Ok(l.data.get_or_init(|| codes))
    }

    /// Noktadaki değer (g): komşu dört düğümden çift doğrusal ara değer;
    /// biri eksikse (kıyı) mevcutların ağırlıklı ortalaması. Izgara dışı
    /// ya da dört düğüm de boşsa `None`.
    pub fn value(&self, layer: usize, lon: f64, lat: f64) -> Option<f64> {
        let codes = self.codes(layer).ok()?;
        let g = self.grid;
        let fx = (lon - g.west) / g.step;
        let fy = (lat - g.south) / g.step;
        if fx < 0.0 || fy < 0.0 || fx > f64::from(g.nx - 1) || fy > f64::from(g.ny - 1) {
            return None;
        }
        let (x0, y0) = (fx.floor() as usize, fy.floor() as usize);
        let (x1, y1) = (
            (x0 + 1).min(g.nx as usize - 1),
            (y0 + 1).min(g.ny as usize - 1),
        );
        let (tx, ty) = (fx - x0 as f64, fy - y0 as f64);
        let mut sum = 0.0;
        let mut wsum = 0.0;
        for (x, y, w) in [
            (x0, y0, (1.0 - tx) * (1.0 - ty)),
            (x1, y0, tx * (1.0 - ty)),
            (x0, y1, (1.0 - tx) * ty),
            (x1, y1, tx * ty),
        ] {
            let c = codes[y * g.nx as usize + x];
            if c > 0 && w > 0.0 {
                sum += w * f64::from(c) * SCALE_G;
                wsum += w;
            }
        }
        (wsum > 1e-9).then(|| sum / wsum)
    }

    /// Bir olasılıkta eş tehlike spektrumu (UHS): (periyot, g), periyoda göre
    /// artan; PGA periyot 0'da. Veri yoksa boş.
    pub fn uhs(&self, annual_poe: f64, lon: f64, lat: f64) -> Vec<(f64, f64)> {
        let mut out: Vec<(f64, f64)> = self
            .layers
            .iter()
            .enumerate()
            .filter(|(_, l)| (l.info.annual_poe - annual_poe).abs() < annual_poe * 1e-3)
            .filter_map(|(i, l)| Some((l.info.period, self.value(i, lon, lat)?)))
            .collect();
        out.sort_by(|a, b| a.0.total_cmp(&b.0));
        out
    }
}

/// Dosyayı yazar. `layers`: bilgi ve nx·ny değer (g; NaN = veri yok).
/// `compress`: zstd (veri aracı yüksek düzey, testler ruzstd).
pub fn write(
    grid: Grid,
    layers: &[(LayerInfo, Vec<f32>)],
    compress: impl Fn(&[u8]) -> io::Result<Vec<u8>>,
    w: &mut impl Write,
) -> io::Result<()> {
    let n = grid.nx as usize * grid.ny as usize;
    let mut bodies = Vec::with_capacity(layers.len());
    for (_, vals) in layers {
        if vals.len() != n {
            return Err(bad("katman boyutu ızgarayla uyuşmuyor"));
        }
        let mut raw = Vec::with_capacity(n * 2);
        for &v in vals {
            let c = if v.is_finite() && v > 0.0 {
                ((f64::from(v) / SCALE_G).round()).clamp(1.0, f64::from(u16::MAX)) as u16
            } else {
                0
            };
            raw.extend_from_slice(&c.to_le_bytes());
        }
        bodies.push(compress(&raw)?);
    }
    w.write_all(MAGIC)?;
    w.write_all(&VERSION.to_le_bytes())?;
    w.write_all(&grid.west.to_le_bytes())?;
    w.write_all(&grid.south.to_le_bytes())?;
    w.write_all(&grid.step.to_le_bytes())?;
    w.write_all(&grid.nx.to_le_bytes())?;
    w.write_all(&grid.ny.to_le_bytes())?;
    w.write_all(&(layers.len() as u32).to_le_bytes())?;
    let header = 4 + 4 + 8 * 3 + 4 + 4 + 4 + layers.len() * (4 + 8 + 8 + 4);
    let mut offset = header as u64;
    for ((info, _), body) in layers.iter().zip(&bodies) {
        w.write_all(&(info.period as f32).to_le_bytes())?;
        w.write_all(&info.annual_poe.to_le_bytes())?;
        w.write_all(&offset.to_le_bytes())?;
        w.write_all(&(body.len() as u32).to_le_bytes())?;
        offset += body.len() as u64;
    }
    for b in &bodies {
        w.write_all(b)?;
    }
    Ok(())
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

    /// 50 yılda %10 ↔ yıllık 0,002103 ↔ 475 yıl (ESHM20 / EC8 uzlaşımı).
    #[test]
    fn olasilik_donusumu() {
        let l = LayerInfo {
            period: 0.0,
            annual_poe: 0.002103,
        };
        assert!((l.poe_in(50.0) - 0.10).abs() < 0.001);
        assert!((l.return_period() - 475.0).abs() < 1.0);
        let l2 = LayerInfo {
            period: 0.0,
            annual_poe: 0.000399999,
        };
        assert!((l2.poe_in(50.0) - 0.0198).abs() < 0.001);
    }

    #[test]
    fn yaz_oku_ara_deger() {
        let grid = Grid {
            west: 29.0,
            south: 40.0,
            step: 0.1,
            nx: 3,
            ny: 2,
        };
        // Satır 0 güney: (29.0,40.0)=0.2, (29.1,40.0)=0.4, (29.2,40.0)=yok;
        // satır 1: 0.3, 0.5, 0.6.
        let a = vec![0.2, 0.4, f32::NAN, 0.3, 0.5, 0.6];
        let b: Vec<f32> = a.iter().map(|v| v * 2.0).collect();
        let pga = LayerInfo {
            period: 0.0,
            annual_poe: 0.002103,
        };
        let sa1 = LayerInfo {
            period: 1.0,
            annual_poe: 0.002103,
        };
        let mut buf = Vec::new();
        write(grid, &[(pga, a), (sa1, b)], zstd, &mut buf).unwrap();
        let m = HazardMap::open(buf).unwrap();
        assert_eq!(m.layers(), vec![pga, sa1]);
        let v = |l, lon, lat| m.value(l, lon, lat);
        assert!((v(0, 29.0, 40.0).unwrap() - 0.2).abs() < 1e-4);
        // Hücre ortası: dört düğümün ortalaması.
        assert!((v(0, 29.05, 40.05).unwrap() - 0.35).abs() < 1e-4);
        // Eksik düğüm: mevcutlarla.
        let e = v(0, 29.15, 40.05).unwrap();
        assert!((e - (0.4 + 0.5 + 0.6) / 3.0).abs() < 1e-4, "{e}");
        assert!(v(0, 28.9, 40.0).is_none());
        let u = m.uhs(0.002103, 29.0, 40.0);
        assert_eq!(u.len(), 2);
        assert!((u[1].1 - 0.4).abs() < 1e-4 && u[1].0 == 1.0);
    }
}
