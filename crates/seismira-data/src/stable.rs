//! USGS STREC `stable.geojson` → `data/regions/stable.sout` (kararlı kıta
//! içi bölgeler; seismira_core::tectonic). Noktalar (boylam, enlem) derece.

use std::path::Path;

use anyhow::{Context, Result, bail};
use seismira_core::region::{Ring, write_outlines};

pub fn run(geojson: &Path, output: &Path) -> Result<()> {
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(geojson)?)?;
    let features = v["features"].as_array().context("features yok")?;
    let mut rings = Vec::new();
    for f in features {
        let g = &f["geometry"];
        let polys: Vec<&serde_json::Value> = match g["type"].as_str() {
            Some("Polygon") => vec![&g["coordinates"]],
            Some("MultiPolygon") => g["coordinates"]
                .as_array()
                .context("koordinat yok")?
                .iter()
                .collect(),
            other => bail!("beklenmeyen geometri: {other:?}"),
        };
        for poly in polys {
            for (k, ring) in poly.as_array().context("çokgen")?.iter().enumerate() {
                let points = ring
                    .as_array()
                    .context("halka")?
                    .iter()
                    .map(|p| {
                        let x = p[0].as_f64().context("boylam")?;
                        let y = p[1].as_f64().context("enlem")?;
                        Ok([x as f32, y as f32])
                    })
                    .collect::<Result<Vec<_>>>()?;
                rings.push(Ring {
                    hole: k > 0,
                    points,
                });
            }
        }
    }
    let n: usize = rings.iter().map(|r| r.points.len()).sum();
    let mut out = std::io::BufWriter::new(std::fs::File::create(output)?);
    write_outlines(&vec![rings.clone()], &mut out, |b| {
        zstd::bulk::compress(b, 19)
    })?;
    println!("{} halka, {n} nokta → {}", rings.len(), output.display());
    Ok(())
}
