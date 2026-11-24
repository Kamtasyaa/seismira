//! Sıvılaşma doğrulaması (Faz 7 adım 5; docs/dogrulama.md): USGS Ground
//! Failure ürünündeki Zhu 2017 genel model haritası ile bizim hesabımız.
//!
//! Sarsıntı girdisi olarak USGS'in kullandığı ShakeMap ızgarası (grid.xml:
//! PGA %g, PGV cm/s; ShakeMap bileşeni = iki yatay bileşenin büyüğü) verilir;
//! böylece fark yalnızca zemin katmanlarından (Vs30, yağış, kıyıya/nehre
//! uzaklık, su seviyesi, eğim) ve uygulamadan gelir.
//!
//! Karşılaştırma bizim 30″ hücrelerimizde: USGS haritası 15″ (her 30″
//! hücreye 2 × 2 hücre düşer, ortalaması alınır; maskeli NaN hücreler 0).
//! ShakeMap ızgarası daha kaba olabilir (Kahramanmaraş v17: 2′); sarsıntı
//! hücre merkezine çift doğrusal enterpole edilir.
//!
//! Özet: USGS'in "aggregate hazard" ölçüsü (PGA ≥ %10 g hücrelerde Σ kaplama
//! × hücre alanı, km²; info.json hagg0.1g), hücre korelasyonu, sınıf uyumu.

use crate::geotiff::GeoTiff;
use crate::vs30::{CODE_LAKE, CODE_OCEAN};
use anyhow::{Context, Result, bail, ensure};
use seismira_core::groundfailure::zhu2017::{self, Variant};
use seismira_core::raster::Raster;
use std::path::Path;

const R_KM: f64 = 6371.0088;

/// ShakeMap grid.xml: düğüm ızgarası ve PGA (g), PGV (cm/s), SVEL (m/s).
pub(crate) struct ShakeMapGrid {
    pub(crate) lon_min: f64,
    pub(crate) lat_max: f64,
    pub(crate) dlon: f64,
    pub(crate) dlat: f64,
    pub(crate) nlon: usize,
    pub(crate) nlat: usize,
    pub(crate) pga: Vec<f32>,
    pub(crate) pgv: Vec<f32>,
    pub(crate) svel: Vec<f32>,
}

fn attr(s: &str, name: &str) -> Result<f64> {
    let key = format!("{name}=\"");
    let i = s.find(&key).with_context(|| format!("'{name}' yok"))? + key.len();
    let j = s[i..].find('"').context("tırnak")?;
    Ok(s[i..i + j].parse()?)
}

pub(crate) fn read_grid_xml(path: &Path) -> Result<ShakeMapGrid> {
    let text = std::fs::read_to_string(path)?;
    let spec_at = text
        .find("<grid_specification")
        .context("grid_specification yok")?;
    let spec = &text[spec_at..spec_at + text[spec_at..].find("/>").context("spec sonu")?];
    let (lon_min, lon_max) = (attr(spec, "lon_min")?, attr(spec, "lon_max")?);
    let (lat_min, lat_max) = (attr(spec, "lat_min")?, attr(spec, "lat_max")?);
    let nlon = attr(spec, "nlon")? as usize;
    let nlat = attr(spec, "nlat")? as usize;
    // Gerçek aralık: (max − min) / (n − 1) (nominal değer yuvarlanmış).
    let dlon = (lon_max - lon_min) / (nlon - 1) as f64;
    let dlat = (lat_max - lat_min) / (nlat - 1) as f64;
    let mut idx = std::collections::HashMap::new();
    for part in text.split("<grid_field").skip(1) {
        let i = attr(part, "index")? as usize;
        let key = "name=\"";
        let a = part.find(key).context("alan adı")? + key.len();
        let b = part[a..].find('"').context("alan adı sonu")?;
        idx.insert(part[a..a + b].to_owned(), i - 1);
    }
    let col = |n: &str| {
        idx.get(n)
            .copied()
            .with_context(|| format!("'{n}' alanı yok"))
    };
    let (ipga, ipgv, isvel) = (col("PGA")?, col("PGV")?, col("SVEL")?);
    let nf = idx.len();
    let a = text.find("<grid_data>").context("grid_data yok")? + "<grid_data>".len();
    let b = text.find("</grid_data>").context("grid_data sonu")?;
    let n = nlon * nlat;
    let (mut pga, mut pgv, mut svel) = (
        Vec::with_capacity(n),
        Vec::with_capacity(n),
        Vec::with_capacity(n),
    );
    for line in text[a..b].lines().filter(|l| !l.trim().is_empty()) {
        let v: Vec<f32> = line
            .split_whitespace()
            .map(str::parse)
            .collect::<std::result::Result<_, _>>()?;
        ensure!(v.len() == nf, "satırda {} alan, beklenen {nf}", v.len());
        pga.push(v[ipga] / 100.0);
        pgv.push(v[ipgv]);
        svel.push(v[isvel]);
    }
    ensure!(pga.len() == n, "{} düğüm, beklenen {n}", pga.len());
    Ok(ShakeMapGrid {
        lon_min,
        lat_max,
        dlon,
        dlat,
        nlon,
        nlat,
        pga,
        pgv,
        svel,
    })
}

#[derive(Default)]
struct Sums {
    hagg_ours: f64,
    hagg_usgs: f64,
    n: u64,
    // Korelasyon (en az biri > 0 olan hücreler).
    sx: f64,
    sy: f64,
    sxx: f64,
    syy: f64,
    sxy: f64,
    m: u64,
    same_class: u64,
    both_pos: u64,
    ln_ratio: f64,
    general: u64,
    coastal: u64,
}

fn class(c: f64) -> i32 {
    const LIMS: [f64; 6] = [0.005, 0.01, 0.02, 0.05, 0.1, 0.2];
    LIMS.iter().rposition(|&l| c >= l).map_or(-1, |k| k as i32)
}

/// `ground`: data/ground (inputs30, slope75, isteğe bağlı wtd); `vs30_mode`:
/// "map" (bizim Vs30 haritamız) ya da "svel" (ShakeMap'in Vs30'u).
pub fn run(
    grid_xml: &Path,
    usgs_tif: &Path,
    ground: &Path,
    vs30: &Path,
    mw: f64,
    vs30_mode: &str,
) -> Result<()> {
    let sm = read_grid_xml(grid_xml)?;
    println!(
        "ShakeMap: {} × {} düğüm, aralık {:.6}° × {:.6}°",
        sm.nlon, sm.nlat, sm.dlon, sm.dlat
    );
    let use_svel = match vs30_mode {
        "map" => false,
        "svel" => true,
        m => bail!("vs30 kipi 'map' ya da 'svel' olmalı, '{m}' verildi"),
    };
    let open = |p: &Path| -> Result<Raster<Vec<u8>>> {
        Ok(Raster::open(
            std::fs::read(p).with_context(|| p.display().to_string())?,
        )?)
    };
    let inputs = open(&ground.join("inputs30.sras"))?;
    let slope = open(&ground.join("slope75.sras"))?;
    let wtd = open(&ground.join("wtd.sras")).ok();
    let vsr = open(vs30)?;
    let g = vsr.grid;
    ensure!(inputs.grid == slope.grid, "ızgaralar farklı");
    // Bizim hücre penceresi: ShakeMap kapsamı.
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
    let (pr, ipr) = win(&inputs, "precip_mm")?;
    let (dc, idc) = win(&inputs, "dc_km")?;
    let (dr, idr) = win(&inputs, "dr_km")?;
    let qs: Vec<(Vec<u8>, usize)> = (0..4)
        .map(|k| win(&slope, &format!("slope_q{k}")))
        .collect::<Result<_>>()?;
    let wt = match &wtd {
        Some(r) => Some(win(r, "wtd_m")?),
        None => None,
    };
    let (vsc, ivs) = win(&vsr, "vs30")?;
    println!(
        "pencere {w} × {h} hücre; wtd {}",
        if wt.is_some() {
            "var (genel model)"
        } else {
            "YOK (kıyı modeli)"
        }
    );

    // USGS 15″ haritası.
    let mut tif = GeoTiff::open(usgs_tif)?;
    let (tw, th) = tif.size();
    let mut usgs = vec![f32::NAN; (tw * th) as usize];
    tif.rows(0, th, &mut usgs)?;
    let usgs_at = |lon: f64, lat: f64| -> Option<f64> {
        let c = ((lon - tif.west) / tif.cell).floor();
        let r = ((tif.north - lat) / tif.cell).floor();
        if c < 0.0 || r < 0.0 || c >= f64::from(tw) || r >= f64::from(th) {
            return None;
        }
        // Maskelenen hücreler (sarsıntı eşiği, Vs30, eğim) NaN: tehlike 0.
        let v = usgs[(r as u32 * tw + c as u32) as usize];
        Some(if v.is_finite() { f64::from(v) } else { 0.0 })
    };

    let val = |r: &Raster<Vec<u8>>, li: usize, codes: &[u8], i: usize| -> Option<f64> {
        r.layers[li].value(codes[i]).map(f64::from)
    };
    // Tanı (docs/dogrulama.md): SEISMIRA_DIAG_DR_ADD=km nehre uzaklığa,
    // SEISMIRA_DIAG_PRECIP_MUL=k yağışa uygulanır (duyarlılık denemesi).
    let env = |k: &str, d: f64| {
        std::env::var(k)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(d)
    };
    let (dr_add, pr_mul) = (
        env("SEISMIRA_DIAG_DR_ADD", 0.0),
        env("SEISMIRA_DIAG_PRECIP_MUL", 1.0),
    );
    if dr_add != 0.0 || pr_mul != 1.0 {
        println!("TANI: dr + {dr_add} km, yağış × {pr_mul}");
    }
    let mut s = Sums::default();
    for y in 0..h as usize {
        for x in 0..w as usize {
            let i = y * w as usize + x;
            if matches!(vsc[i], CODE_OCEAN | CODE_LAKE) {
                continue;
            }
            // Hücre merkezi; ShakeMap ızgarasından çift doğrusal enterpolasyon
            // (USGS de sarsıntıyı model ızgarasına doğrusal yeniden örnekler).
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
            // USGS'in 2 × 2 alt hücresinin ortalaması.
            let q = g.cell / 4.0;
            let sub: Vec<f64> = [(-q, q), (q, q), (-q, -q), (q, -q)]
                .iter()
                .filter_map(|(dx, dy)| usgs_at(lon + dx, lat + dy))
                .collect();
            if sub.len() < 4 {
                continue;
            }
            let usg = sub.iter().sum::<f64>() / 4.0;
            let vs = if use_svel {
                Some(bil(&sm.svel))
            } else {
                val(&vsr, ivs, &vsc, i)
            };
            let flat = qs
                .iter()
                .filter(|(c, li)| {
                    slope.layers[*li]
                        .value(c[i])
                        .is_some_and(|v| f64::from(v) <= zhu2017::SLOPE_MAX_DEG)
                })
                .count();
            let wv = wt
                .as_ref()
                .and_then(|(c, li)| wtd.as_ref()?.layers[*li].value(c[i]).map(f64::from));
            let ours = match (
                vs,
                val(&inputs, ipr, &pr, i),
                val(&inputs, idc, &dc, i),
                val(&inputs, idr, &dr, i),
            ) {
                (Some(vs), Some(p), Some(dcv), Some(drv)) => {
                    let v = if wv.is_some() {
                        s.general += 1;
                        Variant::General
                    } else {
                        s.coastal += 1;
                        Variant::Coastal
                    };
                    zhu2017::cell_coverage(
                        v,
                        &zhu2017::Inputs {
                            pgv_cms: pgv,
                            pga_g: pga,
                            mw,
                            vs30: vs,
                            precip_mm: p * pr_mul,
                            dc_km: dcv,
                            dr_km: drv + dr_add,
                            wtd_m: wv.unwrap_or(0.0),
                        },
                        flat as f64 / 4.0,
                    )
                }
                _ => continue,
            };
            s.n += 1;
            let area = (R_KM * g.cell.to_radians()).powi(2) * lat.to_radians().cos();
            if pga >= 0.10 {
                s.hagg_ours += ours * area;
                s.hagg_usgs += usg * area;
            }
            if ours > 0.0 || usg > 0.0 {
                s.m += 1;
                s.sx += ours;
                s.sy += usg;
                s.sxx += ours * ours;
                s.syy += usg * usg;
                s.sxy += ours * usg;
                s.same_class += u64::from(class(ours) == class(usg));
            }
            if ours >= 0.005 && usg >= 0.005 {
                s.both_pos += 1;
                s.ln_ratio += (ours / usg).ln();
            }
        }
    }
    let m = s.m as f64;
    let r = (s.sxy / m - (s.sx / m) * (s.sy / m))
        / ((s.sxx / m - (s.sx / m).powi(2)).sqrt() * (s.syy / m - (s.sy / m).powi(2)).sqrt());
    println!(
        "kara hücresi {}, genel {}, kıyı {}",
        s.n, s.general, s.coastal
    );
    println!(
        "toplam tehlike (PGA ≥ %10 g, Σ kaplama × alan): bizim {:.1} km², USGS (30″ ortalama) {:.1} km², oran {:.2}",
        s.hagg_ours,
        s.hagg_usgs,
        s.hagg_ours / s.hagg_usgs
    );
    println!(
        "en az biri > 0 olan {} hücre: korelasyon {r:.3}, aynı sınıf %{:.1}",
        s.m,
        100.0 * s.same_class as f64 / m
    );
    if s.both_pos > 0 {
        println!(
            "ikisi de ≥ %0,5 olan {} hücre: ortalama ln(bizim / USGS) {:+.3} (oran {:.2})",
            s.both_pos,
            s.ln_ratio / s.both_pos as f64,
            (s.ln_ratio / s.both_pos as f64).exp()
        );
    }
    Ok(())
}

/// Uçtan uca: kendi sarsıntı modelimiz (uygulamadaki zincirin aynısı:
/// `grid::compute` + `liquefaction_grid::compute`, adım 1) ile Kahramanmaraş
/// Mw 7,8 senaryosu; USGS haritasıyla aynı özet.
pub fn run_scenario(usgs_tif: &Path, ground: &Path) -> Result<()> {
    use seismira_core::grid::{self, Base, Domain};
    use seismira_core::groundfailure::liquefaction_grid::{
        Inputs, Layer, RawInputs, StaticInputs, compute,
    };
    use seismira_core::groundfailure::static_layer;
    use seismira_core::quake::{FaultType, Quake};
    use seismira_core::shaking::Scenario;
    use seismira_core::site::Vs30Map;
    use seismira_core::traveltime::TravelTimes;
    use std::sync::atomic::AtomicBool;

    let data = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../data"));
    let tt = TravelTimes::load(&data.join("traveltime/ak135.ttt"))?;
    let vs30 = Vs30Map::open(std::fs::read(data.join("vs30/vs30.sras"))?)?;
    // USGS olay sayfası us6000jllz: merkez üssü, derinlik, Mww düğüm düzlemi
    // (validate_damage.rs ile aynı).
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
    let open = |p: &Path| -> Result<Raster<Vec<u8>>> {
        Ok(Raster::open(
            std::fs::read(p).with_context(|| p.display().to_string())?,
        )?)
    };
    let inputs = open(&ground.join("inputs30.sras"))?;
    let slope = open(&ground.join("slope75.sras"))?;
    let wtd = open(&ground.join("wtd.sras")).ok();
    let read = |r: &Raster<Vec<u8>>, name: &str| -> Result<(Vec<u8>, usize)> {
        let li = r.layer(name).with_context(|| format!("'{name}' yok"))?;
        Ok((
            r.window(li, domain.col0, domain.row0, domain.w, domain.h, u8::MAX)?,
            li,
        ))
    };
    let pr = read(&inputs, "precip_mm")?;
    let dc = read(&inputs, "dc_km")?;
    let dr = read(&inputs, "dr_km")?;
    let qs = [
        read(&slope, "slope_q0")?,
        read(&slope, "slope_q1")?,
        read(&slope, "slope_q2")?,
        read(&slope, "slope_q3")?,
    ];
    let wt = match &wtd {
        Some(r) => Some((r, read(r, "wtd_m")?)),
        None => None,
    };
    fn layer<'a>(r: &'a Raster<Vec<u8>>, (codes, li): &'a (Vec<u8>, usize)) -> Layer<'a> {
        Layer {
            codes,
            info: &r.layers[*li],
        }
    }
    let liq = compute(
        &shake,
        &domain,
        step,
        q.mw,
        &Inputs::Raw(RawInputs {
            vs30: &win,
            precip_mm: layer(&inputs, &pr),
            dc_km: layer(&inputs, &dc),
            dr_km: layer(&inputs, &dr),
            slope_q: qs.each_ref().map(|x| layer(&slope, x)),
            wtd_m: wt.as_ref().map(|(r, w)| layer(r, w)),
        }),
        &cancel,
    )
    .context("iptal")?;

    // Önceden hesaplanmış katman (uygulamanın kullandığı) ham yolla aynı
    // sonucu vermeli (16 bit ve yalnızca kaba kodla).
    if let Ok(st) = open(&ground.join(static_layer::FILE)) {
        let x = read(&st, "liq_x")?;
        let cls = read(&st, "liq_cls")?;
        let fine = st
            .layer("liq_xf")
            .map(|_| read(&st, "liq_xf"))
            .transpose()?;
        let total = |c: &[f32]| -> f64 {
            let spec = shake.spec;
            let mut s = 0.0;
            for row in 0..spec.height {
                for col in 0..spec.width {
                    let i = (row * spec.width + col) as usize;
                    let lat = spec.center(col, row).1;
                    let area = (R_KM * spec.cell.to_radians()).powi(2) * lat.to_radians().cos();
                    s += f64::from(c[i]) * area;
                }
            }
            s
        };
        let raw_total = total(&liq.coverage);
        for (label, f) in [("16 bit", fine.as_ref()), ("8 bit", None)] {
            if label == "16 bit" && f.is_none() {
                continue;
            }
            let s = compute(
                &shake,
                &domain,
                step,
                q.mw,
                &Inputs::Static(StaticInputs {
                    x: &x.0,
                    fine: f.map(|f| &f.0[..]),
                    cls: &cls.0,
                }),
                &cancel,
            )
            .context("iptal")?;
            let max_abs = s
                .coverage
                .iter()
                .zip(&liq.coverage)
                .map(|(a, b)| (a - b).abs())
                .fold(0.0f32, f32::max);
            println!(
                "{}: statik katman ({label}) toplam {:.2} km² / ham {:.2} km² (oran {:.4}), \
                 hücre en büyük fark {max_abs:.2e}",
                static_layer::FILE,
                total(&s.coverage),
                raw_total,
                total(&s.coverage) / raw_total
            );
        }
    }

    let mut tif = GeoTiff::open(usgs_tif)?;
    let (tw, th) = tif.size();
    let mut usgs = vec![f32::NAN; (tw * th) as usize];
    tif.rows(0, th, &mut usgs)?;
    let spec = shake.spec;
    let (mut ours, mut theirs, mut ours_all) = (0.0, 0.0, 0.0);
    let (mut cells_ours, mut cells_usgs) = (0u64, 0u64);
    for row in 0..spec.height {
        for col in 0..spec.width {
            let i = (row * spec.width + col) as usize;
            let (lon, lat) = spec.center(col, row);
            let area = (R_KM * spec.cell.to_radians()).powi(2) * lat.to_radians().cos();
            let c = f64::from(liq.coverage[i]);
            ours_all += c * area;
            if f64::from(shake.pga[i]) >= 0.10 {
                ours += c * area;
            }
            cells_ours += u64::from(c >= 0.005);
            // USGS: hücrenin 2 × 2 alt hücresi.
            let q4 = spec.cell / 4.0;
            let mut sum = 0.0;
            let mut n = 0;
            for (dx, dy) in [(-q4, q4), (q4, q4), (-q4, -q4), (q4, -q4)] {
                let cx = ((lon + dx - tif.west) / tif.cell).floor();
                let cy = ((tif.north - (lat + dy)) / tif.cell).floor();
                if cx >= 0.0 && cy >= 0.0 && cx < f64::from(tw) && cy < f64::from(th) {
                    let v = usgs[(cy as u32 * tw + cx as u32) as usize];
                    sum += if v.is_finite() { f64::from(v) } else { 0.0 };
                    n += 1;
                }
            }
            if n == 4 {
                theirs += sum / 4.0 * area;
                cells_usgs += u64::from(sum / 4.0 >= 0.005);
            }
        }
    }
    println!(
        "senaryo alanı {} × {} hücre (adım {step}); genel {}, kıyı {}",
        spec.width, spec.height, liq.general, liq.coastal
    );
    println!(
        "toplam tehlike: bizim {ours:.1} km² (PGA ≥ %10 g; tümü {ours_all:.1}), USGS aynı alanda {theirs:.1} km², oran {:.2}",
        ours / theirs
    );
    println!("≥ %0,5 hücre: bizim {cells_ours}, USGS {cells_usgs}");
    Ok(())
}
