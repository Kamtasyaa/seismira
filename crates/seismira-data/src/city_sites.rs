//! Şehir başına zemin (`.ssit`): Vs30 haritası + GHSL yapılaşma alanı.
//!
//! Her şehir için Vs30'un yapılaşma ağırlıklı medyanı
//! ([`seismira_core::site::Vs30Map::built_weighted`]); pencerede yapılaşma
//! yoksa nokta kuralı ([`seismira_core::site::Vs30Map::site`]).
//! GHSL: GHS-BUILT-S R2023A, 2020, EPSG:4326 30″ (hücre başına yapılaşmış
//! alan, m²), JRC, CC BY 4.0.

use crate::geotiff::GeoTiff;
use anyhow::Result;
use seismira_core::site::{Site, Vs30Map, Vs30Source, write_sites};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;

pub fn run(cities: &Path, vs30: &Path, ghsl: &Path, output: &Path) -> Result<()> {
    let cities = seismira_core::cities::read(&mut BufReader::new(File::open(cities)?))?;
    let map = Vs30Map::open(std::fs::read(vs30)?)?;
    let mut built = GeoTiff::open(ghsl)?;

    // Önbellek yerelliği için enleme göre sırayla işle, sonucu yerine yaz.
    let mut order: Vec<usize> = (0..cities.len()).collect();
    order.sort_by(|&a, &b| cities[b].lat.total_cmp(&cities[a].lat));
    let mut sites = vec![Site::reference(); cities.len()];
    let mut counts = [0usize; 4];
    let mut changed: Vec<f64> = Vec::new();
    for &i in &order {
        let c = &cities[i];
        let weighted = map.built_weighted(c.lon, c.lat, |lon, lat| {
            built
                .value_at(lon, lat)
                .map(f64::from)
                .map_err(|e| std::io::Error::other(e.to_string()))
        })?;
        let point = map.site(c.lon, c.lat)?;
        let s = weighted.unwrap_or(point);
        if s.source == Vs30Source::Built {
            changed.push((s.vs30 / point.vs30).ln());
        }
        counts[s.source as usize] += 1;
        sites[i] = s;
    }

    let mut out = BufWriter::new(File::create(output)?);
    write_sites(&sites, &mut out)?;
    drop(out);

    println!(
        "{} şehir: yapılaşma ağırlıklı {}, harita noktası {}, komşu kara {}, referans {}",
        cities.len(),
        counts[3],
        counts[0],
        counts[1],
        counts[2]
    );
    if !changed.is_empty() {
        let mut c = changed.clone();
        c.sort_by(f64::total_cmp);
        let q = |p: f64| c[((c.len() - 1) as f64 * p) as usize].exp();
        println!(
            "ağırlıklı / nokta Vs30 oranı: %10 {:.2}, medyan {:.2}, %90 {:.2}",
            q(0.1),
            q(0.5),
            q(0.9)
        );
    }
    for name in [
        "Gölcük",
        "Adapazarı",
        "İzmit",
        "Derince",
        "Istanbul",
        "Tokyo",
        "Mexico City",
    ] {
        if let Some(i) = cities.iter().position(|c| c.name == name) {
            let c = &cities[i];
            let p = map.site(c.lon, c.lat)?;
            println!(
                "  {name}: nokta {:.0} m/s → {:.0} m/s ({:?})",
                p.vs30, sites[i].vs30, sites[i].source
            );
        }
    }
    println!(
        "{}: {:.0} KB",
        output.display(),
        std::fs::metadata(output)?.len() as f64 / 1e3
    );
    Ok(())
}
