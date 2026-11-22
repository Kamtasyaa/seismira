//! Deneme: tek bir deprem için tsunami tahmini (Faz 8 adım 5). Merkez üssü
//! çevresinde ± `radius_deg` pencerede 1′ derinlik (k × k birleştirilmiş),
//! kıyı noktaları, `tsunami::forecast::compute`. Özet ve en yüksek
//! tırmanmalı kıyı noktaları yazılır.

use anyhow::{Context, Result};
use seismira_core::cities;
use seismira_core::quake::{FaultType, Quake, Setting};
use seismira_core::raster::Raster;
use seismira_core::tsunami::{bathy, coast, forecast};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::atomic::AtomicBool;

#[allow(clippy::too_many_arguments)]
pub fn run(
    data: &Path,
    epi: (f64, f64),
    mw: f64,
    depth_km: f64,
    strike: f64,
    dip: f64,
    rake: f64,
    setting: &str,
    radius_deg: f64,
    k: usize,
) -> Result<()> {
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
    let li = r.layer("depth_m").context("depth_m")?;
    let info = &r.layers[li];
    let cell1 = g.cell;
    let col0 = ((epi.0 - radius_deg - g.west) / cell1).floor() as i64;
    let row0 = (((g.north - (epi.1 + radius_deg)) / cell1).floor() as i64).max(0);
    let row1 = (((g.north - (epi.1 - radius_deg)) / cell1).ceil() as i64).min(i64::from(g.height));
    let w1 = ((2.0 * radius_deg / cell1).ceil() as u32).min(g.width) / k as u32 * k as u32;
    let h1 = ((row1 - row0) as u32) / k as u32 * k as u32;
    let codes = r.window(li, col0, row0, w1, h1, bathy::LAND)?;
    let (w, h) = (w1 as usize / k, h1 as usize / k);
    let mut depth = vec![f32::NAN; w * h];
    for y in 0..h {
        for x in 0..w {
            let (mut s, mut n) = (0.0f32, 0u32);
            for dy in 0..k {
                for dx in 0..k {
                    if let Some(v) = info.value(codes[(y * k + dy) * w1 as usize + x * k + dx]) {
                        s += v;
                        n += 1;
                    }
                }
            }
            if n > 0 {
                depth[y * w + x] = s / n as f32;
            }
        }
    }
    let dw = forecast::Depth {
        west: g.west + col0 as f64 * cell1,
        north: g.north - row0 as f64 * cell1,
        cell: cell1 * k as f64,
        width: w,
        height: h,
        depth: &depth,
        wrap: false,
    };
    let pts = coast::read(&mut BufReader::new(File::open(
        data.join("tsunami").join(coast::FILE),
    )?))?;
    let cities = cities::read(&mut BufReader::new(File::open(
        data.join("cities").join("cities15000.scty"),
    )?))?;
    let t0 = std::time::Instant::now();
    let f = forecast::compute(&q, &dw, &pts, 24.0 * 3600.0, &AtomicBool::new(false))
        .context("iptal")?;
    println!(
        "pencere {w} × {h} ({:.0}″), hesap {} ms",
        dw.cell * 3600.0,
        t0.elapsed().as_millis()
    );
    println!("değerlendirme: {:?}", f.assessment);
    println!(
        "kayma {:.2} m, H {:.2} m @ {:.2},{:.2}, d {:.0} m, eğim {:.4}, R0 {:.1} m",
        f.slip, f.h_max, f.at.0, f.at.1, f.d, f.slope, f.r0
    );
    let mut v: Vec<_> = f.coast.iter().filter(|c| c.runup_m.is_some()).collect();
    v.sort_by(|a, b| b.runup_m.unwrap().total_cmp(&a.runup_m.unwrap()));
    println!(
        "kıyı noktası: {} (yakın alan {}, tehdit {})",
        f.coast.len(),
        v.len(),
        f.coast.iter().filter(|c| c.threat).count()
    );
    for c in v.iter().take(12) {
        let p = &pts[c.point];
        let name = cities.get(p.city as usize).map_or("-".to_owned(), |x| {
            format!("{} ({:.0} km)", x.name, p.city_km)
        });
        println!(
            "  {:.2},{:.2} {name}: {:.1} m, varış {:.0} dk",
            p.lon,
            p.lat,
            c.runup_m.unwrap(),
            c.eta_s / 60.0
        );
    }
    Ok(())
}
