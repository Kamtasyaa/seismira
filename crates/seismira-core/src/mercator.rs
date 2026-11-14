//! Web Mercator izdüşümü (EPSG:3857, "Popular Visualisation Pseudo Mercator").
//!
//! Koordinatlar birim kareye normalize edilir: x ∈ [0, 1] batıdan doğuya
//! (−180° → +180°), y ∈ [0, 1] kuzeyden güneye (+85.05° → −85.05°).
//! Bu, döşeme haritalarının kullandığı biçimdir.
//!
//! Kaynak: IOGP Geomatics Guidance Note 7-2, "Popular Visualisation Pseudo
//! Mercator" (EPSG yöntem kodu 1024): küresel Mercator formülünün elipsoit
//! koordinatlarına doğrudan uygulanması. Normalize biçim: OpenStreetMap Wiki,
//! "Slippy map tilenames". Bu izdüşüm yalnızca ÇİZİM içindir; mesafe ve
//! alan hesapları jeodezik yapılır (CLAUDE.md Bölüm 8).

use std::f64::consts::PI;

/// Web Mercator'un enlem sınırı: haritanın kare olmasını sağlayan değer,
/// atan(sinh(π)) derece cinsinden.
pub const MAX_LAT: f64 = 85.051_128_779_806_59;

/// Boylam/enlem (derece) → birim kare (x, y). Enlem ±[`MAX_LAT`] ile kırpılır.
pub fn to_unit(lon: f64, lat: f64) -> (f64, f64) {
    let lat = lat.clamp(-MAX_LAT, MAX_LAT).to_radians();
    let x = (lon + 180.0) / 360.0;
    let y = 0.5 - (PI / 4.0 + lat / 2.0).tan().ln() / (2.0 * PI);
    (x, y)
}

/// Birim kare (x, y) → boylam/enlem (derece). x, [0, 1] dışındaysa boylam
/// [−180, 180) aralığına sarılır (yatay tekrar).
pub fn from_unit(x: f64, y: f64) -> (f64, f64) {
    let lon = (x * 360.0 - 180.0 + 180.0).rem_euclid(360.0) - 180.0;
    let lat = (PI * (1.0 - 2.0 * y)).sinh().atan().to_degrees();
    (lon, lat)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, eps: f64) -> bool {
        (a - b).abs() < eps
    }

    #[test]
    fn sinir_enlemi_tanimdan() {
        // MAX_LAT = atan(sinh(π)) derece.
        assert!(close(PI.sinh().atan().to_degrees(), MAX_LAT, 1e-12));
    }

    #[test]
    fn bilinen_noktalar() {
        assert_eq!(to_unit(0.0, 0.0), (0.5, 0.5));
        let (x, y) = to_unit(-180.0, MAX_LAT);
        assert!(close(x, 0.0, 1e-12) && close(y, 0.0, 1e-12));
        let (x, y) = to_unit(180.0, -MAX_LAT);
        assert!(close(x, 1.0, 1e-12) && close(y, 1.0, 1e-12));
        // Kırpma: kutup, sınır enlemine eşlenir.
        assert!(close(to_unit(0.0, 90.0).1, 0.0, 1e-12));
    }

    #[test]
    fn ceyrek_noktalar() {
        // y = 0.25 ⇔ ln(tan(π/4 + φ/2)) = π/2 ⇔ φ = atan(sinh(π/2)) ≈ 66.513°.
        let lat = (PI / 2.0).sinh().atan().to_degrees();
        let (x, y) = to_unit(90.0, lat);
        assert!(close(x, 0.75, 1e-12) && close(y, 0.25, 1e-12));
        let (_, y) = to_unit(0.0, -lat);
        assert!(close(y, 0.75, 1e-12));
    }

    #[test]
    fn gidis_donus() {
        for &(lon, lat) in &[
            (28.97, 41.01),
            (-122.4, 37.8),
            (139.7, 35.7),
            (-70.6, -33.4),
        ] {
            let (x, y) = to_unit(lon, lat);
            let (lon2, lat2) = from_unit(x, y);
            assert!(close(lon, lon2, 1e-9) && close(lat, lat2, 1e-9));
        }
        // Yatay tekrar: x + 1 aynı boylam.
        let (x, y) = to_unit(170.0, 10.0);
        assert!(close(from_unit(x + 1.0, y).0, 170.0, 1e-9));
    }
}
