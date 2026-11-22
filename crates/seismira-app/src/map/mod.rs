//! Harita bileşeni: kaydırma, yakınlaştırma, yatay tekrar, merkez üssü seçimi.

mod gpu;
mod grid_gpu;
mod ground_gpu;
mod region_lines;
pub mod view;

pub use region_lines::RegionLines;

use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, Vec2};
use seismira_core::grid::{GridSpec, ShakeGrid};

/// Sıvılaşma katmanı: kaplama ve aynı ızgaranın sarsıntı sonucu.
pub type GroundData = (
    Arc<seismira_core::groundfailure::liquefaction_grid::LiquefactionGrid>,
    Arc<ShakeGrid>,
);

/// Sıvılaşma karesinin belirme süresi (s; "Hareketi azalt"ta anında).
const GROUND_FADE_S: f32 = 1.5;
use seismira_core::mapfile::MapFile;
use seismira_core::mercator;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, TryRecvError, channel};

use gpu::{Draw, LOD_FILES, LayerId, MapGpu, MapPaint, Uniforms};
use view::MapView;

/// Harita renkleri (koyu tema). Okyanus, panel arka planından koyudur.
mod colors {
    use eframe::egui::Color32;
    pub const OCEAN: Color32 = crate::theme::MAP_BG;
    pub const LAND: Color32 = Color32::from_rgb(0x1F, 0x26, 0x31);
    pub const ADMIN0: Color32 = Color32::from_rgb(0x55, 0x61, 0x73);
    pub const ADMIN1: Color32 = Color32::from_rgb(0x33, 0x3D, 0x4B);
    pub const EPICENTER: Color32 = Color32::from_rgb(0xFF, 0x4D, 0x4D);
}

/// Ayrıntı düzeyi eşikleri (dünya genişliği, point). Natural Earth
/// ölçekleri yaklaşık 1:110M, 1:50M, 1:10M haritalar için tasarlanmıştır;
/// eşikler görsel olarak ayarlanmıştır.
const LOD_50M_FROM: f64 = 1_500.0;
const LOD_10M_FROM: f64 = 5_000.0;
/// İl/eyalet sınırları bu ölçekten itibaren çizilir (dünya ölçeğinde
/// yalnızca etkilenen bölgeler çizilecek, CLAUDE.md Bölüm 9).
const ADMIN1_FROM: f64 = 5_000.0;

enum Load {
    /// Arka plan iş parçacığı dosyaları okuyor.
    Loading(Receiver<(usize, Result<MapFile, String>)>),
    Done,
}

/// Haritanın bir karedeki çıktısı.
pub struct MapOutput {
    /// İmlecin altındaki nokta (boylam, enlem).
    pub hover: Option<(f64, f64)>,
    /// Bu karede tıklanarak seçilen nokta (boylam, enlem).
    pub clicked: Option<(f64, f64)>,
}

pub struct MapWidget {
    view: MapView,
    load: Load,
    /// Hangi ayrıntı düzeyleri bellekte, GPU'ya yüklenmeyi bekliyor.
    pending: Vec<(usize, MapFile)>,
    loaded: [bool; 3],
    errors: Vec<String>,
    gpu_ready: bool,
    /// GPU'ya yüklenmeyi bekleyen sarsıntı ızgarası (`Some(None)`: kaldır).
    grid_pending: Option<Option<Arc<ShakeGrid>>>,
    /// Yüklü ızgaranın geometrisi.
    grid_spec: Option<GridSpec>,
    /// Bu karede ızgara gösterilecekse simülasyon zamanı.
    grid_time: Option<f64>,
    /// GPU'ya yüklenmeyi bekleyen sıvılaşma katmanı (`Some(None)`: kaldır).
    ground_pending: Option<Option<GroundData>>,
    ground_spec: Option<GridSpec>,
    /// Bu karede gösterilecekse simülasyon zamanı ve "Hareketi azalt".
    ground_time: Option<(f64, bool)>,
    /// GPU'ya yüklenmeyi bekleyen bölge sınırları (`Some(None)`: kaldır).
    lines_pending: Option<Option<Arc<RegionLines>>>,
    /// Yüklü sınırların bölge → köşe aralığı.
    line_ranges: std::collections::HashMap<seismira_core::region::RegionId, std::ops::Range<u32>>,
    /// Yüklü dolguların bölge → indeks aralığı.
    fill_ranges: std::collections::HashMap<seismira_core::region::RegionId, std::ops::Range<u32>>,
    /// Bu karede vurgulanacak bölgeler (çizim sırasıyla).
    region_styles: Vec<RegionStyle>,
}

/// Bir bölgenin bu karedeki görünümü: sınır (koyu konturla, yüksek
/// kontrast) ve dolgu (düşük kontrast; yanıp sönme genelde burada).
#[derive(Clone, Copy, Debug)]
pub struct RegionStyle {
    pub id: seismira_core::region::RegionId,
    /// Sınır rengi (opaklık dahil).
    pub line: Color32,
    /// Sınır kalınlığı (point); 0 ise sınır çizilmez.
    pub line_width: f32,
    /// Dolgu rengi (opaklık dahil); tam saydamsa dolgu çizilmez.
    pub fill: Color32,
}

/// Sınırın altındaki koyu kontur (kontrast için): renk ve ek kalınlık.
const HALO: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 190);
const HALO_EXTRA: f32 = 2.5;

/// Sarsıntı ızgarası katmanının opaklığı (görsel ayar): sınırlar ve
/// şehir kutuları okunur kalsın.
const GRID_OPACITY: f32 = 0.7;

impl MapWidget {
    pub fn new(data_dir: PathBuf, view: MapView) -> Self {
        let (tx, rx) = channel();
        std::thread::Builder::new()
            .name("harita-yükle".into())
            .spawn(move || {
                // Kabadan inceye: önce dünya görünümü hazır olsun.
                for (lod, name) in LOD_FILES.iter().enumerate() {
                    let path = data_dir.join("map").join(format!("{name}.smap"));
                    let result = std::fs::File::open(&path)
                        .map(std::io::BufReader::new)
                        .and_then(|mut f| MapFile::read(&mut f))
                        .map_err(|e| format!("{}: {e}", path.display()));
                    if tx.send((lod, result)).is_err() {
                        return;
                    }
                }
            })
            .expect("iş parçacığı başlatılamadı");
        Self {
            view,
            load: Load::Loading(rx),
            pending: Vec::new(),
            loaded: [false; 3],
            errors: Vec::new(),
            gpu_ready: false,
            grid_pending: None,
            grid_spec: None,
            grid_time: None,
            ground_pending: None,
            ground_spec: None,
            ground_time: None,
            lines_pending: None,
            line_ranges: Default::default(),
            fill_ranges: Default::default(),
            region_styles: Vec::new(),
        }
    }

    /// Etkilenen bölgelerin sınırları (ya da `None`: kaldır).
    pub fn set_region_lines(&mut self, lines: Option<Arc<RegionLines>>) {
        self.lines_pending = Some(lines);
    }

    /// Bu karede vurgulanacak bölgeler (sonraki en üstte çizilir).
    pub fn set_region_styles(&mut self, styles: Vec<RegionStyle>) {
        self.region_styles = styles;
    }

    /// Yeni sarsıntı ızgarası (ya da `None`: kaldır). Bir sonraki karede
    /// GPU'ya yüklenir.
    pub fn set_grid(&mut self, grid: Option<Arc<ShakeGrid>>) {
        self.grid_pending = Some(grid);
    }

    /// Bu karede ızgara gösterilecekse simülasyon zamanı.
    pub fn set_grid_time(&mut self, t: Option<f64>) {
        self.grid_time = t;
    }

    /// Yeni sıvılaşma katmanı (ya da `None`: kaldır).
    pub fn set_ground(&mut self, ground: Option<GroundData>) {
        self.ground_pending = Some(ground);
    }

    /// Bu karede sıvılaşma gösterilecekse simülasyon zamanı.
    pub fn set_ground_time(&mut self, t: Option<f64>, reduce_motion: bool) {
        self.ground_time = t.map(|t| (t, reduce_motion));
    }

    /// Kaba harita yüklendi mi (ekranda bir şey var mı)?
    pub fn is_ready(&self) -> bool {
        self.loaded.iter().any(|&l| l)
    }

    pub fn errors(&self) -> &[String] {
        &self.errors
    }

    fn poll_loader(&mut self) {
        if let Load::Loading(rx) = &self.load {
            loop {
                match rx.try_recv() {
                    Ok((lod, Ok(map))) => self.pending.push((lod, map)),
                    Ok((_, Err(e))) => {
                        log::error!("harita verisi okunamadı: {e}");
                        self.errors.push(e);
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        self.load = Load::Done;
                        break;
                    }
                }
            }
        }
    }

    /// GPU kaynaklarını kurar ve bekleyen ayrıntı düzeylerini yükler.
    fn sync_gpu(&mut self, frame: &eframe::Frame, msaa: u32) {
        let Some(rs) = frame.wgpu_render_state() else {
            return;
        };
        let mut renderer = rs.renderer.write();
        let resources = &mut renderer.callback_resources;
        if resources.get::<MapGpu>().is_none() {
            resources.insert(MapGpu::new(&rs.device, rs.target_format, msaa));
        }
        self.gpu_ready = true;
        let gpu = resources.get_mut::<MapGpu>().expect("az önce eklendi");
        for (lod, map) in self.pending.drain(..) {
            gpu.upload(&rs.device, lod, &map);
            self.loaded[lod] = gpu.has_lod(lod);
            log::info!("harita yüklendi: {}", LOD_FILES[lod]);
        }
        if let Some(grid) = self.grid_pending.take() {
            if resources.get::<grid_gpu::GridGpu>().is_none() {
                resources.insert(grid_gpu::GridGpu::new(&rs.device, rs.target_format, msaa));
            }
            let g = resources
                .get_mut::<grid_gpu::GridGpu>()
                .expect("az önce eklendi");
            match grid {
                Some(grid) => g.upload(&rs.device, &rs.queue, &grid),
                None => g.loaded = None,
            }
            self.grid_spec = g.loaded.as_ref().map(|(_, s)| *s);
        }
        if let Some(ground) = self.ground_pending.take() {
            if resources.get::<ground_gpu::GroundGpu>().is_none() {
                resources.insert(ground_gpu::GroundGpu::new(
                    &rs.device,
                    rs.target_format,
                    msaa,
                ));
            }
            let g = resources
                .get_mut::<ground_gpu::GroundGpu>()
                .expect("az önce eklendi");
            match ground {
                Some((liq, shake)) => g.upload(&rs.device, &rs.queue, &liq, &shake),
                None => g.loaded = None,
            }
            self.ground_spec = g.loaded.as_ref().map(|(_, s)| *s);
        }
        if let Some(lines) = self.lines_pending.take() {
            if resources.get::<region_lines::RegionLinesGpu>().is_none() {
                resources.insert(region_lines::RegionLinesGpu::new(
                    &rs.device,
                    rs.target_format,
                    msaa,
                ));
            }
            let g = resources
                .get_mut::<region_lines::RegionLinesGpu>()
                .expect("az önce eklendi");
            g.upload(&rs.device, lines.as_deref());
            self.line_ranges = lines.as_ref().map(|l| l.ranges.clone()).unwrap_or_default();
            self.fill_ranges = lines.map(|l| l.fill_ranges.clone()).unwrap_or_default();
        }
    }

    /// Ölçeğe uygun ve yüklenmiş en iyi ayrıntı düzeyi.
    fn pick_lod(&self) -> Option<usize> {
        let wanted = if self.view.scale >= LOD_10M_FROM {
            2
        } else if self.view.scale >= LOD_50M_FROM {
            1
        } else {
            0
        };
        // İstenen yüklenmediyse en yakın yüklenmiş olanı kullan.
        (0..=wanted)
            .rev()
            .chain(wanted + 1..3)
            .find(|&l| self.loaded[l])
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, frame: &eframe::Frame, msaa: u32) -> MapOutput {
        self.poll_loader();
        self.sync_gpu(frame, msaa);
        if matches!(self.load, Load::Loading(_)) {
            // Yükleme bitene kadar kanal kontrol edilsin.
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(50));
        }

        let rect = ui.max_rect();
        let response = ui.allocate_rect(rect, Sense::click_and_drag());
        self.view.normalize(rect);

        // Girdi: sürükle = kaydır, tekerlek / iki parmak = yakınlaştır.
        if response.dragged() {
            self.view.pan(rect, response.drag_delta());
        }
        if let Some(p) = response.hover_pos() {
            let (scroll, zoom) = ui.input(|i| (i.smooth_scroll_delta.y, i.zoom_delta()));
            let factor = f64::from(zoom) * (f64::from(scroll) * 0.0025).exp();
            if (factor - 1.0).abs() > 1e-6 {
                self.view.zoom_at(rect, p, factor);
            }
        }

        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, colors::OCEAN);
        if let Some(lod) = self.pick_lod() {
            painter.add(gpu::paint_callback(rect, self.paint(rect, lod)));
        }

        let to_lonlat = |p: Pos2| {
            let w = self.view.screen_to_world(rect, p);
            mercator::from_unit(w[0], w[1])
        };
        MapOutput {
            hover: response.hover_pos().map(to_lonlat),
            clicked: if response.clicked() {
                response.interact_pointer_pos().map(to_lonlat)
            } else {
                None
            },
        }
    }

    fn paint(&self, rect: Rect, lod: usize) -> MapPaint {
        let v = &self.view;
        let (first_copy, n_copies) = v.copies(rect);
        let uniforms = |color: Color32| Uniforms {
            scale: [
                (2.0 * v.scale / f64::from(rect.width())) as f32,
                (-2.0 * v.scale / f64::from(rect.height())) as f32,
            ],
            center: [v.center[0] as f32, v.center[1] as f32],
            color: color.to_normalized_gamma_f32(),
            first_copy: first_copy as f32,
            _pad: [0.0; 3],
        };
        let mut layers = vec![
            (LayerId::Land, colors::LAND),
            (LayerId::Lakes, colors::OCEAN),
        ];
        if v.scale >= ADMIN1_FROM {
            layers.push((LayerId::Admin1, colors::ADMIN1));
        }
        layers.push((LayerId::Admin0, colors::ADMIN0));
        // Izgara, kara ve göllerden sonra (sınırların altında).
        let grid = self
            .grid_spec
            .zip(self.grid_time)
            .map(|(spec, t)| (self.grid_uniforms(rect, &spec, t), 2));
        let ground = self
            .ground_spec
            .zip(self.ground_time)
            .map(|(spec, (t, reduce))| self.ground_uniforms(rect, &spec, t, reduce));
        let (regions, fills) = self.region_draws(rect);
        MapPaint {
            ground,
            lod,
            regions,
            fills,
            // Dolgu, ızgaradan sonra (kara, göller, ızgara, dolgu, sınırlar).
            fills_before: 2,
            region_copies: 0..n_copies + 2,
            grid,
            // Izgara [0, 1) dışına taşabilir: bir kopya öncesi ve sonrası da.
            copies: if self.grid_spec.is_some() {
                0..n_copies + 2
            } else {
                0..0
            },
            draws: layers
                .into_iter()
                .map(|(layer, color)| Draw {
                    layer,
                    uniforms: uniforms(color),
                    copies: 0..n_copies,
                })
                .collect(),
        }
    }

    fn region_draws(
        &self,
        rect: Rect,
    ) -> (Vec<region_lines::RegionDraw>, Vec<region_lines::FillDraw>) {
        let v = &self.view;
        let (first_copy, _) = v.copies(rect);
        let scale = [
            (2.0 * v.scale / f64::from(rect.width())) as f32,
            (-2.0 * v.scale / f64::from(rect.height())) as f32,
        ];
        let center = [v.center[0] as f32, v.center[1] as f32];
        let line =
            |range: std::ops::Range<u32>, color: Color32, width: f32| region_lines::RegionDraw {
                range,
                uniforms: region_lines::LineUniforms {
                    scale,
                    center,
                    viewport: [rect.width(), rect.height()],
                    half_width: width / 2.0,
                    first_copy: (first_copy - 1) as f32,
                    color: color.to_normalized_gamma_f32(),
                },
            };
        // Stiller zayıftan güçlüye sıralıdır (güçlü olan üstte çizilir). Sınır
        // aşılırsa en ZAYIF bölgeler atlanır; eskiden çizim listesi sondan
        // kesildiği için en güçlü bölgelerin renkli sınırı kayboluyordu.
        let skip = self
            .region_styles
            .len()
            .saturating_sub(region_lines::MAX_REGIONS);
        let styles = &self.region_styles[skip..];
        let mut lines = Vec::new();
        let mut fills = Vec::new();
        for st in styles {
            if st.fill.a() > 0
                && let Some(r) = self.fill_ranges.get(&st.id)
            {
                fills.push(region_lines::FillDraw {
                    range: r.clone(),
                    uniforms: region_lines::FillUniforms {
                        scale,
                        center,
                        color: st.fill.to_normalized_gamma_f32(),
                        first_copy: (first_copy - 1) as f32,
                        _pad: [0.0; 3],
                    },
                });
            }
        }
        // Önce tüm konturlar, sonra renkli çizgiler: komşu bölgenin konturu
        // bir önceki bölgenin rengini örtmesin.
        for st in styles.iter().filter(|s| s.line_width > 0.0) {
            if let Some(r) = self.line_ranges.get(&st.id) {
                lines.push(line(r.clone(), HALO, st.line_width + HALO_EXTRA));
            }
        }
        for st in styles.iter().filter(|s| s.line_width > 0.0) {
            if let Some(r) = self.line_ranges.get(&st.id) {
                lines.push(line(r.clone(), st.line, st.line_width));
            }
        }
        (lines, fills)
    }

    fn ground_uniforms(
        &self,
        rect: Rect,
        spec: &GridSpec,
        t: f64,
        reduce_motion: bool,
    ) -> ground_gpu::GroundUniforms {
        let g = self.grid_uniforms(rect, spec, t);
        // Ekranda bir hücrenin kenarı (piksel); shader blok düzeyini seçer.
        let cell_px = (spec.cell / 360.0 * self.view.scale) as f32;
        let mut colors = [[0.0; 4]; 6];
        for (k, (_, c)) in crate::ground_style::CLASSES.iter().enumerate() {
            colors[k] = c.to_normalized_gamma_f32();
        }
        ground_gpu::GroundUniforms {
            scale: g.scale,
            center: g.center,
            bbox: g.bbox,
            north: g.north,
            cell: g.cell,
            first_copy: g.first_copy,
            t: g.t,
            fade: if reduce_motion { 0.0 } else { GROUND_FADE_S },
            cell_px,
            _pad: [0.0; 2],
            colors,
        }
    }

    fn grid_uniforms(&self, rect: Rect, spec: &GridSpec, t: f64) -> grid_gpu::GridUniforms {
        use seismira_core::intensity::Shindo;
        let v = &self.view;
        let (first_copy, _) = v.copies(rect);
        let east = spec.west + spec.cell * f64::from(spec.width);
        let south = spec.north - spec.cell * f64::from(spec.height);
        let (x0, y0) = mercator::to_unit(spec.west, spec.north);
        // Doğu kenarı batıya göre açılmış kalsın (180° meridyeni).
        let (x1, y1) = mercator::to_unit(spec.west, south);
        let x1 = x1 + (east - spec.west) / 360.0;
        let mut colors = [[0.0; 4]; 10];
        for (k, s) in Shindo::ALL.iter().enumerate() {
            colors[k] = crate::shindo_style::color(*s).to_normalized_gamma_f32();
        }
        grid_gpu::GridUniforms {
            scale: [
                (2.0 * v.scale / f64::from(rect.width())) as f32,
                (-2.0 * v.scale / f64::from(rect.height())) as f32,
            ],
            center: [v.center[0] as f32, v.center[1] as f32],
            bbox: [x0 as f32, y0 as f32, x1 as f32, y1 as f32],
            north: spec.north as f32,
            cell: spec.cell as f32,
            // Izgara bir kopya önceden başlayabilir (batı kenarı < −180°).
            first_copy: (first_copy - 1) as f32,
            t: t as f32,
            opacity: GRID_OPACITY,
            _pad: [0.0; 3],
            colors,
        }
    }

    /// Merkez üssü işareti (×), görünen tüm dünya kopyalarında.
    pub fn draw_epicenter(&self, ui: &egui::Ui, rect: Rect, lonlat: (f64, f64)) {
        let (x, y) = mercator::to_unit(lonlat.0, lonlat.1);
        let (first, n) = self.view.copies(rect);
        let painter = ui.painter_at(rect);
        let r = 7.0;
        for k in first..first + n as i32 {
            let c = self.view.world_to_screen(rect, [x + f64::from(k), y]);
            for (a, b) in [
                (Vec2::new(-r, -r), Vec2::new(r, r)),
                (Vec2::new(-r, r), Vec2::new(r, -r)),
            ] {
                painter.line_segment(
                    [c + a, c + b],
                    Stroke::new(5.0, Color32::from_black_alpha(160)),
                );
                painter.line_segment([c + a, c + b], Stroke::new(2.5, colors::EPICENTER));
            }
        }
    }
}

impl MapWidget {
    /// Haritayı bir noktaya ortalar (ölçek değişmez).
    pub fn center_on(&mut self, lonlat: (f64, f64)) {
        let (x, y) = mercator::to_unit(lonlat.0, lonlat.1);
        self.view.center = [x, y];
    }

    /// (boylam, enlem) çizgisini çizer. Ardışık noktalar arasında 180°
    /// sıçraması açılır (x sürekli tutulur), çizgi görünen tüm dünya
    /// kopyalarında tekrarlanır. Kutbu çevreleyen halka Mercator'da kapanmaz;
    /// komşu kopyaya taşan uçlar da çizildiği için kesintisiz görünür.
    pub fn draw_lonlat_line(&self, ui: &egui::Ui, rect: Rect, pts: &[(f64, f64)], stroke: Stroke) {
        self.draw_lonlat_impl(ui, rect, pts, stroke, false);
    }

    /// Kesikli çizgi (ör. model geçerlilik sınırı).
    pub fn draw_lonlat_dashed(
        &self,
        ui: &egui::Ui,
        rect: Rect,
        pts: &[(f64, f64)],
        stroke: Stroke,
    ) {
        self.draw_lonlat_impl(ui, rect, pts, stroke, true);
    }

    fn draw_lonlat_impl(
        &self,
        ui: &egui::Ui,
        rect: Rect,
        pts: &[(f64, f64)],
        stroke: Stroke,
        dashed: bool,
    ) {
        if pts.len() < 2 {
            return;
        }
        let mut world: Vec<[f64; 2]> = Vec::with_capacity(pts.len());
        for &(lon, lat) in pts {
            let (mut x, y) = mercator::to_unit(lon, lat);
            if let Some(prev) = world.last() {
                x += (prev[0] - x).round();
            }
            world.push([x, y]);
        }
        let (first, n) = self.view.copies(rect);
        // Açılmış çizgi [0, 1) dışına taşabilir: bir kopya öncesi/sonrası da.
        let min_x = world.iter().map(|w| w[0]).fold(f64::INFINITY, f64::min);
        let max_x = world.iter().map(|w| w[0]).fold(f64::NEG_INFINITY, f64::max);
        let painter = ui.painter_at(rect);
        let from = first - max_x.ceil() as i32;
        let to = first + n as i32 - min_x.floor() as i32;
        for k in from..=to {
            let screen: Vec<Pos2> = world
                .iter()
                .map(|w| self.view.world_to_screen(rect, [w[0] + f64::from(k), w[1]]))
                .collect();
            let bb = Rect::from_points(&screen);
            if bb.intersects(rect) {
                if dashed {
                    painter.extend(egui::Shape::dashed_line(&screen, stroke, 7.0, 5.0));
                } else {
                    painter.add(egui::Shape::line(screen, stroke));
                }
            }
        }
    }
}

impl MapWidget {
    /// Dünya genişliği (point); ayrıntı ve etiket yoğunluğu kararları için.
    pub fn scale(&self) -> f64 {
        self.view.scale
    }

    /// (boylam, enlem) → ekran; görünüm merkezine en yakın dünya kopyasında.
    pub fn to_screen(&self, rect: Rect, lonlat: (f64, f64)) -> Pos2 {
        let (x, y) = mercator::to_unit(lonlat.0, lonlat.1);
        let x = x + (self.view.center[0] - x).round();
        self.view.world_to_screen(rect, [x, y])
    }
}
