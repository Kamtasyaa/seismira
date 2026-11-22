//! Deneme: gerçek derinlik verisinde fast marching (Faz 8 adım 4). Nokta
//! kaynaktan (en yakın deniz hücresi) verilen noktalara varış süresi ve hesap
//! süresi. `k`: 1′ hücrelerin k × k birleştirilmesi (deniz: alt hücrelerden
//! biri deniz; derinlik: deniz alt hücrelerinin ortalaması).

use anyhow::{Context, Result};
use seismira_core::raster::Raster;
use seismira_core::tsunami::{bathy, travel};
use std::path::Path;
use std::sync::atomic::AtomicBool;

pub fn run(bathy_path: &Path, src: (f64, f64), k: usize, pts: &[(f64, f64)]) -> Result<()> {
    let r = Raster::open(std::fs::read(bathy_path)?)?;
    let g = r.grid;
    let li = r.layer("depth_m").context("depth_m yok")?;
    let codes = r.window(li, 0, 0, g.width, g.height, bathy::LAND)?;
    let info = &r.layers[li];
    let (w1, h1) = (g.width as usize, g.height as usize);
    let (w, h) = (w1 / k, h1 / k);
    let t0 = std::time::Instant::now();
    let mut depth = vec![f32::NAN; w * h];
    for y in 0..h {
        for x in 0..w {
            let (mut s, mut n) = (0.0f32, 0u32);
            for dy in 0..k {
                for dx in 0..k {
                    if let Some(v) = info.value(codes[(y * k + dy) * w1 + x * k + dx]) {
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
    let cell = g.cell * k as f64;
    let idx = |lon: f64, lat: f64| -> usize {
        let c = (((lon - g.west) / cell).floor() as i64).rem_euclid(w as i64) as usize;
        let rr = (((g.north - lat) / cell).floor() as usize).min(h - 1);
        rr * w + c
    };
    // En yakın deniz hücresi (küçük karelerde arama).
    let nearest_sea = |i: usize| -> Option<usize> {
        let (r0, c0) = ((i / w) as i64, (i % w) as i64);
        for rad in 0i64..50 {
            for dr in -rad..=rad {
                for dc in -rad..=rad {
                    if dr.abs() != rad && dc.abs() != rad {
                        continue;
                    }
                    let (rr, cc) = (r0 + dr, (c0 + dc).rem_euclid(w as i64));
                    if rr >= 0 && rr < h as i64 {
                        let j = rr as usize * w + cc as usize;
                        if depth[j].is_finite() {
                            return Some(j);
                        }
                    }
                }
            }
        }
        None
    };
    let seed = nearest_sea(idx(src.0, src.1)).context("kaynak yakınında deniz yok")?;
    println!(
        "ızgara {w} × {h} ({:.0}″), hazırlık {} ms",
        cell * 3600.0,
        t0.elapsed().as_millis()
    );
    let g2 = travel::Grid {
        west: g.west,
        north: g.north,
        cell,
        width: w,
        height: h,
        depth: &depth,
        wrap: true,
    };
    let t1 = std::time::Instant::now();
    let t = travel::compute(&g2, &[(seed, 0.0)], f64::INFINITY, &AtomicBool::new(false))
        .context("iptal")?;
    println!("fast marching {} ms", t1.elapsed().as_millis());
    for &(lon, lat) in pts {
        let v = nearest_sea(idx(lon, lat)).map(|j| f64::from(t[j]));
        match v {
            Some(s) => println!("{lon:.3},{lat:.3}: {:.2} saat", s / 3600.0),
            None => println!("{lon:.3},{lat:.3}: deniz yok"),
        }
    }
    Ok(())
}
