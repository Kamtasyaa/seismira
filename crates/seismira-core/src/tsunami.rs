//! Tsunami (Faz 8; docs/bilim.md 7.10).
//!
//! - [`bathy`]: uygulamanın okuduğu derinlik ızgarası (`data/tsunami/bathy.sras`).

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
