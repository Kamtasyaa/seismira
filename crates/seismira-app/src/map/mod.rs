//! Harita bileşeni: kaydırma, yakınlaştırma, yatay tekrar, merkez üssü seçimi.

mod gpu;
pub mod view;

use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, Vec2};
use seismira_core::mapfile::MapFile;
use seismira_core::mercator;
use std::path::PathBuf;
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
}

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
        }
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
        MapPaint {
            lod,
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
                painter.add(egui::Shape::line(screen, stroke));
            }
        }
    }

    /// Doğrultu önizlemesi: merkez üssünden fay doğrultusu yönünde ince
    /// kesikli çizgi (Bölüm 6). Yalnızca YÖN gösterir, uzunluğu ekran
    /// sabitidir; kırılma uzunluğu Faz 3'te kaynak geometrisiyle gelir.
    pub fn draw_strike_preview(
        &self,
        ui: &egui::Ui,
        rect: Rect,
        epicenter: (f64, f64),
        strike: f64,
    ) {
        // Yerel yönü bulmak için doğrultu boyunca 1 km ileri/geri noktalar.
        let ahead = seismira_core::geo::destination(epicenter, strike, 1.0);
        let (x0, y0) = mercator::to_unit(epicenter.0, epicenter.1);
        let (mut x1, y1) = mercator::to_unit(ahead.0, ahead.1);
        x1 += (x0 - x1).round();
        let dir = Vec2::new((x1 - x0) as f32, (y1 - y0) as f32).normalized();
        if !dir.x.is_finite() {
            return;
        }
        let (first, n) = self.view.copies(rect);
        let painter = ui.painter_at(rect);
        let half = 60.0;
        for k in first..first + n as i32 {
            let c = self.view.world_to_screen(rect, [x0 + f64::from(k), y0]);
            let pts = [c - dir * half, c + dir * half];
            painter.add(egui::Shape::dashed_line(
                &pts,
                Stroke::new(1.5, Color32::from_rgba_unmultiplied(255, 255, 255, 150)),
                6.0,
                4.0,
            ));
        }
    }
}
