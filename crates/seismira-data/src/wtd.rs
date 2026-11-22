//! Fan vd. (2017) yıllık ortalama yeraltı su seviyesi → `.sras` (tek katman
//! `wtd_m`, Vs30 ızgarası). Girdi: scripts/gen_fan_wtd.py'nin ham int16
//! çıktısı (santimetre, -32768 veri yok).
//!
//! LİSANS: yalnızca araştırma amaçlı serbest; yeniden dağıtım izni
//! beklenirken çıktı yalnızca yerelde kullanılır (docs/veri.md). Bu yüzden
//! diğer girdilerden AYRI bir dosyadır.
//!
//! Kuantalama: logaritmik 0,1–1000 m (göreli adım %3,7). Zhu 2017'de
//! −0,0333/m: 10 m'de adım 0,37 m (logit 0,012); 0,1 m altı 0,1'e kırpılır
//! (logit farkı ≤ 0,003).

use crate::ground_inputs::encode_full;
use crate::vs30::{CODE_LAKE, CODE_NODATA, CODE_OCEAN};
use anyhow::{Result, ensure};
use rayon::prelude::*;
use seismira_core::raster::{self, LayerInfo, Raster, Scale};
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

pub fn layers() -> Vec<LayerInfo> {
    vec![LayerInfo {
        name: "wtd_m".into(),
        scale: Scale::Log,
        n_valid: 251,
        lo: 0.1,
        hi: 1000.0,
    }]
}

pub fn run(raw: &Path, vs30: &Path, output: &Path) -> Result<()> {
    let vs = Raster::open(std::fs::read(vs30)?)?;
    let grid = vs.grid;
    let (w, h) = (grid.width as usize, grid.height as usize);
    let bytes = std::fs::read(raw)?;
    ensure!(
        bytes.len() == 2 * w * h,
        "ham WTD boyutu ızgarayla uyuşmuyor"
    );
    let info = &layers()[0];
    let t = grid.tile as usize;
    let mut water = vec![0u8; w * h];
    for ty in 0..grid.tiles_y() as usize {
        for tx in 0..grid.tiles_x() as usize {
            let tile = vs.tile(0, tx as u32, ty as u32)?;
            for y in 0..t.min(h - ty * t) {
                for x in 0..t.min(w - tx * t) {
                    water[(ty * t + y) * w + tx * t + x] = tile.get(grid.tile, x as u32, y as u32);
                }
            }
        }
    }
    let codes: Vec<u8> = (0..w * h)
        .into_par_iter()
        .map(|i| match water[i] {
            c @ (CODE_OCEAN | CODE_LAKE) => c,
            _ => {
                let v = i16::from_le_bytes([bytes[2 * i], bytes[2 * i + 1]]);
                if v == i16::MIN {
                    CODE_NODATA
                } else {
                    info.quantize(f32::from(v) / 100.0)
                }
            }
        })
        .collect();
    let land_missing = (0..w * h)
        .filter(|&i| water[i] < CODE_OCEAN && codes[i] == CODE_NODATA)
        .count();
    println!("WTD'si olmayan kara hücresi (Vs30'da kara): {land_missing}");
    let enc = encode_full(&codes, &grid)?;
    let mut out = BufWriter::new(File::create(output)?);
    raster::write(&mut out, &grid, &layers(), &[enc])?;
    drop(out);
    println!(
        "{}: {:.1} MB",
        output.display(),
        std::fs::metadata(output)?.len() as f64 / 1e6
    );
    Ok(())
}
