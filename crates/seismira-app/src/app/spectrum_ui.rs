//! Tepki spektrumu penceresi (README "Önemli": tepki spektrumu + TBDY-2018).
//!
//! Simülasyon açıkken haritada sağ tıklanan noktanın %5 sönümlü spektrumu
//! (RotD50; medyan ve ±1σ, [`seismira_core::shaking::Scenario::spectrum`])
//! ve kullanıcı S_S / S_1 girdiyse TBDY-2018 yatay elastik tasarım
//! spektrumu ([`seismira_core::tbdy`]). Hesap küçük (≈ 50 × 4 model
//! değerlendirmesi): arayüz iş parçacığında, yalnızca tıklamada.

use super::SeismiraApp;
use eframe::egui::{self, Color32, Pos2, RichText, Stroke, pos2, vec2};
use seismira_core::shaking::SiteSpectrum;
use seismira_core::tbdy::{DesignSpectrum, SiteClass};
use std::fmt::Write as _;

const MEDIAN: Color32 = Color32::from_rgb(0xFF, 0xB3, 0x47);
const BAND: Color32 = Color32::from_rgba_premultiplied(0x60, 0x43, 0x1A, 0x60);
const DESIGN: Color32 = Color32::from_rgb(0x7F, 0xD8, 0xFF);

pub(super) struct SpectrumState {
    pub show: bool,
    /// Seçili nokta ve hesaplanan spektrum (Vs30 kaynağı metni ile).
    pub point: Option<(f64, f64)>,
    result: Option<(SiteSpectrum, f64, &'static str)>,
    /// TBDY harita katsayıları (kullanıcı girer; 0 = girilmedi).
    ss: f64,
    s1: f64,
    /// Zemin sınıfı elle seçildiyse; `None`: Vs30'dan.
    class: Option<SiteClass>,
    /// Grafiğin üst periyodu (s).
    t_max: f64,
    msg: Option<String>,
}

impl Default for SpectrumState {
    fn default() -> Self {
        Self {
            show: false,
            point: None,
            result: None,
            ss: 0.0,
            s1: 0.0,
            class: None,
            t_max: 4.0,
            msg: None,
        }
    }
}

impl SpectrumState {
    /// Simülasyon kapanınca eski sonuç atılır.
    pub fn clear(&mut self) {
        self.point = None;
        self.result = None;
    }
}

impl SeismiraApp {
    /// Sağ tık: spektrum noktasını seçer ve hesaplar (simülasyon açıkken).
    pub(super) fn spectrum_pick(&mut self, at: (f64, f64)) {
        let (Some(sim), Ok(tt)) = (&self.sim, &self.travel_times) else {
            return;
        };
        let q = sim.quake;
        let country = self.regions.as_deref().and_then(|r| {
            seismira_core::region::country_near(&r.map, &r.infos, q.epicenter, 100.0)
        });
        let region = country.as_deref().map_or(
            seismira_core::gmpe::CrustRegion::Global,
            seismira_core::gmpe::CrustRegion::from_iso,
        );
        let scenario = seismira_core::shaking::Scenario::with_slab(q, tt, self.slab.as_deref())
            .with_crust_region(region);
        // Zemin: kullanıcı Vs30 → USGS haritası → referans.
        let (vs30, src) = match self.user.vs30.as_ref().and_then(|u| u.sample(at.0, at.1)) {
            Some(v) => (v, "spectrum-vs30-user"),
            None => match self.vs30.as_ref().and_then(|m| m.site(at.0, at.1).ok()) {
                Some(s) if s.source != seismira_core::site::Vs30Source::Reference => {
                    (s.vs30, "spectrum-vs30-map")
                }
                _ => (
                    seismira_core::site::REFERENCE_VS30,
                    "spectrum-vs30-reference",
                ),
            },
        };
        let sp = scenario.spectrum(at, vs30);
        self.spectrum.point = Some(at);
        self.spectrum.result = Some((sp, vs30, src));
        self.spectrum.msg = None;
        self.spectrum.show = true;
    }

    pub(super) fn spectrum_window(&mut self, ctx: &egui::Context) {
        let lang = self.lang;
        let t = |k: &str| lang.t(k);
        let mut open = self.spectrum.show;
        let mut save = false;
        egui::Window::new(t("spectrum-title"))
            .open(&mut open)
            .default_pos([360.0, 90.0])
            .default_width(520.0)
            .show(ctx, |ui| {
                let st = &mut self.spectrum;
                let Some((sp, vs30_raw, src)) = &st.result else {
                    ui.label(t(if self.sim.is_some() {
                        "spectrum-pick"
                    } else {
                        "spectrum-need-sim"
                    }));
                    return;
                };
                let (lon, lat) = st.point.unwrap_or_default();
                ui.label(
                    RichText::new(lang.t_args(
                        "spectrum-site",
                        &[
                            ("lat", lang.num(lat, 3)),
                            ("lon", lang.num(lon, 3)),
                            ("rrup", lang.num(sp.dist.rrup, 0)),
                            ("vs30", lang.num(*vs30_raw, 0)),
                            ("src", lang.t(src)),
                        ],
                    ))
                    .small(),
                );
                if sp.sa.iter().all(|(_, m)| m.is_none()) {
                    ui.colored_label(crate::panel::WARN_COLOR, t("spectrum-out-of-model"));
                    return;
                }
                // TBDY girdileri.
                ui.add_space(4.0);
                ui.label(RichText::new(t("spectrum-tbdy")).small().weak());
                let auto = SiteClass::from_vs30(*vs30_raw);
                ui.horizontal_wrapped(|ui| {
                    let l = ui.label("S_S").id;
                    ui.add(
                        egui::DragValue::new(&mut st.ss)
                            .range(0.0..=4.0)
                            .speed(0.01)
                            .max_decimals(3),
                    )
                    .labelled_by(l);
                    let l = ui.label("S_1").id;
                    ui.add(
                        egui::DragValue::new(&mut st.s1)
                            .range(0.0..=2.0)
                            .speed(0.01)
                            .max_decimals(3),
                    )
                    .labelled_by(l);
                    let l = ui.label(t("spectrum-class")).id;
                    let cur = st.class.unwrap_or(auto);
                    egui::ComboBox::from_id_salt("tbdy-class")
                        .selected_text(if st.class.is_none() {
                            lang.t_args("spectrum-class-auto", &[("c", auto.label().to_string())])
                        } else {
                            cur.label().to_string()
                        })
                        .show_ui(ui, |ui| {
                            if ui
                                .selectable_label(
                                    st.class.is_none(),
                                    lang.t_args(
                                        "spectrum-class-auto",
                                        &[("c", auto.label().to_string())],
                                    ),
                                )
                                .clicked()
                            {
                                st.class = None;
                            }
                            for c in SiteClass::ALL {
                                if ui
                                    .selectable_label(st.class == Some(c), c.label())
                                    .clicked()
                                {
                                    st.class = Some(c);
                                }
                            }
                        })
                        .response
                        .labelled_by(l);
                });
                let class = st.class.unwrap_or(auto);
                let design = DesignSpectrum::new(st.ss, st.s1, class);
                if class == SiteClass::ZF {
                    ui.label(RichText::new(t("spectrum-zf")).small().weak());
                } else if let Some(d) = &design {
                    ui.label(
                        RichText::new(lang.t_args(
                            "spectrum-design-values",
                            &[
                                ("sds", lang.num(d.sds, 3)),
                                ("sd1", lang.num(d.sd1, 3)),
                                ("ta", lang.num(d.ta, 3)),
                                ("tb", lang.num(d.tb, 3)),
                            ],
                        ))
                        .small(),
                    );
                }
                ui.horizontal(|ui| {
                    let l = ui.label(t("spectrum-tmax")).id;
                    for v in [2.0, 4.0, 6.0] {
                        ui.selectable_value(&mut st.t_max, v, format!("{} s", lang.num(v, 0)))
                            .labelled_by(l);
                    }
                });
                plot(ui, lang, sp, design.as_ref(), st.t_max);
                // Açıklama.
                ui.horizontal_wrapped(|ui| {
                    ui.colored_label(MEDIAN, "━");
                    ui.label(RichText::new(t("spectrum-legend-median")).small());
                    ui.colored_label(BAND.gamma_multiply(3.0), "█");
                    ui.label(RichText::new(t("spectrum-legend-band")).small());
                    if design.is_some() {
                        ui.colored_label(DESIGN, "╍");
                        ui.label(RichText::new(t("spectrum-legend-design")).small());
                    }
                });
                ui.label(RichText::new(t("spectrum-note")).small().weak());
                if ui.button(t("spectrum-save")).clicked() {
                    save = true;
                }
                if let Some(m) = &st.msg {
                    ui.label(RichText::new(m).small());
                }
            });
        self.spectrum.show = open;
        if save {
            self.spectrum_save();
        }
    }

    fn spectrum_save(&mut self) {
        let lang = self.lang;
        let st = &mut self.spectrum;
        let Some((sp, vs30, _)) = &st.result else {
            return;
        };
        let (lon, lat) = st.point.unwrap_or_default();
        let class = st.class.unwrap_or(SiteClass::from_vs30(*vs30));
        let design = DesignSpectrum::new(st.ss, st.s1, class);
        let mut s = format!(
            "# Seismira tepki spektrumu / response spectrum (%5 sönüm, RotD50, g). EĞİTİM SİMÜLASYONU.\n\
             # nokta / site: {lon:.5} E, {lat:.5} N; Rrup {:.1} km; Vs30 {:.0} m/s (hesapta {:.0})\n\
             # TBDY-2018: S_S {} S_1 {} sınıf {}\n\
             period_s,sa_median_g,sa_minus1sigma_g,sa_plus1sigma_g,tbdy_sae_g\n",
            sp.dist.rrup,
            vs30,
            sp.vs30,
            st.ss,
            st.s1,
            class.label()
        );
        let row = |s: &mut String, t: f64, m: Option<seismira_core::gmpe::LnMotion>| {
            let d = design.map_or(String::new(), |d| format!("{:.5}", d.sae(t)));
            match m {
                Some(m) => {
                    let _ = writeln!(
                        s,
                        "{t:.4},{:.5},{:.5},{:.5},{d}",
                        m.median(),
                        (m.mean - m.sigma).exp(),
                        (m.mean + m.sigma).exp()
                    );
                }
                None => {
                    let _ = writeln!(s, "{t:.4},,,,{d}");
                }
            }
        };
        row(&mut s, 0.0, sp.pga);
        for &(t, m) in &sp.sa {
            row(&mut s, t, m);
        }
        let dir = super::user_ui::export_base();
        let path = dir.join(format!("spektrum-{}.csv", super::user_ui::timestamp()));
        st.msg = Some(
            match std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(&path, s)) {
                Ok(()) => lang.t_args("export-done", &[("dir", path.display().to_string())]),
                Err(e) => format!("{}: {e}", path.display()),
            },
        );
    }

    /// Haritada spektrum noktası (elmas).
    pub(super) fn draw_spectrum_marker(&self, ui: &egui::Ui, rect: egui::Rect) {
        if !self.spectrum.show {
            return;
        }
        let Some(p) = self.spectrum.point else {
            return;
        };
        let c = self.map.to_screen(rect, p);
        let r = 7.0;
        let pts = vec![
            c + vec2(0.0, -r),
            c + vec2(r, 0.0),
            c + vec2(0.0, r),
            c + vec2(-r, 0.0),
        ];
        let painter = ui.painter_at(rect);
        painter.add(egui::Shape::convex_polygon(
            pts.clone(),
            MEDIAN,
            Stroke::new(1.5, Color32::BLACK),
        ));
    }
}

/// Doğrusal eksenli spektrum grafiği: ±1σ bandı, medyan, tasarım spektrumu.
fn plot(
    ui: &mut egui::Ui,
    lang: crate::i18n::Lang,
    sp: &SiteSpectrum,
    design: Option<&DesignSpectrum>,
    t_max: f64,
) {
    let w = ui.available_width().max(300.0);
    let (resp, painter) = ui.allocate_painter(vec2(w, 280.0), egui::Sense::hover());
    let outer = resp.rect;
    let area = egui::Rect::from_min_max(outer.min + vec2(46.0, 8.0), outer.max - vec2(10.0, 30.0));
    // Veri: (T, medyan, alt, üst); PGA T = 0'da.
    let mut pts: Vec<(f64, f64, f64, f64)> = Vec::new();
    if let Some(m) = sp.pga {
        pts.push((
            0.0,
            m.median(),
            (m.mean - m.sigma).exp(),
            (m.mean + m.sigma).exp(),
        ));
    }
    for &(t, m) in &sp.sa {
        if t > t_max * 1.0001 {
            break;
        }
        if let Some(m) = m {
            pts.push((
                t,
                m.median(),
                (m.mean - m.sigma).exp(),
                (m.mean + m.sigma).exp(),
            ));
        }
    }
    let mut y_max = pts.iter().map(|p| p.3).fold(0.0, f64::max);
    if let Some(d) = design {
        y_max = y_max.max(d.sds);
    }
    let y_max = nice_ceil(y_max.max(0.01));
    let x = |t: f64| area.left() + (t / t_max) as f32 * area.width();
    let y = |v: f64| area.bottom() - (v / y_max) as f32 * area.height();
    let text = ui.visuals().text_color();
    let grid = ui.visuals().widgets.noninteractive.bg_stroke.color;
    let font = egui::FontId::proportional(11.0);
    // Izgara ve eksen yazıları.
    for k in 0..=5 {
        let v = y_max * f64::from(k) / 5.0;
        let yy = y(v);
        painter.line_segment(
            [pos2(area.left(), yy), pos2(area.right(), yy)],
            Stroke::new(0.5, grid),
        );
        painter.text(
            pos2(area.left() - 4.0, yy),
            egui::Align2::RIGHT_CENTER,
            lang.num(v, 2),
            font.clone(),
            text,
        );
    }
    let step = if t_max <= 2.0 { 0.25 } else { 0.5 };
    let mut tk = 0.0;
    while tk <= t_max + 1e-9 {
        let xx = x(tk);
        painter.line_segment(
            [pos2(xx, area.top()), pos2(xx, area.bottom())],
            Stroke::new(0.5, grid),
        );
        painter.text(
            pos2(xx, area.bottom() + 3.0),
            egui::Align2::CENTER_TOP,
            lang.num(tk, 2),
            font.clone(),
            text,
        );
        tk += step;
    }
    painter.text(
        pos2(area.center().x, outer.bottom() - 2.0),
        egui::Align2::CENTER_BOTTOM,
        lang.t("spectrum-axis-t"),
        font.clone(),
        text,
    );
    painter.text(
        pos2(outer.left() + 2.0, area.top() - 2.0),
        egui::Align2::LEFT_BOTTOM,
        lang.t("spectrum-axis-sa"),
        font,
        text,
    );
    // ±1σ bandı (üçgen şerit; dışbükey olmayabilir).
    if pts.len() >= 2 {
        let mut mesh = egui::Mesh::default();
        for p in &pts {
            mesh.colored_vertex(pos2(x(p.0), y(p.3.min(y_max))), BAND);
            mesh.colored_vertex(pos2(x(p.0), y(p.2)), BAND);
        }
        for i in 0..pts.len() as u32 - 1 {
            let k = 2 * i;
            mesh.add_triangle(k, k + 1, k + 2);
            mesh.add_triangle(k + 1, k + 3, k + 2);
        }
        painter.add(egui::Shape::mesh(mesh));
        let line: Vec<Pos2> = pts.iter().map(|p| pos2(x(p.0), y(p.1))).collect();
        painter.add(egui::Shape::line(line, Stroke::new(2.5, MEDIAN)));
    }
    // Tasarım spektrumu (kesikli).
    if let Some(d) = design {
        let n = 240;
        let line: Vec<Pos2> = (0..=n)
            .map(|i| {
                let t = t_max * f64::from(i) / f64::from(n);
                pos2(x(t), y(d.sae(t).min(y_max)))
            })
            .collect();
        painter.extend(egui::Shape::dashed_line(
            &line,
            Stroke::new(2.0, DESIGN),
            6.0,
            4.0,
        ));
    }
    // İmleç: en yakın periyottaki değer.
    if let Some(h) = resp.hover_pos()
        && area.contains(h)
        && !pts.is_empty()
    {
        let tt = f64::from((h.x - area.left()) / area.width()) * t_max;
        let p = pts
            .iter()
            .min_by(|a, b| (a.0 - tt).abs().total_cmp(&(b.0 - tt).abs()))
            .copied()
            .unwrap_or_default();
        painter.line_segment(
            [pos2(x(p.0), area.top()), pos2(x(p.0), area.bottom())],
            Stroke::new(1.0, text),
        );
        let mut s = lang.t_args(
            "spectrum-hover",
            &[
                ("t", lang.num(p.0, 2)),
                ("m", lang.num(p.1, 3)),
                ("lo", lang.num(p.2, 3)),
                ("hi", lang.num(p.3, 3)),
            ],
        );
        if let Some(d) = design {
            s.push_str(&lang.t_args("spectrum-hover-design", &[("d", lang.num(d.sae(p.0), 3))]));
        }
        let g = painter.layout_no_wrap(s, egui::FontId::proportional(12.0), text);
        let r = egui::Rect::from_min_size(pos2(area.left() + 8.0, area.top() + 6.0), g.size())
            .expand(4.0);
        painter.rect_filled(r, 3.0, crate::theme::MAP_BG.gamma_multiply(0.92));
        painter.galley(r.min + vec2(4.0, 4.0), g, text);
    }
}

/// Eksen üst sınırı için "güzel" yuvarlama (1, 2, 2,5, 5 × 10^k).
fn nice_ceil(v: f64) -> f64 {
    let e = v.log10().floor();
    let base = 10f64.powf(e);
    for m in [1.0, 2.0, 2.5, 5.0, 10.0] {
        if v <= m * base {
            return m * base;
        }
    }
    10.0 * base
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Grafik pencere açmadan çizilir (gerçek senaryo spektrumu + TBDY),
    /// imleç grafiğin üstündeyken de.
    #[test]
    fn grafik_cizilir() {
        let t = seismira_core::traveltime::TravelTimes::load(std::path::Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/traveltime/ak135.ttt"
        )))
        .unwrap();
        let mut q = seismira_core::quake::Quake::new((37.0, 37.2));
        q.mw = 7.8;
        let sc = seismira_core::shaking::Scenario::new(q, &t);
        let sp = sc.spectrum((36.9, 37.1), 300.0);
        let d = DesignSpectrum::new(1.2, 0.35, SiteClass::ZD);
        for t_max in [2.0, 4.0, 6.0] {
            let mut h = egui_kittest::Harness::builder()
                .with_size([520.0, 400.0])
                .with_context_setup(|ctx| {
                    crate::fonts::install(ctx, &crate::paths::assets_dir(), crate::i18n::Lang::Tr);
                })
                .build_ui(|ui| plot(ui, crate::i18n::Lang::Tr, &sp, d.as_ref(), t_max));
            h.run();
            h.hover_at(egui::pos2(260.0, 120.0));
            h.run();
        }
    }

    #[test]
    fn guzel_yuvarlama() {
        assert_eq!(super::nice_ceil(0.83), 1.0);
        assert_eq!(super::nice_ceil(1.3), 2.0);
        assert_eq!(super::nice_ceil(2.2), 2.5);
        assert_eq!(super::nice_ceil(0.031), 0.05);
    }
}
