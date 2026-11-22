//! Heyelanın (Nowicki Jessee vd. 2018) sarsıntıdan bağımsız kısmını önceden
//! hesaplar → `data/ground/ls_static.sras` (biçim: seismira-core
//! `groundfailure::landslide_static`; docs/bilim.md 7.9).
//!
//! Girdi (hepsi data/ground/ altında, Vs30 ızgarası): `inputs30.sras`
//! (cti), `slope75.sras` (slope_q0–3), `lith.sras` (glim),
//! `landcover.sras` (cci_lc).
//!
//! LİSANS: litoloji GLiM vektöründen (izin bekleniyor, docs/veri.md); bu
//! dosya da izin gelene kadar yalnızca yereldir.

use anyhow::{Context, Result, ensure};
use rayon::prelude::*;
use seismira_core::groundfailure::jessee2018;
use seismira_core::groundfailure::landslide_static::{self as ls, MASKED};
use seismira_core::raster::{self, Encoded, Raster};
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

/// Kuantalama hatası özeti için başvuru sarsıntısı: PGV 50 cm/s, PGA 0,3 g.
const REF: (f64, f64) = (50.0, 0.3);

#[derive(Default, Clone, Copy)]
struct Stats {
    cells: u64,
    no_lith: u64,
    no_lc: u64,
    exact: f64,
    coded: f64,
}

pub fn run(ground: &Path, vs30: &Path, output: &Path) -> Result<()> {
    let t0 = std::time::Instant::now();
    let open = |p: &Path| -> Result<Raster<Vec<u8>>> {
        Ok(Raster::open(
            std::fs::read(p).with_context(|| p.display().to_string())?,
        )?)
    };
    let vs = open(vs30)?;
    let inputs = open(&ground.join("inputs30.sras"))?;
    let slope = open(&ground.join("slope75.sras"))?;
    let lith = open(&ground.join("lith.sras"))?;
    let lc = open(&ground.join("landcover.sras"))?;
    let grid = vs.grid;
    for r in [&inputs, &slope, &lith, &lc] {
        ensure!(r.grid == grid, "zemin girdisi Vs30 ızgarasıyla aynı değil");
    }
    let lay = |r: &Raster<Vec<u8>>, name: &str| -> Result<usize> {
        r.layer(name)
            .with_context(|| format!("'{name}' katmanı yok"))
    };
    let icti = lay(&inputs, "cti")?;
    let ilith = lay(&lith, "glim")?;
    let ilc = lay(&lc, "cci_lc")?;
    let iq: Vec<usize> = (0..4)
        .map(|k| lay(&slope, &format!("slope_q{k}")))
        .collect::<Result<_>>()?;
    // ls_q kodları slope75 kodlarıyla aynı olmalı.
    let li = ls::layers();
    for (k, &q) in iq.iter().enumerate() {
        let (a, b) = (&slope.layers[q], &li[1 + k]);
        ensure!(
            a.lo == b.lo && a.hi == b.hi && a.n_valid == b.n_valid,
            "slope75 kodlaması ls_q ile aynı değil"
        );
    }

    let t = grid.tile as usize;
    let (tx_n, ty_n) = (grid.tiles_x() as usize, grid.tiles_y() as usize);
    let (w, h) = (grid.width as usize, grid.height as usize);
    let tile_codes = |r: &Raster<Vec<u8>>, li: usize, tx: usize, ty: usize| -> Result<Vec<u8>> {
        let tl = r.tile(li, tx as u32, ty as u32)?;
        let mut v = vec![0u8; t * t];
        for y in 0..t {
            for x in 0..t {
                v[y * t + x] = tl.get(grid.tile, x as u32, y as u32);
            }
        }
        Ok(v)
    };
    let tiles: Vec<(Vec<Encoded>, Stats)> = (0..tx_n * ty_n)
        .into_par_iter()
        .map(|k| -> Result<(Vec<Encoded>, Stats)> {
            let (tx, ty) = (k % tx_n, k / tx_n);
            let cti = tile_codes(&inputs, icti, tx, ty)?;
            let gl = tile_codes(&lith, ilith, tx, ty)?;
            let cc = tile_codes(&lc, ilc, tx, ty)?;
            let qs: Vec<Vec<u8>> = iq
                .iter()
                .map(|&li| tile_codes(&slope, li, tx, ty))
                .collect::<Result<_>>()?;
            let mut out = vec![vec![MASKED; t * t]; 5];
            let mut st = Stats::default();
            let cti_info = &inputs.layers[icti];
            let lith_info = &lith.layers[ilith];
            let lc_info = &lc.layers[ilc];
            for y in 0..t.min(h.saturating_sub(ty * t)) {
                for x in 0..t.min(w.saturating_sub(tx * t)) {
                    let i = y * t + x;
                    let kept = [0, 1, 2, 3].map(|q| ls::keep_slope(qs[q][i]));
                    if kept.iter().all(|&c| c == 0) {
                        continue;
                    }
                    let Some(cti) = cti_info.value(cti[i]) else {
                        continue;
                    };
                    let rock = lith_info
                        .value(gl[i])
                        .and_then(|v| jessee2018::lithology_coef(v as u8));
                    let land = lc_info
                        .value(cc[i])
                        .and_then(|v| jessee2018::cci_land_cover_coef(v as u8));
                    let (Some(rock), Some(land)) = (rock, land) else {
                        st.no_lith += u64::from(rock.is_none());
                        st.no_lc += u64::from(land.is_none());
                        continue;
                    };
                    let a = jessee2018::static_part(rock, f64::from(cti), land);
                    let code = ls::encode_a(a);
                    out[0][i] = code;
                    for q in 0..4 {
                        out[1 + q][i] = kept[q];
                    }
                    st.cells += 1;
                    let cov = |a: f64| -> f64 {
                        kept.iter()
                            .map(|&c| {
                                jessee2018::sub_coverage_static(a, REF.0, REF.1, ls::slope_deg(c))
                            })
                            .sum::<f64>()
                            / 4.0
                    };
                    st.exact += cov(a);
                    st.coded += cov(ls::decode_a(code).expect("az önce kodlandı"));
                }
            }
            let zstd = |b: &[u8]| zstd::bulk::compress(b, 19);
            let enc = out
                .iter()
                .map(|o| raster::encode(o, zstd))
                .collect::<std::io::Result<Vec<_>>>()?;
            Ok((enc, st))
        })
        .collect::<Result<_>>()?;

    let mut enc: Vec<Vec<Encoded>> = vec![Vec::new(); 5];
    let mut st = Stats::default();
    for (e, s) in tiles {
        st.cells += s.cells;
        st.no_lith += s.no_lith;
        st.no_lc += s.no_lc;
        st.exact += s.exact;
        st.coded += s.coded;
        for (dst, x) in enc.iter_mut().zip(e) {
            dst.push(x);
        }
    }
    let bytes = |l: &[Encoded]| -> f64 {
        l.iter()
            .map(|e| match e {
                Encoded::Zstd(z) => z.len(),
                Encoded::Fill(_) => 0,
            })
            .sum::<usize>() as f64
            / 1e6
    };
    println!(
        "hücre: {} (litolojisi yok {}, arazi örtüsü yok {}; bu hücreler maskeli)",
        st.cells, st.no_lith, st.no_lc
    );
    println!(
        "başvuru sarsıntısında (PGV {} cm/s, PGA {} g) kaplama toplamı: tam {:.1}, kodlu {:.1} \
         (oran {:.6})",
        REF.0,
        REF.1,
        st.exact,
        st.coded,
        st.coded / st.exact
    );
    let sizes: Vec<String> = li
        .iter()
        .zip(&enc)
        .map(|(l, e)| format!("{} {:.1} MB", l.name, bytes(e)))
        .collect();
    println!("sıkıştırılmış: {}", sizes.join(", "));
    let mut out = BufWriter::new(File::create(output)?);
    raster::write(&mut out, &grid, &li, &enc)?;
    drop(out);
    println!(
        "{}: {:.1} MB, {:.0} s",
        output.display(),
        std::fs::metadata(output)?.len() as f64 / 1e6,
        t0.elapsed().as_secs_f64()
    );
    Ok(())
}
