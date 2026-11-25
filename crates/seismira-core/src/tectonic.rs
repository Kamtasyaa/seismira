//! Kararlı kıta içi bölgeler (docs/bilim.md 7.4): sığ kabuk yer hareketi
//! modelleri (NGA-West2) aktif kabuk verisiyle kurulmuştur; kararlı kıta
//! içinde sönüm farklıdır (ör. Doğu Kuzey Amerika). İlk sürümde özel model
//! yok; merkez üssü bu bölgelerdeyse arayüz açıkça uyarır.
//!
//! Kaynak: USGS STREC (Seismo-Tectonic Regime Earthquake Calculator),
//! `strec/data/stable.geojson`, commit 6e13af7f (github.com/usgs/strec,
//! arşivlenmiş depo), kamu malı (depodaki license.txt). Ayrıntı ve SHA256:
//! data/SOURCES.toml "usgs-strec-stable".
//!
//! Dosya: `data/regions/stable.sout`, [`crate::region::write_outlines`]
//! biçiminde TEK bölge; noktalar Web Mercator değil, (boylam, enlem)
//! derecedir.

use std::io::{self, Read};

use crate::quake::{Issue, Quake, Setting};
use crate::region::{Ring, read_outlines};

/// Kararlı kıta içi bölge çokgenleri (boylam, enlem; derece).
#[derive(Clone, Debug, Default)]
pub struct StableRegions {
    rings: Vec<Ring>,
}

impl StableRegions {
    pub fn new(rings: Vec<Ring>) -> Self {
        Self { rings }
    }

    pub fn read(r: &mut impl Read) -> io::Result<Self> {
        let mut out = read_outlines(r)?;
        Ok(Self {
            rings: out.pop().unwrap_or_default(),
        })
    }

    /// Nokta bölgelerden birinin içinde mi? Çift-tek kuralı tüm halkalar
    /// üzerinden: delikler (ör. kararlı alan içindeki aktif yarık) kendiliğinden
    /// dışarıda kalır. Boylam [−180, 180)'e sarılır.
    pub fn contains(&self, lon: f64, lat: f64) -> bool {
        let lon = (lon + 180.0).rem_euclid(360.0) - 180.0;
        let p = [lon as f32, lat as f32];
        self.rings
            .iter()
            .filter(|r| r.points.len() >= 3)
            .fold(false, |inside, r| {
                inside ^ crate::region::point_in_ring(&r.points, p)
            })
    }
}

/// Sığ kabuk depremi kararlı kıta içinde ise uyarı ekler (engellemez).
pub fn check(q: &Quake, stable: &StableRegions, out: &mut Vec<Issue>) {
    if q.setting == Setting::ShallowCrust && stable.contains(q.epicenter.0, q.epicenter.1) {
        out.push(Issue::StableContinental);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(x0: f32, y0: f32, x1: f32, y1: f32, hole: bool) -> Ring {
        Ring {
            hole,
            points: vec![[x0, y0], [x1, y0], [x1, y1], [x0, y1], [x0, y0]],
        }
    }

    #[test]
    fn icerde_disarda_delik() {
        let s = StableRegions::new(vec![
            square(0.0, 0.0, 10.0, 10.0, false),
            square(4.0, 4.0, 6.0, 6.0, true),
        ]);
        assert!(s.contains(2.0, 2.0));
        assert!(!s.contains(5.0, 5.0));
        assert!(!s.contains(12.0, 2.0));
        // Boylam sarma: 362° = 2°.
        assert!(s.contains(362.0, 2.0));
    }

    #[test]
    fn yalnizca_sig_kabukta_uyarir() {
        let s = StableRegions::new(vec![square(0.0, 0.0, 10.0, 10.0, false)]);
        let mut q = Quake::new((2.0, 2.0));
        let mut out = Vec::new();
        check(&q, &s, &mut out);
        assert_eq!(out, vec![Issue::StableContinental]);
        assert!(!Issue::StableContinental.blocks());
        out.clear();
        q.setting = Setting::Intraslab;
        check(&q, &s, &mut out);
        assert!(out.is_empty());
        q.setting = Setting::ShallowCrust;
        q.epicenter = (20.0, 2.0);
        check(&q, &s, &mut out);
        assert!(out.is_empty());
    }

    /// Depodaki dosya (data/regions/stable.sout, STREC): bilinen kararlı
    /// kıta içi depremler içeride, aktif bölgeler dışarıda.
    #[test]
    fn strec_dosyasi() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/regions/stable.sout"
        );
        let s = StableRegions::read(&mut io::BufReader::new(
            std::fs::File::open(path).expect("stable.sout"),
        ))
        .unwrap();
        for (name, lon, lat) in [
            ("Meckering 1968", 116.98, -31.6),
            ("New Madrid", -89.6, 36.6),
            ("Virginia 2011", -77.9, 37.9),
            ("Bhuj 2001", 70.3, 23.4),
            ("Kanada kalkanı", -90.0, 55.0),
        ] {
            assert!(s.contains(lon, lat), "{name}");
        }
        for (name, lon, lat) in [
            ("İzmit", 29.9, 40.7),
            ("Kahramanmaraş", 37.0, 37.2),
            ("Ridgecrest", -117.6, 35.8),
            ("Kumamoto", 130.8, 32.8),
            ("Noto", 137.2, 37.5),
        ] {
            assert!(!s.contains(lon, lat), "{name}");
        }
    }
}
