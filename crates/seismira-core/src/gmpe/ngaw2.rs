//! Sığ kabuk için model ortalaması (mantık ağacı): varsayılan dört
//! NGA-West2 modelinin eşit ağırlıklı ortalaması, ASK14, BSSA14, CB14, CY14
//! (CLAUDE.md Bölüm 7.4 ve 15 "model ortalaması"; kullanıcı kararı
//! 2026-11-24, gerekçe docs/dogrulama.md "dört NGA-West2 modeli").
//! Kullanıcı ağırlıkları değiştirebilir ve Akkar, Sandıkkaya & Bommer
//! (2014) Avrupa–Orta Doğu modelini ([`asb14`]) ekleyebilir ([`Weights`];
//! varsayılan ağırlığı 0, yani varsayılan sonuçlar değişmez).
//!
//! Modeller ln uzayında ağırlıklarla ortalanır (varsayılan eşit ağırlıklar
//! SEÇİM; veriye göre ayarlanmadı). Ağırlıklar, o sahada geçerli olan
//! modeller arasında yeniden normalleştirilir. Bölgesel düzeltmeler her
//! modelin kendi tanımladığı bölgelerle ([`CrustRegion`]; ASB14'ün bölge
//! terimi yok).
//!
//! Standart sapma: karışım dağılımının varyansı,
//! σ² = Σ wᵢ (σᵢ² + (μᵢ − μ̄)²); aynı biçimde τ² = Σ wᵢ (τᵢ² + (μᵢ − μ̄)²).
//! Modeller arası fark (epistemik) tüm sahalarda ortak olduğu için olaylar
//! arası (τ) kısma da eklenir. SEÇİM (kaynakta reçete yok); belirsizlik
//! aralığını biraz genişletir.
//!
//! Aralık: her model kendi yayımlanmış aralığıyla çağrılır; Vs30 her modelin
//! aralığına çekilir (ASK14 1000 m/s üstünde 1000 ile). Aralığı dışında
//! kalan model (ör. BSSA14 normal fay Mw > 7, CB14 odak > 20 km ya da eğim
//! < 15°, ASB14 Rjb > 200 km ya da Mw > 8) ortalamaya katılmaz; hiçbiri
//! geçerli değilse `None`. Bir modelin düşmesi, düştüğü sınırda ortalamada
//! küçük bir sıçrama yaratır (docs/bilim.md 7.4).

use super::{CrustRegion, Imt, LnMotion, asb14, ask14, bssa14, cb14, cy14};
use crate::quake::FaultType;

/// Bir saha için girdi (dört modelin ortak kümesi).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Input {
    pub mw: f64,
    pub rrup: f64,
    pub rjb: f64,
    pub rx: f64,
    pub ztor: f64,
    /// Odak derinliği (km), CB14 için.
    pub zhyp: f64,
    pub dip: f64,
    /// Kırılma genişliği (km), ASK14 ve CB14 tavan bloğu terimi için.
    pub width: f64,
    pub fault: FaultType,
    pub vs30: f64,
    pub region: CrustRegion,
    /// Merkezlenmiş DPP (yönelim; yalnızca CY14 kullanır, diğer üç modelde
    /// yönelim terimi yok). 0: ortalama yönelim.
    pub dpp: f64,
    /// Model ağırlıkları (mantık ağacı).
    pub weights: Weights,
}

/// Mesafe sınırı (km, Rrup): dört modelin ortak sınırı (BSSA14 Rjb ≤ 400).
pub const MAX_RRUP_KM: f64 = 300.0;
/// Vs30 aralığı (m/s): harita değeri buna çekilir (CY14 ve ASK14'ün alt
/// sınırı 180; üst sınır CY14 / BSSA14 / CB14'ün 1500'ü).
pub const VS30_MIN: f64 = 180.0;
pub const VS30_MAX: f64 = 1500.0;

/// Ortalamadaki modeller.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Model {
    Ask14,
    Bssa14,
    Cb14,
    Cy14,
    Asb14,
}

pub const MODELS: [Model; 5] = [
    Model::Ask14,
    Model::Bssa14,
    Model::Cb14,
    Model::Cy14,
    Model::Asb14,
];

impl Model {
    /// Kısa ad (kod ve dışa aktarma; arayüz metni değil).
    pub fn code(self) -> &'static str {
        match self {
            Model::Ask14 => "ASK14",
            Model::Bssa14 => "BSSA14",
            Model::Cb14 => "CB14",
            Model::Cy14 => "CY14",
            Model::Asb14 => "ASB14",
        }
    }

    fn index(self) -> usize {
        MODELS.iter().position(|&m| m == self).expect("model")
    }
}

/// Mantık ağacı ağırlıkları, [`MODELS`] sırasıyla. Negatif olamaz; toplam
/// bire normalleştirilmek zorunda değil (her sahada geçerli modeller
/// arasında normalleştirilir). Varsayılan: dört NGA-West2 eşit, ASB14 0.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Weights(pub [f64; 5]);

impl Default for Weights {
    fn default() -> Self {
        Weights([1.0, 1.0, 1.0, 1.0, 0.0])
    }
}

impl Weights {
    pub fn get(&self, m: Model) -> f64 {
        self.0[m.index()].max(0.0)
    }

    pub fn set(&mut self, m: Model, w: f64) {
        self.0[m.index()] = w.max(0.0);
    }

    /// En az bir model sıfırdan büyük ağırlıkta mı?
    pub fn any(&self) -> bool {
        MODELS.iter().any(|&m| self.get(m) > 0.0)
    }

    /// Toplamı bire normalleştirilmiş ağırlık (gösterim için); hepsi sıfırsa 0.
    pub fn share(&self, m: Model) -> f64 {
        let sum: f64 = MODELS.iter().map(|&k| self.get(k)).sum();
        if sum > 0.0 { self.get(m) / sum } else { 0.0 }
    }
}

/// Tek bir modelin tahmini (Vs30 modelin aralığına çekilir).
pub fn predict_one(model: Model, imt: Imt, i: &Input) -> Option<LnMotion> {
    let vs = |lo: f64, hi: f64| i.vs30.clamp(lo, hi);
    match model {
        Model::Ask14 => ask14::predict(
            imt,
            &ask14::Input {
                mw: i.mw,
                rrup: i.rrup,
                rjb: i.rjb,
                rx: i.rx,
                ztor: i.ztor,
                dip: i.dip,
                width: i.width,
                fault: i.fault,
                vs30: vs(ask14::VS30_MIN, ask14::VS30_MAX),
                region: i.region,
            },
        ),
        Model::Bssa14 => bssa14::predict(
            imt,
            &bssa14::Input {
                mw: i.mw,
                rjb: i.rjb,
                fault: i.fault,
                vs30: vs(bssa14::VS30_MIN, bssa14::VS30_MAX),
                region: i.region,
            },
        ),
        Model::Cb14 => cb14::predict(
            imt,
            &cb14::Input {
                mw: i.mw,
                rrup: i.rrup,
                rjb: i.rjb,
                rx: i.rx,
                ztor: i.ztor,
                zhyp: i.zhyp,
                dip: i.dip,
                width: i.width,
                fault: i.fault,
                vs30: vs(cb14::VS30_MIN, cb14::VS30_MAX),
                region: i.region,
            },
        ),
        // CY14 bölgesel kolu: Japonya / İtalya / Çin (cy14.rs modül notu).
        Model::Cy14 => cy14::predict(
            imt,
            &cy14::Input {
                mw: i.mw,
                rrup: i.rrup,
                rjb: i.rjb,
                rx: i.rx,
                ztor: i.ztor,
                dip: i.dip,
                fault: i.fault,
                vs30: vs(cy14::VS30_MIN, cy14::VS30_MAX),
                dpp: i.dpp,
                region: i.region,
            },
        ),
        Model::Asb14 => asb14::predict(
            imt,
            &asb14::Input {
                mw: i.mw,
                rjb: i.rjb,
                rrup: i.rrup,
                zhyp: i.zhyp,
                fault: i.fault,
                vs30: vs(asb14::VS30_MIN, asb14::VS30_MAX),
            },
        ),
    }
}

/// Ağırlıklı ortalama (modül notu). Hiçbir model geçerli değilse `None`.
pub fn predict(imt: Imt, i: &Input) -> Option<LnMotion> {
    if i.rrup > MAX_RRUP_KM {
        return None;
    }
    let mut ms = [(
        0.0,
        LnMotion {
            mean: 0.0,
            sigma: 0.0,
            tau: 0.0,
        },
    ); MODELS.len()];
    let mut n = 0;
    for m in MODELS {
        let w = i.weights.get(m);
        if w <= 0.0 {
            continue;
        }
        if let Some(p) = predict_one(m, imt, i) {
            ms[n] = (w, p);
            n += 1;
        }
    }
    combine(&ms[..n])
}

/// Karışım: ln ortalamaların ağırlıklı ortalaması ve karışım varyansı.
fn combine(ms: &[(f64, LnMotion)]) -> Option<LnMotion> {
    let total: f64 = ms.iter().map(|(w, _)| w).sum();
    if ms.is_empty() || total <= 0.0 {
        return None;
    }
    let avg = |f: &dyn Fn(&LnMotion) -> f64| ms.iter().map(|(w, m)| w * f(m)).sum::<f64>() / total;
    let mean = avg(&|m| m.mean);
    let spread = avg(&|m| (m.mean - mean).powi(2));
    let var = avg(&|m| m.sigma * m.sigma) + spread;
    let tau2 = avg(&|m| m.tau * m.tau) + spread;
    Some(LnMotion {
        mean,
        sigma: var.sqrt(),
        tau: tau2.sqrt(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> Input {
        Input {
            mw: 7.0,
            rrup: 20.0,
            rjb: 20.0,
            rx: -20.0,
            ztor: 0.0,
            zhyp: 10.0,
            dip: 90.0,
            width: 15.0,
            fault: FaultType::StrikeSlip,
            vs30: 400.0,
            region: CrustRegion::Global,
            dpp: 0.0,
            weights: Weights::default(),
        }
    }

    /// Ortalama, tek tek model tahminlerinin ln ortalamasıdır; σ en küçük
    /// model σ'sından küçük olamaz.
    #[test]
    fn esit_agirlikli_ortalama() {
        let i = input();
        for imt in [Imt::Pga, Imt::Pgv, Imt::Sa(0.3), Imt::Sa(1.0)] {
            let each: Vec<LnMotion> = MODELS[..4]
                .iter()
                .map(|&m| predict_one(m, imt, &i).unwrap())
                .collect();
            let p = predict(imt, &i).unwrap();
            let mean = each.iter().map(|m| m.mean).sum::<f64>() / 4.0;
            assert!((p.mean - mean).abs() < 1e-12);
            let min_sigma = each.iter().map(|m| m.sigma).fold(f64::MAX, f64::min);
            assert!(p.sigma >= min_sigma, "{p:?} {each:?}");
            assert!(p.tau <= p.sigma);
        }
    }

    /// Aralığı dışındaki model düşer: normal fay Mw 7.5'te BSSA14 (≤ 7) yok,
    /// CB14 (≤ 7.5) var; 25 km odakta CB14 yok.
    #[test]
    fn aralik_disi_model_duser() {
        let i = Input {
            mw: 7.5,
            fault: FaultType::Normal,
            dip: 55.0,
            ..input()
        };
        let ok = |m, i: &Input| predict_one(m, Imt::Pgv, i).is_some();
        assert!(!ok(Model::Bssa14, &i));
        assert!(ok(Model::Cb14, &i));
        assert!(predict(Imt::Pgv, &i).is_some());
        let deep = Input {
            zhyp: 25.0,
            ..input()
        };
        assert!(!ok(Model::Cb14, &deep));
        assert!(predict(Imt::Pgv, &deep).is_some());
        assert!(
            predict(
                Imt::Pgv,
                &Input {
                    rrup: 301.0,
                    rjb: 301.0,
                    ..input()
                }
            )
            .is_none()
        );
    }

    /// Vs30 1500'de ASK14 1000'e çekilerek katılır.
    #[test]
    fn ask14_vs30_cekilir() {
        let i = Input {
            vs30: 1400.0,
            ..input()
        };
        assert!(predict_one(Model::Ask14, Imt::Pga, &i).is_some());
    }

    /// Ağırlıklar: tek modele tam ağırlık o modeli verir; ASB14 eklenince
    /// ortalama ağırlığıyla kayar; 200 km ötesinde ASB14 düşer.
    #[test]
    fn agirliklar() {
        let base = input();
        let only = |m: Model| {
            let mut w = Weights([0.0; 5]);
            w.set(m, 1.0);
            Input { weights: w, ..base }
        };
        for m in MODELS {
            let p = predict(Imt::Pga, &only(m)).unwrap();
            let one = predict_one(m, Imt::Pga, &base).unwrap();
            assert!((p.mean - one.mean).abs() < 1e-12, "{m:?}");
            assert!((p.sigma - one.sigma).abs() < 1e-12, "{m:?}");
        }
        let nga = predict(Imt::Pga, &base).unwrap();
        let asb = predict_one(Model::Asb14, Imt::Pga, &base).unwrap();
        let mut w = Weights::default();
        w.set(Model::Asb14, 4.0); // NGA toplamı 4: yarı yarıya
        let mixed = predict(Imt::Pga, &Input { weights: w, ..base }).unwrap();
        assert!((mixed.mean - 0.5 * (nga.mean + asb.mean)).abs() < 1e-9);
        assert!((w.share(Model::Asb14) - 0.5).abs() < 1e-12);
        // 250 km'de ASB14 yok: yalnızca dört NGA-West2.
        let far = Input {
            rrup: 250.0,
            rjb: 250.0,
            rx: -250.0,
            weights: w,
            ..base
        };
        let far_nga = Input {
            weights: Weights::default(),
            ..far
        };
        assert_eq!(predict(Imt::Pga, &far), predict(Imt::Pga, &far_nga));
        // Yalnızca ASB14 seçiliyse 250 km model dışı.
        assert!(
            predict(
                Imt::Pga,
                &Input {
                    rrup: 250.0,
                    rjb: 250.0,
                    ..only(Model::Asb14)
                }
            )
            .is_none()
        );
        // Hepsi sıfır: tahmin yok.
        assert!(
            predict(
                Imt::Pga,
                &Input {
                    weights: Weights([0.0; 5]),
                    ..base
                }
            )
            .is_none()
        );
        assert!(!Weights([0.0; 5]).any() && Weights::default().any());
    }
}
