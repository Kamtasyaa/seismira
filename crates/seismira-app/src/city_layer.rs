//! Şehir katmanı: noktalar, adlar, P varış işaretleri, Shindo kutuları
//! (CLAUDE.md Bölüm 2 adım 4–5, Bölüm 10).

use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Stroke, vec2};
use seismira_core::cities::City;
use seismira_core::intensity::Shindo;
use std::path::Path;
use std::sync::Arc;

use crate::compute::CityResults;
use crate::map::MapWidget;
use crate::shindo_style;

/// En fazla kaç ad etiketi ve kaç Shindo kutusu çizilir (okunabilirlik).
const MAX_LABELS: usize = 250;
const MAX_BOXES: usize = 300;

pub struct CityLayer {
    pub cities: Arc<Vec<City>>,
    /// Şehir başına zemin (yapılaşma ağırlıklı Vs30, `.ssit`); yoksa
    /// hesap Vs30 haritasından noktasal okur.
    pub sites: Option<Arc<Vec<seismira_core::site::Site>>>,
}

/// Bir karede çizilen şehirler (fare ile seçme için).
pub struct Drawn {
    pub hovered: Option<usize>,
}

impl CityLayer {
    pub fn load(path: &Path) -> Result<Self, String> {
        let file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let cities = seismira_core::cities::read(&mut std::io::BufReader::new(file))
            .map_err(|e| format!("{}: {e}", path.display()))?;
        log::info!("{} şehir yüklendi", cities.len());
        let site_path = path.with_extension("ssit");
        let sites = std::fs::File::open(&site_path)
            .and_then(|f| seismira_core::site::read_sites(&mut std::io::BufReader::new(f)))
            .inspect_err(|e| log::warn!("{}: {e}", site_path.display()))
            .ok()
            .filter(|s| {
                let ok = s.len() == cities.len();
                if !ok {
                    log::warn!(
                        "{}: şehir sayısı uyuşmuyor, kullanılmıyor",
                        site_path.display()
                    );
                }
                ok
            })
            .map(Arc::new);
        Ok(Self {
            cities: Arc::new(cities),
            sites,
        })
    }

    /// Ölçeğe göre ad gösterilecek en küçük nüfus. Görsel ayar.
    fn min_population(scale: f64) -> u32 {
        match scale {
            s if s < 1_500.0 => 5_000_000,
            s if s < 4_000.0 => 1_000_000,
            s if s < 12_000.0 => 300_000,
            s if s < 40_000.0 => 80_000,
            s if s < 150_000.0 => 30_000,
            _ => 0,
        }
    }

    /// Shindo kutusu adaylarının sırası: en güçlü sarsıntı, sonra nüfus.
    pub fn rank_for_boxes(&self, results: &CityResults) -> Vec<usize> {
        let mut idx: Vec<usize> = results
            .sites
            .iter()
            .enumerate()
            .filter(|(_, r)| r.motion.is_some_and(|m| m.shindo() >= Shindo::S1))
            .map(|(i, _)| i)
            .collect();
        idx.sort_by(|&a, &b| {
            let sa = results.sites[a].motion.map(|m| m.jma).unwrap_or(0.0);
            let sb = results.sites[b].motion.map(|m| m.jma).unwrap_or(0.0);
            sb.total_cmp(&sa)
                .then(self.cities[b].population.cmp(&self.cities[a].population))
        });
        idx
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &self,
        ui: &egui::Ui,
        rect: Rect,
        map: &MapWidget,
        sim: Option<(&CityResults, &[usize], f64)>,
        p_color: Color32,
        hover: Option<Pos2>,
    ) -> Drawn {
        let painter = ui.painter_at(rect);
        let label_font = FontId::proportional(12.0);
        let text = ui.visuals().text_color().gamma_multiply(0.85);
        let dot = Color32::from_gray(200);
        let min_pop = Self::min_population(map.scale());
        let inside = rect.shrink(2.0);

        let mut occupied: Vec<Rect> = Vec::new();
        let mut placed: Vec<bool> = vec![false; self.cities.len()];
        // (şehir, nokta, varsa kutu): fare ile seçme için.
        let mut candidates: Vec<(usize, Pos2, Option<Rect>)> = Vec::new();

        // Şehrin o anki Shindo'su (S ulaştıktan sonra kademeli yükselir).
        let shindo_at = |i: usize| -> Option<Shindo> {
            let (results, _, t) = sim?;
            results.sites[i].shindo_at(t)
        };
        // P ulaştı, S henüz değil.
        let p_only = |i: usize| -> bool {
            sim.is_some_and(|(results, _, t)| {
                let r = &results.sites[i];
                r.t_p.is_some_and(|tp| t >= tp) && r.t_s.is_none_or(|ts| t < ts)
            })
        };
        let shown = |c: &City| c.population >= min_pop || (c.capital && map.scale() >= 1_500.0);

        // 1) Shindo kutuları (S ulaşmışsa), en güçlüden başlayarak.
        if let Some((_, ranked, _)) = sim {
            let mut drawn = 0;
            for &i in ranked {
                if drawn >= MAX_BOXES {
                    break;
                }
                if placed[i] {
                    continue;
                }
                let Some(s) = shindo_at(i) else { continue };
                let c = &self.cities[i];
                let p = map.to_screen(rect, (c.lon, c.lat));
                if !inside.contains(p) {
                    continue;
                }
                let box_rect = Rect::from_center_size(p - vec2(0.0, 13.0), shindo_style::BOX_SIZE);
                if occupied.iter().any(|o| o.intersects(box_rect.expand(1.0))) {
                    continue;
                }
                painter.circle_filled(p, 2.5, dot);
                shindo_style::draw_box(&painter, box_rect.center(), s);
                occupied.push(box_rect);
                placed[i] = true;
                candidates.push((i, p, Some(box_rect)));
                drawn += 1;
            }
        }

        // 2) Şehir adları (nüfusa göre; dosya nüfusa göre azalan sırada).
        let mut labels = 0;
        for (i, c) in self.cities.iter().enumerate() {
            if labels >= MAX_LABELS {
                break;
            }
            if placed[i] || !shown(c) {
                continue;
            }
            let p = map.to_screen(rect, (c.lon, c.lat));
            if !inside.contains(p) {
                continue;
            }
            let galley = painter.layout_no_wrap(c.name.clone(), label_font.clone(), text);
            let label_rect =
                Rect::from_min_size(p + vec2(5.0, -galley.size().y / 2.0), galley.size());
            let dot_rect = Rect::from_center_size(p, vec2(6.0, 6.0));
            if occupied
                .iter()
                .any(|o| o.intersects(label_rect) || o.intersects(dot_rect))
            {
                continue;
            }
            if p_only(i) {
                painter.circle_stroke(p, 5.0, Stroke::new(1.5, p_color));
            }
            match shindo_at(i) {
                // Kutusu komşu kutuyla çakıştığı için çizilmedi: nokta Shindo
                // renginde (değer fare ile kartta; renk tek taşıyıcı değil).
                Some(s) => {
                    painter.circle_filled(p, 4.0, shindo_style::color(s));
                    painter.circle_stroke(p, 4.0, Stroke::new(1.0, Color32::from_gray(20)));
                }
                None => {
                    painter.circle_filled(p, 2.5, dot);
                }
            }
            painter.galley(label_rect.min, galley, text);
            occupied.push(label_rect);
            occupied.push(dot_rect);
            candidates.push((i, p, None));
            labels += 1;
        }

        // 3) Fare altındaki şehir: üstündeki Shindo kutusu ya da noktaya en
        // yakın (10 point içinde).
        let hovered = hover.and_then(|h| {
            candidates
                .iter()
                .find(|(_, _, b)| b.is_some_and(|b| b.expand(1.0).contains(h)))
                .map(|&(i, _, _)| i)
                .or_else(|| {
                    candidates
                        .iter()
                        .map(|&(i, p, _)| (i, p.distance(h)))
                        .filter(|&(_, d)| d < 10.0)
                        .min_by(|a, b| a.1.total_cmp(&b.1))
                        .map(|(i, _)| i)
                })
        });
        Drawn { hovered }
    }
}

/// Model geçerlilik sınırı çizgisinin rengi.
pub const MODEL_LIMIT_COLOR: Color32 = Color32::from_rgba_premultiplied(150, 150, 160, 170);

/// Sağ altta Shindo ölçek açıklaması (Bölüm 10).
pub fn draw_legend(ui: &egui::Ui, rect: Rect, title: &str, limit_note: Option<&str>) {
    let painter = ui.painter_at(rect);
    let n = Shindo::ALL.len() - 1; // 0 hariç
    let w = shindo_style::BOX_SIZE.x + 2.0;
    let width = n as f32 * w;
    let note_h = if limit_note.is_some() { 18.0 } else { 0.0 };
    let origin = rect.right_bottom() - vec2(12.0 + width, 12.0 + shindo_style::BOX_SIZE.y + note_h);
    let bg = Rect::from_min_max(
        origin - vec2(6.0, 22.0),
        rect.right_bottom() - vec2(6.0, 6.0),
    );
    painter.rect_filled(bg, 4.0, crate::theme::MAP_BG.gamma_multiply(0.85));
    painter.text(
        origin - vec2(0.0, 4.0),
        Align2::LEFT_BOTTOM,
        title,
        FontId::proportional(12.0),
        ui.visuals().text_color(),
    );
    for (k, s) in Shindo::ALL.iter().skip(1).enumerate() {
        let c = origin + vec2(k as f32 * w + w / 2.0, shindo_style::BOX_SIZE.y / 2.0);
        shindo_style::draw_box(&painter, c, *s);
    }
    if let Some(note) = limit_note {
        let y = origin.y + shindo_style::BOX_SIZE.y + 11.0;
        painter.add(egui::Shape::dashed_line(
            &[egui::pos2(origin.x, y), egui::pos2(origin.x + 22.0, y)],
            Stroke::new(1.5, MODEL_LIMIT_COLOR),
            5.0,
            3.0,
        ));
        painter.text(
            egui::pos2(origin.x + 28.0, y),
            Align2::LEFT_CENTER,
            note,
            FontId::proportional(11.5),
            ui.visuals().weak_text_color(),
        );
    }
}
