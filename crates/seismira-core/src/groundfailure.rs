//! Deprem kaynaklı zemin yenilmesi: sıvılaşma (Zhu vd. 2017) ve heyelan
//! (Nowicki Jessee vd. 2018) olasılık modelleri (CLAUDE.md Bölüm 7.9,
//! docs/bilim.md 7.9).
//!
//! Katsayılar, kırpmalar ve maskeler: USGS groundfailure referans
//! uygulaması (code.usgs.gov/ghsc/esi/groundfailure/groundfailure, kamu
//! malı, commit fa7bd289, 2026-08-27): `src/gfail/models/zhu_2017.py`,
//! `zhu_2017_coastal.py`, `jessee_2018.py`, `logbase.py` ve
//! `defaultconfigfiles/models/*.ini`. Jessee katsayıları makalenin
//! Tablo 3'üyle karşılaştırıldı (docs/bilim.md).
//!
//! İki model de lojistik: X = Σ bᵢ·terimᵢ, P = 1 / (1 + e^(−X)); P sonra
//! "alansal kaplama"ya (hücre alanının etkilenen oranı) çevrilir. USGS'in
//! gösterdiği (ve bizim haritada gösterdiğimiz) değer alansal kaplamadır.
//! Sıra USGS'teki gibi: P → sarsıntı/Vs30 maskeleri → kaplama → eğim
//! maskesi (çarpan).

/// Zhu, Baise & Thompson (2017), "An Updated Geospatial Liquefaction Model
/// for Global Application", BSSA 107(3), 1365–1385, doi:10.1785/0120160198.
pub mod zhu2017 {
    /// Genel model yeraltı su seviyesi (wtd) ister; kıyı modeli istemez
    /// (wtd verisi olmadığında yedek, docs/bilim.md 7.9).
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Variant {
        General,
        Coastal,
    }

    /// Bir hücrenin girdileri.
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct Inputs {
        /// Medyan PGV (cm/s) ve PGA (g).
        pub pgv_cms: f64,
        pub pga_g: f64,
        pub mw: f64,
        /// Haritanın Vs30'u (m/s; yer hareketi modelinin sınırına
        /// çekilmemiş değer: USGS de ham Vs30 katmanını kullanır).
        pub vs30: f64,
        /// Yıllık yağış (mm), kıyıya ve nehre uzaklık (km).
        pub precip_mm: f64,
        pub dc_km: f64,
        pub dr_km: f64,
        /// Yeraltı su seviyesi derinliği (m); yalnızca genel model.
        pub wtd_m: f64,
    }

    /// Eğim maskesi: eğim ≤ 5° (ini: slopemin 0, slopemax 5).
    pub const SLOPE_MAX_DEG: f64 = 5.0;
    /// Sarsıntı ve zemin maskeleri (ini): PGV ≥ 3 cm/s, PGA ≥ %10 g,
    /// Vs30 ≤ 620 m/s.
    pub const MIN_PGV_CMS: f64 = 3.0;
    pub const MIN_PGA_G: f64 = 0.10;
    pub const MAX_VS30: f64 = 620.0;

    /// Büyüklük ölçek çarpanı SF = 1 / (1 + e^(−2(M − 6))). USGS büyüklüğü
    /// tek ondalığa yuvarlar ve e yerine 2.71828 yazar (fark ~1e−6,
    /// ihmal edilir). Döndürülen: ln(PGV·SF), PGV 0–150 cm/s'ye kırpılır.
    pub fn ln_pgv_sf(pgv_cms: f64, mw: f64) -> f64 {
        let m = (mw * 10.0).round() / 10.0;
        let sf = 1.0 / (1.0 + (-2.0 * (m - 6.0)).exp());
        (pgv_cms.clamp(0.0, 150.0) * sf).ln()
    }

    /// ln(PGV·SF) teriminin katsayısı.
    pub fn pgv_coef(v: Variant) -> f64 {
        match v {
            Variant::General => 0.334,
            Variant::Coastal => 0.301,
        }
    }

    /// Lojistik değişkenin sarsıntıdan BAĞIMSIZ kısmı (sabit + zemin
    /// terimleri; `pgv_cms`, `pga_g`, `mw` kullanılmaz). Hücre başına bir kez
    /// önceden hesaplanabilir (paket boyutu, docs/bilim.md 7.9).
    pub fn static_logit(v: Variant, i: &Inputs) -> f64 {
        let pr = i.precip_mm.clamp(0.0, 2500.0);
        let vs = i.vs30.ln();
        match v {
            Variant::General => {
                8.801 - 1.918 * vs + 0.0005408 * pr
                    - 0.2054 * i.dc_km.min(i.dr_km)
                    - 0.0333 * i.wtd_m
            }
            Variant::Coastal => {
                let sdc = i.dc_km.sqrt();
                12.435 - 2.615 * vs + 0.0005556 * pr - 0.0287 * sdc + 0.0666 * i.dr_km
                    - 0.0369 * sdc * i.dr_km
            }
        }
    }

    /// Lojistik değişken X = statik kısım + katsayı·ln(PGV·SF). Kırpma
    /// (USGS CLIPS): yağış 0–2500 mm, PGV 0–150 cm/s.
    pub fn logit(v: Variant, i: &Inputs) -> f64 {
        static_logit(v, i) + pgv_coef(v) * ln_pgv_sf(i.pgv_cms, i.mw)
    }

    /// Olasılık P; maskelenen hücrede (sarsıntı eşiği altında ya da
    /// Vs30 > 620 m/s) `None` = tehlike yok.
    pub fn probability(v: Variant, i: &Inputs) -> Option<f64> {
        if i.pgv_cms < MIN_PGV_CMS || i.pga_g < MIN_PGA_G || i.vs30 > MAX_VS30 {
            return None;
        }
        Some(1.0 / (1.0 + (-logit(v, i)).exp()))
    }

    /// Alansal kaplama (Zhu 2017; USGS COV_COEFFS): a / (1 + b·e^(−c·P))².
    pub fn coverage(v: Variant, p: f64) -> f64 {
        let (a, b, c) = match v {
            Variant::General => (0.4915, 42.4, 9.165),
            Variant::Coastal => (0.4208, 62.59, 11.43),
        };
        a / (1.0 + b * (-c * p).exp()).powi(2)
    }

    /// Hücrenin alansal kaplaması: `flat_fraction` = hücrede eğimi ≤ 5°
    /// olan alan oranı (0–1).
    pub fn cell_coverage(v: Variant, i: &Inputs, flat_fraction: f64) -> f64 {
        probability(v, i).map_or(0.0, |p| coverage(v, p) * flat_fraction)
    }

    /// [`cell_coverage`] ile aynı, ama zemin kısmı önceden hesaplanmış
    /// ([`static_logit`]; Vs30 maskesi o aşamada uygulanmış olmalı).
    pub fn cell_coverage_static(
        v: Variant,
        x_static: f64,
        pgv_cms: f64,
        pga_g: f64,
        mw: f64,
        flat_fraction: f64,
    ) -> f64 {
        if pgv_cms < MIN_PGV_CMS || pga_g < MIN_PGA_G {
            return 0.0;
        }
        let x = x_static + pgv_coef(v) * ln_pgv_sf(pgv_cms, mw);
        coverage(v, 1.0 / (1.0 + (-x).exp())) * flat_fraction
    }
}

/// Nowicki Jessee, M.A. vd. (2018), "A Global Empirical Model for Near-Real-
/// Time Assessment of Seismically Induced Landslides", JGR Earth Surface
/// 123, 1835–1859, doi:10.1029/2017JF004494. Eş. 8 ve 9, Tablo 3.
pub mod jessee2018 {
    /// Eğim maskesi: eğim > 2° (ini: slopemin 2, slopemax 90).
    pub const SLOPE_MIN_DEG: f64 = 2.0;
    /// Sarsıntı maskesi: PGA ≥ %2 g (ini; Jibson & Harp 2016).
    pub const MIN_PGA_G: f64 = 0.02;

    /// Lojistik değişken X (Eş. 8). `rock` ve `land` sınıf katsayıları
    /// ([`lithology_coef`], [`land_cover_coef`]). Kırpma (USGS): PGV
    /// 0–211 cm/s, CTI 0–19.
    pub fn logit(pgv_cms: f64, slope_deg: f64, rock: f64, cti: f64, land: f64) -> f64 {
        let l = pgv_cms.clamp(0.0, 211.0).ln();
        -6.30
            + 1.65 * l
            + 0.06 * slope_deg
            + rock
            + 0.03 * cti.clamp(0.0, 19.0)
            + land
            + 0.01 * l * slope_deg
    }

    /// Alansal kaplama (Eş. 9): exp(−7.592 + 5.237P − 3.042P² + 4.035P³).
    pub fn coverage(p: f64) -> f64 {
        (-7.592 + 5.237 * p - 3.042 * p * p + 4.035 * p * p * p).exp()
    }

    /// Bir alt alanın kaplaması; eğim ≤ 2° ya da PGA < %2 g ise 0.
    pub fn sub_coverage(
        pgv_cms: f64,
        pga_g: f64,
        slope_deg: f64,
        rock: f64,
        cti: f64,
        land: f64,
    ) -> f64 {
        if pga_g < MIN_PGA_G || slope_deg <= SLOPE_MIN_DEG {
            return 0.0;
        }
        let x = logit(pgv_cms, slope_deg, rock, cti, land);
        coverage(1.0 / (1.0 + (-x).exp()))
    }

    /// 30″ hücrenin kaplaması: 7,5″ eğim çeyreklerinin (her biri hücrenin
    /// dörtte biri, docs/bilim.md 7.9) ortalaması.
    pub fn cell_coverage(
        pgv_cms: f64,
        pga_g: f64,
        slopes_deg: [f64; 4],
        rock: f64,
        cti: f64,
        land: f64,
    ) -> f64 {
        slopes_deg
            .iter()
            .map(|&s| sub_coverage(pgv_cms, pga_g, s, rock, cti, land))
            .sum::<f64>()
            / 4.0
    }

    /// GLiM üst sınıfları (Hartmann & Moosdorf 2012), USGS dosyasındaki sıra
    /// (alfabetik, 1'den): 1 ev … 16 wb.
    pub const GLIM_CLASSES: [&str; 16] = [
        "ev", "ig", "mt", "nd", "pa", "pb", "pi", "py", "sc", "sm", "ss", "su", "va", "vb", "vi",
        "wb",
    ];

    /// Pekişmemiş tortul (su): makale −3.22; USGS işletimde −1.36 kullanır
    /// (jessee_2018.py, "OLD_UNCONSOLIDATED"/"NEW_UNCONSOLIDATED"). Seçim:
    /// USGS değeri (docs/bilim.md 7.9).
    pub const UNCONSOLIDATED_USGS: f64 = -1.36;

    /// Litoloji katsayısı (Tablo 3), GLiM sınıf numarası (1–16). Tabloda
    /// olmayan ev (evaporit, R'de referans düzey) ve ig (buzul) = 0
    /// (VARSAYIM, docs/bilim.md). Su kütlesi (wb) ve bilinmeyen kod: `None`
    /// (model uygulanmaz).
    pub fn lithology_coef(class: u8) -> Option<f64> {
        Some(
            match GLIM_CLASSES.get(usize::from(class).checked_sub(1)?)? {
                &"ev" | &"ig" => 0.0,
                &"mt" => -1.87,
                &"nd" => -0.66,
                &"pa" => -0.78,
                &"pb" => -1.88,
                &"pi" => -1.61,
                &"py" => -1.05,
                &"sc" => -0.95,
                &"sm" => -1.36,
                &"ss" => -1.92,
                &"su" => UNCONSOLIDATED_USGS,
                &"va" => -1.54,
                &"vb" => -1.50,
                &"vi" => -0.81,
                _ => return None,
            },
        )
    }

    /// Arazi örtüsü katsayısı (Tablo 3), GlobCover 2009 sınıf kodu.
    /// Tabloda olmayan 11 (referans düzey) ve 170 = 0; 210 (su) ve
    /// bilinmeyen kod: `None`.
    pub fn land_cover_coef(class: u8) -> Option<f64> {
        Some(match class {
            11 | 170 => 0.0,
            14 => 0.91,
            20 => 0.88,
            30 => 0.78,
            40 => 0.68,
            50 => 0.30,
            60 => 1.77,
            70 => 1.71,
            90 => -1.26,
            100 => 1.50,
            110 => 0.68,
            120 => 1.13,
            130 => 0.79,
            140 => 1.03,
            150 => 0.54,
            160 => 2.34,
            180 => 1.19,
            190 => 0.30,
            200 => -0.06,
            220 => -0.18,
            230 => -1.08,
            _ => return None,
        })
    }

    /// ESA CCI Land Cover (v2.0.7, PUG v2 Tablo 3-1) sınıfının karşılık
    /// geldiği GlobCover 2009 sınıf(lar)ı. İki lejant da UN-LCCS'den
    /// türetilmiş ama kodlar farklı; eşleme sınıf tanımlarının
    /// karşılaştırılmasıyla kuruldu (VARSAYIM, docs/bilim.md 7.9). İki sınıf
    /// dönerse CCI sınıfı GlobCover'da yoğunluğa göre ikiye bölünmüştür
    /// (CCI 60, 70 "kapalı-açık >%15"; GlobCover kapalı >%40 / açık
    /// %15–40): katsayıların ortalaması alınır ([`cci_land_cover_coef`]).
    /// `None`: su (210), veri yok (0), bilinmeyen kod.
    pub fn cci_to_globcover(cci: u8) -> Option<&'static [u8]> {
        Some(match cci {
            10..=12 => &[14],
            20 => &[11],
            30 => &[20],
            40 => &[30],
            50 => &[40],
            60 => &[50, 60],
            61 => &[50],
            62 => &[60],
            70 => &[70, 90],
            71 => &[70],
            // GlobCover'da kapalı iğne yapraklı yaprak döken sınıfı yok;
            // 90 "açık iğne yapraklı, yaprak döken ya da her dem yeşil".
            72 | 80..=82 => &[90],
            90 => &[100],
            100 => &[110],
            110 => &[120],
            120..=122 => &[130],
            // Liken ve yosun GlobCover'da yok; otsu bitki örtüsüne (140).
            130 | 140 => &[140],
            150..=153 => &[150],
            160 => &[160],
            170 => &[170],
            180 => &[180],
            190 => &[190],
            200..=202 => &[200],
            220 => &[220],
            _ => return None,
        })
    }

    /// CCI sınıfının Jessee arazi örtüsü katsayısı ([`cci_to_globcover`]
    /// sınıflarının [`land_cover_coef`] ortalaması).
    pub fn cci_land_cover_coef(cci: u8) -> Option<f64> {
        let gc = cci_to_globcover(cci)?;
        let mut sum = 0.0;
        for &g in gc {
            sum += land_cover_coef(g)?;
        }
        Some(sum / gc.len() as f64)
    }
}

/// Sıvılaşmanın zemin kısmının önceden hesaplanmış katmanları
/// (`data/ground/liq_static.sras`, Vs30 ızgarası; docs/bilim.md 7.9 "paket
/// boyutu"). Uygulamanın yedi ham girdi katmanı (~930 MB) yerine iki ya da
/// üç `u8` katmanı taşımasını sağlar.
///
/// - `liq_x`: [`zhu2017::static_logit`], doğrusal [`X_LO`]–[`X_HI`], 251
///   kod (adım 0,06). [`MASKED`] = model uygulanmaz (su, veri yok,
///   Vs30 > 620 m/s, hiç düz çeyrek yok).
/// - `liq_xf` (isteğe bağlı): ince düzeltme; `liq_x` en yakın kod olduğu
///   için ondan farkı ±0,5 kod, (kod − 128) / 254 olarak saklanır (16 bit).
/// - `liq_cls`: (düz çeyrek sayısı − 1) + 4·(kıyı modeli ise 1).
///
/// X_LO kırpması: statik X < −9 ise en güçlü sarsıntıda bile (PGV 150 cm/s,
/// +1,67) P < 6,3·10⁻⁴; kaplama tabandaki değerinden (2,6·10⁻⁴) en çok
/// %1,1 farklıdır. X_HI: kıyı modelinin analitik üst sınırı (~5,2) + pay.
pub mod static_layer {
    use super::zhu2017::Variant;
    use crate::raster::{LayerInfo, Scale};

    pub const FILE: &str = "liq_static.sras";
    pub const X_LO: f64 = -9.0;
    pub const X_HI: f64 = 6.0;
    const N_VALID: u8 = 251;
    pub const MASKED: u8 = 255;

    pub fn layers(fine: bool) -> Vec<LayerInfo> {
        let mut v = vec![LayerInfo {
            name: "liq_x".into(),
            scale: Scale::Linear,
            n_valid: N_VALID,
            lo: X_LO as f32,
            hi: X_HI as f32,
        }];
        if fine {
            v.push(LayerInfo {
                name: "liq_xf".into(),
                scale: Scale::Linear,
                n_valid: 255,
                lo: -0.5,
                hi: 0.5,
            });
        }
        v.push(LayerInfo {
            name: "liq_cls".into(),
            scale: Scale::Linear,
            n_valid: 8,
            lo: 0.0,
            hi: 7.0,
        });
        v
    }

    /// Kodlar (liq_x, liq_xf, liq_cls); `flat` 1–4. Maskeli hücre için
    /// [`MASKED`] üçlüsü kullanılır.
    pub fn encode(x: f64, flat: u8, v: Variant) -> [u8; 3] {
        debug_assert!((1..=4).contains(&flat));
        let n = f64::from(N_VALID - 1);
        let t = ((x - X_LO) / (X_HI - X_LO)).clamp(0.0, 1.0) * n;
        let c = t.round();
        let f = ((t - c) * 254.0).round() + 128.0;
        let cls = (flat - 1) + if v == Variant::Coastal { 4 } else { 0 };
        [c as u8, f.clamp(1.0, 255.0) as u8, cls]
    }

    /// (statik X, düz çeyrek sayısı, model); maskeli ya da bozuk kodda
    /// `None`. `fine` yoksa yalnızca kaba kod kullanılır.
    pub fn decode(x: u8, fine: Option<u8>, cls: u8) -> Option<(f64, u8, Variant)> {
        if x >= N_VALID || cls >= 8 {
            return None;
        }
        let mut t = f64::from(x);
        if let Some(f) = fine {
            t += (f64::from(f) - 128.0) / 254.0;
        }
        let xs = X_LO + t / f64::from(N_VALID - 1) * (X_HI - X_LO);
        let v = if cls >= 4 {
            Variant::Coastal
        } else {
            Variant::General
        };
        Some((xs, cls % 4 + 1, v))
    }
}

/// Sıvılaşmanın sarsıntı ızgarası üzerinde hesabı (CLAUDE.md Bölüm 2.6).
pub mod liquefaction_grid {
    use super::zhu2017::{self, Variant};
    use crate::grid::{Domain, GridSpec, NO_DATA, ShakeGrid};
    use crate::raster::LayerInfo;
    use rayon::prelude::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    /// Taban ızgarada (`domain.w × domain.h`, satır satır) bir katman
    /// penceresi: kodlar ve kod → değer eşlemesi.
    #[derive(Clone, Copy)]
    pub struct Layer<'a> {
        pub codes: &'a [u8],
        pub info: &'a LayerInfo,
    }

    impl Layer<'_> {
        fn at(&self, i: usize) -> Option<f64> {
            self.info.value(*self.codes.get(i)?).map(f64::from)
        }
    }

    /// Ham girdi pencereleri. Hepsi Vs30 ızgarasına hizalı (taban ızgara).
    /// Veri hazırlama ve duyarlılık denemeleri için; uygulama
    /// [`StaticInputs`] kullanır.
    pub struct RawInputs<'a> {
        /// Vs30 (m/s; su / buz NaN).
        pub vs30: &'a [f32],
        pub precip_mm: Layer<'a>,
        pub dc_km: Layer<'a>,
        pub dr_km: Layer<'a>,
        /// 7,5″ eğim çeyrekleri (derece).
        pub slope_q: [Layer<'a>; 4],
        /// Yeraltı su seviyesi (m); yoksa (ya da hücrede değer yoksa) kıyı
        /// modeli.
        pub wtd_m: Option<Layer<'a>>,
    }

    /// Önceden hesaplanmış zemin kısmı ([`super::static_layer`] kodları,
    /// taban ızgarada pencere).
    pub struct StaticInputs<'a> {
        pub x: &'a [u8],
        pub fine: Option<&'a [u8]>,
        pub cls: &'a [u8],
    }

    pub enum Inputs<'a> {
        Raw(RawInputs<'a>),
        Static(StaticInputs<'a>),
    }

    impl Inputs<'_> {
        fn len(&self) -> usize {
            match self {
                Inputs::Raw(r) => r.vs30.len(),
                Inputs::Static(s) => s.x.len(),
            }
        }

        /// Taban hücrenin (statik X, düz çeyrek, model); maskeli ya da girdi
        /// eksikse `None` (kaplama 0).
        fn base(&self, i: usize) -> Option<(f64, u8, Variant)> {
            match self {
                Inputs::Static(s) => super::static_layer::decode(
                    *s.x.get(i)?,
                    s.fine.and_then(|f| f.get(i).copied()),
                    *s.cls.get(i)?,
                ),
                Inputs::Raw(r) => raw_base(r, i),
            }
        }
    }

    /// Ham girdilerden statik kısım (veri aracı aynı işlevi kullanır, böylece
    /// önceden hesaplanan katman ile ham yol birebir aynı kalır).
    pub fn raw_base(r: &RawInputs, i: usize) -> Option<(f64, u8, Variant)> {
        let vs30 = f64::from(*r.vs30.get(i)?);
        if !vs30.is_finite() || vs30 > zhu2017::MAX_VS30 {
            return None;
        }
        let mut flat = 0u8;
        for q in &r.slope_q {
            if q.at(i)? <= zhu2017::SLOPE_MAX_DEG {
                flat += 1;
            }
        }
        if flat == 0 {
            return None;
        }
        let wtd = r.wtd_m.and_then(|l| l.at(i));
        let variant = if wtd.is_some() {
            Variant::General
        } else {
            Variant::Coastal
        };
        let x = zhu2017::Inputs {
            pgv_cms: 0.0,
            pga_g: 0.0,
            mw: 0.0,
            vs30,
            precip_mm: r.precip_mm.at(i)?,
            dc_km: r.dc_km.at(i)?,
            dr_km: r.dr_km.at(i)?,
            wtd_m: wtd.unwrap_or(0.0),
        };
        Some((zhu2017::static_logit(variant, &x), flat, variant))
    }

    /// Hücre başına alansal kaplama (0–1; maskeli ya da veri yok: 0).
    #[derive(Clone, Debug)]
    pub struct LiquefactionGrid {
        pub spec: GridSpec,
        /// Medyan sarsıntıyla.
        pub coverage: Vec<f32>,
        /// Sarsıntı (ln PGV, ln PGA) olaylar arası −1τ / +1τ kaydırılınca:
        /// bölge toplamlarının "olası aralığı".
        pub coverage_lo: Vec<f32>,
        pub coverage_hi: Vec<f32>,
        /// Genel / kıyı modeliyle hesaplanan (sıfırdan büyük) hücre sayısı.
        pub general: u32,
        pub coastal: u32,
    }

    /// `shake` ile aynı ızgarada hesaplar (aynı adım, aynı temsilci taban
    /// hücre: [`crate::grid::compute`] ile aynı kural). İptal: `None`.
    pub fn compute(
        shake: &ShakeGrid,
        domain: &Domain,
        step: u32,
        mw: f64,
        inputs: &Inputs,
        cancel: &AtomicBool,
    ) -> Option<LiquefactionGrid> {
        let spec = shake.spec;
        let n = domain.w as usize * domain.h as usize;
        assert_eq!(inputs.len(), n, "zemin penceresi alanla uyuşmuyor");
        let rows: Vec<Option<Vec<CellOut>>> = (0..spec.height)
            .into_par_iter()
            .map(|row| {
                if cancel.load(Ordering::Relaxed) {
                    return None;
                }
                let out = (0..spec.width)
                    .map(|col| {
                        let g = (row * spec.width + col) as usize;
                        if shake.class[g] == NO_DATA {
                            return ([0.0; 3], 0);
                        }
                        let bx = (col * step + step / 2).min(domain.w - 1) as usize;
                        let by = (row * step + step / 2).min(domain.h - 1) as usize;
                        let i = by * domain.w as usize + bx;
                        cell(shake, g, mw, inputs.base(i)).unwrap_or(([0.0; 3], 0))
                    })
                    .collect();
                Some(out)
            })
            .collect();
        let mut grid = LiquefactionGrid {
            spec,
            coverage: Vec::with_capacity((spec.width * spec.height) as usize),
            coverage_lo: Vec::with_capacity((spec.width * spec.height) as usize),
            coverage_hi: Vec::with_capacity((spec.width * spec.height) as usize),
            general: 0,
            coastal: 0,
        };
        for row in rows {
            for ([c, lo, hi], v) in row? {
                grid.coverage.push(c);
                grid.coverage_lo.push(lo);
                grid.coverage_hi.push(hi);
                if c > 0.0 {
                    match v {
                        1 => grid.general += 1,
                        2 => grid.coastal += 1,
                        _ => {}
                    }
                }
            }
        }
        Some(grid)
    }

    /// Bir hücrenin sonucu: ([medyan, −1τ, +1τ] kaplama, 1 genel / 2 kıyı).
    type CellOut = ([f32; 3], u8);

    /// Bir hücre ([`CellOut`]); maskeli ya da girdi eksikse `None`.
    fn cell(
        shake: &ShakeGrid,
        g: usize,
        mw: f64,
        base: Option<(f64, u8, Variant)>,
    ) -> Option<CellOut> {
        let (xs, flat, variant) = base?;
        let (pgv, pga) = (f64::from(shake.pgv[g]), f64::from(shake.pga[g]));
        if !(pgv.is_finite() && pga.is_finite()) {
            return None;
        }
        let (tv, ta) = (f64::from(shake.pgv_tau[g]), f64::from(shake.pga_tau[g]));
        let c = [0.0, -1.0, 1.0].map(|k: f64| {
            let (kv, ka) = if tv.is_finite() && ta.is_finite() {
                ((k * tv).exp(), (k * ta).exp())
            } else {
                (1.0, 1.0)
            };
            zhu2017::cell_coverage_static(
                variant,
                xs,
                pgv * kv,
                pga * ka,
                mw,
                f64::from(flat) / 4.0,
            ) as f32
        });
        Some((c, if variant == Variant::General { 1 } else { 2 }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) {
        assert!((a - b).abs() < tol, "{a} ≠ {b}");
    }

    // Beklenen değerler: USGS formülleri Python'da bağımsız hesaplandı
    // (2026-11-22; e yerine 2.71828 dahil, fark < 1e−6).
    fn zhu_in() -> zhu2017::Inputs {
        zhu2017::Inputs {
            pgv_cms: 60.0,
            pga_g: 0.4,
            mw: 7.4,
            vs30: 250.0,
            precip_mm: 800.0,
            dc_km: 3.0,
            dr_km: 1.5,
            wtd_m: 2.0,
        }
    }

    #[test]
    fn zhu_genel_usgs_ile_ayni() {
        use zhu2017::*;
        let i = zhu_in();
        close(logit(Variant::General, &i), -0.383_428, 1e-5);
        let p = probability(Variant::General, &i).unwrap();
        close(p, 0.405_300, 1e-6);
        close(coverage(Variant::General, p), 0.118_906, 1e-6);
        // Kırpma (PGV 200 → 150, yağış 3000 → 2500) ve M 6.04 → 6.0.
        let j = Inputs {
            pgv_cms: 200.0,
            mw: 6.04,
            vs30: 180.0,
            precip_mm: 3000.0,
            dc_km: 10.0,
            dr_km: 20.0,
            wtd_m: 0.5,
            ..i
        };
        close(logit(Variant::General, &j), -0.435_700, 1e-5);
        // P = 1'de kaplama USGS'in maxprob değeri (0.487).
        close(coverage(Variant::General, 1.0), 0.487, 5e-4);
    }

    #[test]
    fn zhu_kiyi_usgs_ile_ayni() {
        use zhu2017::*;
        let i = zhu_in();
        close(logit(Variant::Coastal, &i), -0.390_190, 1e-5);
        let p = probability(Variant::Coastal, &i).unwrap();
        close(coverage(Variant::Coastal, p), 0.160_258, 1e-6);
    }

    #[test]
    fn zhu_maskeleri() {
        use zhu2017::*;
        let i = zhu_in();
        for m in [
            Inputs { pgv_cms: 2.9, ..i },
            Inputs { pga_g: 0.09, ..i },
            Inputs { vs30: 621.0, ..i },
        ] {
            assert!(probability(Variant::General, &m).is_none());
            assert_eq!(cell_coverage(Variant::General, &m, 1.0), 0.0);
        }
        let full = cell_coverage(Variant::General, &i, 1.0);
        close(cell_coverage(Variant::General, &i, 0.25), full / 4.0, 1e-12);
    }

    #[test]
    fn jessee_usgs_ile_ayni() {
        use jessee2018::*;
        let sc = lithology_coef(9).unwrap(); // sc
        assert_eq!(GLIM_CLASSES[8], "sc");
        let lc = land_cover_coef(120).unwrap();
        close(logit(80.0, 25.0, sc, 8.0, lc), 3.945_851, 1e-5);
        close(sub_coverage(80.0, 0.3, 25.0, sc, 8.0, lc), 0.207_565, 1e-6);
        // Pekişmemiş tortul: USGS değeri (−1.36), makaledeki −3.22 değil.
        let su = lithology_coef(12).unwrap();
        close(su, -1.36, 1e-12);
        close(sub_coverage(80.0, 0.3, 25.0, su, 8.0, lc), 0.187_583, 1e-6);
        // Kırpma: PGV 300 → 211, CTI 25 → 19.
        let mt = lithology_coef(3).unwrap();
        close(logit(300.0, 10.0, mt, 25.0, 0.0), 2.365_752, 1e-5);
        // P = 1'de kaplama USGS maxprob (0.256).
        close(coverage(1.0), 0.256, 5e-4);
    }

    #[test]
    fn jessee_maskeleri_ve_ceyrekler() {
        use jessee2018::*;
        assert_eq!(sub_coverage(80.0, 0.3, 2.0, 0.0, 8.0, 0.0), 0.0);
        assert_eq!(sub_coverage(80.0, 0.019, 30.0, 0.0, 8.0, 0.0), 0.0);
        let one = sub_coverage(80.0, 0.3, 30.0, 0.0, 8.0, 0.0);
        let c = cell_coverage(80.0, 0.3, [0.5, 1.0, 1.5, 30.0], 0.0, 8.0, 0.0);
        close(c, one / 4.0, 1e-12);
        // Su kütlesi ve bilinmeyen kodlar modeli kapatır.
        assert!(lithology_coef(16).is_none());
        assert!(lithology_coef(0).is_none());
        assert!(land_cover_coef(210).is_none());
        assert_eq!(land_cover_coef(11), Some(0.0));
    }

    #[test]
    fn cci_globcover_eslemesi() {
        use jessee2018::*;
        // Doğrudan karşılıklar.
        assert_eq!(cci_land_cover_coef(10), land_cover_coef(14));
        assert_eq!(cci_land_cover_coef(20), Some(0.0));
        assert_eq!(cci_land_cover_coef(61), Some(0.30));
        assert_eq!(cci_land_cover_coef(62), Some(1.77));
        assert_eq!(cci_land_cover_coef(81), Some(-1.26));
        // Yoğunluğu belirsiz sınıflar: kapalı ve açık ortalaması.
        close(cci_land_cover_coef(60).unwrap(), (0.30 + 1.77) / 2.0, 1e-12);
        close(cci_land_cover_coef(70).unwrap(), (1.71 - 1.26) / 2.0, 1e-12);
        // Su ve veri yok modeli kapatır.
        assert!(cci_land_cover_coef(210).is_none());
        assert!(cci_land_cover_coef(0).is_none());
        // CCI lejandının her kara sınıfı eşlenmiş.
        for c in [
            10u8, 11, 12, 20, 30, 40, 50, 60, 61, 62, 70, 71, 72, 80, 81, 82, 90, 100, 110, 120,
            121, 122, 130, 140, 150, 151, 152, 153, 160, 170, 180, 190, 200, 201, 202, 220,
        ] {
            assert!(cci_land_cover_coef(c).is_some(), "CCI {c}");
        }
    }

    #[test]
    fn sivilasma_izgarasi() {
        use crate::grid::{Base, Domain, GridSpec, ShakeGrid};
        use crate::raster::{LayerInfo, Scale};
        use liquefaction_grid::*;
        let lin = |hi: f32| LayerInfo {
            name: String::new(),
            scale: Scale::Linear,
            n_valid: 251,
            lo: 0.0,
            hi,
        };
        let (pr, dist, slope) = (lin(2500.0), lin(50.0), lin(75.0));
        // 2 × 1 alan, adım 1: hücre 0 düz, hücre 1'in 3 çeyreği dik.
        let domain = Domain {
            base: Base::GLOBAL_30S,
            col0: 0,
            row0: 0,
            w: 2,
            h: 1,
        };
        let spec = GridSpec {
            west: -180.0,
            north: 90.0,
            cell: 1.0 / 120.0,
            width: 2,
            height: 1,
        };
        let shake = ShakeGrid {
            spec,
            class: vec![6, 6],
            jma: vec![5.6; 2],
            t_p: vec![1.0; 2],
            t_s: vec![2.0; 2],
            t_end: vec![30.0; 2],
            rise: vec![0.4; 2],
            pgv: vec![60.0; 2],
            pga: vec![0.4; 2],
            pgv_tau: vec![0.3; 2],
            pga_tau: vec![0.3; 2],
        };
        let code = |l: &LayerInfo, v: f32| l.quantize(v);
        let prc = [code(&pr, 800.0); 2];
        let dcc = [code(&dist, 3.0); 2];
        let drc = [code(&dist, 1.4); 2];
        let flat = [code(&slope, 0.3); 2];
        let steep = [code(&slope, 0.3), code(&slope, 12.0)];
        let vs30 = [250.0f32; 2];
        let inputs = Inputs::Raw(RawInputs {
            vs30: &vs30,
            precip_mm: Layer {
                codes: &prc,
                info: &pr,
            },
            dc_km: Layer {
                codes: &dcc,
                info: &dist,
            },
            dr_km: Layer {
                codes: &drc,
                info: &dist,
            },
            slope_q: [
                Layer {
                    codes: &flat,
                    info: &slope,
                },
                Layer {
                    codes: &steep,
                    info: &slope,
                },
                Layer {
                    codes: &steep,
                    info: &slope,
                },
                Layer {
                    codes: &steep,
                    info: &slope,
                },
            ],
            wtd_m: None,
        });
        let g = compute(
            &shake,
            &domain,
            1,
            7.4,
            &inputs,
            &std::sync::atomic::AtomicBool::new(false),
        )
        .unwrap();
        // Kıyı modeli (wtd yok); hücre 1 dörtte bir.
        let x = zhu2017::Inputs {
            pgv_cms: 60.0,
            pga_g: 0.4,
            mw: 7.4,
            vs30: 250.0,
            precip_mm: f64::from(pr.value(prc[0]).unwrap()),
            dc_km: f64::from(dist.value(dcc[0]).unwrap()),
            dr_km: f64::from(dist.value(drc[0]).unwrap()),
            wtd_m: 0.0,
        };
        let full = zhu2017::cell_coverage(zhu2017::Variant::Coastal, &x, 1.0);
        close(f64::from(g.coverage[0]), full, 1e-6);
        close(f64::from(g.coverage[1]), full / 4.0, 1e-6);
        assert_eq!((g.general, g.coastal), (0, 2));
        // Aralık: daha güçlü sarsıntı daha çok sıvılaşma.
        assert!(g.coverage_lo[0] < g.coverage[0] && g.coverage[0] < g.coverage_hi[0]);
        let hi = zhu2017::cell_coverage(
            zhu2017::Variant::Coastal,
            &zhu2017::Inputs {
                pgv_cms: 60.0 * 0.3f64.exp(),
                pga_g: 0.4 * 0.3f64.exp(),
                ..x
            },
            1.0,
        );
        close(f64::from(g.coverage_hi[0]), hi, 1e-5);

        // Önceden hesaplanmış katman (16 bit) ham yolla aynı sonucu verir;
        // yalnızca kaba kodla (X adımı 0,06) hücre başına %5 içinde.
        use crate::groundfailure::static_layer;
        let Inputs::Raw(raw) = &inputs else {
            unreachable!()
        };
        let enc: Vec<[u8; 3]> = (0..2)
            .map(|i| {
                let (x, flat, v) = raw_base(raw, i).unwrap();
                static_layer::encode(x, flat, v)
            })
            .collect();
        let (xs, fs, cs) = (
            enc.iter().map(|e| e[0]).collect::<Vec<_>>(),
            enc.iter().map(|e| e[1]).collect::<Vec<_>>(),
            enc.iter().map(|e| e[2]).collect::<Vec<_>>(),
        );
        for (fine, tol) in [(Some(&fs[..]), 1e-4), (None, 5e-2)] {
            let st = Inputs::Static(StaticInputs {
                x: &xs,
                fine,
                cls: &cs,
            });
            let h = compute(
                &shake,
                &domain,
                1,
                7.4,
                &st,
                &std::sync::atomic::AtomicBool::new(false),
            )
            .unwrap();
            for k in 0..2 {
                let rel = (h.coverage[k] / g.coverage[k] - 1.0).abs();
                assert!(rel < tol, "hücre {k}: göreli fark {rel}");
                let rel = (h.coverage_hi[k] / g.coverage_hi[k] - 1.0).abs();
                assert!(rel < tol, "hücre {k} (+1τ): göreli fark {rel}");
            }
            assert_eq!((h.general, h.coastal), (0, 2));
        }
    }

    #[test]
    fn statik_katman_kodlari() {
        use static_layer::*;
        use zhu2017::Variant;
        for x in [-9.0, -3.217, 0.0, 0.031, 4.99, 6.0] {
            for (flat, v) in [(1, Variant::General), (4, Variant::Coastal)] {
                let [c, f, cls] = encode(x, flat, v);
                let (y, fl, w) = decode(c, Some(f), cls).unwrap();
                close(y, x, 3e-4);
                assert_eq!((fl, w), (flat, v));
                let (y, ..) = decode(c, None, cls).unwrap();
                close(y, x, 0.031);
            }
        }
        // Kırpma ve maske.
        let [c, ..] = encode(-20.0, 2, Variant::General);
        close(decode(c, None, 1).unwrap().0, X_LO, 1e-12);
        assert!(decode(MASKED, Some(128), 0).is_none());
        assert!(decode(10, None, MASKED).is_none());
        // Statik + PGV terimi = tam logit.
        let i = zhu_in();
        for v in [Variant::General, Variant::Coastal] {
            let full = zhu2017::cell_coverage(v, &i, 0.5);
            let st = zhu2017::cell_coverage_static(
                v,
                zhu2017::static_logit(v, &i),
                i.pgv_cms,
                i.pga_g,
                i.mw,
                0.5,
            );
            close(st, full, 1e-12);
        }
    }
}
