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

/// Bölge başına birikim: (temsil edilen yerleşim payı, [medyan, −1τ, +1τ]).
type Acc = HashMap<RegionId, (f64, [Damage; 3])>;

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
    let spec = domain.spec(step);
    let weight_cells = f64::from(step) * f64::from(step);
    let partial: Vec<Option<Acc>> = (0..spec.height)
        .into_par_iter()
        .map(|row| {
            if cancel.load(Ordering::Relaxed) {
                return None;
            }
            let mut acc: Acc = HashMap::new();
            for col in 0..spec.width {
                let bx = (col * step + step / 2).min(domain.w - 1) as usize;
                let by = (row * step + step / 2).min(domain.h - 1) as usize;
                let k = by * domain.w as usize + bx;
                let raw = ids[k];
                if raw & BUILT_BIT == 0 {
                    continue;
                }
                let id = raw & ID_MASK;
                let Some(stock) = exposure.region(id) else {
                    continue;
                };
                if stock.built_cells == 0 {
                    continue;
                }
                let v = match vs30 {
                    Some(win) if win[k].is_nan() => continue,
                    Some(win) => f64::from(win[k]),
                    None => REFERENCE_VS30,
                };
                let lonlat = spec.center(col, row);
                let ims = site_ims(scenario, lonlat, v);
                let share = weight_cells / f64::from(stock.built_cells);
                let e = acc.entry(id).or_insert((0.0, [[0.0; N_DS]; 3]));
                e.0 += share;
                if ims.iter().all(Option::is_none) {
                    continue;
                }
                for (sum, d) in e.1.iter_mut().zip(damage_three(exposure, stock, &ims)) {
                    for (s, x) in sum.iter_mut().zip(d) {
                        *s += share * x;
                    }
                }
            }
            Some(acc)
        })
        .collect();
    let mut total: Acc = HashMap::new();
    for part in partial {
        for (id, (share, d)) in part? {
            let e = total.entry(id).or_insert((0.0, [[0.0; N_DS]; 3]));
            e.0 += share;
            for (sum, x) in e.1.iter_mut().zip(d) {
                for (s, v) in sum.iter_mut().zip(x) {
                    *s += v;
                }
            }
        }
    }
    let mut out: Vec<RegionDamage> = total
        .into_iter()
        .map(|(id, (share, [median, low, high]))| {
            // Örnekleme payların toplamını 1'in biraz üstüne çıkarabilir
            // (adım² yaklaşımı); o durumda 1'e ölçeklenir.
            let norm = 1.0 / share.max(1.0);
            let buildings = exposure.region(id).map_or(0.0, |s| s.buildings());
            RegionDamage {
                id,
                buildings,
                median: median.map(|x| x * norm),
                low: low.map(|x| x * norm),
                high: high.map(|x| x * norm),
            }
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
}
