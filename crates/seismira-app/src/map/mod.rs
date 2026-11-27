//! Harita bileşeni: kaydırma, yakınlaştırma, yatay tekrar, merkez üssü seçimi.

mod gpu;
mod grid_gpu;
mod region_lines;
pub mod view;

pub use region_lines::RegionLines;

use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, Vec2};
use seismira_core::grid::{GridSpec, ShakeGrid};

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
    pub const LAND: Color32 = crate::theme::MAP_LAND;
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
    /// Bu karede çift tıklandı (fay izi çizimini bitirir).
    pub double_clicked: bool,
    /// Sağ tıklanan nokta (tepki spektrumu noktası).
    pub secondary_clicked: Option<(f64, f64)>,
}

/// Kesirli "satır" birimli kaydırmayı noktaya çevirme (egui'nin satır
/// yüksekliğine yakın).
const LINE_POINTS: f32 = 40.0;
/// Sağ tuşla sürüklemede nokta başına yakınlaştırma üssü (100 nokta ≈ ×2,7).
const DRAG_ZOOM_PER_POINT: f64 = 0.01;
/// Aynısı, imleç kilitliyken ham (hızlandırılmamış) hareket birimi için.
/// Dokunmatik yüzeyin ham birimi ekran noktasından büyük; kullanıcı
/// "parmak hareketine göre fazla" dedi (2026-11-27): 0,01 → 0,004 → 0,002
/// (kullanıcının GNOME kaydırma / sıkıştırma çarpanı 0,20 ile uyumlu; o
/// çarpan ham harekete uygulanmaz).
const RAW_DRAG_ZOOM_PER_UNIT: f64 = 0.002;
/// Fare tekerleğinin bir çentiği için yakınlaştırma üssü (önceki
/// davranış: 50 nokta × 0,0025 ≈ 0,125).
const WHEEL_ZOOM_PER_LINE: f64 = 0.125;

pub struct MapWidget {
    view: MapView,
    /// Sağ tuşla sürükleyerek yakınlaştırmanın çapası (sürüklemenin
    /// başladığı nokta; Google Earth davranışı).
    zoom_anchor: Option<Pos2>,
    load: Load,
    /// Hangi ayrıntı düzeyleri bellekte, GPU'ya yüklenmeyi bekliyor.
    pending: Vec<(usize, MapFile)>,
    loaded: [bool; 3],
    errors: Vec<String>,
    gpu_ready: bool,
    /// GPU'ya yüklenmeyi bekleyen sarsıntı ızgarası (`Some(None)`: kaldır).
    grid_pending: Option<Option<Arc<ShakeGrid>>>,
    /// Son verilen ızgara (ölçek değişince yeniden yüklemek için).
    grid_current: Option<Arc<ShakeGrid>>,
    /// Şiddet ölçeği ve (MMI için) Mw.
    scale: (crate::mmi_style::Scale, f64),
    /// Yüklü ızgaranın geometrisi.
    grid_spec: Option<GridSpec>,
    /// Bu karede ızgara gösterilecekse simülasyon zamanı.
    grid_time: Option<f64>,
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
            zoom_anchor: None,
            errors: Vec::new(),
            gpu_ready: false,
            grid_pending: None,
            grid_current: None,
            scale: (crate::mmi_style::Scale::Shindo, 0.0),
            grid_spec: None,
            grid_time: None,
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
        self.grid_current.clone_from(&grid);
        self.grid_pending = Some(grid);
    }

    /// Izgaranın gösterileceği şiddet ölçeği; MMI için depremin Mw'si
    /// (Worden vd. 2012 büyüklük terimi). Değişince ızgara yeniden yüklenir.
    pub fn set_intensity_scale(&mut self, scale: crate::mmi_style::Scale, mw: f64) {
        if self.scale != (scale, mw) {
            self.scale = (scale, mw);
            if self.grid_current.is_some() {
                self.grid_pending = Some(self.grid_current.clone());
            }
        }
    }

    /// Bu karede ızgara gösterilecekse simülasyon zamanı.
    pub fn set_grid_time(&mut self, t: Option<f64>) {
        self.grid_time = t;
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
                Some(grid) => g.upload(&rs.device, &rs.queue, &grid, self.scale),
                None => g.loaded = None,
            }
            self.grid_spec = g.loaded.as_ref().map(|(_, s)| *s);
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

        // Girdi (dizüstü / dokunmatik yüzey uyumu, 2026-11-26 kullanıcı
        // isteği): sürükle = kaydır; fare tekerleği = yakınlaştır; dokunmatik
        // yüzeyde iki parmak kaydırma = her yöne kaydır; iki parmak sıkıştırma
        // (macOS, Windows) ya da Ctrl + iki parmak / tekerlek = yakınlaştır.
        // Tekerlek / dokunmatik yüzey ayrımı: winit 0.30 kaynağı yalnızca
        // macOS'ta söylüyor; Wayland'de tekerlek "satır", dokunmatik yüzey
        // "nokta" birimiyle gelir; Windows'ta dokunmatik yüzey kesirli satır
        // verir (tam sayı satır = tekerlek). Linux'ta sıkıştırma winit
        // 0.30'da iletilmiyor: Ctrl + iki parmak ya da + / − düğmeleri.
        // Sol tuş (ya da tek parmak basılı) sürükleme = kaydır; sağ tuş (iki
        // parmak basılı dokunmatik yüzey) sürükleme = yakınlaştır: yukarı
        // yakın, aşağı uzak, çapa sürüklemenin başladığı nokta (Google
        // Earth; kullanıcı isteği 2026-11-26).
        // İmleç sürükleme boyunca yerinde kalır (kullanıcı isteği
        // 2026-11-27): imleç kilitlenir (Wayland: pointer-constraints) ve
        // yakınlaştırma ham fare hareketinden (Event::MouseMoved) hesaplanır.
        // Kilit desteklenmezse (ör. X11) ya da ham hareket gelmezse imleç
        // hareket eder ve sürükleme farkı kullanılır.
        if response.drag_started_by(egui::PointerButton::Secondary) {
            self.zoom_anchor = response.interact_pointer_pos();
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::CursorGrab(egui::CursorGrab::Locked));
        }
        if response.dragged_by(egui::PointerButton::Secondary) {
            let raw: Option<f32> = ui.input(|i| {
                i.raw
                    .events
                    .iter()
                    .filter_map(|e| match e {
                        egui::Event::MouseMoved(d) => Some(d.y),
                        _ => None,
                    })
                    .reduce(|a, b| a + b)
                    .map(|y| y / i.pixels_per_point)
            });
            let k = match raw {
                Some(y) => f64::from(y) * RAW_DRAG_ZOOM_PER_UNIT,
                None => f64::from(response.drag_delta().y) * DRAG_ZOOM_PER_POINT,
            };
            let at = self.zoom_anchor.unwrap_or(rect.center());
            self.view.zoom_at(rect, at, (-k).exp());
        } else if response.dragged() {
            self.view.pan(rect, response.drag_delta());
        }
        if response.drag_stopped() && self.zoom_anchor.take().is_some() {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::CursorGrab(egui::CursorGrab::None));
        }
        if let Some(p) = response.hover_pos() {
            let (wheel, pan, zoom) = ui.input(|i| {
                let (mut wheel, mut pan) = (0.0f64, egui::Vec2::ZERO);
                for e in &i.raw.events {
                    if let egui::Event::MouseWheel {
                        unit,
                        delta,
                        modifiers,
                        ..
                    } = e
                    {
                        if modifiers.command || modifiers.ctrl {
                            continue; // egui bunu zoom_delta'ya çevirir
                        }
                        let integral = delta.x.fract() == 0.0 && delta.y.fract() == 0.0;
                        match unit {
                            egui::MouseWheelUnit::Line if integral => {
                                wheel += f64::from(delta.y);
                            }
                            egui::MouseWheelUnit::Line => pan += *delta * LINE_POINTS,
                            egui::MouseWheelUnit::Point => pan += *delta,
                            egui::MouseWheelUnit::Page => wheel += f64::from(delta.y) * 3.0,
                        }
                    }
                }
                (wheel, pan, i.zoom_delta())
            });
            if pan != egui::Vec2::ZERO {
                self.view.pan(rect, pan);
            }
            let factor = f64::from(zoom) * (wheel * WHEEL_ZOOM_PER_LINE).exp();
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
            double_clicked: response.double_clicked(),
            secondary_clicked: if response.secondary_clicked() {
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
        let (regions, fills) = self.region_draws(rect);
        MapPaint {
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
        match self.scale.0 {
            crate::mmi_style::Scale::Shindo => {
                for (k, s) in Shindo::ALL.iter().enumerate() {
                    colors[k] = crate::shindo_style::color(*s).to_normalized_gamma_f32();
                }
            }
            // Doku sınıfı = MMI − 1 (I → 0, çizilmez).
            crate::mmi_style::Scale::Mmi => {
                for (k, c) in colors.iter_mut().enumerate() {
                    if let Some(col) = crate::mmi_style::color(k as u8 + 1) {
                        *c = col.to_normalized_gamma_f32();
                    }
                }
            }
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

    /// Bir noktanın görünen tüm dünya kopyalarındaki ekran konumları.
    pub fn screen_positions(&self, rect: Rect, lonlat: (f64, f64)) -> Vec<Pos2> {
        let (x, y) = mercator::to_unit(lonlat.0, lonlat.1);
        let (first, n) = self.view.copies(rect);
        (first..first + n as i32)
            .map(|k| self.view.world_to_screen(rect, [x + f64::from(k), y]))
            .filter(|p| rect.contains(*p))
            .collect()
    }

    /// Kısa bir (boylam, enlem) parçasının görünen dünya kopyalarındaki ekran
    /// uçları. 180° meridyenini kesen parça kısa yoldan çizilir (x sürekli).
    pub fn segment_positions(&self, rect: Rect, a: (f64, f64), b: (f64, f64)) -> Vec<[Pos2; 2]> {
        let (xa, ya) = mercator::to_unit(a.0, a.1);
        let (xb, yb) = mercator::to_unit(b.0, b.1);
        let xb = xb + (xa - xb).round();
        let (first, n) = self.view.copies(rect);
        (first..first + n as i32)
            .map(|k| {
                let dx = f64::from(k);
                [
                    self.view.world_to_screen(rect, [xa + dx, ya]),
                    self.view.world_to_screen(rect, [xb + dx, yb]),
                ]
            })
            .filter(|[p, q]| Rect::from_two_pos(*p, *q).intersects(rect))
            .collect()
    }

    /// Zemin yenilmesi kareleri (Bölüm 2.6): içi boş kırmızı çerçeve ve
    /// köşesinde tür harfi. `boxes`: (batı, kuzey, doğu, güney) derece,
    /// opaklık 0–1, harf, harf sağ köşede mi.
    pub fn draw_ground_boxes(
        &self,
        ui: &egui::Ui,
        rect: Rect,
        boxes: &[([f64; 4], f32, &str, bool)],
    ) {
        let (first, n) = self.view.copies(rect);
        let painter = ui.painter_at(rect);
        for &([w, nlat, e, slat], alpha, letter, right) in boxes {
            let (x0, y0) = mercator::to_unit(w, nlat);
            let (x1, y1) = mercator::to_unit(e, slat);
            for k in first..first + n as i32 {
                let dx = f64::from(k);
                let r = Rect::from_two_pos(
                    self.view.world_to_screen(rect, [x0 + dx, y0]),
                    self.view.world_to_screen(rect, [x1 + dx, y1]),
                );
                if !r.intersects(rect) {
                    continue;
                }
                crate::ground_style::draw_box(&painter, r, alpha, letter, right);
            }
        }
    }
}

impl MapWidget {
    /// Haritayı bir noktaya ortalar (ölçek değişmez).
    /// Haritanın sağ kenarında + / − düğmeleri (dokunmatik yüzeyde sıkıştırma
    /// olmayan sistemler için) ve klavyede + / − (metin kutusu odakta
    /// değilken). Harita üstündeki katmanlardan sonra çağrılır.
    pub fn zoom_buttons(&mut self, ui: &mut egui::Ui, rect: Rect, labels: (&str, &str)) {
        let size = egui::vec2(30.0, 30.0);
        let x = rect.right() - size.x - 10.0;
        let y = rect.center().y - size.y - 3.0;
        let step = 1.6;
        let mut factor = 1.0;
        for (i, (text, tip, f)) in [("+", labels.0, step), ("−", labels.1, 1.0 / step)]
            .into_iter()
            .enumerate()
        {
            let r = Rect::from_min_size(egui::pos2(x, y + i as f32 * (size.y + 6.0)), size);
            let b = ui.put(r, egui::Button::new(egui::RichText::new(text).size(18.0)));
            if b.on_hover_text(tip).clicked() {
                factor *= f;
            }
        }
        let typing = ui.ctx().memory(|m| m.focused().is_some());
        if !typing {
            let (plus, minus) = ui.input(|i| {
                (
                    i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals),
                    i.key_pressed(egui::Key::Minus),
                )
            });
            if plus {
                factor *= step;
            }
            if minus {
                factor /= step;
            }
        }
        if factor != 1.0 {
            self.view.zoom_at(rect, rect.center(), factor);
        }
    }

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
