//! Heyelan doğrulaması (Faz 7; docs/dogrulama.md): USGS Ground Failure
//! ürünündeki Nowicki Jessee (2018) haritası ile bizim hesabımız.
//!
//! Sarsıntı: USGS'in kullandığı ShakeMap ızgarası (grid.xml; PGA %g, PGV
//! cm/s); fark yalnızca zemin katmanlarından (GLiM litolojisi, ESA CCI arazi
//! örtüsü ve GlobCover eşlemesi, CTI, 7,5″ eğim çeyrekleri) gelir. Jessee
//! Vs30 kullanmaz.
//!
//! Karşılaştırma bizim 30″ hücrelerimizde: USGS haritası 7,5″ (her 30″
//! hücreye 4 × 4 hücre; ortalama, maskeli NaN = 0).
//!
//! Tanı: `SEISMIRA_DIAG_LC=open|mean` CCI 60/70'i (yoğunluğu belirsiz
//! orman) GlobCover'ın açık (60/90) sınıfına ya da kapalı ve açık
//! katsayılarının ortalamasına eşler; varsayılan kapalı (50/70; uygulamadaki
//! gibi, `jessee2018::cci_to_globcover`).

use crate::geotiff::GeoTiff;
use crate::validate_liquefaction::read_grid_xml;
use crate::vs30::{CODE_LAKE, CODE_OCEAN};
use anyhow::{Context, Result, ensure};
use seismira_core::groundfailure::jessee2018;
use seismira_core::raster::Raster;
use std::path::Path;

const R_KM: f64 = 6371.0088;

/// Arazi örtüsü katsayısı, tanı kipine göre.
fn land_coef(cci: u8, mode: &str) -> Option<f64> {
    match (cci, mode) {
        (60, "open") => jessee2018::land_cover_coef(60),
        (70, "open") => jessee2018::land_cover_coef(90),
        (60, "mean") => Some((0.30 + 1.77) / 2.0),
        (70, "mean") => Some((1.71 - 1.26) / 2.0),
        _ => jessee2018::cci_land_cover_coef(cci),
    }
}

#[derive(Default)]
struct Sums {
    n: u64,
    hagg_ours: [f64; 2],
    hagg_usgs: [f64; 2],
    m: u64,
    sx: f64,
    sy: f64,
    sxx: f64,
    syy: f64,
    sxy: f64,
    no_input: u64,
}

pub fn run(grid_xml: &Path, usgs_tif: &Path, ground: &Path, vs30: &Path) -> Result<()> {
    let sm = read_grid_xml(grid_xml)?;
    let mode = std::env::var("SEISMIRA_DIAG_LC").unwrap_or_else(|_| "closed".into());
    println!(
        "ShakeMap: {} × {} düğüm; CCI 60/70 eşlemesi: {mode}",
        sm.nlon, sm.nlat
    );
    let open = |p: &Path| -> Result<Raster<Vec<u8>>> {
        Ok(Raster::open(
            std::fs::read(p).with_context(|| p.display().to_string())?,
        )?)
    };
    let inputs = open(&ground.join("inputs30.sras"))?;
    let slope = open(&ground.join("slope75.sras"))?;
    let lith = open(&ground.join("lith.sras"))?;
    let lc = open(&ground.join("landcover.sras"))?;
    let vsr = open(vs30)?;
    let g = vsr.grid;
    for r in [&inputs, &slope, &lith, &lc] {
        ensure!(r.grid == g, "ızgaralar farklı");
    }
    let col0 = ((sm.lon_min - g.west) / g.cell).floor() as i64;
    let row0 = ((g.north - sm.lat_max) / g.cell).floor() as i64;
    let w = ((sm.nlon - 1) as f64 * sm.dlon / g.cell).round() as u32;
    let h = ((sm.nlat - 1) as f64 * sm.dlat / g.cell).round() as u32;
    let win = |r: &Raster<Vec<u8>>, name: &str| -> Result<(Vec<u8>, usize)> {
        let li = r
            .layer(name)
            .with_context(|| format!("'{name}' katmanı yok"))?;
        Ok((r.window(li, col0, row0, w, h, u8::MAX)?, li))
    };
    let (cti, icti) = win(&inputs, "cti")?;
    let (gl, igl) = win(&lith, "glim")?;
    let (cc, icc) = win(&lc, "cci_lc")?;
    let (vsc, _) = win(&vsr, "vs30")?;
    let qs: Vec<(Vec<u8>, usize)> = (0..4)
        .map(|k| win(&slope, &format!("slope_q{k}")))
        .collect::<Result<_>>()?;

    let mut tif = GeoTiff::open(usgs_tif)?;
    let (tw, th) = tif.size();
    let mut usgs = vec![f32::NAN; (tw * th) as usize];
    tif.rows(0, th, &mut usgs)?;
    let k = (g.cell / tif.cell).round() as i64;
    ensure!(
        (g.cell / tif.cell - k as f64).abs() < 1e-6,
        "USGS hücresi bizim hücremizi tam bölmüyor"
    );
    // Bizim hücrenin (lon, lat merkez) k × k USGS hücresinin ortalaması.
    let usgs_cell = |lon: f64, lat: f64| -> Option<f64> {
        let c0 = ((lon - g.cell / 2.0 - tif.west) / tif.cell).round() as i64;
        let r0 = ((tif.north - (lat + g.cell / 2.0)) / tif.cell).round() as i64;
        if c0 < 0 || r0 < 0 || c0 + k > i64::from(tw) || r0 + k > i64::from(th) {
            return None;
        }
        let mut s = 0.0;
        for r in r0..r0 + k {
            for c in c0..c0 + k {
                let v = usgs[(r * i64::from(tw) + c) as usize];
                if v.is_finite() {
                    s += f64::from(v);
                }
            }
        }
        Some(s / (k * k) as f64)
    };

    let mut s = Sums::default();
    for y in 0..h as usize {
        for x in 0..w as usize {
            let i = y * w as usize + x;
            if matches!(vsc[i], CODE_OCEAN | CODE_LAKE) {
                continue;
            }
            let lon = g.west + (col0 as f64 + x as f64 + 0.5) * g.cell;
            let lat = g.north - (row0 as f64 + y as f64 + 0.5) * g.cell;
            let (fx, fy) = ((lon - sm.lon_min) / sm.dlon, (sm.lat_max - lat) / sm.dlat);
            if fx < 0.0 || fy < 0.0 {
                continue;
            }
            let (nx, ny) = (fx.floor() as usize, fy.floor() as usize);
            if nx + 1 >= sm.nlon || ny + 1 >= sm.nlat {
                continue;
            }
            let (tx, ty) = (fx - nx as f64, fy - ny as f64);
            let bil = |a: &[f32]| -> f64 {
                let k = ny * sm.nlon + nx;
                let v = |k: usize| f64::from(a[k]);
                (v(k) * (1.0 - tx) + v(k + 1) * tx) * (1.0 - ty)
                    + (v(k + sm.nlon) * (1.0 - tx) + v(k + sm.nlon + 1) * tx) * ty
            };
            let (pga, pgv) = (bil(&sm.pga), bil(&sm.pgv));
            let Some(usg) = usgs_cell(lon, lat) else {
                continue;
            };
            let rock = lith.layers[igl]
                .value(gl[i])
                .and_then(|v| jessee2018::lithology_coef(v as u8));
            let land = lc.layers[icc]
                .value(cc[i])
                .and_then(|v| land_coef(v as u8, &mode));
            let ctiv = inputs.layers[icti].value(cti[i]);
            let ours = match (rock, land, ctiv) {
                (Some(rock), Some(land), Some(ctiv)) => {
                    let a = jessee2018::static_part(rock, f64::from(ctiv), land);
                    qs.iter()
                        .map(|(c, li)| {
                            let sl = slope.layers[*li].value(c[i]).map_or(0.0, f64::from);
                            jessee2018::sub_coverage_static(a, pgv, pga, sl)
                        })
                        .sum::<f64>()
                        / 4.0
                }
                _ => {
                    s.no_input += 1;
                    0.0
                }
            };
            s.n += 1;
            let area = (R_KM * g.cell.to_radians()).powi(2) * lat.to_radians().cos();
            for (j, thr) in [0.0, 0.10].into_iter().enumerate() {
                if pga >= thr {
                    s.hagg_ours[j] += ours * area;
                    s.hagg_usgs[j] += usg * area;
                }
            }
            if ours > 0.0 || usg > 0.0 {
                s.m += 1;
                s.sx += ours;
                s.sy += usg;
                s.sxx += ours * ours;
                s.syy += usg * usg;
                s.sxy += ours * usg;
            }
        }
    }
    let m = s.m as f64;
    let r = (s.sxy / m - (s.sx / m) * (s.sy / m))
        / ((s.sxx / m - (s.sx / m).powi(2)).sqrt() * (s.syy / m - (s.sy / m).powi(2)).sqrt());
    println!(
        "kara hücresi {} (litoloji / arazi örtüsü / CTI eksik: {})",
        s.n, s.no_input
    );
    for (j, name) in ["tüm hücreler", "PGA ≥ %10 g"].iter().enumerate() {
        println!(
            "toplam tehlike ({name}): bizim {:.1} km², USGS (30″ ortalama) {:.1} km², oran {:.2}",
            s.hagg_ours[j],
            s.hagg_usgs[j],
            s.hagg_ours[j] / s.hagg_usgs[j]
        );
    }
    println!("en az biri > 0 olan {} hücre: korelasyon {r:.3}", s.m);
    Ok(())
}

/// Uçtan uca: kendi sarsıntı modelimiz ve uygulamanın okuduğu
/// `ls_static.sras` (uygulamadaki zincirin aynısı: `grid::compute` +
/// `landslide_grid::compute`) ile Kahramanmaraş Mw 7,8; USGS haritasıyla aynı
/// alanda toplam tehlike.
pub fn run_scenario(usgs_tif: &Path, ground: &Path) -> Result<()> {
    use seismira_core::grid::{self, Base, Domain};
    use seismira_core::groundfailure::landslide_grid::{StaticInputs, compute};
    use seismira_core::groundfailure::landslide_static as ls;
    use seismira_core::quake::{FaultType, Quake};
    use seismira_core::shaking::Scenario;
    use seismira_core::site::Vs30Map;
    use seismira_core::traveltime::TravelTimes;
    use std::sync::atomic::AtomicBool;

    let data = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../data"));
    let tt = TravelTimes::load(&data.join("traveltime/ak135.ttt"))?;
    let vs30 = Vs30Map::open(std::fs::read(data.join("vs30/vs30.sras"))?)?;
    // validate_liquefaction::run_scenario ile aynı (USGS us6000jllz).
    let mut q = Quake::new((37.0143, 37.2256));
    q.depth_km = 10.0;
    q.mw = 7.8;
    q.strike = 227.6;
    q.dip = 89.2;
    q.fault = FaultType::StrikeSlip;
    q.rake = 0.0;
    let scenario = Scenario::new(q, &tt).with_crust_region(crate::crust_region_at(q.epicenter)?);
    let g = vs30.grid();
    let domain = Domain::for_scenario(
        &scenario,
        Base {
            west: g.west,
            north: g.north,
            cell: g.cell,
        },
    );
    let step = domain.fine_step();
    let win = vs30.window(domain.col0, domain.row0, domain.w, domain.h)?;
    let cancel = AtomicBool::new(false);
    let shake = grid::compute(&scenario, &domain, step, Some(&win), &cancel).context("iptal")?;
    let st = Raster::open(std::fs::read(ground.join(ls::FILE))?)?;
    let read = |name: &str| -> Result<Vec<u8>> {
        let li = st.layer(name).with_context(|| format!("'{name}' yok"))?;
        Ok(st.window(li, domain.col0, domain.row0, domain.w, domain.h, ls::MASKED)?)
    };
    let a = read("ls_a")?;
    let q4: Vec<Vec<u8>> = (0..4)
        .map(|k| read(&format!("ls_q{k}")))
        .collect::<Result<_>>()?;
    let res = compute(
        &shake,
        &domain,
        step,
        &StaticInputs {
            a: &a,
            q: [&q4[0], &q4[1], &q4[2], &q4[3]],
        },
        &cancel,
    )
    .context("iptal")?;

    let mut tif = GeoTiff::open(usgs_tif)?;
    let (tw, th) = tif.size();
    let mut usgs = vec![f32::NAN; (tw * th) as usize];
    tif.rows(0, th, &mut usgs)?;
    let spec = shake.spec;
    let k = (spec.cell / tif.cell).round().max(1.0) as i64;
    let (mut ours, mut ours_all, mut theirs) = (0.0, 0.0, 0.0);
    for row in 0..spec.height {
        for col in 0..spec.width {
            let i = (row * spec.width + col) as usize;
            let (lon, lat) = spec.center(col, row);
            let area = (R_KM * spec.cell.to_radians()).powi(2) * lat.to_radians().cos();
            let c = f64::from(res.coverage[i]);
            ours_all += c * area;
            if f64::from(shake.pga[i]) >= 0.10 {
                ours += c * area;
            }
            let c0 = ((lon - spec.cell / 2.0 - tif.west) / tif.cell).round() as i64;
            let r0 = ((tif.north - (lat + spec.cell / 2.0)) / tif.cell).round() as i64;
            if c0 < 0 || r0 < 0 || c0 + k > i64::from(tw) || r0 + k > i64::from(th) {
                continue;
            }
            let mut sum = 0.0;
            for r in r0..r0 + k {
                for cc in c0..c0 + k {
                    let v = usgs[(r * i64::from(tw) + cc) as usize];
                    if v.is_finite() {
                        sum += f64::from(v);
                    }
                }
            }
            theirs += sum / (k * k) as f64 * area;
        }
    }
    println!(
        "senaryo alanı {} × {} hücre (adım {step})",
        spec.width, spec.height
    );
    println!(
        "toplam tehlike: bizim {ours:.1} km² (PGA ≥ %10 g; tümü {ours_all:.1}), USGS aynı alanda \
         (tümü) {theirs:.1} km², oran {:.2} (tümü / tümü {:.2})",
        ours / theirs,
        ours_all / theirs
    );
    Ok(())
}
