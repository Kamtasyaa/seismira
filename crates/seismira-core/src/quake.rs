//! Deprem tanımı ve girdi geçerlilik kuralları (CLAUDE.md Bölüm 6).
//!
//! Aralıklar yer hareketi modellerinin yayımlanmış geçerlilik aralıklarından
//! gelir: sığ kabukta CY14 (Faz 3), dalma-batmada Parker vd. (2020) (Faz 5,
//! [`crate::gmpe::psbah20`]). Ortamların ALT derinlik sınırları (arayüz 5 km,
//! levha içi 30 km) modelden değil sınıflandırmadan gelen yaklaşık
//! değerlerdir (docs/arayuz.md, "Geçerlilik kuralları"). Sessiz
//! ekstrapolasyon yapılmaz: aralık dışı girdi engellenir ya da açıkça uyarılır.

/// Tektonik ortam.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Setting {
    /// Sığ kabuk (aktif tektonik bölge).
    ShallowCrust,
    /// Dalma-batma arayüzü.
    Interface,
    /// Levha içi (dalan levha içinde).
    Intraslab,
}

/// Fay tipi.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FaultType {
    StrikeSlip,
    Normal,
    Reverse,
}

impl FaultType {
    /// Varsayılan (eğim, kayma açısı), derece. Bölüm 7.1'deki uzlaşımsal
    /// değerler (doğrulanacak): doğrultu atımlı dikey ve saf yanal; normal
    /// ~55° ve −90°; ters ~40° ve +90°.
    pub fn default_dip_rake(self) -> (f64, f64) {
        match self {
            FaultType::StrikeSlip => (90.0, 0.0),
            FaultType::Normal => (55.0, -90.0),
            FaultType::Reverse => (40.0, 90.0),
        }
    }

    /// Kayma açısından (rake, derece) fay tipi. Chiou & Youngs (2014) fay
    /// tipi bayraklarının tanımı: ters (F_RV) 30° ≤ λ ≤ 150°, normal (F_NM)
    /// −120° ≤ λ ≤ −60°, kalanı doğrultu atımlı. Sınırlar dahil. İki
    /// bağımsız uygulamayla teyit edildi (2026-11-15): OpenQuake hazardlib
    /// `chiou_youngs_2014` ve wltcwpf/GMPE `cy_2014_nga.R`; makale metni
    /// görülmedi. Arayüz, kayma açısı değişince fay tipini buna göre seçer;
    /// böylece hesapta kullanılan tip ile girilen açı çelişmez.
    pub fn from_rake(rake: f64) -> FaultType {
        if (30.0..=150.0).contains(&rake) {
            FaultType::Reverse
        } else if (-120.0..=-60.0).contains(&rake) {
            FaultType::Normal
        } else {
            FaultType::StrikeSlip
        }
    }
}

/// Kullanıcının tanımladığı deprem.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quake {
    /// Merkez üssü (boylam, enlem), derece.
    pub epicenter: (f64, f64),
    /// Moment büyüklüğü.
    pub mw: f64,
    /// Odak derinliği (km).
    pub depth_km: f64,
    pub setting: Setting,
    pub fault: FaultType,
    /// Doğrultu (strike), 0–360°.
    pub strike: f64,
    /// Eğim (dip), 0–90°.
    pub dip: f64,
    /// Kayma açısı (rake), −180–180°.
    pub rake: f64,
    /// İleri seviye: hiposantrın fay üzerindeki konumu, doğrultu boyunca
    /// (0 = doğrultunun başlangıç ucu, 1 = bitiş ucu). Varsayılan 0.5.
    pub hypo_along: f64,
    /// İleri seviye: hiposantrın eğim boyunca konumu (0 = üst kenar,
    /// 1 = alt kenar). Varsayılan 0.5. Ztor ≥ 0 ve sismojenik taban kısıtı
    /// gerekirse bunu değiştirir (bkz. source.rs).
    pub hypo_down: f64,
}

impl Quake {
    pub fn new(epicenter: (f64, f64)) -> Self {
        let fault = FaultType::StrikeSlip;
        let (dip, rake) = fault.default_dip_rake();
        Self {
            epicenter,
            mw: 7.0,
            depth_km: 10.0,
            setting: Setting::ShallowCrust,
            fault,
            strike: 0.0,
            dip,
            rake,
            hypo_along: 0.5,
            hypo_down: 0.5,
        }
    }
}

/// Bölüm 6 tablosu (bkz. modül notu).
pub mod limits {
    use crate::gmpe::psbah20;

    /// Seyahat süresi tablosunun derinlik sınırı (km).
    pub const MAX_DEPTH: f64 = 700.0;

    pub const CRUST_MW: (f64, f64) = (4.0, 8.5);
    /// Sığ kabukta ters ve normal faylar için üst sınır.
    pub const CRUST_MW_MAX_DIP_SLIP: f64 = 8.0;
    pub const CRUST_DEPTH: (f64, f64) = (0.0, 35.0);
    /// Arayüz: Parker vd. (2020) M 4.5–9.5, Zhyp ≤ 40 km.
    pub const INTERFACE_MW: (f64, f64) = psbah20::INTERFACE_MW;
    pub const INTERFACE_DEPTH: (f64, f64) = (5.0, psbah20::INTERFACE_ZHYP_MAX);
    /// Levha içi: Parker vd. (2020) M 4.5–8.5.
    pub const INTRASLAB_MW: (f64, f64) = psbah20::INTRASLAB_MW;
    /// Levha içi: 30 km'den başlar. Modelin sınırı (200 km) ötesi
    /// [`super::Issue::DeepFocus`] uyarısıyla seyahat süresi tablosunun
    /// sınırına kadar izinli: dalgalar gösterilir, şiddet hesaplanmaz.
    pub const INTRASLAB_DEPTH: (f64, f64) = (30.0, MAX_DEPTH);
    /// Bu derinliğin ötesinde yer hareketi modeli yok (Parker vd. 2020,
    /// levha içi Zhyp ≤ 200 km).
    pub const DEEP_FOCUS_FROM: f64 = psbah20::INTRASLAB_ZHYP_MAX;
}

/// Bir geçerlilik sorunu. `Block` simülasyonu engeller, `Warn` açıkça
/// gösterilir ama engellemez.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Issue {
    /// Mw, ortam/fay tipi için aralık dışı.
    MwOutOfRange { min: f64, max: f64 },
    /// Derinlik, ortam için aralık dışı.
    DepthOutOfRange { min: f64, max: f64 },
    /// Dalma-batma arayüzü yalnızca ters fay olabilir.
    InterfaceNotReverse,
    /// Odak yer hareketi modelinin derinlik sınırından derin
    /// ([`limits::DEEP_FOCUS_FROM`]): şiddet hesaplanmaz, yalnızca dalgalar
    /// gösterilir. Engellemez, açıkça uyarır.
    DeepFocus { max: f64 },
    /// Eğim 0–90° dışında ya da sayı değil.
    BadGeometry,
    /// Sığ kabukta kırılmanın üst kenarı (Ztor) yer hareketi modelinin
    /// sınırından derin: model hiçbir sahada geçerli olmaz (CY14, Ztor ≤
    /// 20 km). Küçük ve derin depremlerde olur; derinlik aralığı (0–35 km)
    /// tek başına bunu yakalamaz, çünkü Ztor büyüklüğe de bağlıdır.
    ZtorOutOfRange { ztor: f64, max: f64 },
    /// Sığ kabukta büyüklük, kırılma boyutu bağıntısının veri aralığının
    /// üstünde ([`crate::source::WC94_MW_MAX`]): fay boyutları
    /// ekstrapolasyondur. Engellemez, açıkça uyarır.
    RuptureSizeExtrapolated { max: f64 },
}

impl Issue {
    pub fn blocks(self) -> bool {
        !matches!(
            self,
            Issue::DeepFocus { .. } | Issue::RuptureSizeExtrapolated { .. }
        )
    }
}

/// Girdiyi Bölüm 6 kurallarına göre denetler.
pub fn validate(q: &Quake) -> Vec<Issue> {
    use limits::*;
    let mut out = Vec::new();
    let (mw_range, depth_range) = match q.setting {
        Setting::ShallowCrust => {
            let max = if q.fault == FaultType::StrikeSlip {
                CRUST_MW.1
            } else {
                CRUST_MW_MAX_DIP_SLIP
            };
            ((CRUST_MW.0, max), CRUST_DEPTH)
        }
        Setting::Interface => (INTERFACE_MW, INTERFACE_DEPTH),
        Setting::Intraslab => (INTRASLAB_MW, INTRASLAB_DEPTH),
    };
    if !(mw_range.0..=mw_range.1).contains(&q.mw) {
        out.push(Issue::MwOutOfRange {
            min: mw_range.0,
            max: mw_range.1,
        });
    }
    if !(depth_range.0..=depth_range.1).contains(&q.depth_km) {
        out.push(Issue::DepthOutOfRange {
            min: depth_range.0,
            max: depth_range.1,
        });
    }
    if q.setting == Setting::Interface && q.fault != FaultType::Reverse {
        out.push(Issue::InterfaceNotReverse);
    }
    if q.depth_km > DEEP_FOCUS_FROM && q.depth_km <= MAX_DEPTH {
        out.push(Issue::DeepFocus {
            max: DEEP_FOCUS_FROM,
        });
    }
    let finite = [
        q.mw,
        q.depth_km,
        q.strike,
        q.dip,
        q.rake,
        q.epicenter.0,
        q.epicenter.1,
    ]
    .iter()
    .all(|v| v.is_finite());
    if !finite || !(0.0..=90.0).contains(&q.dip) || !(-90.0..=90.0).contains(&q.epicenter.1) {
        out.push(Issue::BadGeometry);
    } else if q.setting == Setting::ShallowCrust {
        if q.mw > crate::source::WC94_MW_MAX {
            out.push(Issue::RuptureSizeExtrapolated {
                max: crate::source::WC94_MW_MAX,
            });
        }
        // Kaynak geometrisi kurulabiliyorsa: üst kenar modelin sınırında mı?
        let ztor = crate::source::Fault::from_quake(q).ztor;
        let max = crate::gmpe::cy14::ZTOR_MAX_KM;
        if ztor > max {
            out.push(Issue::ZtorOutOfRange { ztor, max });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quake() -> Quake {
        Quake::new((29.0, 40.7))
    }

    #[test]
    fn varsayilan_gecerli() {
        assert!(validate(&quake()).is_empty());
    }

    #[test]
    fn kabukta_buyukluk_siniri_fay_tipine_bagli() {
        let mut q = quake();
        q.mw = 8.3;
        // Doğrultu atımlı: 8.5'e kadar izinli; 7.9'un üstünde yalnızca
        // kırılma boyutu uyarısı (engellemez).
        let warn = Issue::RuptureSizeExtrapolated { max: 7.9 };
        assert_eq!(validate(&q), vec![warn]);
        assert!(!warn.blocks());
        q.fault = FaultType::Reverse;
        assert_eq!(
            validate(&q),
            vec![Issue::MwOutOfRange { min: 4.0, max: 8.0 }, warn]
        );
        q.mw = 7.9;
        assert!(validate(&q).is_empty());
    }

    #[test]
    fn derinlik_ortama_gore() {
        let mut q = quake();
        q.depth_km = 50.0;
        assert!(matches!(validate(&q)[0], Issue::DepthOutOfRange { .. }));
        q.setting = Setting::Intraslab;
        assert!(validate(&q).is_empty());
        q.depth_km = 200.0;
        assert!(validate(&q).is_empty(), "modelin sınırı dahil");
        q.depth_km = 450.0;
        let issues = validate(&q);
        assert_eq!(issues, vec![Issue::DeepFocus { max: 200.0 }]);
        assert!(!issues[0].blocks());
        q.depth_km = 701.0;
        assert!(validate(&q).iter().any(|i| i.blocks()));
    }

    #[test]
    fn arayuz_yalnizca_ters_fay() {
        let mut q = quake();
        q.setting = Setting::Interface;
        q.mw = 9.0;
        q.depth_km = 25.0;
        assert_eq!(validate(&q), vec![Issue::InterfaceNotReverse]);
        q.fault = FaultType::Reverse;
        assert!(validate(&q).is_empty());
        // Parker vd. (2020): arayüzde Zhyp ≤ 40 km, M ≤ 9.5.
        q.depth_km = 41.0;
        assert_eq!(
            validate(&q),
            vec![Issue::DepthOutOfRange {
                min: 5.0,
                max: 40.0
            }]
        );
        q.depth_km = 40.0;
        q.mw = 9.6;
        assert!(matches!(validate(&q)[..], [Issue::MwOutOfRange { max, .. }] if max == 9.5));
    }

    #[test]
    fn levha_ici_buyukluk_siniri() {
        let mut q = quake();
        q.setting = Setting::Intraslab;
        q.depth_km = 60.0;
        q.mw = 8.5;
        assert!(validate(&q).is_empty());
        q.mw = 8.6;
        assert!(matches!(validate(&q)[..], [Issue::MwOutOfRange { max, .. }] if max == 8.5));
    }

    #[test]
    fn kayma_acisindan_fay_tipi() {
        use FaultType::*;
        for (rake, want) in [
            (0.0, StrikeSlip),
            (180.0, StrikeSlip),
            (-180.0, StrikeSlip),
            (29.9, StrikeSlip),
            (30.0, Reverse),
            (90.0, Reverse),
            (150.0, Reverse),
            (150.1, StrikeSlip),
            (-59.9, StrikeSlip),
            (-60.0, Normal),
            (-90.0, Normal),
            (-120.0, Normal),
            (-120.1, StrikeSlip),
        ] {
            assert_eq!(FaultType::from_rake(rake), want, "{rake}");
        }
        // Varsayılan açılar kendi tiplerine döner.
        for f in [StrikeSlip, Normal, Reverse] {
            assert_eq!(FaultType::from_rake(f.default_dip_rake().1), f);
        }
    }

    #[test]
    fn derin_ve_kucuk_kabuk_depremi_engellenir() {
        // Mw 5, 30 km: derinlik aralıkta ama üst kenar ~28 km → CY14 dışı.
        let mut q = quake();
        q.mw = 5.0;
        q.depth_km = 30.0;
        let issues = validate(&q);
        assert!(
            matches!(issues[..], [Issue::ZtorOutOfRange { ztor, max }] if ztor > max),
            "{issues:?}"
        );
        assert!(issues[0].blocks());
        // Aynı derinlikte büyük deprem: düzlem yukarı uzanır, sorun yok.
        q.mw = 7.0;
        assert!(validate(&q).is_empty());
        // Denetim geçiyorsa model merkez üssünde sonuç verir (sessiz boşluk yok).
        for (mw, depth) in [(4.0, 20.0), (5.0, 21.0), (6.0, 25.0), (6.5, 35.0)] {
            q.mw = mw;
            q.depth_km = depth;
            if validate(&q).is_empty() {
                let f = crate::source::Fault::from_quake(&q);
                assert!(f.ztor <= crate::gmpe::cy14::ZTOR_MAX_KM, "{mw} {depth}");
            }
        }
        // Levha içinde bu denetim yok (model zaten yok).
        q.mw = 5.0;
        q.depth_km = 60.0;
        q.setting = Setting::Intraslab;
        assert!(validate(&q).is_empty());
    }

    #[test]
    fn gecersiz_sayi_engellenir() {
        let mut q = quake();
        q.mw = f64::NAN;
        assert!(validate(&q).iter().any(|i| i.blocks()));
    }
}
