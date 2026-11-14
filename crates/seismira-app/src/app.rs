//! Ana pencere ve panel düzeni (CLAUDE.md Bölüm 10).

use eframe::egui::{self, Align2, Color32, FontId, RichText, Stroke};
use seismira_core::sim::{Event, EventKind, Simulation};
use seismira_core::traveltime::{Front, Phase, TravelTimes};
use seismira_core::{geo, mercator, quake};
use std::path::PathBuf;

use crate::map::{MapWidget, view::MapView};
use crate::timeline::{self, Clock};
use crate::{fonts, i18n::Lang, panel, paths, text_test, theme};

/// Dalga halkası renkleri. P (önce gelen, zayıf sarsıntı) soğuk, S (güçlü
/// sarsıntı) sıcak renk. Renk tek bilgi taşıyıcı değildir: haritada
/// açıklama kutusu var (Bölüm 3.2).
const P_COLOR: Color32 = Color32::from_rgb(0x5A, 0xB0, 0xFF);
const S_COLOR: Color32 = Color32::from_rgb(0xFF, 0x8A, 0x45);
/// Halka başına jeodezik nokta sayısı.
const RING_POINTS: usize = 256;

const STORAGE_LANG: &str = "lang";

pub struct SeismiraApp {
    lang: Lang,
    assets: PathBuf,
    show_text_test: bool,
    /// Eğitim uyarısı. Kapatılabilir ama her açılışta yeniden görünür
    /// (bilerek kaydedilmez); Görünüm menüsünden geri açılır.
    show_disclaimer: bool,
    map: MapWidget,
    /// Sağ paneldeki taslak deprem.
    draft: panel::Draft,
    /// Çalışan simülasyon; taslak değişince iptal edilir.
    sim: Option<Simulation>,
    clock: Clock,
    travel_times: Result<TravelTimes, String>,
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
        // Küçük tablo (~0.5 MB): açılışta, ilk kareden önce okunur.
        let tt_path = paths::data_dir().join("traveltime").join("ak135.ttt");
        let travel_times = TravelTimes::load(&tt_path).map_err(|e| {
            log::error!("{}: {e}", tt_path.display());
            format!("{}: {e}", tt_path.display())
        });
        Self {
            lang,
            assets,
            show_text_test: false,
            show_disclaimer: true,
            map: MapWidget::new(paths::data_dir(), initial_view()),
            draft: panel::Draft::default(),
            sim: None,
            clock: Clock::default(),
            travel_times,
            hover: None,
        }
        .with_demo()
    }

    /// Geliştirme: `SEISMIRA_DEMO=boylam,enlem,Mw,derinlik,saniye` ile depremi
    /// başlatıp verilen anda duraklatır (ekran görüntüsü testleri için).
    fn with_demo(mut self) -> Self {
        let Some(v) = std::env::var("SEISMIRA_DEMO").ok().map(|s| {
            s.split(',')
                .filter_map(|x| x.trim().parse::<f64>().ok())
                .collect::<Vec<_>>()
        }) else {
            return self;
        };
        if let [lon, lat, mw, depth, t] = v[..] {
            let q = &mut self.draft.quake;
            q.epicenter = (lon, lat);
            q.mw = mw;
            q.depth_km = depth;
            if depth > quake::limits::CRUST_DEPTH.1 {
                q.setting = quake::Setting::Intraslab;
            }
            self.draft.has_epicenter = true;
            self.start_sim();
            self.clock.t = t;
            self.clock.playing = false;
        }
        self
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
        if let Some(c) = out.clicked {
            self.draft.quake.epicenter = c;
            self.draft.has_epicenter = true;
            self.cancel_sim();
        }
        self.hover = out.hover;

        if let Some(sim) = &self.sim {
            let (p, s) = sim.fronts(self.clock.t);
            let center = sim.quake.epicenter;
            for (front, color) in [(s, S_COLOR), (p, P_COLOR)] {
                if let Front::At(deg) = front
                    && deg > 0.0
                {
                    let pts = geo::ring(center, deg, RING_POINTS);
                    self.map
                        .draw_lonlat_line(ui, rect, &pts, Stroke::new(2.0, color));
                }
            }
            self.map.draw_epicenter(ui, rect, center);
            self.legend(ui, rect);
        } else if self.draft.has_epicenter {
            let q = &self.draft.quake;
            self.map
                .draw_strike_preview(ui, rect, q.epicenter, q.strike);
            self.map.draw_epicenter(ui, rect, q.epicenter);
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

impl SeismiraApp {
    fn cancel_sim(&mut self) {
        self.sim = None;
        self.clock = Clock {
            speed: self.clock.speed,
            ..Clock::default()
        };
    }

    fn start_sim(&mut self) {
        let Ok(tt) = &self.travel_times else {
            return;
        };
        if quake::validate(&self.draft.quake)
            .iter()
            .any(|i| i.blocks())
        {
            log::warn!("geçersiz girdiyle başlatma engellendi");
            return;
        }
        self.sim = Simulation::new(self.draft.quake, tt);
        self.clock = Clock {
            t: 0.0,
            playing: self.sim.is_some(),
            speed: self.clock.speed,
        };
    }

    /// Sağ üstte dalga renk açıklaması.
    fn legend(&self, ui: &egui::Ui, rect: egui::Rect) {
        let painter = ui.painter_at(rect);
        let font = FontId::proportional(13.0);
        let mut y = rect.top() + 12.0;
        let right = rect.right() - 12.0;
        for (key, color) in [("wave-p", P_COLOR), ("wave-s", S_COLOR)] {
            let galley =
                painter.layout_no_wrap(self.lang.t(key), font.clone(), ui.visuals().text_color());
            let text_pos = egui::pos2(right - galley.size().x, y);
            let line_y = y + galley.size().y / 2.0;
            let bg = egui::Rect::from_min_max(
                egui::pos2(text_pos.x - 30.0, y),
                egui::pos2(right, y + galley.size().y),
            )
            .expand(4.0);
            painter.rect_filled(bg, 4.0, theme::MAP_BG.gamma_multiply(0.85));
            painter.line_segment(
                [
                    egui::pos2(text_pos.x - 26.0, line_y),
                    egui::pos2(text_pos.x - 6.0, line_y),
                ],
                Stroke::new(2.0, color),
            );
            painter.galley(text_pos, galley, ui.visuals().text_color());
            y += 24.0;
        }
    }

    fn event_text(&self, sim: &Simulation, e: &Event) -> String {
        let lang = self.lang;
        let limit = || lang.num(sim.max_distance(), 0);
        match e.kind {
            EventKind::Origin => lang.t_args(
                "event-origin",
                &[
                    ("mw", lang.num(sim.quake.mw, 1)),
                    ("depth", lang.num(sim.quake.depth_km, 0)),
                ],
            ),
            EventKind::Surfaced(Phase::P) => lang.t("event-surfaced-p"),
            EventKind::Surfaced(Phase::S) => lang.t("event-surfaced-s"),
            EventKind::ReachedLimit(Phase::P) => lang.t_args("event-limit-p", &[("deg", limit())]),
            EventKind::ReachedLimit(Phase::S) => lang.t_args("event-limit-s", &[("deg", limit())]),
        }
    }

    /// Sol panel: bildirim akışı, en yenisi üstte. Tıklanınca harita oraya gider.
    fn notifications(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        ui.add_space(6.0);
        ui.heading(lang.t("panel-notifications"));
        let Some(sim) = &self.sim else {
            ui.label(RichText::new(lang.t("notifications-empty")).weak());
            return;
        };
        let mut focus = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            for e in sim.events_until(self.clock.t).iter().rev() {
                let color = match e.kind {
                    EventKind::Origin => Color32::from_rgb(0xFF, 0x6B, 0x6B),
                    EventKind::Surfaced(_) => ui.visuals().text_color(),
                    EventKind::ReachedLimit(_) => ui.visuals().weak_text_color(),
                };
                let time = lang.t_args("time-elapsed", &[("t", timeline::format_hms(e.t))]);
                let r = egui::Frame::group(ui.style())
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(RichText::new(time).small().weak().monospace());
                        ui.label(RichText::new(self.event_text(sim, e)).color(color));
                    })
                    .response
                    .interact(egui::Sense::click());
                if r.clicked() {
                    focus = Some(e.location);
                }
            }
        });
        if let Some(loc) = focus {
            self.map.center_on(loc);
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
                timeline::ui(ui, lang, &mut self.clock, self.sim.as_ref());
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
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let issues = quake::validate(&self.draft.quake);
                    let can_run = self.draft.has_epicenter
                        && self.travel_times.is_ok()
                        && !issues.iter().any(|i| i.blocks());
                    match panel::ui(ui, lang, &mut self.draft, &issues, can_run) {
                        panel::Action::Changed => self.cancel_sim(),
                        panel::Action::Start => self.start_sim(),
                        panel::Action::None => {}
                    }
                    if self.travel_times.is_err() {
                        ui.colored_label(ui.visuals().error_fg_color, t("traveltime-missing"));
                    }
                });
            });

        egui::Panel::left("left")
            .default_size(280.0)
            .size_range(220.0..=420.0)
            .show(ui, |ui| self.notifications(ui));

        // Simülasyon saati: duvar saatine göre ilerler; oynarken sürekli çizim.
        if let Some(sim) = &self.sim {
            let dt = f64::from(ui.input(|i| i.stable_dt));
            self.clock.advance(dt, sim.end_time());
            if self.clock.playing {
                ui.ctx().request_repaint();
            }
        }

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
