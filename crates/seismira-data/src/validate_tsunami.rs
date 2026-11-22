//! Tsunami doğrulaması (Faz 8 adım 7; docs/dogrulama.md): NOAA NCEI Küresel
//! Tarihsel Tsunami Veritabanı gözlemleriyle (scripts/fetch_tsunami_validation.py).
//!
//! Ölçüm türü (typeMeasurementId; kayıt adlarından çıkarıldı, API tanım
//! vermiyor): 2 mareograf, 3 DART (BPR), 4 / 5 saha araştırması, 1 görgü.
//!
//! - Varış: tür 2 ve 3, `travHours` / `travMins` verilmiş kayıtlar. Model:
//!   bölgesel 1′ pencerede (uygulamayla aynı yarıçap kuralı) ya da Mw ≥ 7,9
//!   ise küresel 4′ ızgarada, gözlem noktasına 15 km içindeki en yakın deniz
//!   hücresinin varış süresi.
//! - Yükseklik: tür 2 ve 4 (`runupHt`), modelin yakın alan tırmanması olan
//!   kıyı noktasına ≤ 20 km olanlar; JMA kategorisi uyumu ve ln(model /
//!   gözlem).

use crate::tsunami_scenario::depth_window;
use anyhow::{Context, Result};
use seismira_core::geo;
use seismira_core::quake::{FaultType, Quake, Setting};
use seismira_core::raster::Raster;
use seismira_core::tsunami::{bathy, coast, forecast, trigger};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::atomic::AtomicBool;

fn category(h: f64) -> u8 {
    if h > 3.0 {
        3
    } else if h > 1.0 {
        2
    } else if h >= 0.2 {
        1
    } else {
        0
    }
}

struct Win {
    west: f64,
    north: f64,
    cell: f64,
    width: usize,
    height: usize,
    wrap: bool,
    eta: Vec<f32>,
    depth: Vec<f32>,
}

impl Win {
    fn dw(&self) -> forecast::Depth<'_> {
        forecast::Depth {
            west: self.west,
            north: self.north,
            cell: self.cell,
            width: self.width,
            height: self.height,
            depth: &self.depth,
            wrap: self.wrap,
        }
    }

    /// Noktaya 15 km içindeki en yakın denizdeki varış (s).
    fn eta_near(&self, lon: f64, lat: f64) -> Option<f64> {
        let d = self.dw();
        let step = (15.0 / 111.0 / self.cell).ceil() as i64;
        let mut best: Option<(f64, f64)> = None;
        for dy in -step..=step {
            for dx in -step..=step {
                let (x, y) = (
                    lon + dx as f64 * self.cell / lat.to_radians().cos().max(0.2),
                    lat + dy as f64 * self.cell,
                );
                let Some(i) = d.index(x, y) else {
                    continue;
                };
                let t = f64::from(*self.eta.get(i)?);
                if !self.depth[i].is_finite() || !t.is_finite() {
                    continue;
                }
                let km = geo::distance_azimuth((lon, lat), d.center(i)).0;
                if km <= 15.0 && best.is_none_or(|(bk, _)| km < bk) {
                    best = Some((km, t));
                }
            }
        }
        best.map(|b| b.1)
    }
}

#[allow(clippy::too_many_arguments)]
pub fn run(
    obs: &Path,
    epi: (f64, f64),
    mw: f64,
    depth_km: f64,
    strike: f64,
    dip: f64,
    rake: f64,
    setting: &str,
) -> Result<()> {
    let data = Path::new("data");
    let mut q = Quake::new(epi);
    q.mw = mw;
    q.depth_km = depth_km;
    q.strike = strike;
    q.dip = dip;
    q.rake = rake;
    q.fault = FaultType::from_rake(rake);
    q.setting = match setting {
        "interface" => Setting::Interface,
        "intraslab" => Setting::Intraslab,
        _ => Setting::ShallowCrust,
    };
    let r = Raster::open(std::fs::read(data.join("tsunami").join(bathy::FILE))?)?;
    let g = r.grid;
    let pts = coast::read(&mut BufReader::new(File::open(
        data.join("tsunami").join(coast::FILE),
    )?))?;
    let cancel = AtomicBool::new(false);
    // Bölgesel pencere: uygulamadaki kural (crates/seismira-app/src/tsunami.rs).
    let pre = trigger::assess(mw, depth_km, true);
    let rad = match pre {
        trigger::Assessment::Threat {
            radius_km: Some(rk),
        } => (rk / 111.0 + 2.0).min(14.0),
        _ => 8.0,
    };
    let cell = g.cell;
    let west = ((epi.0 - rad / epi.1.to_radians().cos().max(0.2)) / cell).floor() * cell;
    let north = ((epi.1 + rad).min(90.0) / cell).ceil() * cell;
    let south = ((epi.1 - rad).max(-90.0) / cell).floor() * cell;
    let w1 = ((2.0 * (epi.0 - west) / cell).ceil() as u32).min(g.width);
    let h1 = ((north - south) / cell).round() as u32;
    let mut reg = Win {
        west,
        north,
        cell,
        width: w1 as usize,
        height: h1 as usize,
        wrap: false,
        eta: Vec::new(),
        depth: depth_window(&r, west, north, w1, h1, 1)?,
    };
    let f = forecast::compute(&q, &reg.dw(), &pts, 6.0 * 3600.0, &cancel).context("iptal")?;
    println!(
        "{:?}; H {:.2} m, d {:.0} m, eğim {:.4}, R0 {:.1} m",
        f.assessment, f.h_max, f.d, f.slope, f.r0
    );
    reg.eta = f.eta.clone();
    let mut glob = None;
    if matches!(
        f.assessment,
        trigger::Assessment::Threat { radius_km: None }
    ) {
        let k = 4;
        let mut w = Win {
            west: g.west,
            north: g.north,
            cell: cell * k as f64,
            width: (g.width as usize) / k,
            height: (g.height as usize) / k,
            wrap: true,
            eta: Vec::new(),
            depth: depth_window(&r, g.west, g.north, g.width, g.height, k)?,
        };
        let fg = forecast::compute(&q, &w.dw(), &pts, 24.0 * 3600.0, &cancel).context("iptal")?;
        w.eta = fg.eta;
        glob = Some(w);
    }
    let items: Vec<serde_json::Value> = serde_json::from_reader(BufReader::new(File::open(obs)?))?;
    // Varış süreleri.
    let mut rows = Vec::new();
    for it in &items {
        let ty = it["typeMeasurementId"].as_i64().unwrap_or(0);
        if ty != 2 && ty != 3 {
            continue;
        }
        let (Some(lon), Some(lat)) = (it["longitude"].as_f64(), it["latitude"].as_f64()) else {
            continue;
        };
        let (h, m) = (it["travHours"].as_f64(), it["travMins"].as_f64());
        if h.is_none() && m.is_none() {
            continue;
        }
        let t_obs = h.unwrap_or(0.0) * 3600.0 + m.unwrap_or(0.0) * 60.0;
        if t_obs <= 0.0 {
            continue;
        }
        let model = reg
            .eta_near(lon, lat)
            .or_else(|| glob.as_ref().and_then(|w| w.eta_near(lon, lat)));
        let km = geo::distance_azimuth(epi, (lon, lat)).0;
        rows.push((
            it["locationName"].as_str().unwrap_or("?").to_owned(),
            ty,
            km,
            t_obs,
            model,
        ));
    }
    rows.sort_by(|a, b| a.2.total_cmp(&b.2));
    let summarize = |name: &str, sel: &dyn Fn(f64) -> bool| {
        let v: Vec<f64> = rows
            .iter()
            .filter(|r| sel(r.2))
            .filter_map(|r| r.4.map(|m| m / r.3))
            .collect();
        let n_all = rows.iter().filter(|r| sel(r.2)).count();
        if v.is_empty() {
            println!("varış {name}: gözlem {n_all}, modelde karşılığı yok");
            return;
        }
        let mut s = v.clone();
        s.sort_by(f64::total_cmp);
        let within = v.iter().filter(|x| (*x - 1.0).abs() < 0.2).count();
        println!(
            "varış {name}: {} / {n_all} gözlem; model/gözlem medyan {:.2} (çeyrekler {:.2}–{:.2}); ±%20 içinde %{:.0}",
            v.len(),
            s[s.len() / 2],
            s[s.len() / 4],
            s[3 * s.len() / 4],
            100.0 * within as f64 / v.len() as f64
        );
    };
    summarize("< 1000 km", &|km| km < 1000.0);
    summarize("≥ 1000 km", &|km| km >= 1000.0);
    for r in rows.iter().filter(|r| r.2 < 1000.0).take(12) {
        println!(
            "  {:<40} tür {} {:>6.0} km: gözlem {:>5.0} dk, model {}",
            r.0,
            r.1,
            r.2,
            r.3 / 60.0,
            r.4.map_or("-".into(), |m| format!("{:.0} dk", m / 60.0))
        );
    }
    // Yükseklikler (yakın alan).
    let near: Vec<(f64, f64, f64)> = f
        .coast
        .iter()
        .filter_map(|c| {
            let p = &pts[c.point];
            c.runup_m.map(|h| (f64::from(p.lon), f64::from(p.lat), h))
        })
        .collect();
    let (mut n, mut same, mut lnsum, mut obs_max) = (0usize, 0usize, 0.0, 0.0f64);
    let mut confusion = [[0usize; 4]; 4];
    for it in &items {
        let ty = it["typeMeasurementId"].as_i64().unwrap_or(0);
        if ty != 2 && ty != 4 {
            continue;
        }
        let (Some(lon), Some(lat), Some(ho)) = (
            it["longitude"].as_f64(),
            it["latitude"].as_f64(),
            it["runupHt"].as_f64(),
        ) else {
            continue;
        };
        if ho <= 0.0 {
            continue;
        }
        let best = near
            .iter()
            .map(|&(x, y, h)| (geo::distance_azimuth((lon, lat), (x, y)).0, h))
            .min_by(|a, b| a.0.total_cmp(&b.0));
        let Some((km, hm)) = best else {
            continue;
        };
        if km > 20.0 {
            continue;
        }
        n += 1;
        obs_max = obs_max.max(ho);
        let (co, cm) = (category(ho), category(hm));
        confusion[co as usize][cm as usize] += 1;
        same += usize::from(co == cm);
        lnsum += (hm.max(0.01) / ho).ln();
    }
    // Bölge bazında (NCEI `area`, Japonya'da il): JMA kategorisi tahmin
    // bölgesindeki EN YÜKSEK değere göre verilir ("高いところで"); gözlenen
    // en yüksek ile bölgedeki gözlemlere ≤ 20 km kıyı noktalarındaki model en
    // yükseği karşılaştırılır.
    let mut areas: std::collections::BTreeMap<String, (f64, f64)> = Default::default();
    for it in &items {
        let ty = it["typeMeasurementId"].as_i64().unwrap_or(0);
        if ty != 2 && ty != 4 {
            continue;
        }
        let (Some(lon), Some(lat), Some(ho), Some(area)) = (
            it["longitude"].as_f64(),
            it["latitude"].as_f64(),
            it["runupHt"].as_f64(),
            it["area"].as_str(),
        ) else {
            continue;
        };
        let hm = near
            .iter()
            .filter(|&&(x, y, _)| geo::distance_azimuth((lon, lat), (x, y)).0 <= 20.0)
            .map(|&(_, _, h)| h)
            .fold(f64::NAN, f64::max);
        if !hm.is_finite() {
            continue;
        }
        let e = areas.entry(area.to_owned()).or_insert((0.0, 0.0));
        e.0 = e.0.max(ho);
        e.1 = e.1.max(hm);
    }
    if !areas.is_empty() {
        let same = areas
            .values()
            .filter(|(o, m)| category(*o) == category(*m))
            .count();
        println!(
            "bölge bazında (en yüksek / en yüksek): {} bölge, aynı JMA kategorisi {} (%{:.0})",
            areas.len(),
            same,
            100.0 * same as f64 / areas.len() as f64
        );
        for (a, (o, m)) in &areas {
            println!(
                "    {a:<28} gözlem {o:>5.1} m, model {m:>5.1} m  ({} / {})",
                category(*o),
                category(*m)
            );
        }
    }
    if n > 0 {
        println!(
            "yükseklik (yakın alan, tür 2 + 4, ≤ 20 km): {n} gözlem, aynı JMA kategorisi %{:.0}, \
             ortalama ln(model/gözlem) {:+.2} (oran {:.2}); gözlenen en yüksek {obs_max:.1} m, R0 {:.1} m",
            100.0 * same as f64 / n as f64,
            lnsum / n as f64,
            (lnsum / n as f64).exp(),
            f.r0
        );
        println!("  karışıklık (satır gözlem, sütun model; 0 yok, 1 sarı, 2 kırmızı, 3 mor):");
        for (k, row) in confusion.iter().enumerate() {
            println!("    {k}: {row:?}");
        }
    } else {
        println!("yükseklik: yakın alanda eşlenen gözlem yok");
    }
    Ok(())
}
