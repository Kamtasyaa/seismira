//! PGV → Değiştirilmiş Mercalli Şiddeti (MMI), CLAUDE.md Bölüm 7.6.
//!
//! Kaynak: Worden, C. B., Gerstenberger, M. C., Rhoades, D. A. & Wald,
//! D. J. (2012). Probabilistic relationships between ground-motion
//! parameters and Modified Mercalli Intensity in California. BSSA 102(1),
//! 204-221. İki doğrulu bağıntı + mesafe ve büyüklük terimleri:
//!
//! ```text
//! MMI = c1 + c2·log10(PGV)                 log10(PGV) ≤ t1
//! MMI = c3 + c4·log10(PGV)                 log10(PGV) > t1
//!       + c5 + c6·log10(D) + c7·M          (D 10–300 km, M 3,0–7,3'e çekilir)
//! ```
//! PGV cm/s, iki yatay bileşenin BÜYÜĞÜ ([`crate::component`]). Sonuç
//! 1–10 aralığına çekilir.
//!
//! Katsayılar ve uygulama ayrıntıları: USGS ShakeMap (kamu malı / CC0)
//! `shakelib/gmice/wgrw12.py`, 2026-11-15 (makalenin kendisi görülmedi).
//! MMI 2'nin altı için ShakeMap'in ek doğrusu (makalede tartışılan ama
//! açıkça verilmeyen 1–2 aralığı) da oradan.

use crate::component;

// PGV satırı.
const C1: f64 = 3.78;
const C2: f64 = 1.47;
const C3: f64 = 2.89;
const C4: f64 = 3.16;
const C5: f64 = 0.90;
const C6: f64 = 0.00;
const C7: f64 = -0.18;
/// Doğruların kesiştiği log10(PGV) (MMI ≈ 4,56).
const T1: f64 = 0.53;
// MMI 1–2 aralığı (ShakeMap).
const LOW_C1: f64 = 4.62;
const LOW_C2: f64 = 2.17;
const LOW_T1: f64 = -1.21;

/// Bağıntının standart sapması (MMI birimi), PGV.
pub const SIGMA_MMI: f64 = 0.63;

/// Kırpılmamış MMI ve eğim (dMMI / dlog10 PGV). `pgv_larger`: iki yatay
/// bileşenin büyüğü (cm/s).
fn raw(pgv_larger: f64, mw: f64, rrup_km: f64) -> (f64, f64) {
    let l = pgv_larger.log10();
    let (base, slope) = if l < LOW_T1 {
        (LOW_C1 + LOW_C2 * l, LOW_C2)
    } else if l < T1 {
        (C1 + C2 * l, C2)
    } else {
        (C3 + C4 * l, C4)
    };
    let resid = C5 + C6 * rrup_km.clamp(10.0, 300.0).log10() + C7 * mw.clamp(3.0, 7.3);
    (base + resid, slope)
}

/// Büyük bileşen PGV (cm/s) → MMI (1–10).
pub fn mmi_from_pgv_larger(pgv_larger: f64, mw: f64, rrup_km: f64) -> f64 {
    raw(pgv_larger, mw, rrup_km).0.clamp(1.0, 10.0)
}

/// RotD50 PGV (cm/s) → MMI (1–10); bileşen dönüşümü Boore & Kishida (2017).
pub fn mmi_from_pgv_rotd50(pgv_rotd50: f64, mw: f64, rrup_km: f64) -> f64 {
    let larger = pgv_rotd50 * component::ln_larger_per_rotd50_pgv(mw, rrup_km).exp();
    mmi_from_pgv_larger(larger, mw, rrup_km)
}

/// GMPE'nin ln(PGV) standart sapmasını MMI birimine taşır: bileşen
/// dönüşümünün saçılımı eklenir, bağıntının o noktadaki eğimiyle
/// çarpılır ve bağıntının kendi belirsizliğiyle birleştirilir (bağımsız
/// kabul edilerek).
pub fn mmi_sigma(pgv_rotd50: f64, sigma_ln_pgv: f64, mw: f64, rrup_km: f64) -> f64 {
    let larger = pgv_rotd50 * component::ln_larger_per_rotd50_pgv(mw, rrup_km).exp();
    let slope = raw(larger, mw, rrup_km).1;
    let s_ln = (sigma_ln_pgv.powi(2) + component::SIGMA_LARGER_PGV.powi(2)).sqrt();
    let from_gm = slope * s_ln / std::f64::consts::LN_10;
    (from_gm * from_gm + SIGMA_MMI * SIGMA_MMI).sqrt()
}

/// Romen rakamıyla MMI sınıfı (en yakın tam sayı, I–X).
pub fn roman(mmi: f64) -> &'static str {
    const R: [&str; 10] = ["I", "II", "III", "IV", "V", "VI", "VII", "VIII", "IX", "X"];
    R[(mmi.round().clamp(1.0, 10.0) as usize) - 1]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mesafe / büyüklük terimi: PGV için c5 + c7·M (c6 = 0).
    fn resid(mw: f64) -> f64 {
        0.90 - 0.18 * mw
    }

    #[test]
    fn worden_pgv_degerleri() {
        // Elle: log10(PGV) = 1 (üst doğru): 2.89 + 3.16 = 6.05.
        let m = mmi_from_pgv_larger(10.0, 6.0, 50.0);
        assert!((m - (6.05 + resid(6.0))).abs() < 1e-12, "{m}");
        // log10(PGV) = 0 (alt doğru): 3.78.
        let m = mmi_from_pgv_larger(1.0, 5.0, 50.0);
        assert!((m - (3.78 + resid(5.0))).abs() < 1e-12, "{m}");
        // M 5'te terim sıfırdır (0.90 − 0.18·5).
        assert!(resid(5.0).abs() < 1e-12);
        // Büyüklük 3,0–7,3'e çekilir; PGV'de mesafenin etkisi yok (c6 = 0).
        assert_eq!(
            mmi_from_pgv_larger(10.0, 8.0, 20.0),
            mmi_from_pgv_larger(10.0, 7.3, 250.0)
        );
    }

    #[test]
    fn dogrular_kesisimde_surekli_ve_sinirlar() {
        // t1 = 0.53'te iki doğru ~4,56'da buluşur (katsayı yuvarlaması kadar).
        let (a, b) = (C1 + C2 * T1, C3 + C4 * T1);
        assert!((a - 4.56).abs() < 0.01 && (b - 4.56).abs() < 0.01);
        // MMI 2'de alt uzantı ile birleşir.
        assert!(((LOW_C1 + LOW_C2 * LOW_T1) - (C1 + C2 * LOW_T1)).abs() < 0.02);
        // 1–10 aralığına çekilir, PGV ile artar.
        assert_eq!(mmi_from_pgv_larger(1e-6, 6.0, 50.0), 1.0);
        assert_eq!(mmi_from_pgv_larger(1e4, 6.0, 50.0), 10.0);
        let mut last = 0.0;
        for k in 0..60 {
            let v = 10f64.powf(-2.0 + 0.08 * f64::from(k));
            let m = mmi_from_pgv_larger(v, 6.5, 30.0);
            assert!(m >= last);
            last = m;
        }
    }

    #[test]
    fn bilesen_donusumu_ve_belirsizlik() {
        // RotD50 → büyük bileşen: M 5.5, 50 km'de ×1,1327.
        let direct = mmi_from_pgv_larger(10.0 * 1.1327, 5.5, 50.0);
        assert!((mmi_from_pgv_rotd50(10.0, 5.5, 50.0) - direct).abs() < 1e-3);
        // σ_lnPGV = 0 ve eğim 3,16: yalnızca bileşen saçılımı + bağıntı.
        let s = mmi_sigma(10.0, 0.0, 5.5, 50.0);
        let want =
            ((3.16f64 * 0.0962530004202919 / std::f64::consts::LN_10).powi(2) + 0.63 * 0.63).sqrt();
        assert!((s - want).abs() < 1e-12);
        assert!(mmi_sigma(10.0, 0.6, 5.5, 50.0) > s);
    }

    #[test]
    fn romen_rakami() {
        assert_eq!(roman(1.0), "I");
        assert_eq!(roman(6.49), "VI");
        assert_eq!(roman(6.5), "VII");
        assert_eq!(roman(9.7), "X");
        assert_eq!(roman(12.0), "X");
    }
}
