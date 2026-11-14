//! Koyu tema: haritayı öne çıkaran, sade renkler.

use eframe::egui::{self, Color32, Theme};

/// Harita zemini (okyanus). Paneller bundan biraz açık durur ki harita
/// en koyu ve en geniş yüzey olsun.
pub const MAP_BG: Color32 = Color32::from_rgb(0x0B, 0x0E, 0x13);
const PANEL: Color32 = Color32::from_rgb(0x15, 0x19, 0x1F);
const WINDOW: Color32 = Color32::from_rgb(0x1A, 0x1F, 0x27);
const FAINT: Color32 = Color32::from_rgb(0x1E, 0x24, 0x2D);
/// Eğitim uyarısı gibi her zaman görünmesi gereken bilgi metni.
pub const WARN_TEXT: Color32 = Color32::from_rgb(0xD9, 0xB8, 0x6A);
const ACCENT: Color32 = Color32::from_rgb(0x3B, 0xA7, 0xB8);

pub fn apply(ctx: &egui::Context) {
    let mut v = egui::Visuals::dark();
    v.panel_fill = PANEL;
    v.window_fill = WINDOW;
    v.extreme_bg_color = MAP_BG;
    v.faint_bg_color = FAINT;
    v.hyperlink_color = ACCENT;
    v.selection.bg_fill = ACCENT.gamma_multiply(0.45);
    ctx.set_visuals_of(Theme::Dark, v);
    ctx.set_theme(Theme::Dark);
}
