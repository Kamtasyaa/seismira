//! Ana pencere ve panel düzeni (CLAUDE.md Bölüm 10).

use eframe::egui::{self, Align2, FontId, RichText};
use seismira_core::mercator;
use std::path::PathBuf;

use crate::map::{MapWidget, view::MapView};
use crate::{fonts, i18n::Lang, paths, text_test, theme};

const STORAGE_LANG: &str = "lang";

pub struct SeismiraApp {
    lang: Lang,
    assets: PathBuf,
    show_text_test: bool,
    /// Eğitim uyarısı. Kapatılabilir ama her açılışta yeniden görünür
    /// (bilerek kaydedilmez); Görünüm menüsünden geri açılır.
    show_disclaimer: bool,
    map: MapWidget,
    /// Seçilen merkez üssü (boylam, enlem).
    epicenter: Option<(f64, f64)>,
    /// İmlecin altındaki nokta (boylam, enlem).
    hover: Option<(f64, f64)>,
}

/// Başlangıç görünümü. Geliştirme için `SEISMIRA_VIEW=boylam,enlem,ölçek`.
fn initial_view() -> MapView {
    let parsed = std::env::var("SEISMIRA_VIEW").ok().and_then(|s| {
        let v: Vec<f64> = s.split(',').filter_map(|x| x.trim().parse().ok()).collect();
        (v.len() == 3).then(|| (v[0], v[1], v[2]))
    });
    // Varsayılan: Avrupa, Afrika ve Asya'nın batısı görünür.
    let (lon, lat, scale) = parsed.unwrap_or((30.0, 30.0, 1800.0));
    let (x, y) = mercator::to_unit(lon, lat);
    MapView::new([x, y], scale)
}

impl SeismiraApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let lang = cc
            .storage
            .and_then(|s| s.get_string(STORAGE_LANG))
            .and_then(|code| Lang::from_code(&code))
            .unwrap_or_else(Lang::from_system);
        let assets = paths::assets_dir();
        log::info!("veri klasörü: {}", assets.display());
        theme::apply(&cc.egui_ctx);
        fonts::install(&cc.egui_ctx, &assets, lang);
        Self {
            lang,
            assets,
            show_text_test: false,
            show_disclaimer: true,
            map: MapWidget::new(paths::data_dir(), initial_view()),
            epicenter: None,
            hover: None,
        }
    }

    fn set_lang(&mut self, ctx: &egui::Context, lang: Lang) {
        if lang != self.lang {
            self.lang = lang;
            fonts::install(ctx, &self.assets, lang);
        }
    }

    fn menu_bar(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        let t = |k| lang.t(k);
        let mut new_lang = self.lang;
        egui::MenuBar::new().ui(ui, |ui| {
            ui.label(RichText::new(t("app-title")).strong());
            ui.separator();
            ui.menu_button(t("menu-view"), |ui| {
                ui.checkbox(&mut self.show_disclaimer, t("menu-disclaimer"));
                ui.checkbox(&mut self.show_text_test, t("menu-text-test"));
            });
            ui.menu_button(t("menu-language"), |ui| {
                for lang in Lang::ALL {
                    ui.radio_value(&mut new_lang, lang, lang.endonym());
                }
            });
        });
        let ctx = ui.ctx().clone();
        self.set_lang(&ctx, new_lang);
    }
}

impl SeismiraApp {
    fn map_area(&mut self, ui: &mut egui::Ui, frame: &eframe::Frame) {
        let lang = self.lang;
        let rect = ui.max_rect();
        let out = self.map.ui(ui, frame, crate::MSAA);
        if out.clicked.is_some() {
            self.epicenter = out.clicked;
        }
        self.hover = out.hover;
        if let Some(e) = self.epicenter {
            self.map.draw_epicenter(ui, rect, e);
        }

        let painter = ui.painter_at(rect);
        let weak = ui.visuals().weak_text_color();
        if !self.map.is_ready() {
            let msg = if self.map.errors().is_empty() {
                lang.t("map-loading")
            } else {
                format!(
                    "{}\n\npython3 scripts/prepare_data.py",
                    lang.t("map-missing")
                )
            };
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                msg,
                FontId::proportional(15.0),
                weak,
            );
        }
        // Sol altta imlecin koordinatı.
        if let Some((lon, lat)) = self.hover {
            let galley = painter.layout_no_wrap(
                lang.lonlat(lon, lat),
                FontId::proportional(13.0),
                ui.visuals().text_color(),
            );
            let pos = rect.left_bottom() + egui::vec2(10.0, -10.0 - galley.size().y);
            let bg = egui::Rect::from_min_size(pos, galley.size()).expand(4.0);
            painter.rect_filled(bg, 4.0, theme::MAP_BG.gamma_multiply(0.85));
            painter.galley(pos, galley, weak);
        }
    }
}

impl eframe::App for SeismiraApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let lang = self.lang;
        let t = |k| lang.t(k);

        egui::Panel::top("menu").show(ui, |ui| {
            ui.add_space(3.0);
            self.menu_bar(ui);
            ui.add_space(1.0);
        });

        // Alt: zaman çizgisi, altında (kapatılabilir) eğitim uyarısı.
        egui::Panel::bottom("bottom")
            .resizable(false)
            .show(ui, |ui| {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new(t("timeline")).weak());
                });
                if self.show_disclaimer {
                    ui.separator();
                    egui::Sides::new().shrink_left().show(
                        ui,
                        |ui| {
                            ui.label(
                                RichText::new(format!("⚠ {}", t("disclaimer")))
                                    .color(theme::WARN_TEXT),
                            );
                        },
                        |ui| {
                            if ui
                                .small_button("✕")
                                .on_hover_text(t("disclaimer-close"))
                                .clicked()
                            {
                                self.show_disclaimer = false;
                            }
                        },
                    );
                }
                ui.add_space(2.0);
            });

        egui::Panel::right("settings")
            .default_size(300.0)
            .size_range(260.0..=420.0)
            .show(ui, |ui| {
                ui.add_space(6.0);
                ui.heading(t("panel-settings"));
                ui.separator();
                ui.label(RichText::new(t("epicenter")).strong());
                match self.epicenter {
                    Some((lon, lat)) => ui.label(lang.lonlat(lon, lat)),
                    None => ui.label(RichText::new(t("epicenter-hint")).weak()),
                };
                ui.add_space(12.0);
                ui.label(RichText::new(t("panel-settings-soon")).weak());
                ui.add_space(12.0);
                ui.add_enabled(
                    false,
                    egui::Button::new(t("button-start")).min_size(egui::vec2(120.0, 32.0)),
                );
            });

        egui::Panel::left("left")
            .default_size(280.0)
            .size_range(220.0..=420.0)
            .show(ui, |ui| {
                ui.add_space(6.0);
                ui.heading(t("panel-notifications"));
                ui.label(RichText::new(t("notifications-empty")).weak());
            });

        egui::CentralPanel::no_frame().show(ui, |ui| self.map_area(ui, frame));

        let mut open = self.show_text_test;
        egui::Window::new(t("text-test-title"))
            .open(&mut open)
            .default_pos([240.0, 48.0])
            .default_size([520.0, 700.0])
            .vscroll(true)
            .show(ui.ctx(), |ui| {
                ui.label(RichText::new(t("text-test-note")).small().weak());
                ui.separator();
                egui::Grid::new("samples")
                    .num_columns(2)
                    .spacing([16.0, 6.0])
                    .show(ui, |ui| {
                        for (name, text) in text_test::SAMPLES {
                            ui.label(RichText::new(*name).weak());
                            ui.label(RichText::new(*text).size(18.0));
                            ui.end_row();
                        }
                    });
            });
        self.show_text_test = open;

        #[cfg(feature = "screenshot")]
        crate::devshot::tick(ui.ctx(), self.map.is_ready());
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        storage.set_string(STORAGE_LANG, self.lang.code().to_owned());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::Harness;

    /// Ekran görüntüsü: `cargo test -p seismira-app -- --ignored ekran`
    /// Çıktı: target/screenshots/*.png. GPU gerektirdiği için CI'da çalışmaz.
    #[test]
    #[ignore]
    fn ekran_goruntusu() {
        let out = PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../target/screenshots"
        ));
        std::fs::create_dir_all(&out).unwrap();
        for (lang, text_test) in [(Lang::Tr, false), (Lang::Tr, true), (Lang::En, true)] {
            let mut harness = Harness::builder()
                .with_size([1280.0, 800.0])
                .wgpu()
                .build_eframe(|cc| {
                    let mut app = SeismiraApp::new(cc);
                    app.set_lang(&cc.egui_ctx, lang);
                    app.show_text_test = text_test;
                    app
                });
            harness.run();
            let img = harness.render().expect("render");
            let name = format!(
                "app_{}_{}.png",
                lang.code(),
                if text_test { "yazi" } else { "bos" }
            );
            img.save(out.join(&name)).unwrap();
        }
    }
}
