//! Jeodezik hesaplar: mesafe, merkez üssünden belli uzaklıktaki noktalar.
//!
//! Yüzey geometrisi WGS84 elipsoidi üzerinde jeodezik (Karney 2013,
//! "Algorithms for geodesics", J. Geodesy 87, 43-55; geographiclib-rs).
//! Seyahat süresi tabloları ise küresel bir Dünya için episantr açısıyla
//! (derece) verilir. İkisi arasındaki dönüşüm ObsPy'nin uzlaşımıyla aynıdır:
//! derece = km / (π/180 · 6371) (`obspy.geodetics.kilometers2degrees`,
//! varsayılan yarıçap 6371 km; ak135 modelinin yarıçapı da 6371 km).

use geographiclib_rs::{DirectGeodesic as _, Geodesic, InverseGeodesic as _};
use std::sync::LazyLock;

/// Derece ↔ km dönüşümünde kullanılan Dünya yarıçapı (km).
pub const EARTH_RADIUS_KM: f64 = 6371.0;

static WGS84: LazyLock<Geodesic> = LazyLock::new(Geodesic::wgs84);

/// Episantr açısı (derece) → yüzey mesafesi (km).
pub fn deg_to_km(deg: f64) -> f64 {
    deg.to_radians() * EARTH_RADIUS_KM
}

/// Yüzey mesafesi (km) → episantr açısı (derece).
pub fn km_to_deg(km: f64) -> f64 {
    (km / EARTH_RADIUS_KM).to_degrees()
}

/// İki nokta arası jeodezik mesafe (km). Girdi: (boylam, enlem) derece.
pub fn distance_km(a: (f64, f64), b: (f64, f64)) -> f64 {
    let m: f64 = WGS84.inverse(a.1, a.0, b.1, b.0);
    m / 1000.0
}

/// İki nokta arası jeodezik mesafe (km) ve `a`'daki azimut (derece, kuzeyden
/// saat yönünde).
pub fn distance_azimuth(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    let (s12, azi1, _azi2, _a12): (f64, f64, f64, f64) = WGS84.inverse(a.1, a.0, b.1, b.0);
    (s12 / 1000.0, azi1)
}

/// `from` noktasından `azimuth_deg` yönünde `km` uzaktaki nokta (boylam, enlem).
pub fn destination(from: (f64, f64), azimuth_deg: f64, km: f64) -> (f64, f64) {
    let (lat, lon): (f64, f64) = WGS84.direct(from.1, from.0, azimuth_deg, km * 1000.0);
    (lon, lat)
}

/// `center` noktasından `dist_deg` episantr açısı uzaktaki noktalar,
/// `n` eşit aralıklı azimutta (0° kuzey, saat yönünde). Halka kapalıdır:
/// son nokta ilkinin tekrarıdır. Çıktı: (boylam, enlem) derece; boylam
/// [-180, 180] aralığındadır (çizim tarafı sarmayı açar).
pub fn ring(center: (f64, f64), dist_deg: f64, n: usize) -> Vec<(f64, f64)> {
    let s12 = deg_to_km(dist_deg) * 1000.0;
    (0..=n)
        .map(|i| {
            let azi = 360.0 * i as f64 / n as f64;
            let (lat, lon): (f64, f64) = WGS84.direct(center.1, center.0, azi, s12);
            (lon, lat)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ekvator_boyunca_bir_derece() {
        // Ekvator bir çember: 1° boylam = a·π/180, a = 6378137 m (WGS84).
        let expected = 6_378_137.0 * std::f64::consts::PI / 180.0 / 1000.0;
        let d = distance_km((0.0, 0.0), (1.0, 0.0));
        assert!((d - expected).abs() < 1e-6, "{d} {expected}");
    }

    #[test]
    fn hedef_nokta_mesafesi() {
        let p = destination((28.97, 41.01), 45.0, 250.0);
        assert!((distance_km((28.97, 41.01), p) - 250.0).abs() < 1e-6);
    }

    #[test]
    fn derece_km_donusumu() {
        assert!((deg_to_km(1.0) - 111.194_926_644_558_73).abs() < 1e-9);
        assert!((km_to_deg(deg_to_km(37.5)) - 37.5).abs() < 1e-12);
    }

    #[test]
    fn halka_noktalari_merkezden_esit_uzakta() {
        for &center in &[(28.97, 41.01), (179.5, -17.5), (0.0, 89.0)] {
            for &deg in &[0.5, 10.0, 60.0] {
                let pts = ring(center, deg, 64);
                assert_eq!(pts.len(), 65);
                for &p in &pts {
                    let d = km_to_deg(distance_km(center, p));
                    assert!((d - deg).abs() < 1e-6, "{center:?} {deg} {p:?} {d}");
                }
            }
        }
    }
}
