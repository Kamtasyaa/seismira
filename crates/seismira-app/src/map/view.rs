//! Harita görünümü: merkez, ölçek, kaydırma, yakınlaştırma, yatay tekrar.
//!
//! Dünya koordinatları Web Mercator birim karesidir (`seismira_core::mercator`):
//! x ∈ [0, 1) bir tur, y ∈ [0, 1] kuzeyden güneye. Ekran birimi egui
//! "point"idir. `scale`, bir dünya genişliğinin kaç point olduğudur.

use eframe::egui::{Pos2, Rect, Vec2, pos2};

/// En yakın görünüm: ~2^13 döşeme yakınlaştırması. Natural Earth 1:10m
/// bundan sonrasında ayrıntı vermez; f32 köşe hassasiyeti de burada
/// yaklaşık 0.1 point'tir.
pub const MAX_SCALE: f64 = 256.0 * 8192.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MapView {
    /// Görünüm merkezi (dünya birimi). x her zaman [0, 1) aralığına sarılır.
    pub center: [f64; 2],
    /// Dünya genişliği (point).
    pub scale: f64,
}

impl MapView {
    pub fn new(center: [f64; 2], scale: f64) -> Self {
        Self { center, scale }
    }

    /// Dünya koordinatı → ekran (dünya kopyası seçilmez; x olduğu gibi).
    pub fn world_to_screen(&self, rect: Rect, w: [f64; 2]) -> Pos2 {
        let c = rect.center();
        pos2(
            c.x + ((w[0] - self.center[0]) * self.scale) as f32,
            c.y + ((w[1] - self.center[1]) * self.scale) as f32,
        )
    }

    /// Ekran → dünya koordinatı. x sarılmaz (yatay tekrarın hangi
    /// kopyasında olduğunu korur); boylam için `mercator::from_unit` sarar.
    pub fn screen_to_world(&self, rect: Rect, p: Pos2) -> [f64; 2] {
        let c = rect.center();
        [
            self.center[0] + f64::from(p.x - c.x) / self.scale,
            self.center[1] + f64::from(p.y - c.y) / self.scale,
        ]
    }

    pub fn pan(&mut self, rect: Rect, delta: Vec2) {
        self.center[0] -= f64::from(delta.x) / self.scale;
        self.center[1] -= f64::from(delta.y) / self.scale;
        self.normalize(rect);
    }

    /// `p` ekran noktası yerinde kalacak şekilde yakınlaştırır.
    pub fn zoom_at(&mut self, rect: Rect, p: Pos2, factor: f64) {
        let anchor = self.screen_to_world(rect, p);
        self.scale *= factor;
        self.normalize(rect);
        let c = rect.center();
        self.center[0] = anchor[0] - f64::from(p.x - c.x) / self.scale;
        self.center[1] = anchor[1] - f64::from(p.y - c.y) / self.scale;
        self.normalize(rect);
    }

    /// Ölçeği ve merkezi geçerli aralığa çeker: dünya en az görünüm
    /// yüksekliğini doldurur, kutupların ötesi gösterilmez, x sarılır.
    pub fn normalize(&mut self, rect: Rect) {
        let min_scale = f64::from(rect.height().max(1.0));
        self.scale = self.scale.clamp(min_scale, MAX_SCALE.max(min_scale));
        self.center[0] = self.center[0].rem_euclid(1.0);
        let half_h = f64::from(rect.height()) / 2.0 / self.scale;
        self.center[1] = if half_h >= 0.5 {
            0.5
        } else {
            self.center[1].clamp(half_h, 1.0 - half_h)
        };
    }

    /// Görünür dünya kopyaları: (ilk kopya, kopya sayısı). Kopya k,
    /// x ∈ [k, k+1) aralığıdır.
    pub fn copies(&self, rect: Rect) -> (i32, u32) {
        let half_w = f64::from(rect.width()) / 2.0 / self.scale;
        let first = (self.center[0] - half_w).floor() as i32;
        let last = (self.center[0] + half_w).floor() as i32;
        (first, (last - first + 1) as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect() -> Rect {
        Rect::from_min_size(pos2(100.0, 50.0), Vec2::new(800.0, 600.0))
    }

    #[test]
    fn ekran_dunya_gidis_donus() {
        let v = MapView::new([0.3, 0.4], 5000.0);
        let r = rect();
        assert_eq!(v.world_to_screen(r, [0.3, 0.4]), r.center());
        let w = v.screen_to_world(r, pos2(123.0, 456.0));
        let p = v.world_to_screen(r, w);
        assert!((p - pos2(123.0, 456.0)).length() < 1e-3);
    }

    #[test]
    fn imlec_altindaki_nokta_sabit_kalir() {
        let r = rect();
        let mut v = MapView::new([0.6, 0.35], 3000.0);
        let p = pos2(700.0, 200.0);
        let before = v.screen_to_world(r, p);
        v.zoom_at(r, p, 2.5);
        let after = v.screen_to_world(r, p);
        assert!((before[0] - after[0]).abs() < 1e-9);
        assert!((before[1] - after[1]).abs() < 1e-9);
        assert_eq!(v.scale, 7500.0);
    }

    #[test]
    fn sinirlar() {
        let r = rect();
        let mut v = MapView::new([1.25, -3.0], 10.0);
        v.normalize(r);
        assert_eq!(v.scale, 600.0); // dünya en az görünüm yüksekliği kadar
        assert_eq!(v.center, [0.25, 0.5]);
        v.scale = 1e12;
        v.normalize(r);
        assert_eq!(v.scale, MAX_SCALE);
        // Kutbun ötesine kaydırılamaz.
        v.scale = 1200.0;
        v.center[1] = 0.0;
        v.normalize(r);
        assert_eq!(v.center[1], 0.25); // yarım yükseklik = 300/1200
    }

    #[test]
    fn yatay_tekrar_kopyalari() {
        let r = rect(); // genişlik 800
        // Dünya 400 point: merkez 0.5 → görünür x ∈ [-0.5, 1.5] → kopya -1, 0, 1.
        let v = MapView::new([0.5, 0.5], 400.0);
        assert_eq!(v.copies(r), (-1, 3));
        // Yakın görünümde tek kopya; 180° meridyeni yakınında iki kopya.
        assert_eq!(MapView::new([0.5, 0.5], 1e5).copies(r), (0, 1));
        assert_eq!(MapView::new([0.999, 0.5], 1e5).copies(r), (0, 2));
        assert_eq!(MapView::new([0.001, 0.5], 1e5).copies(r), (-1, 2));
    }
}
