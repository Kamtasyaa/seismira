//! GLiM litolojisi (Hartmann & Moosdorf 2012) → `.sras` (tek katman `glim`,
//! Vs30 ızgarası). Girdi: scripts/gen_glim.py'nin ham u8 çıktısı (0 =
//! çokgen yok, 1–16 = `jessee2018::GLIM_CLASSES` sırası).
//!
//! LİSANS: vektör sürümün lisans metni görülmedi, izin istendi
//! (docs/veri.md). Çıktı yalnızca yerelde kullanılır; diğer girdilerden
//! AYRI bir dosyadır.
//!
//! Kodlar kategoriktir: kod = sınıf numarası (1–16). Vs30'da okyanus/göl
//! olan hücreler o kodu alır; karada çokgen yoksa `CODE_NODATA`.

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
        name: "glim".into(),
        scale: Scale::Linear,
        n_valid: 17,
        lo: 0.0,
        hi: 16.0,
    }]
}

pub fn run(raw: &Path, vs30: &Path, output: &Path) -> Result<()> {
    let vs = Raster::open(std::fs::read(vs30)?)?;
    let grid = vs.grid;
    let (w, h) = (grid.width as usize, grid.height as usize);
    let bytes = std::fs::read(raw)?;
    ensure!(bytes.len() == w * h, "ham GLiM boyutu ızgarayla uyuşmuyor");
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
        .map(|i| match (water[i], bytes[i]) {
            (c @ (CODE_OCEAN | CODE_LAKE), _) => c,
            (_, 0) => CODE_NODATA,
            (_, c) if c <= 16 => c,
            _ => CODE_NODATA,
        })
        .collect();
    let land_missing = (0..w * h)
        .filter(|&i| water[i] < CODE_OCEAN && codes[i] == CODE_NODATA)
        .count();
    let bad = bytes.iter().filter(|&&b| b > 16).count();
    ensure!(bad == 0, "ham GLiM'de 16'dan büyük {bad} kod var");
    println!("GLiM'i olmayan kara hücresi (Vs30'da kara): {land_missing}");
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
