//! Olasılıksal tehlike haritası katmanı (ESHM20; [`seismira_core::hazard`]):
//! Görünüm → "Tehlike haritası (ESHM20)" ile katman seçilir (şiddet ölçüsü ×
//! olasılık). Harita: renk sınıflı doku (en yakın komşu), şehir kutularının
//! altında; sol üstte açıklama; imleç konumunda değer. Spektrum
//! penceresinde aynı noktanın eş tehlike spektrumu (spectrum_ui).
//!
//! Çizim: ızgara bir kez dokuya çevrilir (katman değişince yeniden);
//! Web Mercator'a 1°'lik enlem şeritleriyle yerleştirilir (şerit içinde
//! doğrusal: 60° enlemde 1°'lik şeritte kayma < 0,02°).

use super::SeismiraApp;
use eframe::egui::{self, Color32, Stroke, vec2};
use seismira_core::hazard::{HazardMap, LayerInfo, SCALE_G};

pub type Hazard = HazardMap<memmap2::Mmap>;

/// Sınıf sınırları (g) ve renkleri (sıralı, açıktan koyuya; renk körü
/// dostu sarı → kırmızı → mor). İlk sınıf (< 0,02 g) çizilmez.
pub const CLASS_EDGES_G: [f64; 10] = [0.02, 0.05, 0.1, 0.2, 0.3, 0.4, 0.6, 0.8, 1.0, 1.5];
pub const CLASS_COLORS: [Color32; 10] = [
    Color32::from_rgb(255, 255, 204),
    Color32::from_rgb(255, 237, 160),
    Color32::from_rgb(254, 217, 118),
    Color32::from_rgb(254, 178, 76),
    Color32::from_rgb(253, 141, 60),
    Color32::from_rgb(252, 78, 42),
    Color32::from_rgb(227, 26, 28),
    Color32::from_rgb(189, 0, 38),
    Color32::from_rgb(128, 0, 38),
    Color32::from_rgb(84, 0, 84),
];

fn class_color(g: f64) -> Color32 {
    match CLASS_EDGES_G.iter().rposition(|&e| g >= e) {
        Some(k) => CLASS_COLORS[k],
        None => Color32::TRANSPARENT,
    }
}

#[derive(Default)]
pub(super) struct HazardState {
    /// Seçili katman (dosyadaki sıra); `None`: kapalı.
    pub layer: Option<usize>,
    pub(super) texture: Option<(usize, egui::TextureHandle)>,
}

/// "PGA" / "SA(1,0 s)".
pub(super) fn imt_text(lang: crate::i18n::Lang, l: &LayerInfo) -> String {
    if l.period == 0.0 {
        "PGA".into()
    } else {
        format!(
            "SA({} s)",
            lang.num(l.period, if l.period < 1.0 { 1 } else { 0 })
        )
    }
}

/// "PGA — 50 yılda %10 (475 yıl)".
pub(super) fn layer_text(lang: crate::i18n::Lang, l: &LayerInfo) -> String {
    lang.t_args(
        "hazard-layer",
        &[
            ("imt", imt_text(lang, l)),
            ("p", lang.num(l.poe_in(50.0) * 100.0, 0)),
            ("rp", lang.num(l.return_period(), 0)),
        ],
    )
}

impl SeismiraApp {
    /// Görünüm menüsündeki seçim.
    pub(super) fn hazard_menu(&mut self, ui: &mut egui::Ui) {
        let Some(h) = &self.hazard_data else {
            return;
        };
        let lang = self.lang;
        let layers = h.layers();
        ui.menu_button(lang.t("menu-hazard"), |ui| {
            ui.radio_value(&mut self.hazard.layer, None, lang.t("hazard-off"));
            for (i, l) in layers.iter().enumerate() {
                ui.radio_value(&mut self.hazard.layer, Some(i), layer_text(lang, l));
            }
        });
    }

    /// Haritada katman (şehir kutularının altında).
    pub(super) fn draw_hazard(&mut self, ui: &egui::Ui, rect: egui::Rect) {
        let (Some(h), Some(layer)) = (&self.hazard_data, self.hazard.layer) else {
            return;
        };
        let g = h.grid;
        if self.hazard.texture.as_ref().is_none_or(|t| t.0 != layer) {
            let Ok(codes) = h.codes(layer) else {
                return;
            };
            let (nx, ny) = (g.nx as usize, g.ny as usize);
            let mut img = egui::ColorImage::filled([nx, ny], Color32::TRANSPARENT);
            for y in 0..ny {
                // Görüntü satırı 0 kuzey; ızgara satırı 0 güney.
                let src = (ny - 1 - y) * nx;
                for x in 0..nx {
                    let c = codes[src + x];
                    if c > 0 {
                        img.pixels[y * nx + x] = class_color(f64::from(c) * SCALE_G);
                    }
                }
            }
            let tex = ui
                .ctx()
                .load_texture("eshm20", img, egui::TextureOptions::NEAREST);
            self.hazard.texture = Some((layer, tex));
        }
        let Some((_, tex)) = &self.hazard.texture else {
            return;
        };
        // Düğümler piksel merkezi: kenarlar yarım adım dışarıda.
        let west = g.west - g.step / 2.0;
        let east = g.west + (f64::from(g.nx) - 0.5) * g.step;
        let south = g.south - g.step / 2.0;
        let north = g.south + (f64::from(g.ny) - 0.5) * g.step;
        let mut mesh = egui::Mesh::with_texture(tex.id());
        let tint = Color32::from_white_alpha(170);
        let mut lat = north;
        while lat > south {
            let bot = (lat - 1.0).max(south);
            let v0 = ((north - lat) / (north - south)) as f32;
            let v1 = ((north - bot) / (north - south)) as f32;
            let p = |lon: f64, la: f64| self.map.to_screen(rect, (lon, la));
            let base = mesh.vertices.len() as u32;
            for (pos, uv) in [
                (p(west, lat), egui::pos2(0.0, v0)),
                (p(east, lat), egui::pos2(1.0, v0)),
                (p(east, bot), egui::pos2(1.0, v1)),
                (p(west, bot), egui::pos2(0.0, v1)),
            ] {
                mesh.vertices.push(egui::epaint::Vertex {
                    pos,
                    uv,
                    color: tint,
                });
            }
            mesh.add_triangle(base, base + 1, base + 2);
            mesh.add_triangle(base, base + 2, base + 3);
            lat = bot;
        }
        ui.painter_at(rect).add(egui::Shape::mesh(mesh));
    }

    /// Sol üstte açıklama (sınıflar, katman, atıf).
    pub(super) fn draw_hazard_legend(&self, ui: &egui::Ui, rect: egui::Rect) {
        let (Some(h), Some(layer)) = (&self.hazard_data, self.hazard.layer) else {
            return;
        };
        let lang = self.lang;
        let Some(info) = h.layers().get(layer).copied() else {
            return;
        };
        let painter = ui.painter_at(rect);
        let font = egui::FontId::proportional(12.0);
        let text_c = ui.visuals().text_color();
        let weak = ui.visuals().weak_text_color();
        let title = painter.layout_no_wrap(
            lang.t_args("hazard-legend-title", &[("layer", layer_text(lang, &info))]),
            egui::FontId::proportional(13.0),
            text_c,
        );
        let credit = painter.layout_no_wrap(lang.t("hazard-credit"), font.clone(), weak);
        let sw = 30.0;
        let w = (title.size().x.max(credit.size().x)).max(sw * 10.0) + 16.0;
        let h_box = title.size().y + 14.0 + 16.0 + credit.size().y + 16.0;
        let top_left = rect.left_top() + vec2(12.0, 12.0);
        let bg = egui::Rect::from_min_size(top_left, vec2(w, h_box));
        painter.rect_filled(bg, 6.0, ui.visuals().extreme_bg_color.gamma_multiply(0.92));
        let mut y = top_left.y + 6.0;
        painter.galley(egui::pos2(top_left.x + 8.0, y), title, text_c);
        y += 20.0;
        for (k, c) in CLASS_COLORS.iter().enumerate() {
            let r = egui::Rect::from_min_size(
                egui::pos2(top_left.x + 8.0 + sw * k as f32, y),
                vec2(sw - 2.0, 10.0),
            );
            painter.rect_filled(r, 1.0, *c);
            painter.rect_stroke(
                r,
                1.0,
                Stroke::new(0.5, Color32::BLACK),
                egui::StrokeKind::Middle,
            );
            painter.text(
                egui::pos2(r.left(), r.bottom() + 1.0),
                egui::Align2::LEFT_TOP,
                lang.num(CLASS_EDGES_G[k], if CLASS_EDGES_G[k] < 0.1 { 2 } else { 1 }),
                egui::FontId::proportional(10.0),
                weak,
            );
        }
        y += 26.0;
        painter.galley(egui::pos2(top_left.x + 8.0, y), credit, weak);
    }

    /// İmleçteki değer metni (koordinat satırına eklenir).
    pub(super) fn hazard_at(&self, lon: f64, lat: f64) -> Option<String> {
        let (h, layer) = (self.hazard_data.as_ref()?, self.hazard.layer?);
        let info = *h.layers().get(layer)?;
        let v = h.value(layer, lon, lat)?;
        Some(format!(
            "{}: {} g",
            imt_text(self.lang, &info),
            self.lang.num(v, 2)
        ))
    }
}

/// Spektrum penceresi için: noktada %10 ve %2 (50 yıl) eş tehlike spektrumları.
pub(super) fn uhs_pair(h: &Hazard, lon: f64, lat: f64) -> Vec<(f64, Vec<(f64, f64)>)> {
    let mut poes: Vec<f64> = h.layers().iter().map(|l| l.annual_poe).collect();
    poes.sort_by(|a, b| b.total_cmp(a));
    poes.dedup_by(|a, b| (*a - *b).abs() < *b * 1e-3);
    poes.into_iter()
        .map(|p| (p, h.uhs(p, lon, lat)))
        .filter(|(_, u)| u.len() >= 2)
        .collect()
}
