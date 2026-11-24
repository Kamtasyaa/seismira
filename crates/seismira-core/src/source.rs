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
//! taşınır (azimutal eşit uzaklık izdüşümü). Merkez üssüne uzaklık ve azimut
//! tamdır; dik yönde ölçek hatası ≈ (d/R)²/6: CY14'ün 300 km sınırında
//! %0,04, Parker vd. (2020)'nin 1000 km sınırında %0,4 (~4 km). Şiddete
//! etkisi uzakta ihmal edilebilir (2026-11-20).

use crate::geo;
use crate::quake::{FaultType, Quake, Setting};

/// Sığ kabukta sismojenik tabanın derinliği (km). VARSAYIM: CLAUDE.md
/// "~15–25 km (doğrula)" diyor; aktif kabuk için orta değer seçildi.
/// Hiposantr daha derindeyse taban hiposantrı kapsayacak kadar iner.
pub const SEISMOGENIC_BOTTOM_KM: f64 = 20.0;

/// Dalma-batma arayüzünün sismojenik alt sınırı (km): arayüz depremlerinin
/// en derin olduğu yerde levha yüzeyinin ortalama derinliği, "Z_BC".
/// Kaynak: Contreras vd., PEER Report 2020/02 (NGA-Sub veri tabanı),
/// Bölüm 4.4, s. 65–66 (Hayes vd. 2012; Tichelaar & Ruff 1993'e dayanır):
/// "50 km in ALK and CAM, 55 km in JPN and SAM". Bölgeden bağımsız alt
/// değer seçildi (55 km genişliği ~%10 artırırdı). Arayüz fayı bu
/// derinliği aşamaz: büyük depremde genişlik doyar, alan korunarak
/// uzunluk artar (kabuktaki [`SEISMOGENIC_BOTTOM_KM`] gibi).
pub const INTERFACE_BOTTOM_KM: f64 = 50.0;

/// Arayüz depreminde kırılma genişliğinin üst sınırı (km). Kaynak: Allen &
/// Hayes (2017), BSSA 107(3), 1240–1253, doi:10.1785/0120160255, ÖZET (USGS
/// Publications Warehouse 70189636; tam metin kapalı erişim, görülmedi):
/// "the down-dip rupture width for interface earthquakes saturates near
/// 200 km (196 km on average)", yaklaşık Mw > 8,6 için. [`INTERFACE_BOTTOM_KM`]
/// sınırı çok yatık arayüzlerde (eğim < ~15°) bundan geniş fay verirdi.
pub const INTERFACE_MAX_WIDTH_KM: f64 = 196.0;

/// Fay alanı (km²), Wells & Coppersmith (1994, BSSA 84(4), 974-1002),
/// Tablo 2A, "tüm fay tipleri" M–RA bağıntısı M = 4.07 + 0.98·log10(A)'nın
/// tersi. USGS nshmp-lib de alanı bu tersten hesaplar
/// (`RuptureScaling.lengthCa08`, M < 6.9 dalı).
///
/// Birincil kaynaktan DOĞRULANDI (2026-11-15, makale s. 990, Tablo 2A,
/// "M = a + b·log(RA)", "All" satırı): 148 deprem, a = 4.07 (±0.06),
/// b = 0.98 (±0.03), s = 0.24, r = 0.95, büyüklük aralığı 4.8–7.9, alan
/// aralığı 2.2–5 184 km². Aralığın üstü için bkz. [`WC94_MW_MAX`].
pub fn area_wc94(mw: f64) -> f64 {
    10f64.powf((mw - 4.07) / 0.98)
}

/// [`area_wc94`] bağıntısının veri aralığının üst sınırı (Tablo 2A: 4.8–7.9).
/// Üstünde alan ekstrapolasyondur (Mw 8.0'da ~10 200 km², verideki en
/// büyük alanın iki katı). Girdi denetimi bunu AÇIKÇA uyarır (engellemez).
/// Sığ kabuk doğrultu atımlı faylar WC94 yerine [`length_tmg17_ss`]
/// kullanır (2026-11-24): WC94 + 20 km genişlik orada Mw 8.0'da 512 km,
/// 8.5'te 1 657 km veriyordu.
/// Alt sınırın (4.8) altı uyarılmaz: Mw 4.0'da alan ~0.9 km², kaynak
/// pratikte noktadır ve mesafelere etkisi 1 km'nin altındadır.
pub const WC94_MW_MAX: f64 = 7.9;

/// Sığ kabuk doğrultu atımlı kırılma uzunluğu (km): Thingbaijam, Mai &
/// Goda (2017), "New empirical earthquake source-scaling laws", BSSA
/// 107(5), 2225–2246, doi:10.1785/0120170017. Tablo 1, "Strike-slip"
/// satırı: log10 L = −2.943 + 0.681·Mw (σ = 0.151; genel ortogonal
/// regresyon, sonlu fay modelleri). Birincil kaynaktan DOĞRULANDI
/// (2026-11-24, yazar kabul kopyası, Bristol: Tablo 1).
///
/// Veri aralığı Mw 5.38–8.70, L 6.0–580.0 km ([`TMG17_SS_MW`]). Uzunluk
/// genişlikten bağımsız regresyondur: genişlik sismojenik tabana göre
/// kırpılsa da uzunluk değişmez (makale: doğrultu atımlıda genişlik
/// büyümesi "strongly inhibited", L büyüklükle W'dan hızlı büyür). WC94
/// alanı + doymuş genişlik Mw 8.5'te ~1 650 km verirken bu ~700 km verir.
///
/// Kullanım (2026-11-24): sığ kabuk doğrultu atımlıda uzunluk =
/// min(WC94 alanından uzunluk, bu). Genişlik WC94 kolundaki gibi
/// (√A, tabana göre kırpılmış). 20 km genişlikte iki uzunluk Mw ≈ 7.4'te
/// eşitlenir; altında WC94 (değişiklik yok), üstünde TMG17. Neden yalnızca
/// TMG17 değil: doğrulamada (docs/dogrulama.md) TMG17 Mw ~7'de gözlenenden
/// uzun fay verdi (Kumamoto 67 km ↔ ~30–50; Düzce 83 km ↔ ~47) ve sapmayı
/// büyüttü; WC94 ise 7.9 üstünde ekstrapolasyonla patlıyor.
pub fn length_tmg17_ss(mw: f64) -> f64 {
    10f64.powf(-2.943 + 0.681 * mw)
}

/// Sığ kabuk doğrultu atımlı kırılma genişliği (km), aynı kaynak, Tablo 1:
/// log10 W = −0.543 + 0.261·Mw (σ = 0.105; veri W 6.5–50.0 km). Fay
/// kurulumunda KULLANILMIYOR (genişlik WC94 kolundan; Mw ≥ 7.4'te ikisi de
/// 20 km tabana kırpılır); karşılaştırma ve test için.
pub fn width_tmg17_ss(mw: f64) -> f64 {
    10f64.powf(-0.543 + 0.261 * mw)
}

/// [`length_tmg17_ss`] / [`width_tmg17_ss`] veri aralığı (Mw), Tablo 1.
/// Kabukta doğrultu atımlı girdi sınırı (8.5) bunun içinde kalır.
pub const TMG17_SS_MW: (f64, f64) = (5.38, 8.70);

/// Otomatik kırılma uzunluğu TMG17 ile sınırlanıyor mu (yalnızca sığ
/// kabuk doğrultu atımlı); değilse yalnızca WC94 alanı.
pub fn uses_tmg17_ss(q: &Quake) -> bool {
    q.setting == Setting::ShallowCrust && q.fault == FaultType::StrikeSlip
}

/// Otomatik kırılma boyutu bağıntısının veri aralığı üst sınırı (Mw):
/// üstü ekstrapolasyondur. Doğrultu atımlıda Mw > 7.9'da uzunluk her zaman
/// TMG17'den gelir (WC94 uzunluğu ~7.4'ten sonra daha uzun) ([`crate::quake::Issue::RuptureSizeExtrapolated`]).
pub fn auto_size_mw_max(q: &Quake) -> f64 {
    if uses_tmg17_ss(q) {
        TMG17_SS_MW.1
    } else {
        WC94_MW_MAX
    }
}

/// Ortalama kırılma hızı (km/s), fay tipine göre. Vera, Tilmann & Saul
/// (2024), "A decade of short-period earthquake rupture histories from
/// multi-array back-projection", JGR Solid Earth 129, e2023JB027260 (CC BY),
/// Bölüm 3.5.2: 2010–2022 tüm Mw ≥ 7.5 depremleri (56); ters fay ortalaması
/// 2,1 km/s (%56 Vs; aralık %25–86), normal 2,5 km/s (%60), doğrultu atımlı
/// 2,7 km/s (%78; bir kısmı Vs'yi aşar). Notlar (makale): değerler yatay
/// izdüşümden ve "en büyük ortalama hız"dır; eğimli faylarda yatay izdüşüm
/// hızı ~%15'e kadar düşük gösterebilir. Küçük depremlerde fay kısa olduğu
/// için hızın zamanlamaya etkisi önemsizdir.
///
/// CLAUDE.md'deki "≈ 0,8·Vs ≈ 2,8 km/s (doğrula)" yalnızca doğrultu atımlı
/// faylarla uyumlu çıktı; ters faylar belirgin daha yavaş (2026-11-20).
pub fn rupture_velocity_kms(fault: FaultType) -> f64 {
    match fault {
        FaultType::Reverse => 2.1,
        FaultType::Normal => 2.5,
        FaultType::StrikeSlip => 2.7,
    }
}

/// Sahaya en yakın fay noktası (alt fay zamanlaması, Bölüm 7.3 "t_tepe").
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PeakPath {
    /// Odaktan bu noktaya fay düzlemi üzerinde uzaklık (km); kırılma
    /// zamanı = bu / kırılma hızı.
    pub rupture_km: f64,
    /// Noktanın derinliği (km).
    pub depth_km: f64,
    /// Noktanın yüzey izdüşümünden sahaya yatay uzaklık (km).
    pub horiz_km: f64,
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
    /// Hiposantrın düzlem koordinatları (doğrultu boyunca, eğim boyunca; km).
    hypo_plane: [f64; 2],
}

struct PlaneCoords {
    u: f64,
    w: f64,
    v: f64,
    n: f64,
    cu: f64,
    cv: f64,
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
    /// aşamaz. Otomatik boyut: WC94 alanı, genişlik doyunca alan korunacak
    /// şekilde uzunluk artar; sığ kabuk doğrultu atımlıda uzunluk TMG17
    /// uzunluğuyla sınırlıdır ([`length_tmg17_ss`]).
    /// `Quake::rupture_km` verilmişse boyutlar ondan (genişlik kırpılabilir).
    pub fn from_quake(q: &Quake) -> Self {
        let dip = q.dip.clamp(1.0, 90.0);
        let sin_d = dip.to_radians().sin();
        let bottom = match q.setting {
            Setting::ShallowCrust => SEISMOGENIC_BOTTOM_KM.max(q.depth_km + 1.0),
            Setting::Interface => INTERFACE_BOTTOM_KM.max(q.depth_km + 1.0),
            // Levha içi: genişlik sınırı yok (VARSAYIM; levha kalınlığına
            // bağlı bir sınır için kaynak bulunmadı).
            Setting::Intraslab => f64::INFINITY,
        };
        let w_max = match q.setting {
            Setting::Interface => (bottom / sin_d).min(INTERFACE_MAX_WIDTH_KM),
            _ => bottom / sin_d,
        };
        let (length, width) = match q.rupture_km {
            // Elle boyut: genişlik tabana sığacak kadar kırpılır, uzunluk
            // olduğu gibi (alan korunmaz; kullanıcı ne verdiyse).
            Some([l, w]) => (l, w.min(w_max)),
            None => {
                let area = area_wc94(q.mw);
                let width = area.sqrt().min(w_max);
                let length = area / width;
                // Doğrultu atımlı kabuk: WC94 uzunluğu TMG17'yi aşarsa
                // TMG17 (Mw ≳ 7.4; alan korunmaz). Gerekçe: [`length_tmg17_ss`].
                if uses_tmg17_ss(q) {
                    (length.min(length_tmg17_ss(q.mw)), width)
                } else {
                    (length, width)
                }
            }
        };
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
            hypo_plane: [length * along, width * down_dip_frac],
        }
    }

    /// Yatay birim vektörler: doğrultu yönü ve eğim (yatay) yönü (x doğu, y kuzey).
    pub(crate) fn axes(strike: f64) -> ([f64; 2], [f64; 2]) {
        let a = strike.to_radians();
        let s = [a.sin(), a.cos()];
        let d = [a.cos(), -a.sin()]; // doğrultunun 90° saat yönü
        (s, d)
    }

    /// Üst kenarın iki ucu (boylam, enlem): yüzey izi / önizleme çizgisi.
    /// Üst kenarın doğrultu boyunca başlangıç noktası (yerel km).
    pub(crate) fn top_start(&self) -> [f64; 2] {
        self.top_start
    }

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

    pub(crate) fn local_to_lonlat(&self, p: [f64; 2]) -> (f64, f64) {
        let dist = (p[0] * p[0] + p[1] * p[1]).sqrt();
        let az = p[0].atan2(p[1]).to_degrees();
        geo::destination(self.epicenter, az, dist)
    }

    pub(crate) fn lonlat_to_local(&self, site: (f64, f64)) -> ([f64; 2], f64) {
        let (km, az) = geo::distance_azimuth(self.epicenter, site);
        let a = az.to_radians();
        ([km * a.sin(), km * a.cos()], km)
    }

    /// Yüzeydeki bir noktanın düzlem koordinatları: (u doğrultu boyunca,
    /// w eğim yönünde yatay, v eğim boyunca düzlem içinde, n normal) ve
    /// düzlemin en yakın noktası (cu, cv).
    fn plane_coords(&self, p: [f64; 2]) -> PlaneCoords {
        let (s, d) = Self::axes(self.strike);
        let (sin_d, cos_d) = (self.dip.to_radians().sin(), self.dip.to_radians().cos());
        let rel = [p[0] - self.top_start[0], p[1] - self.top_start[1]];
        let u = rel[0] * s[0] + rel[1] * s[1];
        let w = rel[0] * d[0] + rel[1] * d[1];
        // Fay düzlemi koordinatları: doğrultu ekseni (u), eğim ekseni (v,
        // düzlem içinde aşağı), normal. Saha yüzeyde: z = 0; üst kenar
        // z = Ztor.
        let dz = -self.ztor; // saha – üst kenar (aşağı pozitif)
        let v = w * cos_d + dz * sin_d;
        let n = -w * sin_d + dz * cos_d;
        PlaneCoords {
            u,
            w,
            v,
            n,
            cu: u.clamp(0.0, self.length),
            cv: v.clamp(0.0, self.width),
        }
    }

    /// Sahaya en yakın fay noktası ve odaktan oraya kırılma yolu.
    pub fn peak_path(&self, site: (f64, f64)) -> PeakPath {
        let (p, _) = self.lonlat_to_local(site);
        self.peak_path_local(p)
    }

    /// [`Fault::peak_path`], yerel yatay konum için.
    pub fn peak_path_local(&self, p: [f64; 2]) -> PeakPath {
        let c = self.plane_coords(p);
        let (s, d) = Self::axes(self.strike);
        let (sin_d, cos_d) = (self.dip.to_radians().sin(), self.dip.to_radians().cos());
        // En yakın noktanın yüzey izdüşümü (yerel) ve derinliği.
        let along_dip = c.cv * cos_d;
        let q = [
            self.top_start[0] + s[0] * c.cu + d[0] * along_dip,
            self.top_start[1] + s[1] * c.cu + d[1] * along_dip,
        ];
        PeakPath {
            rupture_km: (c.cu - self.hypo_plane[0]).hypot(c.cv - self.hypo_plane[1]),
            depth_km: self.ztor + c.cv * sin_d,
            horiz_km: (p[0] - q[0]).hypot(p[1] - q[1]),
        }
    }

    /// Yerel yatay konumdaki bir saha için mesafeler.
    pub fn distances_local(&self, p: [f64; 2], repi: f64) -> Distances {
        let cos_d = self.dip.to_radians().cos();
        let PlaneCoords { u, w, v, n, cu, cv } = self.plane_coords(p);

        // Rx: üst kenar izine dik yatay mesafe, eğim yönü pozitif.
        let rx = w;
        // Rjb: yüzey izdüşümü dikdörtgeni [0, L] × [0, W cos δ].
        let du = (-u).max(0.0).max(u - self.length);
        let dw = (-w).max(0.0).max(w - self.width * cos_d);
        let rjb = (du * du + dw * dw).sqrt();
        // Rrup: 3B nokta–dikdörtgen.
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

    /// Mesafeler ve tepe yolu birlikte (jeodezik dönüşüm bir kez).
    pub fn distances_and_peak(&self, site: (f64, f64)) -> (Distances, PeakPath) {
        let (p, repi) = self.lonlat_to_local(site);
        (self.distances_local(p, repi), self.peak_path_local(p))
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

    /// En yakın nokta: merkez üssünde odağın tam üstündeki üst kenar
    /// (kırılma odaktan yukarı Ztor'a kadar ilerler); fayın ucunun ötesinde
    /// uç nokta. Dikey doğrultu atımlı fay, doğrultu 90° (doğu).
    #[test]
    fn tepe_yolu_en_yakin_fay_noktasi() {
        let f = Fault::from_quake(&quake(7.6, 10.0, FaultType::StrikeSlip, 90.0));
        let at = f.peak_path_local([0.0, 0.0]);
        assert!((at.rupture_km - (10.0 - f.ztor)).abs() < 1e-9, "{at:?}");
        assert!(at.horiz_km < 1e-9 && (at.depth_km - f.ztor).abs() < 1e-9);
        // Doğuda, fayın ucundan 50 km ötede: kırılma yarım uzunluk kadar
        // (ve derinlik farkı kadar) yol alır; yatay uzaklık 50 km.
        let half = f.length / 2.0;
        let far = f.peak_path_local([half + 50.0, 0.0]);
        assert!((far.horiz_km - 50.0).abs() < 1e-6, "{far:?}");
        // Saha yüzeyde: en yakın nokta üst kenarda (Ztor), odak 10 km'de.
        let dv = 10.0 - f.ztor;
        assert!((far.rupture_km - half.hypot(dv)).abs() < 1e-6, "{far:?}");
        assert!((far.depth_km - f.ztor).abs() < 1e-9);
    }

    #[test]
    fn wc94_alan() {
        // M = 4.07 + 0.98 log A → M 6.03 için A = 100 km².
        assert!((area_wc94(4.07 + 0.98 * 2.0) - 100.0).abs() < 1e-9);
    }

    #[test]
    fn genislik_doyunca_uzunluk_artar() {
        // Ters fay (WC94): genişlik tabanla sınırlı, alan korunur.
        let mut q = quake(7.6, 15.0, FaultType::Reverse, 90.0);
        q.dip = 90.0;
        let f = Fault::from_quake(&q);
        assert!((f.width - SEISMOGENIC_BOTTOM_KM).abs() < 1e-9);
        assert!((f.length * f.width - area_wc94(7.6)).abs() < 1e-6);
        assert!(f.length > 100.0);
        assert!(f.ztor >= 0.0 && f.ztor + f.width <= SEISMOGENIC_BOTTOM_KM + 1e-9);
        // Küçük depremde kare düzlem, hiposantr eğim boyunca ortada.
        let g = Fault::from_quake(&quake(5.0, 10.0, FaultType::Reverse, 0.0));
        assert!((g.length - g.width).abs() < 1e-9);
        let half_v = g.width / 2.0 * g.dip.to_radians().sin();
        assert!((g.ztor + half_v - 10.0).abs() < 1e-9);
    }

    /// Thingbaijam vd. (2017) Tablo 1, doğrultu atımlı: Mw 8.0'da
    /// log10 L = −2.943 + 5.448 = 2.505 → 319.9 km; log10 W = −0.543 +
    /// 2.088 = 1.545 → 35.08 km. Mw 7.0: L = 10^1.824 = 66.68 km.
    #[test]
    fn tmg17_dogrultu_atimli() {
        assert!((length_tmg17_ss(8.0) - 10f64.powf(2.505)).abs() < 1e-9);
        assert!((length_tmg17_ss(8.0) - 319.9).abs() < 0.1);
        assert!((width_tmg17_ss(8.0) - 35.08).abs() < 0.01);
        assert!((length_tmg17_ss(7.0) - 66.68).abs() < 0.01);
        // Fayda: dikey fay, genişlik tabana (20 km) kırpılır, uzunluk TMG17
        // (WC94: 10 200 / 20 ≈ 512 km).
        let f = Fault::from_quake(&quake(8.0, 10.0, FaultType::StrikeSlip, 90.0));
        assert!((f.width - SEISMOGENIC_BOTTOM_KM).abs() < 1e-9, "{f:?}");
        assert!((f.length - length_tmg17_ss(8.0)).abs() < 1e-9, "{f:?}");
        // Mw 8.5: ~700 km (WC94 + 20 km ile ~1 657 km idi).
        let g = Fault::from_quake(&quake(8.5, 10.0, FaultType::StrikeSlip, 90.0));
        assert!((690.0..710.0).contains(&g.length), "{g:?}");
        // Mw 7.4 altında WC94 aynen (alan korunur): Mw 7.0 → 977 / 20 ≈ 49 km.
        let h = Fault::from_quake(&quake(7.0, 10.0, FaultType::StrikeSlip, 0.0));
        assert!((h.length * h.width - area_wc94(7.0)).abs() < 1e-6, "{h:?}");
        assert!(h.length < length_tmg17_ss(7.0), "{h:?}");
        // Kesişim (20 km genişlik): 1.0204·M − 5.454 = 0.681·M − 2.943 → M ≈ 7.40.
        for m in [7.35, 7.45] {
            let k = Fault::from_quake(&quake(m, 10.0, FaultType::StrikeSlip, 0.0));
            let wc = area_wc94(m) / k.width;
            assert_eq!(k.length < wc, m > 7.4, "{m}: {k:?} wc {wc}");
        }
        // Uzunluk büyüklükle tekdüze artar (kesişimde sıçrama yok).
        let mut prev = 0.0;
        for i in 0..=40 {
            let m = 6.5 + 0.05 * f64::from(i);
            let l = Fault::from_quake(&quake(m, 10.0, FaultType::StrikeSlip, 0.0)).length;
            assert!(l > prev, "{m}: {l} ≤ {prev}");
            prev = l;
        }
        // Ters fay WC94'te kalır.
        assert!(!uses_tmg17_ss(&quake(8.0, 10.0, FaultType::Reverse, 0.0)));
    }

    /// 2011 Tōhoku benzeri: Mw 9, eğim 14°, odak 29 km. Genişlik doyar:
    /// 50 / sin 14° ≈ 207 km, Allen & Hayes (2017) üst sınırı 196 km; L = A / W
    /// ≈ 549 km (gözlenen kırılma ~450–500 × 200 km). Sınır olmadan kare
    /// (~328 × 328 km) çıkıyordu. Eğim 25°'de 50 km taban sınırı bağlar.
    #[test]
    fn arayuz_genisligi_doyar() {
        let mut q = quake(9.0, 29.0, FaultType::Reverse, 193.0);
        q.setting = Setting::Interface;
        q.dip = 14.0;
        let f = Fault::from_quake(&q);
        assert!((f.width - INTERFACE_MAX_WIDTH_KM).abs() < 1e-9, "{f:?}");
        assert!((f.length * f.width - area_wc94(9.0)).abs() < 1e-6);
        assert!((540.0..560.0).contains(&f.length), "{f:?}");
        assert!(
            f.ztor >= 0.0
                && f.ztor + f.width * 14f64.to_radians().sin() <= INTERFACE_BOTTOM_KM + 1e-9
        );
        // Dik arayüzde taban sınırı bağlar: 50 / sin 25° ≈ 118 km.
        let mut steep = q;
        steep.dip = 25.0;
        let f = Fault::from_quake(&steep);
        let w_max = INTERFACE_BOTTOM_KM / 25f64.to_radians().sin();
        assert!((f.width - w_max).abs() < 1e-9, "{f:?}");
        // Küçük arayüz depremi doymaz (kare).
        q.mw = 7.0;
        let f = Fault::from_quake(&q);
        assert!((f.width - f.length).abs() < 1e-9, "{f:?}");
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
