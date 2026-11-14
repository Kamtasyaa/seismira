//! Yer hareketi tahmin denklemleri (GMPE), CLAUDE.md Bölüm 7.4.

pub mod cy14;

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
