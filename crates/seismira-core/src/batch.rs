//! Toplu senaryo (README "Toplu senaryo ve olasılıksal analiz"; kullanıcı
//! kararı 2026-11-27: önce toplu senaryo, sonra ESHM20 ile PSHA).
//!
//! Bir fay üzerinde çok sayıda deprem: verilen depremin kırılma izi (çok
//! parçalı iz ya da tek düzlemin üst kenarı) "fay" kabul edilir. Her
//! büyüklükte kırılma uzunluğu otomatik bağıntıyla (WC94 / TMG17,
//! [`crate::source::Fault::from_quake`]) bulunur ve kırılma fay boyunca
//! kaydırılır ("yüzen kırılma"; [`Spec::positions`] konum, eşit aralıklı).
//! Kırılma fayın tamamından uzunsa fayın tamamı kırılır (büyüklük faya
//! göre büyük; arayüz belirtir). Merkez üssü kırılmanın ortası.
//!
//! Birleştirme ([`aggregate`]): her hücrede senaryoların ortalama ve en
//! yüksek aletsel şiddeti (JMA) ile PGV. Ağırlıklar EŞİT (SEÇİM): bu bir
//! olasılık hesabı DEĞİL; büyüklük–sıklık (Gutenberg–Richter) ve oluş hızı
//! ESHM20 aşamasında gelecek. Bir senaryonun model sınırı dışında kalan kara
//! hücreleri ortalamaya şiddet 0 ve PGV 0 ile girer (o senaryoda sarsıntı
//! hissedilmez sayılır); her senaryoda veri olmayan hücre (deniz) veri yok.

use crate::geo;
use crate::grid::{NO_DATA, ShakeGrid};
use crate::intensity::Shindo;
use crate::quake::{Quake, Trace};
use crate::rupture::Rupture;
use crate::source::Fault;

/// Toplu senaryo tanımı.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spec {
    pub mw_min: f64,
    pub mw_max: f64,
    pub mw_step: f64,
    /// Her büyüklükte kırılmanın fay boyunca konum sayısı (≥ 1).
    pub positions: usize,
}

impl Default for Spec {
    fn default() -> Self {
        Self {
            mw_min: 6.5,
            mw_max: 7.5,
            mw_step: 0.25,
            positions: 3,
        }
    }
}

impl Spec {
    /// Büyüklükler (uçlar dahil, adım ≥ 0,05).
    pub fn magnitudes(&self) -> Vec<f64> {
        let step = self.mw_step.max(0.05);
        let mut out = Vec::new();
        let mut k = 0;
        loop {
            let m = self.mw_min + step * f64::from(k);
            if m > self.mw_max + 1e-9 {
                break;
            }
            out.push((m * 100.0).round() / 100.0);
            k += 1;
        }
        out
    }
}

/// Bir üye senaryo.
#[derive(Clone, Debug, PartialEq)]
pub struct Member {
    pub quake: Quake,
    /// Kırılma uzunluğu (km) ve fay üzerindeki başlangıcı (km).
    pub length_km: f64,
    pub start_km: f64,
    /// Otomatik uzunluk fayın tamamını aştı (fayın tamamı kırıldı).
    pub whole_fault: bool,
}

/// Verilen depremin fay çizgisi: kırılmanın üst kenar izi.
pub fn fault_line(base: &Quake) -> Vec<(f64, f64)> {
    Rupture::from_quake(base).trace_lonlat()
}

fn polyline_km(line: &[(f64, f64)]) -> f64 {
    line.windows(2).map(|w| geo::distance_km(w[0], w[1])).sum()
}

/// Çizgi üzerinde baştan `s` km'deki nokta.
fn point_at(line: &[(f64, f64)], s: f64) -> (f64, f64) {
    let mut acc = 0.0;
    for w in line.windows(2) {
        let (d, az) = geo::distance_azimuth(w[0], w[1]);
        if acc + d >= s {
            return geo::destination(w[0], az, (s - acc).max(0.0));
        }
        acc += d;
    }
    *line.last().expect("boş çizgi")
}

/// Çizginin [a, b] km arası parçası (uçlar enterpolasyonla). Uçlara
/// 1 km'den yakın ara köşeler atlanır: iz doğrulaması 1 km'den kısa parçayı
/// kabul etmez ([`crate::quake::validate`]).
fn subline(line: &[(f64, f64)], a: f64, b: f64) -> Vec<(f64, f64)> {
    let mut out = vec![point_at(line, a)];
    let mut acc = 0.0;
    for w in line.windows(2) {
        acc += geo::distance_km(w[0], w[1]);
        if acc > a + 1.0 && acc < b - 1.0 {
            out.push(w[1]);
        }
    }
    out.push(point_at(line, b));
    out
}

/// Üye senaryolar (büyüklük artan, konum fay başından sonuna).
pub fn members(base: &Quake, spec: &Spec) -> Vec<Member> {
    let line = fault_line(base);
    let total = polyline_km(&line);
    let mut out = Vec::new();
    if line.len() < 2 || total <= 0.0 {
        return out;
    }
    for mw in spec.magnitudes() {
        let mut q = Quake {
            mw,
            rupture_km: None,
            trace: None,
            ..*base
        };
        let auto = Fault::from_quake(&q).length;
        let whole = auto >= total - 1e-6;
        let len = auto.min(total);
        let slack = total - len;
        let n = if whole { 1 } else { spec.positions.max(1) };
        for k in 0..n {
            let start = if n == 1 {
                slack / 2.0
            } else {
                slack * k as f64 / (n - 1) as f64
            };
            let sub = subline(&line, start, start + len);
            let Some(trace) = Trace::new(&sub) else {
                continue;
            };
            q.trace = Some(trace);
            q.epicenter = point_at(&line, start + len / 2.0);
            out.push(Member {
                quake: q,
                length_km: len,
                start_km: start,
                whole_fault: whole,
            });
        }
    }
    out
}

/// Birleşik sonuç: aynı ızgarada ortalama ve en yüksek.
#[derive(Clone, Debug)]
pub struct Aggregate {
    /// Ortalama / en yüksek JMA aletsel şiddeti; veri yoksa NaN.
    pub jma_mean: Vec<f32>,
    pub jma_max: Vec<f32>,
    /// Ortalama / en yüksek PGV (cm/s; ızgaradaki tanımla); veri yoksa NaN.
    pub pgv_mean: Vec<f32>,
    pub pgv_max: Vec<f32>,
}

/// Artımlı birleştirici: üyelerin ızgaraları sırayla eklenir, bellekte
/// yalnızca toplamlar kalır (39 üyelik 1024² ızgara ~1,5 GB tutmasın).
#[derive(Clone, Debug, Default)]
pub struct Accumulator {
    n: usize,
    have: Vec<bool>,
    sum_j: Vec<f32>,
    sum_v: Vec<f32>,
    max_j: Vec<f32>,
    max_v: Vec<f32>,
}

impl Accumulator {
    /// Izgarayı ekler; boyut öncekilerle uyuşmazsa `false`.
    pub fn add(&mut self, g: &ShakeGrid) -> bool {
        let n = g.class.len();
        if self.n == 0 {
            self.have = vec![false; n];
            self.sum_j = vec![0.0; n];
            self.sum_v = vec![0.0; n];
            self.max_j = vec![0.0; n];
            self.max_v = vec![0.0; n];
        } else if self.have.len() != n {
            return false;
        }
        for i in 0..n {
            if g.class[i] == NO_DATA {
                continue; // şiddet 0, PGV 0 (modül notu)
            }
            self.have[i] = true;
            self.sum_j[i] += g.jma[i];
            self.sum_v[i] += g.pgv[i];
            self.max_j[i] = self.max_j[i].max(g.jma[i]);
            self.max_v[i] = self.max_v[i].max(g.pgv[i]);
        }
        self.n += 1;
        true
    }

    pub fn count(&self) -> usize {
        self.n
    }

    /// Ortalama ve en yüksek; hiç üye yoksa `None`.
    pub fn finish(&self) -> Option<Aggregate> {
        if self.n == 0 {
            return None;
        }
        let m = self.n as f32;
        let pick = |v: &[f32], div: f32| -> Vec<f32> {
            v.iter()
                .zip(&self.have)
                .map(|(&x, &h)| if h { x / div } else { f32::NAN })
                .collect()
        };
        Some(Aggregate {
            jma_mean: pick(&self.sum_j, m),
            jma_max: pick(&self.max_j, 1.0),
            pgv_mean: pick(&self.sum_v, m),
            pgv_max: pick(&self.max_v, 1.0),
        })
    }
}

/// Aynı alan ve adımla hesaplanmış ızgaraları birleştirir (modül notu).
/// Boyutlar uyuşmazsa `None`.
pub fn aggregate(grids: &[&ShakeGrid]) -> Option<Aggregate> {
    let mut acc = Accumulator::default();
    for g in grids {
        if !acc.add(g) {
            return None;
        }
    }
    acc.finish()
}

/// Birleşik sonucu haritada sabit gösterim için ızgaraya çevirir: sınıf
/// `jma`'dan, S varışı 0, yükseliş anlık (zaman ilerlemesi yok). Diğer
/// alanlar şablondan (`template`; aynı alan).
pub fn as_grid(template: &ShakeGrid, jma: &[f32], pgv: &[f32]) -> ShakeGrid {
    let n = jma.len();
    let mut g = template.clone();
    for i in 0..n {
        if jma[i].is_nan() {
            g.class[i] = NO_DATA;
            continue;
        }
        g.class[i] = Shindo::from_instrumental(f64::from(jma[i])) as u8;
        g.jma[i] = jma[i];
        g.pgv[i] = pgv[i];
        g.t_p[i] = 0.0;
        g.t_s[i] = 0.0;
        g.rise[i] = 1e-3;
        g.t_end[i] = f32::INFINITY;
    }
    g
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quake::FaultType;

    fn base() -> Quake {
        let mut q = Quake::new((30.0, 40.7));
        q.mw = 7.6;
        q.fault = FaultType::StrikeSlip;
        q.strike = 90.0;
        q
    }

    #[test]
    fn buyuklukler() {
        let s = Spec {
            mw_min: 6.5,
            mw_max: 7.5,
            mw_step: 0.25,
            positions: 3,
        };
        assert_eq!(s.magnitudes(), vec![6.5, 6.75, 7.0, 7.25, 7.5]);
    }

    /// Kırılmalar fayın içinde; küçük depremde 3 konum, uzunluk büyüklükle
    /// artar; fayı aşan büyüklükte tek üye ve "fayın tamamı".
    #[test]
    fn yuzen_kirilmalar() {
        let b = base();
        let line = fault_line(&b);
        let total = polyline_km(&line);
        let ms = members(
            &b,
            &Spec {
                mw_min: 6.5,
                mw_max: 8.0,
                mw_step: 0.5,
                positions: 3,
            },
        );
        let small: Vec<&Member> = ms.iter().filter(|m| m.quake.mw == 6.5).collect();
        assert_eq!(small.len(), 3);
        assert!(small[0].start_km.abs() < 1e-9);
        assert!((small[2].start_km + small[2].length_km - total).abs() < 1e-6);
        for m in &ms {
            assert!(m.start_km >= -1e-9 && m.start_km + m.length_km <= total + 1e-6);
            let r = Rupture::from_quake(&m.quake);
            assert!(
                (r.length - m.length_km).abs() < 0.5,
                "{} {}",
                r.length,
                m.length_km
            );
            // Merkez üssü kırılmanın üstünde.
            let d = m.quake.trace.unwrap();
            let mid = d.points()[0];
            assert!(geo::distance_km(mid, m.quake.epicenter) <= m.length_km / 2.0 + 0.5);
        }
        let big: Vec<&Member> = ms.iter().filter(|m| m.quake.mw == 8.0).collect();
        assert_eq!(big.len(), 1);
        assert!(big[0].whole_fault);
        let l65 = small[0].length_km;
        let l75 = ms.iter().find(|m| m.quake.mw == 7.5).unwrap().length_km;
        assert!(l75 > 2.0 * l65);
    }

    #[test]
    fn birlestirme() {
        let spec = crate::grid::GridSpec {
            west: 0.0,
            north: 1.0,
            cell: 0.5,
            width: 2,
            height: 1,
        };
        let mk = |c: [u8; 2], j: [f32; 2], v: [f32; 2]| ShakeGrid {
            spec,
            class: c.to_vec(),
            jma: j.to_vec(),
            t_p: vec![1.0; 2],
            t_s: vec![2.0; 2],
            t_end: vec![9.0; 2],
            rise: vec![0.4; 2],
            pgv: v.to_vec(),
            pga: vec![0.1; 2],
            pgv_tau: vec![0.3; 2],
            pga_tau: vec![0.3; 2],
        };
        let a = mk([5, NO_DATA], [5.0, f32::NAN], [40.0, f32::NAN]);
        let b = mk([3, NO_DATA], [3.0, f32::NAN], [10.0, f32::NAN]);
        let c = mk([NO_DATA, NO_DATA], [f32::NAN; 2], [f32::NAN; 2]);
        let g = aggregate(&[&a, &b, &c]).unwrap();
        // Model dışı senaryo 0 ile ortalamaya girer.
        assert!((g.jma_mean[0] - 8.0 / 3.0).abs() < 1e-6);
        assert_eq!(g.jma_max[0], 5.0);
        assert_eq!(g.pgv_max[0], 40.0);
        assert!(g.jma_mean[1].is_nan());
        let shown = as_grid(&a, &g.jma_max, &g.pgv_max);
        assert_eq!(shown.class[0], Shindo::from_instrumental(5.0) as u8);
        assert_eq!(shown.t_s[0], 0.0);
        assert_eq!(shown.class[1], NO_DATA);
    }
}
