//! Sarsıntı süresi: Afshari & Stewart (2016), anlamlı süre D5-95
//! (Arias şiddetinin %5'ten %95'e çıktığı süre).
//!
//! Kaynak: Afshari, K. & Stewart, J. P. (2016). Physically Parameterized
//! Prediction Equations for Significant Duration in Active Crustal Regions.
//! Earthquake Spectra 32(4), 2057–2081. doi:10.1193/063015EQS106M.
//! Katsayılar: Tablo 1–3 (D5-95 sütunu). Denklemler:
//! - Kaynak (Eş. 3–6): Δσ = exp(b1 + b2·(min(M, M2) − M*) + b3·max(M − M2, 0)),
//!   M0 = 10^(1,5 M + 16,05) dyn·cm, f0 = 4,9·10⁶ · β · (Δσ / M0)^(1/3)
//!   (β = 3,2 km/s, Δσ bar), F_E = b0 (M ≤ M1) ya da 1 / f0.
//! - Yol (Eş. 7): F_P = c1·min(R, R1) + c2·max(min(R, R2) − R1, 0) +
//!   c3·max(R − R2, 0), R1 = 10 km, R2 = 50 km.
//! - Saha (Eş. 8): F_S = c4·ln(min(Vs30, V1) / Vref); havza terimi YOK
//!   (Z1.0 bilinmiyor; CY14'teki ΔZ1 = 0 seçimiyle tutarlı).
//! - ln D = ln(F_E + F_P) + F_S; σ = √(τ² + φ²), τ ve φ büyüklükle doğrusal.
//!
//! Referans karşılaştırma: pygmm 0.9.1 (MIT), `data/test/as16_ref.csv`
//! (`scripts/gen_duration_ref.py`).
//!
//! Geçerlilik (pygmm parametre sınırları; doğrula: makalenin veri aralığı):
//! M 3–7,9, Rrup 0–200 km, Vs30 200–1000 m/s. Dışındaki girdiler uca
//! çekilir: süre yalnızca "sarsıntı bitti" görselinde (bölge sınırı yanıp
//! sönmesinin durması) kullanılır, bilimsel çıktı olarak gösterilmez.

use crate::quake::FaultType;

pub const MW_RANGE: (f64, f64) = (3.0, 7.9);
pub const RRUP_RANGE: (f64, f64) = (0.0, 200.0);
pub const VS30_RANGE: (f64, f64) = (200.0, 1000.0);

// Tablo 1 (kaynak), D5-95.
const M1: f64 = 5.2;
const M2: f64 = 7.4;
const M_STAR: f64 = 6.0;
const B2: f64 = 0.9443;
const B3: f64 = -3.911;
// Tablo 2 (yol ve saha), D5-95.
const C1: f64 = 0.3165;
const C2: f64 = 0.2539;
const C3: f64 = 0.0932;
// Makale katsayısı; 1/π ile benzerliği tesadüf.
#[allow(clippy::approx_constant)]
const C4: f64 = -0.3183;
const R1: f64 = 10.0;
const R2: f64 = 50.0;
const V1: f64 = 600.0;
const V_REF: f64 = 369.9;
// Tablo 3 (belirsizlik), D5-95.
const TAU1: f64 = 0.25;
const TAU2: f64 = 0.19;
const PHI1: f64 = 0.43;
const PHI2: f64 = 0.35;
/// Kabuk kayma dalgası hızı (km/s), köşe frekansı bağıntısında.
const BETA: f64 = 3.2;

/// (b0, b1), fay tipine göre (Tablo 1).
fn b0_b1(fault: FaultType) -> (f64, f64) {
    match fault {
        FaultType::StrikeSlip => (2.302, 3.467),
        FaultType::Reverse => (1.612, 4.536),
        FaultType::Normal => (2.541, 3.170),
    }
}

/// D5-95 (s) medyanı ve ln standart sapması. Girdiler geçerlilik
/// aralığına çekilir (modül belgesi).
pub fn d595(mw: f64, rrup_km: f64, vs30: f64, fault: FaultType) -> (f64, f64) {
    let m = mw.clamp(MW_RANGE.0, MW_RANGE.1);
    let r = rrup_km.clamp(RRUP_RANGE.0, RRUP_RANGE.1);
    let v = vs30.clamp(VS30_RANGE.0, VS30_RANGE.1);
    let (b0, b1) = b0_b1(fault);
    let stress_drop = (b1 + B2 * (m.min(M2) - M_STAR) + B3 * (m - M2).max(0.0)).exp();
    let moment = 10f64.powf(1.5 * m + 16.05);
    let f0 = 4.9e6 * BETA * (stress_drop / moment).cbrt();
    let f_e = if m <= M1 { b0 } else { 1.0 / f0 };
    let f_p = C1 * r.min(R1) + C2 * (r.min(R2) - R1).max(0.0) + C3 * (r - R2).max(0.0);
    let f_s = C4 * (v.min(V1) / V_REF).ln();
    let ln_d = (f_e + f_p).ln() + f_s;
    let tau = TAU1 + (TAU2 - TAU1) * (m.clamp(6.5, 7.0) - 6.5) / 0.5;
    let phi = PHI1 + (PHI2 - PHI1) * (m.clamp(5.5, 5.75) - 5.5) / 0.25;
    (ln_d.exp(), (tau * tau + phi * phi).sqrt())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pygmm_ile_ayni() {
        let csv = include_str!("../../../data/test/as16_ref.csv");
        let mut n = 0;
        for line in csv.lines().skip(1) {
            let f: Vec<&str> = line.split(',').collect();
            let num = |i: usize| f[i].parse::<f64>().unwrap();
            let fault = match f[3] {
                "SS" => FaultType::StrikeSlip,
                "RS" => FaultType::Reverse,
                _ => FaultType::Normal,
            };
            let (d, sigma) = d595(num(0), num(1), num(2), fault);
            assert!(
                (d.ln() - num(4)).abs() < 1e-6,
                "{line}: ln D = {} (beklenen {})",
                d.ln(),
                num(4)
            );
            assert!((sigma - num(5)).abs() < 1e-5, "{line}: σ = {sigma}");
            n += 1;
        }
        assert_eq!(n, 810);
    }

    #[test]
    fn buyukluk_ve_mesafeyle_artar() {
        let ss = FaultType::StrikeSlip;
        let small = d595(5.5, 20.0, 400.0, ss).0;
        let big = d595(7.5, 20.0, 400.0, ss).0;
        let far = d595(7.5, 150.0, 400.0, ss).0;
        assert!(small < big && big < far);
        // Yumuşak zeminde daha uzun.
        assert!(d595(7.0, 30.0, 200.0, ss).0 > d595(7.0, 30.0, 760.0, ss).0);
        // Aralık dışı girdiler uca çekilir.
        assert_eq!(d595(8.5, 300.0, 100.0, ss), d595(7.9, 200.0, 200.0, ss));
    }
}
