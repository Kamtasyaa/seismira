//! Abrahamson, Silva & Kamai (2014) NGA-West2 modeli, sığ kabuk.
//!
//! Künye: Abrahamson, N.A., Silva, W.J. & Kamai, R. (2014). Summary of the
//! ASK14 ground motion relation for active crustal regions. Earthquake
//! Spectra 30(3), 1025–1055. doi:10.1193/070913EQS198M
//!
//! Denklemler: PEER Report 2013/04 (aynı yazarlar, açık erişim; SHA256
//! 84f2ee74…), Eş. 4.1–4.24 (ortalama) ve 7.1–7.10 (standart sapma). Dergi
//! sürümü görülmedi (kapalı erişim); rapordan şu üç farkı pygmm 0.9.1 ve
//! OpenQuake `abrahamson_2014.py` (yalnızca değer / biçim karşılaştırma)
//! aynı biçimde uyguluyor, dergi biçimi olarak onlar izlendi:
//! - Referans kaya Vs30 = 1180 m/s (raporda 1100): Sa1180.
//! - Japonya zemin terimi (rapor Eş. 4.24 basamak fonksiyonu) Vs30 bin
//!   merkezleri 150, 250, 350, 450, 600, 850, 1150 m/s arasında doğrusal ara
//!   değer, uçlarda sabit.
//! - Tayvan Vs30 terimi (Eş. 4.23) min(Vs30, V1) ile.
//!
//! Katsayılar: USGS nshmp-lib `ASK14.csv` (CC0), bkz. `ask14.csv` başlığı.
//! pygmm 0.9.1 ile karşılaştırılarak test edilir (`data/test/ask14_ref.csv`).
//!
//! Kapsam ve seçimler:
//! - Bileşen: RotD50.
//! - Bölge (Eş. 4.22): Tayvan, Çin, Japonya; diğerleri temel model.
//! - Havza terimi yok (Z1 = Z1,ref → f10 = 0); artçı şok terimi yok.
//! - Tavan bloğu T5 kesmesi Rjb ile (Eş. 4.15b; Ry0 hesaplanmıyor, rapor bu
//!   seçeneği veriyor).
//! - Vs30 "tahmini" kabul edilir (haritadan; s1e, s2e).
//! - Geçerlilik: Mw 3–8.5, Rrup 0–300 km (rapor özeti); Vs30 180–1000 m/s
//!   (alt sınır rapor s. 59: "180 m/s ve altında kullanılmamalı"; üst sınır
//!   pygmm'in önerdiği aralık, birincil kaynakta görülmedi).

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
    /// Eğim (derece).
    pub dip: f64,
    /// Kırılma genişliği (km), tavan bloğu terimi için.
    pub width: f64,
    pub fault: FaultType,
    /// Üst 30 m ortalama kayma dalgası hızı (m/s).
    pub vs30: f64,
    pub region: CrustRegion,
}

/// Modelin mesafe sınırı (km, Rrup).
pub const MAX_RRUP_KM: f64 = 300.0;
/// Vs30 geçerlilik aralığı (m/s), uçlar dahil.
pub const VS30_MIN: f64 = 180.0;
pub const VS30_MAX: f64 = 1000.0;
/// Referans kaya (dergi sürümü; bkz. modül notu).
const VS30_REF: f64 = 1180.0;
/// Eş. 4.2'deki ikinci büyüklük kırılması.
const M2: f64 = 5.0;

/// Modelin yayımlanmış geçerlilik aralığı içinde mi?
pub fn in_range(i: &Input) -> bool {
    (3.0..=8.5).contains(&i.mw)
        && (0.0..=MAX_RRUP_KM).contains(&i.rrup)
        && (VS30_MIN..=VS30_MAX).contains(&i.vs30)
}

#[derive(Clone, Debug)]
struct Coeffs {
    /// Periyot (s); PGA 0, PGV için `None`.
    period: Option<f64>,
    vlin: f64,
    b: f64,
    n: f64,
    m1: f64,
    c: f64,
    c4: f64,
    a: [f64; 9], // a1..a8 (indeks 1..8; 0 kullanılmaz)
    a10: f64,
    a11: f64,
    a12: f64,
    a13: f64,
    a15: f64,
    a17: f64,
    a25: f64,
    a28: f64,
    a29: f64,
    a31: f64,
    /// a36..a42 (Japonya Vs30 binleri).
    jp: [f64; 7],
    s1e: f64,
    s2e: f64,
    s3: f64,
    s4: f64,
    s5: f64,
    s6: f64,
}

struct Table {
    pga: Coeffs,
    pgv: Coeffs,
    /// Periyoda göre artan sırada.
    sa: Vec<Coeffs>,
}

static TABLE: LazyLock<Table> = LazyLock::new(|| parse(include_str!("ask14.csv")));

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
        let mut a = [0.0; 9];
        for (k, slot) in a.iter_mut().enumerate().skip(1) {
            *slot = v(&format!("a{k}"));
        }
        let jp = std::array::from_fn(|k| v(&format!("a{}", 36 + k)));
        let mut c = Coeffs {
            period: None,
            vlin: v("Vlin"),
            b: v("b"),
            n: v("n"),
            m1: v("M1"),
            c: v("c"),
            c4: v("c4"),
            a,
            a10: v("a10"),
            a11: v("a11"),
            a12: v("a12"),
            a13: v("a13"),
            a15: v("a15"),
            a17: v("a17"),
            a25: v("a25"),
            a28: v("a28"),
            a29: v("a29"),
            a31: v("a31"),
            jp,
            s1e: v("s1e"),
            s2e: v("s2e"),
            s3: v("s3"),
            s4: v("s4"),
            s5: v("s5"),
            s6: v("s6"),
        };
        match f[0] {
            "PGA" => {
                c.period = Some(0.0);
                pga = Some(c);
            }
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

/// Eş. 4.9: Vs30 ölçeklemesinin durduğu hız (m/s). PGV: 1500 (pygmm ve
/// OpenQuake; rapor yalnızca periyoda göre veriyor).
fn v1(c: &Coeffs) -> f64 {
    match c.period {
        Some(t) if t >= 3.0 => 800.0,
        Some(t) if t > 0.5 => (-0.35 * (t / 0.5).ln() + 1500f64.ln()).exp(),
        _ => 1500.0,
    }
}

/// Eş. 4.2–4.4: büyüklük ve mesafe ölçeklemesi.
fn f1(c: &Coeffs, m: f64, rrup: f64) -> f64 {
    let a = &c.a;
    let c4m = c.c4 - (c.c4 - 1.0) * (5.0 - m).clamp(0.0, 1.0);
    let ln_r = (rrup * rrup + c4m * c4m).sqrt().ln();
    let base = if m <= M2 {
        a[4] * (M2 - c.m1)
            + a[8] * (8.5 - M2).powi(2)
            + a[6] * (m - M2)
            + a[7] * (m - M2).powi(2)
            + (a[2] + a[3] * (M2 - c.m1)) * ln_r
    } else {
        let slope = if m <= c.m1 { a[4] } else { a[5] };
        slope * (m - c.m1) + a[8] * (8.5 - m).powi(2) + (a[2] + a[3] * (m - c.m1)) * ln_r
    };
    a[1] + base + c.a17 * rrup
}

/// Eş. 4.10–4.15b: tavan bloğu terimi. Taraf Rx'in işaretinden: −0 (taban
/// bloğu tarafında, izin hemen üstü) taban bloğudur.
fn f4(c: &Coeffs, i: &Input) -> f64 {
    if i.rx.is_sign_negative() {
        return 0.0;
    }
    let t1 = (90.0 - i.dip).min(60.0) / 45.0;
    let m = i.mw;
    let a2hw = 0.2;
    let t2 = if m >= 6.5 {
        1.0 + a2hw * (m - 6.5)
    } else if m > 5.5 {
        1.0 + a2hw * (m - 6.5) - (1.0 - a2hw) * (m - 6.5).powi(2)
    } else {
        0.0
    };
    let r1 = i.width * i.dip.to_radians().cos();
    let r2 = 3.0 * r1;
    let (h1, h2, h3) = (0.25, 1.5, -0.75);
    let t3 = if i.rx < r1 {
        h1 + h2 * (i.rx / r1) + h3 * (i.rx / r1).powi(2)
    } else if i.rx <= r2 {
        1.0 - (i.rx - r1) / (r2 - r1)
    } else {
        0.0
    };
    let t4 = (1.0 - i.ztor * i.ztor / 100.0).clamp(0.0, 1.0);
    let t5 = (1.0 - i.rjb / 30.0).clamp(0.0, 1.0);
    c.a13 * t1 * t2 * t3 * t4 * t5
}

/// Zemin dışı terimler (kaynak + yol + tavan bloğu + Ztor + fay tipi).
fn ln_source(c: &Coeffs, i: &Input) -> f64 {
    let taper = (i.mw - 4.0).clamp(0.0, 1.0); // Eş. 4.5–4.6
    let sof = match i.fault {
        FaultType::Reverse => c.a11 * taper,
        FaultType::Normal => c.a12 * taper,
        FaultType::StrikeSlip => 0.0,
    };
    let f6 = c.a15 * (i.ztor / 20.0).clamp(0.0, 1.0); // Eş. 4.16
    f1(c, i.mw, i.rrup) + f4(c, i) + f6 + sof
}

/// Eş. 4.22–4.24: bölgesel terim (Vs30'a bağlı kısım dahil).
fn regional(c: &Coeffs, i: &Input, vs30: f64) -> f64 {
    match i.region {
        CrustRegion::Taiwan => c.a31 * (vs30.min(v1(c)) / c.vlin).ln() + c.a25 * i.rrup,
        CrustRegion::China => c.a28 * i.rrup,
        CrustRegion::Japan => {
            const BINS: [f64; 7] = [150.0, 250.0, 350.0, 450.0, 600.0, 850.0, 1150.0];
            let f13 = if vs30 <= BINS[0] {
                c.jp[0]
            } else if vs30 >= BINS[6] {
                c.jp[6]
            } else {
                let k = BINS.partition_point(|&b| b < vs30);
                let t = (vs30 - BINS[k - 1]) / (BINS[k] - BINS[k - 1]);
                c.jp[k - 1] + t * (c.jp[k] - c.jp[k - 1])
            };
            f13 + c.a29 * i.rrup
        }
        _ => 0.0,
    }
}

/// Eş. 4.7–4.8: zemin terimi. `sa_ref`: referans kayada tahmin (Sa1180).
fn f5(c: &Coeffs, vs30: f64, sa_ref: f64) -> f64 {
    let ratio = vs30.min(v1(c)) / c.vlin;
    if ratio < 1.0 {
        c.a10 * ratio.ln() - c.b * (sa_ref + c.c).ln() + c.b * (sa_ref + c.c * ratio.powf(c.n)).ln()
    } else {
        (c.a10 + c.b * c.n) * ratio.ln()
    }
}

fn compute(c: &Coeffs, i: &Input) -> LnMotion {
    let src = ln_source(c, i);
    // Referans kaya: zemin terimi doğrusal (Vs30 ≥ Vlin), bölge terimi dahil.
    let sa_ref = (src + regional(c, i, VS30_REF) + f5(c, VS30_REF, f64::NAN)).exp();
    let mean = src + regional(c, i, i.vs30) + f5(c, i.vs30, sa_ref);

    // Eş. 7.1–7.3 ve 7.7–7.10.
    let m = i.mw;
    let phi_al = if i.region == CrustRegion::Japan {
        c.s5 + (c.s6 - c.s5) * ((i.rrup - 30.0) / 50.0).clamp(0.0, 1.0)
    } else {
        c.s1e + (c.s2e - c.s1e) * ((m - 4.0) / 2.0).clamp(0.0, 1.0)
    };
    let tau_al = c.s3 + (c.s4 - c.s3) * ((m - 5.0) / 2.0).clamp(0.0, 1.0);
    let phi_amp = 0.4;
    let phi_b = (phi_al * phi_al - phi_amp * phi_amp).max(0.0).sqrt();
    let deriv = if i.vs30 >= c.vlin {
        0.0
    } else {
        -c.b * sa_ref / (sa_ref + c.c) + c.b * sa_ref / (sa_ref + c.c * (i.vs30 / c.vlin).powf(c.n))
    };
    let tau = tau_al * (1.0 + deriv);
    let phi = (phi_b * phi_b * (1.0 + deriv).powi(2) + phi_amp * phi_amp).sqrt();
    LnMotion {
        mean,
        sigma: (phi * phi + tau * tau).sqrt(),
        tau,
    }
}

/// ASK14 tahmini. Model aralığı dışında `None` (sessiz ekstrapolasyon yok).
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
            let periods: Vec<f64> = t.sa.iter().map(|c| c.period.unwrap()).collect();
            super::interp_period(&periods, period, |k| compute(&t.sa[k], input))
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
            "taiwan" => CrustRegion::Taiwan,
            "china" => CrustRegion::China,
            _ => panic!("{s}"),
        }
    }

    /// pygmm 0.9.1 ile karşılaştırma (scripts/gen_gmpe_ref.py): ortalama ve
    /// standart sapma ln biriminde 0,01 içinde (≈ %1, CLAUDE.md Bölüm 14).
    #[test]
    fn pygmm_ile_ayni() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/test/ask14_ref.csv");
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
                dip: num("dip"),
                width: num("width"),
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

    #[test]
    fn aralik_disi() {
        let i = Input {
            mw: 7.0,
            rrup: 10.0,
            rjb: 10.0,
            rx: -10.0,
            ztor: 0.0,
            dip: 90.0,
            width: 15.0,
            fault: FaultType::StrikeSlip,
            vs30: 400.0,
            region: CrustRegion::Global,
        };
        assert!(predict(Imt::Pga, &i).is_some());
        assert!(predict(Imt::Pga, &Input { mw: 8.6, ..i }).is_none());
        assert!(predict(Imt::Pga, &Input { rrup: 301.0, ..i }).is_none());
        assert!(predict(Imt::Pga, &Input { vs30: 170.0, ..i }).is_none());
        assert!(predict(Imt::Pga, &Input { vs30: 1100.0, ..i }).is_none());
    }
}
