//! Tek serbestlik dereceli sönümlü salınıcının tepkisi ve tepki spektrumu.
//!
//! Kaynak: Nigam, N.C. & Jennings, P.C. (1969), "Calculation of response
//! spectra from strong-motion earthquake records", BSSA 59(2), 909–922
//! (CaltechAUTHORS 4v0yw-dax32, data/raw/refs/synthetic/; birincil metinden
//! okundu 2026-11-26). Taban ivmesi parça parça doğrusal kabul edilir; her
//! adımda kesin çözüm (Eş. 4–6): x_{i+1} = A x_i + B a_i, x = (yer
//! değiştirme, hız), a_i = (a_i, a_{i+1}); A ve B Eş. 6d–6e. Mutlak ivme
//! Eş. 7: z̈ = −(2βω ẋ + ω² x). Spektral ivme Sa = max |z̈| (Eş. 8c).

/// Eş. 6d–6e katsayıları (β sönüm oranı, ω doğal frekans, Δt adım).
fn coeffs(beta: f64, w: f64, dt: f64) -> ([[f64; 2]; 2], [[f64; 2]; 2]) {
    let wd = w * (1.0 - beta * beta).sqrt();
    let e = (-beta * w * dt).exp();
    let (s, c) = (wd * dt).sin_cos();
    let q = beta / (1.0 - beta * beta).sqrt();
    let a11 = e * (q * s + c);
    let a12 = e / wd * s;
    let a21 = -w / (1.0 - beta * beta).sqrt() * e * s;
    let a22 = e * (c - q * s);
    let k1 = (2.0 * beta * beta - 1.0) / (w * w * dt);
    let k2 = 2.0 * beta / (w * w * w * dt);
    let b11 = e * ((k1 + beta / w) * s / wd + (k2 + 1.0 / (w * w)) * c) - k2;
    let b12 = -e * (k1 * s / wd + k2 * c) - 1.0 / (w * w) + k2;
    let b21 = e * ((k1 + beta / w) * (c - q * s) - (k2 + 1.0 / (w * w)) * (wd * s + beta * w * c))
        + 1.0 / (w * w * dt);
    let b22 = -e * (k1 * (c - q * s) - k2 * (wd * s + beta * w * c)) - 1.0 / (w * w * dt);
    ([[a11, a12], [a21, a22]], [[b11, b12], [b21, b22]])
}

/// Mutlak ivme spektrumu (taban ivmesiyle aynı birim), `periods` (s) için,
/// sönüm oranı `beta` (ör. 0,05). `acc`: sabit `dt` aralıklı taban ivmesi.
pub fn spectral_acceleration(acc: &[f64], dt: f64, periods: &[f64], beta: f64) -> Vec<f64> {
    periods
        .iter()
        .map(|&t| {
            let w = 2.0 * std::f64::consts::PI / t;
            let (a, b) = coeffs(beta, w, dt);
            let (mut x, mut v) = (0.0, 0.0);
            let mut peak: f64 = 0.0;
            for k in 0..acc.len().saturating_sub(1) {
                let (g0, g1) = (acc[k], acc[k + 1]);
                let nx = a[0][0] * x + a[0][1] * v + b[0][0] * g0 + b[0][1] * g1;
                let nv = a[1][0] * x + a[1][1] * v + b[1][0] * g0 + b[1][1] * g1;
                x = nx;
                v = nv;
                peak = peak.max((2.0 * beta * w * v + w * w * x).abs());
            }
            peak
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Rezonansta harmonik taban ivmesi A sin(ωt): kararlı durumda göreli
    /// yer değiştirme genliği A / (2βω²) → mutlak ivme genliği
    /// ≈ A·√(1 + 4β²) / (2β) (β = 0,05 → 10,05 A). Uzun kayıtta yakınsar.
    #[test]
    fn rezonans() {
        let (t, beta, dt) = (1.0, 0.05, 0.005);
        let w = 2.0 * std::f64::consts::PI / t;
        let acc: Vec<f64> = (0..(200.0 / dt) as usize)
            .map(|k| (w * k as f64 * dt).sin())
            .collect();
        let sa = spectral_acceleration(&acc, dt, &[t], beta)[0];
        let want = (1.0 + 4.0 * beta * beta).sqrt() / (2.0 * beta);
        assert!((sa / want - 1.0).abs() < 0.01, "{sa} {want}");
    }

    /// Çok kısa periyotta (rijit salınıcı) Sa ≈ en büyük taban ivmesi; çok
    /// uzun periyotta Sa → 0 (statik değil, yalnızca küçük).
    #[test]
    fn uclar() {
        let dt = 0.01;
        let acc: Vec<f64> = (0..2000)
            .map(|k| {
                let t = k as f64 * dt;
                (t * 7.3).sin() * (-((t - 8.0) / 3.0).powi(2)).exp() * 0.4
            })
            .collect();
        let pga = acc.iter().fold(0.0f64, |m, x| m.max(x.abs()));
        let sa = spectral_acceleration(&acc, dt, &[0.02, 20.0], 0.05);
        assert!((sa[0] / pga - 1.0).abs() < 0.02, "{} {pga}", sa[0]);
        assert!(sa[1] < 0.2 * pga, "{}", sa[1]);
    }
}
