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
//! bileşenin VEKTÖREL bileşkesinin tepe değeridir (= RotD100); GMPE'nin
//! RotD50'sinden dönüşüm [`crate::component`] (Boore & Kishida 2017).
//!
//! Sınıf eşikleri ve yuvarlama: JMA "計測震度の算出方法" sayfası, Tablo 1
//! (2026-11-14'te kontrol edildi).

use std::fmt;

/// Midorikawa vd. (1999) Tablo 2, PGV, I = 0~7.
const A: f64 = 2.54;
const B: f64 = 1.82;
/// Bağıntının standart sapması (şiddet birimi).
pub const SIGMA_CONVERSION: f64 = 0.19;

/// Vektörel bileşke (RotD100) PGV (cm/s) → JMA aletsel şiddeti (ham,
/// yuvarlanmamış): makaledeki bağıntının kendisi.
pub fn jma_from_pgv_vector(pgv_cm_s: f64) -> f64 {
    A + B * pgv_cm_s.log10()
}

/// RotD50 PGV (cm/s) → JMA aletsel şiddeti (ham, yuvarlanmamış). GMPE'nin
/// RotD50'si önce vektörel bileşkeye (RotD100) çevrilir: Boore & Kishida
/// (2017), büyüklük ve mesafeye bağlı oran ([`crate::component`]). PGV için
/// Mw 4–8,5 ve Rrup 1–300 km'de 1,19–1,29; faya çok yakında (Rrup → 0)
/// 1,37'ye çıkar. 2026-11-15'e kadar sabit 1,23 varsayımı kullanılıyordu:
/// fark −0,03 ile +0,04 şiddet birimi arasında, fay üzerinde +0,08.
pub fn jma_from_pgv_rotd50(pgv_cm_s: f64, mw: f64, rrup_km: f64) -> f64 {
    let ratio = crate::component::ln_rotd100_per_rotd50_pgv(mw, rrup_km).exp();
    jma_from_pgv_vector(pgv_cm_s * ratio)
}

/// GMPE'nin ln(PGV) standart sapmasını şiddet birimine taşır: bileşen
/// dönüşümünün saçılımı eklenir ve bağıntının belirsizliğiyle birleştirilir
/// (bağımsız kabul edilerek).
pub fn jma_sigma(sigma_ln_pgv: f64) -> f64 {
    let s_ln = (sigma_ln_pgv.powi(2) + crate::component::SIGMA_ROTD100_PGV.powi(2)).sqrt();
    let from_gmpe = B * s_ln / std::f64::consts::LN_10;
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
        assert!((jma_from_pgv_vector(10.0) - 4.36).abs() < 1e-12);
        // Vektörel PGV 1 cm/s → I = 2.54.
        assert!((jma_from_pgv_vector(1.0) - 2.54).abs() < 1e-12);
        // RotD50 → RotD100: M 5.5, 50 km'de ×1,2272 (Boore & Kishida 2017).
        let i = jma_from_pgv_rotd50(10.0 / 1.2272, 5.5, 50.0);
        assert!((i - 4.36).abs() < 1e-4, "{i}");
        // Eski sabit 1,23 varsayımına göre fark: Rrup ≥ 1 km'de −0,03…+0,04.
        for (m, r) in [(4.0, 300.0), (5.9, 1.0), (7.0, 10.0), (8.5, 300.0)] {
            let d = jma_from_pgv_rotd50(5.0, m, r) - jma_from_pgv_vector(5.0 * 1.23);
            assert!((-0.03..0.04).contains(&d), "M{m} R{r}: {d}");
        }
    }

    #[test]
    fn belirsizlik_birlesimi() {
        // σ_lnPGV = 0 iken bileşen saçılımı (0,0856 ln → 0,068) ve bağıntı
        // belirsizliği (0,19) kalır: √(0,068² + 0,19²) ≈ 0,2017.
        assert!((jma_sigma(0.0) - 0.2017).abs() < 1e-4);
        // σ_lnPGV = 0.6 → 1.82·√(0.6² + 0.0856²)/ln10 ≈ 0.479; toplam ≈ 0.515.
        assert!((jma_sigma(0.6) - 0.5154).abs() < 1e-3);
    }
}
