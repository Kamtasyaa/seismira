//! Tsunami (Faz 8; docs/bilim.md 7.10).
//!
//! - [`bathy`]: uygulamanın okuduğu derinlik ızgarası (`data/tsunami/bathy.sras`).
//! - [`coast`]: kıyı tahmin noktaları (`data/tsunami/coast.bin`).
//! - [`okada85`]: sonlu dikdörtgen faydan yüzey yer değiştirmesi.

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
