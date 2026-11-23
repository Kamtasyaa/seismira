//! Arka plan hesabı: şehir başına sarsıntı ve varış zamanları, ardından
//! sarsıntı ızgarası (önce kaba, sonra ince; CLAUDE.md Bölüm 11).
//!
//! Arayüz iş parçacığı beklemez (CLAUDE.md Bölüm 11): hesap ayrı bir iş
//! parçacığında yapılır, sonuç kanalla gelir. Parametre değişince eski iş
//! `cancel` bayrağıyla anında durdurulur.

use seismira_core::cities::City;
use seismira_core::exposure::Exposure;
use seismira_core::quake::Quake;
use seismira_core::shaking::{Scenario, SiteResult};
use seismira_core::site::{Site, Vs30Map};
use seismira_core::slab::Slab2;
use seismira_core::traveltime::TravelTimes;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError, channel};

/// Vs30 haritası (dosya belleğe eşlenir; yalnızca gereken döşemeler okunur).
pub type Vs30 = Vs30Map<memmap2::Mmap>;

/// Bölge kimliği rasterı ve bölge bilgileri (Natural Earth Admin-1).
pub struct Regions {
    pub map: seismira_core::region::RegionMap<Vec<u8>>,
    pub infos: Vec<seismira_core::region::RegionInfo>,
    /// Sınır halkaları (`admin1.sout`, ~6 MB): ilk gerektiğinde arka
    /// planda açılır.
    outlines_path: std::path::PathBuf,
    outlines: std::sync::OnceLock<Option<seismira_core::region::Outlines>>,
}

impl Regions {
    /// `data/regions/admin1.sras` + `.sreg` (küçük: ~3,5 MB, belleğe alınır).
    pub fn load(dir: &std::path::Path) -> Result<Arc<Self>, String> {
        let raster = dir.join("admin1.sras");
        let info = dir.join("admin1.sreg");
        let err = |p: &std::path::Path, e: std::io::Error| format!("{}: {e}", p.display());
        let map = std::fs::read(&raster)
            .and_then(seismira_core::region::RegionMap::open)
            .map_err(|e| err(&raster, e))?;
        let infos = std::fs::File::open(&info)
            .and_then(|f| seismira_core::region::read_infos(&mut std::io::BufReader::new(f)))
            .map_err(|e| err(&info, e))?;
        Ok(Arc::new(Self {
            map,
            infos,
            outlines_path: dir.join("admin1.sout"),
            outlines: std::sync::OnceLock::new(),
        }))
    }

    /// Sınır halkaları (ilk çağrıda dosyadan okunur; arayüz iş parçacığında
    /// çağrılmamalı).
    pub fn outlines(&self) -> Option<&seismira_core::region::Outlines> {
        self.outlines
            .get_or_init(|| {
                let started = std::time::Instant::now();
                let r = std::fs::File::open(&self.outlines_path)
                    .and_then(|f| {
                        seismira_core::region::read_outlines(&mut std::io::BufReader::new(f))
                    })
                    .inspect_err(|e| log::error!("{}: {e}", self.outlines_path.display()))
                    .ok();
                log::info!(
                    "bölge sınırları okundu ({} ms)",
                    started.elapsed().as_millis()
                );
                r
            })
            .as_ref()
    }

    pub fn info(
        &self,
        id: seismira_core::region::RegionId,
    ) -> Option<&seismira_core::region::RegionInfo> {
        self.infos.get(usize::from(id).checked_sub(1)?)
    }
}

/// Belleğe eşlenmiş bir `.sras` rasterı.
pub type MappedRaster = seismira_core::raster::Raster<memmap2::Mmap>;

pub(crate) fn map_raster(path: &std::path::Path) -> Result<MappedRaster, String> {
    let err = |e: std::io::Error| format!("{}: {e}", path.display());
    let file = std::fs::File::open(path).map_err(err)?;
    // SAFETY: load_vs30 ile aynı (salt okunur veri dosyası).
    #[allow(unsafe_code)]
    let map = unsafe { memmap2::Mmap::map(&file) }.map_err(err)?;
    MappedRaster::open(map).map_err(err)
}

/// Zemin yenilmesi girdileri (Faz 7; docs/bilim.md 7.9): sıvılaşmanın
/// önceden hesaplanmış zemin kısmı (`data/ground/liq_static.sras`,
/// `seismira-data liq-static`) belleğe eşlenir, yalnızca deprem alanının
/// döşemeleri okunur.
///
/// Heyelan (`ls_static.sras`, `seismira-data ls-static`) isteğe bağlıdır:
/// yoksa (ör. litoloji izni gelmediği için dağıtılan pakette) heyelan
/// katmanı gösterilmez.
pub struct Ground {
    liq: MappedRaster,
    /// liq_x, liq_xf (yoksa `None`: yalnızca kaba kod), liq_cls.
    layers: (usize, Option<usize>, usize),
    /// Heyelan: raster ve ls_a, ls_q0–3 katmanları.
    landslide: Option<(MappedRaster, [usize; 5])>,
}

/// İki rasterın ızgarası aynı mı (pencereler hizalı olmalı)?
fn same_grid(r: &seismira_core::raster::Grid, g: &seismira_core::raster::Grid) -> bool {
    r.west == g.west
        && r.north == g.north
        && r.cell == g.cell
        && r.width == g.width
        && r.height == g.height
}

impl Ground {
    /// `vs30`: ızgaraların aynı olduğu denetlenir (pencereler hizalı olmalı).
    pub fn load(dir: &std::path::Path, vs30: &Vs30) -> Result<Arc<Self>, String> {
        use seismira_core::groundfailure::static_layer;
        let liq = map_raster(&dir.join(static_layer::FILE))?;
        let (r, g) = (liq.grid, vs30.grid());
        if !same_grid(&r, &g) {
            return Err(format!(
                "{} Vs30 ızgarasıyla aynı değil",
                static_layer::FILE
            ));
        }
        let need = |n: &str| {
            liq.layer(n)
                .ok_or_else(|| format!("{}: '{n}' katmanı yok", static_layer::FILE))
        };
        let layers = (need("liq_x")?, liq.layer("liq_xf"), need("liq_cls")?);
        let landslide = match Self::load_landslide(dir, &g) {
            Ok(l) => Some(l),
            Err(e) => {
                log::warn!("heyelan katmanı yok: {e}");
                None
            }
        };
        Ok(Arc::new(Self {
            liq,
            layers,
            landslide,
        }))
    }

    fn load_landslide(
        dir: &std::path::Path,
        g: &seismira_core::raster::Grid,
    ) -> Result<(MappedRaster, [usize; 5]), String> {
        use seismira_core::groundfailure::landslide_static as ls;
        let r = map_raster(&dir.join(ls::FILE))?;
        if !same_grid(&r.grid, g) {
            return Err(format!("{} Vs30 ızgarasıyla aynı değil", ls::FILE));
        }
        let want = ls::layers();
        let mut idx = [0usize; 5];
        for (k, info) in want.iter().enumerate() {
            let li = r
                .layer(&info.name)
                .ok_or_else(|| format!("{}: '{}' katmanı yok", ls::FILE, info.name))?;
            if r.layers[li] != *info {
                return Err(format!("{}: '{}' kodlaması farklı", ls::FILE, info.name));
            }
            idx[k] = li;
        }
        Ok((r, idx))
    }

    pub fn has_landslide(&self) -> bool {
        self.landslide.is_some()
    }

    /// Alan penceresinde heyelan kaplaması; katman yoksa ya da okuma
    /// hatasında `None`.
    fn landslide(
        &self,
        shake: &seismira_core::grid::ShakeGrid,
        domain: &seismira_core::grid::Domain,
        step: u32,
        cancel: &AtomicBool,
    ) -> Option<seismira_core::groundfailure::landslide_grid::LandslideGrid> {
        use seismira_core::groundfailure::landslide_grid::{StaticInputs, compute};
        use seismira_core::groundfailure::landslide_static::MASKED;
        let (r, idx) = self.landslide.as_ref()?;
        let read = |li: usize| -> Option<Vec<u8>> {
            r.window(li, domain.col0, domain.row0, domain.w, domain.h, MASKED)
                .inspect_err(|e| log::error!("heyelan katmanı okunamadı: {e}"))
                .ok()
        };
        let a = read(idx[0])?;
        let q: Vec<Vec<u8>> = idx[1..].iter().map(|&li| read(li)).collect::<Option<_>>()?;
        let inputs = StaticInputs {
            a: &a,
            q: [&q[0], &q[1], &q[2], &q[3]],
        };
        compute(shake, domain, step, &inputs, cancel)
    }

    /// Alan penceresinde sıvılaşma kaplaması; okuma hatasında `None`
    /// (günlüğe yazılır).
    fn liquefaction(
        &self,
        shake: &seismira_core::grid::ShakeGrid,
        domain: &seismira_core::grid::Domain,
        step: u32,
        mw: f64,
        cancel: &AtomicBool,
    ) -> Option<seismira_core::groundfailure::liquefaction_grid::LiquefactionGrid> {
        use seismira_core::groundfailure::liquefaction_grid::{Inputs, StaticInputs, compute};
        use seismira_core::groundfailure::static_layer::MASKED;
        let read = |li: usize| -> Option<Vec<u8>> {
            self.liq
                .window(li, domain.col0, domain.row0, domain.w, domain.h, MASKED)
                .inspect_err(|e| log::error!("sıvılaşma katmanı okunamadı: {e}"))
                .ok()
        };
        let (ix, ifine, icls) = self.layers;
        let x = read(ix)?;
        let fine = match ifine {
            Some(li) => Some(read(li)?),
            None => None,
        };
        let cls = read(icls)?;
        let inputs = Inputs::Static(StaticInputs {
            x: &x,
            fine: fine.as_deref(),
            cls: &cls,
        });
        compute(shake, domain, step, mw, &inputs, cancel)
    }
}

/// Bir bölgenin sonucu: ızgara özeti ve bölgedeki en güçlü sarsılan şehir.
#[derive(Clone, Copy, Debug)]
pub struct RegionResult {
    pub summary: seismira_core::region::RegionSummary,
    pub best_city: Option<usize>,
}

/// `data/vs30/vs30.sras` dosyasını açar.
pub fn load_vs30(path: &std::path::Path) -> Result<Arc<Vs30>, String> {
    let err = |e: std::io::Error| format!("{}: {e}", path.display());
    let file = std::fs::File::open(path).map_err(err)?;
    // SAFETY: dosya yalnızca okunur ve uygulama çalışırken değiştirilmesi
    // beklenmez (veri klasörü). Değişirse en kötü durumda okunan döşeme
    // bozuk görünür; zstd çözümü ve boyut denetimi bunu hata olarak döndürür.
    #[allow(unsafe_code)]
    let map = unsafe { memmap2::Mmap::map(&file) }.map_err(err)?;
    Vs30Map::open(map).map(Arc::new).map_err(err)
}

/// Bir depremin tüm şehirler için sonuçları (şehir listesiyle aynı sıra).
pub struct CityResults {
    pub sites: Vec<SiteResult>,
    /// Hissedilme sınırı (derece); halkalar ötesinde soluk çizilir.
    pub felt_radius_deg: Option<f64>,
    /// Yer hareketi modelinin mesafe sınırı (Rrup eş-uzaklık eğrisi);
    /// ötesinde şiddet gösterilmez (Bölüm 7.4 "model dışı").
    pub model_limit: Option<Vec<(f64, f64)>>,
}

/// Tahmini bina hasarı (Faz 6): bölgeler (kimliğe göre sıralı) ve
/// şehirler (şehir listesiyle aynı sıra; bulunduğu bölgenin bina karışımıyla
/// hasar durumu ORANLARI: medyan, −1τ, +1τ).
pub struct DamageResults {
    pub regions: Vec<seismira_core::damage::RegionDamage>,
    pub cities: Vec<Option<[seismira_core::exposure::Damage; 3]>>,
    /// GEM ortalama kayıp oranı (Faz 9; ESRM20 stoku OLMAYAN bölgeler,
    /// kimliğe göre sıralı).
    pub losses: Vec<seismira_core::damage::RegionLoss>,
}

impl DamageResults {
    pub fn region(
        &self,
        id: seismira_core::region::RegionId,
    ) -> Option<&seismira_core::damage::RegionDamage> {
        self.regions
            .binary_search_by_key(&id, |r| r.id)
            .ok()
            .map(|i| &self.regions[i])
    }

    pub fn loss(
        &self,
        id: seismira_core::region::RegionId,
    ) -> Option<&seismira_core::damage::RegionLoss> {
        self.losses
            .binary_search_by_key(&id, |r| r.id)
            .ok()
            .map(|i| &self.losses[i])
    }
}

/// Arka plan işinin ürettiği sonuçlar (geliş sırasıyla).
pub enum JobMsg {
    Cities(CityResults),
    /// Sarsıntı ızgarası (önce kaba ön sonuç, sonra ince; sonraki öncekinin yerine geçer).
    Grid(Arc<seismira_core::grid::ShakeGrid>),
    /// İnce ızgaradan bölge özetleri (kimliğe göre sıralı) ve bu
    /// bölgelerin sınır geometrisi.
    Regions(Vec<RegionResult>, Option<Arc<crate::map::RegionLines>>),
    /// Bölge özetlerinden sonra gelir (hesap daha uzun).
    Damage(DamageResults),
    /// Zemin yenilmesi (önce sıvılaşma, sonra heyelan): bölge özetleri ve
    /// haritadaki kırmızı kareler.
    GroundFailure {
        kind: GroundKind,
        regions: Vec<RegionGroundFailure>,
        boxes: Vec<GroundBox>,
    },
    /// Tsunami tahmini (ayrı iş parçacığından).
    Tsunami(crate::tsunami::TsunamiResult),
}

/// Zemin yenilmesi türü.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroundKind {
    Liquefaction = 0,
    Landslide = 1,
}

impl GroundKind {
    pub const ALL: [GroundKind; 2] = [GroundKind::Liquefaction, GroundKind::Landslide];

    /// Çizimde hücrenin sayıldığı en küçük kaplama (USGS görüntüleme
    /// ayarının ilk sınıf sınırı: `zhu_2017_general.ini` 0,005,
    /// `jessee_2018.ini` 0,002). Bölge kartında ilk karenin anı için.
    pub fn min_coverage(self) -> f32 {
        match self {
            GroundKind::Liquefaction => 0.005,
            GroundKind::Landslide => 0.002,
        }
    }

    /// Bir kutunun kare olarak gösterileceği toplam etkilenen alan (km²):
    /// USGS Ground Failure'ın olay düzeyi SARI uyarı eşiği
    /// (groundfailure `utilities.py::get_alert`, main dalı, 2026-11-23
    /// okundu: hazbinLQ = 10, 100, 1000; hazbinLS = 1, 10, 100 km²). Yani
    /// kutu tek başına USGS'te sarı uyarı düzeyine ulaşıyorsa kare çıkar.
    pub fn box_threshold_km2(self) -> f64 {
        match self {
            GroundKind::Liquefaction => 10.0,
            GroundKind::Landslide => 1.0,
        }
    }
}

/// Kutunun kenarı (km; Bölüm 2.6, docs/arayuz.md "kırmızı kareler").
pub const BOX_KM: f64 = 25.0;

/// Tür başına en çok bu kadar kare (eşiği geçenlerin en riskli olanları).
/// Sabit eşik depreme göre ya halı gibi onlarca kare ya da hiç kare
/// veriyordu (2026-11-23 ölçümü, docs/arayuz.md).
pub const MAX_BOXES: usize = 6;

/// Haritadaki bir kırmızı kare: sabit kutu ızgarasının bir kutusu.
#[derive(Clone, Copy, Debug)]
pub struct GroundBox {
    /// Batı, kuzey, doğu, güney (derece).
    pub bounds: [f64; 4],
    /// Kutuda kaplaması çizim eşiğini geçen hücrelerde sarsıntının ilk
    /// bittiği an (s): kare bu anda belirir.
    pub t: f64,
    /// Kutudaki en yüksek kaplamalı hücrenin bölgesi (0: bilinmiyor).
    pub region: seismira_core::region::RegionId,
}

/// Bir bölgenin zemin yenilmesi (sıvılaşma ya da heyelan) özeti: toplam
/// tehlike (Σ kaplama × alan, km²; USGS Ground Failure'ın "aggregate
/// hazard" ölçüsü) ve bölgede ilk karenin belirdiği an (s). Kimliğe göre
/// sıralı.
#[derive(Clone, Copy, Debug)]
pub struct RegionGroundFailure {
    pub id: seismira_core::region::RegionId,
    /// [medyan, −1τ, +1τ].
    pub km2: [f64; 3],
    pub t_first: f64,
}

/// Bölge toplamları (ince ızgaranın temsilci taban hücresinin bölgesine).
/// `cov`: [medyan, −1τ, +1τ] kaplama; `min`: çizilen en küçük oran (ilk
/// karenin anı için).
fn ground_by_region(
    cov: [&[f32]; 3],
    min: f32,
    shake: &seismira_core::grid::ShakeGrid,
    domain: &seismira_core::grid::Domain,
    step: u32,
    ids: &[seismira_core::region::RegionId],
) -> Vec<RegionGroundFailure> {
    use std::collections::BTreeMap;
    const R_KM: f64 = 6371.0088;
    let spec = shake.spec;
    let mut by: BTreeMap<seismira_core::region::RegionId, ([f64; 3], f64)> = BTreeMap::new();
    for row in 0..spec.height {
        let (_, lat) = spec.center(0, row);
        let area = (R_KM * spec.cell.to_radians()).powi(2) * lat.to_radians().cos();
        for col in 0..spec.width {
            let i = (row * spec.width + col) as usize;
            let c = f64::from(cov[0][i]);
            let hi = f64::from(cov[2][i]);
            if hi <= 0.0 {
                continue;
            }
            let bx = (col * step + step / 2).min(domain.w - 1) as usize;
            let by_ = (row * step + step / 2).min(domain.h - 1) as usize;
            let id = ids[by_ * domain.w as usize + bx];
            if id == 0 {
                continue;
            }
            let e = by.entry(id).or_insert(([0.0; 3], f64::INFINITY));
            e.0[0] += c * area;
            e.0[1] += f64::from(cov[1][i]) * area;
            e.0[2] += hi * area;
            if c >= f64::from(min) {
                e.1 = e.1.min(f64::from(shake.t_end[i]));
            }
        }
    }
    by.into_iter()
        .filter(|(_, (_, t))| t.is_finite())
        .map(|(id, (km2, t_first))| RegionGroundFailure { id, km2, t_first })
        .collect()
}

/// Kaplamayı ~[`BOX_KM`]'lik kutulara toplar; eşiği geçen kutuların en
/// çok etkilenen alanlı [`MAX_BOXES`] tanesi döner (belirme anına göre
/// sıralı). Kutular boylamda Δλ, enlemde Δφ = Δλ·cos φ₀
/// (φ₀: alanın orta enlemi): Web Mercator'da o enlemde kare görünür.
fn ground_boxes(
    kind: GroundKind,
    cov: &[f32],
    shake: &seismira_core::grid::ShakeGrid,
    domain: &seismira_core::grid::Domain,
    step: u32,
    ids: Option<&[seismira_core::region::RegionId]>,
) -> Vec<GroundBox> {
    use std::collections::HashMap;
    const R_KM: f64 = 6371.0088;
    let spec = shake.spec;
    let lat0 = spec.north - f64::from(spec.height) * spec.cell / 2.0;
    let dlat = (BOX_KM / R_KM).to_degrees();
    let dlon = dlat / lat0.to_radians().cos().max(0.05);
    // Kutu dizinleri dünya ızgarasına bağlı (alan kaysa da kutular aynı yerde).
    struct Acc {
        km2: f64,
        t: f64,
        best: (f32, seismira_core::region::RegionId),
    }
    let mut boxes: HashMap<(i64, i64), Acc> = HashMap::new();
    let min = kind.min_coverage();
    for row in 0..spec.height {
        let (_, lat) = spec.center(0, row);
        let area = (R_KM * spec.cell.to_radians()).powi(2) * lat.to_radians().cos();
        for col in 0..spec.width {
            let i = (row * spec.width + col) as usize;
            let c = cov[i];
            if c <= 0.0 {
                continue;
            }
            let (lon, lat) = spec.center(col, row);
            let key = ((lon / dlon).floor() as i64, (lat / dlat).floor() as i64);
            let region = ids.map_or(0, |ids| {
                let bx = (col * step + step / 2).min(domain.w - 1) as usize;
                let by = (row * step + step / 2).min(domain.h - 1) as usize;
                ids[by * domain.w as usize + bx]
            });
            let e = boxes.entry(key).or_insert(Acc {
                km2: 0.0,
                t: f64::INFINITY,
                best: (0.0, 0),
            });
            e.km2 += f64::from(c) * area;
            if c >= min {
                e.t = e.t.min(f64::from(shake.t_end[i]));
            }
            if c > e.best.0 && region != 0 {
                e.best = (c, region);
            }
        }
    }
    let mut top: Vec<((i64, i64), Acc)> = boxes
        .into_iter()
        .filter(|(_, a)| a.km2 >= kind.box_threshold_km2() && a.t.is_finite())
        .collect();
    top.sort_by(|a, b| b.1.km2.total_cmp(&a.1.km2));
    top.truncate(MAX_BOXES);
    let mut out: Vec<GroundBox> = top
        .into_iter()
        .map(|((kx, ky), a)| GroundBox {
            bounds: [
                kx as f64 * dlon,
                (ky + 1) as f64 * dlat,
                (kx + 1) as f64 * dlon,
                ky as f64 * dlat,
            ],
            t: a.t,
            region: a.best.1,
        })
        .collect();
    out.sort_by(|a, b| a.t.total_cmp(&b.t));
    out
}

pub struct Job {
    cancel: Arc<AtomicBool>,
    rx: Receiver<JobMsg>,
}

/// Hesabın okuduğu, açılışta yüklenen veri (paylaşılır, kopyalanmaz).
pub struct Datasets {
    pub cities: Arc<Vec<City>>,
    pub city_sites: Option<Arc<Vec<Site>>>,
    pub tt: Arc<TravelTimes>,
    pub vs30: Option<Arc<Vs30>>,
    pub regions: Option<Arc<Regions>>,
    pub slab: Option<Arc<Slab2>>,
    pub exposure: Option<Arc<Exposure>>,
    /// GEM paketi (isteğe bağlı): bölge başına birleşik zafiyet eğrileri.
    pub vuln: Option<Arc<seismira_core::damage::VulnCurves>>,
    pub ground: Option<Arc<Ground>>,
    pub tsunami: Option<Arc<crate::tsunami::TsunamiData>>,
}

impl Job {
    pub fn spawn(quake: Quake, data: Datasets, ctx: eframe::egui::Context) -> Self {
        let Datasets {
            cities,
            city_sites,
            tt,
            vs30,
            regions,
            slab,
            exposure,
            vuln,
            ground,
            tsunami,
        } = data;
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, rx) = channel();
        // Tsunami ayrı iş parçacığında (küresel varış hesabı saniyeler
        // sürebilir; sarsıntı sonuçlarını bekletmesin).
        if let Some(td) = tsunami {
            let (tx, flag, ctx, cities) = (tx.clone(), cancel.clone(), ctx.clone(), cities.clone());
            let _ = std::thread::Builder::new()
                .name("tsunami-hesabı".into())
                .spawn(move || {
                    if let Some(r) = crate::tsunami::run(&quake, &td, &cities, &flag)
                        && tx.send(JobMsg::Tsunami(r)).is_ok()
                    {
                        ctx.request_repaint();
                    }
                });
        }
        let flag = cancel.clone();
        std::thread::Builder::new()
            .name("sarsıntı-hesabı".into())
            .spawn(move || {
                let started = std::time::Instant::now();
                let scenario = Scenario::with_slab(quake, &tt, slab.as_deref());
                if let Some(z) = scenario.slab_zone {
                    log::info!(
                        "Slab2 zonu {}, Parker bölgesi {}",
                        slab.as_deref().map_or("?", |m| m.code(z)),
                        scenario.region.key()
                    );
                }
                let mut sites = Vec::with_capacity(cities.len());
                for (i, c) in cities.iter().enumerate() {
                    if i % 1024 == 0 && flag.load(Ordering::Relaxed) {
                        return; // iptal edildi
                    }
                    sites.push(scenario.site((c.lon, c.lat), || {
                        if let Some(cs) = &city_sites {
                            return cs[i];
                        }
                        let Some(map) = &vs30 else {
                            return Site::reference();
                        };
                        map.site(c.lon, c.lat).unwrap_or_else(|e| {
                            log::error!("Vs30 okunamadı ({}, {}): {e}", c.lon, c.lat);
                            Site::reference()
                        })
                    }));
                }
                let felt_radius_deg = scenario.felt_radius_deg();
                let model_limit = scenario
                    .max_rrup_km()
                    .map(|max| scenario.fault.rrup_contour(max, 180));
                log::info!(
                    "{} şehir hesaplandı ({} ms)",
                    sites.len(),
                    started.elapsed().as_millis()
                );
                let sites_for_regions = sites.clone();
                if tx
                    .send(JobMsg::Cities(CityResults {
                        sites,
                        felt_radius_deg,
                        model_limit,
                    }))
                    .is_err()
                {
                    return;
                }
                ctx.request_repaint();
                if scenario.gmm.is_some() {
                    shake_grid(
                        &scenario,
                        vs30.as_deref(),
                        (
                            regions.as_deref(),
                            exposure.as_deref(),
                            vuln.as_deref(),
                            ground.as_deref(),
                        ),
                        (&cities, &sites_for_regions),
                        &flag,
                        &tx,
                        &ctx,
                    );
                }
            })
            .expect("iş parçacığı başlatılamadı");
        Self { cancel, rx }
    }

    /// Gelen sonuçlar ve iş bitti mi (kanal kapandı).
    pub fn poll(&self) -> (Vec<JobMsg>, bool) {
        let mut out = Vec::new();
        loop {
            match self.rx.try_recv() {
                Ok(m) => out.push(m),
                Err(TryRecvError::Empty) => return (out, false),
                Err(TryRecvError::Disconnected) => return (out, true),
            }
        }
    }
}

impl Drop for Job {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

/// Sarsıntı ızgarası: Vs30 penceresi bir kez okunur, önce kaba sonra ince
/// geçiş gönderilir. Referans makinede Mw 8 için ince geçiş ~0,2 s.
fn shake_grid(
    scenario: &Scenario,
    vs30: Option<&Vs30>,
    (regions, exposure, vuln, ground): (
        Option<&Regions>,
        Option<&Exposure>,
        Option<&seismira_core::damage::VulnCurves>,
        Option<&Ground>,
    ),
    (cities, sites): (&[City], &[SiteResult]),
    cancel: &AtomicBool,
    tx: &std::sync::mpsc::Sender<JobMsg>,
    ctx: &eframe::egui::Context,
) {
    use seismira_core::grid::{self, Base, COARSE_FACTOR, Domain};
    let started = std::time::Instant::now();
    let base = vs30.map_or(Base::GLOBAL_30S, |m| {
        let g = m.grid();
        Base {
            west: g.west,
            north: g.north,
            cell: g.cell,
        }
    });
    let domain = Domain::for_scenario(scenario, base);
    let window = match vs30.map(|m| m.window(domain.col0, domain.row0, domain.w, domain.h)) {
        Some(Ok(w)) => Some(w),
        Some(Err(e)) => {
            log::error!("Vs30 penceresi okunamadı: {e}");
            None
        }
        None => None,
    };
    let step = domain.fine_step();
    for (s, fine) in [(step * COARSE_FACTOR, false), (step, true)] {
        let Some(g) = grid::compute(scenario, &domain, s, window.as_deref(), cancel) else {
            return; // iptal
        };
        let g = Arc::new(g);
        let summaries = if fine {
            regions.and_then(|r| region_results(r, &g, &domain, s, cities, sites))
        } else {
            None
        };
        log::info!(
            "sarsıntı ızgarası {}×{} ({}) {} ms",
            g.spec.width,
            g.spec.height,
            if fine { "ince" } else { "kaba" },
            started.elapsed().as_millis()
        );
        if tx.send(JobMsg::Grid(g.clone())).is_err() {
            return;
        }
        if let Some(r) = summaries {
            let lines = regions.and_then(|reg| reg.outlines()).map(|o| {
                Arc::new(crate::map::RegionLines::build(
                    o,
                    r.iter().map(|x| x.summary.id),
                ))
            });
            if tx.send(JobMsg::Regions(r, lines)).is_err() {
                return;
            }
        }
        ctx.request_repaint();
        if fine && let Some(gr) = ground {
            let t0 = std::time::Instant::now();
            let Some(liq) = gr.liquefaction(&g, &domain, s, scenario.quake.mw, cancel) else {
                return; // iptal ya da okuma hatası (günlükte)
            };
            log::info!(
                "sıvılaşma: {} hücre (genel {}, kıyı {}), {} ms",
                liq.general + liq.coastal,
                liq.general,
                liq.coastal,
                t0.elapsed().as_millis()
            );
            let ids = regions.and_then(|r| {
                r.map
                    .window(domain.col0, domain.row0, domain.w, domain.h)
                    .inspect_err(|e| log::error!("bölge penceresi okunamadı: {e}"))
                    .ok()
            });
            let msg = |kind: GroundKind, cov: [&[f32]; 3]| {
                let regions = ids
                    .as_ref()
                    .map(|ids| ground_by_region(cov, kind.min_coverage(), &g, &domain, s, ids))
                    .unwrap_or_default();
                let boxes = ground_boxes(kind, cov[0], &g, &domain, s, ids.as_deref());
                log::info!(
                    "{kind:?}: {} kare (kutu {BOX_KM} km, eşik {} km²)",
                    boxes.len(),
                    kind.box_threshold_km2()
                );
                JobMsg::GroundFailure {
                    kind,
                    regions,
                    boxes,
                }
            };
            if tx
                .send(msg(
                    GroundKind::Liquefaction,
                    [&liq.coverage, &liq.coverage_lo, &liq.coverage_hi],
                ))
                .is_err()
            {
                return;
            }
            ctx.request_repaint();
            if gr.has_landslide() {
                let t0 = std::time::Instant::now();
                let Some(ls) = gr.landslide(&g, &domain, s, cancel) else {
                    return; // iptal ya da okuma hatası (günlükte)
                };
                log::info!(
                    "heyelan: {} hücre > 0, {} ms",
                    ls.coverage.iter().filter(|&&c| c > 0.0).count(),
                    t0.elapsed().as_millis()
                );
                if tx
                    .send(msg(
                        GroundKind::Landslide,
                        [&ls.coverage, &ls.coverage_lo, &ls.coverage_hi],
                    ))
                    .is_err()
                {
                    return;
                }
                ctx.request_repaint();
            }
        }
    }
    if let Some(reg) = regions
        && (exposure.is_some() || vuln.is_some())
    {
        let started = std::time::Instant::now();
        let ids = match reg.map.window(domain.col0, domain.row0, domain.w, domain.h) {
            Ok(ids) => ids,
            Err(e) => {
                log::error!("bölge penceresi okunamadı: {e}");
                return;
            }
        };
        // Hasar için ince adımın iki katı yeterli (bölge toplamı; her
        // örnek hücre adım² hücreyi temsil eder).
        let dstep = (step * 2).max(2);
        let (mut by_region, mut city) = (Vec::new(), Vec::new());
        if let Some(exp) = exposure {
            let Some(r) = seismira_core::damage::regions(
                scenario,
                exp,
                &domain,
                dstep,
                &ids,
                window.as_deref(),
                cancel,
            ) else {
                return; // iptal
            };
            by_region = r;
            city = cities
                .iter()
                .zip(sites)
                .map(|(c, r)| {
                    r.motion?;
                    let id = reg.map.id_at(c.lon, c.lat).ok().filter(|&id| id != 0)?;
                    seismira_core::damage::city_fractions(
                        scenario,
                        exp,
                        id,
                        (c.lon, c.lat),
                        r.vs30_used,
                    )
                })
                .collect();
        }
        // GEM: ESRM20 stoku olan bölgeler atlanır (Avrupa ve Türkiye'de
        // ESRM20 hasar durumları gösterilir).
        let losses = match vuln {
            Some(v) => {
                let Some(l) = seismira_core::damage::loss_regions(
                    scenario,
                    v,
                    &domain,
                    dstep,
                    &ids,
                    window.as_deref(),
                    cancel,
                    |id| exposure.is_some_and(|e| e.region(id).is_some()),
                ) else {
                    return; // iptal
                };
                l
            }
            None => Vec::new(),
        };
        log::info!(
            "bina hasarı: {} bölge ESRM20, {} şehir, {} bölge GEM ({} ms)",
            by_region.len(),
            city.iter().filter(|c| c.is_some()).count(),
            losses.len(),
            started.elapsed().as_millis()
        );
        if tx
            .send(JobMsg::Damage(DamageResults {
                regions: by_region,
                cities: city,
                losses,
            }))
            .is_ok()
        {
            ctx.request_repaint();
        }
    }
}

/// İnce ızgarayı bölgelere göre toplar; her bölgenin en güçlü sarsılan
/// şehrini (model içindeki şehirler arasından) ekler.
fn region_results(
    regions: &Regions,
    grid: &seismira_core::grid::ShakeGrid,
    domain: &seismira_core::grid::Domain,
    step: u32,
    cities: &[City],
    sites: &[SiteResult],
) -> Option<Vec<RegionResult>> {
    use std::collections::HashMap;
    // Bölge rasterı Vs30 ile aynı ızgarada olmalı; değilse toplanamaz.
    let rg = regions.map.grid();
    let b = domain.base;
    if (rg.west - b.west).abs() > 1e-9
        || (rg.north - b.north).abs() > 1e-9
        || (rg.cell - b.cell).abs() > 1e-12
    {
        log::error!("bölge rasterı sarsıntı ızgarasıyla hizalı değil");
        return None;
    }
    let ids = match regions
        .map
        .window(domain.col0, domain.row0, domain.w, domain.h)
    {
        Ok(ids) => ids,
        Err(e) => {
            log::error!("bölge penceresi okunamadı: {e}");
            return None;
        }
    };
    let summaries = seismira_core::region::summarize(grid, domain, step, &ids);
    let mut best: HashMap<u16, (usize, f64)> = HashMap::new();
    for (i, r) in sites.iter().enumerate() {
        let Some(m) = r.motion else { continue };
        let c = &cities[i];
        let Ok(id) = regions.map.id_at(c.lon, c.lat) else {
            continue;
        };
        if id != 0 && best.get(&id).is_none_or(|&(_, j)| m.jma > j) {
            best.insert(id, (i, m.jma));
        }
    }
    Some(
        summaries
            .into_iter()
            .map(|summary| RegionResult {
                best_city: best.get(&summary.id).map(|&(i, _)| i),
                summary,
            })
            .collect(),
    )
}
