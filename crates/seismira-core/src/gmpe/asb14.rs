//! Akkar, Sandıkkaya & Bommer (2014) Avrupa ve Orta Doğu sığ kabuk modeli
//! (ASB14), Rjb sürümü.
//!
//! Künye: Akkar, S., Sandıkkaya, M.A. & Bommer, J.J. (2014). Empirical
//! ground-motion models for point- and extended-source crustal earthquake
//! scenarios in Europe and the Middle East. Bulletin of Earthquake
//! Engineering 12(1), 359–387. doi:10.1007/s10518-013-9461-4
//!
//! Denklemler: Eş. (1)–(3), kabul edilmiş metin (Imperial College Spiral
//! deposu, sha256 9a2fe3a4…; `data/raw/refs/asb14/`):
//!
//! ```text
//! ln Y    = ln Y_ref(Mw, Rjb, SoF) + ln S(Vs30, PGA_ref)
//! ln Y_ref = a1 + a2·(Mw − c1) + a3·(8.5 − Mw)² + [a4 + a5·(Mw − c1)]·ln√(Rjb² + a6²)
//!            + a8·F_N + a9·F_R                                   (Mw ≤ c1)
//!          (Mw > c1 ise a2 yerine a7)
//! ln S = b1·ln(Vs30/Vref) + b2·ln[(PGA_ref + c·(Vs30/Vref)ⁿ) / ((PGA_ref + c)·(Vs30/Vref)ⁿ)]
//!                                                                 (Vs30 ≤ Vref)
//!      = b1·ln(min(Vs30, Vcon)/Vref)                              (Vs30 > Vref)
//! ```
//! Vref = 750 m/s, Vcon = 1000 m/s; PGA_ref (g), Eş. (2)'nin PGA satırıyla.
//!
//! Katsayılar: yazarların elektronik eki (62 periyot, 0,01–4 s), pygmm 0.9.1
//! veri dosyası üzerinden; makalenin Tablo 3 ve Tablo 4.a'sıyla ortak 20
//! satırda birebir aynı (`scripts/gen_asb14.py` denetler). Makalenin
//! düzeltmesi (erratum, doi:10.1007/s10518-013-9508-6) OKUNAMADI (Springer
//! giriş istiyor); ek katsayılar 2016'da yazarlardan alındığı için
//! düzeltmeden sonradır (docs/bilim.md 7.4).
//!
//! Kapsam ve seçimler:
//! - Bileşen: makale geometrik ortalama (GM_AR, kaydedildiği yönde) verir.
//!   Diğer modellerle aynı tanıma getirmek için Boore & Kishida (2017)
//!   ln(RotD50/GM_AR) oranı eklenir (ShakeMap'in "smooth" katsayıları,
//!   USGS kamu malı; PGA'da ≈ +%1, PGV'de ≈ +%2). Oranın σ'sı GMPE σ'sıyla
//!   karesel toplanır (ShakeMap `convertSigmasOnce` kuralı, component.rs).
//! - Standart sapma büyüklükten ve mesafeden bağımsızdır (Tablo 4.a).
//! - Geçerlilik (makale "Discussion and Conclusions"): Mw 4–8, odak
//!   derinliği ≤ 30 km, Rjb ≤ 200 km ("dikkatle ötesine uzatılabilir";
//!   biz uzatmıyoruz), Vs30 150–1200 m/s. SA periyodu 0,01–4 s. Sınır
//!   dışı `None`.
//! - Yönelim ve tavan bloğu terimi yoktur.

use super::{Imt, LnMotion};
use crate::quake::FaultType;
use std::sync::LazyLock;

/// Bir saha için girdi.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Input {
    pub mw: f64,
    /// Fayın yüzey izdüşümüne en kısa yatay mesafe (km).
    pub rjb: f64,
    /// Kırılmaya en kısa mesafe (km); yalnızca bileşen oranı için.
    pub rrup: f64,
    /// Odak derinliği (km).
    pub zhyp: f64,
    pub fault: FaultType,
    /// Üst 30 m ortalama kayma dalgası hızı (m/s).
    pub vs30: f64,
}

/// Modelin mesafe sınırı (km, Rjb).
pub const MAX_RJB_KM: f64 = 200.0;
/// Odak derinliği sınırı (km).
pub const MAX_ZHYP_KM: f64 = 30.0;
/// Vs30 geçerlilik aralığı (m/s), uçlar dahil.
pub const VS30_MIN: f64 = 150.0;
pub const VS30_MAX: f64 = 1200.0;

/// Modelin yayımlanmış geçerlilik aralığı içinde mi?
pub fn in_range(i: &Input) -> bool {
    (4.0..=8.0).contains(&i.mw)
        && (0.0..=MAX_RJB_KM).contains(&i.rjb)
        && i.zhyp <= MAX_ZHYP_KM
        && (VS30_MIN..=VS30_MAX).contains(&i.vs30)
}

#[derive(Clone, Copy, Debug)]
struct Coeffs {
    period: f64, // −1: PGV, 0: PGA
    a1: f64,
    a2: f64,
    a3: f64,
    a4: f64,
    a5: f64,
    a6: f64,
    a7: f64,
    a8: f64,
    a9: f64,
    c1: f64,
    vcon: f64,
    vref: f64,
    c: f64,
    n: f64,
    b1: f64,
    b2: f64,
    #[cfg_attr(not(test), allow(dead_code))] // yalnızca tablo testi
    phi: f64,
    tau: f64,
    sigma: f64,
    /// Boore & Kishida (2017) ln(RotD50/GM_AR) katsayıları.
    bk_c0: f64,
    bk_r1: f64,
    bk_m1: f64,
    bk_m2: f64,
    bk_sigma: f64,
}

struct Table {
    pga: Coeffs,
    pgv: Coeffs,
    /// Periyoda göre artan sırada.
    sa: Vec<Coeffs>,
}

static TABLE: LazyLock<Table> = LazyLock::new(|| parse(include_str!("asb14.csv")));

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
        "a1",
        "a2",
        "a3",
        "a4",
        "a5",
        "a6",
        "a7",
        "a8",
        "a9",
        "c1",
        "vcon",
        "vref",
        "c",
        "n",
        "b1",
        "b2",
        "sdwithin",
        "sdbetween",
        "sdtotal",
        "bk_c0",
        "bk_r1",
        "bk_m1",
        "bk_m2",
        "bk_sigma",
    ];
    let idx: Vec<usize> = names.iter().map(|n| col(n)).collect();
    let (mut pga, mut pgv, mut sa) = (None, None, Vec::new());
    for line in lines.filter(|l| !l.trim().is_empty()) {
        let f: Vec<&str> = line.split(',').map(str::trim).collect();
        let v: Vec<f64> = idx.iter().map(|&k| f[k].parse().expect("sayı")).collect();
        let period = match f[0] {
            "PGA" => 0.0,
            "PGV" => -1.0,
            t => t.parse().expect("periyot"),
        };
        let c = Coeffs {
            period,
            a1: v[0],
            a2: v[1],
            a3: v[2],
            a4: v[3],
            a5: v[4],
            a6: v[5],
            a7: v[6],
            a8: v[7],
            a9: v[8],
            c1: v[9],
            vcon: v[10],
            vref: v[11],
            c: v[12],
            n: v[13],
            b1: v[14],
            b2: v[15],
            phi: v[16],
            tau: v[17],
            sigma: v[18],
            bk_c0: v[19],
            bk_r1: v[20],
            bk_m1: v[21],
            bk_m2: v[22],
            bk_sigma: v[23],
        };
        match f[0] {
            "PGA" => pga = Some(c),
            "PGV" => pgv = Some(c),
            _ => sa.push(c),
        }
    }
    sa.sort_by(|a, b| a.period.partial_cmp(&b.period).expect("sayı"));
    Table {
        pga: pga.expect("PGA satırı"),
        pgv: pgv.expect("PGV satırı"),
        sa,
    }
}

/// Referans kaya (Vs30 = 750 m/s) tahmini, Eş. (2).
fn ln_ref(c: &Coeffs, i: &Input) -> f64 {
    let m = i.mw;
    let lin = if m <= c.c1 { c.a2 } else { c.a7 };
    let (f_n, f_r) = match i.fault {
        FaultType::Normal => (1.0, 0.0),
        FaultType::Reverse => (0.0, 1.0),
        FaultType::StrikeSlip => (0.0, 0.0),
    };
    c.a1 + lin * (m - c.c1)
        + c.a3 * (8.5 - m).powi(2)
        + (c.a4 + c.a5 * (m - c.c1)) * (i.rjb * i.rjb + c.a6 * c.a6).sqrt().ln()
        + c.a8 * f_n
        + c.a9 * f_r
}

/// Zemin terimi, Eş. (3). `pga_ref`: referans kaya PGA'sı (g).
fn ln_site(c: &Coeffs, vs30: f64, pga_ref: f64) -> f64 {
    if vs30 <= c.vref {
        let x = vs30 / c.vref;
        let xn = x.powf(c.n);
        c.b1 * x.ln() + c.b2 * ((pga_ref + c.c * xn) / ((pga_ref + c.c) * xn)).ln()
    } else {
        c.b1 * (vs30.min(c.vcon) / c.vref).ln()
    }
}

/// Makalenin tanımıyla (GM_AR) tahmin; bileşen dönüşümü yok.
fn compute_gmar(c: &Coeffs, i: &Input, pga_ref: f64) -> LnMotion {
    LnMotion {
        mean: ln_ref(c, i) + ln_site(c, i.vs30, pga_ref),
        sigma: c.sigma,
        tau: c.tau,
    }
}

/// ln(RotD50/GM_AR), Boore & Kishida (2017); Rrup 0,01–400 km ve M 2–9
/// aralığına çekilir (component.rs ile aynı biçim).
fn ln_rotd50_per_gmar(c: &Coeffs, mw: f64, rrup: f64) -> f64 {
    let r = rrup.clamp(1e-2, 400.0);
    let m = mw.clamp(2.0, 9.0) - 5.5;
    c.bk_c0 + c.bk_r1 * (r / 50.0).ln() + c.bk_m1 * m + c.bk_m2 * m * m
}

fn compute(c: &Coeffs, i: &Input, pga_ref: f64) -> LnMotion {
    let g = compute_gmar(c, i, pga_ref);
    LnMotion {
        mean: g.mean + ln_rotd50_per_gmar(c, i.mw, i.rrup),
        sigma: (g.sigma * g.sigma + c.bk_sigma * c.bk_sigma).sqrt(),
        tau: g.tau,
    }
}

fn pga_ref(i: &Input) -> f64 {
    ln_ref(&TABLE.pga, i).exp()
}

fn dispatch(imt: Imt, input: &Input, f: fn(&Coeffs, &Input, f64) -> LnMotion) -> Option<LnMotion> {
    if !in_range(input) {
        return None;
    }
    let t = &*TABLE;
    let pr = pga_ref(input);
    match imt {
        Imt::Pga => Some(f(&t.pga, input, pr)),
        Imt::Pgv => Some(f(&t.pgv, input, pr)),
        Imt::Sa(period) => {
            let periods: Vec<f64> = t.sa.iter().map(|c| c.period).collect();
            super::interp_period(&periods, period, |k| f(&t.sa[k], input, pr))
        }
    }
}

/// ASB14 tahmini, RotD50'ye çevrilmiş (modül notu). Model aralığı dışında
/// `None` (sessiz ekstrapolasyon yok). Tabloda olmayan SA periyodu log-log
/// ara değerle (Bölüm 7.4); 4 s ötesi `None`.
pub fn predict(imt: Imt, input: &Input) -> Option<LnMotion> {
    dispatch(imt, input, compute)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> Input {
        Input {
            mw: 6.5,
            rjb: 10.0,
            rrup: 12.0,
            zhyp: 10.0,
            fault: FaultType::StrikeSlip,
            vs30: 750.0,
        }
    }

    /// Makaledeki Tablo 3 ve Tablo 4.a değerleri (kabul edilmiş metin,
    /// s. 22) gömülü tabloyla aynı.
    #[test]
    fn makale_tablolari() {
        let t = &*TABLE;
        assert_eq!(t.sa.len(), 62);
        let p = &t.pga;
        assert_eq!(
            (p.a2, p.a5, p.a6, p.a7, p.c1, p.c, p.n),
            (0.0029, 0.2529, 7.5, -0.5096, 6.75, 2.5, 3.2)
        );
        assert_eq!((p.vref, p.vcon), (750.0, 1000.0));
        // (T, a1, a3, a4, a8, a9, b1, b2, φ, τ, σ)
        let rows = [
            (
                0.0, 1.85329, -0.02807, -1.23452, -0.1091, 0.0937, -0.41997, -0.28846, 0.6201,
                0.3501, 0.7121,
            ),
            (
                0.2, 2.73872, -0.03462, -1.28877, 0.0, 0.0493, -0.65315, -0.44644, 0.6645, 0.3842,
                0.7676,
            ),
            (
                1.0, 0.52349, -0.14345, -0.81838, 0.0, 0.0, -1.01331, -0.28702, 0.6787, 0.3943,
                0.7849,
            ),
            (
                4.0, -1.37536, -0.23848, -0.66482, 0.0, -0.2231, -0.75645, -0.07749, 0.6196,
                0.3566, 0.7149,
            ),
            (
                -1.0, 5.61201, -0.0998, -0.98388, -0.0616, 0.063, -0.72057, -0.19688, 0.6014,
                0.3311, 0.6865,
            ),
        ];
        for (tp, a1, a3, a4, a8, a9, b1, b2, phi, tau, sigma) in rows {
            let c = match tp {
                0.0 => t.pga,
                -1.0 => t.pgv,
                _ => *t.sa.iter().find(|c| c.period == tp).unwrap(),
            };
            assert_eq!(
                (c.a1, c.a3, c.a4, c.a8, c.a9, c.b1, c.b2),
                (a1, a3, a4, a8, a9, b1, b2),
                "T={tp}"
            );
            assert_eq!((c.phi, c.tau, c.sigma), (phi, tau, sigma), "T={tp}");
        }
    }

    /// Eş. (2)'nin elle hesabı: Mw 6,5 ≤ c1, Rjb 10 km, doğrultu atımlı,
    /// Vs30 = 750 (zemin terimi sıfır):
    /// ln PGA = 1,85329 + 0,0029·(−0,25) − 0,02807·4 + (−1,23452 + 0,2529·(−0,25))·ln 12,5
    #[test]
    fn elle_hesap_pga() {
        let ln12_5 = 12.5f64.ln();
        let expect =
            1.85329 + 0.0029 * -0.25 - 0.02807 * 4.0 + (-1.23452 + 0.2529 * -0.25) * ln12_5;
        let g = dispatch(Imt::Pga, &input(), compute_gmar).unwrap();
        assert!((g.mean - expect).abs() < 1e-12, "{} {expect}", g.mean);
        // ≈ 0,21 g.
        assert!((0.2..0.3).contains(&g.mean.exp()), "{}", g.mean.exp());
        // Ters fay a9 kadar yüksek, normal fay a8 kadar düşük.
        let r = dispatch(
            Imt::Pga,
            &Input {
                fault: FaultType::Reverse,
                ..input()
            },
            compute_gmar,
        )
        .unwrap();
        assert!((r.mean - g.mean - 0.0937).abs() < 1e-12);
    }

    /// Mw > c1'de a7 kolu; iki kol c1'de sürekli.
    #[test]
    fn mentese_buyuklugu_surekli() {
        let at = |mw: f64| {
            dispatch(Imt::Pga, &Input { mw, ..input() }, compute_gmar)
                .unwrap()
                .mean
        };
        assert!((at(6.75 - 1e-9) - at(6.75 + 1e-9)).abs() < 1e-6);
        // Büyük depremde büyüklükle artış yavaşlar (a7 < 0 ölçekleme
        // a3'ün azalan katkısını dengeler).
        assert!(at(7.5) - at(7.0) < at(6.5) - at(6.0));
    }

    /// Zemin terimi: Vs30 = Vref'te sıfır, Vcon üstünde sabit, yumuşak
    /// zeminde büyütme var ve güçlü sarsıntıda azalır (doğrusal olmama).
    #[test]
    fn zemin_terimi() {
        let c = &TABLE.pga;
        assert!(ln_site(c, 750.0, 0.3).abs() < 1e-12);
        assert_eq!(ln_site(c, 1000.0, 0.3), ln_site(c, 1200.0, 0.3));
        let weak = ln_site(c, 250.0, 0.05);
        let strong = ln_site(c, 250.0, 0.6);
        assert!(weak > 0.0 && strong < weak, "{weak} {strong}");
    }

    /// RotD50 dönüşümü küçük ve pozitif; σ biraz büyür.
    #[test]
    fn bilesen_donusumu() {
        for imt in [Imt::Pga, Imt::Pgv, Imt::Sa(1.0)] {
            let g = dispatch(imt, &input(), compute_gmar).unwrap();
            let d = predict(imt, &input()).unwrap();
            let r = d.mean - g.mean;
            assert!((0.0..0.05).contains(&r), "{imt:?} {r}");
            assert!(d.sigma > g.sigma && d.sigma < g.sigma + 0.01);
        }
    }

    #[test]
    fn aralik() {
        let ok = |i: Input| predict(Imt::Pga, &i).is_some();
        assert!(ok(input()));
        assert!(!ok(Input { mw: 3.9, ..input() }));
        assert!(!ok(Input { mw: 8.1, ..input() }));
        assert!(!ok(Input {
            rjb: 201.0,
            ..input()
        }));
        assert!(!ok(Input {
            zhyp: 31.0,
            ..input()
        }));
        assert!(!ok(Input {
            vs30: 1300.0,
            ..input()
        }));
        assert!(predict(Imt::Sa(4.0), &input()).is_some());
        assert!(predict(Imt::Sa(5.0), &input()).is_none());
    }

    /// pygmm 0.9.1 ile karşılaştırma (scripts/gen_gmpe_ref.py asb14):
    /// GM_AR tanımında, ln biriminde 0,01 içinde.
    #[test]
    fn pygmm_ile_ayni() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/test/asb14_ref.csv");
        let csv = std::fs::read_to_string(path).expect("referans dosyası");
        let mut lines = csv.lines();
        let header: Vec<&str> = lines.next().unwrap().split(',').collect();
        let col = |n: &str| header.iter().position(|h| *h == n).unwrap();
        let mut n = 0;
        for line in lines {
            let f: Vec<&str> = line.split(',').collect();
            let num = |n: &str| f[col(n)].parse::<f64>().unwrap();
            let input = Input {
                mw: num("mag"),
                rjb: num("rjb"),
                rrup: num("rjb"),
                zhyp: 10.0,
                fault: match f[col("mech")] {
                    "SS" => FaultType::StrikeSlip,
                    "RS" => FaultType::Reverse,
                    "NS" => FaultType::Normal,
                    s => panic!("{s}"),
                },
                vs30: num("vs30"),
            };
            for (imt, m, s) in [
                (Imt::Pga, "ln_pga", "sd_pga"),
                (Imt::Pgv, "ln_pgv", "sd_pgv"),
                (Imt::Sa(0.3), "ln_sa0.3", "sd_sa0.3"),
                (Imt::Sa(1.0), "ln_sa1.0", "sd_sa1.0"),
                (Imt::Sa(3.0), "ln_sa3.0", "sd_sa3.0"),
            ] {
                let p = dispatch(imt, &input, compute_gmar).unwrap();
                assert!(
                    (p.mean - num(m)).abs() < 0.01,
                    "{line} {imt:?}: {} {}",
                    p.mean,
                    num(m)
                );
                assert!((p.sigma - num(s)).abs() < 0.01, "{line} {imt:?}");
                n += 1;
            }
        }
        assert!(n > 100, "{n}");
    }
}
