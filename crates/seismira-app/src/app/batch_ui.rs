//! "Toplu senaryo" penceresi ([`seismira_core::batch`]): sağ paneldeki
//! depremin fayı üzerinde, bir büyüklük aralığında kayan kırılmalar; her
//! hücrede ortalama ve en kötü şiddet haritası, en çok etkilenen şehirler.
//!
//! Hesap arka planda (iptal edilebilir, ilerleme gösterilir); üyelerin
//! ızgaraları artımlı birleştirilir (bellekte yalnızca toplamlar). Izgara
//! adımı en çok 1024 hücre/kenar (üye sayısıyla çarpılan süre için).
//! Sonuç haritada, açık simülasyon yokken sabit ızgara olarak gösterilir.

use super::SeismiraApp;
use crate::mmi_style::{Level, Scale};
use eframe::egui::{self, RichText};
use seismira_core::batch::{self, Member, Spec};
use seismira_core::grid::{Base, Domain, ShakeGrid};
use seismira_core::intensity::Shindo;
use seismira_core::quake::Quake;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError};

/// Üye başına ızgaranın en büyük kenarı (hücre).
const MAX_DIM: u32 = 1024;
/// En çok üye (süre sınırı).
const MAX_MEMBERS: usize = 60;
/// Listelenen şehir sayısı.
const TOP_CITIES: usize = 15;

pub(super) struct BatchResult {
    base: Quake,
    /// Üye ve en yüksek Shindo'su.
    members: Vec<(Member, Option<Shindo>)>,
    skipped: usize,
    mean: Arc<ShakeGrid>,
    max: Arc<ShakeGrid>,
    /// Üyelerin ortalama Mw'si (haritada MMI dönüşümü için).
    pub mw_mean: f64,
    /// (şehir sırası, ortalama JMA, en yüksek JMA, ortalama PGV, en yüksek PGV).
    cities: Vec<(usize, f32, f32, f32, f32)>,
}

struct BatchJob {
    rx: Receiver<Result<BatchResult, String>>,
    cancel: Arc<AtomicBool>,
    done: Arc<AtomicUsize>,
    total: usize,
}

#[derive(Default)]
pub(super) struct BatchState {
    pub show: bool,
    spec: Spec,
    job: Option<BatchJob>,
    pub result: Option<BatchResult>,
    view_max: bool,
    /// Sonuç haritada mı (açık simülasyon yokken).
    pub on_map: bool,
    msg: Option<String>,
}

impl BatchState {
    /// Haritada gösterilecek ızgara (seçili görünüm).
    fn grid(&self) -> Option<Arc<ShakeGrid>> {
        self.result.as_ref().map(|r| {
            if self.view_max {
                r.max.clone()
            } else {
                r.mean.clone()
            }
        })
    }
}

/// Izgarada noktanın hücre sırası (boylam ızgaranın batısına göre açılır).
fn cell_index(g: &ShakeGrid, lon: f64, lat: f64) -> Option<usize> {
    let s = &g.spec;
    let dx = (lon - s.west).rem_euclid(360.0);
    let col = (dx / s.cell).floor();
    let row = ((s.north - lat) / s.cell).floor();
    (col >= 0.0 && row >= 0.0 && col < f64::from(s.width) && row < f64::from(s.height))
        .then(|| row as usize * s.width as usize + col as usize)
}

impl SeismiraApp {
    /// Her kare: arka plan sonucunu alır; haritada gösterimi yönetir.
    pub(super) fn batch_update(&mut self, ctx: &egui::Context) {
        if let Some(job) = &self.batch.job {
            match job.rx.try_recv() {
                Ok(r) => {
                    self.batch.job = None;
                    match r {
                        Ok(res) => {
                            self.batch.result = Some(res);
                            self.batch.msg = None;
                            if self.sim.is_none() {
                                self.batch.on_map = true;
                                self.map.set_grid(self.batch.grid());
                            } else {
                                self.batch.msg = Some(self.lang.t("batch-close-sim"));
                            }
                        }
                        Err(e) => self.batch.msg = Some(e),
                    }
                }
                Err(TryRecvError::Empty) => {
                    ctx.request_repaint_after(std::time::Duration::from_millis(200));
                }
                Err(_) => self.batch.job = None,
            }
        }
        if self.sim.is_some() {
            self.batch.on_map = false;
        }
    }

    /// Harita karesi: toplu sonuç gösteriliyorsa ızgara zamanı ve MMI Mw'si.
    pub(super) fn batch_map_frame(&mut self) {
        if let (true, None, Some(r)) = (self.batch.on_map, &self.sim, &self.batch.result) {
            self.map.set_grid_time(self.show_grid.then_some(1e12));
            self.map.set_intensity_scale(self.scale, r.mw_mean);
        }
    }

    /// Geliştirme: `SEISMIRA_BATCH` ile başlatma.
    pub(super) fn batch_start_dev(&mut self, ctx: &egui::Context) {
        self.batch_start(ctx);
    }

    /// Toplu senaryo hesabı sürüyor mu? (ekran görüntüsü aracı bekler)
    #[cfg_attr(not(feature = "screenshot"), allow(dead_code))]
    pub(super) fn batch_busy(&self) -> bool {
        self.batch.job.is_some()
    }

    fn batch_start(&mut self, ctx: &egui::Context) {
        let Ok(tt) = &self.travel_times else {
            return;
        };
        let base = self.draft.quake;
        let spec = self.batch.spec;
        let mut members = batch::members(&base, &spec);
        let n_all = members.len();
        members.retain(|m| {
            !seismira_core::quake::validate(&m.quake)
                .iter()
                .any(|i| i.blocks())
        });
        let skipped = n_all - members.len();
        if members.is_empty() {
            self.batch.msg = Some(self.lang.t("batch-none"));
            return;
        }
        let total = members.len();
        let cancel = Arc::new(AtomicBool::new(false));
        let done = Arc::new(AtomicUsize::new(0));
        let (tx, rx) = std::sync::mpsc::channel();
        let (tt, slab, vs30, user, regions) = (
            tt.clone(),
            self.slab.clone(),
            self.vs30.clone(),
            self.user.vs30.clone(),
            self.regions.clone(),
        );
        let cities = self.cities.as_ref().ok().map(|l| l.cities.clone());
        let (c, flag, counter) = (ctx.clone(), cancel.clone(), done.clone());
        let _ = std::thread::Builder::new()
            .name("toplu-senaryo".into())
            .spawn(move || {
                let started = std::time::Instant::now();
                let country = regions.as_deref().and_then(|r| {
                    seismira_core::region::country_near(&r.map, &r.infos, base.epicenter, 100.0)
                });
                let region = country.as_deref().map_or(
                    seismira_core::gmpe::CrustRegion::Global,
                    seismira_core::gmpe::CrustRegion::from_iso,
                );
                let grid_base = vs30.as_deref().map_or(Base::GLOBAL_30S, |m| {
                    let g = m.grid();
                    Base {
                        west: g.west,
                        north: g.north,
                        cell: g.cell,
                    }
                });
                // Ortak alan: fayın tamamı en büyük büyüklükle kırılsa.
                let mut whole = base;
                whole.mw = members.iter().map(|m| m.quake.mw).fold(base.mw, f64::max);
                whole.trace = seismira_core::quake::Trace::new(&batch::fault_line(&base));
                whole.rupture_km = None;
                let sc_whole =
                    seismira_core::shaking::Scenario::with_slab(whole, &tt, slab.as_deref());
                let domain = Domain::for_scenario(&sc_whole, grid_base);
                let window = crate::compute::vs30_window(vs30.as_deref(), user.as_deref(), &domain);
                let step = domain.fine_step_for(MAX_DIM);
                let mut acc = batch::Accumulator::default();
                let mut template = None;
                let mut out_members = Vec::with_capacity(members.len());
                for m in members {
                    let sc =
                        seismira_core::shaking::Scenario::with_slab(m.quake, &tt, slab.as_deref())
                            .with_crust_region(region);
                    if sc.gmm.is_none() {
                        out_members.push((m, None));
                        counter.fetch_add(1, Ordering::Relaxed);
                        continue;
                    }
                    let Some(g) =
                        seismira_core::grid::compute(&sc, &domain, step, window.as_deref(), &flag)
                    else {
                        return; // iptal
                    };
                    acc.add(&g);
                    out_members.push((m, g.max_class()));
                    if template.is_none() {
                        template = Some(g);
                    }
                    counter.fetch_add(1, Ordering::Relaxed);
                    c.request_repaint();
                }
                let (Some(t), Some(agg)) = (template, acc.finish()) else {
                    let _ = tx.send(Err("—".into()));
                    return;
                };
                let mean = batch::as_grid(&t, &agg.jma_mean, &agg.pgv_mean);
                let max = batch::as_grid(&t, &agg.jma_max, &agg.pgv_max);
                let mut top: Vec<(usize, f32, f32, f32, f32)> = cities
                    .as_deref()
                    .map(|cs| {
                        cs.iter()
                            .enumerate()
                            .filter_map(|(i, ct)| {
                                let k = cell_index(&mean, ct.lon, ct.lat)?;
                                let j = agg.jma_mean[k];
                                (!j.is_nan()).then(|| {
                                    (i, j, agg.jma_max[k], agg.pgv_mean[k], agg.pgv_max[k])
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                top.sort_by(|a, b| b.1.total_cmp(&a.1));
                top.truncate(TOP_CITIES);
                let mw_mean = out_members.iter().map(|(m, _)| m.quake.mw).sum::<f64>()
                    / out_members.len() as f64;
                log::info!(
                    "toplu senaryo: {} üye, ızgara {}×{}, {} ms",
                    out_members.len(),
                    mean.spec.width,
                    mean.spec.height,
                    started.elapsed().as_millis()
                );
                let _ = tx.send(Ok(BatchResult {
                    base,
                    members: out_members,
                    skipped,
                    mean: Arc::new(mean),
                    max: Arc::new(max),
                    mw_mean,
                    cities: top,
                }));
                c.request_repaint();
            });
        self.batch.job = Some(BatchJob {
            rx,
            cancel,
            done,
            total,
        });
        self.batch.msg = None;
    }

    pub(super) fn batch_window(&mut self, ctx: &egui::Context) {
        let lang = self.lang;
        let t = |k: &str| lang.t(k);
        let scale = self.scale;
        let mut open = self.batch.show;
        let mut start = false;
        let mut show_on_map = false;
        let mut view_changed = false;
        let preview = (self.draft.has_epicenter && self.batch.job.is_none())
            .then(|| batch::members(&self.draft.quake, &self.batch.spec).len());
        let city_names: Vec<String> = match (&self.cities, &self.batch.result) {
            (Ok(l), Some(r)) => r
                .cities
                .iter()
                .map(|c| l.cities[c.0].name.clone())
                .collect(),
            _ => Vec::new(),
        };
        let sim_open = self.sim.is_some();
        egui::Window::new(t("batch-title"))
            .open(&mut open)
            .default_pos([340.0, 110.0])
            .default_width(520.0)
            .show(ctx, |ui| {
                let st = &mut self.batch;
                ui.label(RichText::new(t("batch-intro")).small());
                ui.add_space(4.0);
                let busy = st.job.is_some();
                ui.add_enabled_ui(!busy, |ui| {
                    egui::Grid::new("batch-spec")
                        .num_columns(2)
                        .spacing([10.0, 6.0])
                        .show(ui, |ui| {
                            let l = ui.label(t("batch-mw-min")).id;
                            ui.add(crate::clip::WithMenu(
                                egui::DragValue::new(&mut st.spec.mw_min)
                                    .range(5.0..=9.0)
                                    .speed(0.05)
                                    .fixed_decimals(2),
                                lang,
                            ))
                            .labelled_by(l);
                            ui.end_row();
                            let l = ui.label(t("batch-mw-max")).id;
                            ui.add(crate::clip::WithMenu(
                                egui::DragValue::new(&mut st.spec.mw_max)
                                    .range(5.0..=9.0)
                                    .speed(0.05)
                                    .fixed_decimals(2),
                                lang,
                            ))
                            .labelled_by(l);
                            ui.end_row();
                            let l = ui.label(t("batch-mw-step")).id;
                            egui::ComboBox::from_id_salt("batch-step")
                                .selected_text(lang.num(st.spec.mw_step, 2))
                                .show_ui(ui, |ui| {
                                    for s in [0.1, 0.25, 0.5] {
                                        ui.selectable_value(
                                            &mut st.spec.mw_step,
                                            s,
                                            lang.num(s, 2),
                                        );
                                    }
                                })
                                .response
                                .labelled_by(l);
                            ui.end_row();
                            let l = ui.label(t("batch-positions")).id;
                            egui::ComboBox::from_id_salt("batch-pos")
                                .selected_text(st.spec.positions.to_string())
                                .show_ui(ui, |ui| {
                                    for p in [1usize, 3, 5] {
                                        ui.selectable_value(
                                            &mut st.spec.positions,
                                            p,
                                            p.to_string(),
                                        );
                                    }
                                })
                                .response
                                .labelled_by(l);
                            ui.end_row();
                        });
                    if st.spec.mw_max < st.spec.mw_min {
                        st.spec.mw_max = st.spec.mw_min;
                    }
                });
                match preview {
                    None if !busy => {
                        ui.label(RichText::new(t("start-needs-epicenter")).weak());
                    }
                    Some(n) => {
                        let ok = n > 0 && n <= MAX_MEMBERS;
                        ui.label(
                            RichText::new(lang.t_args(
                                "batch-count",
                                &[
                                    ("n", lang.num(n as f64, 0)),
                                    ("max", MAX_MEMBERS.to_string()),
                                ],
                            ))
                            .small(),
                        );
                        if ui
                            .add_enabled(
                                ok,
                                egui::Button::new(RichText::new(t("batch-run")).strong()),
                            )
                            .clicked()
                        {
                            start = true;
                        }
                    }
                    None => {}
                }
                if let Some(job) = &st.job {
                    let d = job.done.load(Ordering::Relaxed);
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::ProgressBar::new(d as f32 / job.total.max(1) as f32)
                                .text(format!("{d} / {}", job.total))
                                .desired_width(240.0),
                        );
                        if ui.button(t("batch-cancel")).clicked() {
                            job.cancel.store(true, Ordering::Relaxed);
                        }
                    });
                }
                if let Some(m) = &st.msg {
                    ui.colored_label(crate::panel::warn_color(), m);
                }
                let Some(r) = &st.result else {
                    return;
                };
                ui.separator();
                ui.label(
                    RichText::new(lang.t_args(
                        "batch-done",
                        &[
                            ("n", lang.num(r.members.len() as f64, 0)),
                            ("mw", lang.num(r.base.mw, 1)),
                        ],
                    ))
                    .small(),
                );
                if r.skipped > 0 {
                    ui.label(
                        RichText::new(
                            lang.t_args("batch-skipped", &[("n", lang.num(r.skipped as f64, 0))]),
                        )
                        .small()
                        .weak(),
                    );
                }
                ui.horizontal(|ui| {
                    view_changed |= ui
                        .radio_value(&mut st.view_max, false, t("batch-view-mean"))
                        .changed();
                    view_changed |= ui
                        .radio_value(&mut st.view_max, true, t("batch-view-max"))
                        .changed();
                    if !st.on_map
                        && ui
                            .add_enabled(!sim_open, egui::Button::new(t("batch-show")))
                            .clicked()
                    {
                        show_on_map = true;
                    }
                });
                if sim_open {
                    ui.label(RichText::new(t("batch-close-sim")).small().weak());
                }
                // Büyüklük başına: uzunluk, konum, en yüksek şiddet.
                egui::CollapsingHeader::new(t("batch-members"))
                    .default_open(false)
                    .show(ui, |ui| {
                        egui::Grid::new("batch-members-grid")
                            .num_columns(4)
                            .spacing([12.0, 2.0])
                            .striped(true)
                            .show(ui, |ui| {
                                for k in [
                                    "batch-col-mw",
                                    "batch-col-len",
                                    "batch-col-n",
                                    "batch-col-max",
                                ] {
                                    ui.label(RichText::new(t(k)).weak().small());
                                }
                                ui.end_row();
                                let mut i = 0;
                                while i < r.members.len() {
                                    let mw = r.members[i].0.quake.mw;
                                    let group: Vec<&(Member, Option<Shindo>)> = r.members[i..]
                                        .iter()
                                        .take_while(|m| m.0.quake.mw == mw)
                                        .collect();
                                    let best = group.iter().filter_map(|m| m.1).max();
                                    let whole = group[0].0.whole_fault;
                                    ui.label(lang.num(mw, 2));
                                    ui.label(format!(
                                        "{} km{}",
                                        lang.num(group[0].0.length_km, 0),
                                        if whole { " *" } else { "" }
                                    ));
                                    ui.label(group.len().to_string());
                                    ui.label(best.map_or("–", |s| s.label()));
                                    ui.end_row();
                                    i += group.len();
                                }
                            });
                        ui.label(RichText::new(t("batch-whole-note")).small().weak());
                    });
                // En çok etkilenen şehirler (ortalamaya göre).
                ui.label(RichText::new(t("batch-cities")).strong());
                egui::Grid::new("batch-cities-grid")
                    .num_columns(3)
                    .spacing([14.0, 2.0])
                    .striped(true)
                    .show(ui, |ui| {
                        for k in ["batch-col-city", "batch-view-mean", "batch-view-max"] {
                            ui.label(RichText::new(t(k)).weak().small());
                        }
                        ui.end_row();
                        let lv = |j: f32, v: f32| match scale {
                            Scale::Shindo => Level::Shindo(Shindo::from_instrumental(f64::from(j))),
                            Scale::Mmi => Level::Mmi(crate::mmi_style::class(
                                seismira_core::mmi::mmi_from_pgv_larger(
                                    f64::from(v),
                                    r.mw_mean,
                                    10.0,
                                ),
                            )),
                        };
                        for (c, name) in r.cities.iter().zip(&city_names) {
                            ui.label(RichText::new(name).small());
                            ui.label(RichText::new(lv(c.1, c.3).label()).small().strong());
                            ui.label(RichText::new(lv(c.2, c.4).label()).small());
                            ui.end_row();
                        }
                    });
                ui.add_space(4.0);
                ui.label(RichText::new(t("batch-method")).small().weak());
            });
        self.batch.show = open;
        if start {
            self.batch_start(ctx);
        }
        if (show_on_map || (view_changed && self.batch.on_map)) && self.sim.is_none() {
            self.batch.on_map = true;
            self.map.set_grid(self.batch.grid());
        }
    }
}

#[cfg(test)]
mod tests {
    /// Hazır senaryoların fayında varsayılan toplu senaryonun hiçbir üyesi
    /// geçerlilik denetiminde engellenmez (2026-11-27: uca yakın köşe 1 km'den
    /// kısa parça üretip Mw 6,5 ve 6,75'i düşürüyordu).
    #[test]
    fn hazir_senaryolarda_uye_engellenmez() {
        let path = std::path::Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/scenarios/scenarios.json"
        ));
        for p in crate::scenarios::load(path).unwrap() {
            for m in seismira_core::batch::members(&p.quake, &seismira_core::batch::Spec::default())
            {
                let blocked: Vec<_> = seismira_core::quake::validate(&m.quake)
                    .into_iter()
                    .filter(|i| i.blocks())
                    .collect();
                assert!(
                    blocked.is_empty()
                        || p.quake.setting != seismira_core::quake::Setting::ShallowCrust,
                    "{} Mw {}: {blocked:?}",
                    p.id,
                    m.quake.mw
                );
            }
        }
    }
}
