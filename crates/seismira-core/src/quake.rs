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
    /// Varsayılan (eğim, kayma açısı), derece. Eğim: Kaklamanos, Boore,
    /// Thompson & Campbell (2010, sürüm 1.1 2011), USGS Open-File Report
    /// 2010-1296, Tablo 2 "Estimation of dip angle from style of faulting"
    /// (Kaklamanos vd. 2011, Earthquake Spectra 27(4), doi:10.1193/1.3650372
    /// önerisi): doğrultu atımlı 90°, ters 40°, normal 50° (2026-11-24
    /// doğrulandı; önce normal 55° idi). Kayma açısı: saf mekanizma
    /// (0°, +90°, −90°; uzlaşım).
    pub fn default_dip_rake(self) -> (f64, f64) {
        match self {
            FaultType::StrikeSlip => (90.0, 0.0),
            FaultType::Normal => (50.0, -90.0),
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

/// Fay izindeki en çok nokta (en çok 12 parça; NGA-West2 veritabanında
/// en çok parçalı kırılma 12 parça, PEER 2013/03 Ek D "Number of fault
/// segments ... The maximum is 12").
pub const MAX_TRACE_POINTS: usize = 13;

/// Çok parçalı kırılmanın üst kenar izi (yüzeydeki izdüşümü): ardışık
/// noktalar arasındaki her doğru bir parça ([`crate::rupture`]). Nokta sırası
/// doğrultuyu belirler (Aki & Richards: fay, doğrultu yönüne bakınca sağa
/// eğimli). `Quake` kopyalanabilir kalsın diye sabit boyutlu.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Trace {
    n: usize,
    points: [(f64, f64); MAX_TRACE_POINTS],
    /// Parçaya özel eğim (derece); `None`: depremin ortak eğimi.
    dips: [Option<f64>; MAX_TRACE_POINTS - 1],
}

impl Trace {
    /// Noktalardan iz (en az 2, en çok [`MAX_TRACE_POINTS`]); değilse `None`.
    pub fn new(points: &[(f64, f64)]) -> Option<Self> {
        if !(2..=MAX_TRACE_POINTS).contains(&points.len()) {
            return None;
        }
        let mut t = Self {
            n: points.len(),
            points: [(0.0, 0.0); MAX_TRACE_POINTS],
            dips: [None; MAX_TRACE_POINTS - 1],
        };
        t.points[..points.len()].copy_from_slice(points);
        Some(t)
    }

    pub fn points(&self) -> &[(f64, f64)] {
        &self.points[..self.n]
    }

    pub fn segments(&self) -> usize {
        self.n - 1
    }

    /// `i`. parçanın kendi eğimi (yoksa ortak eğim kullanılır).
    pub fn segment_dip(&self, i: usize) -> Option<f64> {
        self.dips.get(i).copied().flatten()
    }

    pub fn set_segment_dip(&mut self, i: usize, dip: Option<f64>) {
        if i < self.segments() {
            self.dips[i] = dip;
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
    /// İleri seviye: elle kırılma boyutu [uzunluk, genişlik] (km). `None`:
    /// otomatik (WC94 alanı; sığ kabuk doğrultu atımlıda TMG17 uzunluk ve
    /// genişliği, [`crate::source::Fault::from_quake`]).
    /// Genişlik sismojenik taban / arayüz sınırına sığmazsa kırpılır
    /// ([`Issue::RuptureWidthClamped`]).
    pub rupture_km: Option<[f64; 2]>,
    /// Çok parçalı kırılma izi; `None`: tek düzlem (doğrultu + boyut).
    /// Verilmişse uzunluk izden gelir, doğrultu her parçanın yönüdür;
    /// `strike`, `hypo_along` ve `rupture_km` uzunluğu kullanılmaz
    /// (genişlik: `rupture_km`'deki ya da otomatik).
    pub trace: Option<Trace>,
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
            rupture_km: None,
            trace: None,
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
    /// Elle kırılma boyutunun girdi sınırı [uzunluk, genişlik] (km). Bilim
    /// katsayısı değil, yazım hatasını yakalayan akıl sınırı: kayıtlı en
    /// uzun kırılma 2004 Sumatra–Andaman (~1300–1500 km); arayüz genişliği
    /// ~200 km'de doyar ([`crate::source::INTERFACE_MAX_WIDTH_KM`]).
    pub const RUPTURE_MAX_KM: (f64, f64) = (1500.0, 250.0);
    /// Elle alanın WC94 alanından sapma uyarısı: Tablo 2A M–RA standart
    /// sapması s = 0,24 (M birimi), b = 0,98 → log10 A'da 0,245; 2,5σ =
    /// 0,61 → ×4,1.
    pub const RUPTURE_AREA_FACTOR: f64 = 4.1;
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
    /// Otomatik boyutta büyüklük, kırılma boyutu bağıntısının veri aralığının
    /// üstünde ([`crate::source::auto_size_mw_max`]: WC94 7.9; kabuk doğrultu
    /// atımlıda TMG17 8.7, girdi sınırı 8.5 olduğundan oluşmaz; arayüz ve
    /// levha içi de 7.9): fay boyutları ekstrapolasyondur. Engellemez, açıkça
    /// uyarır.
    RuptureSizeExtrapolated { max: f64 },
    /// Elle kırılma boyutu aralık dışı ya da sayı değil (uzunluk
    /// 1–[`limits::RUPTURE_MAX_KM`].0, genişlik 1–[`limits::RUPTURE_MAX_KM`].1).
    RuptureSizeInvalid { max_l: f64, max_w: f64 },
    /// Elle genişlik sismojenik taban / arayüz sınırına sığmadı; bu
    /// değere kırpıldı. Engellemez, açıkça uyarır.
    RuptureWidthClamped { width: f64 },
    /// Elle verilen alan, büyüklüğün WC94 alanından [`limits::RUPTURE_AREA_FACTOR`]
    /// katından fazla farklı (~2,5σ). Engellemez, açıkça uyarır.
    RuptureAreaUnusual { ratio: f64 },
    /// Sığ kabuk depremi kararlı kıta içinde ([`crate::tectonic`]): yer
    /// hareketi modelleri aktif kabuk için; sonuçlar bu bölgede temsil
    /// edici değil. Engellemez, açıkça uyarır (docs/bilim.md 7.4).
    StableContinental,
    /// Fay izi geçersiz: 1 km'den kısa parça, toplam uzunluk
    /// [`limits::RUPTURE_MAX_KM`].0 üstü, sayı olmayan nokta ya da parça
    /// eğimi 1–90° dışında. Engeller.
    TraceInvalid { max_l: f64 },
}

impl Issue {
    pub fn blocks(self) -> bool {
        !matches!(
            self,
            Issue::DeepFocus { .. }
                | Issue::RuptureSizeExtrapolated { .. }
                | Issue::RuptureWidthClamped { .. }
                | Issue::RuptureAreaUnusual { .. }
                | Issue::StableContinental
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
    } else if let Some([l, w]) = q.rupture_km
        && !(l.is_finite()
            && w.is_finite()
            && (1.0..=RUPTURE_MAX_KM.0).contains(&l)
            && (1.0..=RUPTURE_MAX_KM.1).contains(&w))
    {
        out.push(Issue::RuptureSizeInvalid {
            max_l: RUPTURE_MAX_KM.0,
            max_w: RUPTURE_MAX_KM.1,
        });
    } else if q.trace.is_some_and(|t| !trace_valid(&t)) {
        out.push(Issue::TraceInvalid {
            max_l: RUPTURE_MAX_KM.0,
        });
    } else {
        if q.rupture_km.is_some() || q.trace.is_some() {
            // Elle boyut ya da çizilmiş iz: genişlik kırpıldı mı, alan
            // büyüklüğe göre olağan mı (iz uzunluğu kullanıcınındır).
            let r = crate::rupture::Rupture::from_quake(q);
            if let Some([_, w]) = q.rupture_km
                && r.width < w - 1e-6
            {
                out.push(Issue::RuptureWidthClamped { width: r.width });
            }
            let l = match (q.trace, q.rupture_km) {
                (None, Some([l, _])) => l,
                _ => r.length,
            };
            let ratio = l * r.width / crate::source::area_wc94(q.mw);
            if !(1.0 / RUPTURE_AREA_FACTOR..=RUPTURE_AREA_FACTOR).contains(&ratio) {
                out.push(Issue::RuptureAreaUnusual { ratio });
            }
        }
        // Otomatik boyut bağıntısının veri aralığı (tüm ortamlar; bilim.md
        // 7.1: sessiz ekstrapolasyon yok).
        let max = crate::source::auto_size_mw_max(q);
        if q.rupture_km.is_none() && q.trace.is_none() && q.mw > max {
            out.push(Issue::RuptureSizeExtrapolated { max });
        }
        if q.setting == Setting::ShallowCrust {
            crust_checks(q, &mut out);
        }
    }
    out
}

/// Fay izi: parçalar ≥ 1 km, toplam ≤ sınır, noktalar sayı, eğimler 1–90°.
fn trace_valid(t: &Trace) -> bool {
    let pts = t.points();
    let finite = pts
        .iter()
        .all(|p| p.0.is_finite() && p.1.is_finite() && (-90.0..=90.0).contains(&p.1));
    if !finite {
        return false;
    }
    let lens: Vec<f64> = pts
        .windows(2)
        .map(|w| crate::geo::distance_azimuth(w[0], w[1]).0)
        .collect();
    let dips_ok =
        (0..t.segments()).all(|i| t.segment_dip(i).is_none_or(|d| (1.0..=90.0).contains(&d)));
    lens.iter().all(|&l| l >= 1.0)
        && lens.iter().sum::<f64>() <= limits::RUPTURE_MAX_KM.0
        && dips_ok
}

/// Sığ kabuk: Ztor sınırı.
fn crust_checks(q: &Quake, out: &mut Vec<Issue>) {
    // Kaynak geometrisi kurulabiliyorsa: üst kenar modelin sınırında mı?
    let ztor = crate::rupture::Rupture::from_quake(q).ztor;
    let max = crate::gmpe::cy14::ZTOR_MAX_KM;
    if ztor > max {
        out.push(Issue::ZtorOutOfRange { ztor, max });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quake() -> Quake {
        Quake::new((29.0, 40.7))
    }

    #[test]
    fn elle_kirilma_boyutu() {
        let mut q = quake();
        q.mw = 7.6;
        // İzmit benzeri: 145 × 20 km (WC94 alanı ~3 950 km² → oran 0,73).
        q.rupture_km = Some([145.0, 20.0]);
        assert!(validate(&q).is_empty(), "{:?}", validate(&q));
        let f = crate::source::Fault::from_quake(&q);
        assert_eq!((f.length, f.width), (145.0, 20.0));
        // Taban 20 km, düşey fay: 30 km genişlik kırpılır, uyarı.
        q.rupture_km = Some([145.0, 30.0]);
        let v = validate(&q);
        assert!(
            v.contains(&Issue::RuptureWidthClamped { width: 20.0 }),
            "{v:?}"
        );
        assert!(v.iter().all(|i| !i.blocks()));
        // Alan çok küçük: uyarı.
        q.rupture_km = Some([20.0, 10.0]);
        assert!(matches!(
            validate(&q)[..],
            [Issue::RuptureAreaUnusual { .. }]
        ));
        // Aralık dışı: engeller.
        q.rupture_km = Some([0.0, 10.0]);
        assert!(validate(&q).iter().any(|i| i.blocks()));
        q.rupture_km = Some([f64::NAN, 10.0]);
        assert!(validate(&q).iter().any(|i| i.blocks()));
        // Elle boyutta WC94 ekstrapolasyon uyarısı yok.
        q.mw = 8.2;
        q.rupture_km = Some([400.0, 20.0]);
        assert!(
            !validate(&q)
                .iter()
                .any(|i| matches!(i, Issue::RuptureSizeExtrapolated { .. }))
        );
    }

    #[test]
    fn varsayilan_gecerli() {
        assert!(validate(&quake()).is_empty());
    }

    #[test]
    fn kabukta_buyukluk_siniri_fay_tipine_bagli() {
        let mut q = quake();
        q.mw = 8.3;
        // Doğrultu atımlı: 8.5'e kadar izinli; boyut bağıntısı (TMG17,
        // Mw ≤ 8.7) bu aralığı kapsar, uyarı yok.
        assert!(validate(&q).is_empty(), "{:?}", validate(&q));
        q.mw = 8.5;
        assert!(validate(&q).is_empty(), "{:?}", validate(&q));
        q.mw = 8.3;
        // Ters fay: 8.0 sınırı ve WC94 (7.9) ekstrapolasyon uyarısı.
        let warn = Issue::RuptureSizeExtrapolated { max: 7.9 };
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
        q.mw = 7.9;
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
        assert!(matches!(validate(&q)[..], [Issue::MwOutOfRange { max, .. }, _] if max == 9.5));
    }

    #[test]
    fn levha_ici_buyukluk_siniri() {
        let mut q = quake();
        q.setting = Setting::Intraslab;
        q.depth_km = 60.0;
        q.mw = 7.9;
        assert!(validate(&q).is_empty());
        q.mw = 8.6;
        assert!(matches!(validate(&q)[..], [Issue::MwOutOfRange { max, .. }, _] if max == 8.5));
    }

    #[test]
    fn fay_izi_denetimi() {
        let mut q = quake();
        q.mw = 7.8;
        let e = q.epicenter;
        let p1 = crate::geo::destination(e, 40.0, 150.0);
        let p2 = crate::geo::destination(p1, 60.0, 150.0);
        q.trace = Trace::new(&[e, p1, p2]);
        // Mw 7.8, 300 km iz: WC94 alanı ~6 400 km² / 300 → ~21 km, tabana
        // (20 km) kırpılır; oran ~0.94, olağan. Ekstrapolasyon uyarısı yok.
        assert!(validate(&q).is_empty(), "{:?}", validate(&q));
        // Çok kısa parça: engeller.
        let p_close = crate::geo::destination(p1, 60.0, 0.5);
        q.trace = Trace::new(&[e, p1, p_close]);
        let issues = validate(&q);
        assert!(
            issues
                .iter()
                .any(|i| matches!(i, Issue::TraceInvalid { .. }) && i.blocks())
        );
        // Büyüklüğe göre çok kısa iz: olağan dışı alan uyarısı.
        let short = crate::geo::destination(e, 40.0, 15.0);
        q.trace = Trace::new(&[e, short]);
        assert!(
            validate(&q)
                .iter()
                .any(|i| matches!(i, Issue::RuptureAreaUnusual { .. }))
        );
        // Geçersiz parça eğimi.
        let mut t = Trace::new(&[e, p1, p2]).unwrap();
        t.set_segment_dip(0, Some(95.0));
        q.trace = Some(t);
        assert!(validate(&q).iter().any(|i| i.blocks()));
    }

    /// Otomatik boyutun WC94 ekstrapolasyon uyarısı arayüz ve levha içinde
    /// de çıkar (önce yalnızca kabukta); elle boyutta çıkmaz.
    #[test]
    fn dalma_batmada_boyut_ekstrapolasyon_uyarisi() {
        let warn = Issue::RuptureSizeExtrapolated { max: 7.9 };
        for (setting, fault, depth) in [
            (Setting::Interface, FaultType::Reverse, 25.0),
            (Setting::Intraslab, FaultType::Normal, 60.0),
        ] {
            let mut q = quake();
            q.setting = setting;
            q.fault = fault;
            q.depth_km = depth;
            q.mw = 8.2;
            assert_eq!(validate(&q), vec![warn], "{setting:?}");
            q.rupture_km = Some([300.0, 60.0]);
            assert!(!validate(&q).contains(&warn), "{setting:?}");
        }
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
