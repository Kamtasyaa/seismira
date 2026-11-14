//! Noktasal sarsıntı hesabı: bir deprem için şehir/saha başına mesafe,
//! yer hareketi, şiddet ve varış zamanları (CLAUDE.md Bölüm 7.3, 7.4, 7.6).
//!
//! Hepsi simülasyon başında bir kez hesaplanır (Bölüm 11); zamanla değişen
//! görünüm (kutunun ne zaman belireceği vb.) bu değerlerin saf fonksiyonudur.

use crate::geo;
use crate::gmpe::{Imt, LnMotion, cy14};
use crate::intensity::{self, Shindo};
use crate::quake::{Quake, Setting};
use crate::site::{REFERENCE_VS30, Site};
use crate::source::{Distances, Fault};
use crate::traveltime::{Phase, TravelTimes};

/// Bir saha için sonuç.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SiteResult {
    pub dist: Distances,
    /// Saha koşulu (Vs30 haritasından ya da referans).
    pub site: Site,
    /// Hesapta kullanılan Vs30: model aralığına ([`cy14::VS30_MIN`],
    /// [`cy14::VS30_MAX`]) çekilmiş değer. `site.vs30`'dan farklıysa
    /// arayüz "model sınırına çekildi" der (Bölüm 6: sessiz ekstrapolasyon yok).
    pub vs30_used: f64,
    /// İlk P ve ilk S varışı (s). Tablo sınırı (~100°) ötesinde `None`.
    pub t_p: Option<f64>,
    pub t_s: Option<f64>,
    /// Model geçerliyse yer hareketi ve şiddet; değilse `None` ("model dışı").
    pub motion: Option<Motion>,
}

/// S varışından sonra gösterilen Shindo'nun bir sınıf yükselme süresi (s).
/// Kutu 1'den başlar, her adımda bir sınıf artarak son değere varır; güçlü
/// sarsıntı daha çok adım, dolayısıyla daha geç tepe demektir. GÖRSEL bir
/// kabuldür (kullanıcı isteği, 2026-11-14): gerçek kayıtlarda şiddet S
/// varışından sonra saniyeler içinde birikerek artar, ama bu süre burada
/// modellenmiyor. Faz 5'te alt fay zamanlaması (t_tepe) ile fiziksel tepe
/// zamanına bağlanacak.
pub const RISE_STEP_S: f64 = 0.4;

impl SiteResult {
    /// `t` anında gösterilecek Shindo: S ulaşmadıysa, model dışıysa ya da
    /// son değer 1'in altındaysa `None`.
    pub fn shindo_at(&self, t: f64) -> Option<Shindo> {
        let fin = self.motion?.shindo();
        let ts = self.t_s?;
        if t < ts || fin < Shindo::S1 {
            return None;
        }
        let step = ((t - ts) / RISE_STEP_S).floor() as usize;
        let k = (Shindo::S1 as usize + step).min(fin as usize);
        Some(Shindo::ALL[k])
    }

    /// Gösterilen Shindo'nun son değere ulaştığı an.
    pub fn t_peak(&self) -> Option<f64> {
        let fin = self.motion?.shindo();
        let steps = (fin as usize).saturating_sub(Shindo::S1 as usize);
        Some(self.t_s? + steps as f64 * RISE_STEP_S)
    }
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

    fn motion(&self, d: &Distances, vs30: f64) -> Option<Motion> {
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
            vs30,
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

    /// Saha sonucu. `site` yalnızca yer hareketi hesaplanacaksa (model
    /// ortamı var ve Rrup ≤ sınır) çağrılır; Vs30 haritası okuması pahalı
    /// olduğundan uzak şehirlerde referans zemin yazılır.
    pub fn site(&self, lonlat: (f64, f64), site: impl FnOnce() -> Site) -> SiteResult {
        let dist = self.fault.distances(lonlat);
        let site = if has_gmpe(self.quake.setting) && dist.rrup <= cy14::MAX_RRUP_KM {
            site()
        } else {
            Site::reference()
        };
        let deg = geo::km_to_deg(dist.repi);
        let h = self.quake.depth_km;
        let vs30_used = site.vs30.clamp(cy14::VS30_MIN, cy14::VS30_MAX);
        SiteResult {
            t_p: self.tt.time(Phase::P, h, deg),
            t_s: self.tt.time(Phase::S, h, deg),
            motion: self.motion(&dist, vs30_used),
            dist,
            site,
            vs30_used,
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
            // Görsel sınır: referans zemin için (sahaya bağlı değil).
            match self.motion(&d, REFERENCE_VS30) {
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
        let near = s.site((29.92, 40.77), Site::reference).motion.unwrap();
        let far = s.site((32.85, 39.93), Site::reference).motion.unwrap(); // Ankara ~260 km
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
            let r = s.site(site, Site::reference);
            assert!(r.t_s.unwrap() > r.t_p.unwrap());
        }
    }

    #[test]
    fn model_disi_mesafe_ve_ortam() {
        let t = tt();
        let s = Scenario::new(izmit(), &t);
        assert!(s.site((2.35, 48.85), Site::reference).motion.is_none()); // Paris: > 300 km
        let mut q = izmit();
        q.setting = Setting::Interface;
        q.fault = FaultType::Reverse;
        assert!(
            Scenario::new(q, &t)
                .site((29.92, 40.77), Site::reference)
                .motion
                .is_none()
        );
    }

    #[test]
    fn yumusak_zemin_daha_guclu_ve_kirpma_isaretli() {
        use crate::site::Vs30Source;
        let t = tt();
        let s = Scenario::new(izmit(), &t);
        let at = |vs30| {
            let site = Site {
                vs30,
                vs30_ln_sd: None,
                source: Vs30Source::Map,
            };
            s.site((30.40, 40.78), || site)
        };
        let soft = at(200.0);
        let rock = at(1000.0);
        assert!(soft.motion.unwrap().pgv.median() > rock.motion.unwrap().pgv.median());
        // 100 m/s modelin altında: 180'e çekilir, hesap yine yapılır.
        let very_soft = at(100.0);
        assert_eq!(very_soft.vs30_used, 180.0);
        assert!(very_soft.motion.is_some());
        assert_eq!(at(2000.0).vs30_used, 1500.0);
    }

    #[test]
    fn shindo_kademeli_yukselir() {
        let t = tt();
        let s = Scenario::new(izmit(), &t);
        let r = s.site((29.92, 40.77), Site::reference);
        let fin = r.motion.unwrap().shindo();
        let ts = r.t_s.unwrap();
        assert_eq!(r.shindo_at(ts - 0.01), None);
        assert_eq!(r.shindo_at(ts), Some(Shindo::S1));
        assert_eq!(r.shindo_at(ts + RISE_STEP_S), Some(Shindo::S2));
        // Hiç azalmaz, son değeri geçmez ve tepe anında ona ulaşır.
        let peak = r.t_peak().unwrap();
        let mut last = Shindo::S0;
        let mut k = 0.0;
        while ts + k <= peak + 2.0 {
            let now = r.shindo_at(ts + k).unwrap();
            assert!(now >= last && now <= fin);
            last = now;
            k += 0.05;
        }
        assert_eq!(r.shindo_at(peak), Some(fin));
        assert!(r.shindo_at(peak - 0.01).unwrap() < fin);
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
