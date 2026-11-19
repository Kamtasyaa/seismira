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
//! - Bölge ([`Region`]): küresel model ya da raporun bölgesel uyarlamaları
//!   (c0, a0, s1, s2: elektronik ek Tablo E1/E2, `psbah20_regional.csv`,
//!   `scripts/gen_psbah20_regional.py`; köşe büyüklüğü Mc: Tablo 4.1).
//!   Bölgeye göre değişmeyen katsayılar nshmp-lib dosyalarından.
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

/// Bölgesel uyarlama (rapor 4.3.4, 5.2.2, Tablo 4.1). NGA-Sub bölgeleri;
/// listede olmayan dalma-batma zonları (ör. Yeni Zelanda, Akdeniz) küresel
/// modeli kullanır.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Region {
    Global,
    Alaska,
    /// Aleut Adaları: c0 ve Mc ayrı; a0 ve s2 Alaska bölgesinden.
    Aleutian,
    Cascadia,
    /// Orta Amerika ve Meksika, kuzey / güney.
    CamN,
    CamS,
    /// Japonya, Kuril-Kamçatka ve Japonya hendekleri (Pasifik levhası).
    JapanPac,
    /// Japonya, Nankai-Ryukyu (Filipin Denizi levhası).
    JapanPhi,
    /// Güney Amerika, kuzey / güney.
    SaN,
    SaS,
    /// Tayvan, doğu / batı.
    TaiwanE,
    TaiwanW,
}

impl Region {
    pub const ALL: [Region; 12] = [
        Region::Global,
        Region::Alaska,
        Region::Aleutian,
        Region::Cascadia,
        Region::CamN,
        Region::CamS,
        Region::JapanPac,
        Region::JapanPhi,
        Region::SaN,
        Region::SaS,
        Region::TaiwanE,
        Region::TaiwanW,
    ];

    /// `psbah20_regional.csv` içindeki ad (çeviri anahtarı için de).
    pub fn key(self) -> &'static str {
        match self {
            Region::Global => "global",
            Region::Alaska => "alaska",
            Region::Aleutian => "aleutian",
            Region::Cascadia => "cascadia",
            Region::CamN => "cam_n",
            Region::CamS => "cam_s",
            Region::JapanPac => "japan_pac",
            Region::JapanPhi => "japan_phi",
            Region::SaN => "sa_n",
            Region::SaS => "sa_s",
            Region::TaiwanE => "taiwan_e",
            Region::TaiwanW => "taiwan_w",
        }
    }

    /// Köşe (doygunluk) büyüklüğü Mc, rapor Tablo 4.1: arayüz Campbell
    /// (2020), levha içi Ji & Archuleta (2018). Orta Amerika arayüzünde iki
    /// alt bölge için ortak 7.4 (tablo dipnotu 1).
    pub fn mc(self, event: EventType) -> f64 {
        let (interface, slab) = match self {
            Region::Global => (7.9, 7.6),
            Region::Alaska => (8.6, 7.20),
            Region::Aleutian => (8.0, 7.98),
            Region::Cascadia => (7.7, 7.20),
            Region::CamN => (7.4, 7.40),
            Region::CamS => (7.4, 7.60),
            Region::JapanPac => (8.5, 7.65),
            Region::JapanPhi => (7.7, 7.55),
            Region::SaN => (8.5, 7.30),
            Region::SaS => (8.6, 7.25),
            Region::TaiwanE | Region::TaiwanW => (7.1, 7.70),
        };
        match event {
            EventType::Interface => interface,
            EventType::Intraslab => slab,
        }
    }
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
    pub region: Region,
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
// φ modeli (Eş. 6.x): mesafe ve Vs30 köşeleri.
const PHI_R1: f64 = 200.0;
const PHI_R2: f64 = 500.0;
const PHI_V1: f64 = 200.0;
const PHI_V2: f64 = 500.0;

#[derive(Clone, Debug)]
struct Coeffs {
    period: Option<f64>, // None: PGA veya PGV satırı
    /// Köşe büyüklüğü (bölgeye göre, Tablo 4.1).
    mc: f64,
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
    /// Doğrusal zemin eğimleri (Eş. 4.8); s1 yalnızca Japonya ve Tayvan'da
    /// s2'den farklı.
    s1: f64,
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

/// Her (olay tipi, bölge) için tablo: önce olay tipi, sonra `Region::ALL`
/// sırası.
static TABLES: LazyLock<Vec<Table>> = LazyLock::new(|| {
    let regional = include_str!("psbah20_regional.csv");
    [EventType::Interface, EventType::Intraslab]
        .into_iter()
        .flat_map(|event| {
            let csv = match event {
                EventType::Interface => include_str!("psbah20_interface.csv"),
                EventType::Intraslab => include_str!("psbah20_slab.csv"),
            };
            Region::ALL.map(|region| {
                let mut t = parse(csv, event == EventType::Intraslab);
                regionalize(&mut t, regional, event, region);
                t
            })
        })
        .collect()
});

fn table(event: EventType, region: Region) -> &'static Table {
    let e = match event {
        EventType::Interface => 0,
        EventType::Intraslab => 1,
    };
    let r = Region::ALL
        .iter()
        .position(|&x| x == region)
        .expect("bölge");
    &TABLES[e * Region::ALL.len() + r]
}

/// Bölgesel c0, a0, s1, s2 ve Mc'yi tabloya yazar. Her periyot için satır
/// bulunmazsa panikler (test `bolgesel_tablolar_tam` korur).
fn regionalize(t: &mut Table, csv: &str, event: EventType, region: Region) {
    let ev = match event {
        EventType::Interface => "interface",
        EventType::Intraslab => "intraslab",
    };
    let mut lines = csv.lines().filter(|l| !l.trim_start().starts_with('#'));
    let header: Vec<&str> = lines.next().expect("başlık").split(',').collect();
    assert_eq!(header, ["event", "region", "T", "c0", "a0", "s1", "s2"]);
    let mut found = 0;
    for line in lines {
        let f: Vec<&str> = line.split(',').map(str::trim).collect();
        if f[0] != ev || f[1] != region.key() {
            continue;
        }
        let num = |k: usize| -> f64 { f[k].parse().expect("sayı") };
        let c = match f[2] {
            "PGA" => &mut t.pga,
            "PGV" => &mut t.pgv,
            p => {
                let p: f64 = p.parse().expect("periyot");
                t.sa.iter_mut()
                    .find(|c| c.period.is_some_and(|x| (x - p).abs() < 1e-9))
                    .unwrap_or_else(|| panic!("{ev} {region:?}: {p} s tabloda yok"))
            }
        };
        (c.c0, c.a0, c.s1, c.s2) = (num(3), num(4), num(5), num(6));
        found += 1;
    }
    assert_eq!(found, t.sa.len() + 2, "{ev} {region:?}: eksik satır");
    let mc = region.mc(event);
    for c in [&mut t.pga, &mut t.pgv].into_iter().chain(t.sa.iter_mut()) {
        c.mc = mc;
    }
}

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
            mc: f64::NAN, // regionalize() yazar
            c0: v(0),
            c1: v(1),
            a0: v(2),
            c4: v(3),
            c5: v(4),
            c6: v(5),
            v2: v(6),
            s1: v(7),
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
    let mc = c.mc;
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
    // Eş. 4.8: doğrusal zemin terimi.
    let flin = if i.vs30 <= V1 {
        c.s1 * (i.vs30 / V1).ln() + c.s2 * (V1 / VREF).ln()
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
    let t = table(input.event, input.region);
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
                region: match f[col("region")] {
                    "global" => Region::Global,
                    "alaska" => Region::Alaska,
                    "cascadia" => Region::Cascadia,
                    r => panic!("{r}"),
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
        for t in TABLES.iter() {
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
            region: Region::Global,
        }
    }

    /// Bölgesel model ↔ OpenQuake (yalnızca çıktı karşılaştırması; kod
    /// kopyalanmadı). Arayüzde M ≤ Mc (c6 farkı, modül notu). Ölçüt %1.
    #[test]
    fn openquake_bolgesel_referansla_yuzde_birden_az_fark() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/test/psbah20_regional_ref.csv"
        );
        let csv = std::fs::read_to_string(path).expect("referans dosyası");
        let mut lines = csv.lines();
        let header: Vec<&str> = lines.next().unwrap().split(',').collect();
        let col = |n: &str| header.iter().position(|h| *h == n).unwrap();
        let (mut worst_mean, mut worst_sd, mut n) = (0f64, 0f64, 0);
        let mut seen = std::collections::HashSet::new();
        for line in lines {
            let f: Vec<&str> = line.split(',').collect();
            let num = |n: &str| f[col(n)].parse::<f64>().unwrap();
            let region = *Region::ALL
                .iter()
                .find(|r| r.key() == f[col("region")])
                .unwrap();
            seen.insert(region);
            let input = Input {
                mw: num("mag"),
                rrup: num("rrup"),
                zhyp: num("zhyp"),
                vs30: num("vs30"),
                event: match f[col("event")] {
                    "interface" => EventType::Interface,
                    _ => EventType::Intraslab,
                },
                region,
            };
            for (imt, c) in [
                (Imt::Pga, "pga"),
                (Imt::Pgv, "pgv"),
                (Imt::Sa(0.3), "sa0.3"),
                (Imt::Sa(1.0), "sa1.0"),
            ] {
                let p = predict(imt, &input).unwrap_or_else(|| panic!("{input:?}"));
                let dm = (p.mean - num(&format!("ln_{c}"))).abs();
                assert!(dm < 1.01f64.ln(), "{input:?} {c}: ln farkı {dm}");
                worst_mean = worst_mean.max(dm);
                worst_sd = worst_sd.max((p.sigma / num(&format!("sd_{c}")) - 1.0).abs());
                n += 1;
            }
        }
        eprintln!(
            "OpenQuake: {n} karşılaştırma, ln farkı ≤ {worst_mean:.2e}, σ farkı ≤ {worst_sd:.2e}"
        );
        assert_eq!(seen.len(), Region::ALL.len());
        assert!(worst_sd < 0.01, "en büyük σ göreli farkı {worst_sd}");
    }

    /// Her (olay tipi, bölge) tablosu kurulabiliyor: bölgesel dosyada her
    /// periyot için satır var (yoksa `regionalize` panikler) ve Mc yazılmış.
    #[test]
    fn bolgesel_tablolar_tam() {
        assert_eq!(TABLES.len(), 2 * Region::ALL.len());
        for t in TABLES.iter() {
            for c in [&t.pga, &t.pgv].into_iter().chain(&t.sa) {
                assert!(c.mc.is_finite() && c.c0.is_finite() && c.s1.is_finite());
            }
        }
        // Rapor 5.2.2: s1 yalnızca Japonya ve Tayvan'da s2'den farklı.
        for ev in [EventType::Interface, EventType::Intraslab] {
            for r in Region::ALL {
                let t = table(ev, r);
                let differs = [&t.pga, &t.pgv]
                    .into_iter()
                    .chain(&t.sa)
                    .any(|c| c.s1 != c.s2);
                let expect = matches!(
                    r,
                    Region::JapanPac | Region::JapanPhi | Region::TaiwanE | Region::TaiwanW
                );
                assert_eq!(differs, expect, "{ev:?} {r:?}");
            }
        }
    }

    /// Ekteki örnek değerler (Tablo E1, PGV satırı) ve Tablo 4.1 doğru
    /// okunuyor: Japonya-Pasifik arayüzü.
    #[test]
    fn japonya_pasifik_katsayilari() {
        let t = table(EventType::Interface, Region::JapanPac);
        assert_eq!(t.pgv.c0, 8.772125851);
        assert_eq!(t.pgv.a0, -2.39e-3);
        assert_eq!((t.pgv.s1, t.pgv.s2), (-0.738, -0.738));
        assert_eq!(t.pgv.mc, 8.5);
        // Bölgeye göre değişmeyenler küresel tablodan.
        let g = table(EventType::Interface, Region::Global);
        assert_eq!(
            (t.pgv.c1, t.pgv.c4, t.pgv.c6),
            (g.pgv.c1, g.pgv.c4, g.pgv.c6)
        );
        // Levha içinde Japonya tek sabitli; Mc alt bölgeye göre.
        let p = table(EventType::Intraslab, Region::JapanPac);
        let f = table(EventType::Intraslab, Region::JapanPhi);
        assert_eq!(p.pgv.c0, f.pgv.c0);
        assert_eq!((p.pgv.mc, f.pgv.mc), (7.65, 7.55));
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
