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

/// Dalma-batma (arayüz ve levha içi) sarsıntı süresi D5-95: Bahrampouri,
/// Rodriguez-Marek & Green (2021).
///
/// Kaynak: Bahrampouri, M., Rodriguez-Marek, A. & Green, R. A. (2021).
/// Ground motion prediction equations for significant duration using the
/// KiK-net database. Earthquake Spectra 37(2), 903–920.
/// doi:10.1177/8755293020970971. Denklemler 7–12, katsayılar Tablo 1
/// (D5-95), standart sapmalar Tablo 2 (makalenin yayımlanmış sürümü
/// okundu, 2026-11-20; tablo sayfa görüntüsünden doğrulandı):
/// - ln D = ln(F_source + F_path) + F_site (Eş. 7)
/// - F_source = 10^(m1·(M − m2)) + m3 (Eş. 8)
/// - F_path = slope·Rrup (Rrup ≤ R1), slope·[R1 + MSE·(Rrup − R1)] (Rrup > R1)
///   (Eş. 9); slope = r1 + r2·(M − 4) (Eş. 10); MSE = 0 (M ≤ M1),
///   (M − M1)/(M2 − M1) (M1 < M ≤ M2), 1 (M > M2) (Eş. 11)
/// - F_site = s1·ln(min(Vs30, 600)/600) + s2·min(δZ1, 250) + s3 (Eş. 12).
///   δZ1 = 0 alınır (Z1 bilinmiyor; CY14 ve AS16'daki seçimle tutarlı).
/// - σ = √(φSS² + τ² + φS2S²) (Eş. 14, Tablo 2).
///
/// Levha içinde m3 fay tipine bağlı; Tablo 1'de değerler kabuk sütunuyla
/// aynı sırada (RS, SS, NS) verilmiş, etiket yalnızca kabuk sütununda.
/// NOT: tabloda NS'nin m3'ü en büyük, oysa metin (s. 908) "normal
/// faylarda süre daha kısa" diyor; tablo uygulandı (docs/bilim.md).
///
/// Geçerlilik (makale): M 4–9, KiK-net verisi; dalma-batma kayıtları
/// Rrup ≤ 500 km ile sınırlı. Dışındaki girdiler uca çekilir (AS16 ile
/// aynı gerekçe: süre yalnızca "sarsıntı bitti" görselinde kullanılır).
pub mod bahrampouri2021 {
    use crate::gmpe::psbah20::EventType;
    use crate::quake::FaultType;

    pub const MW_RANGE: (f64, f64) = (4.0, 9.0);
    pub const RRUP_RANGE: (f64, f64) = (0.0, 500.0);
    /// Vs30 alt sınırı (m/s): Parker vd. (2020) aralığıyla aynı; site
    /// terimi 600 m/s üstünde zaten sabit.
    pub const VS30_MIN: f64 = 150.0;

    struct Coeffs {
        m1: f64,
        m2: f64,
        /// RS, SS, NS (arayüzde üçü aynı).
        m3: [f64; 3],
        mag1: f64,
        mag2: f64,
        r1: f64,
        r2: f64,
        big_r1: f64,
        s1: f64,
        s3: f64,
        /// φSS, τ, φS2S (Tablo 2).
        sd: [f64; 3],
    }

    // Tablo 1 ve 2, D5-95. s2 (δZ1 eğimi) δZ1 = 0 olduğu için kullanılmaz:
    // levha içi 0.001409, arayüz 0.001234.
    const INTRASLAB: Coeffs = Coeffs {
        m1: 0.3850,
        m2: 4.1604,
        m3: [5.828, 4.231, 5.496],
        mag1: 5.5,
        mag2: 8.0,
        r1: 0.09936,
        r2: 0.02495,
        big_r1: 200.0,
        s1: -0.2440,
        s3: -0.04109,
        sd: [0.335, 0.194, 0.245],
    };
    const INTERFACE: Coeffs = Coeffs {
        m1: 0.2384,
        m2: 4.16,
        m3: [8.4117; 3],
        mag1: 5.5,
        mag2: 8.0,
        r1: 0.08862,
        r2: 0.04194,
        big_r1: 200.0,
        s1: -0.2875,
        s3: -0.03137,
        sd: [0.275, 0.191, 0.223],
    };

    /// D5-95 (s) medyanı ve ln standart sapması. Girdiler geçerlilik
    /// aralığına çekilir.
    pub fn d595(
        mw: f64,
        rrup_km: f64,
        vs30: f64,
        event: EventType,
        fault: FaultType,
    ) -> (f64, f64) {
        let c = match event {
            EventType::Interface => &INTERFACE,
            EventType::Intraslab => &INTRASLAB,
        };
        let m = mw.clamp(MW_RANGE.0, MW_RANGE.1);
        let r = rrup_km.clamp(RRUP_RANGE.0, RRUP_RANGE.1);
        let v = vs30.max(VS30_MIN);
        let m3 = c.m3[match fault {
            FaultType::Reverse => 0,
            FaultType::StrikeSlip => 1,
            FaultType::Normal => 2,
        }];
        let source = 10f64.powf(c.m1 * (m - c.m2)) + m3;
        let slope = c.r1 + c.r2 * (m - 4.0);
        let mse = ((m - c.mag1) / (c.mag2 - c.mag1)).clamp(0.0, 1.0);
        let path = if r <= c.big_r1 {
            slope * r
        } else {
            slope * (c.big_r1 + mse * (r - c.big_r1))
        };
        let site = c.s1 * (v.min(600.0) / 600.0).ln() + c.s3;
        let ln_d = (source + path).ln() + site;
        let [a, b, d] = c.sd;
        (ln_d.exp(), (a * a + b * b + d * d).sqrt())
    }
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

    /// Bahrampouri vd. (2021) ↔ OpenQuake 3.26.2 (yalnızca çıktı,
    /// `scripts/gen_duration_sub_ref.py`): arayüz ve levha içi (RS/SS/NS),
    /// M 4–9, Rrup 5–500 km, Vs30 150–1200.
    #[test]
    fn bahrampouri_openquake_ile_ayni() {
        use crate::gmpe::psbah20::EventType;
        let csv = include_str!("../../../data/test/duration_sub_ref.csv");
        let mut n = 0;
        for line in csv.lines().skip(1) {
            let f: Vec<&str> = line.split(',').collect();
            let num = |i: usize| f[i].parse::<f64>().unwrap();
            let event = match f[0] {
                "interface" => EventType::Interface,
                _ => EventType::Intraslab,
            };
            let fault = match f[1] {
                "RS" => FaultType::Reverse,
                "SS" => FaultType::StrikeSlip,
                _ => FaultType::Normal,
            };
            let (d, sigma) = bahrampouri2021::d595(num(2), num(3), num(4), event, fault);
            assert!(
                // En büyük fark 4,6e-5 (%0,005); OpenQuake'in δZ1 hesabındaki
                // yuvarlama (z1pt0 = −999 dalı), katsayılar aynı.
                (d.ln() - num(5)).abs() < 1e-4,
                "{line}: ln D = {} (beklenen {})",
                d.ln(),
                num(5)
            );
            assert!((sigma - num(6)).abs() < 1e-3, "{line}: σ = {sigma}");
            n += 1;
        }
        assert_eq!(n, 768);
    }

    #[test]
    fn dalma_batma_suresi_kabuktan_uzun_ve_m9_gercekci() {
        use crate::gmpe::psbah20::EventType;
        let rs = FaultType::Reverse;
        // Makalenin ana bulgusu: aynı M ve mesafede dalma-batma süresi
        // kabuktakinden uzun.
        let crust = d595(7.0, 100.0, 400.0, rs).0;
        let sub = bahrampouri2021::d595(7.0, 100.0, 400.0, EventType::Interface, rs).0;
        assert!(sub > crust, "kabuk {crust} dalma-batma {sub}");
        // Mw 9, 300 km: ~1,5–2 dakika (Tōhoku kayıtları 100–200 s).
        let m9 = bahrampouri2021::d595(9.0, 300.0, 400.0, EventType::Interface, rs).0;
        assert!((90.0..200.0).contains(&m9), "{m9}");
        // Aralık dışı uca çekilir.
        assert_eq!(
            bahrampouri2021::d595(9.5, 800.0, 100.0, EventType::Interface, rs),
            bahrampouri2021::d595(9.0, 500.0, 150.0, EventType::Interface, rs)
        );
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
