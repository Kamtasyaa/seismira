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
    /// `ln_im`, olay içi sapma `phi`). Medyan hasarsızlık sınırının altındaysa
    /// sıfır (OpenQuake bu sınırı tek değere uygular; burada medyana).
    pub fn exceedance(&self, ln_im: f64, phi: f64) -> [f64; N_DS] {
        if ln_im < f64::from(self.no_damage).ln() {
            return [0.0; N_DS];
        }
        let s = (f64::from(self.beta).powi(2) + phi * phi).sqrt();
        self.median
            .map(|m| normal_cdf((ln_im - f64::from(m).ln()) / s))
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
        // Hasarsızlık sınırının altında sıfır; şiddet ölçüsü yoksa sıfır.
        let none = e.damage(s, |_| Some((0.01f64.ln(), 0.6)));
        assert_eq!(none, [0.0; N_DS]);
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
