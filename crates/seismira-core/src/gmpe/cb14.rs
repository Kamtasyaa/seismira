//! Campbell & Bozorgnia (2014) NGA-West2 modeli, sığ kabuk.
//!
//! Künye: Campbell, K.W. & Bozorgnia, Y. (2014). NGA-West2 ground motion
//! model for the average horizontal components of PGA, PGV, and 5% damped
//! linear acceleration response spectra. Earthquake Spectra 30(3),
//! 1087–1115. doi:10.1193/062913EQS175M
//!
//! Denklemler: PEER Report 2013/06 (aynı yazarlar, açık erişim; SHA256
//! 047ad1d5…) ve düzeltme notu (2013-07-15; SHA256 75843267…): Eş. 3.1–3.25
//! (ortalama), 3.27–3.30 (standart sapma), 6.9–6.10 (Z2.5 ↔ Vs30).
//! Raporun katsayı tabloları ön sürüm; katsayılar dergi sürümünden: USGS
//! nshmp-lib `CB14.csv` (CC0), bkz. `cb14.csv` başlığı. pygmm 0.9.1 ile
//! karşılaştırılarak test edilir (`data/test/cb14_ref.csv`).
//!
//! Kapsam ve seçimler:
//! - Bileşen: RotD50.
//! - Bölgesel anelastik düzeltme Δc20 (Eş. 3.25, Tablo 3.3): "JP" Japonya
//!   ve İtalya, "CH" Çin; diğerleri "CA" (0). Japonya zemin terimi (Eş.
//!   3.19, SJ = 1) ve Z2.5 bağıntısı (Eş. 6.10) Japonyada.
//! - Z2.5 bilinmediğinden Vs30'dan tahmin (Eş. 6.9 Kaliforniya, 6.10
//!   Japonya; rapor bunu "bilinmiyorsa" diye öneriyor, s. 63). Eş. 3.19'un
//!   iki kolu pygmm'deki gibi ayrık (Vs30 ≤ 200 / > 200).
//! - Odak derinliği depremin kendi derinliği (Eş. 3.22).
//! - Geçerlilik (rapor Bölüm 5.1, s. 57): Mw ≥ 3.3, ≤ 8.5 doğrultu atımlı,
//!   ≤ 8.0 ters, ≤ 7.5 normal; Rrup 0–300 km; Vs30 150–1500 m/s; Ztor
//!   0–20 km; Zhyp 0–20 km; eğim 15–90°. Sınır dışı `None`.

use super::{CrustRegion, Imt, LnMotion};
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
    /// Odak derinliği (km).
    pub zhyp: f64,
    /// Eğim (derece).
    pub dip: f64,
    /// Kırılma genişliği (km).
    pub width: f64,
    pub fault: FaultType,
    /// Üst 30 m ortalama kayma dalgası hızı (m/s).
    pub vs30: f64,
    pub region: CrustRegion,
}

/// Modelin mesafe sınırı (km, Rrup).
pub const MAX_RRUP_KM: f64 = 300.0;
/// Vs30 geçerlilik aralığı (m/s), uçlar dahil.
pub const VS30_MIN: f64 = 150.0;
pub const VS30_MAX: f64 = 1500.0;
/// Referans kaya (A1100).
const VS30_REF: f64 = 1100.0;
/// Tablo 3.1 notu: tüm periyotlar için.
const C: f64 = 1.88;
const N: f64 = 1.18;

/// Modelin yayımlanmış geçerlilik aralığı içinde mi?
pub fn in_range(i: &Input) -> bool {
    let mw_max = match i.fault {
        FaultType::StrikeSlip => 8.5,
        FaultType::Reverse => 8.0,
        FaultType::Normal => 7.5,
    };
    (3.3..=mw_max).contains(&i.mw)
        && (0.0..=MAX_RRUP_KM).contains(&i.rrup)
        && (VS30_MIN..=VS30_MAX).contains(&i.vs30)
        && (0.0..=20.0).contains(&i.ztor)
        && (0.0..=20.0).contains(&i.zhyp)
        && (15.0..=90.0).contains(&i.dip)
}

#[derive(Clone, Debug)]
struct Coeffs {
    /// Periyot (s); PGA ve PGV için `None`.
    period: Option<f64>,
    c: [f64; 20], // c0..c19
    c20: f64,
    dc20_jp: f64,
    dc20_ch: f64,
    a2: f64,
    h: [f64; 7], // h1..h6 (indeks 1..6; 0 kullanılmaz)
    k1: f64,
    k2: f64,
    k3: f64,
    phi1: f64,
    phi2: f64,
    tau1: f64,
    tau2: f64,
    phi_lnaf: f64,
    rho: f64,
}

struct Table {
    pga: Coeffs,
    pgv: Coeffs,
    /// Periyoda göre artan sırada.
    sa: Vec<Coeffs>,
}

static TABLE: LazyLock<Table> = LazyLock::new(|| parse(include_str!("cb14.csv")));

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
    let (mut pga, mut pgv, mut sa) = (None, None, Vec::new());
    for line in lines.filter(|l| !l.trim().is_empty()) {
        let f: Vec<&str> = line.split(',').map(str::trim).collect();
        let v = |n: &str| -> f64 { f[col(n)].parse().expect("sayı") };
        let mut h = [0.0; 7];
        for (k, slot) in h.iter_mut().enumerate().skip(1) {
            *slot = v(&format!("h{k}"));
        }
        let mut c = Coeffs {
            period: None,
            c: std::array::from_fn(|k| v(&format!("c{k}"))),
            c20: v("c20"),
            dc20_jp: v("Dc20_JP"),
            dc20_ch: v("Dc20_CH"),
            a2: v("a2"),
            h,
            k1: v("k1"),
            k2: v("k2"),
            k3: v("k3"),
            phi1: v("phi1"),
            phi2: v("phi2"),
            tau1: v("tau1"),
            tau2: v("tau2"),
            phi_lnaf: v("phi_lnaf"),
            rho: v("rho"),
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

/// Eş. 6.9 (Kaliforniya) ve 6.10 (Japonya): Vs30'dan Z2.5 (km).
fn z25(vs30: f64, region: CrustRegion) -> f64 {
    if region == CrustRegion::Japan {
        (5.359 - 1.102 * vs30.ln()).exp()
    } else {
        (7.089 - 1.144 * vs30.ln()).exp()
    }
}

/// Zemin dışı terimler: Eş. 3.2–3.16 ve 3.21–3.25.
fn ln_source(c: &Coeffs, i: &Input) -> f64 {
    let m = i.mw;
    let k = &c.c;
    // Eş. 3.2: her eğim kendi büyüklüğünün üstünde.
    let mut f_mag = k[0] + k[1] * m;
    for (m0, slope) in [(4.5, k[2]), (5.5, k[3]), (6.5, k[4])] {
        if m > m0 {
            f_mag += slope * (m - m0);
        }
    }
    // Eş. 3.3.
    let f_dis = (k[5] + k[6] * m) * (i.rrup * i.rrup + k[7] * k[7]).sqrt().ln();
    // Eş. 3.4–3.6.
    let taper = (m - 4.5).clamp(0.0, 1.0);
    let f_flt = match i.fault {
        FaultType::Reverse => k[8] * taper,
        FaultType::Normal => k[9] * taper,
        FaultType::StrikeSlip => 0.0,
    };
    // Eş. 3.7–3.16.
    let h = &c.h;
    let r1 = i.width * i.dip.to_radians().cos();
    let r2 = 62.0 * m - 350.0;
    let f_rx = if i.rx < 0.0 {
        0.0
    } else if i.rx <= r1 {
        let q = i.rx / r1;
        h[1] + h[2] * q + h[3] * q * q
    } else {
        let q = (i.rx - r1) / (r2 - r1);
        (h[4] + h[5] * q + h[6] * q * q).max(0.0)
    };
    let f_rrup = if i.rrup == 0.0 {
        1.0
    } else {
        (i.rrup - i.rjb) / i.rrup
    };
    let f_m = if m <= 5.5 {
        0.0
    } else {
        (m - 5.5).min(1.0) * (1.0 + c.a2 * (m - 6.5))
    };
    let f_z = if i.ztor > 16.66 {
        0.0
    } else {
        1.0 - 0.06 * i.ztor
    };
    let f_dip_hw = (90.0 - i.dip) / 45.0;
    let f_hng = k[10] * f_rx * f_rrup * f_m * f_z * f_dip_hw;
    // Eş. 3.21–3.23.
    let f_hyp =
        (i.zhyp - 7.0).clamp(0.0, 13.0) * (k[17] + (k[18] - k[17]) * (m - 5.5).clamp(0.0, 1.0));
    // Eş. 3.24.
    let f_dip = k[19] * i.dip * (5.5 - m).clamp(0.0, 1.0);
    // Eş. 3.25.
    let dc20 = match i.region {
        CrustRegion::Japan | CrustRegion::Italy => c.dc20_jp,
        CrustRegion::China => c.dc20_ch,
        _ => 0.0,
    };
    let f_atn = (c.c20 + dc20) * (i.rrup - 80.0).max(0.0);
    f_mag + f_dis + f_flt + f_hng + f_hyp + f_dip + f_atn
}

/// Eş. 3.17–3.20: zemin ve havza terimi. `a1100`: referans kaya PGA (g);
/// Vs30 > k1 ise kullanılmaz.
fn site(c: &Coeffs, vs30: f64, a1100: f64, region: CrustRegion) -> f64 {
    let k = &c.c;
    let ratio = vs30 / c.k1;
    let mut f = if vs30 <= c.k1 {
        k[11] * ratio.ln() + c.k2 * ((a1100 + C * ratio.powf(N)).ln() - (a1100 + C).ln())
    } else {
        (k[11] + c.k2 * N) * ratio.ln()
    };
    let japan = region == CrustRegion::Japan;
    if japan {
        f += if vs30 <= 200.0 {
            (k[12] + c.k2 * N) * (ratio.ln() - (200.0 / c.k1).ln())
        } else {
            (k[13] + c.k2 * N) * ratio.ln()
        };
    }
    // Eş. 3.20, Z2.5 Vs30'dan.
    let z = z25(vs30, region);
    let f_sed = if z <= 1.0 {
        (k[14] + if japan { k[15] } else { 0.0 }) * (z - 1.0)
    } else if z <= 3.0 {
        0.0
    } else {
        k[16] * c.k3 * (-0.75f64).exp() * (1.0 - (-0.25 * (z - 3.0)).exp())
    };
    f + f_sed
}

fn ln_y(c: &Coeffs, i: &Input, a1100: f64) -> f64 {
    ln_source(c, i) + site(c, i.vs30, a1100, i.region)
}

/// Eş. 3.27–3.30 (ρ düzeltme notundaki değerler, katsayı dosyasında).
fn sigma(c: &Coeffs, p: &Coeffs, i: &Input, a1100: f64) -> (f64, f64) {
    let ramp = |v1: f64, v2: f64| v2 + (v1 - v2) * (5.5 - i.mw).clamp(0.0, 1.0);
    let tau_y = ramp(c.tau1, c.tau2);
    let phi_y = ramp(c.phi1, c.phi2);
    let alpha = if i.vs30 < c.k1 {
        let ratio = i.vs30 / c.k1;
        c.k2 * a1100 * (1.0 / (a1100 + C * ratio.powf(N)) - 1.0 / (a1100 + C))
    } else {
        0.0
    };
    let tau_pga = ramp(p.tau1, p.tau2);
    let tau =
        (tau_y * tau_y + alpha * alpha * tau_pga * tau_pga + 2.0 * alpha * c.rho * tau_y * tau_pga)
            .sqrt();
    let phi_pga = ramp(p.phi1, p.phi2);
    let phi_pga_b = (phi_pga * phi_pga - p.phi_lnaf * p.phi_lnaf).sqrt();
    let phi_y_b = (phi_y * phi_y - c.phi_lnaf * c.phi_lnaf).sqrt();
    let phi = (phi_y_b * phi_y_b
        + c.phi_lnaf * c.phi_lnaf
        + alpha * alpha * phi_pga_b * phi_pga_b
        + 2.0 * alpha * c.rho * phi_y_b * phi_pga_b)
        .sqrt();
    ((phi * phi + tau * tau).sqrt(), tau)
}

fn compute(c: &Coeffs, i: &Input, a1100: f64, ln_pga: f64) -> LnMotion {
    let mut mean = ln_y(c, i, a1100);
    // Eş. 3.1: T < 0,25 s'de PSA, PGA'dan küçük olamaz.
    if c.period.is_some_and(|t| t < 0.25) {
        mean = mean.max(ln_pga);
    }
    let (sigma, tau) = sigma(c, &TABLE.pga, i, a1100);
    LnMotion { mean, sigma, tau }
}

/// CB14 tahmini. Model aralığı dışında `None` (sessiz ekstrapolasyon yok).
/// Tabloda olmayan SA periyodu, komşu periyotlardan log-log ara değerle
/// bulunur (Bölüm 7.4).
pub fn predict(imt: Imt, input: &Input) -> Option<LnMotion> {
    if !in_range(input) {
        return None;
    }
    let t = &*TABLE;
    // A1100: Vs30 = 1100 m/s'de PGA (Z2.5 de o Vs30'dan); k1 < 1100
    // olduğundan zemin terimi doğrusal, A1100 gerekmez.
    let rock = Input {
        vs30: VS30_REF,
        ..*input
    };
    let a1100 = ln_y(&t.pga, &rock, f64::NAN).exp();
    let ln_pga = ln_y(&t.pga, input, a1100);
    match imt {
        Imt::Pga => Some(compute(&t.pga, input, a1100, ln_pga)),
        Imt::Pgv => Some(compute(&t.pgv, input, a1100, ln_pga)),
        Imt::Sa(period) => {
            let periods: Vec<f64> = t.sa.iter().map(|c| c.period.unwrap()).collect();
            super::interp_period(&periods, period, |k| {
                compute(&t.sa[k], input, a1100, ln_pga)
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fault(s: &str) -> FaultType {
        match s {
            "SS" => FaultType::StrikeSlip,
            "RS" => FaultType::Reverse,
            "NS" => FaultType::Normal,
            _ => panic!("{s}"),
        }
    }

    fn region(s: &str) -> CrustRegion {
        match s {
            "global" => CrustRegion::Global,
            "japan" => CrustRegion::Japan,
            "china" => CrustRegion::China,
            _ => panic!("{s}"),
        }
    }

    /// pygmm 0.9.1 ile karşılaştırma (scripts/gen_gmpe_ref.py): ortalama ve
    /// standart sapma ln biriminde 0,01 içinde (≈ %1, CLAUDE.md Bölüm 14).
    #[test]
    fn pygmm_ile_ayni() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/test/cb14_ref.csv");
        let csv = std::fs::read_to_string(path).expect("referans dosyası");
        let mut lines = csv.lines();
        let header: Vec<&str> = lines.next().unwrap().split(',').collect();
        let col = |n: &str| header.iter().position(|h| *h == n).unwrap();
        let (mut worst_mean, mut worst_sd, mut n) = (0f64, 0f64, 0);
        for line in lines {
            let f: Vec<&str> = line.split(',').collect();
            let num = |n: &str| f[col(n)].parse::<f64>().unwrap();
            let input = Input {
                mw: num("mag"),
                rrup: num("rrup"),
                rjb: num("rjb"),
                rx: num("rx"),
                ztor: num("ztor"),
                zhyp: num("zhyp"),
                dip: num("dip"),
                width: num("width"),
                fault: fault(f[col("mech")]),
                vs30: num("vs30"),
                region: region(f[col("region")]),
            };
            let mut check = |imt: Imt, mean: &str, sd: &str| {
                let Some(p) = predict(imt, &input) else {
                    return;
                };
                worst_mean = worst_mean.max((p.mean - num(mean)).abs());
                worst_sd = worst_sd.max((p.sigma - num(sd)).abs());
                n += 1;
            };
            check(Imt::Pga, "ln_pga", "sd_pga");
            check(Imt::Pgv, "ln_pgv", "sd_pgv");
            for t in ["0.3", "0.5", "0.75", "1.0"] {
                let period: f64 = t.parse().unwrap();
                check(Imt::Sa(period), &format!("ln_sa{t}"), &format!("sd_sa{t}"));
            }
        }
        assert!(n > 500, "{n}");
        assert!(worst_mean < 0.01, "en büyük ortalama farkı {worst_mean}");
        assert!(worst_sd < 0.01, "en büyük σ farkı {worst_sd}");
    }

    #[test]
    fn aralik_disi() {
        let i = Input {
            mw: 7.0,
            rrup: 10.0,
            rjb: 10.0,
            rx: -10.0,
            ztor: 0.0,
            zhyp: 10.0,
            dip: 90.0,
            width: 15.0,
            fault: FaultType::StrikeSlip,
            vs30: 400.0,
            region: CrustRegion::Global,
        };
        assert!(predict(Imt::Pga, &i).is_some());
        assert!(
            predict(
                Imt::Pga,
                &Input {
                    mw: 7.6,
                    fault: FaultType::Normal,
                    ..i
                }
            )
            .is_none()
        );
        assert!(predict(Imt::Pga, &Input { zhyp: 25.0, ..i }).is_none());
        assert!(predict(Imt::Pga, &Input { dip: 10.0, ..i }).is_none());
        assert!(predict(Imt::Pga, &Input { rrup: 301.0, ..i }).is_none());
    }
}
