//! Parker vd. (2020) NGA-Subduction modeli: dalma-batma arayüzü ve levha
//! içi depremler, küresel model.
//!
//! Künye: Parker, G.A., Stewart, J.P., Boore, D.M., Atkinson, G.M. &
//! Hassani, B. (2020). NGA-Subduction global ground motion models with
//! regional adjustment factors. PEER Report 2020/03, Pacific Earthquake
//! Engineering Research Center, Berkeley. Dergi sürümü: Earthquake Spectra
//! 38(1), 456-493 (2022), doi:10.1177/87552930211034889.
//!
//! Denklemler raporun numaralarıyla (Eş. 4.1–4.10, 6.x); rapor metni okundu
//! (2026-11-20). Katsayılar: USGS nshmp-lib `PSBAH20_interface.csv` ve
//! `PSBAH20_slab.csv` (CC0), bkz. dosya başlıkları. Aktarım nshmp-lib
//! `ParkerEtAl_2020.java` (kamu malı) küresel modeline göre yapıldı ve pygmm
//! 0.9.1 ile karşılaştırılarak test edilir (`data/test/psbah20_ref.csv`).
//!
//! Katsayı notu: PEER elektronik ekiyle (Tablo E1–E3) karşılaştırıldı; tek
//! fark arayüz c6'sı. Ek, PGV ve 0.2–0.75 s'de c6 > c4 veriyor; bu, raporun
//! "c6 ≤ c4" kısıtını (4.3.2: Mc üstünde ölçekleme yavaşlar) ihlal eder.
//! nshmp-lib kısıta uyan c6 = c4 değerlerini kullanır; biz de (docs/bilim.md
//! 7.4, açık konu). Test `c6_kisiti` bunu korur.
//!
//! Kapsam ve seçimler:
//! - Bileşen: RotD50.
//! - Bölge: KÜRESEL model (köşe büyüklüğü arayüzde 7.9, levha içinde 7.6;
//!   rapor Tablo 4.1). Bölgesel katsayılar (Japonya, Güney Amerika, Orta
//!   Amerika ve Meksika vb.) raporun elektronik ekinde; henüz yok.
//! - Epistemik dallar (Ek E) KULLANILMAZ: orta dal, yani modelin kendi
//!   medyanı (nshmp-lib `*_NO_EPI` karşılığı). Senaryo için en olası değer
//!   budur; dalları birleştirmek medyanı yukarı kaydırırdı.
//! - Havza terimi yok (Z2.5 bilinmiyor; Cascadia'ya özgü).
//! - Levha içi derinlik terimi (Eş. 4.6) odak derinliğiyle (Zhyp) hesaplanır;
//!   model Zhyp ile kurulmuştur (rapor 4.3.3). nshmp-lib ise Ztor + 3.12 km
//!   kullanır, çünkü nokta kaynakta Zhyp bilinmez.
//! - Geçerlilik (rapor 8.2.1): arayüz M 4.5–9.5, Rrup 20–1000 km, Zhyp ≤ 40
//!   km; levha içi M 4.5–8.5, Rrup 35–1000 km, Zhyp 0–200 km; ikisinde de
//!   Vs30 150–2000 m/s. Yalnızca yay önü (forearc) sahalar.
//!   Kısa mesafe (Rrup < 20 / 35 km) BİLİNÇLİ OLARAK hesaplanır: yakın
//!   kaynak doygunluğu (h, Eş. 4.4) bu bölge için kurulmuştur; nshmp-lib ve
//!   pygmm de hesaplar. Büyük arayüz depremlerinde kıyı şehirleri tam bu
//!   mesafededir; onları boş bırakmak yanıltıcı olur (docs/bilim.md).

use super::{Imt, LnMotion};
use std::sync::LazyLock;

/// Olay tipi.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventType {
    Interface,
    Intraslab,
}

/// Bir saha için girdi.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Input {
    pub mw: f64,
    /// Fay düzlemine en kısa mesafe (km).
    pub rrup: f64,
    /// Odak derinliği (km); yalnızca levha içinde kullanılır.
    pub zhyp: f64,
    /// Üst 30 m ortalama kayma dalgası hızı (m/s).
    pub vs30: f64,
    pub event: EventType,
}

/// Modelin mesafe sınırı (km, Rrup), rapor 8.2.1.
pub const MAX_RRUP_KM: f64 = 1000.0;
/// Vs30 geçerlilik aralığı (m/s), rapor 8.2.1 (nshmp-lib 1500'de keser).
pub const VS30_MIN: f64 = 150.0;
pub const VS30_MAX: f64 = 2000.0;
/// Büyüklük aralığı, rapor 8.2.1.
pub const INTERFACE_MW: (f64, f64) = (4.5, 9.5);
pub const INTRASLAB_MW: (f64, f64) = (4.5, 8.5);
/// Odak derinliği sınırı (km), rapor 8.2.1.
pub const INTERFACE_ZHYP_MAX: f64 = 40.0;
pub const INTRASLAB_ZHYP_MAX: f64 = 200.0;

/// Modelin yayımlanmış geçerlilik aralığı içinde mi? (Kısa mesafe için
/// modül notuna bakın.)
pub fn in_range(i: &Input) -> bool {
    let (mw, zmax) = match i.event {
        EventType::Interface => (INTERFACE_MW, INTERFACE_ZHYP_MAX),
        EventType::Intraslab => (INTRASLAB_MW, INTRASLAB_ZHYP_MAX),
    };
    (mw.0..=mw.1).contains(&i.mw)
        && (0.0..=MAX_RRUP_KM).contains(&i.rrup)
        && (0.0..=zmax).contains(&i.zhyp)
        && (VS30_MIN..=VS30_MAX).contains(&i.vs30)
}

// Tüm periyotlar için sabitler (rapor Bölüm 4.1).
const B4: f64 = 0.1;
const V1: f64 = 270.0;
const VREF: f64 = 760.0;
const F3: f64 = 0.05;
const DB1: f64 = 20.0;
const DB2: f64 = 67.0;
/// Küresel köşe büyüklükleri, Tablo 4.1 (arayüz: Campbell 2020; levha
/// içi: Ji & Archuleta 2018).
const MC_INTERFACE: f64 = 7.9;
const MC_SLAB: f64 = 7.6;
// φ modeli (Eş. 6.x): mesafe ve Vs30 köşeleri.
const PHI_R1: f64 = 200.0;
const PHI_R2: f64 = 500.0;
const PHI_V1: f64 = 200.0;
const PHI_V2: f64 = 500.0;

#[derive(Clone, Debug)]
struct Coeffs {
    period: Option<f64>, // None: PGA veya PGV satırı
    c0: f64,
    c1: f64,
    a0: f64,
    c4: f64,
    c5: f64,
    c6: f64,
    /// Levha içi derinlik terimi (arayüzde 0).
    d: f64,
    m: f64,
    v2: f64,
    s2: f64,
    f4: f64,
    f5: f64,
    tau: f64,
    phi21: f64,
    phi22: f64,
    phi2v: f64,
}

struct Table {
    pga: Coeffs,
    pgv: Coeffs,
    /// Periyoda göre artan sırada.
    sa: Vec<Coeffs>,
}

static INTERFACE: LazyLock<Table> =
    LazyLock::new(|| parse(include_str!("psbah20_interface.csv"), false));
static SLAB: LazyLock<Table> = LazyLock::new(|| parse(include_str!("psbah20_slab.csv"), true));

fn parse(csv: &str, slab: bool) -> Table {
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
    let mut names = vec![
        "Global_c0",
        "c1",
        "Global_a0",
        "c4",
        "c5",
        "c6",
        "V2",
        "Global_s2",
        "f4",
        "f5",
        "Tau",
        "phi21",
        "phi22",
        "phi2V",
    ];
    if slab {
        names.extend(["d", "m"]);
    }
    let idx: Vec<usize> = names.iter().map(|n| col(n)).collect();
    let (mut pga, mut pgv, mut sa) = (None, None, Vec::new());
    for line in lines.filter(|l| !l.trim().is_empty()) {
        let f: Vec<&str> = line.split(',').map(str::trim).collect();
        let v = |k: usize| -> f64 { f[idx[k]].parse().expect("sayı") };
        let mut c = Coeffs {
            period: None,
            c0: v(0),
            c1: v(1),
            a0: v(2),
            c4: v(3),
            c5: v(4),
            c6: v(5),
            v2: v(6),
            s2: v(7),
            f4: v(8),
            f5: v(9),
            tau: v(10),
            phi21: v(11),
            phi22: v(12),
            phi2v: v(13),
            d: if slab { v(14) } else { 0.0 },
            m: if slab { v(15) } else { 0.0 },
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

/// Referans zemin (Vs30 = 760 m/s) için ln ortalama: Eş. 4.1, zemin terimi
/// olmadan (kaynak + yol + derinlik).
fn ln_reference(c: &Coeffs, i: &Input) -> f64 {
    let m = i.mw;
    let slab = i.event == EventType::Intraslab;
    let mc = if slab { MC_SLAB } else { MC_INTERFACE };
    // Eş. 4.4a / 4.4b: yakın kaynak doygunluğu.
    let h = if !slab {
        10f64.powf(-0.82 + 0.252 * m)
    } else if m <= mc {
        10f64.powf(1.050 / (mc - 4.0) * (m - mc) + 1.544)
    } else {
        35.0
    };
    // Eş. 4.2 ve 4.3: yol terimi.
    let r = i.rrup.hypot(h);
    let r_ref = 1f64.hypot(h);
    let fp = c.c1 * r.ln() + B4 * m * (r / r_ref).ln() + c.a0 * r;
    // Eş. 4.5: büyüklük terimi.
    let dm = m - mc;
    let fm = if m <= mc {
        c.c4 * dm + c.c5 * dm * dm
    } else {
        c.c6 * dm
    };
    // Eş. 4.6: kaynak derinliği (yalnızca levha içi; arayüzde d = m = 0).
    let fd = if !slab {
        0.0
    } else if i.zhyp < DB1 {
        c.m * (DB1 - DB2) + c.d
    } else if i.zhyp > DB2 {
        c.d
    } else {
        c.m * (i.zhyp - DB2) + c.d
    };
    c.c0 + fp + fm + fd
}

/// Tek bir katsayı satırıyla hesap. `pga_ref`: Vs30 = 760 m/s'de PGA (g).
fn compute(c: &Coeffs, i: &Input, pga_ref: f64) -> LnMotion {
    // Eş. 4.8: doğrusal zemin terimi (küresel modelde s1 = s2).
    let s1 = c.s2;
    let flin = if i.vs30 <= V1 {
        s1 * (i.vs30 / V1).ln() + c.s2 * (V1 / VREF).ln()
    } else if i.vs30 > c.v2 {
        c.s2 * (c.v2 / VREF).ln()
    } else {
        c.s2 * (i.vs30 / VREF).ln()
    };
    // Eş. 4.9 ve 4.10: doğrusal olmayan zemin terimi (f1 = 0).
    let f2 = c.f4 * ((c.f5 * (i.vs30.min(VREF) - 200.0)).exp() - (c.f5 * (VREF - 200.0)).exp());
    let fnl = f2 * ((pga_ref + F3) / F3).ln();
    let mean = ln_reference(c, i) + flin + fnl;

    // Belirsizlik (Bölüm 6): φ² mesafe ve Vs30'a bağlı; σ = √(τ² + φ²).
    let ln_r = (PHI_R2 / PHI_R1).ln();
    let r_prime = i.rrup.clamp(PHI_R1, PHI_R2);
    let var_r = (PHI_R2 / r_prime).ln() / ln_r;
    let dvar = if i.vs30 <= PHI_V1 {
        c.phi2v * var_r
    } else if i.vs30 >= PHI_V2 {
        0.0
    } else {
        c.phi2v * ((PHI_V2 / i.vs30).ln() / (PHI_V2 / PHI_V1).ln()) * var_r
    };
    let phi2 = if i.rrup <= PHI_R1 {
        c.phi21
    } else if i.rrup >= PHI_R2 {
        c.phi22
    } else {
        (c.phi22 - c.phi21) / ln_r * (i.rrup / PHI_R1).ln() + c.phi21
    };
    let phi_sq = phi2 + dvar;
    LnMotion {
        mean,
        sigma: (c.tau * c.tau + phi_sq).sqrt(),
        tau: c.tau,
    }
}

/// Parker vd. (2020) tahmini. Model aralığı dışında `None` (sessiz
/// ekstrapolasyon yok). Tabloda olmayan SA periyodu log-log ara değerle.
pub fn predict(imt: Imt, input: &Input) -> Option<LnMotion> {
    if !in_range(input) {
        return None;
    }
    let t: &Table = match input.event {
        EventType::Interface => &INTERFACE,
        EventType::Intraslab => &SLAB,
    };
    let pga_ref = ln_reference(&t.pga, input).exp();
    match imt {
        Imt::Pga => Some(compute(&t.pga, input, pga_ref)),
        Imt::Pgv => Some(compute(&t.pgv, input, pga_ref)),
        Imt::Sa(period) => {
            let periods: Vec<f64> = t.sa.iter().map(|c| c.period.expect("SA")).collect();
            super::interp_period(&periods, period, |k| compute(&t.sa[k], input, pga_ref))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Bölüm 14: referans uygulamayla fark %1'den az. Medyan için ln farkı
    /// < ln(1.01); sigma için göreli fark < %1.
    #[test]
    fn pygmm_referansiyla_yuzde_birden_az_fark() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/test/psbah20_ref.csv"
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
                rrup: num("rrup"),
                zhyp: num("zhyp"),
                vs30: num("vs30"),
                event: match f[col("event")] {
                    "interface" => EventType::Interface,
                    "intraslab" => EventType::Intraslab,
                    e => panic!("{e}"),
                },
            };
            let mut check = |imt: Imt, mean: &str, sd: &str| {
                let p = predict(imt, &input).unwrap_or_else(|| panic!("{input:?}"));
                worst_mean = worst_mean.max((p.mean - num(mean)).abs());
                worst_sd = worst_sd.max((p.sigma / num(sd) - 1.0).abs());
                n += 1;
            };
            check(Imt::Pga, "ln_pga", "sd_pga");
            check(Imt::Pgv, "ln_pgv", "sd_pgv");
            for t in ["0.3", "0.5", "0.6", "0.75", "1.0"] {
                check(
                    Imt::Sa(t.parse().unwrap()),
                    &format!("ln_sa{t}"),
                    &format!("sd_sa{t}"),
                );
            }
        }
        eprintln!(
            "{n} karşılaştırma: ln farkı ≤ {worst_mean:.2e}, σ göreli farkı ≤ {worst_sd:.2e}"
        );
        assert!(n > 1000, "{n} karşılaştırma");
        assert!(worst_mean < 1.01f64.ln(), "en büyük ln farkı {worst_mean}");
        assert!(worst_sd < 0.01, "en büyük σ göreli farkı {worst_sd}");
    }

    /// Rapor 4.3.2: büyüklük eğimi Mc üstünde yavaşlar (c6 ≤ c4), her iki
    /// olay tipinde ve her ölçüde.
    #[test]
    fn c6_kisiti() {
        for t in [&*INTERFACE, &*SLAB] {
            for c in [&t.pga, &t.pgv].into_iter().chain(&t.sa) {
                assert!(c.c6 <= c.c4, "{:?}: c6 {} > c4 {}", c.period, c.c6, c.c4);
            }
        }
    }

    fn input(event: EventType, mw: f64, rrup: f64) -> Input {
        Input {
            mw,
            rrup,
            zhyp: 50.0,
            vs30: 760.0,
            event,
        }
    }

    #[test]
    fn aralik_disi_none() {
        let mut i = input(EventType::Intraslab, 8.6, 100.0);
        assert!(predict(Imt::Pgv, &i).is_none(), "levha içi M > 8.5");
        i.event = EventType::Interface;
        assert!(predict(Imt::Pgv, &i).is_none(), "arayüz Zhyp > 40 km");
        i.zhyp = 30.0;
        assert!(predict(Imt::Pgv, &i).is_some());
        i.rrup = 1001.0;
        assert!(predict(Imt::Pgv, &i).is_none(), "Rrup > 1000 km");
        i.rrup = 100.0;
        i.vs30 = 140.0;
        assert!(predict(Imt::Pgv, &i).is_none(), "Vs30 < 150");
    }

    /// Fiziksel tutarlılık: mesafeyle azalır, büyüklükle artar, yumuşak
    /// zemin (Vs30 küçük) zayıf sarsıntıda büyütür.
    #[test]
    fn monoton_davranis() {
        for ev in [EventType::Interface, EventType::Intraslab] {
            let mut i = input(ev, 7.0, 30.0);
            i.zhyp = 30.0;
            let at = |i: &Input| predict(Imt::Pgv, i).unwrap().mean;
            let near = at(&i);
            i.rrup = 300.0;
            let far = at(&i);
            assert!(near > far);
            i.mw = 8.0;
            assert!(at(&i) > far);
            let rock = at(&i);
            i.vs30 = 250.0;
            assert!(at(&i) > rock);
        }
    }
}
