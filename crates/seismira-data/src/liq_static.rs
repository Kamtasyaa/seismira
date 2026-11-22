//! Sıvılaşmanın (Zhu vd. 2017) sarsıntıdan bağımsız kısmını önceden hesaplar
//! → `data/ground/liq_static.sras` (biçim: seismira-core
//! `groundfailure::static_layer`; docs/bilim.md 7.9 "paket boyutu").
//!
//! Girdi: `ground-inputs` (precip_mm, dc_km, dr_km), `slope75` (slope_q0–3),
//! isteğe bağlı `wtd` ve Vs30. Hücre hesabı çekirdeğin ham yoluyla
//! (`liquefaction_grid::raw_base`) aynı işlevdir.
//!
//! Yazılan katmanlar: liq_x ve liq_cls (8 bit). İnce düzeltme (liq_xf)
//! hesaplanıp yalnızca niceleme hatası raporlanır.
//!
//! `coastal_only`: wtd verilmez, her hücre Zhu kıyı modeliyle hesaplanır.
//! DAĞITILACAK paket böyle üretilir: Fan WTD'nin yeniden dağıtım izni yok
//! (docs/veri.md); wtd'den türetilmiş statik katman da dağıtılamaz.

use anyhow::{Context, Result, ensure};
use rayon::prelude::*;
use seismira_core::groundfailure::liquefaction_grid::{Layer, RawInputs, raw_base};
use seismira_core::groundfailure::static_layer::{self, MASKED};
use seismira_core::groundfailure::zhu2017::{self, Variant};
use seismira_core::raster::{self, Encoded, Raster};
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

/// Niceleme hatası özeti için başvuru sarsıntısı (M 7,5; PGV 50 cm/s,
/// PGA 0,3 g): bu sarsıntıda tüm hücrelerin kaplama toplamı.
const REF: (f64, f64, f64) = (50.0, 0.3, 7.5);

#[derive(Default, Clone, Copy)]
struct Stats {
    general: u64,
    coastal: u64,
    clipped: u64,
    exact: f64,
    coarse: f64,
    fine: f64,
    max_rel_coarse: f64,
}

impl Stats {
    fn add(mut self, o: Self) -> Self {
        self.general += o.general;
        self.coastal += o.coastal;
        self.clipped += o.clipped;
        self.exact += o.exact;
        self.coarse += o.coarse;
        self.fine += o.fine;
        self.max_rel_coarse = self.max_rel_coarse.max(o.max_rel_coarse);
        self
    }
}

pub fn run(ground: &Path, vs30: &Path, output: &Path, coastal_only: bool) -> Result<()> {
    let t0 = std::time::Instant::now();
    let open = |p: &Path| -> Result<Raster<Vec<u8>>> {
        Ok(Raster::open(
            std::fs::read(p).with_context(|| p.display().to_string())?,
        )?)
    };
    let vs = open(vs30)?;
    let inputs = open(&ground.join("inputs30.sras"))?;
    let slope = open(&ground.join("slope75.sras"))?;
    let wtd = if coastal_only {
        None
    } else {
        Some(open(&ground.join("wtd.sras")).context("wtd.sras yok (--coastal ile kıyı modeli)")?)
    };
    let grid = vs.grid;
    for r in [&inputs, &slope].into_iter().chain(wtd.as_ref()) {
        ensure!(r.grid == grid, "zemin girdisi Vs30 ızgarasıyla aynı değil");
    }
    let lay = |r: &Raster<Vec<u8>>, name: &str| -> Result<usize> {
        r.layer(name)
            .with_context(|| format!("'{name}' katmanı yok"))
    };
    let ivs = lay(&vs, "vs30")?;
    let (ipr, idc, idr) = (
        lay(&inputs, "precip_mm")?,
        lay(&inputs, "dc_km")?,
        lay(&inputs, "dr_km")?,
    );
    let iq: Vec<usize> = (0..4)
        .map(|k| lay(&slope, &format!("slope_q{k}")))
        .collect::<Result<_>>()?;
    let iw = wtd.as_ref().map(|r| lay(r, "wtd_m")).transpose()?;

    let t = grid.tile as usize;
    let (tx_n, ty_n) = (grid.tiles_x() as usize, grid.tiles_y() as usize);
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
    let (w, h) = (grid.width as usize, grid.height as usize);
    let tiles: Vec<([Encoded; 3], Stats)> = (0..tx_n * ty_n)
        .into_par_iter()
        .map(|k| -> Result<([Encoded; 3], Stats)> {
            let (tx, ty) = (k % tx_n, k / tx_n);
            let vinfo = &vs.layers[ivs];
            let vs30: Vec<f32> = tile_codes(&vs, ivs, tx, ty)?
                .into_iter()
                .map(|c| vinfo.value(c).unwrap_or(f32::NAN))
                .collect();
            let pr = tile_codes(&inputs, ipr, tx, ty)?;
            let dc = tile_codes(&inputs, idc, tx, ty)?;
            let dr = tile_codes(&inputs, idr, tx, ty)?;
            let qs: Vec<Vec<u8>> = iq
                .iter()
                .map(|&li| tile_codes(&slope, li, tx, ty))
                .collect::<Result<_>>()?;
            let wt = match (&wtd, iw) {
                (Some(r), Some(li)) => Some(tile_codes(r, li, tx, ty)?),
                _ => None,
            };
            fn l<'a>(codes: &'a [u8], r: &'a Raster<Vec<u8>>, li: usize) -> Layer<'a> {
                Layer {
                    codes,
                    info: &r.layers[li],
                }
            }
            let ri = RawInputs {
                vs30: &vs30,
                precip_mm: l(&pr, &inputs, ipr),
                dc_km: l(&dc, &inputs, idc),
                dr_km: l(&dr, &inputs, idr),
                slope_q: [0, 1, 2, 3].map(|q| l(&qs[q], &slope, iq[q])),
                wtd_m: match (&wtd, iw, &wt) {
                    (Some(r), Some(li), Some(c)) => Some(l(c, r, li)),
                    _ => None,
                },
            };
            let mut out = [
                vec![MASKED; t * t],
                vec![MASKED; t * t],
                vec![MASKED; t * t],
            ];
            let mut st = Stats::default();
            for y in 0..t.min(h.saturating_sub(ty * t)) {
                for x in 0..t.min(w.saturating_sub(tx * t)) {
                    let i = y * t + x;
                    let Some((xs, flat, v)) = raw_base(&ri, i) else {
                        continue;
                    };
                    let e = static_layer::encode(xs, flat, v);
                    for (o, c) in out.iter_mut().zip(e) {
                        o[i] = c;
                    }
                    match v {
                        Variant::General => st.general += 1,
                        Variant::Coastal => st.coastal += 1,
                    }
                    st.clipped +=
                        u64::from(!(static_layer::X_LO..=static_layer::X_HI).contains(&xs));
                    let cov = |x: f64| {
                        zhu2017::cell_coverage_static(
                            v,
                            x,
                            REF.0,
                            REF.1,
                            REF.2,
                            f64::from(flat) / 4.0,
                        )
                    };
                    let exact = cov(xs);
                    let dec = |f| {
                        static_layer::decode(e[0], f, e[2])
                            .expect("az önce kodlandı")
                            .0
                    };
                    let coarse = cov(dec(None));
                    st.exact += exact;
                    st.coarse += coarse;
                    st.fine += cov(dec(Some(e[1])));
                    st.max_rel_coarse = st.max_rel_coarse.max((coarse / exact - 1.0).abs());
                }
            }
            let zstd = |b: &[u8]| zstd::bulk::compress(b, 19);
            Ok((
                [
                    raster::encode(&out[0], zstd)?,
                    raster::encode(&out[1], zstd)?,
                    raster::encode(&out[2], zstd)?,
                ],
                st,
            ))
        })
        .collect::<Result<_>>()?;

    let mut enc: [Vec<Encoded>; 3] = Default::default();
    let mut st = Stats::default();
    for (e, s) in tiles {
        st = st.add(s);
        for (dst, x) in enc.iter_mut().zip(e) {
            dst.push(x);
        }
    }
    let bytes = |l: &[Encoded]| -> usize {
        l.iter()
            .map(|e| match e {
                Encoded::Zstd(z) => z.len(),
                Encoded::Fill(_) => 0,
            })
            .sum()
    };
    println!(
        "hücre: genel {} / kıyı {} (X aralık dışı, kırpıldı: {})",
        st.general, st.coastal, st.clipped
    );
    println!(
        "başvuru sarsıntısında (M{} PGV {} cm/s) kaplama toplamı: tam {:.1}, 16 bit {:.1} \
         (oran {:.6}), 8 bit {:.1} (oran {:.6}, hücre en büyük göreli fark {:.3})",
        REF.2,
        REF.0,
        st.exact,
        st.fine,
        st.fine / st.exact,
        st.coarse,
        st.coarse / st.exact,
        st.max_rel_coarse
    );
    println!(
        "sıkıştırılmış: liq_x {:.1} MB, liq_xf {:.1} MB, liq_cls {:.1} MB",
        bytes(&enc[0]) as f64 / 1e6,
        bytes(&enc[1]) as f64 / 1e6,
        bytes(&enc[2]) as f64 / 1e6
    );
    // Yalnızca kaba kod yazılır (liq_xf yok): 16 bit paketi ~2 kat büyütüyor,
    // 8 bit toplamı ~%0,03 değiştiriyor (docs/bilim.md 7.9 "paket boyutu").
    let [ex, _ef, ec] = enc;
    let mut out = BufWriter::new(File::create(output)?);
    raster::write(&mut out, &grid, &static_layer::layers(false), &[ex, ec])?;
    drop(out);
    println!(
        "{}: {:.1} MB, {:.0} s",
        output.display(),
        std::fs::metadata(output)?.len() as f64 / 1e6,
        t0.elapsed().as_secs_f64()
    );
    Ok(())
}
