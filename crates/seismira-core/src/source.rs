//! Sonlu fay kaynağı: boyutlar, düzlemin yerleşimi ve mesafe ölçüleri
//! (CLAUDE.md Bölüm 7.1).
//!
//! Fay tek dikdörtgen düzlemdir. Uzlaşım (Aki & Richards): doğrultu
//! yönüne bakıldığında fay SAĞA doğru eğimlidir; tavan bloğu (hanging wall)
//! eğim yönündedir.
//!
//! Mesafe ölçüleri NGA-West2 tanımlarıyla (Ancheta vd. 2014, Earthquake
//! Spectra 30(3)): Rrup (düzleme en kısa 3B mesafe), Rjb (yüzey izdüşümüne
//! en kısa yatay mesafe), Rx (üst kenar izine dik yatay mesafe, tavan bloğu
//! tarafı pozitif), Ztor (üst kenar derinliği).
//!
//! Saha, merkez üssü çevresinde yerel düzleme jeodezik mesafe ve azimutla
//! taşınır (azimutal eşit uzaklık izdüşümü). CY14'ün 300 km sınırı içinde
//! bu yaklaşımın hatası ihmal edilebilir.

use crate::geo;
use crate::quake::{FaultType, Quake, Setting};

/// Sığ kabukta sismojenik tabanın derinliği (km). VARSAYIM: CLAUDE.md
/// "~15–25 km (doğrula)" diyor; aktif kabuk için orta değer seçildi.
/// Hiposantr daha derindeyse taban hiposantrı kapsayacak kadar iner.
pub const SEISMOGENIC_BOTTOM_KM: f64 = 20.0;

/// Fay alanı (km²), Wells & Coppersmith (1994, BSSA 84(4), 974-1002),
/// Tablo 2A, "tüm fay tipleri" M–RA bağıntısı M = 4.07 + 0.98·log10(A)'nın
/// tersi. USGS nshmp-lib de alanı bu tersten hesaplar
/// (`RuptureScaling.lengthCa08`, M < 6.9 dalı).
pub fn area_wc94(mw: f64) -> f64 {
    10f64.powf((mw - 4.07) / 0.98)
}

/// Yerleşmiş fay düzlemi. Yerel koordinatlar km cinsinden: x doğu, y kuzey,
/// z aşağı; orijin merkez üssü (hiposantrın yüzeydeki izdüşümü).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fault {
    pub epicenter: (f64, f64),
    pub hypo_depth: f64,
    pub strike: f64,
    pub dip: f64,
    pub fault: FaultType,
    pub length: f64,
    pub width: f64,
    pub ztor: f64,
    /// Üst kenarın doğrultu boyunca başlangıç noktası (yerel x, y).
    top_start: [f64; 2],
}

/// Bir saha için mesafe ölçüleri (km).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Distances {
    pub rrup: f64,
    pub rjb: f64,
    pub rx: f64,
    /// Merkez üssüne jeodezik yüzey mesafesi (km).
    pub repi: f64,
}

impl Fault {
    /// Depremden fay düzlemini kurar. Hiposantrın düzlemdeki konumu
    /// `Quake::hypo_along` ve `hypo_down` ile (varsayılan ikisi de ortada);
    /// düzlem yüzeyin üstüne çıkamaz (Ztor ≥ 0) ve kabukta sismojenik tabanı
    /// aşamaz. Genişlik doyunca alan korunacak şekilde uzunluk artar.
    pub fn from_quake(q: &Quake) -> Self {
        let dip = q.dip.clamp(1.0, 90.0);
        let sin_d = dip.to_radians().sin();
        let area = area_wc94(q.mw);
        let bottom = match q.setting {
            Setting::ShallowCrust => SEISMOGENIC_BOTTOM_KM.max(q.depth_km + 1.0),
            // Faz 5'te Slab2 ve Strasser vd. (2010) ile değişecek; şimdilik
            // derinlik sınırı yok.
            Setting::Interface | Setting::Intraslab => f64::INFINITY,
        };
        let w_max = bottom / sin_d;
        let width = area.sqrt().min(w_max);
        let length = area / width;
        let vertical = width * sin_d;
        // Hiposantr eğim boyunca ortada; düzlem [0, taban] içinde tutulur.
        let down = q.hypo_down.clamp(0.0, 1.0);
        let along = q.hypo_along.clamp(0.0, 1.0);
        let ztor = (q.depth_km - down * vertical).clamp(0.0, (bottom - vertical).max(0.0));
        let down_dip_frac = if vertical > 0.0 {
            (q.depth_km - ztor) / vertical
        } else {
            0.0
        };
        // Hiposantrın yatay konumu orijin; üst kenar ondan eğim yönünün
        // tersine, yatayda down_dip_frac·W·cos δ uzakta.
        let (s, d) = Self::axes(q.strike);
        let back = down_dip_frac * width * dip.to_radians().cos();
        let top_mid = [-d[0] * back, -d[1] * back];
        let top_start = [
            top_mid[0] - s[0] * length * along,
            top_mid[1] - s[1] * length * along,
        ];
        Self {
            epicenter: q.epicenter,
            hypo_depth: q.depth_km,
            strike: q.strike,
            dip,
            fault: q.fault,
            length,
            width,
            ztor,
            top_start,
        }
    }

    /// Yatay birim vektörler: doğrultu yönü ve eğim (yatay) yönü (x doğu, y kuzey).
    fn axes(strike: f64) -> ([f64; 2], [f64; 2]) {
        let a = strike.to_radians();
        let s = [a.sin(), a.cos()];
        let d = [a.cos(), -a.sin()]; // doğrultunun 90° saat yönü
        (s, d)
    }

    /// Üst kenarın iki ucu (boylam, enlem): yüzey izi / önizleme çizgisi.
    pub fn top_edge_lonlat(&self) -> [(f64, f64); 2] {
        let (s, _) = Self::axes(self.strike);
        let a = self.top_start;
        let b = [a[0] + s[0] * self.length, a[1] + s[1] * self.length];
        [self.local_to_lonlat(a), self.local_to_lonlat(b)]
    }

    /// Yüzey izdüşümünün dört köşesi (boylam, enlem).
    pub fn surface_outline_lonlat(&self) -> [(f64, f64); 4] {
        let (s, d) = Self::axes(self.strike);
        let h = self.width * self.dip.to_radians().cos();
        let a = self.top_start;
        let b = [a[0] + s[0] * self.length, a[1] + s[1] * self.length];
        let c = [b[0] + d[0] * h, b[1] + d[1] * h];
        let e = [a[0] + d[0] * h, a[1] + d[1] * h];
        [a, b, c, e].map(|p| self.local_to_lonlat(p))
    }

    fn local_to_lonlat(&self, p: [f64; 2]) -> (f64, f64) {
        let dist = (p[0] * p[0] + p[1] * p[1]).sqrt();
        let az = p[0].atan2(p[1]).to_degrees();
        geo::destination(self.epicenter, az, dist)
    }

    fn lonlat_to_local(&self, site: (f64, f64)) -> ([f64; 2], f64) {
        let (km, az) = geo::distance_azimuth(self.epicenter, site);
        let a = az.to_radians();
        ([km * a.sin(), km * a.cos()], km)
    }

    /// Yerel yatay konumdaki bir saha için mesafeler.
    pub fn distances_local(&self, p: [f64; 2], repi: f64) -> Distances {
        let (s, d) = Self::axes(self.strike);
        let (sin_d, cos_d) = (self.dip.to_radians().sin(), self.dip.to_radians().cos());
        let rel = [p[0] - self.top_start[0], p[1] - self.top_start[1]];
        let u = rel[0] * s[0] + rel[1] * s[1]; // doğrultu boyunca
        let w = rel[0] * d[0] + rel[1] * d[1]; // eğim yönünde (yatay)

        // Rx: üst kenar izine dik yatay mesafe, eğim yönü pozitif.
        let rx = w;
        // Rjb: yüzey izdüşümü dikdörtgeni [0, L] × [0, W cos δ].
        let du = (-u).max(0.0).max(u - self.length);
        let dw = (-w).max(0.0).max(w - self.width * cos_d);
        let rjb = (du * du + dw * dw).sqrt();
        // Rrup: 3B nokta–dikdörtgen. Fay düzlemi koordinatları: doğrultu
        // ekseni (u), eğim ekseni (v, düzlem içinde aşağı), normal.
        // Saha yüzeyde: z = 0; üst kenar z = Ztor.
        let dz = -self.ztor; // saha – üst kenar (aşağı pozitif)
        let v = w * cos_d + dz * sin_d;
        let n = -w * sin_d + dz * cos_d;
        let cu = u.clamp(0.0, self.length);
        let cv = v.clamp(0.0, self.width);
        let rrup = ((u - cu).powi(2) + (v - cv).powi(2) + n * n).sqrt();
        Distances {
            rrup,
            rjb,
            rx,
            repi,
        }
    }

    /// Rrup = `limit_km` eş-uzaklık eğrisi (boylam, enlem), `n` azimutta;
    /// kapalı (son nokta ilkinin tekrarı). Model geçerlilik sınırını haritada
    /// göstermek için. Her azimutta merkez üssünden dışa ikiye bölme ile
    /// aranır (Rrup, ışın boyunca dışarıda monoton artar).
    pub fn rrup_contour(&self, limit_km: f64, n: usize) -> Vec<(f64, f64)> {
        (0..=n)
            .map(|i| {
                let az = 360.0 * i as f64 / n as f64;
                let dir = [az.to_radians().sin(), az.to_radians().cos()];
                let rrup = |r: f64| self.distances_local([dir[0] * r, dir[1] * r], r).rrup;
                let (mut lo, mut hi) = (0.0, limit_km + self.length + self.width);
                for _ in 0..40 {
                    let mid = 0.5 * (lo + hi);
                    if rrup(mid) < limit_km {
                        lo = mid;
                    } else {
                        hi = mid;
                    }
                }
                geo::destination(self.epicenter, az, 0.5 * (lo + hi))
            })
            .collect()
    }

    pub fn distances(&self, site: (f64, f64)) -> Distances {
        let (p, repi) = self.lonlat_to_local(site);
        self.distances_local(p, repi)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quake(mw: f64, depth: f64, fault: FaultType, strike: f64) -> Quake {
        let mut q = Quake::new((30.0, 40.0));
        q.mw = mw;
        q.depth_km = depth;
        q.fault = fault;
        (q.dip, q.rake) = fault.default_dip_rake();
        q.strike = strike;
        q
    }

    #[test]
    fn wc94_alan() {
        // M = 4.07 + 0.98 log A → M 6.03 için A = 100 km².
        assert!((area_wc94(4.07 + 0.98 * 2.0) - 100.0).abs() < 1e-9);
    }

    #[test]
    fn genislik_doyunca_uzunluk_artar() {
        let f = Fault::from_quake(&quake(7.6, 15.0, FaultType::StrikeSlip, 90.0));
        // Dikey fay: genişlik sismojenik kalınlıkla sınırlı.
        assert!((f.width - SEISMOGENIC_BOTTOM_KM).abs() < 1e-9);
        assert!((f.length * f.width - area_wc94(7.6)).abs() < 1e-6);
        assert!(f.length > 100.0);
        assert!(f.ztor >= 0.0 && f.ztor + f.width <= SEISMOGENIC_BOTTOM_KM + 1e-9);
        // Küçük depremde kare düzlem, hiposantr ortada.
        let g = Fault::from_quake(&quake(5.0, 10.0, FaultType::StrikeSlip, 0.0));
        assert!((g.length - g.width).abs() < 1e-9);
        assert!((g.ztor + g.width / 2.0 - 10.0).abs() < 1e-9);
    }

    #[test]
    fn hiposantr_konumu_fayi_kaydirir() {
        // Doğrultu 90° (doğu): hypo_along = 0 → fay tamamen doğuda.
        let mut q = quake(7.0, 10.0, FaultType::StrikeSlip, 90.0);
        q.hypo_along = 0.0;
        let f = Fault::from_quake(&q);
        let west = f.distances_local([-5.0, 0.0], 5.0);
        let east = f.distances_local([5.0, 0.0], 5.0);
        assert!((west.rjb - 5.0).abs() < 1e-9, "{west:?}");
        assert!(east.rjb.abs() < 1e-9, "{east:?}");
    }

    #[test]
    fn rrup_esuzaklik_egrisi() {
        let f = Fault::from_quake(&quake(7.6, 15.0, FaultType::StrikeSlip, 85.0));
        for p in f.rrup_contour(300.0, 36) {
            let d = f.distances(p);
            assert!((d.rrup - 300.0).abs() < 0.5, "{d:?}");
        }
    }

    #[test]
    fn ztor_yuzeyin_ustune_cikmaz() {
        let f = Fault::from_quake(&quake(7.0, 3.0, FaultType::Reverse, 0.0));
        assert_eq!(f.ztor, 0.0);
    }

    #[test]
    fn dikey_fay_mesafeleri() {
        // Doğrultu 90° (doğu), dikey, yüzeyde kırılmış: üst kenar y = 0
        // boyunca x ∈ [-L/2, L/2].
        let mut q = quake(6.0, 2.0, FaultType::StrikeSlip, 90.0);
        q.depth_km = 2.0;
        let f = Fault::from_quake(&q);
        assert_eq!(f.ztor, 0.0);
        let half = f.length / 2.0;
        // Fayın ortasının 10 km kuzeyi.
        let d = f.distances_local([0.0, 10.0], 10.0);
        assert!((d.rjb - 10.0).abs() < 1e-9 && (d.rrup - 10.0).abs() < 1e-9);
        // Doğrultu 90°'de eğim yönü güney; kuzey taban bloğu: Rx < 0.
        assert!((d.rx + 10.0).abs() < 1e-9);
        // Ucun 5 km ötesi, iz üzerinde.
        let d = f.distances_local([half + 5.0, 0.0], half + 5.0);
        assert!((d.rjb - 5.0).abs() < 1e-9 && (d.rrup - 5.0).abs() < 1e-9);
    }

    #[test]
    fn egimli_fay_tavan_blogu() {
        // Ters fay, doğrultu 0° (kuzey), eğim 40° doğuya; yüzeyde kırılmış.
        let mut q = quake(6.5, 0.0, FaultType::Reverse, 0.0);
        q.depth_km = 0.0;
        let f = Fault::from_quake(&q);
        assert_eq!(f.ztor, 0.0);
        let cos_d = 40f64.to_radians().cos();
        let tan_d = 40f64.to_radians().tan();
        // Tavan bloğunda, iz ile izdüşüm alt kenarı arasında bir saha:
        // Rjb = 0, Rx = x, Rrup = x·sin δ (düzleme dik uzaklık).
        let x = 0.5 * f.width * cos_d;
        let d = f.distances_local([x, f.length / 2.0], x);
        assert!(d.rjb.abs() < 1e-9);
        assert!((d.rx - x).abs() < 1e-9);
        assert!((d.rrup - x * 40f64.to_radians().sin()).abs() < 1e-9);
        // Taban bloğunda 10 km batı: Rrup = Rjb = 10, Rx = −10.
        let d = f.distances_local([-10.0, f.length / 2.0], 10.0);
        assert!(
            (d.rjb - 10.0).abs() < 1e-9
                && (d.rrup - 10.0).abs() < 1e-9
                && (d.rx + 10.0).abs() < 1e-9
        );
        let _ = tan_d;
    }

    #[test]
    fn jeodezik_ve_yerel_tutarli() {
        let q = quake(7.0, 10.0, FaultType::StrikeSlip, 45.0);
        let f = Fault::from_quake(&q);
        // Yüzey izinin uçları, kendi yerel konumlarına geri dönmeli.
        let [a, _] = f.top_edge_lonlat();
        let d = f.distances(a);
        assert!(d.rjb < 1e-6, "{d:?}");
        assert!((d.rrup - f.ztor).abs() < 1e-6, "{d:?}");
    }
}
