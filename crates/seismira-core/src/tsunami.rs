//! Tsunami (Faz 8; docs/bilim.md 7.10).
//!
//! - [`bathy`]: uygulamanın okuduğu derinlik ızgarası (`data/tsunami/bathy.sras`).
//! - [`coast`]: kıyı tahmin noktaları (`data/tsunami/coast.bin`).
//! - [`okada85`]: sonlu dikdörtgen faydan yüzey yer değiştirmesi.
//! - [`trigger`]: ilk değerlendirme (PTWC ölçütleri).
//! - [`travel`]: varış süresi haritası (fast marching, √(g·h)).
//! - [`runup`]: yakın alanda en yüksek tırmanma (Riquelme vd. 2015).
//! - [`source`]: depremden deniz tabanı düşey yer değiştirmesi (Okada).

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
    use crate::source::Fault;

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
        /// En büyük |uz| (m) ve konumu (boylam, enlem).
        pub h_max: f64,
        pub at: (f64, f64),
        /// Ortalama kayma (m).
        pub slip: f64,
    }

    /// Fay yüzey izdüşümünün çevresinde (her yönde `margin_km`) `cell`
    /// derecelik ızgarada uz. Okada koordinatları: x doğrultu, y doğrultunun
    /// 90° saat yönü tersi (yukarı eğim yönü), orijin ALT kenarın başı,
    /// d = üst kenar derinliği + W sin δ. U1 = D cos(rake), U2 = D sin(rake)
    /// (Okada: U2 > 0 tavan bloğu yukarı = ters faylanma).
    pub fn uplift(q: &Quake, fault: &Fault, cell: f64, margin_km: f64) -> Uplift {
        let (s, dvec) = Fault::axes(fault.strike);
        let (sd, cd) = fault.dip.to_radians().sin_cos();
        let top = fault.top_start();
        let horiz = fault.width * cd;
        let origin = [top[0] + dvec[0] * horiz, top[1] + dvec[1] * horiz];
        let depth_bottom = fault.ztor + fault.width * sd;
        let slip = mean_slip(q.mw, fault.length * fault.width);
        let rake = q.rake.to_radians();
        let u = [slip * rake.cos(), slip * rake.sin(), 0.0];
        // Kapsama kutusu: izdüşüm köşeleri ± pay.
        let corners = fault.surface_outline_lonlat();
        let km_deg = 111.195;
        let lat_mid = corners.iter().map(|c| c.1).sum::<f64>() / 4.0;
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
        let (mut h_max, mut at) = (0.0f64, q.epicenter);
        for r in 0..height {
            let lat = north - (r as f64 + 0.5) * cell;
            for c in 0..width {
                let lon = west + (c as f64 + 0.5) * cell;
                let (p, _) = fault.lonlat_to_local((lon, lat));
                let rel = [p[0] - origin[0], p[1] - origin[1]];
                let x = rel[0] * s[0] + rel[1] * s[1];
                let y = -(rel[0] * dvec[0] + rel[1] * dvec[1]);
                let d = okada85::displacement(
                    x,
                    y,
                    depth_bottom,
                    fault.dip,
                    fault.length,
                    fault.width,
                    u,
                    okada85::POISSON_SOLID,
                );
                let v = d[2];
                uz[r * width + c] = v as f32;
                if v.abs() > h_max {
                    h_max = v.abs();
                    at = (lon, lat);
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
            let f = Fault::from_quake(&q);
            let up = uplift(&q, &f, 0.05, 100.0);
            // Doğrultu kuzey, eğim doğuya: fay doğuya derinleşir, tavan bloğu
            // (yükselen kısım) üst kenarın doğusunda, fayın üstünde.
            let top = f.top_edge_lonlat();
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
