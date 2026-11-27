//! `hazard` komutu: `scripts/fetch_eshm20.py`'nin CSV'sinden tehlike haritası
//! dosyası (`.shaz`, [`seismira_core::hazard`]). Izgara verinin kendisinden:
//! ana bölgenin kafesi (boylam 0,0821339 + k·0,1, enlem j·0,1; 2026-11-27
//! denetlendi). ESHM20'nin ayrı alt modelleri (İspanya'nın güney ve
//! batısındaki adalar, İzlanda) farklı ızgaralarda: kafese ±0,001° içinde
//! oturmayan noktalar atlanır ve sayılır (docs/veri.md "ESHM20").

use anyhow::{Context, Result, bail};
use seismira_core::hazard::{self, Grid, LayerInfo};
use std::collections::BTreeMap;
use std::path::Path;

const STEP: f64 = 0.1;
/// (yıllık olasılık bitleri, periyot bitleri) → (boylam, enlem, g) noktaları.
type Layers = BTreeMap<(u64, u64), Vec<(f64, f64, f32)>>;
/// Ana bölge kafesinin boylam kayması.
const LON_ORIGIN: f64 = 0.0821339;

fn period_of(imt: &str) -> Result<f64> {
    if imt == "PGA" {
        return Ok(0.0);
    }
    let p = imt
        .strip_prefix("SA[")
        .and_then(|s| s.strip_suffix("s]"))
        .with_context(|| format!("şiddet ölçüsü {imt}"))?;
    Ok(p.parse()?)
}

pub fn run(csv_path: &Path, out: &Path) -> Result<()> {
    let mut rdr = csv::Reader::from_path(csv_path)?;
    let mut layers: Layers = BTreeMap::new();
    let (mut w, mut s, mut e, mut n) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    let on_lattice = |lon: f64, lat: f64| {
        let fx = (lon - LON_ORIGIN) / STEP;
        let fy = lat / STEP;
        (fx - fx.round()).abs() < 0.01 && (fy - fy.round()).abs() < 0.01
    };
    let mut off = 0usize;
    for rec in rdr.records() {
        let r = rec?;
        let period = period_of(&r[0])?;
        let poe: f64 = r[1].parse()?;
        let (lon, lat, v): (f64, f64, f32) = (r[2].parse()?, r[3].parse()?, r[4].parse()?);
        if !on_lattice(lon, lat) {
            off += 1;
            continue;
        }
        w = w.min(lon);
        s = s.min(lat);
        e = e.max(lon);
        n = n.max(lat);
        layers
            .entry((poe.to_bits(), period.to_bits()))
            .or_default()
            .push((lon, lat, v));
    }
    if layers.is_empty() {
        bail!("veri yok");
    }
    let grid = Grid {
        west: w,
        south: s,
        step: STEP,
        nx: ((e - w) / STEP).round() as u32 + 1,
        ny: ((n - s) / STEP).round() as u32 + 1,
    };
    let cells = grid.nx as usize * grid.ny as usize;
    let mut out_layers = Vec::new();
    let mut ordered: Vec<_> = layers.into_iter().collect();
    // Önce olasılık (büyükten küçüğe: %10, %2), sonra periyot.
    ordered.sort_by(|a, b| {
        f64::from_bits(b.0.0)
            .total_cmp(&f64::from_bits(a.0.0))
            .then(f64::from_bits(a.0.1).total_cmp(&f64::from_bits(b.0.1)))
    });
    for ((poe, period), pts) in ordered {
        let mut vals = vec![f32::NAN; cells];
        for (lon, lat, v) in pts {
            let fx = (lon - grid.west) / STEP;
            let fy = (lat - grid.south) / STEP;
            if (fx - fx.round()).abs() > 0.01 || (fy - fy.round()).abs() > 0.01 {
                bail!("düzensiz ızgara: {lon} {lat}");
            }
            vals[fy.round() as usize * grid.nx as usize + fx.round() as usize] = v;
        }
        out_layers.push((
            LayerInfo {
                period: f64::from_bits(period),
                annual_poe: f64::from_bits(poe),
            },
            vals,
        ));
    }
    let mut f = std::io::BufWriter::new(std::fs::File::create(out)?);
    hazard::write(
        grid,
        &out_layers,
        |raw| zstd::bulk::compress(raw, 19),
        &mut f,
    )?;
    drop(f);
    println!(
        "{} katman, ızgara {}×{} ({:.3}…{:.3} D, {:.1}…{:.1} K); kafes dışı {off} nokta atlandı; {} ({} bayt)",
        out_layers.len(),
        grid.nx,
        grid.ny,
        w,
        e,
        s,
        n,
        out.display(),
        std::fs::metadata(out)?.len()
    );
    Ok(())
}
