//! Deniz derinliği → `data/tsunami/bathy.sras` (biçim: seismira-core
//! `tsunami::bathy`). Girdi: scripts/gen_bathy.py'nin ham int16 çıktısı
//! (21600 × 10800, 1′, derinlik m, kara -1).

use crate::ground_inputs::encode_full;
use anyhow::{Result, ensure};
use rayon::prelude::*;
use seismira_core::raster::{self, Grid};
use seismira_core::tsunami::bathy;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

pub fn run(raw: &Path, output: &Path) -> Result<()> {
    let grid = Grid {
        west: -180.0,
        north: 90.0,
        cell: 1.0 / 60.0,
        width: 21_600,
        height: 10_800,
        tile: 240,
    };
    let n = (grid.width * grid.height) as usize;
    let bytes = std::fs::read(raw)?;
    ensure!(
        bytes.len() == 2 * n,
        "ham derinlik boyutu ızgarayla uyuşmuyor"
    );
    let info = &bathy::layers()[0];
    let codes: Vec<u8> = (0..n)
        .into_par_iter()
        .map(|i| {
            let d = i16::from_le_bytes([bytes[2 * i], bytes[2 * i + 1]]);
            if d < 1 {
                bathy::LAND
            } else {
                info.quantize(f32::from(d))
            }
        })
        .collect();
    drop(bytes);
    let sea = codes.iter().filter(|&&c| c != bathy::LAND).count();
    println!("deniz hücresi {sea} / {n}");
    let enc = encode_full(&codes, &grid)?;
    if let Some(dir) = output.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut out = BufWriter::new(File::create(output)?);
    raster::write(&mut out, &grid, &bathy::layers(), &[enc])?;
    drop(out);
    println!(
        "{}: {:.1} MB",
        output.display(),
        std::fs::metadata(output)?.len() as f64 / 1e6
    );
    Ok(())
}
