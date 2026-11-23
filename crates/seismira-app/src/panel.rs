//! Sağ panel: deprem ayarları ve geçerlilik uyarıları (CLAUDE.md Bölüm 6).

use eframe::egui::{self, Color32, RichText};
use seismira_core::quake::{FaultType, Issue, Quake, Setting, limits};
use seismira_core::shaking::has_gmpe;
use seismira_core::slab::{self, Slab2};
use seismira_core::source::Fault;

use crate::i18n::Lang;

const BLOCK_COLOR: Color32 = Color32::from_rgb(0xFF, 0x6B, 0x6B);
const WARN_COLOR: Color32 = Color32::from_rgb(0xE8, 0xB4, 0x4C);

/// Paneldeki taslak deprem. Merkez üssü seçilene kadar Başlat kapalı.
pub struct Draft {
    pub quake: Quake,
    pub has_epicenter: bool,
    /// Seçili hazır senaryo (kimlik); girdi elle değişince düşer.
    pub preset: Option<String>,
}

impl Default for Draft {
    fn default() -> Self {
        Self {
            quake: Quake::new((0.0, 0.0)),
            has_epicenter: false,
            preset: None,
        }
    }
}

pub enum Action {
    None,
    /// Taslak değişti (çalışan simülasyon iptal edilir).
    Changed,
    /// Hazır senaryo seçildi: taslak değişti, harita merkez üssüne gitsin.
    Preset,
    Start,
    /// Çalışan simülasyonu kapat (haritaya tıklayınca yeniden merkez üssü
    /// seçilebilsin).
    Stop,
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
        Issue::DeepFocus { max } => lang.t_args("issue-deep", &[("max", lang.num(max, 0))]),
        Issue::BadGeometry => lang.t("issue-geometry"),
        Issue::RuptureSizeExtrapolated { max } => {
            lang.t_args("issue-rupture-size", &[("max", lang.num(max, 1))])
        }
        Issue::ZtorOutOfRange { ztor, max } => lang.t_args(
            "issue-ztor",
            &[("ztor", lang.num(ztor, 1)), ("max", lang.num(max, 0))],
        ),
        Issue::RuptureSizeInvalid { max_l, max_w } => lang.t_args(
            "issue-rupture-invalid",
            &[("l", lang.num(max_l, 0)), ("w", lang.num(max_w, 0))],
        ),
        Issue::RuptureWidthClamped { width } => {
            lang.t_args("issue-rupture-clamped", &[("w", lang.num(width, 0))])
        }
        Issue::RuptureAreaUnusual { ratio } => lang.t_args(
            "issue-rupture-area",
            &[("r", lang.num(ratio, if ratio < 1.0 { 2 } else { 1 }))],
        ),
    }
}

/// Dalma-batma: merkez üssünün altındaki Slab2 levhası, Parker bölgesi ve
/// arayüz depremi için "levha geometrisini uygula" (doğrultu, eğim, odak
/// derinliği = levha yüzeyi, kayma açısı 90°). Değerler öneridir; kullanıcı
/// sonra değiştirebilir (docs/arayuz.md).
fn slab_section(ui: &mut egui::Ui, lang: Lang, q: &mut Quake, slab: &Slab2) {
    let (lon, lat) = q.epicenter;
    // Odak derinliğine en yakın levha; ortamdan bağımsız (levha içinde odağın
    // levhanın üstünde kaldığını da göstermek için).
    let Some(hit) = slab.locate(lon, lat, q.depth_km, Setting::Interface) else {
        // Uyarı (gri not değil): levhasız yerde arayüz / levha içi depremi
        // olası değil. Engellenmez: Slab2 her zonu kapsamaz (ör. Kıbrıs,
        // Cebelitarık yayları).
        ui.label(
            RichText::new(format!("⚠ {}", lang.t("slab-none")))
                .small()
                .color(WARN_COLOR),
        );
        return;
    };
    let code = slab.code(hit.zone);
    let g = hit.geometry;
    let region = slab::parker_region(code, lon, lat);
    ui.label(
        RichText::new(lang.t_args(
            "slab-info",
            &[
                ("zone", lang.t(&format!("slab-zone-{code}"))),
                ("d", lang.num(g.depth_km, 0)),
                ("strike", lang.num(g.strike, 0)),
                ("dip", lang.num(g.dip, 0)),
            ],
        ))
        .small(),
    );
    ui.label(
        RichText::new(lang.t_args(
            "slab-region",
            &[("region", lang.t(&format!("parker-region-{}", region.key())))],
        ))
        .small()
        .weak(),
    );
    match q.setting {
        Setting::Interface => {
            let (lo, hi) = limits::INTERFACE_DEPTH;
            if (lo..=hi).contains(&g.depth_km) {
                if ui.button(lang.t("slab-apply")).clicked() {
                    q.strike = (g.strike.round()).rem_euclid(360.0);
                    q.dip = g.dip.round().max(1.0);
                    q.depth_km = g.depth_km.round().clamp(lo, hi);
                    q.rake = 90.0;
                    q.fault = FaultType::Reverse;
                }
            } else {
                ui.label(
                    RichText::new(format!(
                        "⚠ {}",
                        lang.t_args(
                            "slab-too-deep",
                            &[("d", lang.num(g.depth_km, 0)), ("max", lang.num(hi, 0))]
                        )
                    ))
                    .small()
                    .color(WARN_COLOR),
                );
            }
        }
        Setting::Intraslab if q.depth_km < g.depth_km - 10.0 => {
            ui.label(
                RichText::new(format!(
                    "⚠ {}",
                    lang.t_args("slab-above", &[("d", lang.num(g.depth_km, 0))])
                ))
                .small()
                .color(WARN_COLOR),
            );
        }
        _ => {}
    }
}

#[allow(clippy::too_many_arguments)]
pub fn ui(
    ui: &mut egui::Ui,
    lang: Lang,
    draft: &mut Draft,
    presets: &[crate::scenarios::Preset],
    slab: Option<&Slab2>,
    issues: &[Issue],
    can_run: bool,
    running: bool,
) -> Action {
    let t = |k| lang.t(k);
    let mut action = Action::None;

    // Hazır senaryolar (Bölüm 14 doğrulama depremleri).
    if !presets.is_empty() {
        let selected = draft
            .preset
            .as_ref()
            .and_then(|id| presets.iter().find(|p| &p.id == id));
        let label = selected.map_or_else(|| t("scenario-custom"), |p| lang.t(&p.name_key()));
        let mut chosen = None;
        ui.horizontal(|ui| {
            let label_id = ui.label(t("scenario")).id;
            egui::ComboBox::from_id_salt("scenario")
                .selected_text(label)
                .width(ui.available_width())
                .show_ui(ui, |ui| {
                    for p in presets {
                        let on = selected.is_some_and(|s| s.id == p.id);
                        if ui.selectable_label(on, lang.t(&p.name_key())).clicked() {
                            chosen = Some(p);
                        }
                    }
                })
                .response
                .labelled_by(label_id);
        });
        if let Some(p) = chosen {
            draft.quake = p.quake;
            draft.has_epicenter = true;
            draft.preset = Some(p.id.clone());
            action = Action::Preset;
        }
        if draft.preset.is_some() {
            ui.label(RichText::new(t("scenario-note")).small().weak());
        }
        ui.add_space(4.0);
    }

    let before = draft.quake;
    let q = &mut draft.quake;

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

    if q.setting != Setting::ShallowCrust && draft.has_epicenter {
        ui.add_space(4.0);
        match slab {
            Some(slab) => slab_section(ui, lang, q, slab),
            None => {
                ui.label(
                    RichText::new(format!("ⓘ {}", t("slab-missing")))
                        .small()
                        .weak(),
                );
            }
        }
    }

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
                    // Hesap fay tipini kullanır; açı değişince tip ona uyar
                    // (CY14 tanımı), eğim olduğu gibi kalır.
                    if ui
                        .add(number(lang, &mut q.rake, -180.0..=180.0, 0, 1.0))
                        .changed()
                    {
                        q.fault = FaultType::from_rake(q.rake);
                    }
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
                                // Yalnızca kullanıcı oynatınca yuvarla: varsayılan
                                // (Always) mevcut değeri her karede tam sayıya
                                // yazıp "değişti" sayıyordu; hazır senaryo
                                // (ör. %44,9) "Özel"e düşüyordu (2026-11-23).
                                .clamping(egui::SliderClamping::Edits)
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
                                // Yalnızca kullanıcı oynatınca yuvarla: varsayılan
                                // (Always) mevcut değeri her karede tam sayıya
                                // yazıp "değişti" sayıyordu; hazır senaryo
                                // (ör. %44,9) "Özel"e düşüyordu (2026-11-23).
                                .clamping(egui::SliderClamping::Edits)
                                .suffix(" %"),
                        )
                        .changed()
                    {
                        q.hypo_down = pct / 100.0;
                    }
                    ui.end_row();
                    // Kırılma boyutu: otomatik (WC94) ya da elle. Elle'ye
                    // geçince o anki otomatik değerlerle başlar.
                    ui.label(t("rupture-manual"));
                    let mut manual = q.rupture_km.is_some();
                    if ui.checkbox(&mut manual, "").changed() {
                        q.rupture_km = manual.then(|| {
                            let auto = Fault::from_quake(&Quake {
                                rupture_km: None,
                                ..*q
                            });
                            [auto.length.round(), auto.width.round()]
                        });
                    }
                    ui.end_row();
                    if let Some([l, w]) = &mut q.rupture_km {
                        let (max_l, max_w) = seismira_core::quake::limits::RUPTURE_MAX_KM;
                        ui.label(t("rupture-length"));
                        ui.add(number(lang, l, 1.0..=max_l, 0, 1.0).suffix(" km"));
                        ui.end_row();
                        ui.label(t("rupture-width"));
                        ui.add(number(lang, w, 1.0..=max_w, 0, 0.5).suffix(" km"));
                        ui.end_row();
                    }
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
        draft.preset = None;
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
    if !has_gmpe(&draft.quake) {
        ui.label(RichText::new(format!("ⓘ {}", t("no-gmpe"))).color(WARN_COLOR));
    }
    if !draft.has_epicenter {
        ui.label(RichText::new(t("start-needs-epicenter")).weak());
    }

    ui.add_space(8.0);
    let start = egui::Button::new(RichText::new(t("button-start")).strong())
        .min_size(egui::vec2(140.0, 34.0));
    ui.horizontal(|ui| {
        if ui.add_enabled(can_run, start).clicked() {
            action = Action::Start;
        }
        if running && ui.button(t("button-stop")).clicked() {
            action = Action::Stop;
        }
    });
    if running {
        ui.label(RichText::new(t("running-hint")).small().weak());
    }
    ui.add_space(8.0);
    ui.label(RichText::new(t("ranges-note")).small().weak());
    action
}

#[cfg(test)]
mod preset_tests {
    use super::*;
    use egui_kittest::kittest::Queryable;

    /// Hazır senaryo seçildikten sonraki karelerde hiçbir widget depremi
    /// kendiliğinden değiştirmemeli (aksi hâlde seçim "Özel"e düşer).
    #[test]
    fn senaryo_secimi_kareler_boyunca_korunur() {
        let path = std::path::Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/scenarios/scenarios.json"
        ));
        let presets = crate::scenarios::load(path).unwrap();
        for p in &presets {
            let draft = Draft {
                quake: p.quake,
                has_epicenter: true,
                preset: Some(p.id.clone()),
            };
            let mut h = egui_kittest::Harness::builder()
                .with_accessibility_check(false)
                .with_context_setup(|ctx| {
                    crate::fonts::install(ctx, &crate::paths::assets_dir(), Lang::Tr);
                })
                .build_ui_state(
                    |ui, draft: &mut Draft| {
                        let issues = seismira_core::quake::validate(&draft.quake);
                        super::ui(ui, Lang::Tr, draft, &presets, None, &issues, true, false);
                    },
                    draft,
                );
            h.run();
            h.get_by_label("İleri seviye").click();
            for _ in 0..3 {
                h.step();
            }
            let draft = h.state();
            assert_eq!(draft.quake, p.quake, "{}: deprem değişti", p.id);
            assert_eq!(draft.preset.as_deref(), Some(p.id.as_str()), "{}", p.id);
        }
    }

    /// Gerçek tıklama: liste açılır, senaryo seçilir, birkaç kare sonra
    /// seçim ve değerler korunur.
    #[test]
    fn senaryo_tiklamayla_secilir() {
        let path = std::path::Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/scenarios/scenarios.json"
        ));
        let presets = crate::scenarios::load(path).unwrap();
        let mut h = egui_kittest::Harness::builder()
            .with_accessibility_check(false)
            .with_size([420.0, 900.0])
            .with_context_setup(|ctx| {
                crate::fonts::install(ctx, &crate::paths::assets_dir(), Lang::Tr);
            })
            .build_ui_state(
                |ui, draft: &mut Draft| {
                    let issues = seismira_core::quake::validate(&draft.quake);
                    super::ui(ui, Lang::Tr, draft, &presets, None, &issues, true, false);
                },
                Draft::default(),
            );
        h.run();
        h.get_by_label("İleri seviye").click();
        h.run();
        h.get_by_label("Hazır senaryo").click();
        h.run();
        h.get_by_label("2010 Maule (Şili), Mw 8,8").click();
        for _ in 0..5 {
            h.step();
        }
        let maule = presets.iter().find(|p| p.id == "maule2010").unwrap();
        let d = h.state();
        assert_eq!(
            d.preset.as_deref(),
            Some("maule2010"),
            "seçim düştü: {:?}",
            d.quake
        );
        assert_eq!(d.quake, maule.quake);
    }
}
