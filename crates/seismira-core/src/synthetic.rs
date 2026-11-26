//! Hedef tepki spektrumuna uyumlu yapay ivme kaydı (eğitim / ön inceleme).
//!
//! Yöntem: Gasparini & Vanmarcke (1976), "Simulated earthquake motions
//! compatible with prescribed response spectra", MIT R76-4 (SIMQKE)
//! yaklaşımı: rastgele fazlı sinüslerin toplamı × zarf; genlikler hedef /
//! hesaplanan spektrum oranıyla yinelenerek düzeltilir. RAPOR OKUNAMADI
//! (açık kopya yok, 2026-11-26): ayrıntılar (zarf biçimi, yineleme kuralı)
//! bizim seçimimizdir; doğruluk ÇIKTIDAN sınanır:
//! - spektrum ([`crate::sdof`], Nigam & Jennings 1969 ile doğrulanmış)
//!   hedefe yakın (testte ±%15 içinde, ortalama |hata| < %6),
//! - Arias şiddetinden ölçülen D5-95 süresi hedefe yakın (±%10).
//!
//! TBDY-2018 2.5.1.2: zaman tanım alanında hesapta öncelik gerçek
//! kayıtlarındır; benzeştirilmiş kayıtlar ancak yeterli gerçek kayıt yoksa
//! ve model bölge kayıtlarıyla uyumlu gösterilirse kullanılabilir; en az 11
//! kayıt. Bu kayıt TASARIM İÇİN DEĞİLDİR (dosya başlığında yazar).
//!
//! Zarf (seçim): t₁'e kadar ikinci dereceden yükseliş, t₂'ye kadar sabit,
//! sonra üstel azalma exp(−c(t − t₂)); t₁ = 0,1·D, c = 3/D, plato D5-95
//! hedefini tutacak şekilde ikiye bölmeyle (Arias ∝ ∫ zarf²).

use crate::sdof::spectral_acceleration;

/// Tekrarlanabilir rastgele sayı (SplitMix64; tohumlu).
struct Rng(u64);
impl Rng {
    fn next_f64(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        (z >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// Üretilen kayıt.
#[derive(Clone, Debug)]
pub struct Accelerogram {
    /// Örnekleme aralığı (s).
    pub dt: f64,
    /// İvme (g).
    pub acc: Vec<f64>,
    /// Hedef periyotlarda elde edilen spektrum (g) ve hedef (g).
    pub periods: Vec<f64>,
    pub achieved: Vec<f64>,
    pub target: Vec<f64>,
    /// Hedef ve ölçülen D5-95 (s).
    pub d595_target: f64,
    pub d595: f64,
}

/// Zarf: (t₁, t₂, c) ile.
fn envelope(t: f64, t1: f64, t2: f64, c: f64) -> f64 {
    if t < t1 {
        (t / t1).powi(2)
    } else if t <= t2 {
        1.0
    } else {
        (-c * (t - t2)).exp()
    }
}

/// Arias şiddetinin %5'ten %95'e süresi (s).
pub fn d595(acc: &[f64], dt: f64) -> f64 {
    let total: f64 = acc.iter().map(|a| a * a).sum();
    if total <= 0.0 {
        return 0.0;
    }
    let (mut cum, mut t5, mut t95) = (0.0, None, None);
    for (k, a) in acc.iter().enumerate() {
        cum += a * a;
        if t5.is_none() && cum >= 0.05 * total {
            t5 = Some(k as f64 * dt);
        }
        if t95.is_none() && cum >= 0.95 * total {
            t95 = Some(k as f64 * dt);
            break;
        }
    }
    t95.unwrap_or(0.0) - t5.unwrap_or(0.0)
}

/// Zarf parametreleri: hedef D5-95 için plato uzunluğu (ikiye bölme).
fn envelope_for(d: f64, dt: f64, n: usize) -> (f64, f64, f64) {
    let (t1, c) = (0.1 * d, 3.0 / d);
    let measure = |plateau: f64| {
        let env: Vec<f64> = (0..n)
            .map(|k| envelope(k as f64 * dt, t1, t1 + plateau, c))
            .collect();
        d595(&env, dt)
    };
    let (mut lo, mut hi) = (0.0, 2.0 * d);
    for _ in 0..40 {
        let mid = 0.5 * (lo + hi);
        if measure(mid) < d {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    (t1, t1 + 0.5 * (lo + hi), c)
}

/// Hedef spektruma (`periods` artan, `target` g) uyumlu kayıt; %5 sönüm.
/// `d595_target` (s): anlamlı süre; `seed`: tekrarlanabilirlik.
pub fn generate(periods: &[f64], target: &[f64], d595_target: f64, seed: u64) -> Accelerogram {
    assert_eq!(periods.len(), target.len());
    let dt = 0.01;
    let d = d595_target.clamp(2.0, 120.0);
    // Toplam uzunluk: zarfın kuyruğu sönsün (c·(T − t₂) ≥ ~8).
    let total = (d * 2.0 + 8.0 / (3.0 / d) * 0.5).max(20.0);
    let n = (total / dt) as usize;
    let (t1, t2, c) = envelope_for(d, dt, n);
    let env: Vec<f64> = (0..n).map(|k| envelope(k as f64 * dt, t1, t2, c)).collect();
    // Frekanslar: 0,1–25 Hz (spektrum 0,04–10 s), eşit aralıklı.
    let df = 1.0 / total;
    let freqs: Vec<f64> = (1..)
        .map(|k| f64::from(k) * df)
        .take_while(|&f| f <= 25.0)
        .filter(|&f| f >= 0.1)
        .collect();
    let mut rng = Rng(seed ^ 0x5EED_5E15_A1A0);
    let phases: Vec<f64> = freqs
        .iter()
        .map(|_| 2.0 * std::f64::consts::PI * rng.next_f64())
        .collect();
    // Hedefin periyoda göre log-log ara değeri (uçlarda sabit).
    let interp = |vals: &[f64], t: f64| -> f64 {
        if t <= periods[0] {
            return vals[0];
        }
        for k in 1..periods.len() {
            if t <= periods[k] {
                let f = (t / periods[k - 1]).ln() / (periods[k] / periods[k - 1]).ln();
                return (vals[k - 1].ln() + f * (vals[k].ln() - vals[k - 1].ln())).exp();
            }
        }
        vals[periods.len() - 1]
    };
    let mut amp: Vec<f64> = freqs.iter().map(|&f| interp(target, 1.0 / f)).collect();
    let synth = |amp: &[f64]| -> Vec<f64> {
        let w: Vec<f64> = freqs
            .iter()
            .map(|f| 2.0 * std::f64::consts::PI * f)
            .collect();
        let mut x: Vec<f64> = (0..n)
            .map(|k| {
                let t = k as f64 * dt;
                let s: f64 = (0..freqs.len())
                    .map(|j| amp[j] * (w[j] * t + phases[j]).cos())
                    .sum();
                env[k] * s
            })
            .collect();
        // Taban düzeltmesi (basit): ortalama sıfır.
        let mean = x.iter().sum::<f64>() / n as f64;
        x.iter_mut().for_each(|v| *v -= mean);
        x
    };
    let mut acc = synth(&amp);
    let mut achieved = spectral_acceleration(&acc, dt, periods, 0.05);
    for _ in 0..12 {
        let ratio: Vec<f64> = target
            .iter()
            .zip(&achieved)
            .map(|(t, a)| t / a.max(1e-12))
            .collect();
        for (a, f) in amp.iter_mut().zip(&freqs) {
            *a *= interp(&ratio, 1.0 / f);
        }
        acc = synth(&amp);
        achieved = spectral_acceleration(&acc, dt, periods, 0.05);
    }
    Accelerogram {
        dt,
        d595: d595(&acc, dt),
        acc,
        periods: periods.to_vec(),
        achieved,
        target: target.to_vec(),
        d595_target: d,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// TBDY-benzeri hedef (S_DS 1,0, S_D1 0,4, ZC): spektrum uyumu ve süre.
    #[test]
    fn hedefe_uyar() {
        let ds = crate::tbdy::DesignSpectrum::new(0.83, 0.27, crate::tbdy::SiteClass::ZC).unwrap();
        let periods: Vec<f64> = crate::shaking::SPECTRUM_PERIODS
            .iter()
            .copied()
            .filter(|&t| (0.05..=4.0).contains(&t))
            .collect();
        let target: Vec<f64> = periods.iter().map(|&t| ds.sae(t)).collect();
        let g = generate(&periods, &target, 15.0, 7);
        let errs: Vec<f64> = g
            .achieved
            .iter()
            .zip(&g.target)
            .map(|(a, t)| a / t - 1.0)
            .collect();
        let max = errs.iter().fold(0.0f64, |m, e| m.max(e.abs()));
        let mean = errs.iter().map(|e| e.abs()).sum::<f64>() / errs.len() as f64;
        eprintln!(
            "spektrum hatası: en büyük {max:.3}, ortalama {mean:.3}; D5-95 {:.2} s (hedef 15)",
            g.d595
        );
        assert!(
            max < 0.15 && mean < 0.06,
            "en büyük {max:.3}, ortalama {mean:.3}"
        );
        assert!((g.d595 / 15.0 - 1.0).abs() < 0.10, "D5-95 {}", g.d595);
        // Tekrarlanabilir.
        let h = generate(&periods, &target, 15.0, 7);
        assert_eq!(g.acc, h.acc);
    }

    #[test]
    fn arias_suresi() {
        // Sabit genlik 10 s, dt 0,01: D5-95 = 9 s.
        let acc = vec![1.0; 1000];
        assert!((d595(&acc, 0.01) - 9.0).abs() < 0.02);
    }
}
