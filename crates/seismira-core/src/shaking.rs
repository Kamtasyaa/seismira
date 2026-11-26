//! Noktasal sarsıntı hesabı: bir deprem için şehir/saha başına mesafe,
//! yer hareketi, şiddet ve varış zamanları (CLAUDE.md Bölüm 7.3, 7.4, 7.6).
//!
//! Hepsi simülasyon başında bir kez hesaplanır (Bölüm 11); zamanla değişen
//! görünüm (kutunun ne zaman belireceği vb.) bu değerlerin saf fonksiyonudur.

use crate::geo;
use crate::gmpe::psbah20::{self, EventType};
use crate::gmpe::{CrustRegion, Imt, LnMotion, ngaw2};
use crate::intensity::{self, Shindo};
use crate::quake::{Quake, Setting};
use crate::rupture::Rupture;
use crate::site::{REFERENCE_VS30, Site};
use crate::slab::{self, Slab2};
use crate::source::{self, Distances, PeakPath};
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
    /// Gösterilen Shindo'nun bir sınıf yükselme süresi (s): en az
    /// [`RISE_STEP_S`]; fiziksel tepe anı (t_tepe, [`Scenario::physical_peak`])
    /// daha geçse son değere tam o anda varacak şekilde uzar.
    pub rise: f64,
    /// Dalma-batma depreminde saha yay ardında (Slab2, [`Slab2::is_backarc`]).
    /// Parker vd. (2020) yalnızca yay önü sahalar için kurulmuştur ve yay
    /// ardı terimi yoktur (PEER 2020/03, 8.3.2); şiddet yine hesaplanır ama
    /// arayüz bunu belirtir. Japonya'da yay ardında daha güçlü sönümlenme
    /// gözlenmiştir; Tōhoku 2011 istasyonlarında fark küçük ama aynı yönde
    /// (docs/dogrulama.md).
    pub backarc: bool,
}

/// S varışından sonra gösterilen Shindo'nun EN KISA bir sınıf yükselme
/// süresi (s). Kutu 1'den başlar, her adımda bir sınıf artarak son değere
/// varır. GÖRSEL alt sınır (kullanıcı isteği, 2026-11-14): odağa yakın
/// sahalarda fiziksel tepe S varışıyla neredeyse aynı anda olsa da yükseliş
/// görünür kalır. Uzak uçtaki sahalarda adım t_tepe'ye göre uzar (Faz 5).
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
        let step = ((t - ts) / self.rise + 1e-9).floor() as usize;
        let k = (Shindo::S1 as usize + step).min(fin as usize);
        Some(Shindo::ALL[k])
    }

    /// Gösterilen Shindo'nun son değere ulaştığı an.
    pub fn t_peak(&self) -> Option<f64> {
        let fin = self.motion?.shindo();
        let steps = (fin as usize).saturating_sub(Shindo::S1 as usize);
        Some(self.t_s? + steps as f64 * self.rise)
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
    /// Sığ kabuk: dört NGA-West2 modelinin eşit ağırlıklı ortalaması
    /// (ASK14, BSSA14, CB14, CY14; [`ngaw2`]), bölgesel düzeltmelerle.
    Crust,
    /// Dalma-batma arayüzü ve levha içi: Parker vd. (2020), küresel.
    Psbah20(EventType),
}

impl Gmm {
    /// Depreme uygun model; yoksa `None` (yalnızca dalgalar gösterilir).
    /// Levha içinde modelin derinlik sınırının ötesi model dışıdır (girdi
    /// denetimi bunu [`crate::quake::Issue::DeepFocus`] ile uyarır).
    pub fn for_quake(q: &Quake) -> Option<Gmm> {
        match q.setting {
            Setting::ShallowCrust => Some(Gmm::Crust),
            Setting::Interface => Some(Gmm::Psbah20(EventType::Interface)),
            Setting::Intraslab => (q.depth_km <= psbah20::INTRASLAB_ZHYP_MAX)
                .then_some(Gmm::Psbah20(EventType::Intraslab)),
        }
    }

    /// Modelin mesafe sınırı (km, Rrup); ötesi "model dışı".
    pub fn max_rrup_km(self) -> f64 {
        match self {
            Gmm::Crust => ngaw2::MAX_RRUP_KM,
            Gmm::Psbah20(_) => psbah20::MAX_RRUP_KM,
        }
    }

    /// Modelin Vs30 geçerlilik aralığı (m/s); harita değeri buna çekilir.
    pub fn vs30_range(self) -> (f64, f64) {
        match self {
            Gmm::Crust => (ngaw2::VS30_MIN, ngaw2::VS30_MAX),
            Gmm::Psbah20(_) => (psbah20::VS30_MIN, psbah20::VS30_MAX),
        }
    }
}

/// Bu deprem için yer hareketi modeli var mı?
pub fn has_gmpe(q: &Quake) -> bool {
    Gmm::for_quake(q).is_some()
}

/// Spektrum periyotları (s): 0,02–6 s, logaritmik 46 nokta (TBDY T_L = 6 s'ye
/// kadar; NGA-West2 ve Parker vd. 2020 tabloları 10 s'ye kadar).
pub const SPECTRUM_PERIODS: [f64; 46] = {
    let mut p = [0.0; 46];
    // 0,02 · 300^(k/45): sabit ifade için önceden hesaplanmış çarpan
    // tablosu yerine döngü (const fn'de powf yok): adım oranı r = 300^(1/45).
    let r = 1.135_134_018_185_281_4_f64;
    let mut k = 0;
    let mut v = 0.02;
    while k < 46 {
        p[k] = v;
        v *= r;
        k += 1;
    }
    p
};

/// [`Scenario::spectrum`] sonucu.
#[derive(Clone, Debug)]
pub struct SiteSpectrum {
    pub dist: Distances,
    /// Hesapta kullanılan Vs30 (modelin aralığına çekilmiş).
    pub vs30: f64,
    pub pga: Option<LnMotion>,
    /// (periyot s, ln SA).
    pub sa: Vec<(f64, Option<LnMotion>)>,
}

/// Bir depremin hesap bağlamı.
pub struct Scenario<'a> {
    pub quake: Quake,
    pub fault: Rupture,
    /// Yer hareketi modeli; yoksa yalnızca varış zamanları hesaplanır.
    pub gmm: Option<Gmm>,
    /// Dalma-batmada merkez üssünün altındaki Slab2 zonu (bulunduysa).
    pub slab_zone: Option<usize>,
    /// Parker vd. (2020) bölgesi: zondan ([`slab::parker_region`]); zon
    /// yoksa küresel.
    pub region: psbah20::Region,
    /// Sığ kabuk modellerinin bölgesi (merkez üssünün ülkesi;
    /// [`Scenario::with_crust_region`]). Varsayılan: küresel.
    pub crust_region: CrustRegion,
    slab: Option<&'a Slab2>,
    tt: &'a TravelTimes,
}

impl<'a> Scenario<'a> {
    /// Slab2 olmadan: dalma-batmada küresel model, yay ardı ayrımı yok.
    pub fn new(quake: Quake, tt: &'a TravelTimes) -> Self {
        Self::with_slab(quake, tt, None)
    }

    pub fn with_slab(quake: Quake, tt: &'a TravelTimes, slab: Option<&'a Slab2>) -> Self {
        let gmm = Gmm::for_quake(&quake);
        let (lon, lat) = quake.epicenter;
        let slab_zone = match gmm {
            Some(Gmm::Psbah20(_)) => slab
                .and_then(|m| m.locate(lon, lat, quake.depth_km, quake.setting))
                .map(|h| h.zone),
            _ => None,
        };
        let region = match (slab, slab_zone) {
            (Some(m), Some(z)) => slab::parker_region(m.code(z), lon, lat),
            _ => psbah20::Region::Global,
        };
        Self {
            fault: {
                // Yönelim (CY14 ΔDPP): yalnızca sığ kabukta ve Mw > 5,5'te
                // (CY14 f_M = 0 altında; dalma-batma modelinde terim yok).
                let r = Rupture::from_quake(&quake);
                if quake.setting == crate::quake::Setting::ShallowCrust && quake.mw > 5.5 {
                    r.with_directivity(quake.rake)
                } else {
                    r
                }
            },
            gmm,
            slab_zone,
            region,
            crust_region: CrustRegion::Global,
            slab,
            quake,
            tt,
        }
    }

    /// Sığ kabuk bölgesini ayarlar (ör. [`crate::region::country_near`] ile
    /// bulunan ülke kodundan [`CrustRegion::from_iso`]).
    pub fn with_crust_region(mut self, region: CrustRegion) -> Self {
        self.crust_region = region;
        self
    }

    /// Modelin mesafe sınırı (km); model yoksa `None`.
    pub fn max_rrup_km(&self) -> Option<f64> {
        self.gmm.map(Gmm::max_rrup_km)
    }

    /// Herhangi bir yer hareketi ölçüsü (ln, g ya da cm/s): depremin
    /// modeliyle (kabukta dört NGA-West2 ortalaması, dalma-batmada Parker vd.
    /// 2020; bölgeleriyle). SA'da tabloda
    /// olmayan periyot log-log ara değerle ([`crate::gmpe`]). Model yoksa,
    /// mesafe ya da girdi aralık dışıysa `None`. Bileşen: RotD50.
    pub fn ln_motion(&self, imt: Imt, d: &Distances, vs30: f64) -> Option<LnMotion> {
        match self.gmm? {
            Gmm::Crust => ngaw2::predict(
                imt,
                &ngaw2::Input {
                    mw: self.quake.mw,
                    rrup: d.rrup,
                    rjb: d.rjb,
                    rx: d.rx,
                    ztor: self.fault.ztor,
                    zhyp: self.quake.depth_km,
                    dip: self.fault.dip,
                    width: self.fault.width,
                    fault: self.quake.fault,
                    vs30,
                    region: self.crust_region,
                    dpp: d.dpp,
                },
            ),
            Gmm::Psbah20(event) => psbah20::predict(
                imt,
                &psbah20::Input {
                    mw: self.quake.mw,
                    rrup: d.rrup,
                    zhyp: self.quake.depth_km,
                    vs30,
                    event,
                    region: self.region,
                },
            ),
        }
    }

    /// Bir noktanın %5 sönümlü tepki spektrumu (RotD50, g): [`SPECTRUM_PERIODS`]
    /// periyotlarında depremin modeliyle (`ln_motion`, tabloda olmayan
    /// periyotlar log-log ara değer). Model dışı (mesafe, girdi) `None`.
    /// Dönen: mesafeler, kullanılan Vs30 (modele çekilmiş), PGA ve spektrum.
    pub fn spectrum(&self, lonlat: (f64, f64), vs30: f64) -> SiteSpectrum {
        let d = self.fault.distances(lonlat);
        let v = self.clamp_vs30(vs30);
        let in_range = self.max_rrup_km().is_some_and(|m| d.rrup <= m);
        let get = |imt| in_range.then(|| self.ln_motion(imt, &d, v)).flatten();
        SiteSpectrum {
            dist: d,
            vs30: v,
            pga: get(Imt::Pga),
            sa: SPECTRUM_PERIODS
                .iter()
                .map(|&t| (t, get(Imt::Sa(t))))
                .collect(),
        }
    }

    /// Anlamlı süre D5-95 medyanı (s) bir sahada: kabukta Afshari & Stewart
    /// (2016), dalma-batmada Bahrampouri vd. (2021) ([`crate::duration`];
    /// ızgaradaki t_bitiş ile aynı seçim).
    pub fn d595(&self, rrup: f64, vs30_used: f64) -> f64 {
        let q = &self.quake;
        match self.gmm {
            Some(Gmm::Psbah20(event)) => {
                crate::duration::bahrampouri2021::d595(q.mw, rrup, vs30_used, event, q.fault).0
            }
            _ => crate::duration::d595(q.mw, rrup, vs30_used, q.fault).0,
        }
    }

    /// Hesapta kullanılan Vs30: modelin aralığına çekilmiş.
    pub fn clamp_vs30(&self, vs30: f64) -> f64 {
        let (lo, hi) = self.gmm.map_or((0.0, f64::INFINITY), Gmm::vs30_range);
        vs30.clamp(lo, hi)
    }

    fn motion(&self, d: &Distances, vs30: f64) -> Option<Motion> {
        let pga = self.ln_motion(Imt::Pga, d, vs30)?;
        let pgv = self.ln_motion(Imt::Pgv, d, vs30)?;
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
    /// Fiziksel tepe anı t_tepe (Bölüm 7.3): sahaya en yakın fay noktasının
    /// kırılma zamanı (odaktan düzlem üzerinde uzaklık / kırılma hızı,
    /// [`source::rupture_velocity_kms`]) + o noktadan sahaya ak135 S süresi.
    /// "En yakın alt fay" tanımının sürekli hali: düzlem ayrık alt faylara
    /// bölünmez, en yakın nokta doğrudan bulunur. Tablo dışında `None`.
    pub fn physical_peak(&self, path: &PeakPath) -> Option<f64> {
        let t_rup = path.rupture_km / source::rupture_velocity_kms(self.quake.fault);
        let ts = self
            .tt
            .time(Phase::S, path.depth_km, geo::km_to_deg(path.horiz_km))?;
        Some(t_rup + ts)
    }

    pub fn site(&self, lonlat: (f64, f64), site: impl FnOnce() -> Site) -> SiteResult {
        let (dist, path) = self.fault.distances_and_peak(lonlat);
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
        let t_s = self.tt.time(Phase::S, h, deg);
        // Yükseliş adımı: son değere t_tepe'de varılır (en az RISE_STEP_S).
        let rise = match (motion, t_s, self.physical_peak(&path)) {
            (Some(m), Some(ts), Some(peak)) => {
                let steps = (m.shindo() as usize).saturating_sub(Shindo::S1 as usize);
                if steps == 0 {
                    RISE_STEP_S
                } else {
                    ((peak - ts) / steps as f64).max(RISE_STEP_S)
                }
            }
            _ => RISE_STEP_S,
        };
        let backarc = motion.is_some()
            && self
                .slab
                .zip(self.slab_zone)
                .and_then(|(m, z)| m.is_backarc(z, lonlat.0, lonlat.1))
                .unwrap_or(false);
        SiteResult {
            backarc,
            t_p: self.tt.time(Phase::P, h, deg),
            t_s,
            rise,
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
                dpp: 0.0,
            };
            // Görsel sınır: referans zemin için (sahaya bağlı değil).
            match self.motion(&d, REFERENCE_VS30) {
                Some(m) if m.jma >= 0.5 => last_felt = Some(r),
                Some(_) => break,
                None => break,
            }
            r += 2.0;
        }
        // Tek düzlemde fayın yarı uzunluğu (önceki davranış); çok parçalıda
        // merkez üssünden kırılmanın en uzak köşesi.
        let pad = if self.fault.segments.len() == 1 {
            self.fault.length / 2.0
        } else {
            self.fault.max_extent_km()
        };
        last_felt.map(|r| geo::km_to_deg(r + pad))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Spektrum: periyotlar 0,02–6 s; yakın sahada her periyotta değer;
    /// kısa periyot ucu PGA'ya yakın (0,02 s), 6 s'de çok daha küçük.
    #[test]
    fn spektrum() {
        assert!((SPECTRUM_PERIODS[0] - 0.02).abs() < 1e-12);
        assert!(
            (SPECTRUM_PERIODS[45] - 6.0).abs() < 1e-6,
            "{}",
            SPECTRUM_PERIODS[45]
        );
        let t = tt();
        let mut q = Quake::new((29.97, 40.7));
        q.mw = 7.4;
        let s = Scenario::new(q, &t);
        let sp = s.spectrum((30.2, 40.6), 400.0);
        assert!(sp.sa.iter().all(|(_, m)| m.is_some()));
        let pga = sp.pga.unwrap().median();
        let sa002 = sp.sa[0].1.unwrap().median();
        let sa6 = sp.sa[45].1.unwrap().median();
        assert!((sa002 / pga - 1.0).abs() < 0.15, "{sa002} {pga}");
        assert!(sa6 < 0.2 * pga, "{sa6} {pga}");
        // Model sınırı dışı: hepsi None.
        let far = s.spectrum((45.0, 40.0), 400.0);
        assert!(far.pga.is_none() && far.sa.iter().all(|(_, m)| m.is_none()));
    }
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
        assert_eq!(s.gmm, Some(Gmm::Crust));
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

    /// Alt fay zamanlaması (t_tepe): fayın uzak ucunun ötesindeki saha son
    /// değerine, kırılma oraya varıp S dalgası geldiğinde ulaşır; odak
    /// yakınında kural en kısa adımdır.
    #[test]
    fn uzak_uctaki_saha_tepeye_kirilma_ile_ulasir() {
        let t = tt();
        let mut q = izmit();
        q.mw = 7.8; // ~320 km'lik doğrultu atımlı fay, doğuya (90°)
        let s = Scenario::new(q, &t);
        let half = s.fault.length / 2.0;
        assert!(half > 100.0, "{}", s.fault.length);
        // Fayın doğu ucunun 20 km ötesi.
        let east = geo::destination(q.epicenter, 90.0, half + 20.0);
        let r = s.site(east, Site::reference);
        let ts = r.t_s.unwrap();
        let (_, path) = s.fault.distances_and_peak(east);
        let peak = s.physical_peak(&path).unwrap();
        // Kırılma ~half km'yi 2,7 km/s ile alır; ilk S'ten belirgin geç.
        assert!(peak - ts > 10.0, "ts {ts} tepe {peak}");
        assert!(r.rise > RISE_STEP_S);
        assert!((r.t_peak().unwrap() - peak).abs() < 1e-9);
        assert_eq!(r.shindo_at(peak), Some(r.motion.unwrap().shindo()));
        assert!(r.shindo_at(peak - 0.5).unwrap() < r.motion.unwrap().shindo());
        // Odağın yanındaki saha: en kısa adım (fiziksel tepe S ile hemen
        // hemen aynı).
        let near = s.site((29.97, 40.80), Site::reference);
        assert_eq!(near.rise, RISE_STEP_S);
    }

    /// Slab2 ile: bölge (Japonya – Pasifik) ve yay ardı işareti.
    #[test]
    fn slab2_bolge_ve_yay_ardi() {
        let t = tt();
        let slab = crate::slab::test_model();
        let mut q = Quake::new((142.37, 38.30));
        q.setting = Setting::Interface;
        q.fault = FaultType::Reverse;
        q.mw = 8.0;
        q.depth_km = 24.0;
        q.strike = 195.0;
        q.dip = 10.0;
        q.rake = 90.0;
        let s = Scenario::with_slab(q, &t, Some(&slab));
        assert_eq!(s.region, psbah20::Region::JapanPac);
        let sendai = s.site((140.87, 38.27), Site::reference);
        let akita = s.site((140.10, 39.72), Site::reference);
        assert!(!sendai.backarc && akita.backarc);
        // Bölgesel model küreselden farklı sonuç verir.
        let global = Scenario::new(q, &t).site((140.87, 38.27), Site::reference);
        assert!(!global.backarc);
        let (a, b) = (sendai.motion.unwrap().jma, global.motion.unwrap().jma);
        assert!((a - b).abs() > 0.01, "bölgesel {a} küresel {b}");
        // Sığ kabukta Slab2 kullanılmaz.
        let mut crust = q;
        crust.setting = Setting::ShallowCrust;
        crust.mw = 7.0;
        crust.depth_km = 10.0;
        let s = Scenario::with_slab(crust, &t, Some(&slab));
        assert_eq!((s.slab_zone, s.region), (None, psbah20::Region::Global));
        assert!(!s.site((140.10, 39.72), Site::reference).backarc);
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
