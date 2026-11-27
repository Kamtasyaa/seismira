//! "Kendi verini yükle" ve "Sonuçları dışa aktar" pencereleri
//! ([`crate::userdata`], [`crate::export`]).
//!
//! Dosya seçme: sistemin dosya penceresi (`rfd` 0.18; Linux'ta xdg-desktop-
//! portal, GTK yok; 2026-11-26 kullanıcı onayı), yol kutusu ve pencereye
//! sürükle-bırak. Bırakma winit 0.30'da Wayland'de yok (yalnızca X11 /
//! Windows / macOS; kaynakta denetlendi). Dosya penceresi rfd'nin önerdiği
//! gibi ana iş parçacığında açılır, sonucu yardımcı iş parçacığında
//! beklenir (arayüz donmaz; macOS'ta da doğru).
//! Okuma ve yazma arka plan iş parçacığında (Bölüm 11).

use super::SeismiraApp;
use crate::compute;
use crate::export::csv_field;
use crate::userdata::{Inventory, LoadError, Loaded, UserVs30};
use eframe::egui::{self, Color32, RichText, Stroke};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, TryRecvError};

/// Haritada çizilecek en çok envanter noktası (daha fazlası çizilmez).
const MAX_DRAWN_POINTS: usize = 5000;

#[derive(Default)]
pub(super) struct UserState {
    pub show_load: bool,
    pub show_export: bool,
    path: String,
    pub vs30: Option<Arc<UserVs30>>,
    pub inventory: Option<Arc<Inventory>>,
    pub inventory_results: Option<compute::InventoryResults>,
    loading: Option<Receiver<(PathBuf, Result<Loaded, LoadError>)>>,
    /// Açık dosya seçme penceresinin sonucu.
    picking: Option<Receiver<Option<PathBuf>>>,
    /// Son yükleme sonucu: (başarılı mı, metin).
    msg: Option<(bool, String)>,
    exporting: Option<Receiver<Result<PathBuf, String>>>,
    export_msg: Option<(bool, String, Option<PathBuf>)>,
    /// Son ince sarsıntı ızgarası (dışa aktarma için).
    pub grid: Option<Arc<seismira_core::grid::ShakeGrid>>,
    /// Geliştirme: `SEISMIRA_EXPORT=1` ile bir kez dışa aktarıldı.
    pub dev_exported: bool,
}

impl SeismiraApp {
    /// Her karede: bırakılan dosyalar, biten yükleme ve dışa aktarma.
    pub(super) fn user_poll(&mut self, ctx: &egui::Context) {
        let dropped: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .map(|f| f.path().to_path_buf())
                .filter(|p| !p.as_os_str().is_empty())
                .collect()
        });
        for p in dropped {
            self.user.show_load = true;
            self.user_start_load(ctx, p);
        }
        if let Some(rx) = &self.user.loading {
            match rx.try_recv() {
                Ok((path, res)) => {
                    self.user.loading = None;
                    self.user_loaded(ctx, &path, res);
                }
                Err(TryRecvError::Empty) => {
                    ctx.request_repaint_after(std::time::Duration::from_millis(100));
                }
                Err(TryRecvError::Disconnected) => self.user.loading = None,
            }
        }
        if let Some(rx) = &self.user.picking {
            match rx.try_recv() {
                Ok(p) => {
                    self.user.picking = None;
                    if let Some(p) = p {
                        self.user.path = p.display().to_string();
                        self.user_start_load(ctx, p);
                    }
                }
                Err(TryRecvError::Empty) => {
                    ctx.request_repaint_after(std::time::Duration::from_millis(100));
                }
                Err(TryRecvError::Disconnected) => self.user.picking = None,
            }
        }
        if let Some(rx) = &self.user.exporting {
            let lang = self.lang;
            match rx.try_recv() {
                Ok(Ok(dir)) => {
                    self.user.exporting = None;
                    let text = lang.t_args("export-done", &[("dir", dir.display().to_string())]);
                    self.user.export_msg = Some((true, text, Some(dir)));
                }
                Ok(Err(e)) => {
                    self.user.exporting = None;
                    self.user.export_msg = Some((false, e, None));
                }
                Err(TryRecvError::Empty) => {
                    ctx.request_repaint_after(std::time::Duration::from_millis(100));
                }
                Err(TryRecvError::Disconnected) => self.user.exporting = None,
            }
        }
    }

    fn user_start_load(&mut self, ctx: &egui::Context, path: PathBuf) {
        if self.user.loading.is_some() {
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        let ctx = ctx.clone();
        let p = path.clone();
        let _ = std::thread::Builder::new()
            .name("kullanıcı-verisi".into())
            .spawn(move || {
                let r = crate::userdata::load(&p);
                let _ = tx.send((p, r));
                ctx.request_repaint();
            });
        self.user.loading = Some(rx);
        self.user.msg = None;
        log::info!("kullanıcı verisi yükleniyor: {}", path.display());
    }

    fn user_loaded(&mut self, ctx: &egui::Context, path: &Path, res: Result<Loaded, LoadError>) {
        let lang = self.lang;
        let name = path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        match res {
            Err(e) => {
                log::warn!(
                    "kullanıcı verisi okunamadı: {}: {} {}",
                    path.display(),
                    e.key,
                    e.detail
                );
                let mut text = lang.t_args("user-err", &[("file", name)]);
                text.push(' ');
                text.push_str(&lang.t(e.key));
                if !e.detail.is_empty() {
                    let _ = write!(text, " ({})", e.detail);
                }
                self.user.msg = Some((false, text));
            }
            Ok(Loaded::Fault(pts)) => {
                // Çizim bitirmeyle aynı yol: iz olur, merkez üssü yoksa ortası.
                self.cancel_sim();
                self.draft.drawing = Some(pts.clone());
                self.draft.finish_drawing();
                self.map.center_on(self.draft.quake.epicenter);
                self.user.msg = Some((
                    true,
                    lang.t_args(
                        "user-ok-fault",
                        &[("file", name), ("n", lang.num((pts.len() - 1) as f64, 0))],
                    ),
                ));
            }
            Ok(Loaded::Vs30(v)) => {
                let text = lang.t_args(
                    "user-ok-vs30",
                    &[
                        ("file", name),
                        ("w", lang.num(v.width as f64, 0)),
                        ("h", lang.num(v.height as f64, 0)),
                        ("valid", lang.num(v.valid as f64, 0)),
                    ],
                );
                self.user.vs30 = Some(Arc::new(v));
                self.user.msg = Some((true, text));
                self.user_recompute(ctx);
            }
            Ok(Loaded::Inventory(inv)) => {
                let text = lang.t_args(
                    "user-ok-inventory",
                    &[
                        ("file", name),
                        ("n", lang.num(inv.points.len() as f64, 0)),
                        ("b", lang.num(inv.total(), 0)),
                        ("skipped", lang.num(inv.skipped as f64, 0)),
                    ],
                );
                self.user.inventory = Some(Arc::new(inv));
                self.user.inventory_results = None;
                self.user.msg = Some((true, text));
                self.user_recompute(ctx);
            }
        }
    }

    /// Açık simülasyon varsa yeni veriyle yeniden hesaplanır (saat korunur).
    fn user_recompute(&mut self, ctx: &egui::Context) {
        if self.sim.is_some() {
            self.user.inventory_results = None;
            self.spawn_job(ctx);
        }
    }

    pub(super) fn user_windows(&mut self, ctx: &egui::Context) {
        let lang = self.lang;
        let t = |k: &str| lang.t(k);
        let mut open = self.user.show_load;
        let mut load_path = None;
        let mut pick = false;
        let mut remove_vs30 = false;
        let mut remove_inv = false;
        egui::Window::new(t("user-title"))
            .open(&mut open)
            .default_pos([320.0, 80.0])
            .default_width(460.0)
            .show(ctx, |ui| {
                ui.label(RichText::new(t("user-intro")).small());
                ui.add_space(6.0);
                let busy = self.user.loading.is_some() || self.user.picking.is_some();
                if ui
                    .add_enabled(
                        !busy,
                        egui::Button::new(RichText::new(t("user-pick")).strong()),
                    )
                    .clicked()
                {
                    pick = true;
                }
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let l = ui.label(t("user-path")).id;
                    let r = ui
                        .add(
                            egui::TextEdit::singleline(&mut self.user.path)
                                .hint_text("/home/…/fay.geojson")
                                .desired_width(ui.available_width() - 80.0),
                        )
                        .labelled_by(l);
                    let r = crate::clip::EditMenu::edit_menu(r, lang);
                    let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    let busy = self.user.loading.is_some();
                    if (ui
                        .add_enabled(!busy, egui::Button::new(t("user-load")))
                        .clicked()
                        || enter)
                        && !self.user.path.trim().is_empty()
                    {
                        // Yapıştırılan yol tırnaklı ya da file:// olabilir.
                        let p = self.user.path.trim().trim_matches(['"', '\'']);
                        let p = p.strip_prefix("file://").unwrap_or(p);
                        load_path = Some(PathBuf::from(p));
                    }
                });
                ui.label(RichText::new(t("user-drop")).small().weak());
                if self.user.loading.is_some() {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(t("user-loading"));
                    });
                }
                if let Some((ok, text)) = &self.user.msg {
                    let color = if *ok {
                        ui.visuals().text_color()
                    } else {
                        crate::panel::warn_color()
                    };
                    ui.colored_label(color, text);
                }
                ui.separator();
                // Yüklü Vs30.
                match &self.user.vs30 {
                    Some(v) => {
                        let [w, s, e, n] = v.bounds();
                        ui.label(lang.t_args(
                            "user-vs30-loaded",
                            &[
                                ("file", v.name.clone()),
                                ("valid", lang.num(v.valid as f64, 0)),
                                ("w", lang.num(w, 2)),
                                ("s", lang.num(s, 2)),
                                ("e", lang.num(e, 2)),
                                ("n", lang.num(n, 2)),
                            ],
                        ));
                        if ui.button(t("user-remove")).clicked() {
                            remove_vs30 = true;
                        }
                    }
                    None => {
                        ui.label(RichText::new(t("user-vs30-none")).weak());
                    }
                }
                ui.separator();
                // Yüklü envanter ve sonucu.
                match &self.user.inventory {
                    Some(inv) => {
                        ui.label(lang.t_args(
                            "user-inv-loaded",
                            &[
                                ("file", inv.name.clone()),
                                ("n", lang.num(inv.points.len() as f64, 0)),
                                ("b", lang.num(inv.total(), 0)),
                            ],
                        ));
                        match &self.user.inventory_results {
                            Some(r) => {
                                let ([m, lo, hi], covered) = r.totals(inv);
                                ui.label(
                                    RichText::new(lang.t_args(
                                        "user-inv-result",
                                        &[
                                            ("m", lang.num(m, 0)),
                                            ("lo", lang.num(lo, 0)),
                                            ("hi", lang.num(hi, 0)),
                                            ("c", lang.num(covered, 0)),
                                        ],
                                    ))
                                    .strong(),
                                );
                                if covered < inv.total() {
                                    ui.label(RichText::new(t("user-inv-partial")).small().weak());
                                }
                            }
                            None if self.sim.is_some() => {
                                ui.label(RichText::new(t("user-inv-wait")).weak());
                            }
                            None => {
                                ui.label(RichText::new(t("user-inv-run")).weak());
                            }
                        }
                        if ui.button(t("user-remove")).clicked() {
                            remove_inv = true;
                        }
                    }
                    None => {
                        ui.label(RichText::new(t("user-inv-none")).weak());
                    }
                }
            });
        self.user.show_load = open;
        if pick {
            let task = rfd::AsyncFileDialog::new()
                .set_title(t("user-pick-title"))
                .add_filter(
                    t("user-pick-filter"),
                    &["geojson", "json", "tif", "tiff", "csv", "txt"],
                )
                .pick_file();
            let (tx, rx) = std::sync::mpsc::channel();
            let c = ctx.clone();
            let _ = std::thread::Builder::new()
                .name("dosya-seçimi".into())
                .spawn(move || {
                    let r = pollster::block_on(task).map(|f| f.path().to_path_buf());
                    let _ = tx.send(r);
                    c.request_repaint();
                });
            self.user.picking = Some(rx);
        }
        if let Some(p) = load_path {
            self.user_start_load(ctx, p);
        }
        if remove_vs30 {
            self.user.vs30 = None;
            self.user_recompute(ctx);
        }
        if remove_inv {
            self.user.inventory = None;
            self.user.inventory_results = None;
        }

        // Dışa aktarma penceresi.
        let mut open = self.user.show_export;
        let mut start = false;
        let mut open_dir = None;
        egui::Window::new(t("export-title"))
            .open(&mut open)
            .default_pos([340.0, 120.0])
            .default_width(440.0)
            .show(ctx, |ui| {
                ui.label(RichText::new(t("export-intro")).small());
                ui.add_space(4.0);
                ui.label(
                    RichText::new(lang.t_args(
                        "export-where",
                        &[("dir", export_base().display().to_string())],
                    ))
                    .small()
                    .weak(),
                );
                let ready = self.sim.is_some() && self.results.is_some();
                let busy = self.user.exporting.is_some();
                if !ready {
                    ui.label(RichText::new(t("export-need-sim")).weak());
                } else if self.job.is_some() {
                    ui.label(RichText::new(t("export-still-running")).small().weak());
                }
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(ready && !busy, egui::Button::new(t("export-button")))
                        .clicked()
                    {
                        start = true;
                    }
                    if busy {
                        ui.spinner();
                    }
                });
                if let Some((ok, text, dir)) = &self.user.export_msg {
                    let color = if *ok {
                        ui.visuals().text_color()
                    } else {
                        crate::panel::warn_color()
                    };
                    ui.colored_label(color, text);
                    if let Some(d) = dir {
                        ui.horizontal(|ui| {
                            if ui.button(t("export-open")).clicked() {
                                open_dir = Some(d.clone());
                            }
                            let report = d.join("rapor_report.html");
                            if report.exists() && ui.button(t("export-open-report")).clicked() {
                                open_dir = Some(report);
                            }
                        });
                    }
                }
            });
        self.user.show_export = open;
        if start {
            self.export_now(ctx);
        }
        if let Some(d) = open_dir {
            open_folder(&d);
        }
    }

    /// Kullanıcı Vs30 kapsamı (kesikli dikdörtgen) ve envanter noktaları.
    pub(super) fn draw_user_overlays(&self, ui: &egui::Ui, rect: egui::Rect) {
        if let Some(v) = &self.user.vs30 {
            let [w, s, e, n] = v.bounds();
            let ring = [(w, n), (e, n), (e, s), (w, s), (w, n)];
            let mut pts = Vec::new();
            for k in 0..4 {
                let (a, b) = (ring[k], ring[k + 1]);
                pts.extend((0..=16).map(|i| {
                    let f = f64::from(i) / 16.0;
                    (a.0 + (b.0 - a.0) * f, a.1 + (b.1 - a.1) * f)
                }));
            }
            self.map.draw_lonlat_dashed(
                ui,
                rect,
                &pts,
                Stroke::new(1.5, Color32::from_rgb(0x7F, 0xD8, 0xFF)),
            );
        }
        if let Some(inv) = &self.user.inventory
            && inv.points.len() <= MAX_DRAWN_POINTS
        {
            let painter = ui.painter_at(rect);
            for p in &inv.points {
                let c = self.map.to_screen(rect, (p.lon, p.lat));
                if rect.contains(c) {
                    let r = egui::Rect::from_center_size(c, egui::vec2(6.0, 6.0));
                    painter.rect_filled(r.expand(1.0), 1.0, Color32::from_black_alpha(170));
                    painter.rect_filled(r, 1.0, Color32::from_rgb(0x7F, 0xD8, 0xFF));
                }
            }
        }
    }

    /// Dosya metinleri arayüzde hazırlanır (küçük), yazma ve GeoTIFF'ler
    /// arka planda.
    pub(super) fn export_now(&mut self, ctx: &egui::Context) {
        let Some(sim) = &self.sim else {
            return;
        };
        let Some(results) = &self.results else {
            return;
        };
        let lang = self.lang;
        let quake = sim.quake;
        let rupture = seismira_core::rupture::Rupture::from_quake(&quake);
        let mut files: Vec<(String, String)> = Vec::new();

        // Şehirler (model içindekiler).
        if let Ok(layer) = &self.cities {
            let mut s = String::from(
                "name,admin1,country,lon,lat,population,shindo,jma_intensity,mmi,pga_g,pgv_cm_s,\
                 p_arrival_s,s_arrival_s,rrup_km,vs30_m_s,heavy_damage_pct,heavy_damage_pct_low,\
                 heavy_damage_pct_high\n",
            );
            for (i, (c, r)) in layer.cities.iter().zip(&results.sites).enumerate() {
                let Some(m) = r.motion else {
                    continue;
                };
                let dmg = self
                    .damage
                    .as_ref()
                    .and_then(|d| d.cities.get(i).copied().flatten())
                    .map(|d| d.map(|x| 100.0 * seismira_core::damage::RegionDamage::heavy(&x)));
                let _ = writeln!(
                    s,
                    "{},{},{},{:.5},{:.5},{},{},{:.2},{:.2},{:.4},{:.2},{},{},{:.1},{:.0},{}",
                    csv_field(&c.name),
                    csv_field(&c.admin1),
                    String::from_utf8_lossy(&c.country),
                    c.lon,
                    c.lat,
                    c.population,
                    m.shindo().label(),
                    m.jma,
                    m.mmi,
                    m.pga.median(),
                    m.pgv.median(),
                    opt(r.t_p, 1),
                    opt(r.t_s, 1),
                    r.dist.rrup,
                    r.vs30_used,
                    match dmg {
                        Some([a, b, c]) => format!("{a:.2},{b:.2},{c:.2}"),
                        None => ",,".into(),
                    }
                );
            }
            files.push(("sehirler_cities.csv".into(), s));
        }

        // Bölgeler.
        if let Some(reg) = &self.regions
            && !self.region_results.is_empty()
        {
            let mut s = String::from(
                "iso,name,country,max_shindo,first_p_s,first_s_s,buildings,heavy_buildings,\
                 heavy_buildings_low,heavy_buildings_high,liquefaction_km2,landslide_km2\n",
            );
            for rr in &self.region_results {
                let id = rr.summary.id;
                let Some(info) = reg.info(id) else {
                    continue;
                };
                let name = match lang {
                    crate::i18n::Lang::Tr => &info.name_tr,
                    _ => &info.name_en,
                };
                let d = self.damage.as_ref().and_then(|d| d.region(id));
                let gf = |k: usize| {
                    self.ground_failure[k]
                        .0
                        .iter()
                        .find(|g| g.id == id)
                        .map_or(String::new(), |g| format!("{:.1}", g.km2[0]))
                };
                let _ = writeln!(
                    s,
                    "{},{},{},{},{:.1},{:.1},{},{},{}",
                    csv_field(&info.iso),
                    csv_field(name),
                    String::from_utf8_lossy(&info.country),
                    rr.summary.max.label(),
                    rr.summary.t_first_p,
                    rr.summary.t_first_s,
                    match d {
                        Some(d) => format!(
                            "{:.0},{:.0},{:.0},{:.0}",
                            d.buildings,
                            seismira_core::damage::RegionDamage::heavy(&d.median),
                            seismira_core::damage::RegionDamage::heavy(&d.low),
                            seismira_core::damage::RegionDamage::heavy(&d.high)
                        ),
                        None => ",,,".into(),
                    },
                    gf(0),
                    gf(1)
                );
            }
            files.push(("bolgeler_regions.csv".into(), s));
        }

        // Fay (iz + parça izdüşümleri).
        files.push(("fay_fault.geojson".into(), fault_geojson(&quake, &rupture)));

        // Zemin yenilmesi kareleri.
        {
            let mut feats = Vec::new();
            for (k, kind) in ["liquefaction", "landslide"].iter().enumerate() {
                for b in &self.ground_failure[k].1 {
                    let [w, n, e, s] = b.bounds;
                    let iso = self
                        .regions
                        .as_ref()
                        .and_then(|r| r.info(b.region))
                        .map_or(String::new(), |i| i.iso.clone());
                    feats.push(format!(
                        r#"{{"type":"Feature","properties":{{"kind":"{kind}","appears_s":{:.1},"region":{}}},"geometry":{{"type":"Polygon","coordinates":[[[{w},{n}],[{e},{n}],[{e},{s}],[{w},{s}],[{w},{n}]]]}}}}"#,
                        b.t,
                        serde_json::Value::String(iso)
                    ));
                }
            }
            files.push((
                "zemin_ground_failure.geojson".into(),
                format!(
                    r#"{{"type":"FeatureCollection","features":[{}]}}"#,
                    feats.join(",")
                ),
            ));
        }

        // Envanter.
        if let (Some(inv), Some(r)) = (&self.user.inventory, &self.user.inventory_results) {
            let mut s = String::from(
                "lon,lat,buildings,label,shindo,jma_intensity,mmi,pga_g,pgv_cm_s,vs30_m_s,vs30_source,\
                 heavy_buildings,heavy_buildings_low,heavy_buildings_high\n",
            );
            for (p, row) in inv.points.iter().zip(&r.rows) {
                let m = row.motion;
                let _ = writeln!(
                    s,
                    "{:.6},{:.6},{},{},{},{},{},{},{},{:.0},{},{}",
                    p.lon,
                    p.lat,
                    p.buildings,
                    csv_field(&p.label),
                    m.map_or(String::new(), |m| m.shindo().label().to_string()),
                    m.map_or(String::new(), |m| format!("{:.2}", m.jma)),
                    m.map_or(String::new(), |m| format!("{:.2}", m.mmi)),
                    m.map_or(String::new(), |m| format!("{:.4}", m.pga.median())),
                    m.map_or(String::new(), |m| format!("{:.2}", m.pgv.median())),
                    row.vs30,
                    if row.vs30_user { "user" } else { "map" },
                    match row.heavy {
                        Some([a, b, c]) => format!("{a:.1},{b:.1},{c:.1}"),
                        None => ",,".into(),
                    }
                );
            }
            files.push(("envanter_inventory.csv".into(), s));
        }

        files.push(("BENIOKU_README.txt".into(), readme(&quake, &rupture)));
        // Otomatik rapor (Markdown + tarayıcıda yazdırılabilir HTML).
        if let Some((md, html)) = self.build_report() {
            files.push(("rapor_report.md".into(), md));
            files.push(("rapor_report.html".into(), html));
        }

        let grid = self.user.grid.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        let ctx = ctx.clone();
        let _ = std::thread::Builder::new()
            .name("dışa-aktarma".into())
            .spawn(move || {
                let r = write_export(&files, grid.as_deref());
                let _ = tx.send(r);
                ctx.request_repaint();
            });
        self.user.exporting = Some(rx);
        self.user.export_msg = None;
    }
}

fn opt(v: Option<f64>, digits: usize) -> String {
    v.map_or(String::new(), |x| format!("{x:.digits$}"))
}

/// Dışa aktarma kök klasörü: `~/Seismira`.
pub(super) fn export_base() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map_or_else(|| PathBuf::from("."), PathBuf::from)
        .join("Seismira")
}

/// Klasörü oluşturur ve dosyaları yazar; ızgara varsa üç GeoTIFF.
fn write_export(
    files: &[(String, String)],
    grid: Option<&seismira_core::grid::ShakeGrid>,
) -> Result<PathBuf, String> {
    let dir = export_base().join(format!("seismira-{}", timestamp()));
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for (name, text) in files {
        let p = dir.join(name);
        std::fs::write(&p, text).map_err(|e| format!("{}: {e}", p.display()))?;
    }
    if let Some(g) = grid {
        let s = g.spec;
        for (name, data) in [
            ("sarsinti_jma_intensity.tif", &g.jma),
            ("sarsinti_pgv_cm_s.tif", &g.pgv),
            ("sarsinti_pga_g.tif", &g.pga),
        ] {
            let p = dir.join(name);
            crate::export::write_geotiff(
                &p,
                s.west,
                s.north,
                s.cell,
                s.width,
                s.height,
                data,
                Some(-9999.0),
            )
            .map_err(|e| format!("{}: {e}", p.display()))?;
        }
    }
    log::info!("dışa aktarıldı: {}", dir.display());
    Ok(dir)
}

/// Yerel değil UTC zaman damgası: YYYYAAGG-SSDDss (bağımlılıksız; Howard
/// Hinnant'ın "civil_from_days" algoritması).
pub(super) fn timestamp() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs()) as i64;
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}{m:02}{d:02}-{:02}{:02}{:02}",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

fn fault_geojson(q: &seismira_core::quake::Quake, r: &seismira_core::rupture::Rupture) -> String {
    let coords = |pts: &[(f64, f64)]| {
        pts.iter()
            .map(|p| format!("[{:.6},{:.6}]", p.0, p.1))
            .collect::<Vec<_>>()
            .join(",")
    };
    let mut feats = vec![format!(
        r#"{{"type":"Feature","properties":{{"part":"top_edge_trace","mw":{},"depth_km":{},"rake":{},"segments":{},"length_km":{:.1},"width_km":{:.1},"ztor_km":{:.2},"dip":{:.1},"epicenter":[{:.5},{:.5}]}},"geometry":{{"type":"LineString","coordinates":[{}]}}}}"#,
        q.mw,
        q.depth_km,
        q.rake,
        r.segments.len(),
        r.length,
        r.width,
        r.ztor,
        r.dip,
        q.epicenter.0,
        q.epicenter.1,
        coords(&r.trace_lonlat())
    )];
    for (i, f) in r.segments.iter().enumerate() {
        let mut c = f.surface_outline_lonlat().to_vec();
        c.push(c[0]);
        feats.push(format!(
            r#"{{"type":"Feature","properties":{{"part":"segment_surface_projection","segment":{},"strike":{:.1},"dip":{:.1},"length_km":{:.1}}},"geometry":{{"type":"Polygon","coordinates":[[{}]]}}}}"#,
            i + 1,
            f.strike,
            f.dip,
            f.length,
            coords(&c)
        ));
    }
    format!(
        r#"{{"type":"FeatureCollection","features":[{}]}}"#,
        feats.join(",")
    )
}

fn readme(q: &seismira_core::quake::Quake, r: &seismira_core::rupture::Rupture) -> String {
    format!(
        "Seismira dışa aktarma / export\n\
         ==============================\n\n\
         BU BİR EĞİTİM SİMÜLASYONUDUR. Gerçek deprem bilgisi için AFAD, Kandilli, JMA ya da\n\
         USGS gibi resmi kurumları takip edin. / THIS IS AN EDUCATIONAL SIMULATION.\n\n\
         Deprem / Earthquake: Mw {}, derinlik {} km, merkez üssü {:.4} E, {:.4} N;\n\
         kırılma {:.0} × {:.0} km, {} parça / segments.\n\n\
         Koordinatlar / CRS: EPSG:4326 (boylam, enlem / lon, lat).\n\n\
         Dosyalar / Files:\n\
         - sarsinti_jma_intensity.tif: JMA aletsel şiddet (medyan) / instrumental intensity.\n\
         - sarsinti_pgv_cm_s.tif: PGV, cm/s, iki yatay bileşenin büyüğü / larger horizontal.\n\
         - sarsinti_pga_g.tif: PGA, g, RotD50.\n\
           GeoTIFF: tek bant f32, nodata -9999 (model sınırı dışı ya da deniz).\n\
         - sehirler_cities.csv: model içindeki şehirler; hasar yüzdesi = ağır hasar + göçme,\n\
           şehrin bulunduğu ilin bina karışımıyla (ESRM20; Türkiye ve Avrupa).\n\
         - bolgeler_regions.csv: il / eyalet özeti; ağır hasarlı bina (medyan, -1τ, +1τ),\n\
           sıvılaşma / heyelan toplam tehlike alanı (km², USGS 'aggregate hazard').\n\
         - fay_fault.geojson: kırılma üst kenar izi ve parça yüzey izdüşümleri.\n\
         - zemin_ground_failure.geojson: sıvılaşma / heyelan kareleri (~25 km).\n\
         - envanter_inventory.csv: yüklenen bina envanteri ve sonuçları (varsa).\n\n\
         Veri kaynakları ve lisansları: uygulamada Yardım → Veri Kaynakları.\n\
         Data sources and licences: Help → Data sources in the application.\n",
        q.mw,
        q.depth_km,
        q.epicenter.0,
        q.epicenter.1,
        r.length,
        r.width,
        r.segments.len()
    )
}

/// Klasörü sistemin dosya yöneticisinde açar.
fn open_folder(dir: &Path) {
    #[cfg(target_os = "windows")]
    let cmd = "explorer";
    #[cfg(target_os = "macos")]
    let cmd = "open";
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let cmd = "xdg-open";
    if let Err(e) = std::process::Command::new(cmd).arg(dir).spawn() {
        log::warn!("klasör açılamadı ({cmd}): {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zaman_damgasi_bicimi() {
        let s = timestamp();
        assert_eq!(s.len(), 15, "{s}");
        assert!(s.starts_with("20"), "{s}");
        assert_eq!(&s[8..9], "-");
    }
}
