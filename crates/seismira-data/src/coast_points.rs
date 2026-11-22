//! Kıyı tahmin noktaları → `data/tsunami/coast.bin` (biçim: seismira-core
//! `tsunami::coast`).
//!
//! Kıyı hücresi: 8 komşusundan en az biri kara olan 1′ deniz hücresi
//! (boylamda 180° sarmalı). Seyreltme: ~10 km'lik enlem-boylam kutuları
//! (boylam adımı 1/cos φ ile genişler); her kutuda kutu merkezine en yakın
//! kıyı hücresi. En yakın şehir: büyük daire uzaklığı, 1° kovalarla.

use anyhow::{Result, ensure};
use rayon::prelude::*;
use seismira_core::raster::Raster;
use seismira_core::tsunami::{bathy, coast};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

const R_KM: f64 = 6371.0088;

fn haversine_km(a: (f64, f64), b: (f64, f64)) -> f64 {
    let (la1, la2) = (a.1.to_radians(), b.1.to_radians());
    let dl = (b.0 - a.0).to_radians();
    let h = ((la2 - la1) / 2.0).sin().powi(2) + la1.cos() * la2.cos() * (dl / 2.0).sin().powi(2);
    2.0 * R_KM * h.sqrt().asin()
}

pub fn run(bathy_path: &Path, cities_path: &Path, output: &Path) -> Result<()> {
    let r = Raster::open(std::fs::read(bathy_path)?)?;
    let g = r.grid;
    let li = r
        .layer("depth_m")
        .ok_or_else(|| anyhow::anyhow!("depth_m yok"))?;
    let (w, h) = (g.width as usize, g.height as usize);
    let codes = r.window(li, 0, 0, g.width, g.height, bathy::LAND)?;
    ensure!(codes.len() == w * h, "pencere boyutu");
    let info = &r.layers[li];
    let land = |x: i64, y: i64| -> bool {
        if y < 0 || y >= h as i64 {
            return true;
        }
        let x = x.rem_euclid(w as i64) as usize;
        codes[y as usize * w + x] == bathy::LAND
    };
    let dlat = (coast::SPACING_KM / R_KM).to_degrees();
    // (kutu) → (merkeze uzaklık², lon, lat, derinlik)
    let rows: Vec<Vec<((i64, i64), (f64, f64, f64, f32))>> = (0..h)
        .into_par_iter()
        .map(|y| {
            let mut out = Vec::new();
            for x in 0..w {
                let c = codes[y * w + x];
                if c == bathy::LAND {
                    continue;
                }
                let (xi, yi) = (x as i64, y as i64);
                let shore = (-1..=1)
                    .flat_map(|dy| (-1..=1).map(move |dx| (dx, dy)))
                    .any(|(dx, dy)| (dx, dy) != (0, 0) && land(xi + dx, yi + dy));
                if !shore {
                    continue;
                }
                let lon = g.west + (x as f64 + 0.5) * g.cell;
                let lat = g.north - (y as f64 + 0.5) * g.cell;
                let dlon = dlat / lat.to_radians().cos().max(0.02);
                let key = ((lon / dlon).floor() as i64, (lat / dlat).floor() as i64);
                let (cx, cy) = ((key.0 as f64 + 0.5) * dlon, (key.1 as f64 + 0.5) * dlat);
                let d2 = ((lon - cx) / dlon).powi(2) + ((lat - cy) / dlat).powi(2);
                let depth = info.value(c).unwrap_or(bathy::DEPTH_LO);
                out.push((key, (d2, lon, lat, depth)));
            }
            out
        })
        .collect();
    let mut best: HashMap<(i64, i64), (f64, f64, f64, f32)> = HashMap::new();
    let mut shore_cells = 0usize;
    for row in rows {
        for (k, v) in row {
            shore_cells += 1;
            let e = best.entry(k).or_insert(v);
            if v.0 < e.0 {
                *e = v;
            }
        }
    }
    let cities =
        seismira_core::cities::read(&mut std::io::BufReader::new(File::open(cities_path)?))?;
    let mut buckets: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
    for (i, c) in cities.iter().enumerate() {
        buckets
            .entry((c.lon.floor() as i32, c.lat.floor() as i32))
            .or_default()
            .push(i);
    }
    let mut pts: Vec<coast::CoastPoint> = best
        .into_par_iter()
        .map(|(_, (_, lon, lat, depth))| {
            // 1° kovalarda genişleyen halkalarla en yakın şehir; bulunan
            // uzaklık sonraki halkanın en yakın kenarından küçükse durur.
            let (bx, by) = (lon.floor() as i32, lat.floor() as i32);
            let mut found: Option<(f64, usize)> = None;
            for ring in 0i32..=30 {
                for dy in -ring..=ring {
                    for dx in -ring..=ring {
                        if dx.abs() != ring && dy.abs() != ring {
                            continue;
                        }
                        let bxx = (bx + dx + 180).rem_euclid(360) - 180;
                        if let Some(v) = buckets.get(&(bxx, by + dy)) {
                            for &i in v {
                                let d = haversine_km((lon, lat), (cities[i].lon, cities[i].lat));
                                if found.is_none_or(|(fd, _)| d < fd) {
                                    found = Some((d, i));
                                }
                            }
                        }
                    }
                }
                // Halka r'nin dışındaki en yakın şehir en az r° (enlemde
                // ~111 km × r) uzakta.
                if let Some((d, _)) = found
                    && d < f64::from(ring) * 111.0 * lat.to_radians().cos().max(0.05)
                {
                    break;
                }
            }
            let (city, city_km) =
                found.map_or((u32::MAX, f32::INFINITY), |(d, i)| (i as u32, d as f32));
            coast::CoastPoint {
                lon: lon as f32,
                lat: lat as f32,
                depth_m: depth,
                city,
                city_km,
            }
        })
        .collect();
    pts.sort_by(|a, b| a.lat.total_cmp(&b.lat).then(a.lon.total_cmp(&b.lon)));
    let near = pts.iter().filter(|p| p.city_km <= 50.0).count();
    println!(
        "kıyı hücresi {shore_cells}; nokta {} (~{} km arayla), şehri ≤ 50 km olan {near}",
        pts.len(),
        coast::SPACING_KM
    );
    if let Some(dir) = output.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut out = BufWriter::new(File::create(output)?);
    coast::write(&pts, &mut out)?;
    drop(out);
    println!(
        "{}: {:.2} MB",
        output.display(),
        std::fs::metadata(output)?.len() as f64 / 1e6
    );
    Ok(())
}
