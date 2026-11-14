//! Noktasal sarsıntı hesabı: bir deprem için şehir/saha başına mesafe,
//! yer hareketi, şiddet ve varış zamanları (CLAUDE.md Bölüm 7.3, 7.4, 7.6).
//!
//! Hepsi simülasyon başında bir kez hesaplanır (Bölüm 11); zamanla değişen
//! görünüm (kutunun ne zaman belireceği vb.) bu değerlerin saf fonksiyonudur.

use crate::geo;
use crate::gmpe::{Imt, LnMotion, cy14};
use crate::intensity::{self, Shindo};
use crate::quake::{Quake, Setting};
use crate::source::{Distances, Fault};
use crate::traveltime::{Phase, TravelTimes};

/// Faz 4'te Vs30 haritası gelene kadar tüm sahalar için sabit zemin.
/// 400 m/s: J-SHIS'in "mühendislik ana kayası" (工学的基盤) tanımı; tipik
/// şehir zeminine 760 m/s kayadan daha yakın. GEÇİCİ.
pub const REFERENCE_VS30: f64 = 400.0;

/// Bir saha için sonuç.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SiteResult {
    pub dist: Distances,
    /// İlk P ve ilk S varışı (s). Tablo sınırı (~100°) ötesinde `None`.
    pub t_p: Option<f64>,
    pub t_s: Option<f64>,
    /// Model geçerliyse yer hareketi ve şiddet; değilse `None` ("model dışı").
    pub motion: Option<Motion>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Motion {
    pub pga: LnMotion,
    pub pgv: LnMotion,
    /// JMA aletsel şiddeti (medyan) ve standart sapması (şiddet birimi).
    pub jma: f64,
    pub jma_sigma: f64,
}

impl Motion {
    pub fn shindo(&self) -> Shindo {
        Shindo::from_instrumental(self.jma)
    }

    /// "Olası aralık": ±1σ (Bölüm 7.7).
    pub fn shindo_range(&self) -> (Shindo, Shindo) {
        (
            Shindo::from_instrumental(self.jma - self.jma_sigma),
            Shindo::from_instrumental(self.jma + self.jma_sigma),
        )
    }
}

/// Bir depremin hesap bağlamı.
pub struct Scenario<'a> {
    pub quake: Quake,
    pub fault: Fault,
    tt: &'a TravelTimes,
}

/// Yer hareketi modeli bu ortam için hazır mı? (Faz 3: yalnızca sığ kabuk.)
pub fn has_gmpe(setting: Setting) -> bool {
    setting == Setting::ShallowCrust
}

impl<'a> Scenario<'a> {
    pub fn new(quake: Quake, tt: &'a TravelTimes) -> Self {
        Self {
            fault: Fault::from_quake(&quake),
            quake,
            tt,
        }
    }

    fn motion(&self, d: &Distances) -> Option<Motion> {
        if !has_gmpe(self.quake.setting) {
            return None;
        }
        let input = cy14::Input {
            mw: self.quake.mw,
            rrup: d.rrup,
            rjb: d.rjb,
            rx: d.rx,
            ztor: self.fault.ztor,
            dip: self.fault.dip,
            fault: self.quake.fault,
            vs30: REFERENCE_VS30,
        };
        let pga = cy14::predict(Imt::Pga, &input)?;
        let pgv = cy14::predict(Imt::Pgv, &input)?;
        Some(Motion {
            pga,
            pgv,
            jma: intensity::jma_from_pgv_rotd50(pgv.median()),
            jma_sigma: intensity::jma_sigma(pgv.sigma),
        })
    }

    pub fn site(&self, lonlat: (f64, f64)) -> SiteResult {
        let dist = self.fault.distances(lonlat);
        let deg = geo::km_to_deg(dist.repi);
        let h = self.quake.depth_km;
        SiteResult {
            t_p: self.tt.time(Phase::P, h, deg),
            t_s: self.tt.time(Phase::S, h, deg),
            motion: self.motion(&dist),
            dist,
        }
    }

    /// Hissedilme sınırı (episantr açısı, derece): bu mesafenin ötesinde
    /// tahmini Shindo 1'in altında. Halkalar ötesinde soluk çizilir
    /// (Bölüm 7.2). Fay doğrultusuna dik, taban bloğu yönünde bir profil
    /// boyunca aranır ve fayın yarı uzunluğu eklenir (görsel amaçlı).
    pub fn felt_radius_deg(&self) -> Option<f64> {
        if !has_gmpe(self.quake.setting) {
            return None;
        }
        let mut last_felt = None;
        let mut r = 0.0;
        while r <= 300.0 {
            let rrup = (r * r + self.fault.ztor * self.fault.ztor).sqrt();
            let d = Distances {
                rrup,
                rjb: r,
                rx: -r,
                repi: r,
            };
            match self.motion(&d) {
                Some(m) if m.jma >= 0.5 => last_felt = Some(r),
                Some(_) => break,
                None => break,
            }
            r += 2.0;
        }
        last_felt.map(|r| geo::km_to_deg(r + self.fault.length / 2.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quake::FaultType;
    use std::path::Path;

    fn tt() -> TravelTimes {
        let p = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/traveltime/ak135.ttt"
        );
        TravelTimes::load(Path::new(p)).unwrap()
    }

    fn izmit() -> Quake {
        let mut q = Quake::new((29.97, 40.76));
        q.mw = 7.6;
        q.depth_km = 15.0;
        q.fault = FaultType::StrikeSlip;
        q.strike = 90.0;
        q
    }

    #[test]
    fn yakin_saha_guclu_uzak_saha_zayif() {
        let t = tt();
        let s = Scenario::new(izmit(), &t);
        let near = s.site((29.92, 40.77)).motion.unwrap();
        let far = s.site((32.85, 39.93)).motion.unwrap(); // Ankara ~260 km
        assert!(near.shindo() >= Shindo::S6Lower, "{:?}", near);
        assert!(far.jma < near.jma);
        let (lo, hi) = near.shindo_range();
        assert!(lo <= near.shindo() && near.shindo() <= hi);
    }

    #[test]
    fn s_her_zaman_p_den_sonra() {
        let t = tt();
        let s = Scenario::new(izmit(), &t);
        for site in [(29.0, 41.0), (35.0, 39.0), (2.35, 48.85)] {
            let r = s.site(site);
            assert!(r.t_s.unwrap() > r.t_p.unwrap());
        }
    }

    #[test]
    fn model_disi_mesafe_ve_ortam() {
        let t = tt();
        let s = Scenario::new(izmit(), &t);
        assert!(s.site((2.35, 48.85)).motion.is_none()); // Paris: > 300 km
        let mut q = izmit();
        q.setting = Setting::Interface;
        q.fault = FaultType::Reverse;
        assert!(Scenario::new(q, &t).site((29.92, 40.77)).motion.is_none());
    }

    #[test]
    fn hissedilme_siniri_buyuklukle_artar() {
        let t = tt();
        let mut q = izmit();
        let big = Scenario::new(q, &t).felt_radius_deg().unwrap();
        q.mw = 5.0;
        let small = Scenario::new(q, &t).felt_radius_deg().unwrap();
        assert!(big > small && small > 0.0);
    }
}
