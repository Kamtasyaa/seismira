//! Yatay bileşen tanımları arasında dönüşüm (PGV).
//!
//! GMPE'ler RotD50 verir; şiddet dönüşümleri başka tanımlar bekler:
//! - JMA aletsel şiddeti (Midorikawa vd. 1999): iki yatay bileşenin
//!   vektörel bileşkesinin tepesi = RotD100.
//! - MMI (Worden vd. 2012): iki yatay bileşenin büyüğü (USGS ShakeMap
//!   uzlaşımı; ShakeMap GMPE çıktısını bu tanıma çevirip dönüştürür).
//!
//! Model: Boore, D. M. & Kishida, T. (2017). Relations between some
//! horizontal-component ground-motion intensity measures used in practice.
//! BSSA 107(1), 334-343, doi:10.1785/0120160250.
//!
//! ```text
//! ln(Y_a / Y_b) = c0 + r1·ln(Rrup / 50) + m1·(M − 5.5) + m2·(M − 5.5)²
//! ```
//! Rrup 0,01–400 km ve M 2–9 aralığına çekilir (veri aralığı).
//!
//! Katsayılar: makaleye erişilemedi (yazar sitesinin sertifikası süresi
//! dolmuş); USGS ShakeMap'in (kamu malı / CC0) `shakelib/conversions/imc`
//! modülündeki makale veri dosyalarından alındı (2026-11-15), PGV satırı
//! (`per = −1`), ShakeMap'in kullandığı "smooth" sütunları (PGV için ham
//! değerlerle aynı): `nga-w2.lmR1M2_D100D50…csv` ve `…_LargerD50…csv`.
//! Formül ve kırpma sınırları aynı modülden (`boore_kishida_2017.py`).

/// Bir oran modelinin katsayıları.
#[derive(Clone, Copy, Debug)]
struct Coeffs {
    c0: f64,
    r1: f64,
    m1: f64,
    m2: f64,
    /// Oranın ln standart sapması (kayıttan kayda saçılım).
    sigma: f64,
}

/// RotD100 / RotD50, PGV.
const D100_D50_PGV: Coeffs = Coeffs {
    c0: 0.204731623917226,
    r1: -0.0127157764609141,
    m1: 0.00123001053169241,
    m2: -0.0014255923619871,
    sigma: 0.0855527068392153,
};

/// İki yatay bileşenin büyüğü / RotD50, PGV.
const LARGER_D50_PGV: Coeffs = Coeffs {
    c0: 0.124576699819479,
    r1: -0.010187072147666,
    m1: 0.00121314885250292,
    m2: -0.000764374621288474,
    sigma: 0.0962530004202919,
};

fn ln_ratio(c: &Coeffs, mw: f64, rrup_km: f64) -> f64 {
    let r = rrup_km.clamp(1e-2, 400.0);
    let m = mw.clamp(2.0, 9.0) - 5.5;
    c.c0 + c.r1 * (r / 50.0).ln() + c.m1 * m + c.m2 * m * m
}

/// ln(RotD100 / RotD50), PGV.
pub fn ln_rotd100_per_rotd50_pgv(mw: f64, rrup_km: f64) -> f64 {
    ln_ratio(&D100_D50_PGV, mw, rrup_km)
}

/// ln(büyük bileşen / RotD50), PGV.
pub fn ln_larger_per_rotd50_pgv(mw: f64, rrup_km: f64) -> f64 {
    ln_ratio(&LARGER_D50_PGV, mw, rrup_km)
}

/// Dönüşümün eklediği ln standart sapması (GMPE σ'sıyla karesel toplanır;
/// ShakeMap `convertSigmasOnce` ile aynı kural).
pub const SIGMA_ROTD100_PGV: f64 = D100_D50_PGV.sigma;
pub const SIGMA_LARGER_PGV: f64 = LARGER_D50_PGV.sigma;

#[cfg(test)]
mod tests {
    use super::*;

    /// Referans: aynı CSV satırlarından bağımsız (Python) hesap, 2026-11-15.
    #[test]
    fn boore_kishida_oranlari() {
        // (Mw, Rrup, RotD100/RotD50, büyük/RotD50)
        for (m, r, d100, larger) in [
            (5.5, 50.0, 1.2272, 1.1327),
            (6.5, 20.0, 1.2413, 1.1438),
            (7.0, 10.0, 1.2509, 1.1515),
            (7.6, 100.0, 1.2119, 1.1238),
            (7.8, 250.0, 1.1967, 1.1129),
            (5.0, 300.0, 1.1984, 1.1113),
        ] {
            let a = ln_rotd100_per_rotd50_pgv(m, r).exp();
            let b = ln_larger_per_rotd50_pgv(m, r).exp();
            assert!((a - d100).abs() < 6e-5, "M{m} R{r}: {a}");
            assert!((b - larger).abs() < 6e-5, "M{m} R{r}: {b}");
        }
        // Merkez noktada (M 5.5, 50 km) oran exp(c0).
        assert_eq!(ln_rotd100_per_rotd50_pgv(5.5, 50.0), D100_D50_PGV.c0);
    }

    #[test]
    fn aralik_disi_uca_cekilir() {
        assert_eq!(
            ln_rotd100_per_rotd50_pgv(9.5, 1000.0),
            ln_rotd100_per_rotd50_pgv(9.0, 400.0)
        );
        assert_eq!(
            ln_larger_per_rotd50_pgv(1.0, 0.0),
            ln_larger_per_rotd50_pgv(2.0, 0.01)
        );
        // RotD100 ≥ büyük bileşen ≥ RotD50 (tanım gereği).
        for (m, r) in [(4.0, 5.0), (6.0, 60.0), (8.5, 300.0)] {
            let (a, b) = (
                ln_rotd100_per_rotd50_pgv(m, r),
                ln_larger_per_rotd50_pgv(m, r),
            );
            assert!(a > b && b > 0.0, "M{m} R{r}");
        }
    }
}
