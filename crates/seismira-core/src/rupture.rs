//! Kırılma: bir ya da birkaç bitişik düzlem parçası (çok parçalı fay,
//! ör. 6 Şubat 2023 Pazarcık). Tek düzlem, tek parçalı özel haldir ve
//! sonuçları [`Fault`] ile birebir aynıdır (testler).
//!
//! Tanımlar (PEER Report 2013/03, Ancheta vd., NGA-West2 Database,
//! Bölüm 2.3 ve 4.2; birincil kaynaktan okundu 2026-11-25):
//! - Uzunluk ve alan parçaların TOPLAMI; genişlik parçaların EN BÜYÜĞÜ
//!   (Ek D "Total Fault Width: the maximum width").
//! - Doğrultu parça uzunluğuyla, eğim parça genişliğiyle ağırlıklı
//!   ("The fault strike and dip reported was based on a weighted sum of the
//!   segment length and width, respectively"). Doğrultu dairesel ortalama.
//! - Ztor: kırılma modelinin üst kenarı ("Depth to the top of the finite
//!   rupture model") → parçaların en sığı.
//! - Rx (ve Ry): GC2 ile; diğer mesafeler en yakın parçaya göre ("RX and
//!   RY were estimated using the GC2 methodology ... All other distances
//!   were based on the closest segment or minimum distance").
//!
//! GC2: Spudich & Chiou (2015), "Strike-parallel and strike-normal
//! coordinate system around geometrically complicated rupture traces—Use
//! by NGA-West2 and further improvements", USGS Open-File Report 2015-1028,
//! doi:10.3133/ofr20151028 (kamu malı). Tek iz denklemleri 1–10 (s. 3–5):
//! T = Σ wᵢtᵢ / Σ wᵢ, U = Σ wᵢ(uᵢ + sᵢ) / Σ wᵢ, wᵢ = ∫ r⁻² du (kapalı biçim
//! Eş. 4–5); iz üzerinde T = 0, U = uᵢ + sᵢ. "|T| = Rx" (s. 4), U ortak
//! orijin izin başındayken Ry. Çok izli (kopuk) kırılma uygulanmadı: iz
//! tek ve kesintisiz.
//!
//! Basitleştirme (BELGELENDİ): parçalar üst kenarlarından birleşik
//! dikdörtgenlerdir; dirseklerde aşağıda boşluk/örtüşme kalır (NGA-West2
//! bunları yamuklarla birleştirdi, s. 68). Mesafe en yakın parçaya göre
//! olduğundan etkisi dirsek çevresiyle sınırlı.

use crate::geo;
use crate::quake::{FaultType, Quake};
use crate::source::{self, Distances, Fault, PeakPath};

/// Kurulmuş kırılma. Yerel koordinatlar [`Fault`] ile aynı (km; x doğu,
/// y kuzey; orijin merkez üssü).
#[derive(Clone, Debug, PartialEq)]
pub struct Rupture {
    pub segments: Vec<Fault>,
    pub epicenter: (f64, f64),
    pub fault: FaultType,
    /// Uzunlukla ağırlıklı doğrultu (dairesel ortalama).
    pub strike: f64,
    /// Genişlikle ağırlıklı eğim.
    pub dip: f64,
    /// Toplam uzunluk (km).
    pub length: f64,
    /// En büyük genişlik (km).
    pub width: f64,
    /// En sığ üst kenar (km).
    pub ztor: f64,
    /// Üst kenar izinin noktaları (yerel km), parça sayısı + 1.
    trace: Vec<[f64; 2]>,
    /// Hiposantr: iz boyunca (km, izin başından) ve eğim boyunca (km).
    hypo_u: f64,
    hypo_v: f64,
    /// Yönelim (CY14 ΔDPP); yalnızca hesap bağlamında eklenir
    /// ([`Rupture::with_directivity`]), yoksa ΔDPP = 0.
    directivity: Option<Box<crate::directivity::Directivity>>,
}

impl Rupture {
    /// Depremden kırılma: iz varsa çok parçalı, yoksa tek düzlem
    /// ([`Fault::from_quake`]).
    pub fn from_quake(q: &Quake) -> Self {
        match q.trace {
            Some(t) if t.segments() >= 1 => Self::from_trace(q, &t),
            _ => Self::single(Fault::from_quake(q)),
        }
    }

    /// Tek düzlemli kırılma.
    pub fn single(f: Fault) -> Self {
        let (s, _) = Fault::axes(f.strike);
        let a = f.top_start();
        let b = [a[0] + s[0] * f.length, a[1] + s[1] * f.length];
        let [hu, hv] = f.hypo_plane();
        Self {
            epicenter: f.epicenter,
            fault: f.fault,
            strike: f.strike,
            dip: f.dip,
            length: f.length,
            width: f.width,
            ztor: f.ztor,
            trace: vec![a, b],
            hypo_u: hu,
            hypo_v: hv,
            segments: vec![f],
            directivity: None,
        }
    }

    fn from_trace(q: &Quake, t: &crate::quake::Trace) -> Self {
        let pts: Vec<[f64; 2]> = t
            .points()
            .iter()
            .map(|&ll| local(q.epicenter, ll))
            .collect();
        let n = pts.len() - 1;
        let lens: Vec<f64> = (0..n)
            .map(|i| (pts[i + 1][0] - pts[i][0]).hypot(pts[i + 1][1] - pts[i][1]))
            .collect();
        let dips: Vec<f64> = (0..n)
            .map(|i| t.segment_dip(i).unwrap_or(q.dip).clamp(1.0, 90.0))
            .collect();
        let length: f64 = lens.iter().sum();
        // Genişlik: elle verilmişse o, değilse tek düzlemdeki gibi √A_WC94
        // (tabana ve TMG17 normal sınırına kırpılmış). Uzunluk kullanıcının
        // çizdiği; alan korunmaz, büyüklükle uyumsuzsa denetim "olağan dışı
        // alan" der (2026-11-25: önce A / L idi; Mw 7, 326 km iz → 3 km
        // genişlik, uyarısız).
        let w_max = dips
            .iter()
            .map(|&d| source::width_limit(q, d))
            .fold(f64::INFINITY, f64::min);
        let width = match q.rupture_km {
            Some([_, w]) => w.min(w_max),
            None => source::area_wc94(q.mw)
                .sqrt()
                .min(source::auto_width_cap(q, w_max)),
        };
        // Ztor: tek düzlemdeki gibi hiposantrın eğim boyunca konumundan
        // (ortak eğimle); en dik parça tabana sığacak şekilde kırpılır.
        let sin_q = q.dip.clamp(1.0, 90.0).to_radians().sin();
        let max_vertical = dips
            .iter()
            .map(|d| width * d.to_radians().sin())
            .fold(0.0, f64::max);
        let bottom = source::bottom_km(q);
        let ztor = (q.depth_km - q.hypo_down.clamp(0.0, 1.0) * width * sin_q)
            .clamp(0.0, (bottom - max_vertical).max(0.0));
        let segments: Vec<Fault> = (0..n)
            .map(|i| {
                let (a, b) = (pts[i], pts[i + 1]);
                let strike = (b[0] - a[0])
                    .atan2(b[1] - a[1])
                    .to_degrees()
                    .rem_euclid(360.0);
                Fault::plane(q, strike, dips[i], lens[i], width, ztor, a, [0.0, 0.0])
            })
            .collect();
        // Uzunlukla ağırlıklı dairesel doğrultu ortalaması.
        let (se, ce) = segments.iter().fold((0.0, 0.0), |(se, ce), f| {
            let a = f.strike.to_radians();
            (se + f.length * a.sin(), ce + f.length * a.cos())
        });
        let mut r = Self {
            epicenter: q.epicenter,
            fault: q.fault,
            strike: se.atan2(ce).to_degrees().rem_euclid(360.0),
            // Genişlikler eşit: genişlikle ağırlıklı = aritmetik ortalama.
            dip: dips.iter().sum::<f64>() / n as f64,
            length,
            width,
            ztor,
            trace: pts,
            hypo_u: 0.0,
            hypo_v: ((q.depth_km - ztor) / sin_q).clamp(0.0, width),
            segments,
            directivity: None,
        };
        // Hiposantr iz boyunca: merkez üssünün GC2 U koordinatı (izin
        // dışındaysa uca çekilir).
        r.hypo_u = r.gc2([0.0, 0.0]).1.clamp(0.0, length);
        r
    }

    /// Yönelim hesabını ekler (CY14 ΔDPP; merkezleme tablosu ~10–50 ms).
    /// Yalnızca sığ kabukta ve Mw > 5,5'te anlamlı (CY14 f_M = 0 altında);
    /// çağıran karar verir ([`crate::shaking::Scenario`]).
    pub fn with_directivity(mut self, rake: f64) -> Self {
        let d = crate::directivity::Directivity::new(&self, rake);
        self.directivity = Some(Box::new(d));
        self
    }

    /// Hiposantrın iz boyunca (km, izin başından) ve eğim boyunca (km)
    /// konumu.
    pub fn hypocenter_plane(&self) -> (f64, f64) {
        (self.hypo_u, self.hypo_v)
    }

    /// GC2 (T, U) koordinatları, yerel yatay konum için (Spudich & Chiou
    /// 2015, Eş. 1–10). T eğim yönünde pozitif (tavan bloğu), U izin
    /// başından iz boyunca.
    pub fn gc2(&self, y: [f64; 2]) -> (f64, f64) {
        let (mut sw, mut swt, mut swu, mut s) = (0.0, 0.0, 0.0, 0.0);
        for k in 0..self.trace.len() - 1 {
            let (p, q) = (self.trace[k], self.trace[k + 1]);
            let l = (q[0] - p[0]).hypot(q[1] - p[1]);
            if l <= 0.0 {
                continue;
            }
            // ûᵢ doğrultu, t̂ᵢ = ûᵢ × ẑ (doğrultunun 90° saat yönü: eğim yönü).
            let uh = [(q[0] - p[0]) / l, (q[1] - p[1]) / l];
            let th = [uh[1], -uh[0]];
            let rel = [y[0] - p[0], y[1] - p[1]];
            let t = rel[0] * th[0] + rel[1] * th[1];
            let u = rel[0] * uh[0] + rel[1] * uh[1];
            let w = if t.abs() > 1e-9 {
                // Eş. 4 (−π/2 < tan⁻¹ < π/2).
                (((l - u) / t).atan() - (-u / t).atan()) / t
            } else if !(0.0..=l).contains(&u) {
                // Eş. 5: segmentin uzantısında.
                1.0 / (u - l) - 1.0 / u
            } else {
                // İz üzerinde: T = 0, U = uᵢ + sᵢ.
                return (0.0, u + s);
            };
            sw += w;
            swt += w * t;
            swu += w * (u + s);
            s += l;
        }
        (swt / sw, swu / sw)
    }

    /// Yerel yatay konumdaki saha için mesafeler: Rrup ve Rjb en yakın
    /// parçaya, Rx GC2 T'ye göre; yönelim eklendiyse ΔDPP.
    pub fn distances_local(&self, p: [f64; 2], repi: f64) -> Distances {
        let mut d = self.distances_local_raw_repi(p, repi);
        if let Some(dir) = &self.directivity {
            d.dpp = dir.centered(p, d.rrup);
        }
        d
    }

    /// Yönelimsiz mesafeler (repi 0; merkezleme tablosu için).
    pub(crate) fn distances_local_raw(&self, p: [f64; 2]) -> Distances {
        self.distances_local_raw_repi(p, 0.0)
    }

    fn distances_local_raw_repi(&self, p: [f64; 2], repi: f64) -> Distances {
        let (mut rrup, mut rjb) = (f64::INFINITY, f64::INFINITY);
        for f in &self.segments {
            let d = f.distances_local(p, repi);
            rrup = rrup.min(d.rrup);
            rjb = rjb.min(d.rjb);
        }
        let rx = if self.segments.len() == 1 {
            // Tek düzlem: T = t (Eş. 2, tek ağırlık); doğrudan.
            self.segments[0].distances_local(p, repi).rx
        } else {
            self.gc2(p).0
        };
        Distances {
            rrup,
            rjb,
            rx,
            repi,
            dpp: 0.0,
        }
    }

    /// Sahaya en yakın kırılma noktası ve hiposantrdan oraya kırılma yolu:
    /// iz boyunca (U farkı) ve eğim boyunca uzaklığın bileşkesi (kırılma
    /// hiposantrdan iki yöne iz boyunca yayılır).
    pub fn peak_path_local(&self, p: [f64; 2]) -> PeakPath {
        if self.segments.len() == 1 {
            return self.segments[0].peak_path_local(p);
        }
        let mut best = None::<(f64, usize, source::Closest)>;
        for (i, f) in self.segments.iter().enumerate() {
            let c = f.closest(p);
            if best.is_none_or(|(r, _, _)| c.rrup < r) {
                best = Some((c.rrup, i, c));
            }
        }
        let (_, i, c) = best.expect("en az bir parça");
        let s: f64 = self.segments[..i].iter().map(|f| f.length).sum();
        PeakPath {
            rupture_km: (s + c.cu - self.hypo_u).hypot(c.cv - self.hypo_v),
            depth_km: c.depth_km,
            horiz_km: c.horiz_km,
        }
    }

    pub fn distances(&self, site: (f64, f64)) -> Distances {
        let (p, repi) = self.lonlat_to_local(site);
        self.distances_local(p, repi)
    }

    pub fn peak_path(&self, site: (f64, f64)) -> PeakPath {
        let (p, _) = self.lonlat_to_local(site);
        self.peak_path_local(p)
    }

    /// Mesafeler ve tepe yolu birlikte (jeodezik dönüşüm bir kez).
    pub fn distances_and_peak(&self, site: (f64, f64)) -> (Distances, PeakPath) {
        let (p, repi) = self.lonlat_to_local(site);
        (self.distances_local(p, repi), self.peak_path_local(p))
    }

    pub(crate) fn lonlat_to_local(&self, site: (f64, f64)) -> ([f64; 2], f64) {
        let (km, az) = geo::distance_azimuth(self.epicenter, site);
        let a = az.to_radians();
        ([km * a.sin(), km * a.cos()], km)
    }

    /// Merkez üssünden kırılmanın en uzak yüzey izdüşümü köşesine uzaklık
    /// (km).
    pub fn max_extent_km(&self) -> f64 {
        self.segments
            .iter()
            .flat_map(|f| f.surface_outline_local())
            .map(|p| p[0].hypot(p[1]))
            .fold(0.0, f64::max)
    }

    /// Rrup = `limit_km` eş-uzaklık eğrisi (boylam, enlem), `n` azimutta;
    /// kapalı. Her azimutta merkez üssünden dışa adımlanır, Rrup'un sınırı
    /// SON kez geçtiği yer ikiye bölmeyle bulunur (çok parçalı izde Rrup ışın
    /// boyunca tekdüze olmayabilir).
    pub fn rrup_contour(&self, limit_km: f64, n: usize) -> Vec<(f64, f64)> {
        if self.segments.len() == 1 {
            return self.segments[0].rrup_contour(limit_km, n);
        }
        let r_max = limit_km + self.max_extent_km() + self.width;
        let step = 2.0;
        (0..=n)
            .map(|i| {
                let az = 360.0 * i as f64 / n as f64;
                let dir = [az.to_radians().sin(), az.to_radians().cos()];
                let rrup = |r: f64| self.distances_local([dir[0] * r, dir[1] * r], r).rrup;
                let mut last_in = 0.0;
                let mut r = 0.0;
                while r <= r_max {
                    if rrup(r) < limit_km {
                        last_in = r;
                    }
                    r += step;
                }
                let (mut lo, mut hi) = (last_in, last_in + step);
                for _ in 0..30 {
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

    /// Üst kenar izi (boylam, enlem).
    pub fn trace_lonlat(&self) -> Vec<(f64, f64)> {
        self.trace
            .iter()
            .map(|&p| self.segments[0].local_to_lonlat(p))
            .collect()
    }
}

/// Merkez üssüne göre yerel yatay konum (km; azimutal eş uzaklık).
fn local(epicenter: (f64, f64), site: (f64, f64)) -> [f64; 2] {
    let (km, az) = geo::distance_azimuth(epicenter, site);
    let a = az.to_radians();
    [km * a.sin(), km * a.cos()]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quake::{Setting, Trace};

    fn quake(strike: f64, dip: f64) -> Quake {
        let mut q = Quake::new((37.0, 37.5));
        q.mw = 7.6;
        q.depth_km = 10.0;
        q.fault = FaultType::Reverse;
        q.strike = strike;
        q.dip = dip;
        q.setting = Setting::ShallowCrust;
        q
    }

    fn sites() -> Vec<(f64, f64)> {
        let mut v = Vec::new();
        for i in -6..=6 {
            for j in -6..=6 {
                v.push((37.0 + 0.23 * f64::from(i), 37.5 + 0.19 * f64::from(j)));
            }
        }
        v
    }

    /// Tek düzlem: Rupture, Fault ile birebir aynı.
    #[test]
    fn tek_parca_fault_ile_ayni() {
        let q = quake(40.0, 35.0);
        let f = Fault::from_quake(&q);
        let r = Rupture::from_quake(&q);
        for s in sites() {
            assert_eq!(r.distances(s), f.distances(s));
            assert_eq!(r.peak_path(s), f.peak_path(s));
        }
        assert_eq!(r.rrup_contour(300.0, 36), f.rrup_contour(300.0, 36));
    }

    /// Düz bir izi aynı doğru üstünde parçalara bölmek sonucu değiştirmez
    /// (GC2'nin "non-strike-related within-segment subdivision"a
    /// değişmezliği, s. 5; Rrup/Rjb da aynı dikdörtgen birleşimi).
    #[test]
    fn dogrusal_bolme_degismez() {
        let base = quake(30.0, 60.0);
        let f = Fault::from_quake(&base);
        let [a, b] = f.top_edge_lonlat();
        // İzi 3 parçaya böl (aynı doğru üzerinde, jeodezik ara noktalar).
        let (km, az) = geo::distance_azimuth(a, b);
        let pts = [
            a,
            geo::destination(a, az, km * 0.2),
            geo::destination(a, az, km * 0.65),
            b,
        ];
        let mut q = base;
        q.trace = Trace::new(&pts);
        q.rupture_km = Some([f.length, f.width]);
        let r = Rupture::from_quake(&q);
        assert_eq!(r.segments.len(), 3);
        assert!(
            (r.length - f.length).abs() < 1e-3,
            "{} {}",
            r.length,
            f.length
        );
        assert!((r.ztor - f.ztor).abs() < 1e-6);
        for s in sites() {
            let (d1, d2) = (r.distances(s), f.distances(s));
            assert!((d1.rrup - d2.rrup).abs() < 0.05, "{s:?} {d1:?} {d2:?}");
            assert!((d1.rjb - d2.rjb).abs() < 0.05, "{s:?} {d1:?} {d2:?}");
            assert!((d1.rx - d2.rx).abs() < 0.05, "{s:?} {d1:?} {d2:?}");
            let (p1, p2) = (r.peak_path(s), f.peak_path(s));
            assert!(
                (p1.rupture_km - p2.rupture_km).abs() < 0.05,
                "{s:?} {p1:?} {p2:?}"
            );
        }
    }

    /// GC2 sağduyu: dirsekli izde T, dirseğin açıortayında sıçramaz; eğim
    /// yönünde pozitif; iz üzerinde 0 ve U = iz boyunca uzaklık.
    #[test]
    fn gc2_dirsek() {
        let mut q = quake(0.0, 90.0);
        q.fault = FaultType::StrikeSlip;
        // Kuzeye 50 km, sonra kuzeydoğuya 50 km (yerel km'den boylam/enlem).
        let e = q.epicenter;
        let p0 = geo::destination(e, 180.0, 50.0);
        let p1 = e;
        let p2 = geo::destination(e, 45.0, 50.0);
        q.trace = Trace::new(&[p0, p1, p2]);
        let r = Rupture::from_quake(&q);
        // İz üzerinde.
        let (t, u) = r.gc2([0.0, 0.0]);
        assert_eq!(t, 0.0);
        assert!((u - 50.0).abs() < 0.1, "{u}");
        // Eğim yönü (doğrultunun sağı: doğu) pozitif, batı negatif.
        assert!(r.gc2([10.0, -30.0]).0 > 0.0);
        assert!(r.gc2([-10.0, -30.0]).0 < 0.0);
        // Dirsek çevresinde T sürekli: açıortay boyunca küçük adımlarda
        // komşu değerler arasında sıçrama yok.
        let bis = (-67.5f64).to_radians(); // dışbükey taraf açıortayı (yerel)
        let mut prev = None::<f64>;
        for k in 1..200 {
            let rr = 0.25 * f64::from(k);
            let y = [rr * bis.sin(), rr * bis.cos()];
            let t = r.gc2(y).0;
            if let Some(p) = prev {
                assert!((t - p).abs() < 0.5, "{rr}: {t} {p}");
            }
            prev = Some(t);
        }
        // Mesafeler: en yakın parçaya Rrup; iz üzerinde 0.
        let d = r.distances_local([0.0, -20.0], 20.0);
        assert!(d.rrup < 1e-9 + r.ztor, "{d:?}");
    }

    /// Kırılma zamanı: hiposantr izin başındaysa uzak uçtaki sahaya kırılma
    /// yolu ~ iz uzunluğu.
    #[test]
    fn kirilma_yolu_iz_boyunca() {
        let mut q = quake(0.0, 90.0);
        q.fault = FaultType::StrikeSlip;
        let e = q.epicenter;
        let p1 = geo::destination(e, 0.0, 100.0);
        let p2 = geo::destination(p1, 60.0, 100.0);
        q.trace = Trace::new(&[e, p1, p2]);
        let r = Rupture::from_quake(&q);
        let end = geo::destination(p2, 60.0, 5.0);
        let path = r.peak_path(end);
        assert!((path.rupture_km - 200.0).abs() < 2.0, "{path:?}");
        // Başlangıca yakın saha: kırılma yolu ~0.
        let near = geo::destination(e, 90.0, 5.0);
        assert!(r.peak_path(near).rupture_km < 2.0 * r.width);
    }

    /// Çok parçalı özet parametreler (PEER 2013/03): uzunluk toplam,
    /// genişlik en büyük, eğim genişlikle ağırlıklı, Ztor en sığ.
    #[test]
    fn ozet_parametreler() {
        let mut q = quake(0.0, 60.0);
        let e = q.epicenter;
        let p1 = geo::destination(e, 0.0, 40.0);
        let p2 = geo::destination(p1, 20.0, 60.0);
        let mut t = Trace::new(&[e, p1, p2]).unwrap();
        t.set_segment_dip(1, Some(80.0));
        q.trace = Some(t);
        let r = Rupture::from_quake(&q);
        assert!((r.length - 100.0).abs() < 0.1, "{}", r.length);
        assert!((r.dip - 70.0).abs() < 1e-9);
        assert_eq!(r.segments[1].dip, 80.0);
        // Genişlik √A, tabana kırpılmış: Mw 7.6 → √(10^((7.6−4.07)/0.98))
        // ≈ 63 km; eğim 80° parça 20 / sin 80° ≈ 20.3 km'ye sınırlar.
        let w_max = 20.0 / 80f64.to_radians().sin();
        assert!((r.width - w_max).abs() < 1e-9, "{}", r.width);
        // Uç noktalar izde.
        let tr = r.trace_lonlat();
        assert!(geo::distance_azimuth(tr[2], p2).0 < 0.01);
    }
}
