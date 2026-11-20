//! Sarsıntı ızgarası: model geçerlilik alanındaki (Rrup ≤ sınır) her hücre
//! için son Shindo sınıfı, aletsel şiddet ve S varış zamanı
//! (CLAUDE.md Bölüm 7, 11).
//!
//! - Hücreler Vs30 haritasının ızgarasına hizalıdır (30″); böylece her
//!   hücre haritanın bir hücresinin Vs30'unu doğrudan kullanır (yeniden
//!   örnekleme yok). Alan çok büyükse adım büyütülür ([`MAX_DIM`]).
//! - Kademeli sonuç: önce `COARSE_FACTOR` kat kaba, sonra ince ızgara
//!   (çağıran iki kez [`compute`] çağırır).
//! - Su / buz hücreleri ve model dışı hücreler [`NO_DATA`].
//! - Paralel (rayon), satır başına iptal denetimi.
//!
//! Zamanla değişen görünüm (S ulaştı mı, kademeli yükseliş) bu değerlerin
//! saf fonksiyonudur; çizim tarafında hesaplanır.

use crate::duration;
use crate::gmpe::cy14;
use crate::intensity::Shindo;
use crate::shaking::{Gmm, Scenario};
use crate::site::{REFERENCE_VS30, Site, Vs30Source};
use rayon::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};

/// Bir hücrenin sonucu: (sınıf, aletsel şiddet, P varışı, S varışı, bitiş,
/// yükseliş adımı).
type Cell = (u8, f32, f32, f32, f32, f32);

const EMPTY: Cell = (NO_DATA, f32::NAN, f32::NAN, f32::NAN, f32::NAN, f32::NAN);

/// Veri yok: su, buz, model dışı.
pub const NO_DATA: u8 = u8::MAX;

/// Izgaranın bir kenarındaki en fazla hücre (GPU doku sınırı: wgpu'nun
/// en düşük ortak sınırı 2048, WebGL2 / GL yedeği dahil).
pub const MAX_DIM: u32 = 2048;

/// Kaba geçişin ince geçişe göre adım çarpanı.
pub const COARSE_FACTOR: u32 = 8;

/// Taban ızgara (Vs30 haritasının ızgarası): sol üst kenar ve hücre.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Base {
    pub west: f64,
    pub north: f64,
    pub cell: f64,
}

impl Base {
    /// Harita yokken: 30″, −180° / 90° köşeli küresel ızgara.
    pub const GLOBAL_30S: Base = Base {
        west: -180.0,
        north: 90.0,
        cell: 1.0 / 120.0,
    };
}

/// Hesap alanı: taban ızgarada sol üst hücre ve boyut.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Domain {
    pub base: Base,
    pub col0: i64,
    pub row0: i64,
    pub w: u32,
    pub h: u32,
}

/// Bir ızgaranın geometrisi (hücre kenarları; `west`/`north` sol üst köşe).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridSpec {
    pub west: f64,
    pub north: f64,
    pub cell: f64,
    pub width: u32,
    pub height: u32,
}

impl GridSpec {
    pub fn center(&self, col: u32, row: u32) -> (f64, f64) {
        (
            self.west + (f64::from(col) + 0.5) * self.cell,
            self.north - (f64::from(row) + 0.5) * self.cell,
        )
    }
}

impl Domain {
    /// Fayın Rrup ≤ model sınırı eğrisini kapsayan alan (kabukta 300, dalma-
    /// batmada 1000 km). Boylamlar merkez üssüne göre açılır (180°
    /// meridyenini geçen alanlar sürekli). Modeli olmayan depremde
    /// çağrılmaz; çağrılırsa kabuk sınırı kullanılır.
    pub fn for_scenario(scenario: &Scenario, base: Base) -> Domain {
        let max = scenario.max_rrup_km().unwrap_or(cy14::MAX_RRUP_KM);
        let contour = scenario.fault.rrup_contour(max, 180);
        let lon0 = scenario.quake.epicenter.0;
        let unwrap = |lon: f64| lon0 + (lon - lon0 + 180.0).rem_euclid(360.0) - 180.0;
        let (mut w, mut e, mut s, mut n) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
        for &(lon, lat) in &contour {
            let lon = unwrap(lon);
            w = w.min(lon);
            e = e.max(lon);
            s = s.min(lat);
            n = n.max(lat);
        }
        let (s, n) = (s.max(-85.0), n.min(85.0));
        let col0 = ((w - base.west) / base.cell).floor() as i64;
        let col1 = ((e - base.west) / base.cell).ceil() as i64;
        let row0 = ((base.north - n) / base.cell).floor() as i64;
        let row1 = ((base.north - s) / base.cell).ceil() as i64;
        Domain {
            base,
            col0,
            row0,
            w: (col1 - col0).max(1) as u32,
            h: (row1 - row0).max(1) as u32,
        }
    }

    /// İnce geçişin adımı (taban hücre sayısı): kenar ≤ [`MAX_DIM`].
    pub fn fine_step(&self) -> u32 {
        self.w.max(self.h).div_ceil(MAX_DIM).max(1)
    }

    /// `step` taban hücrelik adımla ızgara geometrisi.
    pub fn spec(&self, step: u32) -> GridSpec {
        GridSpec {
            west: self.base.west + self.col0 as f64 * self.base.cell,
            north: self.base.north - self.row0 as f64 * self.base.cell,
            cell: self.base.cell * f64::from(step),
            width: self.w.div_ceil(step),
            height: self.h.div_ceil(step),
        }
    }
}

/// Hesaplanmış ızgara.
#[derive(Clone, Debug)]
pub struct ShakeGrid {
    pub spec: GridSpec,
    /// Son Shindo sınıfı (`Shindo as u8`, 0–9) ya da [`NO_DATA`].
    pub class: Vec<u8>,
    /// Aletsel şiddet (medyan); veri yoksa NaN.
    pub jma: Vec<f32>,
    /// P varışı (s); veri yoksa NaN.
    pub t_p: Vec<f32>,
    /// S varışı (s); veri yoksa NaN.
    pub t_s: Vec<f32>,
    /// Sarsıntının bittiği an (s): tepe anı + D5-95 (Afshari & Stewart 2016;
    /// Bölüm 7.3 "t_bitiş"); veri yoksa NaN.
    pub t_end: Vec<f32>,
    /// Gösterilen Shindo'nun bir sınıf yükselme süresi (s;
    /// [`crate::shaking::SiteResult::rise`]); veri yoksa NaN.
    pub rise: Vec<f32>,
}

impl ShakeGrid {
    /// Hücredeki en yüksek son Shindo (veri olan hücreler).
    pub fn max_class(&self) -> Option<Shindo> {
        self.class
            .iter()
            .filter(|&&c| c != NO_DATA)
            .max()
            .map(|&c| Shindo::ALL[c as usize])
    }
}

/// Izgarayı hesaplar. `vs30`: alanın taban çözünürlükte penceresi
/// (`domain.w × domain.h`, satır satır; su / buz NaN) ya da harita yoksa
/// `None` (her hücrede referans zemin). `step` taban hücrelik adım; her
/// ızgara hücresi, kapsadığı taban hücrelerin ortadakinin Vs30'unu alır.
/// İptal edilirse `None`.
pub fn compute(
    scenario: &Scenario,
    domain: &Domain,
    step: u32,
    vs30: Option<&[f32]>,
    cancel: &AtomicBool,
) -> Option<ShakeGrid> {
    let spec = domain.spec(step);
    let (w, h) = (spec.width as usize, spec.height as usize);
    let rows: Vec<Option<Vec<Cell>>> = (0..spec.height)
        .into_par_iter()
        .map(|row| {
            if cancel.load(Ordering::Relaxed) {
                return None;
            }
            let out = (0..spec.width)
                .map(|col| {
                    let site = match vs30 {
                        Some(win) => {
                            let bx = (col * step + step / 2).min(domain.w - 1) as usize;
                            let by = (row * step + step / 2).min(domain.h - 1) as usize;
                            let v = win[by * domain.w as usize + bx];
                            if v.is_nan() {
                                return EMPTY;
                            }
                            Site {
                                vs30: f64::from(v),
                                vs30_ln_sd: None,
                                source: Vs30Source::Map,
                            }
                        }
                        None => Site {
                            vs30: REFERENCE_VS30,
                            vs30_ln_sd: None,
                            source: Vs30Source::Reference,
                        },
                    };
                    let r = scenario.site(spec.center(col, row), || site);
                    match (r.motion, r.t_p, r.t_s, r.t_peak()) {
                        (Some(m), Some(tp), Some(ts), Some(peak)) => {
                            let q = &scenario.quake;
                            let (d, _) = match scenario.gmm {
                                Some(Gmm::Psbah20(event)) => duration::bahrampouri2021::d595(
                                    q.mw,
                                    r.dist.rrup,
                                    r.vs30_used,
                                    event,
                                    q.fault,
                                ),
                                _ => duration::d595(q.mw, r.dist.rrup, r.vs30_used, q.fault),
                            };
                            let c = m.shindo() as u8;
                            let rise = r.rise as f32;
                            (
                                c,
                                m.jma as f32,
                                tp as f32,
                                ts as f32,
                                (peak + d) as f32,
                                rise,
                            )
                        }
                        _ => EMPTY,
                    }
                })
                .collect();
            Some(out)
        })
        .collect();
    let mut grid = ShakeGrid {
        spec,
        class: Vec::with_capacity(w * h),
        jma: Vec::with_capacity(w * h),
        t_p: Vec::with_capacity(w * h),
        t_s: Vec::with_capacity(w * h),
        t_end: Vec::with_capacity(w * h),
        rise: Vec::with_capacity(w * h),
    };
    for row in rows {
        for (c, j, tp, ts, e, r) in row? {
            grid.class.push(c);
            grid.jma.push(j);
            grid.t_p.push(tp);
            grid.t_s.push(ts);
            grid.t_end.push(e);
            grid.rise.push(r);
        }
    }
    Some(grid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quake::{FaultType, Quake};
    use crate::traveltime::TravelTimes;
    use std::path::Path;

    fn tt() -> TravelTimes {
        let p = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/traveltime/ak135.ttt"
        );
        TravelTimes::load(Path::new(p)).unwrap()
    }

    fn quake(lon: f64) -> Quake {
        let mut q = Quake::new((lon, 40.76));
        q.mw = 7.0;
        q.depth_km = 10.0;
        q.fault = FaultType::StrikeSlip;
        q.strike = 90.0;
        q
    }

    #[test]
    fn alan_model_sinirini_kapsar() {
        let t = tt();
        let s = Scenario::new(quake(29.97), &t);
        let d = Domain::for_scenario(&s, Base::GLOBAL_30S);
        let spec = d.spec(1);
        for (lon, lat) in s.fault.rrup_contour(cy14::MAX_RRUP_KM, 36) {
            assert!(lon >= spec.west && lon <= spec.west + spec.cell * f64::from(spec.width));
            assert!(lat <= spec.north && lat >= spec.north - spec.cell * f64::from(spec.height));
        }
        // 300 km yarıçap + fay: ~7° × 6°, 30″ → binlerce hücre, sınır altında.
        assert!(d.w > 500 && d.w <= 4000, "{d:?}");
        assert_eq!(d.fine_step(), d.w.max(d.h).div_ceil(MAX_DIM).max(1));
    }

    #[test]
    fn hucreler_sehir_hesabiyla_ayni() {
        let t = tt();
        let s = Scenario::new(quake(29.97), &t);
        let d = Domain::for_scenario(&s, Base::GLOBAL_30S);
        let step = 40; // test hızlı olsun
        let g = compute(&s, &d, step, None, &AtomicBool::new(false)).unwrap();
        assert_eq!(g.class.len(), (g.spec.width * g.spec.height) as usize);
        // Rastgele bir hücre: şehir hesabıyla aynı sonuç.
        let (col, row) = (g.spec.width / 2 + 3, g.spec.height / 2 - 2);
        let i = (row * g.spec.width + col) as usize;
        let r = s.site(g.spec.center(col, row), Site::reference);
        let m = r.motion.unwrap();
        assert_eq!(g.class[i], m.shindo() as u8);
        assert!((f64::from(g.jma[i]) - m.jma).abs() < 1e-4);
        assert!((f64::from(g.t_s[i]) - r.t_s.unwrap()).abs() < 1e-3);
        // Sarsıntı tepe anından sonra biter (D5-95 > 0).
        assert!(g.t_end[i] > g.t_s[i]);
        assert!(g.t_p[i] < g.t_s[i]);
        // Köşeler model dışı (Rrup > 300 km).
        assert_eq!(g.class[0], NO_DATA);
        // 40 hücrelik (~37 km) adımda faya en yakın hücre bile birkaç km uzakta.
        assert!(g.max_class().unwrap() >= Shindo::S5Lower);
    }

    #[test]
    fn su_hucresi_ve_iptal() {
        let t = tt();
        let s = Scenario::new(quake(29.97), &t);
        let d = Domain::for_scenario(&s, Base::GLOBAL_30S);
        let win = vec![f32::NAN; (d.w * d.h) as usize];
        let g = compute(&s, &d, 50, Some(&win), &AtomicBool::new(false)).unwrap();
        assert!(g.class.iter().all(|&c| c == NO_DATA));
        assert!(compute(&s, &d, 50, None, &AtomicBool::new(true)).is_none());
    }

    #[test]
    fn meridyen_180_gecen_alan_surekli() {
        let t = tt();
        let s = Scenario::new(quake(179.5), &t);
        let d = Domain::for_scenario(&s, Base::GLOBAL_30S);
        let spec = d.spec(1);
        // Alan 180°'yi geçer ama tek parça (genişlik ~7°, 360° değil).
        assert!(spec.west < 180.0 && spec.west + spec.cell * f64::from(spec.width) > 180.0);
        assert!(f64::from(spec.width) * spec.cell < 20.0);
    }
}
