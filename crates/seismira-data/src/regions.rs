//! Natural Earth Admin-1 çokgenleri → bölge kimliği rasterı (`.sras`, iki
//! bayt katmanı) + bölge bilgileri (`.sreg`).
//!
//! Izgara Vs30 rasterından kopyalanır (aynı hücreler; sarsıntı ızgarası
//! ile hizalı). Tarama çizgisi (scanline) dolgusu: her hücre, MERKEZİ
//! çokgenin içindeyse o bölgeye atanır (çift-tek kuralı; delikler ve çok
//! parçalı çokgenler doğru). Hiç hücre merkezi içermeyen küçük bölgeler
//! (adacıklar) için temsil noktasının hücresi, boşsa, o bölgeye verilir.
//! Kimlik = kayıt sırası + 1 (0: bölge yok).
//!
//! Yerleşim biti ([`seismira_core::region::BUILT_BIT`]): GHSL yapılaşma
//! alanı hücre alanının en az [`BUILT_FRACTION`]'ı ise. Hücreye GHSL'nin en
//! yakın hücresi (merkezden) okunur; iki ızgara 0,15 hücre kaymalı.

use crate::geotiff::GeoTiff;
use anyhow::{Context, Result, bail};
use rayon::prelude::*;
use seismira_core::raster::{self, Encoded, Grid, LayerInfo, Raster, Scale};
use seismira_core::region::{BUILT_BIT, Outlines, RegionInfo, write_infos, write_outlines};
use shapefile::dbase::FieldValue;
use shapefile::{PolygonRing, Shape};
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

fn text(rec: &shapefile::dbase::Record, field: &str) -> String {
    match rec.get(field) {
        Some(FieldValue::Character(Some(s))) => s.trim().to_string(),
        _ => String::new(),
    }
}

/// Yerleşim eşiği: hücrenin yapılaşmış oranı. %1 ≈ 30″ hücrede (~0,7 km²,
/// 40° enlem) ~7 000 m²; birkaç binalık bir köy de yerleşim sayılır
/// (seçim, docs/arayuz.md).
pub const BUILT_FRACTION: f64 = 0.01;

/// Bir kesişim: satır, x (boylam), bölge kimliği.
type Crossing = (u32, f64, u16);

pub fn run(shp: &Path, vs30: &Path, ghsl: &Path, out_raster: &Path, out_info: &Path) -> Result<()> {
    let out_outlines = out_raster.with_extension("sout");
    let mut outlines: Outlines = Vec::new();
    let grid = Raster::open(std::fs::read(vs30)?)?.grid;
    let mut built = GeoTiff::open(ghsl)?;
    let (gw, gh) = built.size();
    let (b_west, b_north, b_cell) = (built.west, built.north, built.cell);
    let mut built_rows: Vec<f32> = Vec::new();
    let mut built_cells = 0u64;
    let mut reader = shapefile::Reader::from_path(shp)
        .with_context(|| format!("{} açılamadı", shp.display()))?;
    let mut infos = Vec::new();
    let mut crossings: Vec<Crossing> = Vec::new();
    // Temsil noktaları (bölge, sütun, satır).
    let mut reps: Vec<(u16, u32, u32)> = Vec::new();
    let (west, north, cell) = (grid.west, grid.north, grid.cell);
    for item in reader.iter_shapes_and_records() {
        let (shape, rec) = item?;
        let id = u16::try_from(infos.len() + 1).context("65535'ten fazla bölge")?;
        let iso_a2 = text(&rec, "iso_a2");
        let country = match iso_a2.as_bytes() {
            [a, b] if a.is_ascii_alphabetic() => [*a, *b],
            _ => *b"  ",
        };
        let name = text(&rec, "name");
        let name_en = Some(text(&rec, "name_en"))
            .filter(|s| !s.is_empty())
            .unwrap_or(name.clone());
        let name_tr = Some(text(&rec, "name_tr"))
            .filter(|s| !s.is_empty())
            .unwrap_or(name_en.clone());
        infos.push(RegionInfo {
            country,
            iso: text(&rec, "iso_3166_2"),
            name_en,
            name_tr,
        });
        let poly = match shape {
            Shape::Polygon(p) => p,
            Shape::NullShape => {
                outlines.push(Vec::new());
                continue;
            }
            other => bail!("beklenmeyen şekil: {}", other.shapetype()),
        };
        outlines.push(
            poly.rings()
                .iter()
                .map(|ring| {
                    ring.points()
                        .iter()
                        .map(|p| {
                            let (x, y) = seismira_core::mercator::to_unit(p.x, p.y);
                            [x as f32, y as f32]
                        })
                        .collect()
                })
                .collect(),
        );
        let mut largest: Option<(usize, (f64, f64))> = None;
        for ring in poly.rings() {
            let pts = ring.points();
            if let PolygonRing::Outer(_) = ring
                && largest.is_none_or(|(n, _)| pts.len() > n)
            {
                // Temsil noktası: en büyük dış halkanın köşelerinin ortalaması
                // değil (dışbükey olmayabilir), ilk köşesi.
                largest = Some((pts.len(), (pts[0].x, pts[0].y)));
            }
            for e in pts.windows(2) {
                let (x1, y1, x2, y2) = (e[0].x, e[0].y, e[1].x, e[1].y);
                if y1 == y2 {
                    continue;
                }
                // Kenarın kestiği satır merkezleri: y_c = north − (r + 0.5)·cell,
                // yarı açık kural (y1 ≤ y_c) != (y2 ≤ y_c).
                let (ylo, yhi) = (y1.min(y2), y1.max(y2));
                let r_first = ((north - yhi) / cell - 0.5).ceil().max(0.0) as i64;
                let r_last = ((north - ylo) / cell - 0.5).floor() as i64;
                for r in r_first..=r_last.min(i64::from(grid.height) - 1) {
                    let yc = north - (r as f64 + 0.5) * cell;
                    if (y1 <= yc) != (y2 <= yc) {
                        let x = x1 + (yc - y1) * (x2 - x1) / (y2 - y1);
                        crossings.push((r as u32, x, id));
                    }
                }
            }
        }
        if let Some((_, (x, y))) = largest
            && let Some((c, r)) = grid.cell_of(x, y)
        {
            reps.push((id, c, r));
        }
    }
    println!("{} bölge, {} kesişim", infos.len(), crossings.len());
    crossings.par_sort_unstable_by(|a, b| (a.0, a.2).cmp(&(b.0, b.2)).then(a.1.total_cmp(&b.1)));
    reps.sort_by_key(|r| r.2);

    let w = grid.width as usize;
    let t = grid.tile as usize;
    let (tx_n, ty_n) = (grid.tiles_x() as usize, grid.tiles_y() as usize);
    let mut enc_lo: Vec<Encoded> = Vec::with_capacity(tx_n * ty_n);
    let mut enc_hi: Vec<Encoded> = Vec::with_capacity(tx_n * ty_n);
    let mut band = vec![0u16; w * t];
    let mut ci = 0usize;
    let mut ri = 0usize;
    let mut filled_cells = 0u64;
    for ty in 0..ty_n {
        band.fill(0);
        let r0 = (ty * t) as u32;
        let r1 = r0 + t as u32;
        // Tarama çizgisi dolgusu: (satır, bölge) gruplarında x'leri çiftle.
        while ci < crossings.len() && crossings[ci].0 < r1 {
            let (row, _, id) = crossings[ci];
            let mut j = ci;
            while j < crossings.len() && crossings[j].0 == row && crossings[j].2 == id {
                j += 1;
            }
            let y = (row - r0) as usize;
            for pair in crossings[ci..j].as_chunks::<2>().0 {
                let c0 = ((pair[0].1 - west) / cell - 0.5).ceil().max(0.0) as i64;
                let c1 = ((pair[1].1 - west) / cell - 0.5)
                    .floor()
                    .min((w - 1) as f64) as i64;
                for c in c0..=c1 {
                    band[y * w + c as usize] = id;
                }
            }
            ci = j;
        }
        while ri < reps.len() && reps[ri].2 < r1 {
            let (id, c, r) = reps[ri];
            if r >= r0 {
                let k = (r - r0) as usize * w + c as usize;
                if band[k] == 0 {
                    band[k] = id;
                }
            }
            ri += 1;
        }
        // Yerleşim biti: bandın GHSL satırlarını bir kez oku.
        let lat_top = north - f64::from(r0) * cell;
        let lat_bot = north - f64::from(r1) * cell;
        let gy0 = ((b_north - lat_top) / b_cell).floor().max(0.0) as u32;
        let gy1 = (((b_north - lat_bot) / b_cell).floor() as u32 + 1).min(gh);
        if gy1 > gy0 {
            let n = gy1 - gy0;
            built_rows.clear();
            built_rows.resize(gw as usize * n as usize, f32::NAN);
            built.rows(gy0, n, &mut built_rows)?;
            const R: f64 = 6_371_008.8; // ortalama yer yarıçapı (m)
            for y in 0..t {
                let lat_n = north - (f64::from(r0) + y as f64) * cell;
                let lat_s = lat_n - cell;
                let lat_c = lat_n - cell / 2.0;
                let area = R
                    * R
                    * cell.to_radians()
                    * (lat_n.to_radians().sin() - lat_s.to_radians().sin());
                let gy = ((b_north - lat_c) / b_cell).floor() as i64 - i64::from(gy0);
                if gy < 0 || gy >= i64::from(n) {
                    continue;
                }
                for x in 0..w {
                    let k = y * w + x;
                    if band[k] == 0 {
                        continue;
                    }
                    let lon_c = west + (x as f64 + 0.5) * cell;
                    let gx = ((lon_c - b_west) / b_cell).floor() as i64;
                    if gx < 0 || gx >= i64::from(gw) {
                        continue;
                    }
                    let v = built_rows[gy as usize * gw as usize + gx as usize];
                    if f64::from(v) >= BUILT_FRACTION * area {
                        band[k] |= BUILT_BIT;
                        built_cells += 1;
                    }
                }
            }
        }
        filled_cells += band.iter().filter(|&&v| v != 0).count() as u64;
        // Döşemelere böl, iki bayt katmanı olarak kodla.
        let tiles: Vec<(Vec<u8>, Vec<u8>)> = (0..tx_n)
            .map(|tx| {
                let mut lo = vec![0u8; t * t];
                let mut hi = vec![0u8; t * t];
                for y in 0..t {
                    for x in 0..t {
                        let c = tx * t + x;
                        if c < w {
                            let v = band[y * w + c];
                            lo[y * t + x] = (v & 0xff) as u8;
                            hi[y * t + x] = (v >> 8) as u8;
                        }
                    }
                }
                (lo, hi)
            })
            .collect();
        let zstd = |b: &[u8]| zstd::bulk::compress(b, 19);
        let encoded: Vec<(Encoded, Encoded)> = tiles
            .par_iter()
            .map(|(lo, hi)| Ok((raster::encode(lo, zstd)?, raster::encode(hi, zstd)?)))
            .collect::<std::io::Result<_>>()?;
        for (lo, hi) in encoded {
            enc_lo.push(lo);
            enc_hi.push(hi);
        }
    }

    let layer = |name: &str| LayerInfo {
        name: name.into(),
        scale: Scale::Linear,
        n_valid: 255,
        lo: 0.0,
        hi: 254.0,
    };
    let out_grid = Grid { ..grid };
    let mut out = BufWriter::new(File::create(out_raster)?);
    raster::write(
        &mut out,
        &out_grid,
        &[layer("region_lo"), layer("region_hi")],
        &[enc_lo, enc_hi],
    )?;
    drop(out);
    write_infos(&infos, &mut BufWriter::new(File::create(out_info)?))?;
    write_outlines(
        &outlines,
        &mut BufWriter::new(File::create(&out_outlines)?),
        |b| zstd::bulk::compress(b, 19),
    )?;
    println!(
        "{}: {:.1} MB",
        out_outlines.display(),
        std::fs::metadata(&out_outlines)?.len() as f64 / 1e6
    );
    println!(
        "dolu hücre {filled_cells}, yerleşim {built_cells}; {}: {:.1} MB; {}: {:.0} KB",
        out_raster.display(),
        std::fs::metadata(out_raster)?.len() as f64 / 1e6,
        out_info.display(),
        std::fs::metadata(out_info)?.len() as f64 / 1e3
    );
    Ok(())
}
