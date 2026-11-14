//! ak135 ilk P / ilk S varış süreleri (CLAUDE.md Bölüm 7.2).
//!
//! Model: ak135 (Kennett, Engdahl & Buland 1995, "Constraints on seismic
//! velocities in the Earth from traveltimes", GJI 122, 108-124). Tablo
//! ObsPy TauP ile çevrimdışı üretilir (scripts/gen_traveltime.py); burada
//! yalnızca okunur ve çift doğrusal ara değerle sorgulanır.
//!
//! Dosya biçimi (`.ttt`, little-endian):
//! ```text
//! "STTT"  u32 sürüm  u32 n_mesafe  u32 n_derinlik
//! f32 × n_mesafe (derece)  f32 × n_derinlik (km)
//! f32 × n_derinlik × n_mesafe  ilk P (s), satır = derinlik
//! f32 × n_derinlik × n_mesafe  ilk S (s)
//! ```

use std::io::{self, Read};
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    P,
    S,
}

#[derive(Clone, Debug)]
pub struct TravelTimes {
    dist: Vec<f32>,
    depth: Vec<f32>,
    p: Vec<f32>,
    s: Vec<f32>,
}

impl TravelTimes {
    pub fn load(path: &Path) -> io::Result<Self> {
        Self::read(&mut io::BufReader::new(std::fs::File::open(path)?))
    }

    pub fn read(r: &mut impl Read) -> io::Result<Self> {
        let mut magic = [0u8; 4];
        r.read_exact(&mut magic)?;
        if &magic != b"STTT" {
            return Err(invalid("STTT dosyası değil"));
        }
        let version = read_u32(r)?;
        if version != 1 {
            return Err(invalid("desteklenmeyen sürüm"));
        }
        let nd = read_u32(r)? as usize;
        let nh = read_u32(r)? as usize;
        let dist = read_f32s(r, nd)?;
        let depth = read_f32s(r, nh)?;
        let p = read_f32s(r, nd * nh)?;
        let s = read_f32s(r, nd * nh)?;
        let sorted = |v: &[f32]| v.windows(2).all(|w| w[0] < w[1]);
        if nd < 2 || nh < 2 || !sorted(&dist) || !sorted(&depth) {
            return Err(invalid("eksenler artan sırada değil"));
        }
        if p.iter().chain(&s).any(|t| !t.is_finite()) {
            return Err(invalid("tabloda geçersiz değer"));
        }
        Ok(Self { dist, depth, p, s })
    }

    /// Tablonun mesafe sınırı (derece). Ötesinde halka çizilmez (P gölge
    /// bölgesi, Bölüm 7.2).
    pub fn max_distance(&self) -> f64 {
        f64::from(*self.dist.last().expect("en az 2 nokta"))
    }

    pub fn max_depth(&self) -> f64 {
        f64::from(*self.depth.last().expect("en az 2 nokta"))
    }

    fn table(&self, phase: Phase) -> &[f32] {
        match phase {
            Phase::P => &self.p,
            Phase::S => &self.s,
        }
    }

    /// İlk varış süresi (s). Tablo dışındaysa `None` (sessiz ekstrapolasyon
    /// yok).
    pub fn time(&self, phase: Phase, depth_km: f64, dist_deg: f64) -> Option<f64> {
        let (i, fi) = locate(&self.depth, depth_km)?;
        let (j, fj) = locate(&self.dist, dist_deg)?;
        let t = self.table(phase);
        let nd = self.dist.len();
        let at = |a: usize, b: usize| f64::from(t[a * nd + b]);
        let top = at(i, j) * (1.0 - fj) + at(i, j + 1) * fj;
        let bottom = at(i + 1, j) * (1.0 - fj) + at(i + 1, j + 1) * fj;
        Some(top * (1.0 - fi) + bottom * fi)
    }

    /// Sabit bir odak derinliği için mesafeye göre varış eğrisi; halka
    /// yarıçapının ters aramasında kullanılır.
    pub fn curve(&self, phase: Phase, depth_km: f64) -> Option<Curve> {
        let mut times = Vec::with_capacity(self.dist.len());
        for &d in &self.dist {
            times.push(self.time(phase, depth_km, f64::from(d))?);
        }
        // İlk varış mesafeyle azalmaz; ara değer gürültüsüne karşı garanti.
        for k in 1..times.len() {
            times[k] = times[k].max(times[k - 1]);
        }
        Some(Curve {
            dist: self.dist.iter().map(|&d| f64::from(d)).collect(),
            times,
        })
    }
}

/// Bir derinlik için t(Δ) eğrisi.
#[derive(Clone, Debug)]
pub struct Curve {
    dist: Vec<f64>,
    times: Vec<f64>,
}

/// t anında dalga cephesinin konumu.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Front {
    /// Dalga henüz yüzeye ulaşmadı (derin deprem, Bölüm 7.2).
    NotYet,
    /// Yüzeydeki cephe bu episantr açısında (derece).
    At(f64),
    /// Cephe tablonun sınırını geçti; halka çizilmez.
    Beyond,
}

impl Curve {
    /// Varışın yüzeye ilk çıktığı an (episantrda, s).
    pub fn first_time(&self) -> f64 {
        self.times[0]
    }

    pub fn last_time(&self) -> f64 {
        *self.times.last().expect("boş değil")
    }

    /// t anındaki cephe: t(Δ) = t denkleminin ters çözümü (doğrusal ara değer).
    pub fn front(&self, t: f64) -> Front {
        if t < self.times[0] {
            return Front::NotYet;
        }
        if t > self.last_time() {
            return Front::Beyond;
        }
        // times[k] > t olan ilk k.
        let k = self.times.partition_point(|&x| x <= t);
        if k == 0 {
            return Front::At(self.dist[0]);
        }
        if k >= self.times.len() {
            return Front::At(*self.dist.last().expect("boş değil"));
        }
        let (t0, t1) = (self.times[k - 1], self.times[k]);
        let f = if t1 > t0 { (t - t0) / (t1 - t0) } else { 0.0 };
        Front::At(self.dist[k - 1] + f * (self.dist[k] - self.dist[k - 1]))
    }
}

/// Eksende değerin yeri: (alt indeks, kesir). Aralık dışıysa `None`.
fn locate(axis: &[f32], v: f64) -> Option<(usize, f64)> {
    let first = f64::from(axis[0]);
    let last = f64::from(*axis.last()?);
    if !(first..=last).contains(&v) {
        return None;
    }
    let k = axis.partition_point(|&a| f64::from(a) <= v);
    let i = k.saturating_sub(1).min(axis.len() - 2);
    let (a, b) = (f64::from(axis[i]), f64::from(axis[i + 1]));
    Some((i, (v - a) / (b - a)))
}

fn read_u32(r: &mut impl Read) -> io::Result<u32> {
    let mut b = [0u8; 4];
    r.read_exact(&mut b)?;
    Ok(u32::from_le_bytes(b))
}

fn read_f32s(r: &mut impl Read, n: usize) -> io::Result<Vec<f32>> {
    let mut v = vec![0f32; n];
    r.read_exact(bytemuck::cast_slice_mut(&mut v))?;
    Ok(v)
}

fn invalid(msg: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/traveltime");

    fn table() -> TravelTimes {
        TravelTimes::load(&Path::new(DIR).join("ak135.ttt")).expect("tablo")
    }

    /// Bölüm 14 kabul ölçütü: referans ak135 (TauP) değerinden sapma < 1 s.
    /// Kontrol noktaları ızgaraya denk gelmeyen rastgele noktalardır.
    #[test]
    fn taup_referansina_gore_hata_1_saniyeden_az() {
        let tt = table();
        let csv = std::fs::read_to_string(Path::new(DIR).join("ak135_check.csv")).unwrap();
        let (mut max_p, mut max_s, mut n) = (0f64, 0f64, 0);
        for line in csv.lines().skip(1) {
            let v: Vec<f64> = line.split(',').map(|x| x.parse().unwrap()).collect();
            let (h, d, p, s) = (v[0], v[1], v[2], v[3]);
            max_p = max_p.max((tt.time(Phase::P, h, d).unwrap() - p).abs());
            max_s = max_s.max((tt.time(Phase::S, h, d).unwrap() - s).abs());
            n += 1;
        }
        println!("{n} nokta: en büyük hata P {max_p:.3} s, S {max_s:.3} s");
        assert!(n >= 4000);
        assert!(max_p < 1.0 && max_s < 1.0, "P {max_p} s, S {max_s} s");
    }

    #[test]
    fn tablo_disi_none() {
        let tt = table();
        assert!(tt.time(Phase::P, 10.0, 100.5).is_none());
        assert!(tt.time(Phase::P, 800.0, 10.0).is_none());
        assert!(tt.time(Phase::P, -1.0, 10.0).is_none());
    }

    #[test]
    fn cephe_ters_arama_tutarli() {
        let tt = table();
        for &h in &[0.0, 10.0, 33.0, 150.0, 600.0] {
            for phase in [Phase::P, Phase::S] {
                let c = tt.curve(phase, h).unwrap();
                for &d in &[0.3, 5.0, 25.0, 77.7, 99.0] {
                    let t = tt.time(phase, h, d).unwrap();
                    match c.front(t) {
                        // Ters arama, t'ye ulaşılan ilk mesafeyi verir;
                        // aynı mesafeye geri dönmeli (düz bölge yoksa).
                        Front::At(d2) => assert!((d2 - d).abs() < 0.05, "{h} {phase:?} {d} {d2}"),
                        f => panic!("{h} {phase:?} {d}: {f:?}"),
                    }
                }
            }
        }
    }

    #[test]
    fn derin_depremde_dalga_gecikmeli_yuzeye_cikar() {
        let tt = table();
        let c = tt.curve(Phase::P, 600.0).unwrap();
        assert!(c.first_time() > 60.0); // 600 km'yi P ile aşmak ~1 dakikadan uzun
        assert_eq!(c.front(1.0), Front::NotYet);
        assert_eq!(c.front(1e5), Front::Beyond);
        // Yüzey kaynağında ilk an 0 s'dir.
        assert_eq!(tt.curve(Phase::S, 0.0).unwrap().first_time(), 0.0);
    }
}
