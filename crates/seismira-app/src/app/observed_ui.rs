//! "Gözlemlerle karşılaştır" (README "Gerçek depremler için devamı"): USGS'den
//! yüklenen gerçek depremde ShakeMap istasyonlarının gözlenen PGV'si ile bu
//! simülasyonun tahmini ([`seismira_core::observed`]). Hesap arka planda,
//! simülasyon her başladığında (ayarlar değişince yeniden).
//!
//! Haritada işaretler: |ΔI| < 0,5 içi boş daire (uyumlu); ΔI ≥ 0,5 yukarı
//! üçgen (gözlem daha güçlü, model az tahmin); ΔI ≤ −0,5 aşağı üçgen (gözlem
//! daha zayıf). Biçim ve renk birlikte (renk tek bilgi taşıyıcı değil,
//! CLAUDE.md Bölüm 3).

use super::SeismiraApp;
use eframe::egui::{self, Color32, RichText, Stroke, vec2};
use seismira_core::observed::{self, Residual, Station, Summary};
use seismira_core::quake::Quake;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, TryRecvError};

/// Yüklenen deprem ile simülasyondaki deprem bu mesafeden uzaksa
/// karşılaştırma yapılmaz (kullanıcı başka bir yere deprem koymuş).
const SAME_EVENT_KM: f64 = 50.0;
/// İşaret eşiği (JMA birimi).
const MARK_DELTA: f64 = 0.5;

const UNDER: Color32 = Color32::from_rgb(235, 105, 80);
const OVER: Color32 = Color32::from_rgb(90, 160, 235);
const NEUTRAL: Color32 = Color32::from_rgb(225, 225, 225);

struct ObsData {
    epicenter: (f64, f64),
    place: String,
    stations: Arc<Vec<Station>>,
}

type Done = (Quake, Vec<Residual>);

pub(super) struct ObservedState {
    pub show: bool,
    on_map: bool,
    data: Option<ObsData>,
    job: Option<(Quake, Receiver<Done>)>,
    result: Option<(Quake, Vec<Residual>, Option<Summary>)>,
}

impl Default for ObservedState {
    fn default() -> Self {
        Self {
            show: false,
            on_map: true,
            data: None,
            job: None,
            result: None,
        }
    }
}

impl ObservedState {
    /// Yeni gerçek deprem yüklendi: istasyonları (boşsa yok) kaydeder.
    pub fn set(&mut self, epicenter: (f64, f64), place: &str, stations: Vec<Station>) {
        self.job = None;
        self.result = None;
        self.data = (!stations.is_empty()).then(|| ObsData {
            epicenter,
            place: place.to_string(),
            stations: Arc::new(stations),
        });
    }
}

impl SeismiraApp {
    /// Simülasyondaki deprem yüklenen gerçek depremle aynı mı (merkez üssü
    /// [`SAME_EVENT_KM`] içinde)?
    fn observed_matches(&self, q: &Quake) -> bool {
        self.observed.data.as_ref().is_some_and(|d| {
            seismira_core::geo::distance_km(d.epicenter, q.epicenter) <= SAME_EVENT_KM
        })
    }

    /// Her kare: gerekirse arka plan hesabını başlatır, sonucu alır.
    pub(super) fn observed_update(&mut self, ctx: &egui::Context) {
        if let Some((q, rx)) = &self.observed.job {
            match rx.try_recv() {
                Ok((q2, rows)) => {
                    let s = observed::summarize(&rows);
                    self.observed.result = Some((q2, rows, s));
                    self.observed.job = None;
                }
                Err(TryRecvError::Empty) => {
                    // Simülasyon değiştiyse eski iş bırakılır.
                    if self.sim.as_ref().map(|s| s.quake) != Some(*q) {
                        self.observed.job = None;
                    } else {
                        ctx.request_repaint_after(std::time::Duration::from_millis(150));
                    }
                }
                Err(_) => self.observed.job = None,
            }
        }
        let Some(q) = self.sim.as_ref().map(|s| s.quake) else {
            return;
        };
        let fresh = self.observed.result.as_ref().is_some_and(|r| r.0 == q)
            || self.observed.job.as_ref().is_some_and(|j| j.0 == q);
        if fresh || !self.observed_matches(&q) {
            return;
        }
        let (Some(d), Ok(tt)) = (&self.observed.data, &self.travel_times) else {
            return;
        };
        let stations = d.stations.clone();
        let tt = tt.clone();
        let (slab, vs30, user, regions) = (
            self.slab.clone(),
            self.vs30.clone(),
            self.user.vs30.clone(),
            self.regions.clone(),
        );
        let (tx, rx) = std::sync::mpsc::channel();
        let c = ctx.clone();
        let _ = std::thread::Builder::new()
            .name("gözlem-karşılaştırma".into())
            .spawn(move || {
                // Uygulamanın hesabıyla aynı bölge ve zemin (compute.rs).
                let country = regions.as_deref().and_then(|r| {
                    seismira_core::region::country_near(&r.map, &r.infos, q.epicenter, 100.0)
                });
                let region = country.as_deref().map_or(
                    seismira_core::gmpe::CrustRegion::Global,
                    seismira_core::gmpe::CrustRegion::from_iso,
                );
                let sc = seismira_core::shaking::Scenario::with_slab(q, &tt, slab.as_deref())
                    .with_crust_region(region);
                let rows = observed::compare(&sc, &stations, |(lon, lat)| {
                    use seismira_core::site::{Site, Vs30Source};
                    if let Some(v) = user.as_ref().and_then(|u| u.sample(lon, lat)) {
                        return Site {
                            vs30: v,
                            vs30_ln_sd: None,
                            source: Vs30Source::User,
                        };
                    }
                    vs30.as_ref()
                        .and_then(|m| m.site(lon, lat).ok())
                        .unwrap_or_else(Site::reference)
                });
                let _ = tx.send((q, rows));
                c.request_repaint();
            });
        self.observed.job = Some((q, rx));
    }

    /// Geçerli simülasyonun sonucu (varsa).
    fn observed_current(&self) -> Option<(&[Residual], Option<&Summary>)> {
        let q = self.sim.as_ref()?.quake;
        let (rq, rows, s) = self.observed.result.as_ref()?;
        (*rq == q).then_some((rows.as_slice(), s.as_ref()))
    }

    pub(super) fn observed_window(&mut self, ctx: &egui::Context) {
        let lang = self.lang;
        let t = |k: &str| lang.t(k);
        let mut open = self.observed.show;
        let mut on_map = self.observed.on_map;
        egui::Window::new(t("obs-title"))
            .open(&mut open)
            .default_pos([340.0, 110.0])
            .default_width(460.0)
            .show(ctx, |ui| {
                let Some(d) = &self.observed.data else {
                    ui.label(t("obs-none"));
                    return;
                };
                ui.label(lang.t_args(
                    "obs-loaded",
                    &[
                        ("n", lang.num(d.stations.len() as f64, 0)),
                        ("place", d.place.clone()),
                    ],
                ));
                let sim_q = self.sim.as_ref().map(|s| s.quake);
                match sim_q {
                    None => {
                        ui.label(RichText::new(t("obs-start")).weak());
                    }
                    Some(q) if !self.observed_matches(&q) => {
                        ui.colored_label(crate::panel::WARN_COLOR, t("obs-other-quake"));
                    }
                    Some(_) => match self.observed_current() {
                        None => {
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.label(t("obs-computing"));
                            });
                        }
                        Some((_, None)) => {
                            ui.label(t("obs-no-overlap"));
                        }
                        Some((rows, Some(s))) => {
                            summary_ui(ui, lang, s);
                            ui.checkbox(&mut on_map, t("obs-on-map"));
                            legend_ui(ui, lang);
                            egui::CollapsingHeader::new(t("obs-stations"))
                                .default_open(false)
                                .show(ui, |ui| stations_ui(ui, lang, rows));
                        }
                    },
                }
                ui.add_space(4.0);
                ui.label(RichText::new(t("obs-method")).small().weak());
                ui.label(RichText::new(t("real-credit-usgs")).small().weak());
            });
        self.observed.show = open;
        self.observed.on_map = on_map;
    }

    /// Haritada istasyon işaretleri ve fare üstündeki istasyonun etiketi.
    /// Fare üstündeki işaretin etiketi (metin, nokta) döner; etiket en son
    /// çizilir ([`SeismiraApp::paint_map_tip`]) ki kutuların üstünde kalsın.
    pub(super) fn draw_observed(
        &self,
        ui: &egui::Ui,
        rect: egui::Rect,
    ) -> Option<(String, egui::Pos2)> {
        if !self.observed.on_map {
            return None;
        }
        let (rows, _) = self.observed_current()?;
        let painter = ui.painter_at(rect);
        let hover = ui.input(|i| i.pointer.hover_pos());
        let mut hovered: Option<(&Residual, egui::Pos2)> = None;
        for r in rows {
            let c = self.map.to_screen(rect, r.lonlat);
            if !rect.expand(10.0).contains(c) {
                continue;
            }
            let d = r.delta();
            let size = (4.0 + 2.0 * d.abs() as f32).min(9.0);
            let edge = Stroke::new(1.0, Color32::BLACK);
            if d >= MARK_DELTA {
                painter.add(egui::Shape::convex_polygon(
                    vec![
                        c + vec2(0.0, -size),
                        c + vec2(size, size * 0.8),
                        c + vec2(-size, size * 0.8),
                    ],
                    UNDER,
                    edge,
                ));
            } else if d <= -MARK_DELTA {
                painter.add(egui::Shape::convex_polygon(
                    vec![
                        c + vec2(-size, -size * 0.8),
                        c + vec2(size, -size * 0.8),
                        c + vec2(0.0, size),
                    ],
                    OVER,
                    edge,
                ));
            } else {
                painter.circle_stroke(c, 3.5, Stroke::new(1.5, NEUTRAL));
            }
            if hover.is_some_and(|h| h.distance(c) < 7.0) {
                hovered = Some((r, c));
            }
        }
        if let Some((r, c)) = hovered {
            let lang = self.lang;
            let text = lang.t_args(
                "obs-tip",
                &[
                    ("code", r.code.clone()),
                    ("d", signed(lang, r.delta())),
                    ("obs", lang.num(r.obs_pgv, 1)),
                    ("pred", lang.num(r.pred_pgv, 1)),
                    ("r", lang.num(r.rrup, 0)),
                ],
            );
            return Some((text, c));
        }
        None
    }
}

fn signed(lang: crate::i18n::Lang, v: f64) -> String {
    let s = lang.num(v.abs(), 2);
    if v >= 0.005 {
        format!("+{s}")
    } else if v <= -0.005 {
        format!("−{s}")
    } else {
        s
    }
}

fn summary_ui(ui: &mut egui::Ui, lang: crate::i18n::Lang, s: &Summary) {
    let t = |k: &str| lang.t(k);
    egui::Grid::new("obs-summary")
        .num_columns(2)
        .spacing([12.0, 4.0])
        .show(ui, |ui| {
            ui.label(t("obs-n"));
            ui.label(lang.num(s.n as f64, 0));
            ui.end_row();
            ui.label(t("obs-mean"));
            ui.label(RichText::new(signed(lang, s.mean_delta)).strong());
            ui.end_row();
            ui.label(t("obs-within"));
            ui.label(lang.t_args("obs-pct", &[("p", lang.num(s.within_one * 100.0, 0))]));
            ui.end_row();
            ui.label(t("obs-std"));
            ui.label(lang.num(s.std_ln, 2));
            ui.end_row();
        });
    ui.label(
        RichText::new(t(if s.mean_delta > 0.3 {
            "obs-verdict-under"
        } else if s.mean_delta < -0.3 {
            "obs-verdict-over"
        } else {
            "obs-verdict-ok"
        }))
        .small(),
    );
    ui.add_space(4.0);
    egui::Grid::new("obs-bins")
        .num_columns(3)
        .spacing([12.0, 2.0])
        .striped(true)
        .show(ui, |ui| {
            ui.label(RichText::new(t("obs-bin-dist")).weak());
            ui.label(RichText::new(t("obs-bin-n")).weak());
            ui.label(RichText::new(t("obs-bin-mean")).weak());
            ui.end_row();
            for b in s.bins.iter().filter(|b| b.n > 0) {
                ui.label(format!(
                    "{}–{} km",
                    lang.num(b.lo_km, 0),
                    lang.num(b.hi_km, 0)
                ));
                ui.label(lang.num(b.n as f64, 0));
                ui.label(b.mean_delta.map_or("–".into(), |m| signed(lang, m)));
                ui.end_row();
            }
        });
}

fn legend_ui(ui: &mut egui::Ui, lang: crate::i18n::Lang) {
    ui.horizontal_wrapped(|ui| {
        for (c, key) in [
            (UNDER, "obs-legend-under"),
            (NEUTRAL, "obs-legend-ok"),
            (OVER, "obs-legend-over"),
        ] {
            let sym = match key {
                "obs-legend-under" => "▲",
                "obs-legend-over" => "▼",
                _ => "○",
            };
            ui.label(RichText::new(sym).color(c));
            ui.label(RichText::new(lang.t(key)).small());
        }
    });
}

fn stations_ui(ui: &mut egui::Ui, lang: crate::i18n::Lang, rows: &[Residual]) {
    let mut idx: Vec<usize> = (0..rows.len()).collect();
    idx.sort_by(|&a, &b| rows[a].rrup.total_cmp(&rows[b].rrup));
    egui::ScrollArea::vertical()
        .max_height(240.0)
        .show(ui, |ui| {
            egui::Grid::new("obs-list")
                .num_columns(5)
                .spacing([10.0, 2.0])
                .striped(true)
                .show(ui, |ui| {
                    for k in [
                        "obs-col-code",
                        "obs-col-r",
                        "obs-col-obs",
                        "obs-col-pred",
                        "obs-col-d",
                    ] {
                        ui.label(RichText::new(lang.t(k)).weak().small());
                    }
                    ui.end_row();
                    for &i in &idx {
                        let r = &rows[i];
                        ui.label(RichText::new(&r.code).small());
                        ui.label(RichText::new(lang.num(r.rrup, 0)).small());
                        ui.label(RichText::new(lang.num(r.obs_pgv, 1)).small());
                        ui.label(RichText::new(lang.num(r.pred_pgv, 1)).small());
                        ui.label(RichText::new(signed(lang, r.delta())).small());
                        ui.end_row();
                    }
                });
        });
}
