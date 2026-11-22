//! ESA CCI Land Cover 2009 (v2.0.7, 300 m) → `.sras` (tek katman `cci_lc`,
//! Vs30 ızgarası). Kod = CCI sınıf değeri (0–220). GlobCover 2009
//! sınıflarına eşleme (Jessee 2018 katsayıları için) çekirdekte:
//! `jessee2018::cci_to_globcover` (docs/bilim.md 7.9).
//!
//! Lisans: CCI Open Data Portal koşulları, "any user for any purpose",
//! atıf şartı (data/raw/esacci/terms_and_conditions.txt, docs/veri.md).
//!
//! Örnekleme: CCI pikseli 1/360°, Vs30 hücresi 1/120°; her hücreye tam
//! 3 × 3 piksel düşer (kenarlar hizalı, PixelIsArea). Hücre değeri bu 9
//! pikselin çoğunluk sınıfıdır; eşitlikte merkez piksel (hücre merkeziyle
//! çakışır) eşit olanlar arasındaysa o, değilse en küçük kod. Su (210) ve
//! veri yok (0) çoğunluğa katılmaz; Vs30'da kara olan hücrede 9 pikselin
//! hepsi su/veri yoksa `CODE_NODATA`. Vs30 okyanus/göl kodu korunur.

use crate::geotiff::GeoTiff;
use crate::ground_inputs::encode_full;
use crate::vs30::{CODE_LAKE, CODE_NODATA, CODE_OCEAN};
use anyhow::{Result, ensure};
use seismira_core::raster::{self, LayerInfo, Raster, Scale};
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

const CCI_WATER: u8 = 210;

pub fn layers() -> Vec<LayerInfo> {
    vec![LayerInfo {
        name: "cci_lc".into(),
        scale: Scale::Linear,
        n_valid: 251,
        lo: 0.0,
        hi: 250.0,
    }]
}

/// 3 × 3 bloğun hücre değeri (modül belgesindeki kural); `None` = kara
/// sınıfı yok.
fn block_class(px: &[u8; 9]) -> Option<u8> {
    let center = px[4];
    let mut counts = [0u32; 256];
    for &c in px {
        if c != 0 && c != CCI_WATER {
            counts[usize::from(c)] += 1;
        }
    }
    let max = *counts.iter().max()?;
    if max == 0 {
        return None;
    }
    if counts[usize::from(center)] == max && center != 0 && center != CCI_WATER {
        return Some(center);
    }
    counts.iter().position(|&n| n == max).map(|c| c as u8)
}

pub fn run(tif: &Path, vs30: &Path, output: &Path) -> Result<()> {
    let vs = Raster::open(std::fs::read(vs30)?)?;
    let grid = vs.grid;
    let (w, h) = (grid.width as usize, grid.height as usize);
    let mut src = GeoTiff::open(tif)?;
    let (sw, sh) = src.size();
    ensure!(
        (src.cell * 3.0 - grid.cell).abs() < 1e-9,
        "CCI hücresi Vs30 hücresinin üçte biri değil"
    );
    let x0 = (grid.west - src.west) / src.cell;
    let y0 = (src.north - grid.north) / src.cell;
    ensure!(
        (x0 - x0.round()).abs() < 1e-6 && (y0 - y0.round()).abs() < 1e-6,
        "ızgaralar hizalı değil"
    );
    let (x0, y0) = (x0.round() as usize, y0.round() as usize);
    ensure!(
        x0 + 3 * w <= sw as usize && y0 + 3 * h <= sh as usize,
        "CCI ızgarası Vs30 ızgarasını kapsamıyor"
    );
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
    let sws = sw as usize;
    let mut buf = vec![0f32; sws * 3];
    let mut codes = vec![CODE_NODATA; w * h];
    let mut land_missing = 0usize;
    for r in 0..h {
        src.rows((y0 + 3 * r) as u32, 3, &mut buf)?;
        for c in 0..w {
            let i = r * w + c;
            if matches!(water[i], CODE_OCEAN | CODE_LAKE) {
                codes[i] = water[i];
                continue;
            }
            let mut px = [0u8; 9];
            for dy in 0..3 {
                for dx in 0..3 {
                    px[dy * 3 + dx] = buf[dy * sws + x0 + 3 * c + dx] as u8;
                }
            }
            match block_class(&px) {
                Some(k) => codes[i] = k,
                None => land_missing += 1,
            }
        }
        if r % 2000 == 0 {
            println!("  satır {r}/{h}");
        }
    }
    println!("CCI kara sınıfı olmayan kara hücresi (Vs30'da kara): {land_missing}");
    let mut hist = [0u64; 256];
    for &k in &codes {
        hist[usize::from(k)] += 1;
    }
    let s: Vec<String> = (1..=220)
        .filter(|&k| hist[k] > 0)
        .map(|k| format!("{k}:{}", hist[k]))
        .collect();
    println!("sınıf hücre sayıları: {}", s.join(" "));
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

#[cfg(test)]
mod tests {
    use super::block_class;

    #[test]
    fn cogunluk_ve_esitlik() {
        assert_eq!(
            block_class(&[10, 10, 10, 50, 50, 50, 50, 210, 210]),
            Some(50)
        );
        // Eşitlik: merkez (50) eşit olanlar arasında.
        assert_eq!(
            block_class(&[10, 10, 10, 210, 50, 50, 50, 210, 0]),
            Some(50)
        );
        // Eşitlik, merkez su: en küçük kod.
        assert_eq!(
            block_class(&[30, 30, 20, 20, 210, 210, 210, 210, 0]),
            Some(20)
        );
        assert_eq!(block_class(&[210; 9]), None);
        assert_eq!(block_class(&[0; 9]), None);
    }
}
