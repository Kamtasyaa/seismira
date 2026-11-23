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

/// Verilen zemin rengi üzerinde okunur yazı rengi: açık zeminde siyah,
/// koyu zeminde beyaz.
pub fn text_on(c: Color32) -> Color32 {
    // Algısal parlaklık (ITU-R BT.601 ağırlıkları).
    let l = 0.299 * f32::from(c.r()) + 0.587 * f32::from(c.g()) + 0.114 * f32::from(c.b());
    if l > 150.0 {
        Color32::BLACK
    } else {
        Color32::WHITE
    }
}

pub const BOX_SIZE: egui::Vec2 = vec2(20.0, 18.0);

/// Etikete göre kutu boyutu (MMI "VIII" Shindo "5-"den geniş).
pub fn box_size(label: &str) -> egui::Vec2 {
    vec2(BOX_SIZE.x.max(8.0 + 7.0 * label.len() as f32), BOX_SIZE.y)
}

/// Merkezi `center` olan Shindo kutusu.
pub fn draw_box(painter: &egui::Painter, center: Pos2, s: Shindo) -> Rect {
    draw_labeled_box(painter, center, color(s), s.label())
}

/// Merkezi `center` olan renkli etiket kutusu (Shindo ya da MMI).
pub fn draw_labeled_box(painter: &egui::Painter, center: Pos2, fill: Color32, label: &str) -> Rect {
    let rect = Rect::from_center_size(center, box_size(label));
    painter.rect_filled(rect, 3.0, fill);
    painter.rect_stroke(
        rect,
        3.0,
        Stroke::new(1.0, Color32::from_black_alpha(180)),
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
