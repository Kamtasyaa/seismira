//! Arka plan hesabı: şehir başına sarsıntı ve varış zamanları.
//!
//! Arayüz iş parçacığı beklemez (CLAUDE.md Bölüm 11): hesap ayrı bir iş
//! parçacığında yapılır, sonuç kanalla gelir. Parametre değişince eski iş
//! `cancel` bayrağıyla anında durdurulur.

use seismira_core::cities::City;
use seismira_core::quake::Quake;
use seismira_core::shaking::{Scenario, SiteResult};
use seismira_core::site::{Site, Vs30Map};
use seismira_core::traveltime::TravelTimes;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, channel};

/// Vs30 haritası (dosya belleğe eşlenir; yalnızca gereken döşemeler okunur).
pub type Vs30 = Vs30Map<memmap2::Mmap>;

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

pub struct Job {
    cancel: Arc<AtomicBool>,
    rx: Receiver<CityResults>,
}

impl Job {
    pub fn spawn(
        quake: Quake,
        cities: Arc<Vec<City>>,
        city_sites: Option<Arc<Vec<Site>>>,
        tt: Arc<TravelTimes>,
        vs30: Option<Arc<Vs30>>,
        ctx: eframe::egui::Context,
    ) -> Self {
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, rx) = channel();
        let flag = cancel.clone();
        std::thread::Builder::new()
            .name("sarsıntı-hesabı".into())
            .spawn(move || {
                let started = std::time::Instant::now();
                let scenario = Scenario::new(quake, &tt);
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
                let model_limit = seismira_core::shaking::has_gmpe(quake.setting).then(|| {
                    scenario
                        .fault
                        .rrup_contour(seismira_core::gmpe::cy14::MAX_RRUP_KM, 180)
                });
                log::info!(
                    "{} şehir hesaplandı ({} ms)",
                    sites.len(),
                    started.elapsed().as_millis()
                );
                if tx
                    .send(CityResults {
                        sites,
                        felt_radius_deg,
                        model_limit,
                    })
                    .is_ok()
                {
                    ctx.request_repaint();
                }
            })
            .expect("iş parçacığı başlatılamadı");
        Self { cancel, rx }
    }

    /// Sonuç geldiyse döndürür.
    pub fn poll(&self) -> Option<CityResults> {
        self.rx.try_recv().ok()
    }
}

impl Drop for Job {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}
