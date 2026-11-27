//! Kritik tesisler (README "Önemli"; [`seismira_core::infra`]): hastane,
//! köprü / viyadük, baraj, trafo merkezi, doğalgaz boru hattı. Veri:
//! OpenStreetMap (ODbL), ana veri paketinde ayrı dosya.
//!
//! Harita: sarsıntı tesise ulaşınca (ilk S), Shindo 5- / MMI VII ve üstü
//! tesisler türüne özgü biçimde, şiddet rengiyle (biçim + renk: renk tek
//! bilgi taşıyıcı değil). Kalabalığı önlemek için ekranda [`DECLUTTER_PX`]
//! piksellik her hücrede yalnızca en güçlü sarsılan tesis çizilir;
//! yakınlaştıkça daha fazlası görünür. Pencere: türe göre özet, sıralı
//! liste, CSV.

use super::SeismiraApp;
use crate::compute::InfraResults;
use crate::mmi_style::{Level, Scale};
use eframe::egui::{self, Color32, RichText, Stroke, vec2};
use seismira_core::infra::{Assessed, Facility, Kind};
use seismira_core::intensity::Shindo;
use std::sync::Arc;

pub(super) struct InfraState {
    pub show: bool,
    pub on_map: bool,
    pub results: Option<Arc<InfraResults>>,
    filter: [bool; 6],
    msg: Option<String>,
}

impl Default for InfraState {
    fn default() -> Self {
        Self {
            show: false,
            on_map: true,
            results: None,
            filter: [true; 6],
            msg: None,
        }
    }
}

/// Seyreltme hücresi (piksel).
const DECLUTTER_PX: f32 = 24.0;

/// Haritada gösterilme eşiği: Shindo 5- / MMI VII (2026-11-27: Shindo 4'te
/// harita kutuları örtecek kadar kalabalıktı; liste ve CSV tümünü içerir).
fn notable(scale: Scale, a: &Assessed) -> bool {
    match scale {
        Scale::Shindo => Shindo::from_instrumental(a.jma) >= Shindo::S5Lower,
        Scale::Mmi => a.mmi >= 7.0,
    }
}

pub(super) fn kind_key(k: Kind) -> &'static str {
    match k {
        Kind::Hospital => "infra-hospital",
        Kind::Bridge => "infra-bridge",
        Kind::Viaduct => "infra-viaduct",
        Kind::Dam => "infra-dam",
        Kind::Substation => "infra-substation",
        Kind::GasPipeline => "infra-gas",
    }
}

fn level(scale: Scale, a: &Assessed) -> Level {
    Level::of(scale, Shindo::from_instrumental(a.jma), a.mmi)
}

fn display_name(lang: crate::i18n::Lang, f: &Facility) -> String {
    if f.name.is_empty() {
        lang.t("infra-unnamed")
    } else {
        f.name.clone()
    }
}

impl SeismiraApp {
    pub(super) fn infra_window(&mut self, ctx: &egui::Context) {
        let lang = self.lang;
        let t = |k: &str| lang.t(k);
        let scale = self.scale;
        let mut open = self.infra.show;
        let mut on_map = self.infra.on_map;
        let mut goto = None;
        let mut export = false;
        egui::Window::new(t("infra-title"))
            .open(&mut open)
            .default_pos([340.0, 110.0])
            .default_width(560.0)
            .show(ctx, |ui| {
                if self.infra_data.is_none() {
                    ui.label(t("infra-nodata"));
                    return;
                }
                let st = &mut self.infra;
                let Some(r) = st.results.clone() else {
                    ui.label(RichText::new(t("infra-start")).weak());
                    ui.label(RichText::new(t("infra-credit")).small().weak());
                    return;
                };
                ui.label(
                    RichText::new(lang.t_args(
                        "infra-count",
                        &[
                            ("n", lang.num(r.assessed.len() as f64, 0)),
                            ("q", lang.num(r.facilities.len() as f64, 0)),
                        ],
                    ))
                    .small(),
                );
                // Türe göre özet: model içinde, Shindo 5- / 6- ve üstü
                // (MMI'da VII / VIII ve üstü).
                let (hi_a, hi_b) = match scale {
                    Scale::Shindo => ("infra-col-5", "infra-col-6"),
                    Scale::Mmi => ("infra-col-7", "infra-col-8"),
                };
                egui::Grid::new("infra-summary")
                    .num_columns(5)
                    .spacing([14.0, 3.0])
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label("");
                        for k in ["infra-col-kind", "infra-col-total", hi_a, hi_b] {
                            ui.label(RichText::new(t(k)).weak().small());
                        }
                        ui.end_row();
                        for (ki, k) in Kind::ALL.iter().enumerate() {
                            let rows: Vec<&Assessed> = r
                                .assessed
                                .iter()
                                .filter(|a| r.facilities[a.index].kind == *k)
                                .collect();
                            let (a5, a6) = rows.iter().fold((0, 0), |(x, y), a| match scale {
                                Scale::Shindo => {
                                    let s = Shindo::from_instrumental(a.jma);
                                    (
                                        x + usize::from(s >= Shindo::S5Lower),
                                        y + usize::from(s >= Shindo::S6Lower),
                                    )
                                }
                                Scale::Mmi => {
                                    (x + usize::from(a.mmi >= 7.0), y + usize::from(a.mmi >= 8.0))
                                }
                            });
                            ui.checkbox(&mut st.filter[ki], "");
                            ui.horizontal(|ui| {
                                symbol_ui(ui, *k);
                                ui.label(t(kind_key(*k)));
                            });
                            ui.label(lang.num(rows.len() as f64, 0));
                            ui.label(lang.num(a5 as f64, 0));
                            ui.label(lang.num(a6 as f64, 0));
                            ui.end_row();
                        }
                    });
                ui.horizontal(|ui| {
                    ui.checkbox(&mut on_map, t("infra-on-map"));
                    if ui.button(t("infra-export")).clicked() {
                        export = true;
                    }
                });
                if let Some(m) = &st.msg {
                    ui.label(RichText::new(m).small());
                }
                ui.separator();
                let rows: Vec<&Assessed> = r
                    .assessed
                    .iter()
                    .filter(|a| st.filter[r.facilities[a.index].kind as usize])
                    .collect();
                ui.label(RichText::new(t("infra-list-note")).small().weak());
                let h = 18.0;
                egui::ScrollArea::vertical()
                    .max_height(300.0)
                    .auto_shrink([false, true])
                    .show_rows(ui, h, rows.len(), |ui, range| {
                        egui::Grid::new("infra-list")
                            .num_columns(6)
                            .spacing([10.0, 2.0])
                            .striped(true)
                            .min_row_height(h)
                            .show(ui, |ui| {
                                for a in &rows[range] {
                                    let f = &r.facilities[a.index];
                                    let lv = level(scale, a);
                                    let (_, rect) = ui.allocate_space(vec2(14.0, 14.0));
                                    paint_symbol(
                                        ui.painter(),
                                        rect.center(),
                                        f.kind,
                                        lv.color(),
                                        5.0,
                                    );
                                    let name = display_name(lang, f);
                                    let clicked = ui
                                        .allocate_ui(vec2(200.0, h), |ui| {
                                            ui.set_min_width(200.0);
                                            ui.add(
                                                egui::Label::new(RichText::new(&name).small())
                                                    .truncate()
                                                    .sense(egui::Sense::click()),
                                            )
                                            .on_hover_text(format!("{name}\n{}", t("infra-goto")))
                                            .clicked()
                                        })
                                        .inner;
                                    if clicked {
                                        goto = Some(a.at);
                                    }
                                    ui.label(RichText::new(lv.label()).small().strong());
                                    ui.label(
                                        RichText::new(format!("{} g", lang.num(a.pga_g, 2)))
                                            .small(),
                                    );
                                    ui.label(
                                        RichText::new(format!("{} cm/s", lang.num(a.pgv, 0)))
                                            .small(),
                                    );
                                    ui.label(
                                        RichText::new(format!("{} km", lang.num(a.rrup, 0)))
                                            .small(),
                                    );
                                    ui.end_row();
                                }
                            });
                    });
                ui.label(RichText::new(t("infra-method")).small().weak());
                ui.label(RichText::new(t("infra-credit")).small().weak());
            });
        self.infra.show = open;
        self.infra.on_map = on_map;
        if let Some(p) = goto {
            self.map.center_on(p);
        }
        if export {
            self.infra.msg = Some(self.infra_export());
        }
    }

    /// CSV: tüm değerlendirilen tesisler (dışa aktarma klasörüne).
    fn infra_export(&self) -> String {
        use std::fmt::Write as _;
        let lang = self.lang;
        let Some(r) = &self.infra.results else {
            return String::new();
        };
        let mut s = String::from(
            "# Seismira kritik tesisler / critical facilities. EĞİTİM SİMÜLASYONU / EDUCATIONAL SIMULATION.\n\
             # Veri / data: © OpenStreetMap contributors (ODbL 1.0)\n\
             kind,name,lon,lat,rrup_km,vs30,pga_g,pgv_cm_s,jma,shindo,mmi,attr\n",
        );
        for a in &r.assessed {
            let f = &r.facilities[a.index];
            let name = f.name.replace('"', "\"\"");
            let _ = writeln!(
                s,
                "{},\"{}\",{:.5},{:.5},{:.1},{:.0},{:.4},{:.2},{:.2},{},{:.2},{}",
                f.kind.code(),
                name,
                a.at.0,
                a.at.1,
                a.rrup,
                a.vs30,
                a.pga_g,
                a.pgv,
                a.jma,
                Shindo::from_instrumental(a.jma).label(),
                a.mmi,
                f.attr
            );
        }
        let dir = super::user_ui::export_base();
        let path = dir.join(format!(
            "kritik-tesisler-{}.csv",
            super::user_ui::timestamp()
        ));
        match std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(&path, s)) {
            Ok(()) => lang.t_args("export-done", &[("dir", path.display().to_string())]),
            Err(e) => format!("{}: {e}", path.display()),
        }
    }

    /// Haritada işaretler (sarsıntı ulaşmış, Shindo 5- / MMI VII ve üstü).
    pub(super) fn draw_infra(&self, ui: &egui::Ui, rect: egui::Rect) {
        if !self.infra.on_map {
            return;
        }
        let (Some(r), Some(_)) = (&self.infra.results, &self.sim) else {
            return;
        };
        let now = self.clock.t;
        let painter = ui.painter_at(rect);
        let hover = ui.input(|i| i.pointer.hover_pos());
        let mut hovered: Option<(&Assessed, egui::Pos2)> = None;
        // Seyreltme: güçlüden zayıfa, her hücrede ilk (en güçlü) tesis.
        let mut taken = std::collections::HashSet::new();
        let mut shown: Vec<(&Assessed, egui::Pos2)> = Vec::new();
        for a in &r.assessed {
            if !notable(self.scale, a) || a.t_s.is_none_or(|ts| now < ts) {
                continue;
            }
            let c = self.map.to_screen(rect, a.at);
            if !rect.expand(8.0).contains(c) {
                continue;
            }
            let cell = (
                ((c.x - rect.left()) / DECLUTTER_PX).floor() as i32,
                ((c.y - rect.top()) / DECLUTTER_PX).floor() as i32,
            );
            if taken.insert(cell) {
                shown.push((a, c));
            }
        }
        // Zayıftan güçlüye çiz: güçlüler üstte kalsın.
        for &(a, c) in shown.iter().rev() {
            let f = &r.facilities[a.index];
            let color = level(self.scale, a).color();
            if !f.line.is_empty() {
                let pts: Vec<egui::Pos2> = f
                    .line
                    .iter()
                    .map(|&(x, y)| self.map.to_screen(rect, (f64::from(x), f64::from(y))))
                    .collect();
                painter.add(egui::Shape::line(
                    pts.clone(),
                    Stroke::new(4.0, Color32::BLACK),
                ));
                painter.add(egui::Shape::line(pts, Stroke::new(2.0, color)));
            }
            paint_symbol(&painter, c, f.kind, color, 5.0);
            if hover.is_some_and(|h| h.distance(c) < 7.0) {
                hovered = Some((a, c));
            }
        }
        if let Some((a, c)) = hovered {
            let lang = self.lang;
            let f = &r.facilities[a.index];
            let text = lang.t_args(
                "infra-tip",
                &[
                    ("name", display_name(lang, f)),
                    ("kind", lang.t(kind_key(f.kind))),
                    ("level", level(self.scale, a).text(lang)),
                    ("pga", lang.num(a.pga_g, 2)),
                    ("pgv", lang.num(a.pgv, 0)),
                ],
            );
            let galley = painter.layout_no_wrap(
                text,
                egui::FontId::proportional(13.0),
                ui.visuals().text_color(),
            );
            let pos = c + vec2(10.0, -galley.size().y - 6.0);
            let bg = egui::Rect::from_min_size(pos, galley.size()).expand(4.0);
            painter.rect_filled(bg, 4.0, ui.visuals().extreme_bg_color);
            painter.galley(pos, galley, ui.visuals().text_color());
        }
    }
}

/// Tür simgesi (pencere satırı ve açıklama için, gri dolgu).
fn symbol_ui(ui: &mut egui::Ui, k: Kind) {
    let (_, rect) = ui.allocate_space(vec2(14.0, 14.0));
    paint_symbol(ui.painter(), rect.center(), k, Color32::from_gray(200), 5.0);
}

/// Türe özgü biçim: hastane kare + artı, köprü / viyadük yatay çubuk
/// (viyadükte iki ayak), baraj üçgen, trafo eşkenar dörtgen, boru hattı
/// daire.
fn paint_symbol(p: &egui::Painter, c: egui::Pos2, k: Kind, fill: Color32, r: f32) {
    let edge = Stroke::new(1.0, Color32::BLACK);
    match k {
        Kind::Hospital => {
            let rect = egui::Rect::from_center_size(c, vec2(2.0 * r, 2.0 * r));
            p.rect(rect, 1.0, fill, edge, egui::StrokeKind::Middle);
            let s = Stroke::new(1.5, Color32::BLACK);
            p.line_segment([c + vec2(-r * 0.6, 0.0), c + vec2(r * 0.6, 0.0)], s);
            p.line_segment([c + vec2(0.0, -r * 0.6), c + vec2(0.0, r * 0.6)], s);
        }
        Kind::Bridge | Kind::Viaduct => {
            let rect = egui::Rect::from_center_size(c, vec2(2.4 * r, 1.0 * r));
            p.rect(rect, 0.0, fill, edge, egui::StrokeKind::Middle);
            if k == Kind::Viaduct {
                let s = Stroke::new(1.5, Color32::BLACK);
                for dx in [-0.7 * r, 0.7 * r] {
                    p.line_segment([c + vec2(dx, 0.5 * r), c + vec2(dx, 1.3 * r)], s);
                }
            }
        }
        Kind::Dam => {
            p.add(egui::Shape::convex_polygon(
                vec![
                    c + vec2(0.0, -r),
                    c + vec2(r, r * 0.8),
                    c + vec2(-r, r * 0.8),
                ],
                fill,
                edge,
            ));
        }
        Kind::Substation => {
            p.add(egui::Shape::convex_polygon(
                vec![
                    c + vec2(0.0, -r),
                    c + vec2(r, 0.0),
                    c + vec2(0.0, r),
                    c + vec2(-r, 0.0),
                ],
                fill,
                edge,
            ));
        }
        Kind::GasPipeline => {
            p.circle(c, r * 0.8, fill, edge);
        }
    }
}
