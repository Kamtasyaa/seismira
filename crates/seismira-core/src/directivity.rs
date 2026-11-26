//! Yönelim (directivity) parametresi DPP ve merkezlenmiş ΔDPP (CY14 yönelim
//! terimi, [`crate::gmpe::cy14`]).
//!
//! Kaynaklar (birincil, 2026-11-26 okundu):
//! - Spudich vd. (2013), "Final report of the NGA-West2 Directivity Working
//!   Group", PEER Report 2013/09, Bölüm 6 (Chiou & Spudich, "The Chiou and
//!   Spudich NGA-West2 directivity predictor DPP"), s. 87–94
//!   (data/raw/refs/directivity/peer2013-09.pdf, SHA256 d48cc8f7…):
//!   - 6.2: doğrudan nokta P_D = hiposantrdan (P_H) sahanın düzlem üzerindeki
//!     dik izdüşümüne (P_P) giden ışının kayan alanın sınırını kestiği nokta;
//!     P_P alanın içindeyse P_D = P_P. E = |P_H P_D| ("E-path").
//!   - ĉ′ = 1 / (1/0,8 − (R_HYP − R_D)/E), E > 0; ĉ′ = 0,8, E = 0.
//!   - Eş. 6.1: DPP = ln(ĉ′ · max(E, 0,1 f) · max(F̄S, 0,2)), f = fay
//!     uzunluğu ile genişliğinin büyüğü.
//!   - 6.3: hiposantr bir kenara uzunluğun (ya da genişliğin) %10'undan
//!     yakınsa DPP için %10 konumuna taşınır.
//!   - Eş. 6.2–6.11: E-yolu boyunca ortalama S yayılım deseni
//!     F̄S = √(I_x² + I_n² + I_φ²) / E; z_S sahanın düzleme işaretli
//!     uzaklığı, l₂ = |P_H P_P|, φ kayma vektörü ile ışın arasındaki açı.
//!     (İşaretler F̄S'yi değiştirmez: terimlerin karesi alınır.)
//!   - Eş. 6.12: çok parçalı kırılmada hiposantrlı parçadan başlayıp sahaya
//!     doğru parçaların ĉ′·max(E,0,1f)·max(F̄S,0,2) çarpımları toplanır; bir
//!     sonraki parçanın "hiposantrı" önceki parçanın doğrudan noktasıdır;
//!     doğrudan nokta komşu parçaya bağlanan kenarda değilse toplam biter.
//! - Chiou & Youngs (2013), PEER Report 2013/07, s. 24: DPP depremin aynı
//!   uzaklıktaki sahalar üzerindeki ortalama DPP'sine göre merkezlenir
//!   (ΔDPP). Ortalama burada faya 75 km'ye kadar 2 km aralıklı düzenli bir
//!   ızgaradan, Rrup'a göre 2 km'lik dilimlerde alınır (alanı eşit örnekleme,
//!   eş-uzaklık eğrisi boyunca eşit örneklemeye yakındır; raporda örnekleme
//!   biçimi tanımlı değil: VARSAYIM). CY14 terimi Rrup > 70 km'de sıfır.

use crate::rupture::Rupture;
use crate::source::Fault;

/// ΔDPP'nin hesaplandığı en büyük Rrup (km): CY14 f_R = 0 ötesinde.
pub const MAX_RRUP_KM: f64 = 70.0;
/// Merkezleme dilimi (km) ve ızgara aralığı (km).
const BIN_KM: f64 = 2.0;
const GRID_KM: f64 = 2.0;

type V3 = [f64; 3];

fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn mul(a: V3, k: f64) -> V3 {
    [a[0] * k, a[1] * k, a[2] * k]
}
fn norm(a: V3) -> f64 {
    dot(a, a).sqrt()
}
fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// Bir parçanın düzlem geometrisi (yerel km; x doğu, y kuzey, z aşağı).
#[derive(Clone, Debug, PartialEq)]
struct Plane {
    origin: V3,
    /// Doğrultu ve eğim boyunca (aşağı) birim vektörler, normal.
    s: V3,
    w: V3,
    n: V3,
    length: f64,
    width: f64,
    /// Kayma birim vektörü (düzlemde).
    slip: V3,
}

impl Plane {
    fn new(f: &Fault, rake: f64) -> Self {
        let (s2, d2) = Fault::axes(f.strike);
        let (sd, cd) = f.dip.to_radians().sin_cos();
        let s = [s2[0], s2[1], 0.0];
        let w = [d2[0] * cd, d2[1] * cd, sd];
        let n = cross(s, w);
        let n = mul(n, 1.0 / norm(n));
        let top = f.top_start();
        let (sr, cr) = rake.to_radians().sin_cos();
        // Aki & Richards: kayma açısı düzlemde doğrultudan; pozitif = tavan
        // bloğu yukarı (eğim yukarısı −w).
        let slip = sub(mul(s, cr), mul(w, sr));
        Self {
            origin: [top[0], top[1], f.ztor],
            s,
            w,
            n,
            length: f.length,
            width: f.width,
            slip,
        }
    }

    fn point(&self, u: f64, v: f64) -> V3 {
        add(self.origin, add(mul(self.s, u), mul(self.w, v)))
    }

    /// Tek parçanın katkısı ĉ′·max(E,0,1f)·max(F̄S,0,2) ve doğrudan noktanın
    /// düzlem koordinatları (u, v). `h`: parça hiposantrı (u, v).
    fn term(&self, h: [f64; 2], site: V3) -> (f64, [f64; 2]) {
        let rel = sub(site, self.origin);
        let z_s = dot(rel, self.n);
        let (up, vp) = (dot(rel, self.s), dot(rel, self.w));
        let (ru, rv) = (up - h[0], vp - h[1]);
        let l2 = ru.hypot(rv);
        let inside = (0.0..=self.length).contains(&up) && (0.0..=self.width).contains(&vp);
        // E: ışın boyunca dikdörtgenden çıkış (P_P içerdeyse l₂).
        let e = if inside || l2 < 1e-9 {
            l2
        } else {
            let mut t = l2;
            let (du, dv) = (ru / l2, rv / l2);
            if du > 1e-12 {
                t = t.min((self.length - h[0]) / du);
            } else if du < -1e-12 {
                t = t.min(-h[0] / du);
            }
            if dv > 1e-12 {
                t = t.min((self.width - h[1]) / dv);
            } else if dv < -1e-12 {
                t = t.min(-h[1] / dv);
            }
            t.max(0.0)
        };
        let pd = if l2 < 1e-9 {
            h
        } else {
            [h[0] + ru / l2 * e, h[1] + rv / l2 * e]
        };
        let r_hyp = (l2 * l2 + z_s * z_s).sqrt();
        let r_d = ((l2 - e).powi(2) + z_s * z_s).sqrt();
        let c_hat = if e > 1e-9 {
            1.0 / (1.0 / 0.8 - (r_hyp - r_d) / e)
        } else {
            0.8
        };
        // φ: kayma ile ışın (P_H → P_P) arasındaki açı.
        let (cphi, sphi) = if l2 < 1e-9 {
            (1.0, 0.0)
        } else {
            let x = add(mul(self.s, ru / l2), mul(self.w, rv / l2));
            let c = dot(self.slip, x).clamp(-1.0, 1.0);
            (c, (1.0 - c * c).sqrt())
        };
        let fs = if e < 1e-6 {
            // E → 0 sınırı: noktasal desen |A^FS| (Eş. 6.2), l = 0.
            let rr = (l2 * l2 + z_s * z_s).sqrt().max(1e-12);
            let (ct, st) = (z_s / rr, l2 / rr);
            let c2t = ct * ct - st * st;
            ((c2t * cphi).powi(2) + (ct * sphi).powi(2)).sqrt()
        } else {
            // Eş. 6.7–6.9 (z_S → 0'da z·ln terimleri sıfıra gider).
            let (ix, in_, iphi) = if z_s.abs() < 1e-9 {
                (0.0, cphi * (-(r_hyp - r_d)), 0.0)
            } else {
                let lg = ((l2 + r_hyp) / (l2 - e + r_d)).ln();
                (
                    cphi * (2.0 * z_s * (l2 / r_hyp - (l2 - e) / r_d) - z_s * lg),
                    cphi * (-2.0 * z_s * z_s * (1.0 / r_hyp - 1.0 / r_d) - (r_hyp - r_d)),
                    sphi * (z_s * lg),
                )
            };
            (ix * ix + in_ * in_ + iphi * iphi).sqrt() / e
        };
        let f = self.length.max(self.width);
        (c_hat * e.max(0.1 * f) * fs.max(0.2), pd)
    }
}

/// Bir kırılmanın yönelim hesabı: parça düzlemleri, hiposantr ve
/// Rrup'a göre ortalama DPP tablosu.
#[derive(Clone, Debug, PartialEq)]
pub struct Directivity {
    planes: Vec<Plane>,
    /// Hiposantr: parça ve düzlem koordinatları (%10 kuralı uygulanmış).
    hypo: (usize, [f64; 2]),
    /// `mean[k]`: Rrup ∈ [k·BIN, (k+1)·BIN) dilimindeki ortalama DPP.
    mean: Vec<f64>,
}

impl Directivity {
    /// `rupture` ve kayma açısı için (merkezleme tablosu dahil, ~10–50 ms).
    pub fn new(rupture: &Rupture, rake: f64) -> Self {
        let planes: Vec<Plane> = rupture
            .segments
            .iter()
            .map(|f| Plane::new(f, rake))
            .collect();
        // Hiposantr: iz boyunca konumu ve eğim boyunca konumu, %10 kuralı
        // (çok parçalıda toplam uzunluğa göre).
        let (hu, hv) = rupture.hypocenter_plane();
        let total = rupture.length;
        let hu = hu.clamp(0.1 * total, 0.9 * total);
        let hv = hv.clamp(0.1 * rupture.width, 0.9 * rupture.width);
        let mut acc = 0.0;
        let mut hypo = (planes.len() - 1, [0.0, hv]);
        for (i, p) in planes.iter().enumerate() {
            if hu <= acc + p.length || i + 1 == planes.len() {
                hypo = (i, [(hu - acc).clamp(0.0, p.length), hv]);
                break;
            }
            acc += p.length;
        }
        let mut d = Self {
            planes,
            hypo,
            mean: Vec::new(),
        };
        d.mean = d.mean_table(rupture);
        d
    }

    /// Ham DPP (yüzeydeki yerel konum, km).
    pub fn dpp(&self, p: [f64; 2]) -> f64 {
        let site = [p[0], p[1], 0.0];
        let (mut i, mut h) = self.hypo;
        let (term, pd) = self.planes[i].term(h, site);
        let mut sum = term;
        // Sahaya doğru komşu parçalara (doğrudan nokta ortak kenardaysa).
        let tol = 1e-6;
        let dir: i64 = if pd[0] >= self.planes[i].length - tol && i + 1 < self.planes.len() {
            1
        } else if pd[0] <= tol && i > 0 {
            -1
        } else {
            0
        };
        let mut pd = pd;
        if dir != 0 {
            loop {
                let next = i as i64 + dir;
                if next < 0 || next as usize >= self.planes.len() {
                    break;
                }
                let at_edge = if dir > 0 {
                    pd[0] >= self.planes[i].length - tol
                } else {
                    pd[0] <= tol
                };
                if !at_edge {
                    break;
                }
                i = next as usize;
                h = if dir > 0 {
                    [0.0, pd[1].min(self.planes[i].width)]
                } else {
                    [self.planes[i].length, pd[1].min(self.planes[i].width)]
                };
                let (t, d) = self.planes[i].term(h, site);
                sum += t;
                pd = d;
            }
        }
        sum.ln()
    }

    /// Merkezlenmiş ΔDPP; Rrup > [`MAX_RRUP_KM`] için 0.
    pub fn centered(&self, p: [f64; 2], rrup: f64) -> f64 {
        if rrup > MAX_RRUP_KM || self.mean.is_empty() {
            return 0.0;
        }
        let k = ((rrup / BIN_KM) as usize).min(self.mean.len() - 1);
        self.dpp(p) - self.mean[k]
    }

    /// Rrup dilimlerinde ortalama DPP (boş dilim komşusundan doldurulur).
    fn mean_table(&self, rupture: &Rupture) -> Vec<f64> {
        let nb = (MAX_RRUP_KM / BIN_KM).ceil() as usize + 1;
        let (mut sum, mut cnt) = (vec![0.0; nb], vec![0usize; nb]);
        // Kırılmanın yüzey izdüşümünü kapsayan kutu ± (70 + pay) km.
        let mut lo = [f64::INFINITY; 2];
        let mut hi = [f64::NEG_INFINITY; 2];
        for p in &self.planes {
            for (u, v) in [
                (0.0, 0.0),
                (p.length, 0.0),
                (0.0, p.width),
                (p.length, p.width),
            ] {
                let q = p.point(u, v);
                for k in 0..2 {
                    lo[k] = lo[k].min(q[k]);
                    hi[k] = hi[k].max(q[k]);
                }
            }
        }
        let pad = MAX_RRUP_KM + BIN_KM;
        let (nx, ny) = (
            ((hi[0] - lo[0] + 2.0 * pad) / GRID_KM).ceil() as usize,
            ((hi[1] - lo[1] + 2.0 * pad) / GRID_KM).ceil() as usize,
        );
        for iy in 0..=ny {
            for ix in 0..=nx {
                let p = [
                    lo[0] - pad + ix as f64 * GRID_KM,
                    lo[1] - pad + iy as f64 * GRID_KM,
                ];
                let r = rupture.distances_local_raw(p).rrup;
                if r > MAX_RRUP_KM + BIN_KM {
                    continue;
                }
                let k = ((r / BIN_KM) as usize).min(nb - 1);
                sum[k] += self.dpp(p);
                cnt[k] += 1;
            }
        }
        let mut mean: Vec<Option<f64>> = sum
            .iter()
            .zip(&cnt)
            .map(|(s, &c)| (c > 0).then(|| s / c as f64))
            .collect();
        // Boş dilimler (fayın çok yakını, ızgara düşmediyse): en yakın dolu.
        for k in 0..nb {
            if mean[k].is_none() {
                let near = (1..nb).find_map(|d| {
                    let a = k.checked_sub(d).and_then(|j| mean[j]);
                    a.or_else(|| mean.get(k + d).copied().flatten())
                });
                mean[k] = near;
            }
        }
        mean.into_iter().map(|m| m.unwrap_or(0.0)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quake::{FaultType, Quake, Setting};

    /// Kuzeye doğrultulu düşey doğrultu atımlı fay: 100 × 15 km, üst kenar
    /// 0 km, hiposantr güney ucuna yakın (%10).
    fn ss_fault(along: f64) -> (Rupture, Quake) {
        let mut q = Quake::new((30.0, 40.0));
        q.mw = 7.3;
        q.depth_km = 7.5;
        q.fault = FaultType::StrikeSlip;
        q.strike = 0.0;
        q.dip = 90.0;
        q.rake = 0.0;
        q.setting = Setting::ShallowCrust;
        q.hypo_along = along;
        q.hypo_down = 0.5;
        q.rupture_km = Some([100.0, 15.0]);
        (Rupture::from_quake(&q), q)
    }

    /// Elle hesaplanan durum (PEER 2013/09 Bölüm 6 tanımları): hiposantr
    /// (u 10, v 7,5), saha fay uzantısında kuzeyde, düzlem içinde (z_S = 0).
    /// P_P = (u 150, v 0) dışarıda; ışın u = 100'de çıkar: E = 140,2·90/140 =
    /// 90,13; R_HYP = 140,20, R_D = |(50, 2,68)| = 50,07 → ĉ′ = 1/(1,25 −
    /// 1,0000) ≈ 4,0 (en büyük değer, ileri yönelim); F̄S = |cos φ| ≈ 0,9986.
    #[test]
    fn ileri_yonelim_elle() {
        let (r, _) = ss_fault(0.1);
        let plane = Plane::new(&r.segments[0], 0.0);
        // Yerel koordinat: hiposantr merkez üssünde; düzlemin başı güneyde.
        let site = plane.point(150.0, 0.0);
        let site = [site[0], site[1], 0.0];
        let (t, pd) = plane.term([10.0, 7.5], site);
        let e = (140f64.hypot(7.5)) * 90.0 / 140.0;
        assert!((pd[0] - 100.0).abs() < 1e-9, "{pd:?}");
        let r_hyp = 140f64.hypot(7.5);
        let r_d = 50f64.hypot(7.5 - 7.5 * 90.0 / 140.0);
        let c = 1.0 / (1.25 - (r_hyp - r_d) / e);
        let cphi = 140.0 / 140f64.hypot(7.5);
        assert!((t - c * e * cphi).abs() < 1e-6, "{t} {}", c * e * cphi);
        assert!(c > 3.9 && c <= 4.0001, "{c}");
    }

    /// İleri (kırılma yönünde) sahada ΔDPP > 0 ve geri (hiposantrın
    /// arkası, aynı Rrup) sahadan belirgin büyük. Not: fay ekseni üzerindeki
    /// geri sahada ĉ′ de en büyük değerine (4) yakındır; fark E-yolunun
    /// uzunluğundan gelir (ileri ~90, geri ~10 km), bu yüzden geri saha
    /// ortalamanın altında olmak zorunda değildir. Ortalama merkezleme
    /// dilimde ~0.
    #[test]
    fn ileri_geri_ve_merkezleme() {
        let (r, q) = ss_fault(0.1);
        let d = Directivity::new(&r, q.rake);
        let pl = &d.planes[0];
        let fwd = pl.point(130.0, 0.0);
        let back = pl.point(-30.0, 0.0);
        let rf = r.distances_local_raw([fwd[0], fwd[1]]).rrup;
        let rb = r.distances_local_raw([back[0], back[1]]).rrup;
        assert!((rf - rb).abs() < 1e-6, "{rf} {rb}");
        let df = d.centered([fwd[0], fwd[1]], rf);
        let db = d.centered([back[0], back[1]], rb);
        assert!(df > 1.0 && df - db > 1.5, "ileri {df}, geri {db}");
        // Ortalama: 30 km dilimindeki ızgara noktalarında ΔDPP ortalaması ~0.
        let (mut s, mut n) = (0.0, 0);
        for i in -60..=60 {
            for j in -60..=160 {
                let p = [f64::from(i) * 2.0, f64::from(j) * 2.0 - 20.0];
                let rr = r.distances_local_raw(p).rrup;
                if (30.0..32.0).contains(&rr) {
                    s += d.centered(p, rr);
                    n += 1;
                }
            }
        }
        assert!(
            n > 50 && (s / n as f64).abs() < 0.05,
            "{} ({n})",
            s / n as f64
        );
        // Uzakta sıfır.
        assert_eq!(d.centered([0.0, 300.0], 200.0), 0.0);
    }

    /// Ortada hiposantr: fayın iki yanındaki ayna sahalar ve iki ucun
    /// ötesindeki sahalar eşit.
    #[test]
    fn simetri() {
        let (r, q) = ss_fault(0.5);
        let d = Directivity::new(&r, q.rake);
        let pl = &d.planes[0];
        let a = pl.point(30.0, 0.0);
        let east = [a[0] + 20.0, a[1]];
        let west = [a[0] - 20.0, a[1]];
        assert!((d.dpp(east) - d.dpp(west)).abs() < 1e-9);
        let n = pl.point(120.0, 0.0);
        let s = pl.point(-20.0, 0.0);
        assert!((d.dpp([n[0], n[1]]) - d.dpp([s[0], s[1]])).abs() < 1e-6);
    }

    /// Çok parçalı: düz izi parçalara bölmek DPP'yi değiştirebilir. Eş. 6.12
    /// parça başına ĉ′, F̄S (0,2 tabanıyla) ve 0,1·f kullanır; uzun tek
    /// E-yolunda F̄S ortalaması küçülüp tabana inerken parçalarda daha büyük
    /// kalır, ĉ′·E de toplanabilir değildir (raporda değişmezlik iddiası
    /// yok). Ölçülen fark ≤ ~1 DPP (CY14 c8 ≤ 0,22 ile uzun periyotta ≤ ~%20;
    /// docs/bilim.md). Fay ekseni uzantısındaki uzak sahada eşit.
    #[test]
    fn cok_parcali_dogrusal() {
        let (r1, q) = ss_fault(0.1);
        let [a, b] = r1.segments[0].top_edge_lonlat();
        let (km, az) = crate::geo::distance_azimuth(a, b);
        let mut q2 = q;
        q2.trace = crate::quake::Trace::new(&[
            a,
            crate::geo::destination(a, az, km * 0.4),
            crate::geo::destination(a, az, km * 0.7),
            b,
        ]);
        let r2 = Rupture::from_quake(&q2);
        assert_eq!(r2.segments.len(), 3);
        let (d1, d2) = (Directivity::new(&r1, 0.0), Directivity::new(&r2, 0.0));
        for p in [[10.0, 60.0], [-25.0, -20.0], [40.0, 95.0]] {
            let (x, y) = (d1.dpp(p), d2.dpp(p));
            assert!(
                (x - y).abs() < 1.0 && x.is_finite() && y.is_finite(),
                "{p:?}: {x} {y}"
            );
        }
        let far = [0.0, 120.0];
        assert!((d1.dpp(far) - d2.dpp(far)).abs() < 0.05);
    }
}
