//! Türkiye Bina Deprem Yönetmeliği (TBDY-2018) yatay elastik tasarım
//! spektrumu. Kaynak: Resmî Gazete 18.03.2018, sayı 30364 (mükerrer),
//! ek 20180318M1-2-1.pdf (SHA256 8d3a9a46…; data/raw/refs/tbdy/), birincil
//! metinden okundu 2026-11-26:
//! - 2.3.2.2, Denk. (2.1): S_DS = S_S·F_S, S_D1 = S_1·F_1.
//! - 2.3.3, Tablo 2.1 (F_S) ve Tablo 2.2 (F_1); "Tablolarda harita spektral
//!   ivme katsayılarının ara değerleri için doğrusal enterpolasyon
//!   yapılabilir." Tablo uçlarının dışı uç değerde sabit (≤ ve ≥ sütunları).
//! - 2.3.4.1, Denk. (2.2)–(2.3): S_ae(T); T_A = 0,2·S_D1/S_DS,
//!   T_B = S_D1/S_DS, T_L = 6 s.
//! - 16.4, Tablo 16.1: yerel zemin sınıfları (Vs)30'a göre: ZA > 1500,
//!   ZB 760–1500, ZC 360–760, ZD 180–360, ZE < 180 m/s. Sınır değerlerde
//!   (tablo örtüşür) üst sınıf seçildi: 760 → ZB, 360 → ZC, 180 → ZD,
//!   1500 → ZB (VARSAYIM; yönetmelik açık değil). ZF (sahaya özel analiz)
//!   Vs30'dan belirlenemez; kullanıcı seçerse spektrum yok.
//!
//! S_S ve S_1 AFAD Türkiye Deprem Tehlike Haritalarından (tdth.afad.gov.tr)
//! deprem yer hareketi düzeyine (DD-1…DD-4) göre alınır; uygulamada bu
//! harita YOK, değerler kullanıcı tarafından girilir. Harita değerleri
//! referans zemin (Vs)30 = 760 m/s ve iki yatay bileşenin geometrik
//! ortalaması içindir (2.3.2.1).

/// Yerel zemin sınıfı (Tablo 16.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SiteClass {
    ZA,
    ZB,
    ZC,
    ZD,
    ZE,
    ZF,
}

impl SiteClass {
    pub const ALL: [SiteClass; 6] = [
        SiteClass::ZA,
        SiteClass::ZB,
        SiteClass::ZC,
        SiteClass::ZD,
        SiteClass::ZE,
        SiteClass::ZF,
    ];

    /// (Vs)30'dan sınıf (ZF hiçbir zaman: zemin araştırması gerekir).
    pub fn from_vs30(vs30: f64) -> SiteClass {
        if vs30 > 1500.0 {
            SiteClass::ZA
        } else if vs30 >= 760.0 {
            SiteClass::ZB
        } else if vs30 >= 360.0 {
            SiteClass::ZC
        } else if vs30 >= 180.0 {
            SiteClass::ZD
        } else {
            SiteClass::ZE
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            SiteClass::ZA => "ZA",
            SiteClass::ZB => "ZB",
            SiteClass::ZC => "ZC",
            SiteClass::ZD => "ZD",
            SiteClass::ZE => "ZE",
            SiteClass::ZF => "ZF",
        }
    }

    fn row(self) -> Option<usize> {
        match self {
            SiteClass::ZA => Some(0),
            SiteClass::ZB => Some(1),
            SiteClass::ZC => Some(2),
            SiteClass::ZD => Some(3),
            SiteClass::ZE => Some(4),
            SiteClass::ZF => None,
        }
    }
}

/// Tablo 2.1 sütunları (S_S) ve F_S değerleri (ZA…ZE).
const SS_COLS: [f64; 6] = [0.25, 0.50, 0.75, 1.00, 1.25, 1.50];
const FS: [[f64; 6]; 5] = [
    [0.8, 0.8, 0.8, 0.8, 0.8, 0.8],
    [0.9, 0.9, 0.9, 0.9, 0.9, 0.9],
    [1.3, 1.3, 1.2, 1.2, 1.2, 1.2],
    [1.6, 1.4, 1.2, 1.1, 1.0, 1.0],
    [2.4, 1.7, 1.3, 1.1, 0.9, 0.8],
];
/// Tablo 2.2 sütunları (S_1) ve F_1 değerleri (ZA…ZE).
const S1_COLS: [f64; 6] = [0.10, 0.20, 0.30, 0.40, 0.50, 0.60];
const F1: [[f64; 6]; 5] = [
    [0.8, 0.8, 0.8, 0.8, 0.8, 0.8],
    [0.8, 0.8, 0.8, 0.8, 0.8, 0.8],
    [1.5, 1.5, 1.5, 1.5, 1.5, 1.4],
    [2.4, 2.2, 2.0, 1.9, 1.8, 1.7],
    [4.2, 3.3, 2.8, 2.4, 2.2, 2.0],
];

/// Tablo sütunları arasında doğrusal ara değer; uçların dışı sabit.
fn table(cols: &[f64; 6], row: &[f64; 6], x: f64) -> f64 {
    if x <= cols[0] {
        return row[0];
    }
    for k in 1..6 {
        if x <= cols[k] {
            let f = (x - cols[k - 1]) / (cols[k] - cols[k - 1]);
            return row[k - 1] + f * (row[k] - row[k - 1]);
        }
    }
    row[5]
}

/// Yerel zemin etki katsayıları (F_S, F_1); ZF için `None`.
pub fn site_factors(class: SiteClass, ss: f64, s1: f64) -> Option<(f64, f64)> {
    let r = class.row()?;
    Some((table(&SS_COLS, &FS[r], ss), table(&S1_COLS, &F1[r], s1)))
}

/// Yatay elastik tasarım spektrumu (Denk. 2.2–2.3).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DesignSpectrum {
    pub sds: f64,
    pub sd1: f64,
    pub ta: f64,
    pub tb: f64,
    pub tl: f64,
}

/// T_L (s), 2.3.4.1.
pub const TL: f64 = 6.0;

impl DesignSpectrum {
    /// Harita katsayıları ve zemin sınıfından; ZF ya da geçersiz girdi `None`.
    pub fn new(ss: f64, s1: f64, class: SiteClass) -> Option<Self> {
        if !(ss > 0.0 && s1 > 0.0 && ss.is_finite() && s1.is_finite()) {
            return None;
        }
        let (fs, f1) = site_factors(class, ss, s1)?;
        let (sds, sd1) = (ss * fs, s1 * f1);
        Some(Self {
            sds,
            sd1,
            ta: 0.2 * sd1 / sds,
            tb: sd1 / sds,
            tl: TL,
        })
    }

    /// S_ae(T), g cinsinden.
    pub fn sae(&self, t: f64) -> f64 {
        if t <= self.ta {
            (0.4 + 0.6 * t / self.ta) * self.sds
        } else if t <= self.tb {
            self.sds
        } else if t <= self.tl {
            self.sd1 / t
        } else {
            self.sd1 * self.tl / (t * t)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zemin_sinifi() {
        assert_eq!(SiteClass::from_vs30(1600.0), SiteClass::ZA);
        assert_eq!(SiteClass::from_vs30(1500.0), SiteClass::ZB);
        assert_eq!(SiteClass::from_vs30(760.0), SiteClass::ZB);
        assert_eq!(SiteClass::from_vs30(759.0), SiteClass::ZC);
        assert_eq!(SiteClass::from_vs30(360.0), SiteClass::ZC);
        assert_eq!(SiteClass::from_vs30(250.0), SiteClass::ZD);
        assert_eq!(SiteClass::from_vs30(179.0), SiteClass::ZE);
    }

    /// Tablo değerleri ve doğrusal ara değer (elle): ZD, S_S = 0,6 → 1,4 +
    /// 0,4·(1,2 − 1,4) = 1,32; S_1 = 0,15 → 2,4 + 0,5·(2,2 − 2,4) = 2,3.
    /// ZE uçlar: S_S 0,1 → 2,4; S_S 2,0 → 0,8.
    #[test]
    fn zemin_katsayilari() {
        let (fs, f1) = site_factors(SiteClass::ZD, 0.6, 0.15).unwrap();
        assert!((fs - 1.32).abs() < 1e-12, "{fs}");
        assert!((f1 - 2.3).abs() < 1e-12, "{f1}");
        assert_eq!(site_factors(SiteClass::ZE, 0.1, 0.05).unwrap(), (2.4, 4.2));
        assert_eq!(site_factors(SiteClass::ZE, 2.0, 0.9).unwrap(), (0.8, 2.0));
        assert_eq!(site_factors(SiteClass::ZC, 1.0, 0.6).unwrap(), (1.2, 1.4));
        assert!(site_factors(SiteClass::ZF, 1.0, 0.3).is_none());
    }

    /// Spektrum (elle): S_S 1,0, S_1 0,3, ZC → S_DS 1,2, S_D1 0,45,
    /// T_A 0,075, T_B 0,375; S_ae(0) 0,48; S_ae(0,2) 1,2; S_ae(1) 0,45;
    /// S_ae(8) = 0,45·6/64 = 0,0421875.
    #[test]
    fn tasarim_spektrumu() {
        let d = DesignSpectrum::new(1.0, 0.3, SiteClass::ZC).unwrap();
        assert!((d.sds - 1.2).abs() < 1e-12 && (d.sd1 - 0.45).abs() < 1e-12);
        assert!((d.ta - 0.075).abs() < 1e-12 && (d.tb - 0.375).abs() < 1e-12);
        assert!((d.sae(0.0) - 0.48).abs() < 1e-12);
        assert!((d.sae(0.2) - 1.2).abs() < 1e-12);
        assert!((d.sae(1.0) - 0.45).abs() < 1e-12);
        assert!((d.sae(8.0) - 0.0421875).abs() < 1e-12);
        // Süreklilik: köşelerde iki taraf eşit.
        for t in [d.ta, d.tb, d.tl] {
            assert!((d.sae(t - 1e-9) - d.sae(t + 1e-9)).abs() < 1e-6, "{t}");
        }
        assert!(DesignSpectrum::new(0.0, 0.3, SiteClass::ZC).is_none());
        assert!(DesignSpectrum::new(1.0, 0.3, SiteClass::ZF).is_none());
    }
}
