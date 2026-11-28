//! Tsunami (Faz 8; docs/bilim.md 7.10).
//!
//! - [`bathy`]: uygulamanın okuduğu derinlik ızgarası (`data/tsunami/bathy.sras`).
//! - [`coast`]: kıyı tahmin noktaları (`data/tsunami/coast.bin`).
//! - [`okada85`]: sonlu dikdörtgen faydan yüzey yer değiştirmesi.
//! - [`trigger`]: ilk değerlendirme (PTWC ölçütleri).
//! - [`travel`]: varış süresi haritası (fast marching, √(g·h)).
//! - [`runup`]: yakın alanda en yüksek tırmanma (Riquelme vd. 2015).
//! - [`source`]: depremden deniz tabanı düşey yer değiştirmesi (Okada).
//! - [`forecast`]: hepsini birleştiren senaryo tahmini.

/// Deniz derinliği ızgarası: NOAA NCEI ETOPO 2022 (60″, CC0) + Natural Earth
/// deniz maskesi (scripts/gen_bathy.py, `seismira-data bathy`; docs/veri.md).
///
/// Tek katman `depth_m`: logaritmik 1–11 000 m, 251 kod (göreli adım %3,8;
/// dalga hızı √(g·h) en çok ±%1 hatalı). [`LAND`] = kara, göl, buz sahanlığı.
pub mod bathy {
    use crate::raster::{LayerInfo, Scale};

    pub const FILE: &str = "bathy.sras";
    pub const LAND: u8 = 255;
    pub const DEPTH_LO: f32 = 1.0;
    pub const DEPTH_HI: f32 = 11_000.0;

    pub fn layers() -> Vec<LayerInfo> {
        vec![LayerInfo {
            name: "depth_m".into(),
            scale: Scale::Log,
            n_valid: 251,
            lo: DEPTH_LO,
            hi: DEPTH_HI,
        }]
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn derinlik_kodlamasi() {
            let l = &layers()[0];
            for d in [1.0f32, 3.0, 50.0, 200.0, 4000.0, 10_752.0] {
                let back = l.value(l.quantize(d)).unwrap();
                // Yarım adım: %1,9.
                assert!((back / d - 1.0).abs() < 0.019, "{d} → {back}");
            }
            assert!(l.value(LAND).is_none());
        }
    }
}

/// Kıyı tahmin noktaları (`data/tsunami/coast.bin`, `seismira-data
/// coast-points`): denize bakan 1′ deniz hücrelerinden ~[`coast::SPACING_KM`]
/// arayla seçilir; tsunami varış süresi ve yüksekliği bu noktalarda
/// hesaplanır (docs/bilim.md 7.10).
pub mod coast {
    use std::io::{self, Read, Write};

    pub const FILE: &str = "coast.bin";
    pub const SPACING_KM: f64 = 10.0;
    const MAGIC: &[u8; 4] = b"SCST";
    const VERSION: u32 = 1;

    /// Bir kıyı noktası. `city`: en yakın şehrin (cities15000.scty) sırası,
    /// `city_km` uzaklığı.
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct CoastPoint {
        pub lon: f32,
        pub lat: f32,
        /// Noktanın (kıyıdaki 1′ deniz hücresinin) derinliği (m).
        pub depth_m: f32,
        pub city: u32,
        pub city_km: f32,
    }

    pub fn write(points: &[CoastPoint], w: &mut impl Write) -> io::Result<()> {
        w.write_all(MAGIC)?;
        w.write_all(&VERSION.to_le_bytes())?;
        w.write_all(&(points.len() as u32).to_le_bytes())?;
        for p in points {
            for v in [p.lon, p.lat, p.depth_m] {
                w.write_all(&v.to_le_bytes())?;
            }
            w.write_all(&p.city.to_le_bytes())?;
            w.write_all(&p.city_km.to_le_bytes())?;
        }
        Ok(())
    }

    pub fn read(r: &mut impl Read) -> io::Result<Vec<CoastPoint>> {
        let bad = |m: &str| io::Error::new(io::ErrorKind::InvalidData, m.to_owned());
        let mut b4 = [0u8; 4];
        r.read_exact(&mut b4)?;
        if &b4 != MAGIC {
            return Err(bad("kıyı dosyası değil"));
        }
        r.read_exact(&mut b4)?;
        if u32::from_le_bytes(b4) != VERSION {
            return Err(bad("kıyı dosyası sürümü farklı"));
        }
        r.read_exact(&mut b4)?;
        let n = u32::from_le_bytes(b4) as usize;
        if n > 10_000_000 {
            return Err(bad("kıyı noktası sayısı makul değil"));
        }
        let mut buf = vec![0u8; n * 20];
        r.read_exact(&mut buf)?;
        Ok(buf
            .as_chunks::<20>()
            .0
            .iter()
            .map(|c| {
                let f = |k: usize| f32::from_le_bytes([c[k], c[k + 1], c[k + 2], c[k + 3]]);
                CoastPoint {
                    lon: f(0),
                    lat: f(4),
                    depth_m: f(8),
                    city: u32::from_le_bytes([c[12], c[13], c[14], c[15]]),
                    city_km: f(16),
                }
            })
            .collect())
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn yaz_oku() {
            let v = vec![
                CoastPoint {
                    lon: 28.97,
                    lat: 41.0,
                    depth_m: 12.5,
                    city: 7,
                    city_km: 3.2,
                },
                CoastPoint {
                    lon: -179.99,
                    lat: -50.0,
                    depth_m: 1.0,
                    city: u32::MAX,
                    city_km: f32::INFINITY,
                },
            ];
            let mut b = Vec::new();
            write(&v, &mut b).unwrap();
            assert_eq!(read(&mut &b[..]).unwrap(), v);
            assert!(read(&mut &b"XXXX"[..]).is_err());
        }
    }
}

/// Okada, Y. (1985), "Surface deformation due to shear and tensile faults in
/// a half-space", BSSA 75(4), 1135–1154 (PDF: NIED, bosai.go.jp). Sonlu
/// dikdörtgen kaynak, yüzey yer değiştirmesi: Eş. (24)–(30); tekillik
/// kuralları s. 1148 (i)–(iii). Birim testleri makalenin Tablo 2'si.
///
/// Koordinatlar (Şekil 1): x doğrultu boyunca, z yukarı; fay x'te 0…L,
/// eğim yönünde η' = 0…W (η' arttıkça yukarı: derinlik d − η' sin δ, y
/// kayması η' cos δ). `d`: η' = 0 kenarının (fayın ALT kenarı) derinliği.
/// U1 doğrultu atımı, U2 eğim atımı, U3 açılma (Burgers vektörü bileşenleri,
/// tavan bloğunun taban bloğuna göre hareketi).
pub mod okada85 {
    use std::f64::consts::PI;

    /// Ortam: μ / (λ + μ). Poisson katısı (λ = μ) için 0,5 (Tablo 2 ile
    /// aynı; kabuk için yaygın varsayım).
    pub const POISSON_SOLID: f64 = 0.5;

    /// Bir kaynak tipinin (ux, uy, uz) katkısı, birim kayma için.
    type Disp = [f64; 3];

    /// Eş. (25)–(30): tek köşe (ξ, η) için [doğrultu, eğim, açılma] ×
    /// (ux, uy, uz), birim kayma (U = 1).
    fn corner(xi: f64, eta: f64, q: f64, sd: f64, cd: f64, m: f64) -> [Disp; 3] {
        let r = (xi * xi + eta * eta + q * q).sqrt();
        let x = (xi * xi + q * q).sqrt();
        let yt = eta * cd + q * sd;
        let dt = eta * sd - q * cd;
        // (iii) R + η = 0: paydasında R + η olan terimler 0, ln(R + η) →
        // −ln(R − η).
        let r_eta_zero = (r + eta).abs() < 1e-12;
        let inv_reta = if r_eta_zero { 0.0 } else { 1.0 / (r + eta) };
        let ln_reta = if r_eta_zero {
            -(r - eta).ln()
        } else {
            (r + eta).ln()
        };
        let inv_rxi = if (r + xi).abs() < 1e-12 {
            0.0
        } else {
            1.0 / (r + xi)
        };
        // (i) q = 0: tan⁻¹(ξη / qR) = 0.
        let th = if q.abs() < 1e-12 {
            0.0
        } else {
            (xi * eta / (q * r)).atan()
        };
        // Eş. (28) / (29).
        let (i1, i2, i3, i4, i5);
        if cd.abs() > 1e-12 {
            i4 = m / cd * ((r + dt).ln() - sd * ln_reta);
            // (ii) ξ = 0: I5 = 0.
            i5 = if xi.abs() < 1e-12 {
                0.0
            } else {
                m * 2.0 / cd
                    * ((eta * (x + q * cd) + x * (r + x) * sd) / (xi * (r + x) * cd)).atan()
            };
            i3 = m * (1.0 / cd * yt / (r + dt) - ln_reta) + sd / cd * i4;
            i1 = m * (-1.0 / cd * xi / (r + dt)) - sd / cd * i5;
        } else {
            let rd = r + dt;
            i1 = -m / 2.0 * xi * q / (rd * rd);
            i3 = m / 2.0 * (eta / rd + yt * q / (rd * rd) - ln_reta);
            i4 = -m * q / rd;
            i5 = -m * xi * sd / rd;
        }
        i2 = m * (-ln_reta) - i3;
        let k = 1.0 / (2.0 * PI);
        let a = xi * q / r * inv_reta; // ξq / R(R+η)
        let strike = [
            -k * (a + th + i1 * sd),
            -k * (yt * q / r * inv_reta + q * cd * inv_reta + i2 * sd),
            -k * (dt * q / r * inv_reta + q * sd * inv_reta + i4 * sd),
        ];
        let dip = [
            -k * (q / r - i3 * sd * cd),
            -k * (yt * q / r * inv_rxi + cd * th - i1 * sd * cd),
            -k * (dt * q / r * inv_rxi + sd * th - i5 * sd * cd),
        ];
        let tensile = [
            k * (q * q / r * inv_reta - i3 * sd * sd),
            k * (-dt * q / r * inv_rxi - sd * (a - th) - i1 * sd * sd),
            k * (yt * q / r * inv_rxi + cd * (a - th) - i5 * sd * sd),
        ];
        [strike, dip, tensile]
    }

    /// Yüzeydeki (x, y) noktasında yer değiştirme (ux, uy, uz), Chinnery
    /// gösterimiyle (Eş. 24): f(x, p) − f(x, p − W) − f(x − L, p) +
    /// f(x − L, p − W). `u`: (U1, U2, U3) kayma; `m`: μ / (λ + μ).
    #[allow(clippy::too_many_arguments)]
    pub fn displacement(
        x: f64,
        y: f64,
        d: f64,
        dip_deg: f64,
        l: f64,
        w: f64,
        u: [f64; 3],
        m: f64,
    ) -> [f64; 3] {
        let (sd, cd) = dip_deg.to_radians().sin_cos();
        // cos δ = 0 tam olmalı (90° için sin_cos 6e-17 verir).
        let cd = if cd.abs() < 1e-12 { 0.0 } else { cd };
        let p = y * cd + d * sd;
        let q = y * sd - d * cd;
        let mut out = [0.0; 3];
        for (xi, eta, sign) in [
            (x, p, 1.0),
            (x, p - w, -1.0),
            (x - l, p, -1.0),
            (x - l, p - w, 1.0),
        ] {
            let c = corner(xi, eta, q, sd, cd, m);
            for (t, ut) in c.iter().zip(u) {
                for (o, v) in out.iter_mut().zip(t) {
                    *o += sign * ut * v;
                }
            }
        }
        out
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        /// Tablo 2 değerleri dört anlamlı basamakla verilmiş: göreli 5e-4
        /// (ya da sıfırlarda mutlak 1e-12).
        fn check(case: (f64, f64, f64, f64), want: [[f64; 3]; 3]) {
            let (x, y, d, dip) = case;
            for (k, w) in want.iter().enumerate() {
                let mut u = [0.0; 3];
                u[k] = 1.0;
                let got = displacement(x, y, d, dip, 3.0, 2.0, u, POISSON_SOLID);
                for (g, e) in got.iter().zip(w) {
                    let ok = if *e == 0.0 {
                        g.abs() < 1e-12
                    } else {
                        ((g - e) / e).abs() < 5e-4
                    };
                    assert!(ok, "durum {case:?}, tip {k}: {got:?} ≠ {w:?}");
                }
            }
        }

        #[test]
        fn tablo2_durum2_egik() {
            check(
                (2.0, 3.0, 4.0, 70.0),
                [
                    [-8.689e-3, -4.298e-3, -2.747e-3],
                    [-4.682e-3, -3.527e-2, -3.564e-2],
                    [-2.660e-4, 1.056e-2, 3.214e-3],
                ],
            );
        }

        #[test]
        fn tablo2_durum3_dusey() {
            check(
                (0.0, 0.0, 4.0, 90.0),
                [
                    [0.0, 5.253e-3, 0.0],
                    [0.0, 0.0, 0.0],
                    [1.223e-2, 0.0, -1.606e-2],
                ],
            );
        }

        #[test]
        fn tablo2_durum4_eksi_90() {
            check(
                (0.0, 0.0, 4.0, -90.0),
                [
                    [0.0, -1.303e-3, 0.0],
                    [0.0, 0.0, 0.0],
                    [3.507e-3, 0.0, -7.740e-3],
                ],
            );
        }
    }
}

/// İlk değerlendirme: PTWC'nin ilk metin ürünü ölçütleri. Kaynak: IOC
/// Technical Series 135 (2017), "User's Guide for the Pacific Tsunami
/// Warning Center Enhanced Products for the CARIBE-EWS", Tablo 2
/// (www.weather.gov/media/ctwp/PDF/Users guide 2017.pdf; data/raw/papers/).
/// Karayipler dışı ("Atlantic") satırları kullanılır; Karayipler'de bilgi
/// eşiği 6,0'dır (aynı tablo).
pub mod trigger {
    /// Değerlendirme sonucu.
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub enum Assessment {
        /// Ürün yok (Mw < 6,5).
        None,
        /// Bilgi bildirimi: tsunami tehdidi beklenmez.
        Information(Reason),
        /// Tehdit mesajı: olası tehdit; `radius_km` içindeki kıyılar
        /// (`None`: Mw ≥ 7,9, varış süresi ≤ 3 saat olan kıyılar).
        Threat { radius_km: Option<f64> },
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Reason {
        TooSmall,
        TooDeep,
        Inland,
    }

    /// Mw ≥ 7,9'da tehdit altındaki kıyıların en geç varış süresi (s).
    pub const BASIN_ETA_S: f64 = 3.0 * 3600.0;

    /// `at_sea`: merkez üssü deniz altında ya da kıyıya çok yakın (tablonun
    /// "under the sea or very near the coast"; nasıl belirleneceği
    /// uygulamanın işi).
    pub fn assess(mw: f64, depth_km: f64, at_sea: bool) -> Assessment {
        if mw < 6.5 {
            return Assessment::None;
        }
        if !at_sea {
            return Assessment::Information(Reason::Inland);
        }
        if mw <= 7.0 {
            return Assessment::Information(Reason::TooSmall);
        }
        if depth_km >= 100.0 {
            return Assessment::Information(Reason::TooDeep);
        }
        // Tablo Mw'yi bir ondalıkla verir (7,1–7,5, 7,6–7,8, ≥ 7,9);
        // aradaki değerler bir üst aralığa yuvarlanır.
        let radius_km = if mw < 7.55 {
            Some(300.0)
        } else if mw < 7.85 {
            Some(1000.0)
        } else {
            None
        };
        Assessment::Threat { radius_km }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn ptwc_tablo2() {
            use Assessment::*;
            assert_eq!(assess(6.4, 10.0, true), None);
            assert_eq!(assess(6.8, 10.0, true), Information(Reason::TooSmall));
            assert_eq!(assess(7.0, 10.0, true), Information(Reason::TooSmall));
            assert_eq!(assess(7.3, 10.0, false), Information(Reason::Inland));
            assert_eq!(assess(7.3, 120.0, true), Information(Reason::TooDeep));
            assert_eq!(
                assess(7.3, 99.0, true),
                Threat {
                    radius_km: Some(300.0)
                }
            );
            assert_eq!(
                assess(7.5, 30.0, true),
                Threat {
                    radius_km: Some(300.0)
                }
            );
            assert_eq!(
                assess(7.7, 30.0, true),
                Threat {
                    radius_km: Some(1000.0)
                }
            );
            assert_eq!(
                assess(7.8, 30.0, true),
                Threat {
                    radius_km: Some(1000.0)
                }
            );
            assert_eq!(
                assess(9.0, 30.0, true),
                Threat {
                    radius_km: Option::None
                }
            );
        }
    }
}

/// Varış süresi: eikonal denklem |∇T| = 1 / c, c = √(g·h) (uzun dalga
/// hızı), birinci dereceden fast marching (Sethian 1996, PNAS 93(4),
/// 1591–1595; güncelleme kuralı: Sethian 1999, "Level Set Methods and Fast
/// Marching Methods", 2. bs., Cambridge, Bölüm 8). Izgara boylam-enlem:
/// hücre kenarları dx = R·cos φ·Δλ, dy = R·Δφ; 4 komşu, iki yönlü
/// (ikinci dereceden denklem) güncelleme. Doğruluk birim testinde: sabit
/// derinlikte nokta kaynaktan analitik süreye göre eksen boyunca < %1, 45°
/// köşegende +%0,66 (docs/bilim.md 7.10).
pub mod travel {
    use std::cmp::Ordering;
    use std::collections::BinaryHeap;
    use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};

    /// Yerçekimi ivmesi (m/s²).
    pub const G: f64 = 9.81;
    const R_M: f64 = 6_371_008.8;

    /// Hesap ızgarası: satır satır, kuzeyden güneye; `depth` (m) NaN = kara.
    pub struct Grid<'a> {
        pub west: f64,
        pub north: f64,
        pub cell: f64,
        pub width: usize,
        pub height: usize,
        pub depth: &'a [f32],
        /// Izgara 360°'yi kaplıyorsa boylamda sarmal.
        pub wrap: bool,
    }

    #[derive(PartialEq)]
    struct Item(f64, usize);
    impl Eq for Item {}
    impl PartialOrd for Item {
        fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
            Some(self.cmp(o))
        }
    }
    impl Ord for Item {
        fn cmp(&self, o: &Self) -> Ordering {
            // En küçük süre önce (BinaryHeap en büyüğü verir).
            o.0.total_cmp(&self.0).then(o.1.cmp(&self.1))
        }
    }

    /// `seeds`: (hücre, başlangıç süresi s). `t_max`: bu süreden sonrası
    /// hesaplanmaz (INFINITY kalır). İptal: `None`.
    pub fn compute(
        g: &Grid,
        seeds: &[(usize, f64)],
        t_max: f64,
        cancel: &AtomicBool,
    ) -> Option<Vec<f32>> {
        let n = g.width * g.height;
        assert_eq!(g.depth.len(), n);
        let mut t = vec![f64::INFINITY; n];
        let mut done = vec![false; n];
        let mut heap = BinaryHeap::new();
        for &(i, t0) in seeds {
            if g.depth[i].is_finite() && t0 < t[i] {
                t[i] = t0;
                heap.push(Item(t0, i));
            }
        }
        let dy = R_M * g.cell.to_radians();
        let dx_row: Vec<f64> = (0..g.height)
            .map(|r| {
                let lat = g.north - (r as f64 + 0.5) * g.cell;
                (R_M * lat.to_radians().cos() * g.cell.to_radians()).max(1.0)
            })
            .collect();
        let mut steps = 0u64;
        while let Some(Item(ti, i)) = heap.pop() {
            if done[i] || ti > t[i] {
                continue;
            }
            done[i] = true;
            if ti > t_max {
                break;
            }
            steps += 1;
            if steps.is_multiple_of(1 << 16) && cancel.load(AtomicOrdering::Relaxed) {
                return None;
            }
            let (r, c) = (i / g.width, i % g.width);
            let mut nb = [usize::MAX; 4];
            if c > 0 {
                nb[0] = i - 1;
            } else if g.wrap {
                nb[0] = i + g.width - 1;
            }
            if c + 1 < g.width {
                nb[1] = i + 1;
            } else if g.wrap {
                nb[1] = i + 1 - g.width;
            }
            if r > 0 {
                nb[2] = i - g.width;
            }
            if r + 1 < g.height {
                nb[3] = i + g.width;
            }
            for &j in &nb {
                if j == usize::MAX || done[j] || !g.depth[j].is_finite() {
                    continue;
                }
                let tj = update(g, &t, &done, j, dx_row[j / g.width], dy);
                if tj < t[j] {
                    t[j] = tj;
                    heap.push(Item(tj, j));
                }
            }
        }
        Some(t.into_iter().map(|v| v as f32).collect())
    }

    /// Hücrenin hızı (m/s); kara 0.
    pub fn speed(depth_m: f32) -> f64 {
        if depth_m.is_finite() && depth_m > 0.0 {
            (G * f64::from(depth_m)).sqrt()
        } else {
            0.0
        }
    }

    /// Birinci dereceden upwind güncellemesi: kabul edilmiş komşulardan
    /// ((T − a)/dx)² + ((T − b)/dy)² = 1/c².
    fn update(g: &Grid, t: &[f64], done: &[bool], j: usize, dx: f64, dy: f64) -> f64 {
        let (r, c) = (j / g.width, j % g.width);
        let val = |k: usize| if done[k] { t[k] } else { f64::INFINITY };
        let left = if c > 0 {
            val(j - 1)
        } else if g.wrap {
            val(j + g.width - 1)
        } else {
            f64::INFINITY
        };
        let right = if c + 1 < g.width {
            val(j + 1)
        } else if g.wrap {
            val(j + 1 - g.width)
        } else {
            f64::INFINITY
        };
        let up = if r > 0 {
            val(j - g.width)
        } else {
            f64::INFINITY
        };
        let down = if r + 1 < g.height {
            val(j + g.width)
        } else {
            f64::INFINITY
        };
        let a = left.min(right);
        let b = up.min(down);
        let f = 1.0 / speed(g.depth[j]);
        let one = (a + dx * f).min(b + dy * f);
        if !a.is_finite() || !b.is_finite() {
            return one;
        }
        // İki yönlü çözüm: (1/dx² + 1/dy²)T² − 2(a/dx² + b/dy²)T +
        // (a²/dx² + b²/dy² − f²) = 0.
        let (ax, by) = (1.0 / (dx * dx), 1.0 / (dy * dy));
        let qa = ax + by;
        let qb = -2.0 * (a * ax + b * by);
        let qc = a * a * ax + b * b * by - f * f;
        let disc = qb * qb - 4.0 * qa * qc;
        if disc < 0.0 {
            return one;
        }
        let tt = (-qb + disc.sqrt()) / (2.0 * qa);
        if tt >= a.max(b) { tt.min(one) } else { one }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn sabit_derinlik_analitik() {
            // Ekvatorda 0,02° hücreli 601 × 601 ızgara, 4000 m derinlik,
            // ortadan nokta kaynak.
            let (w, h) = (601usize, 601usize);
            let depth = vec![4000.0f32; w * h];
            let g = Grid {
                west: -6.01,
                north: 6.01,
                cell: 0.02,
                width: w,
                height: h,
                depth: &depth,
                wrap: false,
            };
            let mid = 300 * w + 300;
            let t = compute(&g, &[(mid, 0.0)], f64::INFINITY, &AtomicBool::new(false)).unwrap();
            let c = speed(4000.0);
            let cell_m = R_M * 0.02f64.to_radians();
            // Doğu: 250 hücre (enlem 0, cos ≈ 1).
            let east = f64::from(t[300 * w + 550]);
            let exact = 250.0 * cell_m / c;
            assert!((east / exact - 1.0).abs() < 0.01, "doğu {east} / {exact}");
            // Güney.
            let south = f64::from(t[550 * w + 300]);
            assert!(
                (south / exact - 1.0).abs() < 0.01,
                "güney {south} / {exact}"
            );
            // Köşegen (45°): iki yönlü güncellemeyle küçük fazlalık.
            let diag = f64::from(t[(300 + 177) * w + 300 + 177]);
            let exact_d = 177.0 * 2f64.sqrt() * cell_m / c;
            // Ölçülen: 1,0066 (2026-11-23).
            assert!(
                diag >= exact_d * 0.99 && diag < exact_d * 1.02,
                "köşegen {diag} / {exact_d}"
            );
        }

        #[test]
        fn kara_engeli_ve_sarmal() {
            // 10 × 3 ızgara, ortada kara sütunu (yalnızca alt satırda geçit),
            // sarmal açık: kaynak solda, karşıya en kısa yol sarmaldan.
            let (w, h) = (10usize, 3usize);
            let mut depth = vec![1000.0f32; w * h];
            for r in 0..2 {
                depth[r * w + 5] = f32::NAN;
            }
            let g = Grid {
                west: 0.0,
                north: 1.0,
                cell: 0.1,
                width: w,
                height: h,
                depth: &depth,
                wrap: true,
            };
            let t = compute(&g, &[(0, 0.0)], f64::INFINITY, &AtomicBool::new(false)).unwrap();
            assert!(t[5].is_infinite());
            // Sütun 9, sarmal üzerinden kaynağa komşu.
            let one = (R_M * 0.1f64.to_radians() * (0.95f64).to_radians().cos()) / speed(1000.0);
            assert!((f64::from(t[9]) / one - 1.0).abs() < 0.01);
            // İptal.
            let stop = AtomicBool::new(true);
            let big = vec![1000.0f32; 512 * 256];
            let gb = Grid {
                west: 0.0,
                north: 1.0,
                cell: 0.01,
                width: 512,
                height: 256,
                depth: &big,
                wrap: false,
            };
            assert!(compute(&gb, &[(0, 0.0)], f64::INFINITY, &stop).is_none());
        }
    }
}

/// Yakın alanda tsunami tırmanma yüksekliği (runup). Riquelme, S., Fuentes,
/// M., Hayes, G. P., Campos, J. (2015), "A rapid estimation of near-field
/// tsunami runup", JGR Solid Earth 120, 6487–6500, doi:10.1002/2015JB012218
/// (CC BY-NC-ND; data/raw/papers/). Synolakis (1987) yalnız dalga tırmanma
/// yasasının çok eğimli kıyı uzantısı (Kânoğlu & Synolakis 1998; Fuentes
/// vd. 2015).
///
/// Geçerlilik (makale): belirgin geometrili dalma-batma zonları, yakın alan;
/// doğrultu atımlı faylar, adalar, koylar, haliçler kapsam DIŞI; tek (ilk)
/// dalga, kırılma / sürtünme / rezonans yok. Sınamada en yüksek tırmanma
/// gözleme çoğunlukla yakın (Peru 2001: 8 / 8,2 m; Pisco 2007: 9,2 / 10,1 m;
/// Tokachi 2003: 6 / 4,4 m). Uygulamada "kaba tahmin" olarak gösterilir.
pub mod runup {
    /// Eş. (3): en yüksek tırmanma R (m). `h`: ilk dalganın en büyük
    /// yüksekliği (Okada düşey yer değiştirmesinin en büyüğü, m), `d`:
    /// ortalama okyanus derinliği (m), `slope`: kıyıya en yakın şelf
    /// segmentinin eğimi |b| (boyutsuz, ör. 0,0027).
    /// R = 2,831 H (H/d)^¼ (1 + H/d)^¼ |b|^(−½).
    pub fn max_runup(h: f64, d: f64, slope: f64) -> f64 {
        if h <= 0.0 || d <= 0.0 || slope <= 0.0 {
            return 0.0;
        }
        let r = h / d;
        2.831 * h * r.powf(0.25) * (1.0 + r).powf(0.25) / slope.abs().sqrt()
    }

    /// Eş. (2)'deki kıyı boyunca azalma (Lorentz): f(p) = 1 / (1 + (2p/L)²),
    /// `p` (km) en yüksek noktadan kıyı boyunca uzaklık, `l` (km) fay
    /// uzunluğu. Gelen dalga açısı bilinmiyorsa θ = 90° (makalenin "en kötü
    /// durum" önerisi): p₀ = α·y, α = √(1 + H/d).
    pub fn along_coast(p_km: f64, l_km: f64) -> f64 {
        1.0 / (1.0 + (2.0 * p_km / l_km).powi(2))
    }

    /// α = √(1 + H/d) (Eş. 2).
    pub fn alpha(h: f64, d: f64) -> f64 {
        (1.0 + h / d).sqrt()
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn esitlik_3_ve_lorentz() {
            // Elle: 2,831 · 2 · (5e-4)^¼ · (1,0005)^¼ / √0,0027 = 16,2961 m.
            let r = max_runup(2.0, 4000.0, 0.0027);
            assert!((r - 16.296_14).abs() < 1e-4, "{r}");
            // H ile (H/d)^¼ üzerinden 5/4 kuvvetle büyür.
            let r2 = max_runup(4.0, 4000.0, 0.0027);
            assert!((r2 / r - 2f64.powf(1.25) * ((1.001f64 / 1.0005).powf(0.25))).abs() < 1e-9);
            assert_eq!(max_runup(0.0, 4000.0, 0.003), 0.0);
            // f(50 km; L = 200 km) = 1 / (1 + 0,25) = 0,8.
            assert!((along_coast(50.0, 200.0) - 0.8).abs() < 1e-12);
            assert_eq!(along_coast(0.0, 100.0), 1.0);
        }
    }
}

/// Depremden tsunami kaynağı: düzgün kaymalı sonlu fay (`source::Fault`,
/// uygulamanın sarsıntı hesabıyla aynı düzlem) ve Okada (1985) ile yüzey
/// düşey yer değiştirmesi (pasif üretim: deniz yüzeyi tabanı izler;
/// Riquelme vd. 2015'in yaklaşımı).
pub mod source {
    use super::okada85;
    use crate::quake::Quake;
    use crate::rupture::Rupture;
    use crate::source::Fault;

    /// Tepe noktası seçimi: düzgün kaymada hendek boyunca yükselme hemen
    /// hemen eşit, ama üst kenar sığsa dar tepe ızgarada ±%10 dalgalanır
    /// (Tōhoku Mw 9, 2′: 4,3–5,0 m); tepe rastgele bir uca düşmesin diye
    /// en büyüğün %90'ını geçen hücrelerden merkez üssüne en yakını alınır.
    pub const PEAK_FRACTION: f64 = 0.9;

    /// Kesme modülü (Pa): Riquelme vd. (2015) ile aynı (30 GPa; s. 6492).
    pub const SHEAR_MODULUS_PA: f64 = 3.0e10;

    /// Sismik moment (N·m): Hanks & Kanamori (1979), M0 = 10^(1,5 Mw +
    /// 9,05) (docs/bilim.md "Moment bağıntısı").
    pub fn moment(mw: f64) -> f64 {
        10f64.powf(1.5 * mw + 9.05)
    }

    /// Ortalama kayma D = M0 / (μ A) (m); `area_km2` fay alanı.
    pub fn mean_slip(mw: f64, area_km2: f64) -> f64 {
        moment(mw) / (SHEAR_MODULUS_PA * area_km2 * 1e6)
    }

    /// Düşey yer değiştirme ızgarası (boylam-enlem, satır satır kuzeyden).
    #[derive(Clone, Debug)]
    pub struct Uplift {
        pub west: f64,
        pub north: f64,
        pub cell: f64,
        pub width: usize,
        pub height: usize,
        /// uz (m), yukarı pozitif.
        pub uz: Vec<f32>,
        /// En büyük |uz| (m), işareti (+1 yükselme / −1 çökme) ve konumu
        /// (boylam, enlem): en büyüğün [`PEAK_FRACTION`]'ına ulaşan
        /// hücrelerden merkez üssüne en yakını.
        pub h_max: f64,
        pub sign: f64,
        pub at: (f64, f64),
        /// Ortalama kayma (m).
        pub slip: f64,
    }

    /// Fay yüzey izdüşümünün çevresinde (her yönde `margin_km`) `cell`
    /// derecelik ızgarada uz. Okada koordinatları: x doğrultu, y doğrultunun
    /// 90° saat yönü tersi (yukarı eğim yönü), orijin ALT kenarın başı,
    /// d = üst kenar derinliği + W sin δ. U1 = D cos(rake), U2 = D sin(rake)
    /// (Okada: U2 > 0 tavan bloğu yukarı = ters faylanma).
    pub fn uplift(q: &Quake, rupture: &Rupture, cell: f64, margin_km: f64) -> Uplift {
        // Çok parçalı kırılma: her parça aynı ortalama kaymayla (toplam alan
        // üzerinden) ayrı Okada kaynağı; yer değiştirmeler toplanır
        // (doğrusal elastisite).
        let area: f64 = rupture.segments.iter().map(|f| f.length * f.width).sum();
        let slip = mean_slip(q.mw, area);
        let rake = q.rake.to_radians();
        let u = [slip * rake.cos(), slip * rake.sin(), 0.0];
        struct Seg {
            s: [f64; 2],
            dvec: [f64; 2],
            origin: [f64; 2],
            depth_bottom: f64,
            dip: f64,
            length: f64,
            width: f64,
        }
        let segs: Vec<Seg> = rupture
            .segments
            .iter()
            .map(|f| {
                let (s, dvec) = Fault::axes(f.strike);
                let (sd, cd) = f.dip.to_radians().sin_cos();
                let top = f.top_start();
                let horiz = f.width * cd;
                Seg {
                    s,
                    dvec,
                    origin: [top[0] + dvec[0] * horiz, top[1] + dvec[1] * horiz],
                    depth_bottom: f.ztor + f.width * sd,
                    dip: f.dip,
                    length: f.length,
                    width: f.width,
                }
            })
            .collect();
        // Kapsama kutusu: tüm parçaların izdüşüm köşeleri ± pay.
        let corners: Vec<(f64, f64)> = rupture
            .segments
            .iter()
            .flat_map(|f| f.surface_outline_lonlat())
            .collect();
        let km_deg = 111.195;
        let lat_mid = corners.iter().map(|c| c.1).sum::<f64>() / corners.len() as f64;
        let pad_lat = margin_km / km_deg;
        let pad_lon = margin_km / (km_deg * lat_mid.to_radians().cos().max(0.05));
        // Boylamlar merkez üssüne göre sürekli tutulur (180° yakınında).
        let unwrap = |lon: f64| {
            let mut d = lon - q.epicenter.0;
            d -= 360.0 * (d / 360.0).round();
            q.epicenter.0 + d
        };
        let west = corners
            .iter()
            .map(|c| unwrap(c.0))
            .fold(f64::INFINITY, f64::min)
            - pad_lon;
        let east = corners
            .iter()
            .map(|c| unwrap(c.0))
            .fold(f64::NEG_INFINITY, f64::max)
            + pad_lon;
        let south = corners.iter().map(|c| c.1).fold(f64::INFINITY, f64::min) - pad_lat;
        let north = corners
            .iter()
            .map(|c| c.1)
            .fold(f64::NEG_INFINITY, f64::max)
            + pad_lat;
        let west = (west / cell).floor() * cell;
        let north = (north / cell).ceil() * cell;
        let width = ((east - west) / cell).ceil() as usize;
        let height = ((north - south) / cell).ceil() as usize;
        let mut uz = vec![0f32; width * height];
        let mut h_max = 0.0f64;
        for r in 0..height {
            let lat = north - (r as f64 + 0.5) * cell;
            for c in 0..width {
                let lon = west + (c as f64 + 0.5) * cell;
                let (p, _) = rupture.lonlat_to_local((lon, lat));
                let v: f64 = segs
                    .iter()
                    .map(|g| {
                        let rel = [p[0] - g.origin[0], p[1] - g.origin[1]];
                        let x = rel[0] * g.s[0] + rel[1] * g.s[1];
                        let y = -(rel[0] * g.dvec[0] + rel[1] * g.dvec[1]);
                        okada85::displacement(
                            x,
                            y,
                            g.depth_bottom,
                            g.dip,
                            g.length,
                            g.width,
                            u,
                            okada85::POISSON_SOLID,
                        )[2]
                    })
                    .sum();
                uz[r * width + c] = v as f32;
                h_max = h_max.max(v.abs());
            }
        }
        let (mut at, mut best, mut sign) = (q.epicenter, f64::INFINITY, 1.0);
        for r in 0..height {
            let lat = north - (r as f64 + 0.5) * cell;
            for c in 0..width {
                let v = f64::from(uz[r * width + c]);
                if v.abs() >= PEAK_FRACTION * h_max {
                    let lon = west + (c as f64 + 0.5) * cell;
                    let km = crate::geo::distance_azimuth(q.epicenter, (lon, lat)).0;
                    if km < best {
                        (best, at, sign) = (km, (lon, lat), v.signum());
                    }
                }
            }
        }
        Uplift {
            west,
            north,
            cell,
            width,
            height,
            uz,
            h_max,
            sign,
            at,
            slip,
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::quake::{FaultType, Quake, Setting};

        fn thrust() -> Quake {
            let mut q = Quake::new((143.0, 38.0));
            q.mw = 8.0;
            q.depth_km = 20.0;
            q.strike = 0.0; // kuzeye; eğim doğuya (doğrultunun saat yönü)
            q.dip = 15.0;
            q.rake = 90.0;
            q.fault = FaultType::Reverse;
            q.setting = Setting::Interface;
            q
        }

        #[test]
        fn ters_fay_tavan_blogu_yukselir() {
            let q = thrust();
            let f = Rupture::from_quake(&q);
            let up = uplift(&q, &f, 0.05, 100.0);
            // Doğrultu kuzey, eğim doğuya: fay doğuya derinleşir, tavan bloğu
            // (yükselen kısım) üst kenarın doğusunda, fayın üstünde.
            let top = f.segments[0].top_edge_lonlat();
            let top_lon = (top[0].0 + top[1].0) / 2.0;
            let max_pos = up.uz.iter().cloned().fold(f32::MIN, f32::max);
            let max_neg = up.uz.iter().cloned().fold(f32::MAX, f32::min);
            assert!(max_pos > 0.5, "yükselme yok: {max_pos}");
            assert!(
                f64::from(max_pos) >= f64::from(-max_neg),
                "{max_pos} {max_neg}"
            );
            assert!(
                up.at.0 > top_lon - 0.05,
                "en büyük yükselme üst kenarın batısında: {:?} / {top_lon}",
                up.at
            );
            // Ortalama kayma makul (Mw 8, A ~ WC94).
            assert!(up.slip > 1.0 && up.slip < 20.0, "{}", up.slip);
        }

        #[test]
        fn moment_hk79() {
            // Mw 9,0 → 10^(13,5 + 9,05) = 3,548e22 N·m.
            assert!((moment(9.0) / 3.548_133_9e22 - 1.0).abs() < 1e-6);
        }
    }
}

/// Senaryo tahmini: tetikleme, kaynak, yakın alan tırmanması, varış süreleri
/// (docs/bilim.md 7.10). Derinlik bir pencere olarak verilir (uygulama ve
/// veri aracı kendi okuyucusuyla doldurur).
pub mod forecast {
    use super::{coast::CoastPoint, runup, source, travel, trigger};
    use crate::geo;
    use crate::quake::Quake;
    use crate::rupture::Rupture;
    use std::sync::atomic::AtomicBool;

    /// Kaynak hücresi eşiği: en büyük yer değiştirmeyle AYNI işaretli ve
    /// |uz| ≥ bu oran × H (FMM tohumları ve "deniz altında" denetimi).
    /// VARSAYIM (%25): %10 ve işaretsiz, Tōhoku'da kıyıdaki ~1 m çökmeyi de
    /// kaynak sayıp varışı kıyıda 0 dk veriyordu (2026-11-23).
    pub const SOURCE_FRACTION: f64 = 0.25;
    /// Şelf eğimi için eş derinlik (m) ve arama uzaklığı (km). VARSAYIM:
    /// Riquelme'nin "kıyıya en yakın segment"i yerine kıyıdan 200 m eş
    /// derinliğe ortalama eğim.
    pub const SHELF_DEPTH_M: f64 = 200.0;
    pub const SHELF_SEARCH_KM: f64 = 300.0;
    /// Yakın alan: doğrultuya dik uzaklık sınırı (km). VARSAYIM.
    pub const NEAR_ACROSS_KM: f64 = 300.0;
    /// Eğimin makul aralığı (Riquelme: 1°–2° kama; şelf 0,0027 örneği).
    pub const SLOPE_MIN: f64 = 5e-4;
    pub const SLOPE_MAX: f64 = 0.1;
    /// Korunaklı kıyı denetimi (2026-11-24): kıyı noktasından DENİZDEKİ en
    /// yakın kaynak hücresine (|uz| ≥ %25 H, [`SOURCE_FRACTION`]; 0,25°
    /// kovalarla seyreltilmiş) giden büyük çember üzerinde, ilk `SHELTER_SKIP_KM`
    /// atlandıktan sonra en az `SHELTER_LAND_KM` kesintisiz kara varsa kıyı
    /// kaynağı doğrudan görmez (koy / körfez içi, yarımadanın arkası, karşı
    /// kıyı). Riquelme vd. (2015) koy ve haliçleri kapsam dışı sayar; bu
    /// kıyılara yakın alan yüksekliği VERİLMEZ (uzak alan gibi yalnızca
    /// tehdit ve varış). Ölçütün kendisi VARSAYIM: 1′ ızgarada kıyı şeridi
    /// pürüzü (5 km) ve küçük adacıklar (< 5 km) yok sayılır.
    pub const SHELTER_SKIP_KM: f64 = 5.0;
    pub const SHELTER_LAND_KM: f64 = 20.0;

    /// Küçük ada sınırı (km): kara parçasının doğu-batı ve kuzey-güney
    /// uzanımlarının büyüğü bundan küçükse "küçük ada" (2026-11-24). Riquelme
    /// vd. (2015) adaları kapsam dışı sayar; PTWC (IOC TS 135, Bölüm II)
    /// kıyı genliği tahmininin küçük adalarda (< 30 km) fazla verdiğini
    /// belirtir. Bu adaların kıyılarına yakın alan yüksekliği VERİLMEZ.
    pub const SMALL_ISLAND_KM: f64 = 30.0;

    /// Kara hücresi `start`ın kara parçası küçük ada mı: sınırlı taşma
    /// doldurma; uzanım [`SMALL_ISLAND_KM`]'yi aşar ya da pencere kenarına
    /// değerse (kıtanın parçası olabilir) hemen `false`.
    fn small_land(depth: &Depth, start: usize) -> bool {
        let (w, h) = (depth.width, depth.height);
        let land = |i: usize| !depth.depth[i].is_finite();
        let km = depth.cell * geo::deg_to_km(1.0);
        let lat = depth.north - ((start / w) as f64 + 0.5) * depth.cell;
        let ew_km = km * lat.to_radians().cos().max(0.05);
        let mut seen = std::collections::HashSet::from([start]);
        let mut stack = vec![start];
        let (r, c) = (start / w, start % w);
        let (mut r0, mut r1, mut c0, mut c1) = (r, r, c, c);
        while let Some(i) = stack.pop() {
            let (r, c) = (i / w, i % w);
            if r == 0 || c == 0 || r + 1 == h || c + 1 == w {
                return false;
            }
            (r0, r1, c0, c1) = (r0.min(r), r1.max(r), c0.min(c), c1.max(c));
            let ns = (r1 - r0 + 1) as f64 * km;
            let ew = (c1 - c0 + 1) as f64 * ew_km;
            if ns.max(ew) >= SMALL_ISLAND_KM {
                return false;
            }
            for j in [i - w, i + w, i - 1, i + 1] {
                if land(j) && seen.insert(j) {
                    stack.push(j);
                }
            }
        }
        true
    }

    /// Kıyı noktasının (deniz hücresi) komşu kara hücreleri yalnızca küçük
    /// adalara mı ait.
    fn on_small_island(depth: &Depth, pt: (f64, f64)) -> bool {
        let Some(i) = depth.index(pt.0, pt.1) else {
            return false;
        };
        let (w, h) = (depth.width as i64, depth.height as i64);
        let (r, c) = ((i / depth.width) as i64, (i % depth.width) as i64);
        let mut any = false;
        for dr in -1..=1 {
            for dc in -1..=1 {
                let (nr, nc) = (r + dr, c + dc);
                if nr < 0 || nc < 0 || nr >= h || nc >= w {
                    continue;
                }
                let j = (nr * w + nc) as usize;
                if !depth.depth[j].is_finite() {
                    if !small_land(depth, j) {
                        return false;
                    }
                    any = true;
                }
            }
        }
        any
    }

    /// Kıyı noktası denizdeki kaynağı doğrudan görmüyor mu (bkz.
    /// [`SHELTER_SKIP_KM`]); `target`: en yakın kaynak hücresi. Pencere dışı
    /// örnekler kara sayılmaz.
    pub fn sheltered(depth: &Depth, from: (f64, f64), target: (f64, f64)) -> bool {
        let (km, az) = geo::distance_azimuth(from, target);
        let step = 1.0;
        let (mut x, mut land) = (SHELTER_SKIP_KM, 0.0);
        while x < km {
            let p = geo::destination(from, az, x);
            match depth.index(p.0, p.1) {
                Some(i) if !depth.depth[i].is_finite() => {
                    land += step;
                    if land >= SHELTER_LAND_KM {
                        return true;
                    }
                }
                _ => land = 0.0,
            }
            x += step;
        }
        false
    }

    /// Derinlik penceresi (m; NaN kara), boylam-enlem, satır satır kuzeyden.
    pub struct Depth<'a> {
        pub west: f64,
        pub north: f64,
        pub cell: f64,
        pub width: usize,
        pub height: usize,
        pub depth: &'a [f32],
        pub wrap: bool,
    }

    impl Depth<'_> {
        pub fn index(&self, lon: f64, lat: f64) -> Option<usize> {
            let mut dl = lon - self.west;
            if self.wrap {
                dl = dl.rem_euclid(360.0);
            } else if dl < 0.0 {
                // Pencere 180°'yi aşıyorsa boylam +360 ile denenir.
                dl += 360.0;
            }
            let c = (dl / self.cell).floor();
            let r = ((self.north - lat) / self.cell).floor();
            if c < 0.0 || r < 0.0 || c >= self.width as f64 || r >= self.height as f64 {
                return None;
            }
            Some(r as usize * self.width + c as usize)
        }

        pub fn at(&self, lon: f64, lat: f64) -> Option<f64> {
            let v = self.depth[self.index(lon, lat)?];
            v.is_finite().then_some(f64::from(v))
        }

        pub fn center(&self, i: usize) -> (f64, f64) {
            let (r, c) = (i / self.width, i % self.width);
            (
                self.west + (c as f64 + 0.5) * self.cell,
                self.north - (r as f64 + 0.5) * self.cell,
            )
        }
    }

    /// Bir kıyı noktasının sonucu.
    #[derive(Clone, Copy, Debug)]
    pub struct CoastResult {
        /// `coast` listesindeki sıra.
        pub point: usize,
        /// İlk varış (s); ulaşılamıyorsa INFINITY.
        pub eta_s: f64,
        /// Yakın alan tırmanması (m); uzak alanda ve korunaklı kıyıda `None`.
        pub runup_m: Option<f64>,
        /// Yakın alanda ama kaynağı doğrudan görmeyen kıyı ([`sheltered`]):
        /// yükseklik verilmez.
        pub sheltered: bool,
        /// Yakın alanda, küçük adanın kıyısı ([`SMALL_ISLAND_KM`]):
        /// yükseklik verilmez.
        pub small_island: bool,
        /// PTWC tehdit bölgesinde mi.
        pub threat: bool,
    }

    /// Tahmin.
    #[derive(Clone, Debug)]
    pub struct Forecast {
        pub assessment: trigger::Assessment,
        /// Kaynak (yalnızca "deniz altında" ve Mw ≥ 6,5 ise).
        pub h_max: f64,
        pub at: (f64, f64),
        pub slip: f64,
        pub d: f64,
        pub slope: f64,
        pub r0: f64,
        pub coast: Vec<CoastResult>,
        /// Pencerenin varış süresi ızgarası (s; `depth` ile aynı hücreler;
        /// kaynak denizde değilse boş).
        pub eta: Vec<f32>,
    }

    /// `depth`: kaynak çevresini ve tehdit bölgesini kapsayan pencere.
    /// `coast`: tüm kıyı noktaları (pencere dışındakiler atlanır). İptal:
    /// `None`.
    pub fn compute(
        q: &Quake,
        depth: &Depth,
        coast: &[CoastPoint],
        t_max_s: f64,
        cancel: &AtomicBool,
    ) -> Option<Forecast> {
        let fault = Rupture::from_quake(q);
        let up = source::uplift(q, &fault, depth.cell.max(1.0 / 60.0), 50.0);
        // Kaynak hücreleri: |uz| ≥ %10 H ve denizde.
        let mut seeds = Vec::new();
        let thr = SOURCE_FRACTION * up.h_max;
        let sign = up.sign;
        for r in 0..up.height {
            for c in 0..up.width {
                let v = f64::from(up.uz[r * up.width + c]) * sign;
                if v < thr || up.h_max <= 0.0 {
                    continue;
                }
                let lon = up.west + (c as f64 + 0.5) * up.cell;
                let lat = up.north - (r as f64 + 0.5) * up.cell;
                if let Some(i) = depth.index(lon, lat)
                    && depth.depth[i].is_finite()
                {
                    seeds.push((i, 0.0));
                }
            }
        }
        seeds.sort_unstable_by_key(|s| s.0);
        seeds.dedup_by_key(|s| s.0);
        let assessment = trigger::assess(q.mw, q.depth_km, !seeds.is_empty());
        let empty = Forecast {
            assessment,
            h_max: up.h_max,
            at: up.at,
            slip: up.slip,
            d: f64::NAN,
            slope: f64::NAN,
            r0: 0.0,
            coast: Vec::new(),
            eta: Vec::new(),
        };
        if seeds.is_empty() || q.mw < 6.5 {
            return Some(empty);
        }
        // Yükseklik zinciri (H, d, b, R0, kıyı boyunca azalma) Riquelme vd.
        // (2015) yöntemiyle: "given any moment tensor, we assume fault
        // dimensions from Blaser et al. [2010] with uniform slip" (JGR 120,
        // s. 6493); formül bu kaynakla kalibre edildi. Kullanıcının /
        // hazır senaryonun kırılma boyutu (ör. ShakeMap) daha küçükse kayma
        // ve H büyür, R0 abartılır (Tōhoku 435 × 191 km: H 8,5 m, R0 74 m;
        // gözlenen en yüksek 38,6 m, docs/dogrulama.md 2026-11-28). Varış
        // süreleri, kaynak hücreleri, tepe konumu (d, şelf eğimi, kıyı boyunca
        // konum) yine gerçek kırılmadan (`up`): küçük Blaser düzlemi odağa
        // göre yerleşince tepesi kıyıya kayıyor, d düşüp R0 şişiyordu
        // (Maule hazır senaryosu d 1530 → 320 m, R0 15 → 31 m). Riquelme
        // kaynağından yalnız H (ve kıyı boyunca azalmadaki L) alınır.
        // Yalnız dalma-batma arayüzünde: makalenin kapsamı ve kalibrasyonu
        // bu; kabuk içi (Noto) Blaser boyutuyla sığ suda R0 57 m veriyordu
        // (gözlenen 6,6 m), orada önceki gibi uygulamanın düzlemi.
        let rq = if q.setting == crate::quake::Setting::Interface {
            riquelme_quake(q)
        } else {
            *q
        };
        let rfault = Rupture::from_quake(&rq);
        let up_r = source::uplift(&rq, &rfault, depth.cell.max(1.0 / 60.0), 50.0);
        // d: en büyük yükselmenin derinliği; karadaysa tohumların medyanı.
        let d = depth.at(up.at.0, up.at.1).unwrap_or_else(|| {
            let mut v: Vec<f32> = seeds.iter().map(|s| depth.depth[s.0]).collect();
            v.sort_by(f32::total_cmp);
            f64::from(v[v.len() / 2])
        });
        let d = d.max(50.0);
        // En yakın kıyı noktası ve şelf eğimi.
        let near = coast
            .iter()
            .map(|p| geo::distance_azimuth(up.at, (f64::from(p.lon), f64::from(p.lat))))
            .enumerate()
            .min_by(|a, b| a.1.0.total_cmp(&b.1.0));
        let slope = near.map_or(SLOPE_MIN, |(k, (km, _))| {
            let p = &coast[k];
            let start = (f64::from(p.lon), f64::from(p.lat));
            let d0 = f64::from(p.depth_m);
            // Kıyıdan kaynağa doğru 1 km adımla 200 m eş derinliği ara.
            let toward = geo::distance_azimuth(start, up.at).1;
            let mut found = None;
            let lim = SHELF_SEARCH_KM.min(km.max(1.0));
            let mut x = 1.0;
            while x <= lim {
                let pt = geo::destination(start, toward, x);
                if let Some(dd) = depth.at(pt.0, pt.1)
                    && dd >= SHELF_DEPTH_M
                {
                    found = Some((dd - d0) / (x * 1000.0));
                    break;
                }
                x += 1.0;
            }
            found
                .unwrap_or_else(|| (d - d0).max(1.0) / (km.max(1.0) * 1000.0))
                .clamp(SLOPE_MIN, SLOPE_MAX)
        });
        let r0 = runup::max_runup(up_r.h_max, d, slope);
        let alpha = runup::alpha(up_r.h_max, d);
        // Varış süreleri.
        let g = travel::Grid {
            west: depth.west,
            north: depth.north,
            cell: depth.cell,
            width: depth.width,
            height: depth.height,
            depth: depth.depth,
            wrap: depth.wrap,
        };
        let eta = travel::compute(&g, &seeds, t_max_s, cancel)?;
        let (sx, sy) = {
            let a = fault.strike.to_radians();
            (a.sin(), a.cos())
        };
        // Korunaklı kıyı hedefleri: kaynak hücreleri 0,25° kovalarda birer
        // temsilciye seyreltilir (en yakın hücre araması ucuz kalsın).
        let targets: Vec<(f64, f64)> = {
            let mut keys = std::collections::HashSet::new();
            seeds
                .iter()
                .map(|s| depth.center(s.0))
                .filter(|c| keys.insert(((c.0 * 4.0).floor() as i64, (c.1 * 4.0).floor() as i64)))
                .collect()
        };
        let threat_radius = match assessment {
            trigger::Assessment::Threat { radius_km } => Some(radius_km),
            _ => None,
        };
        let mut out = Vec::new();
        for (k, p) in coast.iter().enumerate() {
            let pt = (f64::from(p.lon), f64::from(p.lat));
            let Some(i) = depth.index(pt.0, pt.1) else {
                continue;
            };
            let t = f64::from(eta[i]);
            if !t.is_finite() {
                continue;
            }
            let (km, az) = geo::distance_azimuth(up.at, pt);
            let a = az.to_radians();
            let (ex, ny) = (km * a.sin(), km * a.cos());
            let along = ex * sx + ny * sy;
            let across = (ex * sy - ny * sx).abs();
            let near =
                across <= NEAR_ACROSS_KM && fault.fault != crate::quake::FaultType::StrikeSlip;
            let shelter = near && {
                let target = targets
                    .iter()
                    .min_by(|a, b| geo::distance_km(pt, **a).total_cmp(&geo::distance_km(pt, **b)))
                    .copied()
                    .unwrap_or(up.at);
                sheltered(depth, pt, target)
            };
            let island = near && !shelter && on_small_island(depth, pt);
            let runup_m = (near && !shelter && !island)
                .then(|| r0 * runup::along_coast(alpha * along, rfault.length));
            let threat = match threat_radius {
                Some(Some(r)) => geo::distance_azimuth(q.epicenter, pt).0 <= r,
                Some(None) => t <= trigger::BASIN_ETA_S,
                None => false,
            };
            out.push(CoastResult {
                point: k,
                eta_s: t,
                runup_m,
                sheltered: shelter,
                small_island: island,
                threat,
            });
        }
        Some(Forecast {
            assessment,
            d,
            slope,
            r0,
            h_max: up_r.h_max,
            at: up.at,
            slip: up_r.slip,
            coast: out,
            eta,
        })
    }

    /// Riquelme vd. (2015) yükseklik kaynağı: aynı merkez üssü, derinlik,
    /// doğrultu / eğim / kayma açısı ve odak konumu; tek düzlem, boyut
    /// Blaser vd. (2010): log L = −2,81 + 0,62 Mw, log W = −1,79 + 0,45 Mw
    /// (km; Riquelme vd. 2015 JGR 120, s. 6496, "the scaling laws was taken
    /// from Blaser et al. [2010]"). Genişlik arayüz sınırına sığmazsa
    /// `Rupture::from_quake` kırpar (uygulamanın sarsıntı düzlemiyle aynı).
    pub fn riquelme_quake(q: &Quake) -> Quake {
        let mut r = *q;
        r.trace = None;
        r.rupture_km = Some([blaser_length_km(q.mw), blaser_width_km(q.mw)]);
        r
    }

    pub fn blaser_length_km(mw: f64) -> f64 {
        10f64.powf(-2.81 + 0.62 * mw)
    }

    pub fn blaser_width_km(mw: f64) -> f64 {
        10f64.powf(-1.79 + 0.45 * mw)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        /// Blaser vd. (2010) Riquelme vd. (2015)'teki biçimiyle: Mw 9,1 →
        /// L ≈ 679 km, W ≈ 202 km; Mw 8,0 → L ≈ 141 km, W ≈ 65 km.
        #[test]
        fn blaser_boyutlari() {
            assert!((blaser_length_km(9.1) - 679.2).abs() < 1.0);
            assert!((blaser_width_km(9.1) - 201.8).abs() < 0.5);
            assert!((blaser_length_km(8.0) - 141.3).abs() < 0.5);
            assert!((blaser_width_km(8.0) - 64.6).abs() < 0.2);
        }

        /// 3° × 1° deniz (1′), arada `wall` derece genişlikte kara şeridi
        /// (1,0°D'den başlayarak).
        fn grid(wall: f64) -> Vec<f32> {
            let (w, h) = (180, 60);
            (0..w * h)
                .map(|i| {
                    let lon = (i % w) as f64 / 60.0 + 1.0 / 120.0;
                    if (1.0..1.0 + wall).contains(&lon) {
                        f32::NAN
                    } else {
                        1000.0
                    }
                })
                .collect()
        }

        fn depth(v: &[f32]) -> Depth<'_> {
            Depth {
                west: 0.0,
                north: 1.0,
                cell: 1.0 / 60.0,
                width: 180,
                height: 60,
                depth: v,
                wrap: false,
            }
        }

        /// Kaynakla kıyı arasında ~44 km kara: korunaklı. ~11 km (dar
        /// yarımada; dalga dolaşır): korunaklı DEĞİL. Pencere dışı kara
        /// sayılmaz.
        #[test]
        fn korunakli_kiyi() {
            let wide = grid(0.4);
            assert!(sheltered(&depth(&wide), (0.5, 0.5), (2.5, 0.5)));
            let narrow = grid(0.1);
            assert!(!sheltered(&depth(&narrow), (0.5, 0.5), (2.5, 0.5)));
            let sea = grid(0.0);
            assert!(!sheltered(&depth(&sea), (0.5, 0.5), (2.5, 0.5)));
            // Hedef pencere dışında (kuzeyde): yol pencereden çıkınca kara yok.
            assert!(!sheltered(&depth(&sea), (0.5, 0.5), (0.5, 3.0)));
        }

        /// ~11 km'lik ada küçük, ~44 km'lik şerit (pencerenin kuzey-güney
        /// kenarına değiyor) büyük; adanın kıyısındaki deniz hücresi "küçük
        /// ada kıyısı".
        #[test]
        fn kucuk_ada() {
            let mut v = grid(0.0);
            // 0,1° × 0,1° ada (2,0–2,1°D, 0,45–0,55°K).
            for r in 0..60 {
                for c in 0..180 {
                    let lon = c as f64 / 60.0 + 1.0 / 120.0;
                    let lat = 1.0 - (r as f64 + 0.5) / 60.0;
                    if (2.0..2.1).contains(&lon) && (0.45..0.55).contains(&lat) {
                        v[r * 180 + c] = f32::NAN;
                    }
                }
            }
            let d = depth(&v);
            assert!(small_land(&d, d.index(2.05, 0.5).unwrap()));
            assert!(on_small_island(&d, (1.99, 0.5)));
            assert!(!on_small_island(&d, (1.5, 0.5)));
            let wall = grid(0.4);
            let dw = depth(&wall);
            assert!(!small_land(&dw, dw.index(1.2, 0.5).unwrap()));
            assert!(!on_small_island(&dw, (0.99, 0.5)));
        }
    }
}
