//! Yer hareketi tahmin denklemleri (GMPE), CLAUDE.md Bölüm 7.4.

pub mod asb14;
pub mod ask14;
pub mod bssa14;
pub mod cb14;
pub mod cy14;
pub mod ngaw2;
pub mod psbah20;

/// Sığ kabuk NGA-West2 modellerinin bölgesel düzeltmeleri için bölge
/// (merkez üssünün ülkesinden, [`CrustRegion::from_iso`]). Her model
/// yalnızca kendi tanımladığı bölgeleri ayırır; diğerleri temel modeldir.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CrustRegion {
    /// Temel model (küresel / Kaliforniya; modellerin "varsayılan" kolu).
    #[default]
    Global,
    California,
    Japan,
    Taiwan,
    China,
    Turkey,
    Italy,
    NewZealand,
}

impl CrustRegion {
    /// ISO 3166-1 alpha-2 ülke kodundan. Kaliforniya ayrımı yapılmaz (ABD
    /// küresel / Kaliforniya kolu aynıdır: BSSA14 Δc3 "global/CATW",
    /// ASK14 ve CB14 Kaliforniya = temel model).
    pub fn from_iso(code: &str) -> CrustRegion {
        match code {
            "JP" => CrustRegion::Japan,
            "TW" => CrustRegion::Taiwan,
            "CN" => CrustRegion::China,
            "TR" => CrustRegion::Turkey,
            "IT" => CrustRegion::Italy,
            "NZ" => CrustRegion::NewZealand,
            _ => CrustRegion::Global,
        }
    }
}

/// Yer hareketi ölçüsü.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Imt {
    /// En büyük yer ivmesi (g).
    Pga,
    /// En büyük yer hızı (cm/s).
    Pgv,
    /// %5 sönümlü spektral ivme (g), periyot (s).
    Sa(f64),
}

/// Log-normal tahmin: doğal logaritmada ortalama ve standart sapmalar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LnMotion {
    /// ln(değer); birim: PGA ve SA için g, PGV için cm/s.
    pub mean: f64,
    /// Toplam standart sapma (ln).
    pub sigma: f64,
    /// Olaylar arası standart sapma (ln); Bölüm 7.8'deki aralık için.
    pub tau: f64,
}

impl LnMotion {
    pub fn median(&self) -> f64 {
        self.mean.exp()
    }
}

/// Tabloda olmayan SA periyodu için komşu periyotlardan log-log ara değer
/// (Bölüm 7.4): ln y periyodun logaritmasında doğrusal. `periods` artan
/// sırada; `at(k)` k. periyodun tahminidir. Tablo dışında `None`.
pub(crate) fn interp_period(
    periods: &[f64],
    period: f64,
    at: impl Fn(usize) -> LnMotion,
) -> Option<LnMotion> {
    if !(*periods.first()?..=*periods.last()?).contains(&period) {
        return None;
    }
    let k = periods.partition_point(|&p| p < period);
    if (periods[k] - period).abs() < 1e-9 {
        return Some(at(k));
    }
    let (a, b) = (at(k - 1), at(k));
    let f = (period / periods[k - 1]).ln() / (periods[k] / periods[k - 1]).ln();
    let lerp = |x: f64, y: f64| x + f * (y - x);
    Some(LnMotion {
        mean: lerp(a.mean, b.mean),
        sigma: lerp(a.sigma, b.sigma),
        tau: lerp(a.tau, b.tau),
    })
}
