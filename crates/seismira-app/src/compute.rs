//! Arka plan hesabı: şehir başına sarsıntı ve varış zamanları.
//!
//! Arayüz iş parçacığı beklemez (CLAUDE.md Bölüm 11): hesap ayrı bir iş
//! parçacığında yapılır, sonuç kanalla gelir. Parametre değişince eski iş
//! `cancel` bayrağıyla anında durdurulur.

use seismira_core::cities::City;
use seismira_core::quake::Quake;
use seismira_core::shaking::{Scenario, SiteResult};
use seismira_core::traveltime::TravelTimes;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, channel};

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
        tt: Arc<TravelTimes>,
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
                    sites.push(scenario.site((c.lon, c.lat)));
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
