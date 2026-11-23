//! Koyu tema: haritayı öne çıkaran, sade renkler.

use eframe::egui::{self, Color32, Theme};

/// Harita zemini (okyanus). Paneller bundan biraz açık durur ki harita
/// en koyu ve en geniş yüzey olsun.
pub const MAP_BG: Color32 = Color32::from_rgb(0x0B, 0x0E, 0x13);
/// Harita karası (map::colors::LAND ile aynı; kutu kenarı kontrastı için).
pub const MAP_LAND: Color32 = Color32::from_rgb(0x1F, 0x26, 0x31);
const PANEL: Color32 = Color32::from_rgb(0x15, 0x19, 0x1F);
const WINDOW: Color32 = Color32::from_rgb(0x1A, 0x1F, 0x27);
const FAINT: Color32 = Color32::from_rgb(0x1E, 0x24, 0x2D);
/// Eğitim uyarısı gibi her zaman görünmesi gereken bilgi metni.
pub const WARN_TEXT: Color32 = Color32::from_rgb(0xD9, 0xB8, 0x6A);
const ACCENT: Color32 = Color32::from_rgb(0x3B, 0xA7, 0xB8);
/// Normal ve soluk metin (kontrast gerekçesi `apply` içinde).
const TEXT: Color32 = Color32::from_gray(175);
const WEAK_TEXT: Color32 = Color32::from_gray(135);

pub fn apply(ctx: &egui::Context) {
    let mut v = egui::Visuals::dark();
    v.panel_fill = PANEL;
    v.window_fill = WINDOW;
    v.extreme_bg_color = MAP_BG;
    v.faint_bg_color = FAINT;
    v.hyperlink_color = ACCENT;
    // Erişilebilirlik (Faz 10 adım 6, WCAG 2.2 AA, docs/arayuz.md): egui'nin
    // varsayılan metni (gri 140) panelde 5,2:1, soluk metni (%60 alfa)
    // 2,7:1 veriyordu. Metin gri 175 (panelde 8,4:1), soluk metin gri 135
    // (panelde 4,9:1, pencerede 4,7:1).
    v.widgets.noninteractive.fg_stroke.color = TEXT;
    v.weak_text_color = Some(WEAK_TEXT);
    v.selection.bg_fill = ACCENT.gamma_multiply(0.45);
    ctx.set_visuals_of(Theme::Dark, v);
    ctx.set_theme(Theme::Dark);
}
