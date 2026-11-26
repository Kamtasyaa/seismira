//! Bölge ve şehir için tahmini bina hasarı (CLAUDE.md Bölüm 7.8, Faz 6).
//!
//! Bina stoku ([`crate::exposure`]) bölgenin yerleşim hücrelerine EŞİT
//! dağılmış kabul edilir (ESRM20 stoku idari birim düzeyinde; daha ince
//! dağılım bilinmiyor). Sarsıntı ızgarası adımıyla örneklenen her yerleşim
//! hücresi, bölgenin yerleşim hücrelerinin `adım² / toplam` kadarını temsil
//! eder; hücrede dört kırılganlık şiddet ölçüsü (PGA, SA 0,3 / 0,6 / 1,0 s)
//! depremin modeliyle hesaplanır ve bölgenin bina karışımına uygulanır.
//! Sarsıntı alanının (model mesafe sınırı) dışında kalan hücreler hasarsız.
//!
//! Belirsizlik (Bölüm 7.8): olay içi sapma φ kırılganlık eğrisiyle
//! birleştirilir ([`crate::exposure::Fragility::exceedance`]); "olası
//! aralık" olaylar arası terimin ±1τ kaydırılmasıyla (ln IM ± τ). Bileşen:
//! GMPE'ler RotD50 verir; ESRM20 eğrileri ortalama yatay bileşen için
//! (fark küçük, düzeltme yapılmaz; docs/bilim.md 7.8).

use crate::exposure::{Damage, Exposure, FragImt, N_DS};
use crate::grid::Domain;
use crate::region::{BUILT_BIT, ID_MASK, RegionId};
use crate::shaking::Scenario;
use crate::site::REFERENCE_VS30;
use crate::vulnerability::{VulnFunction, VulnModel};
use rayon::prelude::*;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};

/// Bir bölgenin hasar tahmini (bina sayısı, hasar durumu başına).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RegionDamage {
    pub id: RegionId,
    /// Bölgenin toplam bina sayısı (ESRM20).
    pub buildings: f64,
    /// Medyan şiddetle, −1τ ve +1τ ile.
    pub median: Damage,
    pub low: Damage,
    pub high: Damage,
}

impl RegionDamage {
    /// Ağır hasar + göçme (bina).
    pub fn heavy(d: &Damage) -> f64 {
        d[2] + d[3]
    }
}

/// Sahada kırılganlık şiddet ölçüleri: (ln medyan, φ, τ); `None` model dışı.
pub type SiteIms = [Option<(f64, f64, f64)>; 4];

/// Bir sahanın dört kırılganlık şiddet ölçüsü.
pub fn site_ims(scenario: &Scenario, lonlat: (f64, f64), vs30: f64) -> SiteIms {
    let d = scenario.fault.distances(lonlat);
    if scenario.max_rrup_km().is_none_or(|m| d.rrup > m) {
        return [None; 4];
    }
    let v = scenario.clamp_vs30(vs30);
    FragImt::ALL.map(|imt| {
        let m = scenario.ln_motion(imt.imt(), &d, v)?;
        let phi = (m.sigma * m.sigma - m.tau * m.tau).max(0.0).sqrt();
        Some((m.mean, phi, m.tau))
    })
}

/// (medyan, −1τ, +1τ) hasar dağılımları; `stock` tüm bölgenin stoku.
fn damage_three(
    exposure: &Exposure,
    stock: &crate::exposure::RegionStock,
    ims: &SiteIms,
) -> [Damage; 3] {
    [0.0, -1.0, 1.0].map(|k| {
        exposure.damage(stock, |imt| {
            ims[imt as usize].map(|(mu, phi, tau)| (mu + k * tau, phi))
        })
    })
}

/// Şehir: bulunduğu bölgenin bina karışımıyla, hasar durumlarının ORANI
/// (0–1; medyan, −1τ, +1τ). Bölgede stok yoksa `None`.
pub fn city_fractions(
    scenario: &Scenario,
    exposure: &Exposure,
    region: RegionId,
    lonlat: (f64, f64),
    vs30: f64,
) -> Option<[Damage; 3]> {
    let stock = exposure.region(region)?;
    let n = stock.buildings();
    if n <= 0.0 {
        return None;
    }
    let ims = site_ims(scenario, lonlat, vs30);
    Some(damage_three(exposure, stock, &ims).map(|d| d.map(|x| x / n)))
}

/// Bölge → (temsil edilen yerleşim payı, ağırlıklı toplam).
type Sums<const N: usize> = HashMap<RegionId, (f64, [f64; N])>;

/// Alan içindeki yerleşim hücrelerini adım adım örnekler; her hücrenin
/// bölgesi için `eval(bölge, şiddetler)` değerini, hücrenin bölgenin
/// yerleşim hücreleri içindeki payıyla ağırlıklandırıp toplar. Dönen:
/// bölge → (toplam pay, ağırlıklı toplam). `built(bölge)`: bölgenin
/// yerleşim hücrelerinin toplam ağırlığı (stok yoksa `None`);
/// `cell_weight(k)`: pencerede k. hücrenin ağırlığı (eşit dağılımda 1 ve
/// toplam = yerleşim hücresi sayısı). İptal edilirse `None`.
#[allow(clippy::too_many_arguments)]
fn sample_regions<const N: usize>(
    scenario: &Scenario,
    domain: &Domain,
    step: u32,
    ids: &[RegionId],
    vs30: Option<&[f32]>,
    cancel: &AtomicBool,
    built: impl Fn(RegionId) -> Option<f64> + Sync,
    cell_weight: impl Fn(usize) -> f64 + Sync,
    eval: impl Fn(RegionId, &SiteIms) -> [f64; N] + Sync,
) -> Option<Sums<N>> {
    let spec = domain.spec(step);
    let weight_cells = f64::from(step) * f64::from(step);
    let partial: Vec<Option<Sums<N>>> = (0..spec.height)
        .into_par_iter()
        .map(|row| {
            if cancel.load(Ordering::Relaxed) {
                return None;
            }
            let mut acc: Sums<N> = HashMap::new();
            for col in 0..spec.width {
                let bx = (col * step + step / 2).min(domain.w - 1) as usize;
                let by = (row * step + step / 2).min(domain.h - 1) as usize;
                let k = by * domain.w as usize + bx;
                let raw = ids[k];
                if raw & BUILT_BIT == 0 {
                    continue;
                }
                let id = raw & ID_MASK;
                let Some(cells) = built(id).filter(|&c| c > 0.0) else {
                    continue;
                };
                let cw = cell_weight(k);
                if cw <= 0.0 {
                    continue;
                }
                let v = match vs30 {
                    Some(win) if win[k].is_nan() => continue,
                    Some(win) => f64::from(win[k]),
                    None => REFERENCE_VS30,
                };
                let ims = site_ims(scenario, spec.center(col, row), v);
                let share = weight_cells * cw / cells;
                let e = acc.entry(id).or_insert((0.0, [0.0; N]));
                e.0 += share;
                if ims.iter().all(Option::is_none) {
                    continue;
                }
                for (s, x) in e.1.iter_mut().zip(eval(id, &ims)) {
                    *s += share * x;
                }
            }
            Some(acc)
        })
        .collect();
    let mut total: Sums<N> = HashMap::new();
    for part in partial {
        for (id, (share, d)) in part? {
            let e = total.entry(id).or_insert((0.0, [0.0; N]));
            e.0 += share;
            for (s, x) in e.1.iter_mut().zip(d) {
                *s += x;
            }
        }
    }
    // Örnekleme payların toplamını 1'in biraz üstüne çıkarabilir (adım²
    // yaklaşımı); o durumda 1'e ölçeklenir.
    for (share, d) in total.values_mut() {
        let norm = 1.0 / share.max(1.0);
        d.iter_mut().for_each(|x| *x *= norm);
    }
    Some(total)
}

/// Bölge stokunun yerleşim hücrelerine dağıtım ağırlıkları (deney,
/// docs/dogrulama.md "GHSL ağırlığı"): `cells[k]` penceredeki k. hücrenin
/// ağırlığı, `totals(bölge)` bölgenin tüm yerleşim hücrelerindeki toplamı.
pub struct CellWeights<'a> {
    pub cells: &'a [f32],
    pub totals: &'a (dyn Fn(RegionId) -> Option<f64> + Sync),
}

/// Bölgeler: alan içindeki yerleşim hücrelerinden. `ids` bölge rasterının
/// alan penceresi (ham değer), `vs30` aynı pencerede Vs30 (NaN: deniz;
/// yoksa referans zemin). İptal edilirse `None`.
pub fn regions(
    scenario: &Scenario,
    exposure: &Exposure,
    domain: &Domain,
    step: u32,
    ids: &[RegionId],
    vs30: Option<&[f32]>,
    cancel: &AtomicBool,
) -> Option<Vec<RegionDamage>> {
    regions_weighted(scenario, exposure, domain, step, ids, vs30, None, cancel)
}

/// [`regions`], bölge stoku il içinde `weights` ile dağıtılarak (`None`:
/// yerleşim hücrelerine eşit, uygulamanın kuralı).
#[allow(clippy::too_many_arguments)]
pub fn regions_weighted(
    scenario: &Scenario,
    exposure: &Exposure,
    domain: &Domain,
    step: u32,
    ids: &[RegionId],
    vs30: Option<&[f32]>,
    weights: Option<&CellWeights>,
    cancel: &AtomicBool,
) -> Option<Vec<RegionDamage>> {
    let total = sample_regions::<{ 3 * N_DS }>(
        scenario,
        domain,
        step,
        ids,
        vs30,
        cancel,
        |id| match weights {
            Some(w) => (w.totals)(id),
            None => exposure.region(id).map(|s| f64::from(s.built_cells)),
        },
        |k| weights.map_or(1.0, |w| f64::from(w.cells[k])),
        |id, ims| {
            let mut out = [0.0; 3 * N_DS];
            if let Some(stock) = exposure.region(id) {
                for (k, d) in damage_three(exposure, stock, ims).iter().enumerate() {
                    out[k * N_DS..(k + 1) * N_DS].copy_from_slice(d);
                }
            }
            out
        },
    )?;
    let part = |d: &[f64; 3 * N_DS], k: usize| -> Damage {
        d[k * N_DS..(k + 1) * N_DS].try_into().unwrap_or_default()
    };
    let mut out: Vec<RegionDamage> = total
        .into_iter()
        .map(|(id, (_, d))| RegionDamage {
            id,
            buildings: exposure.region(id).map_or(0.0, |s| s.buildings()),
            median: part(&d, 0),
            low: part(&d, 1),
            high: part(&d, 2),
        })
        .collect();
    out.sort_by_key(|r| r.id);
    Some(out)
}

/// Bir bölgenin ortalama yapısal kayıp oranı (GEM zafiyet modeli, Faz 9).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RegionLoss {
    pub id: RegionId,
    /// Bölgenin toplam yeniden yapım değeri (USD).
    pub cost: f64,
    /// Değer ağırlıklı ortalama kayıp oranı (0–1): medyan, −1τ, +1τ.
    pub ratio: [f64; 3],
}

/// Bölge başına birleşik zafiyet eğrileri ([`VulnModel::combined`]),
/// bölge kimliğine göre.
pub struct VulnCurves {
    curves: HashMap<RegionId, (u32, f64, [Option<VulnFunction>; 4])>,
}

impl VulnCurves {
    pub fn new(model: &VulnModel) -> Self {
        Self {
            curves: model
                .regions
                .iter()
                .map(|r| (r.id, (r.built_cells, r.cost(), model.combined(r))))
                .collect(),
        }
    }

    pub fn has(&self, id: RegionId) -> bool {
        self.curves.contains_key(&id)
    }

    /// Sahada bölge karışımının beklenen kayıp oranı (medyan, −1τ, +1τ).
    fn ratio(&self, id: RegionId, ims: &SiteIms) -> [f64; 3] {
        let Some((_, _, curves)) = self.curves.get(&id) else {
            return [0.0; 3];
        };
        [0.0, -1.0, 1.0].map(|k| {
            curves
                .iter()
                .zip(ims)
                .filter_map(|(c, im)| {
                    let (c, (mu, phi, tau)) = (c.as_ref()?, (*im)?);
                    Some(c.expected(mu + k * tau, phi))
                })
                .sum()
        })
    }
}

/// Bölgeler için ortalama kayıp oranı; [`regions`] ile aynı örnekleme.
/// `skip(bölge)` doğruysa bölge atlanır (ör. ESRM20 stoku olan bölgeler).
#[allow(clippy::too_many_arguments)]
pub fn loss_regions(
    scenario: &Scenario,
    curves: &VulnCurves,
    domain: &Domain,
    step: u32,
    ids: &[RegionId],
    vs30: Option<&[f32]>,
    cancel: &AtomicBool,
    skip: impl Fn(RegionId) -> bool + Sync,
) -> Option<Vec<RegionLoss>> {
    let total = sample_regions::<3>(
        scenario,
        domain,
        step,
        ids,
        vs30,
        cancel,
        |id| {
            if skip(id) {
                return None;
            }
            curves.curves.get(&id).map(|c| f64::from(c.0))
        },
        |_| 1.0,
        |id, ims| curves.ratio(id, ims),
    )?;
    let mut out: Vec<RegionLoss> = total
        .into_iter()
        .map(|(id, (_, ratio))| RegionLoss {
            id,
            cost: curves.curves.get(&id).map_or(0.0, |c| c.1),
            ratio,
        })
        .collect();
    out.sort_by_key(|r| r.id);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exposure::{Fragility, RegionStock};
    use crate::grid::Base;
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

    fn exposure() -> Exposure {
        Exposure {
            fragility: vec![Fragility {
                name: "T".into(),
                imt: FragImt::Sa03,
                median: [0.15, 0.3, 0.6, 1.2],
                beta: 0.6,
                no_damage: 0.05,
            }],
            regions: vec![RegionStock {
                id: 1,
                built_cells: 0, // aşağıda doldurulur
                entries: vec![(0, 10_000.0)],
            }],
        }
    }

    /// Yapay bölge: alanın tamamı tek bölge, hepsi yerleşim. Yakın fayda
    /// hasar büyük; daha küçük depremde daha az; aralık medyanı kuşatır.
    #[test]
    fn bolge_hasari() {
        let t = tt();
        let mut q = Quake::new((30.0, 40.0));
        q.mw = 7.0;
        q.depth_km = 10.0;
        q.fault = FaultType::StrikeSlip;
        let s = Scenario::new(q, &t);
        // Merkez üssü çevresinde 0,5° × 0,5° bir pencere.
        let base = Base::GLOBAL_30S;
        let domain = Domain {
            base,
            col0: ((29.75 - base.west) / base.cell) as i64,
            row0: ((base.north - 40.25) / base.cell) as i64,
            w: 60,
            h: 60,
        };
        let ids = vec![1 | BUILT_BIT; 60 * 60];
        let mut e = exposure();
        e.regions[0].built_cells = 3600;
        let cancel = AtomicBool::new(false);
        let r = regions(&s, &e, &domain, 2, &ids, None, &cancel).unwrap();
        assert_eq!(r.len(), 1);
        let d = &r[0];
        let heavy = RegionDamage::heavy(&d.median);
        assert!(heavy > 1000.0 && heavy < 10_000.0, "{d:?}");
        assert!(RegionDamage::heavy(&d.low) < heavy && heavy < RegionDamage::heavy(&d.high));
        assert!(d.median.iter().sum::<f64>() <= 10_000.0 + 1e-6);
        // Bölgenin yerleşimlerinin yalnızca dörtte biri alanda: hasar ~¼.
        e.regions[0].built_cells = 4 * 3600;
        let quarter = regions(&s, &e, &domain, 2, &ids, None, &cancel).unwrap();
        let ratio = RegionDamage::heavy(&quarter[0].median) / heavy;
        assert!((ratio - 0.25).abs() < 0.01, "{ratio}");
        // Daha küçük deprem: daha az hasar.
        let mut small = q;
        small.mw = 5.5;
        let s2 = Scenario::new(small, &t);
        e.regions[0].built_cells = 3600;
        let r2 = regions(&s2, &e, &domain, 2, &ids, None, &cancel).unwrap();
        let h2 = RegionDamage::heavy(&r2[0].median);
        assert!(h2 > 0.0 && h2 < heavy * 0.7, "Mw 7 {heavy}, Mw 5,5 {h2}");
        // Şehir oranı 0–1 aralığında ve bölgeye yakın.
        let c = city_fractions(&s, &e, 1, (30.0, 40.0), REFERENCE_VS30).unwrap();
        let h = c[0][2] + c[0][3];
        assert!(
            (0.0..=1.0).contains(&h) && h > heavy / 10_000.0 * 0.5,
            "{c:?}"
        );
        // İptal.
        cancel.store(true, Ordering::Relaxed);
        assert!(regions(&s, &e, &domain, 2, &ids, None, &cancel).is_none());
    }

    /// Kayıp oranı: tek sınıflı bölgede, doğrudan sınıf eğrisiyle aynı
    /// mertebede; aralık medyanı kuşatır; atlanan bölge çıktıda yok.
    #[test]
    fn bolge_kayip_orani() {
        let t = tt();
        let mut q = Quake::new((30.0, 40.0));
        q.mw = 7.0;
        q.depth_km = 10.0;
        q.fault = FaultType::StrikeSlip;
        let s = Scenario::new(q, &t);
        let base = Base::GLOBAL_30S;
        let domain = Domain {
            base,
            col0: ((29.75 - base.west) / base.cell) as i64,
            row0: ((base.north - 40.25) / base.cell) as i64,
            w: 60,
            h: 60,
        };
        let ids = vec![1 | BUILT_BIT; 60 * 60];
        let model = VulnModel {
            functions: vec![VulnFunction {
                name: "T".into(),
                imt: FragImt::Sa03,
                iml: vec![0.05, 0.3, 1.0, 3.0],
                mean_lr: vec![0.0, 0.05, 0.4, 0.9],
            }],
            regions: vec![crate::vulnerability::VulnRegion {
                id: 1,
                built_cells: 3600,
                entries: vec![(0, 1.0e9, 1000.0)],
            }],
        };
        let curves = VulnCurves::new(&model);
        let cancel = AtomicBool::new(false);
        let r = loss_regions(&s, &curves, &domain, 2, &ids, None, &cancel, |_| false).unwrap();
        assert_eq!(r.len(), 1);
        let [m, lo, hi] = r[0].ratio;
        assert!(m > 0.01 && m < 0.9, "{m}");
        assert!(lo < m && m < hi, "{:?}", r[0].ratio);
        assert_eq!(r[0].cost, 1.0e9);
        // Merkezde doğrudan sınıf eğrisi: bölge ortalaması bundan küçük
        // (kenarlar daha zayıf sarsılır).
        let ims = site_ims(&s, (30.0, 40.0), REFERENCE_VS30);
        let (mu, phi, _) = ims[FragImt::Sa03 as usize].unwrap();
        let center = model.functions[0].expected(mu, phi);
        assert!(m < center && m > center * 0.2, "{m} / {center}");
        let none = loss_regions(&s, &curves, &domain, 2, &ids, None, &cancel, |_| true).unwrap();
        assert!(none.is_empty());
    }
}
