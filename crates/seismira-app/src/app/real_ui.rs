//! "Gerçek depremler" penceresi ([`crate::realquakes`]): son 30 günün
//! depremleri listelenir, seçilen senaryoya aktarılır.
//!
//! Ortam tahmini (VARSAYIM, docs/arayuz.md): Slab2 levhası varsa odak levha
//! yüzeyine ≤ 20 km ve ≤ 40 km derinde → arayüz (levha doğrultu / eğimi,
//! ters fay); levhanın içinde ve > 40 km → levha içi; aksi halde ≤ 35 km
//! sığ kabuk, daha derin levha içi. Kullanıcı panelden değiştirebilir.

use super::SeismiraApp;
use crate::realquakes::{self, NodalPlane, RealQuake, Source};
use eframe::egui::{self, RichText};
use seismira_core::quake::{FaultType, Quake, Setting, limits};
use std::sync::mpsc::{Receiver, TryRecvError};

type Fetch = Result<Vec<RealQuake>, String>;
type Apply = (RealQuake, Result<Option<NodalPlane>, String>);

pub(super) struct RealState {
    pub show: bool,
    source: Source,
    min_mag: f64,
    loading: Option<Receiver<Fetch>>,
    list: Vec<RealQuake>,
    error: Option<String>,
    selected: Option<usize>,
    applying: Option<Receiver<Apply>>,
    msg: Option<String>,
}

impl Default for RealState {
    fn default() -> Self {
        Self {
            show: false,
            source: Source::Usgs,
            min_mag: 5.5,
            loading: None,
            list: Vec::new(),
            error: None,
            selected: None,
            applying: None,
            msg: None,
        }
    }
}

impl SeismiraApp {
    pub(super) fn real_window(&mut self, ctx: &egui::Context) {
        let lang = self.lang;
        let t = |k: &str| lang.t(k);
        // Arka plan sonuçları.
        if let Some(rx) = &self.real.loading {
            match rx.try_recv() {
                Ok(r) => {
                    self.real.loading = None;
                    match r {
                        Ok(l) => {
                            self.real.list = l;
                            self.real.error = None;
                        }
                        Err(e) => self.real.error = Some(e),
                    }
                    self.real.selected = None;
                }
                Err(TryRecvError::Empty) => {
                    ctx.request_repaint_after(std::time::Duration::from_millis(150));
                }
                Err(_) => self.real.loading = None,
            }
        }
        if let Some(rx) = &self.real.applying {
            match rx.try_recv() {
                Ok((q, np)) => {
                    self.real.applying = None;
                    self.real_apply(&q, np);
                }
                Err(TryRecvError::Empty) => {
                    ctx.request_repaint_after(std::time::Duration::from_millis(150));
                }
                Err(_) => self.real.applying = None,
            }
        }
        let mut open = self.real.show;
        let mut fetch = false;
        let mut apply = None;
        egui::Window::new(t("real-title"))
            .open(&mut open)
            .default_pos([340.0, 90.0])
            .default_width(520.0)
            .show(ctx, |ui| {
                let st = &mut self.real;
                ui.label(RichText::new(t("real-intro")).small());
                ui.horizontal_wrapped(|ui| {
                    let l = ui.label(t("real-source")).id;
                    for s in Source::ALL {
                        ui.selectable_value(&mut st.source, s, s.label())
                            .labelled_by(l);
                    }
                    ui.separator();
                    let l = ui.label(t("real-minmag")).id;
                    egui::ComboBox::from_id_salt("real-minmag")
                        .selected_text(format!("M ≥ {}", lang.num(st.min_mag, 1)))
                        .show_ui(ui, |ui| {
                            for m in [4.5, 5.0, 5.5, 6.0, 6.5, 7.0] {
                                ui.selectable_value(
                                    &mut st.min_mag,
                                    m,
                                    format!("M ≥ {}", lang.num(m, 1)),
                                );
                            }
                        })
                        .response
                        .labelled_by(l);
                    let busy = st.loading.is_some();
                    if ui
                        .add_enabled(!busy, egui::Button::new(t("real-fetch")))
                        .clicked()
                    {
                        fetch = true;
                    }
                    if busy {
                        ui.spinner();
                    }
                });
                if let Some(e) = &st.error {
                    ui.colored_label(
                        crate::panel::WARN_COLOR,
                        lang.t_args("real-error", &[("e", e.clone())]),
                    );
                }
                if !st.list.is_empty() {
                    ui.label(
                        RichText::new(
                            lang.t_args("real-count", &[("n", lang.num(st.list.len() as f64, 0))]),
                        )
                        .small()
                        .weak(),
                    );
                    egui::ScrollArea::vertical()
                        .max_height(320.0)
                        .show(ui, |ui| {
                            for (i, q) in st.list.iter().enumerate() {
                                let text = format!(
                                    "{} · M{} {} · {} · {} km",
                                    realquakes::format_utc(q.time_ms),
                                    lang.num(q.mag, 1),
                                    q.mag_type,
                                    q.place,
                                    lang.num(q.depth_km, 0)
                                );
                                if ui
                                    .selectable_label(
                                        st.selected == Some(i),
                                        RichText::new(text).small(),
                                    )
                                    .clicked()
                                {
                                    st.selected = Some(i);
                                }
                            }
                        });
                    ui.horizontal(|ui| {
                        let busy = st.applying.is_some();
                        if ui
                            .add_enabled(
                                st.selected.is_some() && !busy,
                                egui::Button::new(t("real-apply")),
                            )
                            .clicked()
                        {
                            apply = st.selected.map(|i| st.list[i].clone());
                        }
                        if busy {
                            ui.spinner();
                        }
                    });
                    ui.label(RichText::new(t(st.source_credit())).small().weak());
                }
                if let Some(m) = &st.msg {
                    ui.label(RichText::new(m).small());
                }
                ui.label(RichText::new(t("real-disclaimer")).small().weak());
            });
        self.real.show = open;
        if fetch {
            let (tx, rx) = std::sync::mpsc::channel();
            let (src, min, c) = (self.real.source, self.real.min_mag, ctx.clone());
            let _ = std::thread::Builder::new()
                .name("gerçek-depremler".into())
                .spawn(move || {
                    let _ = tx.send(realquakes::fetch(src, min));
                    c.request_repaint();
                });
            self.real.loading = Some(rx);
            self.real.msg = None;
        }
        if let Some(q) = apply {
            // USGS: moment tensörü ayrıntıdan (arka planda); EMSC: yok.
            let (tx, rx) = std::sync::mpsc::channel();
            let c = ctx.clone();
            let _ = std::thread::Builder::new()
                .name("moment-tensörü".into())
                .spawn(move || {
                    let np = match &q.detail {
                        Some(u) => realquakes::fetch_nodal_plane(u),
                        None => Ok(None),
                    };
                    let _ = tx.send((q, np));
                    c.request_repaint();
                });
            self.real.applying = Some(rx);
        }
    }

    /// Seçilen depremi taslağa aktarır.
    fn real_apply(&mut self, rq: &RealQuake, np: Result<Option<NodalPlane>, String>) {
        let lang = self.lang;
        let mut q = Quake::new((rq.lon, rq.lat));
        q.mw = rq.mag;
        q.depth_km = rq.depth_km.max(1.0);
        // Ortam (modül notu).
        let hit = self
            .slab
            .as_deref()
            .and_then(|s| s.locate(rq.lon, rq.lat, q.depth_km, Setting::Interface));
        q.setting = match hit {
            Some(h)
                if (h.geometry.depth_km - q.depth_km).abs() <= 20.0
                    && q.depth_km <= limits::INTERFACE_DEPTH.1 =>
            {
                Setting::Interface
            }
            Some(h) if q.depth_km > 40.0 && q.depth_km >= h.geometry.depth_km - 10.0 => {
                Setting::Intraslab
            }
            _ if q.depth_km <= limits::CRUST_DEPTH.1 => Setting::ShallowCrust,
            _ => Setting::Intraslab,
        };
        let np = np.ok().flatten();
        match (q.setting, np, hit) {
            // Arayüz: levha geometrisi, ters fay (CY / Parker arayüzü).
            (Setting::Interface, _, Some(h)) => {
                q.strike = h.geometry.strike.round().rem_euclid(360.0);
                q.dip = h.geometry.dip.round().max(1.0);
                q.rake = 90.0;
                q.fault = FaultType::Reverse;
            }
            (_, Some(p), _) => {
                q.strike = p.strike.rem_euclid(360.0);
                q.dip = p.dip.clamp(1.0, 90.0);
                q.rake = p.rake;
                q.fault = FaultType::from_rake(p.rake);
            }
            _ => {}
        }
        self.cancel_sim();
        self.draft.quake = q;
        self.draft.has_epicenter = true;
        self.draft.preset = None;
        self.draft.drawing = None;
        self.map.center_on((rq.lon, rq.lat));
        let mut msg = lang.t_args(
            "real-applied",
            &[
                ("place", rq.place.clone()),
                (
                    "setting",
                    lang.t(match q.setting {
                        Setting::ShallowCrust => "setting-crust",
                        Setting::Interface => "setting-interface",
                        Setting::Intraslab => "setting-intraslab",
                    }),
                ),
            ],
        );
        if !rq.is_mw() {
            msg.push(' ');
            msg.push_str(&lang.t_args("real-not-mw", &[("t", rq.mag_type.clone())]));
        }
        msg.push(' ');
        msg.push_str(&lang.t(if np.is_some() {
            "real-mt-used"
        } else {
            "real-mt-none"
        }));
        self.real.msg = Some(msg);
    }
}

impl RealState {
    fn source_credit(&self) -> &'static str {
        self.list
            .first()
            .map_or(self.source, |q| q.source)
            .credit_key()
    }
}
