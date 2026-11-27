//! Shindo renkleri ve kutu çizimi.
//!
//! Renkler: JMA "気象庁ホームページにおける気象情報の配色に関する設定指針"
//! (HPColorGuide, Temmuz 2020), Tablo 2-2 (震度). Shindo 0 için renk
//! tanımlanmamış; kutu gösterilmez.

use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Stroke, vec2};
use seismira_core::intensity::Shindo;

pub fn color(s: Shindo) -> Color32 {
    match s {
        Shindo::S7 => Color32::from_rgb(180, 0, 104),
        Shindo::S6Upper => Color32::from_rgb(165, 0, 33),
        Shindo::S6Lower => Color32::from_rgb(255, 40, 0),
        Shindo::S5Upper => Color32::from_rgb(255, 153, 0),
        Shindo::S5Lower => Color32::from_rgb(255, 230, 0),
        Shindo::S4 => Color32::from_rgb(250, 230, 150),
        Shindo::S3 => Color32::from_rgb(0, 65, 255),
        Shindo::S2 => Color32::from_rgb(0, 170, 255),
        Shindo::S1 => Color32::from_rgb(242, 242, 255),
        Shindo::S0 => Color32::from_rgb(120, 120, 120),
    }
}

/// WCAG 2.x göreli parlaklık (sRGB).
fn luminance(c: Color32) -> f32 {
    let lin = |v: u8| {
        let v = f32::from(v) / 255.0;
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * lin(c.r()) + 0.7152 * lin(c.g()) + 0.0722 * lin(c.b())
}

/// İki rengin WCAG kontrast oranı (1–21).
pub fn contrast(a: Color32, b: Color32) -> f32 {
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

/// Verilen zemin rengi üzerinde okunur yazı rengi: siyah ya da beyazdan
/// kontrastı yüksek olanı (Faz 10 adım 6; önceki parlaklık eşiği Shindo 2
/// kutusunda beyaz seçip 2,6:1 veriyordu, siyah 8,2:1).
pub fn text_on(c: Color32) -> Color32 {
    if contrast(Color32::BLACK, c) >= contrast(Color32::WHITE, c) {
        Color32::BLACK
    } else {
        Color32::WHITE
    }
}

/// Kutu kenarı (WCAG 1.4.11): harita karası üzerinde 3:1'i tutmayan
/// dolgularda karaya zıt kenar (koyu temada açık, açık temada koyu),
/// diğerlerinde koyu kenar.
fn box_outline(fill: Color32) -> Color32 {
    let land = crate::theme::map_land();
    if contrast(fill, land) < 3.0 && luminance(land) < 0.5 {
        Color32::from_white_alpha(170)
    } else {
        Color32::from_black_alpha(180)
    }
}

pub const BOX_SIZE: egui::Vec2 = vec2(20.0, 18.0);

/// Etikete göre kutu boyutu (MMI "VIII" Shindo "5-"den geniş).
pub fn box_size(label: &str) -> egui::Vec2 {
    vec2(BOX_SIZE.x.max(8.0 + 7.0 * label.len() as f32), BOX_SIZE.y)
}

/// Merkezi `center` olan renkli etiket kutusu (Shindo ya da MMI).
pub fn draw_labeled_box(painter: &egui::Painter, center: Pos2, fill: Color32, label: &str) -> Rect {
    let rect = Rect::from_center_size(center, box_size(label));
    painter.rect_filled(rect, 3.0, fill);
    painter.rect_stroke(
        rect,
        3.0,
        Stroke::new(1.0, box_outline(fill)),
        egui::StrokeKind::Outside,
    );
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(12.5),
        text_on(fill),
    );
    rect
}

#[cfg(test)]
mod tests {
    use super::*;
    use seismira_core::intensity::Shindo;

    /// Tüm Shindo ve MMI kutularında yazı en az 4,5:1 (WCAG AA).
    #[test]
    fn kutu_yazilari_okunur() {
        let mut fills: Vec<Color32> = Shindo::ALL[1..].iter().map(|s| color(*s)).collect();
        fills.extend((2..=10).filter_map(crate::mmi_style::color));
        for f in fills {
            let c = contrast(text_on(f), f);
            assert!(c >= 4.5, "{f:?}: {c}");
        }
        // Bilinen değer: siyah / beyaz = 21.
        assert!((contrast(Color32::BLACK, Color32::WHITE) - 21.0).abs() < 0.01);
    }
}
