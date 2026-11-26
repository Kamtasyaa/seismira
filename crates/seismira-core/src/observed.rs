//! Gözlenen istasyon kayıtlarıyla karşılaştırma (README "Gerçek depremler
//! için devamı"): her istasyonda gözlenen PGV ile uygulamanın tahmini.
//!
//! Yöntem `seismira-data validate-shakemap`'in "C" karşılaştırmasıyla aynı
//! (docs/dogrulama.md): tahmin uygulamanın tam zinciriyle ([`Scenario::site`]:
//! fay geometrisi, Vs30 haritası, model ortalaması); gözlem iki yatay kanalın
//! geometrik ortalaması, RotD50 yerine kullanılır (Boore 2010: ikisi pratikte
//! çok yakın). İkisi de aynı dönüşümle JMA aletsel şiddetine çevrilir
//! ([`intensity::jma_from_pgv_rotd50`]); fark ΔI = I_göz − I_tah, yani
//! 1,82 · log10(PGV_göz / PGV_tah). ΔI > 0: model az tahmin ediyor.

use crate::intensity;
use crate::shaking::Scenario;
use crate::site::Site;

/// Bir istasyonun gözlemi.
#[derive(Clone, Debug, PartialEq)]
pub struct Station {
    /// Ağ ve istasyon kodu (ör. "TK.4614").
    pub code: String,
    /// (boylam, enlem), derece.
    pub lonlat: (f64, f64),
    /// Gözlenen PGV (cm/s), iki yatay kanalın geometrik ortalaması.
    pub pgv_cm_s: f64,
}

/// Bir istasyonda karşılaştırma.
#[derive(Clone, Debug, PartialEq)]
pub struct Residual {
    pub code: String,
    pub lonlat: (f64, f64),
    /// Kırılmaya en kısa mesafe (km).
    pub rrup: f64,
    pub obs_pgv: f64,
    /// Tahmin edilen medyan PGV (cm/s, RotD50).
    pub pred_pgv: f64,
    /// Gözlenen ve tahmin edilen JMA aletsel şiddeti.
    pub obs_jma: f64,
    pub pred_jma: f64,
}

impl Residual {
    /// ΔI = I_göz − I_tah (JMA birimi).
    pub fn delta(&self) -> f64 {
        self.obs_jma - self.pred_jma
    }

    /// ln(PGV_göz / PGV_tah).
    pub fn ln_ratio(&self) -> f64 {
        (self.obs_pgv / self.pred_pgv).ln()
    }
}

/// Mesafe dilimi (km): `validate-shakemap` ile aynı sınırlar.
pub const BINS_KM: [(f64, f64); 6] = [
    (0.0, 25.0),
    (25.0, 50.0),
    (50.0, 100.0),
    (100.0, 150.0),
    (150.0, 200.0),
    (200.0, 300.0),
];

#[derive(Clone, Debug, PartialEq)]
pub struct Bin {
    pub lo_km: f64,
    pub hi_km: f64,
    pub n: usize,
    /// Dilimde ortalama ΔI; boşsa `None`.
    pub mean_delta: Option<f64>,
}

/// Özet.
#[derive(Clone, Debug, PartialEq)]
pub struct Summary {
    pub n: usize,
    pub mean_delta: f64,
    pub mean_ln: f64,
    /// ln(göz/tah) standart sapması (n − 1 ile).
    pub std_ln: f64,
    /// |ΔI| < 1 olan istasyonların oranı (0–1).
    pub within_one: f64,
    pub bins: Vec<Bin>,
}

/// Her istasyon için tahmin. `site` istasyonun zeminini verir (uygulamada
/// kullanıcı Vs30 → Vs30 haritası → referans). Model dışı (mesafe, ortam)
/// istasyonlar atlanır; gözlemi sıfır ya da geçersiz olanlar da.
pub fn compare(
    sc: &Scenario,
    stations: &[Station],
    site: impl Fn((f64, f64)) -> Site,
) -> Vec<Residual> {
    let mw = sc.quake.mw;
    stations
        .iter()
        .filter(|s| s.pgv_cm_s.is_finite() && s.pgv_cm_s > 0.0)
        .filter_map(|s| {
            let r = sc.site(s.lonlat, || site(s.lonlat));
            let m = r.motion?;
            let rrup = r.dist.rrup;
            Some(Residual {
                code: s.code.clone(),
                lonlat: s.lonlat,
                rrup,
                obs_pgv: s.pgv_cm_s,
                pred_pgv: m.pgv.median(),
                obs_jma: intensity::jma_from_pgv_rotd50(s.pgv_cm_s, mw, rrup),
                pred_jma: m.jma,
            })
        })
        .collect()
}

/// Özet istatistikler; boş girdide `None`.
pub fn summarize(rows: &[Residual]) -> Option<Summary> {
    if rows.is_empty() {
        return None;
    }
    let n = rows.len();
    let mean = |f: &dyn Fn(&Residual) -> f64| rows.iter().map(f).sum::<f64>() / n as f64;
    let mean_delta = mean(&|r| r.delta());
    let mean_ln = mean(&|r| r.ln_ratio());
    let std_ln = if n > 1 {
        (rows
            .iter()
            .map(|r| (r.ln_ratio() - mean_ln).powi(2))
            .sum::<f64>()
            / (n - 1) as f64)
            .sqrt()
    } else {
        0.0
    };
    let within_one = rows.iter().filter(|r| r.delta().abs() < 1.0).count() as f64 / n as f64;
    let bins = BINS_KM
        .iter()
        .map(|&(lo, hi)| {
            let d: Vec<f64> = rows
                .iter()
                .filter(|r| r.rrup >= lo && r.rrup < hi)
                .map(Residual::delta)
                .collect();
            Bin {
                lo_km: lo,
                hi_km: hi,
                n: d.len(),
                mean_delta: (!d.is_empty()).then(|| d.iter().sum::<f64>() / d.len() as f64),
            }
        })
        .collect();
    Some(Summary {
        n,
        mean_delta,
        mean_ln,
        std_ln,
        within_one,
        bins,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quake::Quake;
    use crate::traveltime::TravelTimes;

    fn tt() -> TravelTimes {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/traveltime/ak135.ttt"
        );
        TravelTimes::load(std::path::Path::new(path)).expect("seyahat süresi tablosu")
    }

    /// Gözlem tahminin tam kendisiyse ΔI = 0; iki katıysa
    /// ΔI = 1,82 · log10 2 ≈ 0,548 (Midorikawa vd. 1999 eğimi).
    #[test]
    fn fark_tanimi() {
        let tt = tt();
        let q = Quake::new((30.0, 40.0));
        let sc = Scenario::new(q, &tt);
        let at = [(30.2, 40.1), (30.5, 40.3), (31.0, 40.0)];
        let probe: Vec<Station> = at
            .iter()
            .map(|&p| Station {
                code: "X".into(),
                lonlat: p,
                pgv_cm_s: 1.0,
            })
            .collect();
        let pred = compare(&sc, &probe, |_| Site::reference());
        assert_eq!(pred.len(), 3);
        for (k, factor) in [1.0, 2.0, 0.5].into_iter().enumerate() {
            let s = Station {
                code: "X".into(),
                lonlat: at[k],
                pgv_cm_s: pred[k].pred_pgv * factor,
            };
            let r = &compare(&sc, &[s], |_| Site::reference())[0];
            let expect = 1.82 * factor.log10();
            assert!((r.delta() - expect).abs() < 1e-9, "{} {expect}", r.delta());
            assert!((r.ln_ratio() - factor.ln()).abs() < 1e-9);
        }
    }

    /// Model dışı (Rrup > 300 km) ve geçersiz gözlemler atlanır; özet
    /// dilimleri doğru sayar.
    #[test]
    fn atlama_ve_ozet() {
        let tt = tt();
        let sc = Scenario::new(Quake::new((30.0, 40.0)), &tt);
        let st = |p: (f64, f64), v: f64| Station {
            code: "X".into(),
            lonlat: p,
            pgv_cm_s: v,
        };
        let rows = compare(
            &sc,
            &[
                st((30.1, 40.0), 20.0),
                st((30.0, 40.6), 5.0),
                st((36.0, 40.0), 1.0), // ~510 km
                st((30.2, 40.0), 0.0),
                st((30.2, 40.0), f64::NAN),
            ],
            |_| Site::reference(),
        );
        assert_eq!(rows.len(), 2);
        let s = summarize(&rows).unwrap();
        assert_eq!(s.n, 2);
        assert_eq!(s.bins.iter().map(|b| b.n).sum::<usize>(), 2);
        let mean = (rows[0].delta() + rows[1].delta()) / 2.0;
        assert!((s.mean_delta - mean).abs() < 1e-12);
        assert!(summarize(&[]).is_none());
    }
}
