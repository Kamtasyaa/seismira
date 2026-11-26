//! Chiou & Youngs (2014) NGA-West2 modeli, sığ kabuk depremleri.
//!
//! Künye: Chiou, B.S.-J. & Youngs, R.R. (2014). Update of the Chiou and
//! Youngs NGA model for the average horizontal component of peak ground
//! motion and response spectra. Earthquake Spectra 30(3), 1117-1153.
//! doi:10.1193/072813EQS219M
//!
//! Katsayılar: USGS nshmp-lib `CY14.csv` (CC0), bkz. `cy14.csv` başlığı.
//! Denklemler makaledeki numaralarla; aktarım nshmp-lib
//! `ChiouYoungs_2014.java` (kamu malı) temel modeline göre yapıldı ve
//! pygmm 0.9.1 ile karşılaştırılarak test edilir (`data/test/cy14_ref.csv`).
//!
//! Kapsam ve seçimler:
//! - Bileşen: RotD50.
//! - Bölge: Kaliforniya/küresel temel model (bölgesel düzeltmeler yok).
//! - Yönelim (directivity): Eş. 3.6–3.8 (PEER 2013/07 s. 24, birincil
//!   kaynaktan 2026-11-26): f_D = c8 · f_R · f_M · exp(−c8a (M − c8b)²) ·
//!   ΔDPP, f_R = max(0, 1 − max(Rrup − 40, 0)/30), f_M = min(1,
//!   max(M − 5.5, 0)/0.8); ln y_ref'e eklenir (Eş. 11 içinde, zemin
//!   teriminden önce; pygmm 0.9.1 ile aynı yer). ΔDPP (merkezlenmiş DPP,
//!   Chiou & Spudich 2013) girdi olarak gelir ([`crate::rupture`]); 0 =
//!   ortalama yönelim (önceki davranış).
//! - Havza terimi yok (ΔZ1 = 0: Z1, Vs30'dan modelin kendi ilişkisiyle).
//! - Vs30 "tahmini" kabul edilir (haritadan; sapma terimi σ3).
//! - Geçerlilik (makale; pygmm uyarılarıyla teyit): Mw 3.5–8.5 doğrultu
//!   atımlı, 3.5–8.0 ters/normal; Rrup ≤ 300 km; Ztor 0–20 km;
//!   Vs30 180–1500 m/s. Sınır dışı çağrılar `None` döner.

use super::{Imt, LnMotion};
use crate::quake::FaultType;
use std::sync::LazyLock;

/// Bir saha için girdi.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Input {
    pub mw: f64,
    /// Fay düzlemine en kısa mesafe (km).
    pub rrup: f64,
    /// Fayın yüzey izdüşümüne en kısa yatay mesafe (km).
    pub rjb: f64,
    /// Üst kenar izine dik yatay mesafe (km); tavan bloğu tarafı pozitif.
    pub rx: f64,
    /// Fayın üst kenar derinliği (km).
    pub ztor: f64,
    /// Eğim (derece).
    pub dip: f64,
    pub fault: FaultType,
    /// Üst 30 m ortalama kayma dalgası hızı (m/s).
    pub vs30: f64,
    /// Merkezlenmiş DPP (ΔDPP); 0: ortalama yönelim.
    pub dpp: f64,
}

/// Modelin mesafe sınırı (km, Rrup).
pub const MAX_RRUP_KM: f64 = 300.0;

/// Modelin Vs30 geçerlilik aralığı (m/s), uçlar dahil (CY14; pygmm de
/// aynı sınırlarla uyarıyor). Harita değeri bu aralık dışındaysa
/// [`crate::shaking`] değeri uca çeker ve bunu sonuçta işaretler.
pub const VS30_MIN: f64 = 180.0;
pub const VS30_MAX: f64 = 1500.0;

/// Modelin üst kenar derinliği (Ztor) sınırı (km). Girdi denetimi
/// ([`crate::quake::validate`]) bunu aşan kaynağı engeller; aşılırsa her
/// sahada `None` döner.
pub const ZTOR_MAX_KM: f64 = 20.0;

/// Modelin yayımlanmış geçerlilik aralığı içinde mi?
pub fn in_range(i: &Input) -> bool {
    let mw_max = if i.fault == FaultType::StrikeSlip {
        8.5
    } else {
        8.0
    };
    (3.5..=mw_max).contains(&i.mw)
        && (0.0..=MAX_RRUP_KM).contains(&i.rrup)
        && (0.0..=ZTOR_MAX_KM).contains(&i.ztor)
        && (VS30_MIN..=VS30_MAX).contains(&i.vs30)
}

#[derive(Clone, Debug)]
struct Coeffs {
    period: Option<f64>, // None: PGA veya PGV satırı
    c1: f64,
    c1a: f64,
    c1b: f64,
    c1c: f64,
    c1d: f64,
    cn: f64,
    cm: f64,
    c3: f64,
    c5: f64,
    chm: f64,
    c6: f64,
    c7: f64,
    c7b: f64,
    c9: f64,
    c9a: f64,
    c9b: f64,
    c11b: f64,
    c8: f64,
    c8a: f64,
    c8b: f64,
    g1: f64,
    g2: f64,
    g3: f64,
    phi1: f64,
    phi2: f64,
    phi3: f64,
    phi4: f64,
    phi5: f64,
    tau1: f64,
    tau2: f64,
    sigma1: f64,
    sigma2: f64,
    sigma3: f64,
}

// Tüm periyotlar için sabit katsayılar (makale Tablo 1).
const C2: f64 = 1.06;
const C4: f64 = -2.1;
const C4A: f64 = -0.5;
const CRB: f64 = 50.0;
const C11: f64 = 0.0;
const PHI6: f64 = 300.0;

struct Table {
    pga: Coeffs,
    pgv: Coeffs,
    /// Periyoda göre artan sırada.
    sa: Vec<Coeffs>,
}

static TABLE: LazyLock<Table> = LazyLock::new(|| parse(include_str!("cy14.csv")));

fn parse(csv: &str) -> Table {
    let mut lines = csv.lines().filter(|l| !l.trim_start().starts_with('#'));
    let header: Vec<&str> = lines
        .next()
        .expect("başlık")
        .split(',')
        .map(str::trim)
        .collect();
    let col = |name: &str| {
        header
            .iter()
            .position(|h| *h == name)
            .unwrap_or_else(|| panic!("{name} sütunu yok"))
    };
    let idx: Vec<usize> = [
        "c1", "c1a", "c1b", "c1c", "c1d", "cn", "cM", "c3", "c5", "cHM", "c6", "c7", "c7b", "c9",
        "c9a", "c9b", "c11b", "cgamma1", "cgamma2", "cgamma3", "phi1", "phi2", "phi3", "phi4",
        "phi5", "tau1", "tau2", "sigma1", "sigma2", "sigma3", "c8", "c8a", "c8b",
    ]
    .iter()
    .map(|n| col(n))
    .collect();
    let (mut pga, mut pgv, mut sa) = (None, None, Vec::new());
    for line in lines.filter(|l| !l.trim().is_empty()) {
        let f: Vec<&str> = line.split(',').map(str::trim).collect();
        let v = |k: usize| -> f64 { f[idx[k]].parse().expect("sayı") };
        let mut c = Coeffs {
            period: None,
            c1: v(0),
            c1a: v(1),
            c1b: v(2),
            c1c: v(3),
            c1d: v(4),
            cn: v(5),
            cm: v(6),
            c3: v(7),
            c5: v(8),
            chm: v(9),
            c6: v(10),
            c7: v(11),
            c7b: v(12),
            c9: v(13),
            c9a: v(14),
            c9b: v(15),
            c11b: v(16),
            g1: v(17),
            g2: v(18),
            g3: v(19),
            phi1: v(20),
            phi2: v(21),
            phi3: v(22),
            phi4: v(23),
            phi5: v(24),
            tau1: v(25),
            tau2: v(26),
            sigma1: v(27),
            sigma2: v(28),
            sigma3: v(29),
            c8: v(30),
            c8a: v(31),
            c8b: v(32),
        };
        match f[0] {
            "PGA" => pga = Some(c),
            "PGV" => pgv = Some(c),
            t => {
                c.period = Some(t.parse().expect("periyot"));
                sa.push(c);
            }
        }
    }
    sa.sort_by(|a, b| a.period.partial_cmp(&b.period).expect("sayı"));
    Table {
        pga: pga.expect("PGA satırı"),
        pgv: pgv.expect("PGV satırı"),
        sa,
    }
}

/// Makale Eş. 4–5: büyüklüğe göre beklenen Ztor (km), merkezleme için.
fn expected_ztor(fault: FaultType, mw: f64) -> f64 {
    let z = if fault == FaultType::Reverse {
        (2.704 - 1.226 * (mw - 5.849).max(0.0)).max(0.0)
    } else {
        (2.673 - 1.136 * (mw - 4.970).max(0.0)).max(0.0)
    };
    z * z
}

/// Tek bir katsayı satırıyla hesap (makale Eş. 11, 12 ve 3.9 / 13).
fn compute(c: &Coeffs, i: &Input) -> LnMotion {
    let m = i.mw;
    // Eş. 11: referans kaya (Vs30 = 1130 m/s) için ln y_ref.
    let r1 = c.c1 + C2 * (m - 6.0) + ((C2 - c.c3) / c.cn) * (1.0 + (c.cn * (c.cm - m)).exp()).ln();
    let r2 = C4 * (i.rrup + c.c5 * (c.c6 * (m - c.chm).max(0.0)).cosh()).ln();
    let gamma = c.g1 + c.g2 / (m - c.g3).max(0.0).cosh();
    let r3 = (C4A - C4) * (i.rrup * i.rrup + CRB * CRB).sqrt().ln() + i.rrup * gamma;
    let cosh_m = (2.0 * (m - 4.5).max(0.0)).cosh();
    let cos_d = i.dip.to_radians().cos();
    let dz = i.ztor - expected_ztor(i.fault, m);
    let mut r4 = (c.c7 + c.c7b / cosh_m) * dz + (C11 + c.c11b / cosh_m) * cos_d * cos_d;
    r4 += match i.fault {
        FaultType::Reverse => c.c1a + c.c1c / cosh_m,
        FaultType::Normal => c.c1b + c.c1d / cosh_m,
        FaultType::StrikeSlip => 0.0,
    };
    // Tavan bloğu etkisi (Rx ≥ 0); dikey fayda cos δ = 0 olduğundan sıfırdır.
    let r5 = if i.rx >= 0.0 {
        c.c9 * cos_d
            * (c.c9a + (1.0 - c.c9a) * (i.rx / c.c9b).tanh())
            * (1.0 - (i.rjb * i.rjb + i.ztor * i.ztor).sqrt() / (i.rrup + 1.0))
    } else {
        0.0
    };
    // Eş. 3.6–3.8: yönelim (ΔDPP = 0 ise sıfır).
    let r6 = if i.dpp == 0.0 {
        0.0
    } else {
        let f_r = (1.0 - (i.rrup - 40.0).max(0.0) / 30.0).max(0.0);
        let f_m = ((m - 5.5).max(0.0) / 0.8).min(1.0);
        c.c8 * f_r * f_m * (-c.c8a * (m - c.c8b).powi(2)).exp() * i.dpp
    };
    let ln_ref = r1 + r2 + r3 + r4 + r5 + r6;
    let y_ref = ln_ref.exp();

    // Eş. 12: zemin etkisi (doğrusal + doğrusal olmayan), havza terimi 0.
    let sl = c.phi1 * (i.vs30 / 1130.0).ln().min(0.0);
    let snl_base = c.phi2
        * ((c.phi3 * (i.vs30.min(1130.0) - 360.0)).exp() - (c.phi3 * (1130.0 - 360.0)).exp());
    let snl = snl_base * ((y_ref + c.phi4) / c.phi4).ln();
    // Havza terimi (Eş. 12): φ5·(1 − exp(−ΔZ1/φ6)); Z1 bilinmediği için
    // ΔZ1 = 0 (Z1, Vs30'dan modelin kendi ilişkisiyle) → terim 0.
    let delta_z1 = 0.0;
    let basin = c.phi5 * (1.0 - (-delta_z1 / PHI6).exp());
    let mean = ln_ref + sl + snl + basin;

    // Belirsizlik: doğrusal olmayan zemin tepkisi NL0 ile ölçeklenir.
    let nl0 = snl_base * y_ref / (y_ref + c.phi4);
    let mt = m.clamp(5.0, 6.5) - 5.0;
    let tau = c.tau1 + (c.tau2 - c.tau1) / 1.5 * mt;
    let sigma_nl0 =
        (c.sigma1 + (c.sigma2 - c.sigma1) / 1.5 * mt) * (c.sigma3 + (1.0 + nl0).powi(2)).sqrt();
    let tau_eff = tau * (1.0 + nl0);
    LnMotion {
        mean,
        sigma: (tau_eff * tau_eff + sigma_nl0 * sigma_nl0).sqrt(),
        tau: tau_eff,
    }
}

/// CY14 tahmini. Model aralığı dışında `None` (sessiz ekstrapolasyon yok).
/// Tabloda olmayan SA periyodu, komşu periyotlardan log-log ara değerle
/// bulunur (Bölüm 7.4).
pub fn predict(imt: Imt, input: &Input) -> Option<LnMotion> {
    if !in_range(input) {
        return None;
    }
    let t = &*TABLE;
    match imt {
        Imt::Pga => Some(compute(&t.pga, input)),
        Imt::Pgv => Some(compute(&t.pgv, input)),
        Imt::Sa(period) => {
            let periods: Vec<f64> = t.sa.iter().map(|c| c.period.expect("SA")).collect();
            super::interp_period(&periods, period, |k| compute(&t.sa[k], input))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Yönelim terimi pygmm 0.9.1 ChiouYoungs2014 (dpp_centered) ile: ΔDPP
    /// verilen ve 0 olan tahminlerin ln farkı (doğrusal olmayan zemin terimi
    /// dahil). Referans 2026-11-26'da .venv'de hesaplandı.
    #[test]
    fn yonelim_pygmm() {
        for (mw, rrup, vs30, dpp, fault, dip, want_sa3, want_pgv) in [
            (
                7.5,
                20.0,
                400.0,
                0.8,
                FaultType::StrikeSlip,
                90.0,
                0.130_775_193,
                0.030_877_965,
            ),
            (
                7.0,
                55.0,
                760.0,
                -0.6,
                FaultType::StrikeSlip,
                90.0,
                -0.060_339_675,
                -0.021_970_117,
            ),
            (
                6.5,
                10.0,
                300.0,
                1.2,
                FaultType::Reverse,
                40.0,
                0.256_455_670,
                0.129_418_799,
            ),
        ] {
            let mk = |dpp| Input {
                mw,
                rrup,
                rjb: rrup,
                rx: -rrup,
                ztor: 0.0,
                dip,
                fault,
                vs30,
                dpp,
            };
            let d =
                |imt| predict(imt, &mk(dpp)).unwrap().mean - predict(imt, &mk(0.0)).unwrap().mean;
            let (sa3, pgv) = (d(Imt::Sa(3.0)), d(Imt::Pgv));
            assert!((sa3 - want_sa3).abs() < 1e-6, "{mw} SA3 {sa3} ≠ {want_sa3}");
            assert!((pgv - want_pgv).abs() < 1e-6, "{mw} PGV {pgv} ≠ {want_pgv}");
        }
    }

    fn fault(s: &str) -> FaultType {
        match s {
            "SS" => FaultType::StrikeSlip,
            "RS" => FaultType::Reverse,
            "NS" => FaultType::Normal,
            _ => panic!("{s}"),
        }
    }

    /// Bölüm 14: referans uygulamayla fark %1'den az. Medyan için ln farkı
    /// < ln(1.01); sigma için göreli fark < %1.
    #[test]
    fn pygmm_referansiyla_yuzde_birden_az_fark() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/test/cy14_ref.csv");
        let csv = std::fs::read_to_string(path).expect("referans dosyası");
        let mut lines = csv.lines();
        let header: Vec<&str> = lines.next().unwrap().split(',').collect();
        let col = |n: &str| header.iter().position(|h| *h == n).unwrap();
        let (mut worst_mean, mut worst_sd, mut n) = (0f64, 0f64, 0);
        for line in lines {
            let f: Vec<&str> = line.split(',').collect();
            let num = |n: &str| f[col(n)].parse::<f64>().unwrap();
            let mut input = Input {
                mw: num("mag"),
                rrup: num("rrup"),
                rjb: num("rjb"),
                rx: num("rx"),
                ztor: num("ztor"),
                dip: num("dip"),
                fault: fault(f[col("mech")]),
                vs30: num("vs30"),
                dpp: 0.0,
            };
            // Referansta tavan bloğu bayrakla verilir; bizde Rx ≥ 0 ile.
            if num("hw") == 0.0 && input.rx >= 0.0 && input.dip < 90.0 {
                input.rx = -1e-9;
            }
            // pygmm aralık dışında da hesaplar (uyarıyla); formül testi için
            // aralık denetimini atlayıp doğrudan hesaplıyoruz.
            let t = &*TABLE;
            let mut check = |c: Option<LnMotion>, m: &str, s: &str| {
                let c = c.expect("hesap");
                worst_mean = worst_mean.max((c.mean - num(m)).abs());
                worst_sd = worst_sd.max((c.sigma / num(s) - 1.0).abs());
                n += 1;
            };
            check(Some(compute(&t.pga, &input)), "ln_pga", "sd_pga");
            check(Some(compute(&t.pgv, &input)), "ln_pgv", "sd_pgv");
            for p in ["0.3", "0.5", "0.75", "1.0"] {
                let period: f64 = p.parse().unwrap();
                // Aralık dışı (ters/normal Mw > 8) SA karşılaştırması atlanır;
                // PGA/PGV formül testi yukarıda aralıktan bağımsız yapıldı.
                let got = if input.mw <= 8.0 || input.fault == FaultType::StrikeSlip {
                    predict(Imt::Sa(period), &input)
                } else {
                    None
                };
                if let Some(g) = got {
                    check(Some(g), &format!("ln_sa{p}"), &format!("sd_sa{p}"));
                }
            }
        }
        println!(
            "{n} karşılaştırma: en büyük |Δln| {worst_mean:.5}, en büyük σ farkı %{:.3}",
            worst_sd * 100.0
        );
        assert!(n > 1500);
        assert!(worst_mean < 1.01f64.ln(), "ortalama farkı {worst_mean}");
        assert!(worst_sd < 0.01, "sigma farkı {worst_sd}");
    }

    #[test]
    fn aralik_disi_none() {
        let i = Input {
            mw: 7.0,
            rrup: 350.0,
            rjb: 350.0,
            rx: -350.0,
            ztor: 0.0,
            dip: 90.0,
            fault: FaultType::StrikeSlip,
            vs30: 760.0,
            dpp: 0.0,
        };
        assert!(predict(Imt::Pga, &i).is_none());
        assert!(
            predict(
                Imt::Pga,
                &Input {
                    rrup: 10.0,
                    rjb: 10.0,
                    ..i
                }
            )
            .is_some()
        );
        assert!(
            predict(
                Imt::Pga,
                &Input {
                    rrup: 10.0,
                    mw: 8.3,
                    fault: FaultType::Reverse,
                    ..i
                }
            )
            .is_none()
        );
        assert!(predict(Imt::Sa(20.0), &Input { rrup: 10.0, ..i }).is_none());
    }

    #[test]
    fn fiziksel_egilimler() {
        let base = Input {
            mw: 6.5,
            rrup: 20.0,
            rjb: 20.0,
            rx: -20.0,
            ztor: 0.0,
            dip: 90.0,
            fault: FaultType::StrikeSlip,
            vs30: 760.0,
            dpp: 0.0,
        };
        let pgv = |i: Input| predict(Imt::Pgv, &i).unwrap().mean;
        // Büyüklük artınca artar, mesafe artınca azalır, yumuşak zeminde artar.
        assert!(pgv(Input { mw: 7.0, ..base }) > pgv(base));
        assert!(
            pgv(Input {
                rrup: 40.0,
                rjb: 40.0,
                ..base
            }) < pgv(base)
        );
        assert!(
            pgv(Input {
                vs30: 300.0,
                ..base
            }) > pgv(base)
        );
    }
}
