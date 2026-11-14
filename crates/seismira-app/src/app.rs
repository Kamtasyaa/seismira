//! Ana pencere ve panel düzeni (CLAUDE.md Bölüm 10).

use eframe::egui::{self, Align2, Color32, FontId, RichText, Stroke};
use seismira_core::intensity::Shindo;
use seismira_core::sim::{Event, EventKind, Simulation};
use seismira_core::source::Fault;
use seismira_core::traveltime::{Front, Phase, TravelTimes};
use seismira_core::{geo, mercator, quake};
use std::path::PathBuf;
use std::sync::Arc;

use crate::city_layer::{self, CityLayer};
use crate::compute::{self, CityResults};
use crate::map::{MapWidget, view::MapView};
use crate::timeline::{self, Clock};
use crate::{fonts, i18n::Lang, panel, paths, shindo_style, text_test, theme};

/// Dalga halkası renkleri. P (önce gelen, zayıf sarsıntı) soğuk, S (güçlü
/// sarsıntı) sıcak renk. Renk tek bilgi taşıyıcı değildir: haritada
/// açıklama kutusu var (Bölüm 3.2).
const P_COLOR: Color32 = Color32::from_rgb(0x5A, 0xB0, 0xFF);
const S_COLOR: Color32 = Color32::from_rgb(0xFF, 0x8A, 0x45);
/// Halka başına jeodezik nokta sayısı.
const RING_POINTS: usize = 256;

const STORAGE_LANG: &str = "lang";

/// Şehir bildirimleri (bildirim akışında yer alır).
#[derive(Clone, Copy, Debug)]
enum CityEventKind {
    PArrived,
    Shindo(Shindo),
}

#[derive(Clone, Copy, Debug)]
struct CityEvent {
    t: f64,
    city: usize,
    /// Bölgenin (il/eyalet) en kalabalık şehri; bildirimde bölge adı yerine
    /// kullanılır (admin1 adları GeoNames'te İngilizce ve bazen boş).
    region_main: usize,
    kind: CityEventKind,
}

/// Bildirim akışına girecek şehir olayları. Kalabalığı önlemek için il /
/// eyalet (GeoNames admin1) başına toplanır (Bölüm 10 örnekleri de bölge
/// düzeyindedir; GeoNames büyük şehirlerin ilçelerini ayrı şehir sayar):
/// - P varışı: hissedilen bölgelerin en kalabalık şehri, nüfusu ≥ 1 milyon
///   ya da başkentse; en erken 10 bölge.
/// - Shindo: bölgedeki en yüksek Shindo; 5- ve üstü bölgeler ile en güçlü
///   3 bölge (daha düşükler haritada ve şehir kartında görünür). Olay
///   zamanı, o şehrin kutusunun son değere vardığı an (S varışından sonra
///   kademeli yükselme, `shaking::RISE_STEP_S`).
fn city_events(layer: &CityLayer, results: &CityResults) -> Vec<CityEvent> {
    use std::collections::HashMap;
    let jma = |i: usize| results.sites[i].motion.map(|m| m.jma).unwrap_or(f64::MIN);
    // bölge → (en kalabalık şehir, en güçlü sarsılan şehir)
    let mut regions: HashMap<([u8; 2], &str), (usize, usize)> = HashMap::new();
    for (i, r) in results.sites.iter().enumerate() {
        if !r.motion.is_some_and(|m| m.shindo() >= Shindo::S1) {
            continue;
        }
        let c = &layer.cities[i];
        let key = (
            c.country,
            if c.admin1.is_empty() {
                c.name.as_str()
            } else {
                c.admin1.as_str()
            },
        );
        let e = regions.entry(key).or_insert((i, i));
        if c.population > layer.cities[e.0].population {
            e.0 = i;
        }
        if jma(i) > jma(e.1) {
            e.1 = i;
        }
    }
    let mut out = Vec::new();
    let mut big: Vec<usize> = regions
        .values()
        .map(|&(main, _)| main)
        .filter(|&i| layer.cities[i].population >= 1_000_000 || layer.cities[i].capital)
        .collect();
    let tp = |i: usize| results.sites[i].t_p.unwrap_or(f64::MAX);
    big.sort_by(|&a, &b| tp(a).total_cmp(&tp(b)));
    for &i in big.iter().take(10) {
        if let Some(t) = results.sites[i].t_p {
            out.push(CityEvent {
                t,
                city: i,
                region_main: i,
                kind: CityEventKind::PArrived,
            });
        }
    }
    let mut strong: Vec<(usize, usize)> = regions.values().copied().collect();
    strong.sort_by(|a, b| jma(b.1).total_cmp(&jma(a.1)));
    for (k, &(main, best)) in strong.iter().enumerate() {
        let r = &results.sites[best];
        let Some(m) = r.motion else { continue };
        if k >= 3 && m.shindo() < Shindo::S5Lower {
            break;
        }
        if let Some(t) = r.t_peak() {
            out.push(CityEvent {
                t,
                city: best,
                region_main: main,
                kind: CityEventKind::Shindo(m.shindo()),
            });
        }
    }
    out.sort_by(|a, b| a.t.total_cmp(&b.t));
    out
}

pub struct SeismiraApp {
    lang: Lang,
    assets: PathBuf,
    show_text_test: bool,
    /// Eğitim uyarısı. Kapatılabilir ama her açılışta yeniden görünür
    /// (bilerek kaydedilmez); Görünüm menüsünden geri açılır.
    show_disclaimer: bool,
    /// Sarsıntı alanı (ızgara) katmanı görünsün mü (Görünüm menüsü).
    show_grid: bool,
    map: MapWidget,
    /// Sağ paneldeki taslak deprem.
    draft: panel::Draft,
    /// Çalışan simülasyon; taslak değişince iptal edilir.
    sim: Option<Simulation>,
    clock: Clock,
    travel_times: Result<Arc<TravelTimes>, String>,
    cities: Result<CityLayer, String>,
    /// Vs30 haritası; yoksa referans zemin kullanılır (kartta belirtilir).
    vs30: Option<Arc<compute::Vs30>>,
    /// Şehir hesabı (arka planda) ve sonuçları.
    job: Option<compute::Job>,
    results: Option<CityResults>,
    ranked: Vec<usize>,
    city_events: Vec<CityEvent>,
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
        let travel_times = TravelTimes::load(&tt_path).map(Arc::new).map_err(|e| {
            log::error!("{}: {e}", tt_path.display());
            format!("{}: {e}", tt_path.display())
        });
        // Şehir listesi (~1.3 MB) de açılışta okunur.
        let cities = CityLayer::load(&paths::data_dir().join("cities").join("cities15000.scty"));
        if let Err(e) = &cities {
            log::error!("{e}");
        }
        // Vs30 (~150 MB) belleğe eşlenir; açılışta yalnızca başlık okunur.
        let vs30 = compute::load_vs30(&paths::data_dir().join("vs30").join("vs30.sras"))
            .inspect_err(|e| log::warn!("Vs30 haritası yok, referans zemin kullanılacak: {e}"))
            .ok();
        Self {
            lang,
            assets,
            show_text_test: false,
            show_disclaimer: true,
            show_grid: true,
            map: MapWidget::new(paths::data_dir(), initial_view()),
            draft: panel::Draft::default(),
            sim: None,
            clock: Clock::default(),
            travel_times,
            cities,
            vs30,
            job: None,
            results: None,
            ranked: Vec::new(),
            city_events: Vec::new(),
            hover: None,
        }
        .with_demo(&cc.egui_ctx)
    }

    /// Geliştirme: `SEISMIRA_DEMO=boylam,enlem,Mw,derinlik,saniye[,doğrultu]` ile depremi
    /// başlatıp verilen anda duraklatır (ekran görüntüsü testleri için).
    fn with_demo(mut self, ctx: &egui::Context) -> Self {
        let Some(v) = std::env::var("SEISMIRA_DEMO").ok().map(|s| {
            s.split(',')
                .filter_map(|x| x.trim().parse::<f64>().ok())
                .collect::<Vec<_>>()
        }) else {
            return self;
        };
        let (strike, v) = match v[..] {
            [a, b, c, d, e, strike] => (strike, vec![a, b, c, d, e]),
            _ => (0.0, v),
        };
        if let [lon, lat, mw, depth, t] = v[..] {
            let q = &mut self.draft.quake;
            q.strike = strike;
            q.epicenter = (lon, lat);
            q.mw = mw;
            q.depth_km = depth;
            if depth > quake::limits::CRUST_DEPTH.1 {
                q.setting = quake::Setting::Intraslab;
            }
            self.draft.has_epicenter = true;
            self.start_sim(ctx);
            self.clock.t = t;
            self.clock.playing = false;
        }
        self
    }

    /// Geliştirme: `SEISMIRA_CARD=şehir adı` o şehrin kartını imleç olmadan
    /// gösterir (ekran görüntüsü testleri için).
    fn dev_card(&self, rect: egui::Rect) -> Option<(usize, egui::Pos2)> {
        let name = std::env::var("SEISMIRA_CARD").ok()?;
        let layer = self.cities.as_ref().ok()?;
        let i = layer.cities.iter().position(|c| c.name == name)?;
        let c = &layer.cities[i];
        Some((i, self.map.to_screen(rect, (c.lon, c.lat))))
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
                ui.checkbox(&mut self.show_grid, t("menu-shake-grid"));
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
        self.map.set_grid_time(
            self.sim
                .as_ref()
                .filter(|_| self.show_grid)
                .map(|_| self.clock.t),
        );
        let out = self.map.ui(ui, frame, crate::MSAA);
        if let Some(c) = out.clicked {
            self.draft.quake.epicenter = c;
            self.draft.has_epicenter = true;
            self.cancel_sim();
        }
        self.hover = out.hover;

        let felt = self.results.as_ref().and_then(|r| r.felt_radius_deg);
        if let Some(sim) = &self.sim {
            let (p, s) = sim.fronts(self.clock.t);
            let center = sim.quake.epicenter;
            for (front, color) in [(s, S_COLOR), (p, P_COLOR)] {
                if let Front::At(deg) = front
                    && deg > 0.0
                {
                    // Hissedilme sınırının ötesinde soluk (Bölüm 7.2).
                    let color = if felt.is_some_and(|f| deg > f) {
                        color.gamma_multiply(0.35)
                    } else {
                        color
                    };
                    let pts = geo::ring(center, deg, RING_POINTS);
                    self.map
                        .draw_lonlat_line(ui, rect, &pts, Stroke::new(2.0, color));
                }
            }
        } else if self.draft.has_epicenter {
            self.draw_fault_preview(ui, rect);
        }

        // Model geçerlilik sınırı (kesikli): ötesinde şiddet tahmini yok.
        if self.sim.is_some()
            && let Some(limit) = self.results.as_ref().and_then(|r| r.model_limit.as_ref())
        {
            self.map.draw_lonlat_dashed(
                ui,
                rect,
                limit,
                Stroke::new(1.5, city_layer::MODEL_LIMIT_COLOR),
            );
        }

        // Şehirler, P işaretleri ve Shindo kutuları.
        let hover_pos = ui
            .input(|i| i.pointer.hover_pos())
            .filter(|p| rect.contains(*p));
        let mut hovered_city = None;
        if let Ok(layer) = &self.cities {
            let sim_state = match (&self.sim, &self.results) {
                (Some(_), Some(r)) => Some((r, self.ranked.as_slice(), self.clock.t)),
                _ => None,
            };
            hovered_city = layer
                .draw(ui, rect, &self.map, sim_state, P_COLOR, hover_pos)
                .hovered;
        }

        if let Some(sim) = &self.sim {
            self.map.draw_epicenter(ui, rect, sim.quake.epicenter);
            self.legend(ui, rect);
            if self.results.is_some() && quake_has_gmpe(&sim.quake) {
                city_layer::draw_legend(
                    ui,
                    rect,
                    &lang.t("shindo-legend"),
                    Some(&lang.t("model-limit")),
                );
            }
        } else if self.draft.has_epicenter {
            self.map
                .draw_epicenter(ui, rect, self.draft.quake.epicenter);
        }
        if let (Some(i), Some(p)) = (hovered_city, hover_pos) {
            self.city_card(ui, rect, i, p);
        } else if let Some((i, p)) = self.dev_card(rect) {
            self.city_card(ui, rect, i, p);
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

fn quake_has_gmpe(q: &quake::Quake) -> bool {
    seismira_core::shaking::has_gmpe(q.setting)
}

impl SeismiraApp {
    /// Fay önizlemesi: üst kenar (ince çizgi) ve eğimli fayda yüzey izdüşümü
    /// (çok soluk). Bölüm 6: bu, sonuçtaki kırılma izi DEĞİLDİR.
    fn draw_fault_preview(&self, ui: &egui::Ui, rect: egui::Rect) {
        let fault = Fault::from_quake(&self.draft.quake);
        let densify = |a: (f64, f64), b: (f64, f64)| -> Vec<(f64, f64)> {
            let mut b = b;
            if (b.0 - a.0).abs() > 180.0 {
                b.0 -= 360.0 * (b.0 - a.0).signum();
            }
            (0..=24)
                .map(|k| {
                    let f = f64::from(k) / 24.0;
                    (a.0 + (b.0 - a.0) * f, a.1 + (b.1 - a.1) * f)
                })
                .collect()
        };
        if fault.dip < 89.0 {
            let c = fault.surface_outline_lonlat();
            let mut ring = Vec::new();
            for k in 0..4 {
                ring.extend(densify(c[k], c[(k + 1) % 4]));
            }
            self.map.draw_lonlat_line(
                ui,
                rect,
                &ring,
                Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 60)),
            );
        }
        let [a, b] = fault.top_edge_lonlat();
        self.map.draw_lonlat_line(
            ui,
            rect,
            &densify(a, b),
            Stroke::new(1.5, Color32::from_rgba_unmultiplied(255, 255, 255, 170)),
        );
    }

    /// Şehir kartı (fare üzerindeyken): Shindo ve olası aralık, PGA, PGV,
    /// varış zamanları, S − P. Tam şehir kartı Faz 4'te.
    fn city_card(&self, ui: &egui::Ui, rect: egui::Rect, i: usize, at: egui::Pos2) {
        let lang = self.lang;
        let Ok(layer) = &self.cities else { return };
        let c = &layer.cities[i];
        let mut lines: Vec<(String, Color32)> = Vec::new();
        let text = ui.visuals().text_color();
        let weak = ui.visuals().weak_text_color();
        let place = if c.admin1.is_empty() {
            String::from_utf8_lossy(&c.country).into_owned()
        } else {
            format!("{}, {}", c.admin1, String::from_utf8_lossy(&c.country))
        };
        lines.push((c.name.clone(), text));
        lines.push((place, weak));
        let mut shindo = None;
        if let (Some(sim), Some(res)) = (&self.sim, &self.results) {
            let r = &res.sites[i];
            match r.motion {
                Some(m) => {
                    let (lo, hi) = m.shindo_range();
                    shindo = Some(m.shindo());
                    lines.push((
                        lang.t_args(
                            "tip-shindo",
                            &[
                                ("s", m.shindo().to_string()),
                                ("i", lang.num(seismira_core::intensity::round_jma(m.jma), 1)),
                                ("lo", lo.to_string()),
                                ("hi", hi.to_string()),
                            ],
                        ),
                        text,
                    ));
                    lines.push((
                        lang.t_args("tip-pga", &[("v", lang.num(m.pga.median() * 100.0, 1))]),
                        text,
                    ));
                    lines.push((
                        lang.t_args("tip-pgv", &[("v", lang.num(m.pgv.median(), 1))]),
                        text,
                    ));
                }
                None if quake_has_gmpe(&sim.quake) => {
                    lines.push((lang.t("tip-outside-model"), weak))
                }
                None => lines.push((lang.t("no-gmpe-short"), weak)),
            }
            if let (Some(tp), Some(ts)) = (r.t_p, r.t_s) {
                lines.push((
                    lang.t_args(
                        "tip-arrivals",
                        &[
                            ("p", timeline::format_hms(tp)),
                            ("s", timeline::format_hms(ts)),
                            ("sp", lang.num(ts - tp, 1)),
                        ],
                    ),
                    text,
                ));
            }
            lines.push((
                lang.t_args("tip-distance", &[("d", lang.num(r.dist.rrup, 0))]),
                weak,
            ));
            if r.motion.is_some() {
                use seismira_core::site::Vs30Source;
                let v = lang.num(r.site.vs30, 0);
                let key = match r.site.source {
                    Vs30Source::Map => "tip-vs30-map",
                    Vs30Source::Nearby => "tip-vs30-nearby",
                    Vs30Source::Reference => "tip-vs30-reference",
                    Vs30Source::Built => "tip-vs30-built",
                };
                lines.push((lang.t_args(key, &[("v", v)]), weak));
                if (r.vs30_used - r.site.vs30).abs() > 0.5 {
                    lines.push((
                        lang.t_args("tip-vs30-clamped", &[("v", lang.num(r.vs30_used, 0))]),
                        weak,
                    ));
                }
            }
        }
        let painter = ui.painter_at(rect);
        let font = FontId::proportional(13.0);
        let galleys: Vec<_> = lines
            .into_iter()
            .map(|(t, c)| painter.layout_no_wrap(t, font.clone(), c))
            .collect();
        let w = galleys.iter().map(|g| g.size().x).fold(0.0, f32::max);
        let h: f32 = galleys.iter().map(|g| g.size().y + 2.0).sum();
        let badge = if shindo.is_some() { 26.0 } else { 0.0 };
        let mut origin = at + egui::vec2(14.0, 14.0);
        let size = egui::vec2(w + badge + 16.0, h + 12.0);
        if origin.x + size.x > rect.right() {
            origin.x = at.x - 14.0 - size.x;
        }
        if origin.y + size.y > rect.bottom() {
            origin.y = at.y - 14.0 - size.y;
        }
        let card = egui::Rect::from_min_size(origin, size);
        painter.rect_filled(card, 6.0, ui.visuals().window_fill);
        painter.rect_stroke(
            card,
            6.0,
            ui.visuals().window_stroke,
            egui::StrokeKind::Inside,
        );
        if let Some(s) = shindo {
            shindo_style::draw_box(
                &painter,
                card.left_top() + egui::vec2(8.0 + 10.0, 8.0 + 9.0),
                s,
            );
        }
        let mut y = card.top() + 6.0;
        for g in galleys {
            let gh = g.size().y;
            painter.galley(egui::pos2(card.left() + 8.0 + badge, y), g, text);
            y += gh + 2.0;
        }
    }

    fn cancel_sim(&mut self) {
        self.job = None; // Drop → arka plan hesabı iptal
        self.map.set_grid(None);
        self.results = None;
        self.ranked.clear();
        self.city_events.clear();
        self.sim = None;
        self.clock = Clock {
            speed: self.clock.speed,
            ..Clock::default()
        };
    }

    fn start_sim(&mut self, ctx: &egui::Context) {
        let Ok(tt) = &self.travel_times else {
            return;
        };
        let tt = tt.clone();
        if quake::validate(&self.draft.quake)
            .iter()
            .any(|i| i.blocks())
        {
            log::warn!("geçersiz girdiyle başlatma engellendi");
            return;
        }
        self.cancel_sim();
        self.sim = Simulation::new(self.draft.quake, &tt);
        if self.sim.is_some()
            && let Ok(layer) = &self.cities
        {
            self.job = Some(compute::Job::spawn(
                self.draft.quake,
                layer.cities.clone(),
                layer.sites.clone(),
                tt,
                self.vs30.clone(),
                ctx.clone(),
            ));
        }
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
        // Çekirdek olayları ve şehir olayları tek akışta, zamana göre.
        struct Note {
            t: f64,
            text: String,
            color: Color32,
            badge: Option<Shindo>,
            at: (f64, f64),
        }
        let now = self.clock.t;
        let mut notes: Vec<Note> = sim
            .events_until(now)
            .iter()
            .map(|e| Note {
                t: e.t,
                text: self.event_text(sim, e),
                color: match e.kind {
                    EventKind::Origin => Color32::from_rgb(0xFF, 0x6B, 0x6B),
                    EventKind::Surfaced(_) => ui.visuals().text_color(),
                    EventKind::ReachedLimit(_) => ui.visuals().weak_text_color(),
                },
                badge: None,
                at: e.location,
            })
            .collect();
        if let Ok(layer) = &self.cities {
            for e in self.city_events.iter().filter(|e| e.t <= now) {
                let c = &layer.cities[e.city];
                let (text, badge) = match e.kind {
                    CityEventKind::PArrived => (
                        lang.t_args("city-p-arrived", &[("city", c.name.clone())]),
                        None,
                    ),
                    CityEventKind::Shindo(s) => {
                        let main = &layer.cities[e.region_main];
                        // Bölge adı: il/eyalet (GeoNames admin1); yoksa
                        // bölgenin en kalabalık şehri.
                        let region = if c.admin1.is_empty() {
                            main.name.clone()
                        } else {
                            c.admin1.clone()
                        };
                        let text = if region == c.name {
                            lang.t_args(
                                "city-shindo",
                                &[("city", c.name.clone()), ("shindo", s.to_string())],
                            )
                        } else {
                            lang.t_args(
                                "region-shindo",
                                &[
                                    ("region", region),
                                    ("city", c.name.clone()),
                                    ("shindo", s.to_string()),
                                ],
                            )
                        };
                        (text, Some(s))
                    }
                };
                notes.push(Note {
                    t: e.t,
                    text,
                    color: if badge.is_some_and(|s| s >= Shindo::S5Lower) {
                        Color32::from_rgb(0xFF, 0x9A, 0x6B)
                    } else {
                        ui.visuals().text_color()
                    },
                    badge,
                    at: (c.lon, c.lat),
                });
            }
        }
        notes.sort_by(|a, b| b.t.total_cmp(&a.t)); // en yenisi üstte
        let mut focus = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            for n in &notes {
                let time = lang.t_args("time-elapsed", &[("t", timeline::format_hms(n.t))]);
                let r = egui::Frame::group(ui.style())
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(RichText::new(time).small().weak().monospace());
                        ui.horizontal_top(|ui| {
                            if let Some(s) = n.badge {
                                let (rect, _) = ui.allocate_exact_size(
                                    shindo_style::BOX_SIZE,
                                    egui::Sense::hover(),
                                );
                                shindo_style::draw_box(ui.painter(), rect.center(), s);
                            }
                            // Kalan genişlikte alt satıra geçsin.
                            ui.vertical(|ui| {
                                ui.add(
                                    egui::Label::new(RichText::new(&n.text).color(n.color)).wrap(),
                                );
                            });
                        });
                    })
                    .response
                    .interact(egui::Sense::click());
                if r.clicked() {
                    focus = Some(n.at);
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
                        panel::Action::Start => {
                            let ctx = ui.ctx().clone();
                            self.start_sim(&ctx);
                        }
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

        // Arka plan hesabının sonuçları (şehirler, kaba / ince ızgara).
        if let Some(job) = &self.job {
            let (msgs, done) = job.poll();
            for msg in msgs {
                match msg {
                    compute::JobMsg::Cities(results) => {
                        if let Ok(layer) = &self.cities {
                            self.ranked = layer.rank_for_boxes(&results);
                            self.city_events = city_events(layer, &results);
                        }
                        self.results = Some(results);
                    }
                    compute::JobMsg::Grid(grid) => self.map.set_grid(Some(grid)),
                }
            }
            if done {
                self.job = None;
            }
        }

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
