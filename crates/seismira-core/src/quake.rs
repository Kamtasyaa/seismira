//! Deprem tanımı ve girdi geçerlilik kuralları (CLAUDE.md Bölüm 6).
//!
//! UYARI: Aralıklar CLAUDE.md'deki YAKLAŞIK tablodan alınmıştır ve henüz
//! doğrulanmamıştır. Faz 3 (CY14) ve Faz 5 (Parker vd. 2020) modelleri
//! eklenirken her modelin yayımlanmış geçerlilik aralığıyla değiştirilecek
//! (docs/arayuz.md, "Geçerlilik kuralları" notu). Sessiz ekstrapolasyon
//! yapılmaz: aralık dışı girdi engellenir ya da açıkça uyarılır.

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
        }
    }
}

/// Bölüm 6 tablosu (yaklaşık; bkz. modül notu).
pub mod limits {
    /// Seyahat süresi tablosunun derinlik sınırı (km).
    pub const MAX_DEPTH: f64 = 700.0;
    /// Bu derinliğin ötesi "derin odak": modeller geçersiz, kaba tahmin.
    pub const DEEP_FOCUS_FROM: f64 = 300.0;

    pub const CRUST_MW: (f64, f64) = (4.0, 8.5);
    /// Sığ kabukta ters ve normal faylar için üst sınır.
    pub const CRUST_MW_MAX_DIP_SLIP: f64 = 8.0;
    pub const CRUST_DEPTH: (f64, f64) = (0.0, 35.0);
    pub const INTERFACE_MW: (f64, f64) = (4.5, 9.5);
    pub const INTERFACE_DEPTH: (f64, f64) = (5.0, 60.0);
    pub const INTRASLAB_MW: (f64, f64) = (4.5, 8.0);
    /// Levha içi: 30 km'den başlar; 300 km ötesi derin odak uyarısıyla izinli.
    pub const INTRASLAB_DEPTH: (f64, f64) = (30.0, MAX_DEPTH);
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
    /// Derin odak (> 300 km): modeller geçersiz, sonuçlar kaba tahmin.
    DeepFocus,
    /// Eğim 0–90° dışında ya da sayı değil.
    BadGeometry,
}

impl Issue {
    pub fn blocks(self) -> bool {
        !matches!(self, Issue::DeepFocus)
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
        out.push(Issue::DeepFocus);
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
        assert!(validate(&q).is_empty()); // doğrultu atımlı: 8.5'e kadar
        q.fault = FaultType::Reverse;
        assert_eq!(
            validate(&q),
            vec![Issue::MwOutOfRange { min: 4.0, max: 8.0 }]
        );
    }

    #[test]
    fn derinlik_ortama_gore() {
        let mut q = quake();
        q.depth_km = 50.0;
        assert!(matches!(validate(&q)[0], Issue::DepthOutOfRange { .. }));
        q.setting = Setting::Intraslab;
        assert!(validate(&q).is_empty());
        q.depth_km = 450.0;
        let issues = validate(&q);
        assert_eq!(issues, vec![Issue::DeepFocus]);
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
    }

    #[test]
    fn gecersiz_sayi_engellenir() {
        let mut q = quake();
        q.mw = f64::NAN;
        assert!(validate(&q).iter().any(|i| i.blocks()));
    }
}
