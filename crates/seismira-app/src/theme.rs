//! Tema: koyu (varsayılan; haritayı öne çıkaran, sade renkler) ve açık
//! (kullanıcı isteği 2026-11-27; Görünüm → Tema, oturumluk: her açılışta
//! koyu).
//!
//! Harita ve harita üstü çizimler sabit renk yerine buradaki işlevleri
//! kullanır ([`map_bg`], [`map_land`], [`box_bg`], [`box_text`],
//! [`map_ink`], [`readable`] …); seçili tema süreç genelinde tek değerdir.

use eframe::egui::{self, Color32, Theme};
use std::sync::atomic::{AtomicBool, Ordering};

static LIGHT: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Dark,
    Light,
}

impl Mode {
    pub fn from_code(s: &str) -> Option<Mode> {
        match s {
            "dark" => Some(Mode::Dark),
            "light" => Some(Mode::Light),
            _ => None,
        }
    }
}

fn light() -> bool {
    LIGHT.load(Ordering::Relaxed)
}

/// Harita zemini (okyanus). Koyu: paneller bundan biraz açık durur ki harita
/// en koyu ve en geniş yüzey olsun. Açık: OSM benzeri açık mavi.
pub fn map_bg() -> Color32 {
    if light() {
        Color32::from_rgb(0xB8, 0xD4, 0xE3)
    } else {
        Color32::from_rgb(0x0B, 0x0E, 0x13)
    }
}

/// Harita karası (kutu kenarı kontrastı için de).
pub fn map_land() -> Color32 {
    if light() {
        Color32::from_rgb(0xF2, 0xEF, 0xE9)
    } else {
        Color32::from_rgb(0x1F, 0x26, 0x31)
    }
}

/// Ülke ve il sınırları.
pub fn admin0() -> Color32 {
    if light() {
        Color32::from_rgb(0x8C, 0x86, 0x80)
    } else {
        Color32::from_rgb(0x55, 0x61, 0x73)
    }
}

pub fn admin1() -> Color32 {
    if light() {
        Color32::from_rgb(0xC4, 0xBE, 0xB6)
    } else {
        Color32::from_rgb(0x33, 0x3D, 0x4B)
    }
}

/// Harita üstü kutuların (açıklama, ipucu) zemini; `alpha` 0–1 opaklık.
pub fn box_bg(alpha: f32) -> Color32 {
    if light() {
        Color32::from_rgb(0xFF, 0xFF, 0xFF).gamma_multiply(alpha)
    } else {
        Color32::from_rgb(0x0B, 0x0E, 0x13).gamma_multiply(alpha)
    }
}

/// [`box_bg`] üstündeki metin.
pub fn box_text() -> Color32 {
    if light() {
        Color32::from_gray(25)
    } else {
        Color32::WHITE
    }
}

/// Haritada doğrudan çizilen nötr çizgi / nokta (fay, iz, uyarı sınırı):
/// koyu temada beyaz, açıkta neredeyse siyah; `alpha` 0–1.
pub fn map_ink(alpha: f32) -> Color32 {
    if light() {
        Color32::from_gray(30).gamma_multiply(alpha)
    } else {
        Color32::WHITE.gamma_multiply(alpha)
    }
}

/// Erken uyarı kapsamındaki ilin sınırı ve yanıp sönen dolgusu (`pulse`
/// 0–1). Koyu temada beyaz; açık temada beyaz / koyu gri açık kara üzerinde
/// okunmadığı için (kullanıcı, 2026-11-27) mavi-mor ton, daha yoğun dolgu.
pub fn warning_edge() -> Color32 {
    if light() {
        Color32::from_rgb(0x3F, 0x3D, 0xB8)
    } else {
        Color32::WHITE
    }
}

pub fn warning_fill(pulse: f32) -> Color32 {
    if light() {
        Color32::from_rgb(0x3F, 0x3D, 0xB8).gamma_multiply(0.10 + 0.22 * pulse)
    } else {
        Color32::WHITE.gamma_multiply(0.04 + 0.10 * pulse)
    }
}

/// Modelin mesafe sınırı (kesikli çizgi).
pub fn model_limit() -> Color32 {
    if light() {
        Color32::from_rgba_premultiplied(70, 70, 80, 210)
    } else {
        Color32::from_rgba_premultiplied(150, 150, 160, 170)
    }
}

/// Sınır çizgilerinin altındaki hâle.
pub fn halo() -> Color32 {
    if light() {
        Color32::from_rgba_premultiplied(255, 255, 255, 200)
    } else {
        Color32::from_rgba_premultiplied(0, 0, 0, 190)
    }
}

/// Panel zemininde okunacak renkli metin (uyarı, bildirim): açık temada
/// koyulaştırılır (açık zeminde parlak sarı / kırmızı okunmaz).
pub fn readable(c: Color32) -> Color32 {
    readable_in(light(), c)
}

fn readable_in(light: bool, c: Color32) -> Color32 {
    if light {
        let k = |v: u8| (f32::from(v) * 0.55) as u8;
        Color32::from_rgb(k(c.r()), k(c.g()), k(c.b()))
    } else {
        c
    }
}

/// Panelde renkli metin (bildirim akışı): beyaz açık temada normal metin
/// rengine, diğerleri [`readable`] ile koyulaşır.
pub fn on_panel(ui: &egui::Ui, c: Color32) -> Color32 {
    if !light() {
        c
    } else if c == Color32::WHITE {
        ui.visuals().text_color()
    } else {
        readable(c)
    }
}

/// Eğitim uyarısı gibi her zaman görünmesi gereken bilgi metni.
pub fn warn_text() -> Color32 {
    if light() {
        Color32::from_rgb(0x7A, 0x52, 0x00)
    } else {
        Color32::from_rgb(0xD9, 0xB8, 0x6A)
    }
}

const ACCENT: Color32 = Color32::from_rgb(0x3B, 0xA7, 0xB8);

/// Temayı uygular ve seçer.
pub fn apply(ctx: &egui::Context, mode: Mode) {
    LIGHT.store(mode == Mode::Light, Ordering::Relaxed);
    match mode {
        Mode::Dark => {
            let mut v = egui::Visuals::dark();
            v.panel_fill = Color32::from_rgb(0x15, 0x19, 0x1F);
            v.window_fill = Color32::from_rgb(0x1A, 0x1F, 0x27);
            v.extreme_bg_color = map_bg();
            v.faint_bg_color = Color32::from_rgb(0x1E, 0x24, 0x2D);
            v.hyperlink_color = ACCENT;
            // Erişilebilirlik (Faz 10 adım 6, WCAG 2.2 AA, docs/arayuz.md):
            // egui'nin varsayılan metni (gri 140) panelde 5,2:1, soluk metni
            // (%60 alfa) 2,7:1 veriyordu. Metin gri 175 (panelde 8,4:1),
            // soluk metin gri 135 (panelde 4,9:1, pencerede 4,7:1).
            v.widgets.noninteractive.fg_stroke.color = Color32::from_gray(175);
            v.weak_text_color = Some(Color32::from_gray(135));
            v.selection.bg_fill = ACCENT.gamma_multiply(0.45);
            ctx.set_visuals_of(Theme::Dark, v);
            ctx.set_theme(Theme::Dark);
        }
        Mode::Light => {
            let mut v = egui::Visuals::light();
            v.panel_fill = Color32::from_rgb(0xF4, 0xF5, 0xF7);
            v.window_fill = Color32::from_rgb(0xFB, 0xFB, 0xFC);
            v.extreme_bg_color = Color32::WHITE;
            v.faint_bg_color = Color32::from_rgb(0xEC, 0xEE, 0xF1);
            v.hyperlink_color = Color32::from_rgb(0x0B, 0x6E, 0x7C);
            // WCAG 2.2 AA: metin gri 30 (panelde ~14:1), soluk metin gri 90
            // (panelde ~6,8:1, pencerede ~7,2:1).
            v.widgets.noninteractive.fg_stroke.color = Color32::from_gray(30);
            v.weak_text_color = Some(Color32::from_gray(90));
            v.selection.bg_fill = ACCENT.gamma_multiply(0.35);
            ctx.set_visuals_of(Theme::Light, v);
            ctx.set_theme(Theme::Light);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// WCAG göreli parlaklık ve kontrast (shindo_style ile aynı tanım).
    fn lum(c: Color32) -> f32 {
        let f = |v: u8| {
            let v = f32::from(v) / 255.0;
            if v <= 0.04045 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * f(c.r()) + 0.7152 * f(c.g()) + 0.0722 * f(c.b())
    }

    fn contrast(a: Color32, b: Color32) -> f32 {
        let (x, y) = (lum(a), lum(b));
        (x.max(y) + 0.05) / (x.min(y) + 0.05)
    }

    /// Açık temada metin ve soluk metin panel / pencerede AA (≥ 4,5:1);
    /// koyulaştırılmış uyarı renkleri panelde ≥ 4,5:1.
    #[test]
    fn acik_tema_kontrast() {
        let panel = Color32::from_rgb(0xF4, 0xF5, 0xF7);
        let window = Color32::from_rgb(0xFB, 0xFB, 0xFC);
        for bg in [panel, window] {
            assert!(contrast(Color32::from_gray(30), bg) >= 4.5);
            assert!(contrast(Color32::from_gray(90), bg) >= 4.5);
        }
        for c in [
            Color32::from_rgb(0xE8, 0xB4, 0x4C),
            Color32::from_rgb(0xFF, 0x6B, 0x6B),
            Color32::from_rgb(0xFF, 0x9A, 0x6B),
        ] {
            let r = readable_in(true, c);
            assert!(
                contrast(r, panel) >= 4.5,
                "{c:?} → {r:?}: {}",
                contrast(r, panel)
            );
        }
        assert!(contrast(Color32::from_rgb(0x7A, 0x52, 0x00), panel) >= 4.5);
        assert!(contrast(Color32::from_gray(25), Color32::WHITE) >= 4.5);
    }
}
