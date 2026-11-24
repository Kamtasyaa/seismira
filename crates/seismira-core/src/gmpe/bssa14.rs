//! Boore, Stewart, Seyhan & Atkinson (2014) NGA-West2 modeli, sığ kabuk.
//!
//! Künye: Boore, D.M., Stewart, J.P., Seyhan, E. & Atkinson, G.M. (2014).
//! NGA-West2 equations for predicting PGA, PGV, and 5% damped PSA for
//! shallow crustal earthquakes. Earthquake Spectra 30(3), 1057–1085.
//! doi:10.1193/070113EQS184M
//!
//! Denklemler: PEER Report 2013/05 (aynı yazarlar, açık erişim; SHA256
//! c22c5ac1…), Eş. 3.1–3.11 (ortalama) ve 4.11–4.14 (standart sapma).
//! Katsayılar: USGS nshmp-lib `BSSA14.csv` (CC0; dergi sürümünün tablosu),
//! bkz. `bssa14.csv` başlığı. pygmm 0.9.1 ile karşılaştırılarak test edilir
//! (`data/test/bssa14_ref.csv`).
//!
//! Kapsam ve seçimler:
//! - Bileşen: RotD50.
//! - Bölgesel anelastik düzeltme Δc3 (Eş. 3.7): "global/CATW" (Kaliforniya,
//!   Tayvan ve diğer her yer), "China/Turkey", "Italy/Japan" sütunları.
//! - Havza terimi yok (δz1 = 0; rapor bu varsayılanı öneriyor, s. 19).
//! - Geçerlilik (rapor özeti, s. iii): Mw 3–8.5 doğrultu atımlı ve ters,
//!   3–7 normal; mesafe ≤ 400 km; Vs30 150–1500 m/s. Sınır dışı `None`.

use super::{CrustRegion, Imt, LnMotion};
use crate::quake::FaultType;
use std::sync::LazyLock;

/// Bir saha için girdi.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Input {
    pub mw: f64,
    /// Fayın yüzey izdüşümüne en kısa yatay mesafe (km).
    pub rjb: f64,
    pub fault: FaultType,
    /// Üst 30 m ortalama kayma dalgası hızı (m/s).
    pub vs30: f64,
    pub region: CrustRegion,
}

/// Modelin mesafe sınırı (km, Rjb), rapor özeti.
pub const MAX_RJB_KM: f64 = 400.0;
/// Vs30 geçerlilik aralığı (m/s), uçlar dahil.
pub const VS30_MIN: f64 = 150.0;
pub const VS30_MAX: f64 = 1500.0;

/// Modelin yayımlanmış geçerlilik aralığı içinde mi?
pub fn in_range(i: &Input) -> bool {
    let mw_max = if i.fault == FaultType::Normal {
        7.0
    } else {
        8.5
    };
    (3.0..=mw_max).contains(&i.mw)
        && (0.0..=MAX_RJB_KM).contains(&i.rjb)
        && (VS30_MIN..=VS30_MAX).contains(&i.vs30)
}

#[derive(Clone, Debug)]
struct Coeffs {
    period: Option<f64>, // None: PGA veya PGV satırı
    e1: f64,
    e2: f64,
    e3: f64,
    e4: f64,
    e5: f64,
    e6: f64,
    mh: f64,
    c1: f64,
    c2: f64,
    c3: f64,
    mref: f64,
    rref: f64,
    h: f64,
    dc3_catw: f64,
    dc3_cntr: f64,
    dc3_itjp: f64,
    c: f64,
    vc: f64,
    vref: f64,
    f1: f64,
    f3: f64,
    f4: f64,
    f5: f64,
    r1: f64,
    r2: f64,
    dphi_r: f64,
    dphi_v: f64,
    v1: f64,
    v2: f64,
    phi1: f64,
    phi2: f64,
    tau1: f64,
    tau2: f64,
}

struct Table {
    pga: Coeffs,
    pgv: Coeffs,
    /// Periyoda göre artan sırada.
    sa: Vec<Coeffs>,
}

static TABLE: LazyLock<Table> = LazyLock::new(|| parse(include_str!("bssa14.csv")));

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
    let names = [
        "e1", "e2", "e3", "e4", "e5", "e6", "Mh", "c1", "c2", "c3", "Mref", "Rref", "h", "Dc3CaTw",
        "Dc3CnTr", "Dc3ItJp", "c", "Vc", "Vref", "f1", "f3", "f4", "f5", "R1", "R2", "dPhiR",
        "dPhiV", "v1", "v2", "phi1", "phi2", "tau1", "tau2",
    ];
    let idx: Vec<usize> = names.iter().map(|n| col(n)).collect();
    let (mut pga, mut pgv, mut sa) = (None, None, Vec::new());
    for line in lines.filter(|l| !l.trim().is_empty()) {
        let f: Vec<&str> = line.split(',').map(str::trim).collect();
        let v: Vec<f64> = idx.iter().map(|&k| f[k].parse().expect("sayı")).collect();
        let mut c = Coeffs {
            period: None,
            e1: v[0],
            e2: v[1],
            e3: v[2],
            e4: v[3],
            e5: v[4],
            e6: v[5],
            mh: v[6],
            c1: v[7],
            c2: v[8],
            c3: v[9],
            mref: v[10],
            rref: v[11],
            h: v[12],
            dc3_catw: v[13],
            dc3_cntr: v[14],
            dc3_itjp: v[15],
            c: v[16],
            vc: v[17],
            vref: v[18],
            f1: v[19],
            f3: v[20],
            f4: v[21],
            f5: v[22],
            r1: v[23],
            r2: v[24],
            dphi_r: v[25],
            dphi_v: v[26],
            v1: v[27],
            v2: v[28],
            phi1: v[29],
            phi2: v[30],
            tau1: v[31],
            tau2: v[32],
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

/// Bölgenin Δc3 sütunu (Eş. 3.7; rapor Bölüm 4.4 ve tablo başlığı:
/// "global/CATW", "China/Turkey", "Italy/Japan").
fn dc3(c: &Coeffs, region: CrustRegion) -> f64 {
    match region {
        CrustRegion::China | CrustRegion::Turkey => c.dc3_cntr,
        CrustRegion::Italy | CrustRegion::Japan => c.dc3_itjp,
        _ => c.dc3_catw,
    }
}

/// Kaynak + yol terimi, zemin terimi olmadan: Eş. 3.3–3.5 ve 3.7.
fn ln_rock(c: &Coeffs, i: &Input) -> f64 {
    let m = i.mw;
    // Eş. 3.5 (Tablo 3.1: tip belirtilmemiş kolu kullanılmaz).
    let mech = match i.fault {
        FaultType::StrikeSlip => c.e1,
        FaultType::Normal => c.e2,
        FaultType::Reverse => c.e3,
    };
    let fe = if m <= c.mh {
        mech + c.e4 * (m - c.mh) + c.e5 * (m - c.mh).powi(2)
    } else {
        mech + c.e6 * (m - c.mh)
    };
    // Eş. 3.3, 3.4 ve 3.7.
    let r = (i.rjb * i.rjb + c.h * c.h).sqrt();
    let fp =
        (c.c1 + c.c2 * (m - c.mref)) * (r / c.rref).ln() + (c.c3 + dc3(c, i.region)) * (r - c.rref);
    fe + fp
}

/// Tek bir katsayı satırıyla hesap. `pga_r`: referans kaya (Vs30 =
/// 760 m/s) PGA'sı (g), Eş. 3.10 için.
fn compute(c: &Coeffs, i: &Input, pga_r: f64) -> LnMotion {
    // Eş. 3.9: doğrusal zemin terimi.
    let flin = c.c * (i.vs30.min(c.vc) / c.vref).ln();
    // Eş. 3.10–3.11: doğrusal olmayan zemin terimi.
    let f2 = c.f4 * ((c.f5 * (i.vs30.min(760.0) - 360.0)).exp() - (c.f5 * (760.0 - 360.0)).exp());
    let fnl = c.f1 + f2 * ((pga_r + c.f3) / c.f3).ln();
    let mean = ln_rock(c, i) + flin + fnl;

    // Eş. 4.11–4.14: standart sapmalar.
    let m = i.mw;
    let ramp = |a: f64, b: f64| {
        if m <= 4.5 {
            a
        } else if m >= 5.5 {
            b
        } else {
            a + (b - a) * (m - 4.5)
        }
    };
    let tau = ramp(c.tau1, c.tau2);
    let mut phi = ramp(c.phi1, c.phi2);
    if i.rjb > c.r2 {
        phi += c.dphi_r;
    } else if i.rjb > c.r1 {
        phi += c.dphi_r * (i.rjb / c.r1).ln() / (c.r2 / c.r1).ln();
    }
    if i.vs30 <= c.v1 {
        phi -= c.dphi_v;
    } else if i.vs30 < c.v2 {
        phi -= c.dphi_v * (c.v2 / i.vs30).ln() / (c.v2 / c.v1).ln();
    }
    LnMotion {
        mean,
        sigma: (phi * phi + tau * tau).sqrt(),
        tau,
    }
}

/// BSSA14 tahmini. Model aralığı dışında `None` (sessiz ekstrapolasyon yok).
/// Tabloda olmayan SA periyodu, komşu periyotlardan log-log ara değerle
/// bulunur (Bölüm 7.4).
pub fn predict(imt: Imt, input: &Input) -> Option<LnMotion> {
    if !in_range(input) {
        return None;
    }
    let t = &*TABLE;
    // Referans kaya PGA'sı: Eş. 3.1, Vs30 = 760 m/s'de zemin terimi sıfır
    // (Eş. 3.9'da ln(760/760) = 0, Eş. 3.11'de f2 = 0); bölge terimi dahil.
    let pga_r = ln_rock(&t.pga, input).exp();
    match imt {
        Imt::Pga => Some(compute(&t.pga, input, pga_r)),
        Imt::Pgv => Some(compute(&t.pgv, input, pga_r)),
        Imt::Sa(period) => {
            let periods: Vec<f64> = t.sa.iter().map(|c| c.period.unwrap()).collect();
            super::interp_period(&periods, period, |k| compute(&t.sa[k], input, pga_r))
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
            "turkey" => CrustRegion::Turkey,
            "japan" => CrustRegion::Japan,
            _ => panic!("{s}"),
        }
    }

    /// pygmm 0.9.1 ile karşılaştırma (scripts/gen_gmpe_ref.py): ortalama ve
    /// standart sapma ln biriminde 0,01 içinde (≈ %1, CLAUDE.md Bölüm 14).
    #[test]
    fn pygmm_ile_ayni() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/test/bssa14_ref.csv"
        );
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
                rjb: num("rjb"),
                fault: fault(f[col("mech")]),
                vs30: num("vs30"),
                region: region(f[col("region")]),
            };
            let mut check = |imt: Imt, mean: &str, sd: &str| {
                let p = predict(imt, &input).expect("aralıkta");
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

    /// Bölgesel Δc3 yalnızca uzak alanda etkili: Türkiye (yavaş sönüm) daha
    /// güçlü, Japonya (hızlı sönüm) daha zayıf; R = Rref'te fark yok.
    #[test]
    fn bolgesel_sonum() {
        let base = Input {
            mw: 7.0,
            rjb: 200.0,
            fault: FaultType::StrikeSlip,
            vs30: 400.0,
            region: CrustRegion::Global,
        };
        let at = |r| {
            predict(Imt::Pgv, &Input { region: r, ..base })
                .unwrap()
                .mean
        };
        assert!(at(CrustRegion::Turkey) > at(CrustRegion::Global));
        assert!(at(CrustRegion::Japan) < at(CrustRegion::Global));
        assert_eq!(at(CrustRegion::Taiwan), at(CrustRegion::Global));
    }

    #[test]
    fn aralik_disi() {
        let i = Input {
            mw: 7.5,
            rjb: 10.0,
            fault: FaultType::Normal,
            vs30: 400.0,
            region: CrustRegion::Global,
        };
        assert!(predict(Imt::Pga, &i).is_none()); // normal fay Mw ≤ 7
        let ok = Input { mw: 7.0, ..i };
        assert!(predict(Imt::Pga, &ok).is_some());
        assert!(predict(Imt::Pga, &Input { rjb: 401.0, ..ok }).is_none());
        assert!(predict(Imt::Pga, &Input { vs30: 140.0, ..ok }).is_none());
    }
}
