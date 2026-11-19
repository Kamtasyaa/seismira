//! Noktasal sarsıntı hesabı: bir deprem için şehir/saha başına mesafe,
//! yer hareketi, şiddet ve varış zamanları (CLAUDE.md Bölüm 7.3, 7.4, 7.6).
//!
//! Hepsi simülasyon başında bir kez hesaplanır (Bölüm 11); zamanla değişen
//! görünüm (kutunun ne zaman belireceği vb.) bu değerlerin saf fonksiyonudur.

use crate::geo;
use crate::gmpe::psbah20::{self, EventType};
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
    /// Hesapta kullanılan Vs30: modelin aralığına ([`Gmm::vs30_range`])
    /// çekilmiş değer. `site.vs30`'dan farklıysa arayüz "model sınırına
    /// çekildi" der (Bölüm 6: sessiz ekstrapolasyon yok).
    pub vs30_used: f64,
    /// İlk P ve ilk S varışı (s). Tablo sınırı (~100°) ötesinde `None`.
    pub t_p: Option<f64>,
    pub t_s: Option<f64>,
    /// Model geçerliyse yer hareketi ve şiddet; değilse `None` ("model dışı").
    pub motion: Option<Motion>,
    /// Zemin belirsizliğinin şiddete etkisi (JMA birimi): Vs30, haritanın
    /// ±1 log standart sapması kadar değiştiğinde aletsel şiddetteki
    /// değişimin yarısı. Haritada SD yoksa ya da model dışıysa `None`.
    ///
    /// "Olası aralık"a AYRICA EKLENMEZ: CY14'ün σ'sı Vs30'u ölçülmemiş
    /// ("inferred") sahalar için zaten büyütülmüştür (Eş. 13, σ3 terimi);
    /// üstüne eklemek aynı belirsizliği iki kez saymak olur. Bilgi olarak
    /// şehir kartında gösterilir (docs/bilim.md 7.7).
    pub jma_vs30_effect: Option<f64>,
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
        // Küçük tolerans: `t_peak` anında (ts + n·adım) kayan nokta hatası
        // n − 1'e yuvarlamasın (macOS libm'de görüldü, CI 2026-11-14).
        let step = ((t - ts) / RISE_STEP_S + 1e-9).floor() as usize;
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
    /// MMI (medyan, 1–10; Worden vd. 2012) ve standart sapması.
    pub mmi: f64,
    pub mmi_sigma: f64,
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

    /// MMI için "olası aralık": ±1σ, 1–10 içinde.
    pub fn mmi_range(&self) -> (f64, f64) {
        (
            (self.mmi - self.mmi_sigma).clamp(1.0, 10.0),
            (self.mmi + self.mmi_sigma).clamp(1.0, 10.0),
        )
    }
}

/// Bir depremin yer hareketi modeli (Bölüm 7.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gmm {
    /// Sığ kabuk: Chiou & Youngs (2014).
    Cy14,
    /// Dalma-batma arayüzü ve levha içi: Parker vd. (2020), küresel.
    Psbah20(EventType),
}

impl Gmm {
    /// Depreme uygun model; yoksa `None` (yalnızca dalgalar gösterilir).
    /// Levha içinde modelin derinlik sınırının ötesi model dışıdır (girdi
    /// denetimi bunu [`crate::quake::Issue::DeepFocus`] ile uyarır).
    pub fn for_quake(q: &Quake) -> Option<Gmm> {
        match q.setting {
            Setting::ShallowCrust => Some(Gmm::Cy14),
            Setting::Interface => Some(Gmm::Psbah20(EventType::Interface)),
            Setting::Intraslab => (q.depth_km <= psbah20::INTRASLAB_ZHYP_MAX)
                .then_some(Gmm::Psbah20(EventType::Intraslab)),
        }
    }

    /// Modelin mesafe sınırı (km, Rrup); ötesi "model dışı".
    pub fn max_rrup_km(self) -> f64 {
        match self {
            Gmm::Cy14 => cy14::MAX_RRUP_KM,
            Gmm::Psbah20(_) => psbah20::MAX_RRUP_KM,
        }
    }

    /// Modelin Vs30 geçerlilik aralığı (m/s); harita değeri buna çekilir.
    pub fn vs30_range(self) -> (f64, f64) {
        match self {
            Gmm::Cy14 => (cy14::VS30_MIN, cy14::VS30_MAX),
            Gmm::Psbah20(_) => (psbah20::VS30_MIN, psbah20::VS30_MAX),
        }
    }
}

/// Bu deprem için yer hareketi modeli var mı?
pub fn has_gmpe(q: &Quake) -> bool {
    Gmm::for_quake(q).is_some()
}

/// Bir depremin hesap bağlamı.
pub struct Scenario<'a> {
    pub quake: Quake,
    pub fault: Fault,
    /// Yer hareketi modeli; yoksa yalnızca varış zamanları hesaplanır.
    pub gmm: Option<Gmm>,
    tt: &'a TravelTimes,
}

impl<'a> Scenario<'a> {
    pub fn new(quake: Quake, tt: &'a TravelTimes) -> Self {
        Self {
            fault: Fault::from_quake(&quake),
            gmm: Gmm::for_quake(&quake),
            quake,
            tt,
        }
    }

    /// Modelin mesafe sınırı (km); model yoksa `None`.
    pub fn max_rrup_km(&self) -> Option<f64> {
        self.gmm.map(Gmm::max_rrup_km)
    }

    fn motion(&self, d: &Distances, vs30: f64) -> Option<Motion> {
        let (pga, pgv) = match self.gmm? {
            Gmm::Cy14 => {
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
                (
                    cy14::predict(Imt::Pga, &input)?,
                    cy14::predict(Imt::Pgv, &input)?,
                )
            }
            Gmm::Psbah20(event) => {
                let input = psbah20::Input {
                    mw: self.quake.mw,
                    rrup: d.rrup,
                    zhyp: self.quake.depth_km,
                    vs30,
                    event,
                    // Bölge seçimi Slab2 ile gelecek (Faz 5); şimdilik küresel.
                    region: psbah20::Region::Global,
                };
                (
                    psbah20::predict(Imt::Pga, &input)?,
                    psbah20::predict(Imt::Pgv, &input)?,
                )
            }
        };
        let (mw, v) = (self.quake.mw, pgv.median());
        Some(Motion {
            pga,
            pgv,
            jma: intensity::jma_from_pgv_rotd50(v, mw, d.rrup),
            jma_sigma: intensity::jma_sigma(pgv.sigma),
            mmi: crate::mmi::mmi_from_pgv_rotd50(v, mw, d.rrup),
            mmi_sigma: crate::mmi::mmi_sigma(v, pgv.sigma, mw, d.rrup),
        })
    }

    /// Saha sonucu. `site` yalnızca yer hareketi hesaplanacaksa (model
    /// ortamı var ve Rrup ≤ sınır) çağrılır; Vs30 haritası okuması pahalı
    /// olduğundan uzak şehirlerde referans zemin yazılır.
    pub fn site(&self, lonlat: (f64, f64), site: impl FnOnce() -> Site) -> SiteResult {
        let dist = self.fault.distances(lonlat);
        let site = match self.max_rrup_km() {
            Some(max) if dist.rrup <= max => site(),
            _ => Site::reference(),
        };
        let deg = geo::km_to_deg(dist.repi);
        let h = self.quake.depth_km;
        let (vs_min, vs_max) = self.gmm.map_or((0.0, f64::INFINITY), Gmm::vs30_range);
        let vs30_used = site.vs30.clamp(vs_min, vs_max);
        let motion = self.motion(&dist, vs30_used);
        let jma_vs30_effect = motion.zip(site.vs30_ln_sd).and_then(|(_, sd)| {
            let at = |f: f64| {
                let v = (site.vs30 * f).clamp(vs_min, vs_max);
                self.motion(&dist, v).map(|m| m.jma)
            };
            // Yumuşak zemin (Vs30 küçük) daha güçlü sarsılır.
            Some((at((-sd).exp())? - at(sd.exp())?) / 2.0)
        });
        SiteResult {
            t_p: self.tt.time(Phase::P, h, deg),
            t_s: self.tt.time(Phase::S, h, deg),
            motion,
            jma_vs30_effect,
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
        let max = self.max_rrup_km()?;
        let mut last_felt = None;
        let mut r = 0.0;
        while r <= max {
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
        assert_eq!(s.gmm, Some(Gmm::Cy14));
        assert!(s.site((2.35, 48.85), Site::reference).motion.is_none()); // Paris: > 300 km
        // Levha içinde modelin derinlik sınırı (200 km) ötesi: model yok,
        // varış zamanları yine var.
        let mut q = izmit();
        q.setting = Setting::Intraslab;
        q.depth_km = 250.0;
        let s = Scenario::new(q, &t);
        assert_eq!(s.gmm, None);
        assert!(s.felt_radius_deg().is_none());
        let r = s.site((29.92, 40.77), Site::reference);
        assert!(r.motion.is_none() && r.t_s.is_some());
        q.depth_km = 200.0;
        assert_eq!(
            Scenario::new(q, &t).gmm,
            Some(Gmm::Psbah20(EventType::Intraslab))
        );
    }

    /// 2011 Tōhoku benzeri arayüz depremi (Mw 9.0, odak 24 km, doğrultu
    /// 195°, eğim 10°; geometri Faz 5 ölçeklemesine kadar kaba). Parker vd.
    /// (2020) bağlı: kıyıda güçlü, ~1000 km'ye kadar hesap, ötesi model dışı.
    #[test]
    fn dalma_batma_arayuz_depremi() {
        let t = tt();
        let mut q = Quake::new((142.37, 38.30));
        q.setting = Setting::Interface;
        q.fault = FaultType::Reverse;
        q.mw = 9.0;
        q.depth_km = 24.0;
        q.strike = 195.0;
        q.dip = 10.0;
        q.rake = 90.0;
        let s = Scenario::new(q, &t);
        assert_eq!(s.gmm, Some(Gmm::Psbah20(EventType::Interface)));
        assert_eq!(s.max_rrup_km(), Some(1000.0));
        let sendai = s.site((140.87, 38.27), Site::reference).motion.unwrap();
        let tokyo = s.site((139.69, 35.69), Site::reference).motion.unwrap();
        assert!(sendai.shindo() >= Shindo::S5Lower, "{sendai:?}");
        assert!(
            tokyo.jma < sendai.jma && tokyo.shindo() >= Shindo::S3,
            "{tokyo:?}"
        );
        // Pekin ~2000 km: model dışı.
        assert!(s.site((116.40, 39.90), Site::reference).motion.is_none());
        // Hissedilme sınırı kabuk modelinin 300 km'sini aşar.
        assert!(s.felt_radius_deg().unwrap() > geo::km_to_deg(300.0));
        // Vs30 Parker aralığına (150–2000) çekilir, CY14'ünkine değil.
        use crate::site::Vs30Source;
        let at = |vs30| {
            let site = Site {
                vs30,
                vs30_ln_sd: None,
                source: Vs30Source::Map,
            };
            s.site((140.87, 38.27), || site).vs30_used
        };
        assert_eq!(at(160.0), 160.0);
        assert_eq!(at(2100.0), 2000.0);
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
        // Harita SD'si yoksa zemin etkisi hesaplanmaz.
        assert!(soft.jma_vs30_effect.is_none());
    }

    #[test]
    fn zemin_belirsizliginin_siddete_etkisi() {
        use crate::site::Vs30Source;
        let t = tt();
        let s = Scenario::new(izmit(), &t);
        let at = |vs30: f64, sd: f64| {
            let site = Site {
                vs30,
                vs30_ln_sd: Some(sd),
                source: Vs30Source::Map,
            };
            s.site((30.40, 40.78), || site)
        };
        // Elle: Vs30·e^∓sd için iki ayrı hesabın farkının yarısı.
        let r = at(400.0, 0.4);
        let direct = |v: f64| {
            let site = Site {
                vs30: v,
                vs30_ln_sd: None,
                source: Vs30Source::Map,
            };
            s.site((30.40, 40.78), || site).motion.unwrap().jma
        };
        let want = (direct(400.0 * (-0.4f64).exp()) - direct(400.0 * 0.4f64.exp())) / 2.0;
        let got = r.jma_vs30_effect.unwrap();
        assert!((got - want).abs() < 1e-12 && got > 0.0, "{got} {want}");
        // Belirsizlik büyüdükçe etki büyür; sıfır SD'de sıfırdır.
        assert!(at(400.0, 0.5).jma_vs30_effect.unwrap() > got);
        assert_eq!(at(400.0, 0.0).jma_vs30_effect, Some(0.0));
        // Model sınırında (180 m/s) alt uç kırpılır: etki küçülür.
        assert!(at(180.0, 0.4).jma_vs30_effect.unwrap() < at(400.0, 0.4).jma_vs30_effect.unwrap());
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
