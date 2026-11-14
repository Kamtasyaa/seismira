//! USGS Küresel Vs30 Mozaiği (2026, CC0) → `.sras`.
//!
//! Girdi: `vs30_mosaic_median_30c.tif` (medyan, m/s) ve
//! `vs30_mosaic_sd_30c.tif` (log standart sapma). Kaynak ızgara olduğu
//! gibi korunur (yeniden örnekleme YOK); yalnızca kuantalanır ve 240 × 240
//! döşemelere bölünür (30″ ızgarada 2° × 2°).
//!
//! Medyan dosyasındaki özel değerler (USGS veri sayfası): 600 okyanus,
//! 601 buz, 602 buzul, 603 göl. Bunlar her iki katmanda da özel kodlara
//! çevrilir; SD dosyasındaki karşılık gelen hücreler değer sayılmaz.

use crate::geotiff::GeoTiff;
use anyhow::{Result, ensure};
use rayon::prelude::*;
use seismira_core::raster::{self, Encoded, LayerInfo, Scale};
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

pub const TILE: u32 = 240;

/// Özel kodlar (her iki katmanda aynı).
pub const CODE_OCEAN: u8 = 251;
pub const CODE_ICE: u8 = 252;
pub const CODE_GLACIER: u8 = 253;
pub const CODE_LAKE: u8 = 254;
pub const CODE_NODATA: u8 = 255;
const N_VALID: u8 = 251;

pub fn layers() -> [LayerInfo; 2] {
    [
        // 80–2500 m/s, logaritmik: en büyük göreli kuantalama hatası %0,7.
        LayerInfo {
            name: "vs30".into(),
            scale: Scale::Log,
            n_valid: N_VALID,
            lo: 80.0,
            hi: 2500.0,
        },
        // ln birimi, doğrusal 0–1,25: adım 0,005.
        LayerInfo {
            name: "vs30_sd".into(),
            scale: Scale::Linear,
            n_valid: N_VALID,
            lo: 0.0,
            hi: 1.25,
        },
    ]
}

fn special(v: f32) -> Option<u8> {
    // Tam eşitlik bilinçli: USGS bu sınıfları tam sayı kodla işaretliyor.
    match v {
        600.0 => Some(CODE_OCEAN),
        601.0 => Some(CODE_ICE),
        602.0 => Some(CODE_GLACIER),
        603.0 => Some(CODE_LAKE),
        _ => None,
    }
}

#[derive(Default)]
struct Stats {
    land: u64,
    special: [u64; 5],
    clamped_lo: u64,
    clamped_hi: u64,
    sd_missing: u64,
    sd_clamped: u64,
    vs_min: f32,
    vs_max: f32,
    sd_min: f32,
    sd_max: f32,
}

pub fn convert(median: &Path, sd: &Path, output: &Path) -> Result<()> {
    let mut med = GeoTiff::open(median)?;
    let mut sdv = GeoTiff::open(sd)?;
    let grid = med.grid(TILE);
    let grid_sd = sdv.grid(TILE);
    ensure!(grid == grid_sd, "medyan ve SD ızgaraları farklı");
    let [lv, ls] = layers();

    let (tx_n, ty_n) = (grid.tiles_x(), grid.tiles_y());
    let w = grid.width as usize;
    let t = TILE as usize;
    let mut band_m = vec![f32::NAN; w * t];
    let mut band_s = vec![f32::NAN; w * t];
    let mut enc_m: Vec<Encoded> = Vec::with_capacity((tx_n * ty_n) as usize);
    let mut enc_s: Vec<Encoded> = Vec::with_capacity((tx_n * ty_n) as usize);
    let mut st = Stats {
        vs_min: f32::MAX,
        vs_max: f32::MIN,
        sd_min: f32::MAX,
        sd_max: f32::MIN,
        ..Default::default()
    };

    for ty in 0..ty_n {
        band_m.fill(f32::NAN);
        band_s.fill(f32::NAN);
        med.rows(ty * TILE, TILE, &mut band_m)?;
        sdv.rows(ty * TILE, TILE, &mut band_s)?;

        let mut row_m = vec![vec![CODE_NODATA; t * t]; tx_n as usize];
        let mut row_s = vec![vec![CODE_NODATA; t * t]; tx_n as usize];
        for y in 0..t {
            for x in 0..w {
                let (m, s) = (band_m[y * w + x], band_s[y * w + x]);
                let (cm, cs) = if m.is_nan() {
                    st.special[4] += 1;
                    (CODE_NODATA, CODE_NODATA)
                } else if let Some(c) = special(m) {
                    st.special[(c - CODE_OCEAN) as usize] += 1;
                    (c, c)
                } else {
                    st.land += 1;
                    st.vs_min = st.vs_min.min(m);
                    st.vs_max = st.vs_max.max(m);
                    st.clamped_lo += u64::from(m < lv.lo);
                    st.clamped_hi += u64::from(m > lv.hi);
                    let cs = if s.is_nan() || special(s).is_some() {
                        st.sd_missing += 1;
                        CODE_NODATA
                    } else {
                        st.sd_min = st.sd_min.min(s);
                        st.sd_max = st.sd_max.max(s);
                        st.sd_clamped += u64::from(s > ls.hi || s < ls.lo);
                        ls.quantize(s)
                    };
                    (lv.quantize(m), cs)
                };
                let i = y * t + x % t;
                row_m[x / t][i] = cm;
                row_s[x / t][i] = cs;
            }
        }
        let zstd = |t: &[u8]| zstd::bulk::compress(t, 19);
        for (row, out) in [(row_m, &mut enc_m), (row_s, &mut enc_s)] {
            let e: Vec<Encoded> = row
                .par_iter()
                .map(|t| raster::encode(t, zstd))
                .collect::<std::io::Result<_>>()?;
            out.extend(e);
        }
        if ty % 10 == 0 {
            println!("  bant {}/{ty_n}", ty + 1);
        }
    }

    println!(
        "toleranslı LZW ile çözülen parça: medyan {}, SD {}",
        med.lenient_chunks, sdv.lenient_chunks
    );
    let mut out = BufWriter::new(File::create(output)?);
    raster::write(&mut out, &grid, &layers(), &[enc_m, enc_s])?;
    drop(out);

    let total = st.land + st.special.iter().sum::<u64>();
    let pct = |n: u64| 100.0 * n as f64 / total.max(1) as f64;
    println!("kara hücresi: {} ({:.1}%)", st.land, pct(st.land));
    println!(
        "okyanus {:.1}%, buz {:.2}%, buzul {:.2}%, göl {:.2}%, veri yok {:.1}%",
        pct(st.special[0]),
        pct(st.special[1]),
        pct(st.special[2]),
        pct(st.special[3]),
        pct(st.special[4])
    );
    println!(
        "Vs30 aralığı {:.1}–{:.1} m/s; alta kırpılan {}, üste kırpılan {}",
        st.vs_min, st.vs_max, st.clamped_lo, st.clamped_hi
    );
    println!(
        "SD aralığı {:.3}–{:.3}; eksik {}, kırpılan {}",
        st.sd_min, st.sd_max, st.sd_missing, st.sd_clamped
    );
    let size = std::fs::metadata(output)?.len();
    println!("{}: {:.1} MB", output.display(), size as f64 / 1e6);
    Ok(())
}
