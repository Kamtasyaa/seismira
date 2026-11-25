//! Zemin yenilmesi (sıvılaşma, heyelan) kareleri ve açıklaması (Bölüm 2.6,
//! docs/arayuz.md "kırmızı kareler").
//!
//! Her riskli ~25 km'lik kutu içi boş, kalın kırmızı bir çerçeveyle
//! gösterilir; altındaki şehir kutuları görünür kalır. İki tür aynı
//! çerçeveyi kullanır, köşedeki harf ayırır (renk tek bilgi taşıyıcı değil,
//! Bölüm 3). Çerçeve ≤ 1 Hz yumuşak yanıp söner; "Hareketi azalt"ta sabit.

use crate::i18n::Lang;
use eframe::egui::{self, Color32, FontId, Painter, Rect, Stroke, StrokeKind, vec2};

/// Çerçeve rengi.
pub const RED: Color32 = Color32::from_rgb(0xFF, 0x1A, 0x1A);
/// Çerçeve kalınlığı (piksel); altında koyu bir kontrast çizgisi var.
const WIDTH: f32 = 3.0;
/// Bu kenarın (piksel) altındaki kareler noktaya dönüşmesin diye büyütülür.
const MIN_SIDE: f32 = 14.0;

/// Yanıp sönmenin opaklığı (`wall`: duvar saati, s): %35–%100 arası
/// kosinüs, 1 Hz; "Hareketi azalt"ta sabit.
pub fn pulse(wall: f64, reduce_motion: bool) -> f32 {
    if reduce_motion {
        1.0
    } else {
        (0.675 + 0.325 * (std::f64::consts::TAU * wall).cos()) as f32
    }
}

/// Tam yanıp sönme (tsunami kıyıları; 2026-11-25 kullanıcı kararı:
/// yumuşak geçiş yerine açık/kapalı): 1 Hz, periyodun ilk %60'ında tam
/// görünür, kalanında kapalı (renk okunabilsin diye görünür kısım uzun).
/// "Hareketi azalt"ta sabit. Dönen: (opaklık, bir sonraki geçişe kalan s).
pub fn blink(wall: f64, reduce_motion: bool) -> (f32, f64) {
    const ON: f64 = 0.6;
    if reduce_motion {
        return (1.0, f64::INFINITY);
    }
    let ph = wall.rem_euclid(1.0);
    if ph < ON {
        (1.0, ON - ph)
    } else {
        (0.0, 1.0 - ph)
    }
}

/// Tek kare: `r` ekran dikdörtgeni, `alpha` opaklık, `letter` tür harfi,
/// `right`: harf sağ üst köşede (aynı kutuda iki tür olunca çakışmasın).
pub fn draw_box(painter: &Painter, r: Rect, alpha: f32, letter: &str, right: bool) {
    let r = if r.width() < MIN_SIDE {
        Rect::from_center_size(r.center(), vec2(MIN_SIDE, MIN_SIDE))
    } else {
        r
    };
    painter.rect_stroke(
        r,
        0.0,
        Stroke::new(
            WIDTH + 2.0,
            Color32::from_black_alpha((140.0 * alpha) as u8),
        ),
        StrokeKind::Middle,
    );
    painter.rect_stroke(
        r,
        0.0,
        Stroke::new(WIDTH, RED.gamma_multiply(alpha)),
        StrokeKind::Middle,
    );
    // Harf: üst köşede, kırmızı zeminli küçük etiket.
    if r.width() >= 2.0 * MIN_SIDE {
        let font = FontId::proportional(11.0);
        let galley = painter.layout_no_wrap(letter.to_owned(), font, Color32::WHITE);
        let pad = vec2(3.0, 1.0);
        let size = galley.size() + 2.0 * pad;
        let tag = if right {
            Rect::from_min_size(r.right_top() - vec2(size.x, 0.0), size)
        } else {
            Rect::from_min_size(r.left_top(), size)
        };
        painter.rect_filled(tag, 0.0, RED.gamma_multiply(alpha));
        painter.galley(tag.min + pad, galley, Color32::WHITE);
    }
}

/// Açıklama: Shindo açıklamasının üstünde, sağ altta. `kinds`: gösterilen
/// türlerin (harf, ad) çiftleri.
pub fn draw_legend(ui: &egui::Ui, rect: Rect, lang: Lang, kinds: &[(String, String)]) {
    if kinds.is_empty() {
        return;
    }
    let painter = ui.painter_at(rect);
    // Shindo açıklamasının yüksekliği (city_layer::draw_legend): kutu +
    // başlık + model sınırı notu + kenar boşlukları.
    let below = 12.0 + crate::shindo_style::BOX_SIZE.y + 18.0 + 22.0 + 14.0;
    let font = FontId::proportional(12.0);
    let rows: Vec<String> = kinds
        .iter()
        .map(|(l, name)| format!("{l}  {name}"))
        .collect();
    let note = lang.t_args(
        "ground-legend-note",
        &[("km", lang.num(crate::compute::BOX_KM, 0))],
    );
    let galleys: Vec<_> = rows
        .iter()
        .chain(std::iter::once(&note))
        .enumerate()
        .map(|(k, t)| {
            let c = if k < rows.len() {
                ui.visuals().text_color()
            } else {
                ui.visuals().weak_text_color()
            };
            painter.layout_no_wrap(t.clone(), font.clone(), c)
        })
        .collect();
    let icon = 14.0;
    let line_h = 18.0;
    let width = galleys.iter().map(|g| g.size().x).fold(0.0, f32::max) + icon + 8.0;
    let height = galleys.len() as f32 * line_h;
    let bottom_right = rect.right_bottom() - vec2(12.0, below);
    let bg = Rect::from_min_max(
        bottom_right - vec2(width + 12.0, height + 10.0),
        bottom_right,
    );
    painter.rect_filled(bg, 4.0, crate::theme::MAP_BG.gamma_multiply(0.85));
    let mut y = bg.top() + 5.0;
    for (k, g) in galleys.into_iter().enumerate() {
        let x = bg.left() + 6.0;
        if k < rows.len() {
            let sq = Rect::from_min_size(egui::pos2(x, y + 2.0), vec2(icon, icon - 2.0));
            painter.rect_stroke(sq, 0.0, Stroke::new(2.0, RED), StrokeKind::Inside);
        }
        painter.galley(egui::pos2(x + icon + 6.0, y), g, Color32::WHITE);
        y += line_h;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tam_yanip_sonme() {
        assert_eq!(blink(10.1, false).0, 1.0);
        assert_eq!(blink(10.7, false).0, 0.0);
        assert!((blink(10.1, false).1 - 0.5).abs() < 1e-9);
        assert_eq!(blink(10.7, true), (1.0, f64::INFINITY));
    }

    #[test]
    fn yanip_sonme_sinirlari() {
        assert_eq!(pulse(123.4, true), 1.0);
        for k in 0..100 {
            let p = pulse(f64::from(k) * 0.013, false);
            assert!((0.349..=1.0).contains(&p), "{p}");
        }
        // 1 Hz: bir saniye sonra aynı değer.
        assert!((pulse(0.3, false) - pulse(1.3, false)).abs() < 1e-5);
    }
}
