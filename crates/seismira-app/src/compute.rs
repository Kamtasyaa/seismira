//! Arka plan hesabı: şehir başına sarsıntı ve varış zamanları, ardından
//! sarsıntı ızgarası (önce kaba, sonra ince; CLAUDE.md Bölüm 11).
//!
//! Arayüz iş parçacığı beklemez (CLAUDE.md Bölüm 11): hesap ayrı bir iş
//! parçacığında yapılır, sonuç kanalla gelir. Parametre değişince eski iş
//! `cancel` bayrağıyla anında durdurulur.

use seismira_core::cities::City;
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

/// Arka plan işinin ürettiği sonuçlar (geliş sırasıyla).
pub enum JobMsg {
    Cities(CityResults),
    /// Sarsıntı ızgarası (önce kaba ön sonuç, sonra ince; sonraki öncekinin yerine geçer).
    Grid(Arc<seismira_core::grid::ShakeGrid>),
    /// İnce ızgaradan bölge özetleri (kimliğe göre sıralı) ve bu
    /// bölgelerin sınır geometrisi.
    Regions(Vec<RegionResult>, Option<Arc<crate::map::RegionLines>>),
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
        } = data;
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, rx) = channel();
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
                        regions.as_deref(),
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
    regions: Option<&Regions>,
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
        if tx.send(JobMsg::Grid(g)).is_err() {
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
