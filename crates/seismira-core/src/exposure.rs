//! Bina stoku ve kırılganlık: tahmini bina hasarı (CLAUDE.md Bölüm 7.8,
//! Faz 6).
//!
//! Kaynak: ESRM20, European Seismic Risk Model (Crowley vd. 2021, EFEHR
//! Technical Report 002, doi:10.7414/EUC-EFEHR-TR002-ESRM20; CC BY 4.0).
//! Bina stoku: ülke başına idari birim düzeyinde, Natural Earth Admin-1
//! bölgelerine koordinatla toplanmış (`seismira-data exposure`). Kırılganlık:
//! `fragility_ESRM20_Various_IM.xml`, dört sınır durumu (hafif, orta, ağır,
//! göçme), log-normal; her bina tipinin kendi şiddet ölçüsü (PGA, SA 0,3 /
//! 0,6 / 1,0 s), g biriminde.
//!
//! Hesap (Bölüm 7.8): bir sahada şiddet ölçüsü log-normal dağılır (GMPE'nin
//! medyanı ve olay içi sapması φ = √(σ² − τ²)); kırılganlık eğrisi de
//! log-normal. İkisinin birleşimi kapalı biçimdedir:
//! P(DS ≥ k) = Φ((ln IM_med − ln θ_k) / √(β² + φ²)). "Olası aralık",
//! olaylar arası terimin ±1τ kaydırılmasıyla (ln IM_med ± τ) verilir.
//!
//! Biçim (`.sexp`, little-endian):
//! ```text
//! "SEXP" u32 sürüm  u32 tip sayısı
//! her tip: u16 ad uzunluğu, ad (UTF-8), u8 şiddet ölçüsü (0 PGA, 1 SA0.3,
//!          2 SA0.6, 3 SA1.0), 3 bayt boşluk, f32 medyan ×4 (g), f32 β,
//!          f32 hasarsızlık sınırı (g)
//! u32 bölge sayısı
//! her bölge: u16 bölge kimliği, u32 yerleşim hücresi sayısı (bölge
//!            rasterında BUILT_BIT'li 30″ hücre), u16 kayıt sayısı,
//!            kayıtlar: u16 tip sırası, f32 bina sayısı (konut + ticari +
//!            sanayi)
//! ```

use crate::gmpe::Imt;
use std::io::{self, Read, Write};

const MAGIC: &[u8; 4] = b"SEXP";
const VERSION: u32 = 2;

/// Hasar durumları (ESRM20 sınır durumları): hafif, orta, ağır, göçme.
pub const N_DS: usize = 4;

/// Kırılganlık eğrisinin şiddet ölçüsü.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum FragImt {
    Pga = 0,
    Sa03 = 1,
    Sa06 = 2,
    Sa10 = 3,
}

impl FragImt {
    pub const ALL: [FragImt; 4] = [FragImt::Pga, FragImt::Sa03, FragImt::Sa06, FragImt::Sa10];

    pub fn imt(self) -> Imt {
        match self {
            FragImt::Pga => Imt::Pga,
            FragImt::Sa03 => Imt::Sa(0.3),
            FragImt::Sa06 => Imt::Sa(0.6),
            FragImt::Sa10 => Imt::Sa(1.0),
        }
    }

    /// ESRM20 / OpenQuake adı ("PGA", "SA(0.3)" ...).
    pub fn parse(s: &str) -> Option<FragImt> {
        match s {
            "PGA" => Some(FragImt::Pga),
            "SA(0.3)" => Some(FragImt::Sa03),
            "SA(0.6)" => Some(FragImt::Sa06),
            "SA(1.0)" => Some(FragImt::Sa10),
            _ => None,
        }
    }

    fn from_u8(v: u8) -> Option<FragImt> {
        FragImt::ALL.get(usize::from(v)).copied()
    }
}

/// Bir bina tipinin kırılganlık eğrisi.
#[derive(Clone, Debug, PartialEq)]
pub struct Fragility {
    /// ESRM20 tip adı (ör. "CR_LFINF-CDM-15_H4").
    pub name: String,
    pub imt: FragImt,
    /// Hasar durumu başına medyan (g).
    pub median: [f32; N_DS],
    /// Log standart sapma.
    pub beta: f32,
    /// Bu değerin altında hasar yok (g; OpenQuake `noDamageLimit`).
    pub no_damage: f32,
}

impl Fragility {
    /// OpenQuake `logncdf` parametrelerinden (doğrusal ortalama ve standart
    /// sapma) medyan ve β: β² = ln(1 + (s/m)²), medyan = m / √(1 + (s/m)²).
    pub fn from_mean_stddev(mean: f64, stddev: f64) -> (f64, f64) {
        let v = 1.0 + (stddev / mean).powi(2);
        (mean / v.sqrt(), v.ln().sqrt())
    }

    /// P(DS ≥ k), k = hafif..göçme; şiddet ölçüsü log-normal (medyanın ln'i
    /// `ln_im`, olay içi sapma `phi`).
    ///
    /// Hasarsızlık sınırı L (`noDamageLimit`): NRML tanımıyla şiddet L'nin
    /// altındaysa hasar olasılığı sıfır; OpenQuake bunu her yer hareketi
    /// değerine uygular. Analitik karşılığı (şiddetin dağılımı üzerinden):
    ///
    /// ```text
    /// P(DS ≥ k) = ∫_{ln L}^{∞} Φ((x − ln m_k) / β) · N(x; μ, φ²) dx
    /// ```
    /// L dağılımın çok altındaysa (z = (ln L − μ)/φ < −4,5) bu, kapalı biçim
    /// Φ((μ − ln m_k) / √(β² + φ²))'ye eşittir; çok üstündeyse (z > 4,5)
    /// sıfırdır (ikisinde de hata ≤ 1 − Φ(4,5) = 3,4·10⁻⁶); arada iki
    /// değişkenli normal dağılımla ([`truncated_exceedance`]).
    /// 2026-11-27'ye kadar sınır medyana uygulanıyordu (μ < ln L ise sıfır):
    /// L'si medyana yakın tiplerde sınırda P(hafif+) 0 ↔ 0,15 sıçraması
    /// (docs/bilim.md 7.8).
    pub fn exceedance(&self, ln_im: f64, phi: f64) -> [f64; N_DS] {
        let a = f64::from(self.no_damage).ln();
        let beta = f64::from(self.beta);
        if phi <= 0.0 {
            if ln_im < a {
                return [0.0; N_DS];
            }
            return self
                .median
                .map(|m| normal_cdf((ln_im - f64::from(m).ln()) / beta));
        }
        let za = (a - ln_im) / phi;
        if za > ZA_CUT {
            return [0.0; N_DS];
        }
        if za < -ZA_CUT {
            let s = (beta * beta + phi * phi).sqrt();
            return self
                .median
                .map(|m| normal_cdf((ln_im - f64::from(m).ln()) / s));
        }
        truncated_exceedance(
            za,
            self.median.map(|m| (ln_im - f64::from(m).ln()) / beta),
            phi / beta,
        )
    }
}

/// Bir bölgenin bina karışımı: (tip sırası, bina sayısı).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RegionStock {
    pub id: u16,
    /// Bölgenin tüm yerleşim hücreleri (30″). Bina stoku bu hücrelere
    /// eşit dağılmış kabul edilir; sarsıntı alanı bölgenin bir kısmını
    /// kapsıyorsa kalan hücreler hasarsız sayılır.
    pub built_cells: u32,
    pub entries: Vec<(u16, f32)>,
}

impl RegionStock {
    pub fn buildings(&self) -> f64 {
        self.entries.iter().map(|e| f64::from(e.1)).sum()
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Exposure {
    pub fragility: Vec<Fragility>,
    /// Kimliğe göre sıralı.
    pub regions: Vec<RegionStock>,
}

/// Hasar durumlarına göre beklenen bina sayısı (DS ≥ k değil, TAM o durum:
/// hafif, orta, ağır, göçme).
pub type Damage = [f64; N_DS];

impl Exposure {
    pub fn region(&self, id: u16) -> Option<&RegionStock> {
        self.regions
            .binary_search_by_key(&id, |r| r.id)
            .ok()
            .map(|i| &self.regions[i])
    }

    /// Bölgenin hasar dağılımı (bina sayısı). `ln_im(imt)` sahadaki
    /// şiddet ölçüsünün ln medyanı ve olay içi sapması; `None` ise o ölçü
    /// yok (model dışı) ve o tipler hasarsız sayılır.
    pub fn damage(
        &self,
        stock: &RegionStock,
        ln_im: impl Fn(FragImt) -> Option<(f64, f64)>,
    ) -> Damage {
        let ims = FragImt::ALL.map(&ln_im);
        let mut out = [0.0; N_DS];
        for &(t, n) in &stock.entries {
            let f = &self.fragility[usize::from(t)];
            let Some((mu, phi)) = ims[f.imt as usize] else {
                continue;
            };
            let p = f.exceedance(mu, phi);
            for k in 0..N_DS {
                let next = if k + 1 < N_DS { p[k + 1] } else { 0.0 };
                out[k] += f64::from(n) * (p[k] - next).max(0.0);
            }
        }
        out
    }

    pub fn write(&self, w: &mut impl Write) -> io::Result<()> {
        w.write_all(MAGIC)?;
        w.write_all(&VERSION.to_le_bytes())?;
        w.write_all(&(self.fragility.len() as u32).to_le_bytes())?;
        for f in &self.fragility {
            let b = f.name.as_bytes();
            w.write_all(
                &u16::try_from(b.len())
                    .map_err(|_| bad("ad uzun"))?
                    .to_le_bytes(),
            )?;
            w.write_all(b)?;
            w.write_all(&[f.imt as u8, 0, 0, 0])?;
            for v in f.median.iter().chain([&f.beta, &f.no_damage]) {
                w.write_all(&v.to_le_bytes())?;
            }
        }
        w.write_all(&(self.regions.len() as u32).to_le_bytes())?;
        for r in &self.regions {
            w.write_all(&r.id.to_le_bytes())?;
            w.write_all(&r.built_cells.to_le_bytes())?;
            let n = u16::try_from(r.entries.len()).map_err(|_| bad("kayıt çok"))?;
            w.write_all(&n.to_le_bytes())?;
            for &(t, b) in &r.entries {
                w.write_all(&t.to_le_bytes())?;
                w.write_all(&b.to_le_bytes())?;
            }
        }
        Ok(())
    }

    pub fn read(r: &mut impl Read) -> io::Result<Self> {
        let mut b4 = [0u8; 4];
        r.read_exact(&mut b4)?;
        if &b4 != MAGIC {
            return Err(bad("başlık yok"));
        }
        let u32_ = |r: &mut dyn Read| -> io::Result<u32> {
            let mut b = [0u8; 4];
            r.read_exact(&mut b)?;
            Ok(u32::from_le_bytes(b))
        };
        let u16_ = |r: &mut dyn Read| -> io::Result<u16> {
            let mut b = [0u8; 2];
            r.read_exact(&mut b)?;
            Ok(u16::from_le_bytes(b))
        };
        let f32_ = |r: &mut dyn Read| -> io::Result<f32> {
            let mut b = [0u8; 4];
            r.read_exact(&mut b)?;
            Ok(f32::from_le_bytes(b))
        };
        if u32_(r)? != VERSION {
            return Err(bad("desteklenmeyen sürüm"));
        }
        let n = u32_(r)? as usize;
        let mut fragility = Vec::with_capacity(n);
        for _ in 0..n {
            let len = usize::from(u16_(r)?);
            let mut name = vec![0u8; len];
            r.read_exact(&mut name)?;
            r.read_exact(&mut b4)?;
            let imt = FragImt::from_u8(b4[0]).ok_or_else(|| bad("şiddet ölçüsü"))?;
            let mut median = [0f32; N_DS];
            for m in &mut median {
                *m = f32_(r)?;
            }
            fragility.push(Fragility {
                name: String::from_utf8(name).map_err(|_| bad("ad UTF-8 değil"))?,
                imt,
                median,
                beta: f32_(r)?,
                no_damage: f32_(r)?,
            });
        }
        let n = u32_(r)? as usize;
        let mut regions = Vec::with_capacity(n);
        for _ in 0..n {
            let id = u16_(r)?;
            let built_cells = u32_(r)?;
            let k = usize::from(u16_(r)?);
            let mut entries = Vec::with_capacity(k);
            for _ in 0..k {
                let t = u16_(r)?;
                if usize::from(t) >= fragility.len() {
                    return Err(bad("tip sırası dışarıda"));
                }
                entries.push((t, f32_(r)?));
            }
            regions.push(RegionStock {
                id,
                built_cells,
                entries,
            });
        }
        if !regions.is_sorted_by_key(|r: &RegionStock| r.id) {
            return Err(bad("bölgeler sıralı değil"));
        }
        Ok(Self { fragility, regions })
    }
}

fn bad(msg: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, format!("sexp: {msg}"))
}

/// Standart normal dağılım fonksiyonu Φ(x) = ½·erfc(−x/√2). erfc: Numerical
/// Recipes in C, 2. baskı (Press vd. 1992), Bölüm 6.2, `erfcc` (Chebyshev
/// yaklaşımı): erfc(z) ≈ t·exp(−z² − 1,26551223 + t·(1,00002368 + …)),
/// t = 1/(1 + z/2); kitaba göre her z için bağıl hata < 1,2·10⁻⁷.
/// Katsayılar test `normal_dagilim` ile Abramowitz & Stegun (1964) Tablo
/// 26.1 değerlerine karşı denetlenir. Hasar olasılığı için fazlasıyla yeterli.
/// Kesik integralin atlandığı eşik (standart sapma); hata ≤ 1 − Φ(4,5).
const ZA_CUT: f64 = 4.5;

/// ∫_{za}^{∞} n(z) · Φ(c + r·z) dz (n: standart normal yoğunluk):
/// [`Fragility::exceedance`]'ın kesik integrali (z = (x − μ)/φ,
/// c = (μ − ln m)/β, r = φ/β).
///
/// Z₁, Z₂ bağımsız standart normal ise Φ(c + r·z) = P(Z₂ ≤ c + r·z), yani
/// integral P(Z₁ ≥ za, Z₂ − r·Z₁ ≤ c) = Φ₂(h, k; ρ) ile h = −za,
/// k = c/√(1 + r²), ρ = r/√(1 + r²). İki değişkenli normal dağılım,
/// ρ üzerinden türevle elde edilen standart özdeşlikle (Drezner &
/// Wesolowsky 1990 ve Genz 2004'ün kullandığı biçim; makaleler bu oturumda
/// GÖRÜLMEDİ, özdeşlik test `hasarsizlik_siniri_integrali`de doğrudan
/// sayısal integrale karşı doğrulanır):
///
/// ```text
/// Φ₂(h, k; ρ) = Φ(h)·Φ(k) + (1/2π) ∫₀^{asin ρ} exp(−(h² + k² − 2hk·sin θ) / (2cos² θ)) dθ
/// ```
/// θ integrali 8 noktalı Gauss–Legendre ile. ESRM20'de β 0,32–0,74, φ ≈ 0,6
/// → ρ ≤ 0,9; test `hasarsizlik_siniri_integrali` β 0,25–0,9 için ince
/// Simpson integraliyle karşılaştırır (fark < 2·10⁻⁵).
fn truncated_exceedance(za: f64, c: [f64; N_DS], r: f64) -> [f64; N_DS] {
    static GL: std::sync::LazyLock<Vec<(f64, f64)>> =
        std::sync::LazyLock::new(|| gauss_legendre(8));
    let q = (1.0 + r * r).sqrt();
    let (h, rho) = (-za, r / q);
    let top = rho.asin();
    // θ düğümleri tüm hasar durumlarında ortak: sin θ ve 1/(2cos²θ) bir kez.
    let mut nodes = [(0.0, 0.0, 0.0); 8];
    for (n, &(x, w)) in nodes.iter_mut().zip(GL.iter()) {
        let (sn, cs) = (0.5 * top * (x + 1.0)).sin_cos();
        *n = (w, sn, 0.5 / (cs * cs));
    }
    let ph = normal_cdf(h);
    let scale = 0.5 * top / (2.0 * std::f64::consts::PI);
    c.map(|c| {
        let k = c / q;
        let hk = h * h + k * k;
        let sum: f64 = nodes
            .iter()
            .map(|&(w, sn, inv)| w * (-(hk - 2.0 * h * k * sn) * inv).exp())
            .sum();
        (ph * normal_cdf(k) + scale * sum).clamp(0.0, 1.0)
    })
}

/// n noktalı Gauss–Legendre düğümleri ve ağırlıkları, [−1, 1]: Legendre
/// polinomunun kökleri Newton yöntemiyle (başlangıç cos(π(i − ¼)/(n + ½))),
/// ağırlık 2 / ((1 − x²) P'ₙ(x)²) (Press vd. 1992, Bölüm 4.5, `gauleg`).
fn gauss_legendre(n: usize) -> Vec<(f64, f64)> {
    let mut out = Vec::with_capacity(n);
    for i in 1..=n {
        let mut x = (std::f64::consts::PI * (i as f64 - 0.25) / (n as f64 + 0.5)).cos();
        let mut dp = 0.0;
        for _ in 0..100 {
            // Üç terimli yineleme: P_k.
            let (mut p0, mut p1) = (1.0, x);
            for k in 2..=n {
                let k = k as f64;
                let p2 = ((2.0 * k - 1.0) * x * p1 - (k - 1.0) * p0) / k;
                p0 = p1;
                p1 = p2;
            }
            let pn = if n == 1 { x } else { p1 };
            let pm = if n == 1 { 1.0 } else { p0 };
            dp = n as f64 * (x * pn - pm) / (x * x - 1.0);
            let dx = pn / dp;
            x -= dx;
            if dx.abs() < 1e-15 {
                break;
            }
        }
        out.push((x, 2.0 / ((1.0 - x * x) * dp * dp)));
    }
    out
}

pub fn normal_cdf(x: f64) -> f64 {
    0.5 * erfc(-x / std::f64::consts::SQRT_2)
}

fn erfc(x: f64) -> f64 {
    let z = x.abs();
    let t = 1.0 / (1.0 + 0.5 * z);
    let poly = -z * z - 1.265_512_23
        + t * (1.000_023_68
            + t * (0.374_091_96
                + t * (0.096_784_18
                    + t * (-0.186_288_06
                        + t * (0.278_868_07
                            + t * (-1.135_203_98
                                + t * (1.488_515_87 + t * (-0.822_152_23 + t * 0.170_872_77))))))));
    let r = t * poly.exp();
    if x >= 0.0 { r } else { 2.0 - r }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Gauss–Legendre: ağırlıklar toplamı 2, x⁴'ün integrali 2/5 (8 nokta
    /// 15. dereceye kadar tam).
    #[test]
    fn gauss_legendre_dugumleri() {
        let gl = gauss_legendre(8);
        assert_eq!(gl.len(), 8);
        let sum: f64 = gl.iter().map(|p| p.1).sum();
        assert!((sum - 2.0).abs() < 1e-13);
        let x4: f64 = gl.iter().map(|&(x, w)| w * x.powi(4)).sum();
        assert!((x4 - 0.4).abs() < 1e-13);
        let x15: f64 = gl.iter().map(|&(x, w)| w * x.powi(14)).sum();
        assert!((x15 - 2.0 / 15.0).abs() < 1e-12);
    }

    /// Kesik integral: ince Simpson'la aynı; sınır dağılımın çok altındayken
    /// kapalı biçime, çok üstündeyken sıfıra sürekli bağlanır; sınırda
    /// sıçrama yok.
    #[test]
    fn hasarsizlik_siniri_integrali() {
        let phi = 0.6;
        let simpson = |mu: f64, m: f64, beta: f64| {
            let (a, b, n) = ((0.05f64).ln(), mu + 10.0 * phi, 20_000);
            if b <= a {
                return 0.0;
            }
            let h = (b - a) / n as f64;
            let g = |x: f64| {
                normal_cdf((x - m.ln()) / beta) * (-0.5 * ((x - mu) / phi).powi(2)).exp()
                    / (phi * (2.0 * std::f64::consts::PI).sqrt())
            };
            (0..=n)
                .map(|i| {
                    let w = if i == 0 || i == n {
                        1.0
                    } else if i % 2 == 1 {
                        4.0
                    } else {
                        2.0
                    };
                    w * g(a + i as f64 * h)
                })
                .sum::<f64>()
                * h
                / 3.0
        };
        // β: ESRM20'de en küçük ~0,3 (ρ = r/√(1+r²) ≈ 0,89) ile 0,9 arası.
        for beta in [0.25, 0.3, 0.586, 0.9] {
            let f = Fragility {
                name: "T".into(),
                imt: FragImt::Pga,
                median: [0.105, 0.2, 0.35, 0.6],
                beta,
                no_damage: 0.05,
            };
            for k in -30..=40 {
                let mu = (0.05f64).ln() + 0.1 * k as f64;
                let p = f.exceedance(mu, phi);
                for (d, &m) in f.median.iter().enumerate() {
                    let want = simpson(mu, f64::from(m), f64::from(beta));
                    assert!(
                        (p[d] - want).abs() < 2e-5,
                        "β {beta} μ {mu} DS{d}: {} {want}",
                        p[d]
                    );
                }
            }
        }
        let f = Fragility {
            name: "T".into(),
            imt: FragImt::Pga,
            median: [0.105, 0.2, 0.35, 0.6],
            beta: 0.586,
            no_damage: 0.05,
        };
        // Kollar arası süreklilik (z = ±4,5 eşikleri; sıçrama ≤ 3,4·10⁻⁶).
        let at = |z: f64| f.exceedance((0.05f64).ln() - z * phi, phi)[0];
        assert!((at(-ZA_CUT + 1e-9) - at(-ZA_CUT - 1e-9)).abs() < 3.5e-6);
        assert!(at(ZA_CUT - 1e-9) < 3.5e-6 && at(ZA_CUT + 1e-9) == 0.0);
        // Eski sıçrama: medyan sınırın hemen altında artık sıfır değil.
        assert!(f.exceedance((0.0499f64).ln(), phi)[0] > 0.1);
        // φ = 0: sınır doğrudan uygulanır.
        assert_eq!(f.exceedance((0.049f64).ln(), 0.0), [0.0; N_DS]);
        assert!(f.exceedance((0.2f64).ln(), 0.0)[1] > 0.49);
    }

    #[test]
    fn normal_dagilim() {
        // Tablo değerleri (Abramowitz & Stegun 1964, Tablo 26.1).
        for (x, want) in [
            (0.0, 0.5),
            (1.0, 0.841_344_746),
            (-1.0, 0.158_655_254),
            (1.96, 0.975_002_105),
            (-2.5, 0.006_209_665),
            (3.0, 0.998_650_102),
        ] {
            assert!(
                (normal_cdf(x) - want).abs() < 2e-7,
                "Φ({x}) = {}",
                normal_cdf(x)
            );
        }
    }

    /// XML (doğrusal ortalama, standart sapma) → medyan, β; ESRM20 xlsx
    /// "Final" sayfasındaki değerlerle (CR_LDUAL-DUH_H1, PGA, DS1:
    /// medyan 1,388092683636863, β 0,3594254642489141).
    #[test]
    fn ortalama_sapmadan_medyan_beta() {
        let (m, b) = Fragility::from_mean_stddev(1.481, 0.55);
        assert!((m - 1.388_092_683_6).abs() < 2e-3, "{m}");
        assert!((b - 0.359_425_464_2).abs() < 2e-3, "{b}");
    }

    fn sample() -> Exposure {
        Exposure {
            fragility: vec![Fragility {
                name: "T".into(),
                imt: FragImt::Sa03,
                median: [0.2, 0.4, 0.8, 1.6],
                beta: 0.5,
                no_damage: 0.05,
            }],
            regions: vec![RegionStock {
                id: 7,
                built_cells: 10,
                entries: vec![(0, 1000.0)],
            }],
        }
    }

    #[test]
    fn hasar_dagilimi() {
        let e = sample();
        let s = e.region(7).unwrap();
        // IM tam ağır hasar medyanında, belirsizlik yok: ağır ve üstü %50.
        let d = e.damage(s, |imt| {
            (imt == FragImt::Sa03).then_some((0.8f64.ln(), 0.0))
        });
        let heavy_plus = d[2] + d[3];
        assert!((heavy_plus - 500.0).abs() < 1e-3, "{d:?}");
        // Toplam, DS ≥ hafif olasılığı × bina sayısı.
        let p1 = normal_cdf((0.8f64 / 0.2).ln() / 0.5);
        assert!((d.iter().sum::<f64>() - 1000.0 * p1).abs() < 1e-6);
        // Hasarsızlık sınırının altında belirsizlik yoksa sıfır; belirsizlikle
        // yalnızca şiddetin sınırı aşan kuyruğu kadar (1000 binada < 0,1);
        // şiddet ölçüsü yoksa sıfır.
        let none = e.damage(s, |_| Some((0.01f64.ln(), 0.0)));
        assert_eq!(none, [0.0; N_DS]);
        let tail = e.damage(s, |_| Some((0.01f64.ln(), 0.6)));
        assert!(tail.iter().sum::<f64>() < 0.1, "{tail:?}");
        assert_eq!(e.damage(s, |_| None), [0.0; N_DS]);
        // Şiddet arttıkça ağır hasar artar.
        let lo = e.damage(s, |_| Some((0.3f64.ln(), 0.6)));
        let hi = e.damage(s, |_| Some((0.6f64.ln(), 0.6)));
        assert!(hi[2] + hi[3] > lo[2] + lo[3]);
    }

    /// Depodaki gerçek dosya (`seismira-data exposure`): 44 ülke, 1570
    /// bölge, 142,6 milyon bina; Türkiye 9,16 milyon (81 il).
    #[test]
    fn esrm20_dosyasi() {
        let p = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/exposure/esrm20.sexp"
        );
        let e = Exposure::read(&mut std::io::BufReader::new(
            std::fs::File::open(p).unwrap(),
        ))
        .unwrap();
        assert_eq!((e.regions.len(), e.fragility.len()), (1570, 511));
        let total: f64 = e.regions.iter().map(RegionStock::buildings).sum();
        assert!((total - 142_569_004.0).abs() < 2e3, "{total}");
        for f in &e.fragility {
            assert!(
                f.median.is_sorted() && f.beta > 0.0 && f.no_damage > 0.0,
                "{f:?}"
            );
        }
    }

    #[test]
    fn yaz_oku() {
        let e = sample();
        let mut buf = Vec::new();
        e.write(&mut buf).unwrap();
        assert_eq!(Exposure::read(&mut buf.as_slice()).unwrap(), e);
    }
}
