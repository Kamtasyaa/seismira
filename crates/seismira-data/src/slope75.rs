//! Nowicki Jessee vd. (2018) eğim girdisi: GMTED2010 md75 (7,5″ medyan
//! yükseklik) eğimi, Vs30 ızgarasına (30″) özetlenmiş.
//!
//! Jessee 2018 (Bölüm 2.2, Tablo 1) eğimi 7,5″ GMTED medyan yüksekliğinden
//! hesaplıyor; 30″'e doğrudan yeniden örneklenmiş eğim dikliği düşürür.
//! Modelde eğim ln(PGV) ile çarpım terimi içinde de geçtiği için (logit =
//! … + c·eğim + g·ln(PGV)·eğim) tek bir ortalama eğim olasılığı bozar:
//! P, eğimin doğrusal olmayan işlevidir. Bu yüzden her 30″ hücredeki
//! 4 × 4 = 16 alt hücrenin eğimi sıralanır ve dört çeyrek grubunun
//! ortalaması saklanır (`slope_q0` en düz … `slope_q3` en dik). Uygulama
//! olasılığı bu dört değer üzerinden ortalar (her biri 4 alt hücreyi temsil
//! eder). Hata: grup içi eğim farkı; 16 değerin tamamı yerine 4 değer.
//!
//! Eğim: Horn (1981) 3 × 3 farkları, enleme göre metrik hücre boyu
//! (ground_inputs.rs ile aynı yöntem). Jessee GMT kullanmış (yöntemi
//! makalede yazmıyor); fark doğrulamada ölçülür.

use crate::aig::{self, Aig};
use crate::vs30::{CODE_LAKE, CODE_NODATA, CODE_OCEAN};
use anyhow::{Result, ensure};
use rayon::prelude::*;
use seismira_core::raster::{self, Encoded, LayerInfo, Raster, Scale};
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

/// Ortalama Dünya yarıçapı (m), IUGG R1.
const R_M: f64 = 6_371_008.8;
/// Alt hücre oranı (30″ / 7,5″).
const K: usize = 4;

pub fn layers() -> Vec<LayerInfo> {
    (0..4)
        .map(|q| LayerInfo {
            name: format!("slope_q{q}"),
            scale: Scale::Linear,
            n_valid: 251,
            lo: 0.0,
            hi: 75.0,
        })
        .collect()
}

pub fn run(md75: &Path, vs30: &Path, output: &Path) -> Result<()> {
    let t0 = std::time::Instant::now();
    let vs = Raster::open(std::fs::read(vs30)?)?;
    let grid = vs.grid;
    let mut dem = Aig::open(md75, crate::gmted_tile_name)?;
    let (w, h) = (grid.width as usize, grid.height as usize);
    ensure!(
        dem.width as usize == K * w
            && dem.height as usize == K * h
            && (dem.west - grid.west).abs() < grid.cell / 10.0
            && (dem.north - grid.north).abs() < grid.cell / 10.0,
        "md75 ızgarası Vs30 ızgarasının 4 katı değil"
    );
    let fw = dem.width as usize;
    let fh = dem.height as usize;
    let bh = dem.block_h() as usize;
    let fine_cell = grid.cell / K as f64;
    let infos = layers();
    let t = grid.tile as usize;
    let (tx_n, ty_n) = (grid.tiles_x() as usize, grid.tiles_y() as usize);
    let mut enc: Vec<Vec<Encoded>> = (0..4).map(|_| Vec::with_capacity(tx_n * ty_n)).collect();

    // Bir döşeme satırı (t kaba satır = 4t ince satır) + üst/alt birer blok kenar.
    let mut fine = vec![0i16; fw * (K * t + 2 * bh)];
    let mut steep = 0u64;
    for ty in 0..ty_n {
        let r0 = ty * t;
        let rows = t.min(h - r0);
        let f0 = K * r0;
        let fa = f0.saturating_sub(bh);
        let fb = (f0 + K * rows + bh).min(fh);
        let buf = &mut fine[..fw * (fb - fa)];
        dem.rows(fa as u32, (fb - fa) as u32, buf)?;
        let buf = &fine[..fw * (fb - fa)];
        let z = |x: usize, y: usize| -> f64 {
            let v = buf[(y - fa) * fw + x];
            if v == aig::NODATA { 0.0 } else { f64::from(v) }
        };
        // Kaba satır başına 4 çeyrek kod: [satır][sütun][q].
        let coarse: Vec<Vec<[u8; 4]>> = (0..rows)
            .into_par_iter()
            .map(|ry| {
                let r = r0 + ry;
                let mut out = vec![[0u8; 4]; w];
                let mut s = [0f32; K * K];
                for (c, o) in out.iter_mut().enumerate() {
                    for sy in 0..K {
                        let y = K * r + sy;
                        let lat = grid.north - (y as f64 + 0.5) * fine_cell;
                        let dy_m = R_M * fine_cell.to_radians();
                        let dx_m = (dy_m * lat.to_radians().cos()).max(1.0);
                        let ym = y.saturating_sub(1).max(fa);
                        let yp = (y + 1).min(fb - 1);
                        let span = (yp - ym) as f64 / 2.0;
                        for sx in 0..K {
                            let x = K * c + sx;
                            let (xm, xp) = ((x + fw - 1) % fw, (x + 1) % fw);
                            let (a, b, cc) = (z(xm, ym), z(x, ym), z(xp, ym));
                            let (d, f) = (z(xm, y), z(xp, y));
                            let (g, hh, i) = (z(xm, yp), z(x, yp), z(xp, yp));
                            let dzdx = ((cc + 2.0 * f + i) - (a + 2.0 * d + g)) / (8.0 * dx_m);
                            let dzdy = ((g + 2.0 * hh + i) - (a + 2.0 * b + cc))
                                / (8.0 * dy_m * span.max(0.5));
                            s[sy * K + sx] = (dzdx.hypot(dzdy)).atan().to_degrees() as f32;
                        }
                    }
                    s.sort_unstable_by(f32::total_cmp);
                    for q in 0..4 {
                        let m = s[4 * q..4 * q + 4].iter().sum::<f32>() / 4.0;
                        o[q] = infos[q].quantize(m);
                    }
                }
                out
            })
            .collect();
        steep += coarse
            .iter()
            .flatten()
            .filter(|q| infos[3].value(q[3]).unwrap_or(0.0) >= 30.0)
            .count() as u64;

        // Döşemelere böl; su hücreleri Vs30 özel kodlarını alır.
        let row_tiles: Vec<[Encoded; 4]> = (0..tx_n)
            .into_par_iter()
            .map(|tx| -> std::io::Result<[Encoded; 4]> {
                let vt = vs.tile(0, tx as u32, ty as u32)?;
                let mut tiles = vec![vec![CODE_NODATA; t * t]; 4];
                for y in 0..rows {
                    for x in 0..t {
                        let gx = tx * t + x;
                        if gx >= w {
                            break;
                        }
                        let code = vt.get(grid.tile, x as u32, y as u32);
                        for q in 0..4 {
                            tiles[q][y * t + x] = match code {
                                CODE_OCEAN | CODE_LAKE => code,
                                _ => coarse[y][gx][q],
                            };
                        }
                    }
                }
                let zstd = |b: &[u8]| zstd::bulk::compress(b, 19);
                Ok([
                    raster::encode(&tiles[0], zstd)?,
                    raster::encode(&tiles[1], zstd)?,
                    raster::encode(&tiles[2], zstd)?,
                    raster::encode(&tiles[3], zstd)?,
                ])
            })
            .collect::<std::io::Result<_>>()?;
        for tl in row_tiles {
            for (q, e) in tl.into_iter().enumerate() {
                enc[q].push(e);
            }
        }
        if ty % 10 == 0 {
            println!(
                "  döşeme satırı {}/{ty_n}, {:.0} s",
                ty + 1,
                t0.elapsed().as_secs_f64()
            );
        }
    }
    println!(
        "CCITT bloğu (RMin ile): {}; en dik çeyreği ≥ 30° olan hücre: {steep}",
        dem.ccitt_blocks
    );
    let mut out = BufWriter::new(File::create(output)?);
    raster::write(&mut out, &grid, &infos, &enc)?;
    drop(out);
    let size = std::fs::metadata(output)?.len();
    println!(
        "{}: {:.1} MB, toplam {:.0} s",
        output.display(),
        size as f64 / 1e6,
        t0.elapsed().as_secs_f64()
    );
    Ok(())
}
