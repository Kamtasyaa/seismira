//! Şiddet dönüşümü: PGV → JMA aletsel şiddeti ve Shindo sınıfı
//! (CLAUDE.md Bölüm 7.6).
//!
//! Bağıntı: Midorikawa, S., Fujimoto, K. & Muramatsu, I. (1999).
//! Correlation of new J.M.A. instrumental seismic intensity with former
//! J.M.A. seismic intensity and ground motion parameters. Journal of Social
//! Safety Science 1, 51-56. Tablo 2, |v(t)|max satırı, I = 0~7 için:
//! I = 2.54 + 1.82·log10(PGV) ± 0.19 (PGV cm/s). (I = 4~7 alt kümesi için
//! 2.68 + 1.72·log10 PGV ± 0.21; JMA erken uyarısı bunu kullanır. Biz tüm
//! aralığı kapsayan 0~7 uydurmasını kullanıyoruz.) Makalede PGV iki yatay
//! bileşenin VEKTÖREL bileşkesinin tepe değeridir (= RotD100).
//!
//! Sınıf eşikleri ve yuvarlama: JMA "計測震度の算出方法" sayfası, Tablo 1
//! (2026-11-14'te kontrol edildi).

use std::fmt;

/// Midorikawa vd. (1999) Tablo 2, PGV, I = 0~7.
const A: f64 = 2.54;
const B: f64 = 1.82;
/// Bağıntının standart sapması (şiddet birimi).
pub const SIGMA_CONVERSION: f64 = 0.19;

/// RotD100 / RotD50 oranı (PGV). VARSAYIM, DOĞRULANMADI: PGV için
/// yayımlanmış değere erişilemedi (Boore & Kishida 2017). Shahi & Baker
/// (2014, Earthquake Spectra 30(3), Tablo 1) spektral ivme için 0.5–1 s
/// aralığında 1.23–1.24 verir; PGV'nin baskın periyodu bu aralıkta kabul
/// edildi. Etkisi ≈ +0.16 şiddet birimi. docs/bilim.md'de açık not.
pub const ROTD100_PER_ROTD50_PGV: f64 = 1.23;

/// RotD50 PGV (cm/s) → JMA aletsel şiddeti (ham, yuvarlanmamış).
pub fn jma_from_pgv_rotd50(pgv_cm_s: f64) -> f64 {
    let pgv_vector = pgv_cm_s * ROTD100_PER_ROTD50_PGV;
    A + B * pgv_vector.log10()
}

/// GMPE'nin ln(PGV) standart sapmasını şiddet birimine taşır ve dönüşüm
/// belirsizliğiyle birleştirir (bağımsız kabul edilerek).
pub fn jma_sigma(sigma_ln_pgv: f64) -> f64 {
    let from_gmpe = B * sigma_ln_pgv / std::f64::consts::LN_10;
    (from_gmpe * from_gmpe + SIGMA_CONVERSION * SIGMA_CONVERSION).sqrt()
}

/// JMA kuralı: "小数第３位を四捨五入し、小数第２位を切り捨て" (üçüncü ondalık
/// yuvarlanır, ikinci ondalık atılır) → 0.1 adımlı aletsel şiddet.
pub fn round_jma(i: f64) -> f64 {
    let hundredths = (i * 100.0).round();
    (hundredths / 10.0).floor() / 10.0
}

/// JMA Shindo sınıfı.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Shindo {
    S0,
    S1,
    S2,
    S3,
    S4,
    S5Lower,
    S5Upper,
    S6Lower,
    S6Upper,
    S7,
}

impl Shindo {
    pub const ALL: [Shindo; 10] = [
        Shindo::S0,
        Shindo::S1,
        Shindo::S2,
        Shindo::S3,
        Shindo::S4,
        Shindo::S5Lower,
        Shindo::S5Upper,
        Shindo::S6Lower,
        Shindo::S6Upper,
        Shindo::S7,
    ];

    /// Aletsel şiddetten (JMA yuvarlaması uygulanarak) sınıf.
    pub fn from_instrumental(i: f64) -> Shindo {
        let i = round_jma(i);
        // Alt sınır dahil, üst sınır hariç (JMA Tablo 1).
        match i {
            x if x < 0.5 => Shindo::S0,
            x if x < 1.5 => Shindo::S1,
            x if x < 2.5 => Shindo::S2,
            x if x < 3.5 => Shindo::S3,
            x if x < 4.5 => Shindo::S4,
            x if x < 5.0 => Shindo::S5Lower,
            x if x < 5.5 => Shindo::S5Upper,
            x if x < 6.0 => Shindo::S6Lower,
            x if x < 6.5 => Shindo::S6Upper,
            _ => Shindo::S7,
        }
    }

    /// Kutuda gösterilen etiket (rakam; renk tek bilgi taşıyıcı değil).
    pub fn label(self) -> &'static str {
        match self {
            Shindo::S0 => "0",
            Shindo::S1 => "1",
            Shindo::S2 => "2",
            Shindo::S3 => "3",
            Shindo::S4 => "4",
            Shindo::S5Lower => "5-",
            Shindo::S5Upper => "5+",
            Shindo::S6Lower => "6-",
            Shindo::S6Upper => "6+",
            Shindo::S7 => "7",
        }
    }
}

impl fmt::Display for Shindo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jma_sinir_degerleri() {
        // JMA Tablo 1: alt sınır dahil.
        assert_eq!(Shindo::from_instrumental(0.49), Shindo::S0);
        assert_eq!(Shindo::from_instrumental(0.5), Shindo::S1);
        assert_eq!(Shindo::from_instrumental(4.5), Shindo::S5Lower);
        assert_eq!(Shindo::from_instrumental(4.99), Shindo::S5Lower);
        assert_eq!(Shindo::from_instrumental(5.0), Shindo::S5Upper);
        assert_eq!(Shindo::from_instrumental(6.49), Shindo::S6Upper);
        assert_eq!(Shindo::from_instrumental(6.5), Shindo::S7);
        assert_eq!(Shindo::from_instrumental(-1.0), Shindo::S0);
    }

    #[test]
    fn jma_yuvarlama_kurali() {
        // 4.496 → üçüncü ondalık yuvarlanır (4.50) → ikinci atılır (4.5) → 5-.
        assert_eq!(round_jma(4.496), 4.5);
        assert_eq!(Shindo::from_instrumental(4.496), Shindo::S5Lower);
        // 4.494 → 4.49 → 4.4 → 4.
        assert_eq!(round_jma(4.494), 4.4);
        assert_eq!(Shindo::from_instrumental(4.494), Shindo::S4);
    }

    #[test]
    fn midorikawa_tablo2_degerleri() {
        // Vektörel PGV 10 cm/s → I = 2.54 + 1.82 = 4.36.
        let pgv50 = 10.0 / ROTD100_PER_ROTD50_PGV;
        assert!((jma_from_pgv_rotd50(pgv50) - 4.36).abs() < 1e-12);
        // Vektörel PGV 1 cm/s → I = 2.54.
        assert!((jma_from_pgv_rotd50(1.0 / ROTD100_PER_ROTD50_PGV) - 2.54).abs() < 1e-12);
    }

    #[test]
    fn belirsizlik_birlesimi() {
        // σ_lnPGV = 0 iken yalnızca dönüşüm belirsizliği kalır.
        assert!((jma_sigma(0.0) - SIGMA_CONVERSION).abs() < 1e-12);
        // σ_lnPGV = 0.6 → 1.82·0.6/ln10 ≈ 0.474; toplam ≈ 0.511.
        assert!((jma_sigma(0.6) - 0.5108).abs() < 1e-3);
    }
}
