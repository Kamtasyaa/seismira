//! Sağ panel: deprem ayarları ve geçerlilik uyarıları (CLAUDE.md Bölüm 6).

use eframe::egui::{self, Color32, RichText};
use seismira_core::quake::{FaultType, Issue, Quake, Setting};
use seismira_core::shaking::has_gmpe;
use seismira_core::source::Fault;

use crate::i18n::Lang;

const BLOCK_COLOR: Color32 = Color32::from_rgb(0xFF, 0x6B, 0x6B);
const WARN_COLOR: Color32 = Color32::from_rgb(0xE8, 0xB4, 0x4C);

/// Paneldeki taslak deprem. Merkez üssü seçilene kadar Başlat kapalı.
pub struct Draft {
    pub quake: Quake,
    pub has_epicenter: bool,
}

impl Default for Draft {
    fn default() -> Self {
        Self {
            quake: Quake::new((0.0, 0.0)),
            has_epicenter: false,
        }
    }
}

pub enum Action {
    None,
    /// Taslak değişti (çalışan simülasyon iptal edilir).
    Changed,
    Start,
}

/// Dile göre ondalık ayraçlı sayı kutusu.
fn number<'a>(
    lang: Lang,
    v: &'a mut f64,
    range: std::ops::RangeInclusive<f64>,
    decimals: usize,
    speed: f64,
) -> egui::DragValue<'a> {
    egui::DragValue::new(v)
        .range(range)
        .speed(speed)
        .custom_formatter(move |v, _| lang.num(v, decimals))
        .custom_parser(|s| s.trim().replace(',', ".").parse().ok())
}

fn setting_key(s: Setting) -> &'static str {
    match s {
        Setting::ShallowCrust => "setting-crust",
        Setting::Interface => "setting-interface",
        Setting::Intraslab => "setting-intraslab",
    }
}

fn fault_key(f: FaultType) -> &'static str {
    match f {
        FaultType::StrikeSlip => "fault-ss",
        FaultType::Normal => "fault-normal",
        FaultType::Reverse => "fault-reverse",
    }
}

pub fn issue_text(lang: Lang, issue: Issue) -> String {
    match issue {
        Issue::MwOutOfRange { min, max } => lang.t_args(
            "issue-mw",
            &[("min", lang.num(min, 1)), ("max", lang.num(max, 1))],
        ),
        Issue::DepthOutOfRange { min, max } => lang.t_args(
            "issue-depth",
            &[("min", lang.num(min, 0)), ("max", lang.num(max, 0))],
        ),
        Issue::InterfaceNotReverse => lang.t("issue-interface"),
        Issue::DeepFocus => lang.t("issue-deep"),
        Issue::BadGeometry => lang.t("issue-geometry"),
    }
}

pub fn ui(
    ui: &mut egui::Ui,
    lang: Lang,
    draft: &mut Draft,
    issues: &[Issue],
    can_run: bool,
) -> Action {
    let t = |k| lang.t(k);
    let before = draft.quake;
    let q = &mut draft.quake;
    let mut action = Action::None;

    egui::Grid::new("quake-main")
        .num_columns(2)
        .spacing([10.0, 8.0])
        .show(ui, |ui| {
            ui.label(t("lat"));
            if ui
                .add(number(lang, &mut q.epicenter.1, -90.0..=90.0, 3, 0.01))
                .changed()
            {
                draft.has_epicenter = true;
            }
            ui.end_row();
            ui.label(t("lon"));
            if ui
                .add(number(lang, &mut q.epicenter.0, -180.0..=180.0, 3, 0.01))
                .changed()
            {
                draft.has_epicenter = true;
            }
            ui.end_row();

            ui.label(t("mw"));
            // Kutunun aralığı geçerlilikten geniş: aralık dışı değer
            // girilebilir ama açıkça uyarılır ve engellenir.
            ui.add(number(lang, &mut q.mw, 3.0..=9.9, 1, 0.02));
            ui.end_row();
            ui.label(t("depth"));
            ui.add(number(lang, &mut q.depth_km, 0.0..=800.0, 0, 0.5));
            ui.end_row();

            ui.label(t("setting"));
            egui::ComboBox::from_id_salt("setting")
                .selected_text(t(setting_key(q.setting)))
                .show_ui(ui, |ui| {
                    for s in [
                        Setting::ShallowCrust,
                        Setting::Interface,
                        Setting::Intraslab,
                    ] {
                        ui.selectable_value(&mut q.setting, s, t(setting_key(s)));
                    }
                });
            ui.end_row();

            ui.label(t("fault"));
            let mut fault = q.fault;
            egui::ComboBox::from_id_salt("fault")
                .selected_text(t(fault_key(fault)))
                .show_ui(ui, |ui| {
                    for f in [FaultType::StrikeSlip, FaultType::Normal, FaultType::Reverse] {
                        ui.selectable_value(&mut fault, f, t(fault_key(f)));
                    }
                });
            if fault != q.fault {
                q.fault = fault;
                (q.dip, q.rake) = fault.default_dip_rake();
            }
            ui.end_row();
        });

    ui.add_space(4.0);
    egui::CollapsingHeader::new(t("fault-geometry"))
        .default_open(true)
        .show(ui, |ui| {
            egui::Grid::new("quake-geom")
                .num_columns(2)
                .spacing([10.0, 8.0])
                .show(ui, |ui| {
                    ui.label(t("strike"));
                    let r = ui.add(number(lang, &mut q.strike, 0.0..=360.0, 0, 1.0));
                    if r.changed() {
                        q.strike = q.strike.rem_euclid(360.0);
                    }
                    ui.end_row();
                    ui.label(t("dip"));
                    ui.add(number(lang, &mut q.dip, 0.0..=90.0, 0, 0.5));
                    ui.end_row();
                    ui.label(t("rake"));
                    ui.add(number(lang, &mut q.rake, -180.0..=180.0, 0, 1.0));
                    ui.end_row();
                });
        });

    egui::CollapsingHeader::new(t("advanced"))
        .default_open(false)
        .show(ui, |ui| {
            egui::Grid::new("quake-adv")
                .num_columns(2)
                .spacing([10.0, 8.0])
                .show(ui, |ui| {
                    ui.label(t("hypo-along"));
                    let mut pct = q.hypo_along * 100.0;
                    if ui
                        .add(
                            egui::Slider::new(&mut pct, 0.0..=100.0)
                                .integer()
                                .suffix(" %"),
                        )
                        .changed()
                    {
                        q.hypo_along = pct / 100.0;
                    }
                    ui.end_row();
                    ui.label(t("hypo-down"));
                    let mut pct = q.hypo_down * 100.0;
                    if ui
                        .add(
                            egui::Slider::new(&mut pct, 0.0..=100.0)
                                .integer()
                                .suffix(" %"),
                        )
                        .changed()
                    {
                        q.hypo_down = pct / 100.0;
                    }
                    ui.end_row();
                });
            let f = Fault::from_quake(q);
            ui.label(
                RichText::new(lang.t_args(
                    "rupture-size",
                    &[
                        ("l", lang.num(f.length, 0)),
                        ("w", lang.num(f.width, 0)),
                        ("ztor", lang.num(f.ztor, 1)),
                    ],
                ))
                .weak(),
            );
        });

    if draft.quake != before {
        action = Action::Changed;
    }

    ui.add_space(8.0);
    for &issue in issues {
        let (icon, color) = if issue.blocks() {
            ("⛔", BLOCK_COLOR)
        } else {
            ("⚠", WARN_COLOR)
        };
        ui.label(RichText::new(format!("{icon} {}", issue_text(lang, issue))).color(color));
    }
    if !has_gmpe(draft.quake.setting) {
        ui.label(RichText::new(format!("ⓘ {}", t("no-gmpe"))).color(WARN_COLOR));
    }
    if !draft.has_epicenter {
        ui.label(RichText::new(t("start-needs-epicenter")).weak());
    }

    ui.add_space(8.0);
    let start = egui::Button::new(RichText::new(t("button-start")).strong())
        .min_size(egui::vec2(140.0, 34.0));
    if ui.add_enabled(can_run, start).clicked() {
        action = Action::Start;
    }
    ui.add_space(8.0);
    ui.label(RichText::new(t("ranges-note")).small().weak());
    action
}
