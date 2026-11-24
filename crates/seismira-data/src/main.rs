//! Veri hazırlama aracı: ham veriyi (shapefile, GeoTIFF, CSV) uygulamanın
//! hızlı okuduğu kompakt ikili biçime çevirir.
//!
//! Kullanım:
//! ```text
//! seismira-data natural-earth <girdi klasörü> <çıktı klasörü>
//! seismira-data geonames <cities15000.txt> <admin1CodesASCII.txt> <çıktı.scty>
//! seismira-data vs30 <median.tif> <sd.tif> <çıktı.sras>
//! seismira-data city-sites <şehirler.scty> <vs30.sras> <ghsl.tif> <çıktı.ssit>
//! seismira-data regions <admin1.shp> <vs30.sras> <ghsl.tif> <çıktı.sras> <çıktı.sreg>
//! seismira-data exposure <esrm20 klasörü> <admin1.sras> <çıktı.sexp>
//! seismira-data gem <gem klasörü> <admin1.sras> <çıktı.svul>
//! seismira-data slope75 <gmted md75_grd> <vs30.sras> <çıktı.sras>
//! seismira-data wtd <wtd_vs30grid.i16> <vs30.sras> <çıktı.sras>
//! seismira-data liq-static <data/ground> <vs30.sras> <çıktı.sras> [--coastal]
//! seismira-data ground-inputs <gmted md30_grd> <vs30.sras> <chelsa bio12.tif> <ne_10m_rivers.shp> <çıktı.sras>
//! ```

use anyhow::{Context, Result, bail, ensure};
use seismira_core::mapfile::{Layer, LayerKind, MapFile, RESTART};
use seismira_core::mercator;
use shapefile::{Point, Polygon, PolygonRing, Polyline, Shape, ShapeReader};
use std::path::Path;

mod aig;
mod bathy;
mod city_sites;
mod coast_points;
mod exposure;
mod gem;
mod geotiff;
mod ground_inputs;
mod land_cover;
mod liq_static;
mod lith;
mod ls_static;
mod regions;
mod slope75;
mod tsunami_eta;
mod tsunami_scenario;
mod validate_damage;
mod validate_gem;
mod validate_landslide;
mod validate_liquefaction;
mod validate_nga;
mod validate_tsunami;
mod vs30;
mod wtd;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [cmd, input, output] if cmd == "natural-earth" => {
            natural_earth(Path::new(input), Path::new(output))
        }
        [cmd, stations, rest @ ..] if cmd == "validate-shakemap" => {
            validate_shakemap(Path::new(stations), rest)
        }
        [cmd, cities, admin1, output] if cmd == "geonames" => {
            geonames(Path::new(cities), Path::new(admin1), Path::new(output))
        }
        [cmd, median, sd, output] if cmd == "vs30" => {
            vs30::convert(Path::new(median), Path::new(sd), Path::new(output))
        }
        [cmd, cities, vs30, ghsl, output] if cmd == "city-sites" => city_sites::run(
            Path::new(cities),
            Path::new(vs30),
            Path::new(ghsl),
            Path::new(output),
        ),
        [cmd, slab] if cmd == "validate-gem" => validate_gem::run(Path::new(slab)),
        [cmd, flatfile, event] if cmd == "validate-nga" => {
            validate_nga::run(Path::new(flatfile), event, None)
        }
        [cmd, flatfile, event, flag, id] if cmd == "validate-nga" && flag == "--scenario" => {
            validate_nga::run(Path::new(flatfile), event, Some(id))
        }
        [cmd, sbb, dwellings] if cmd == "validate-damage" => {
            validate_damage::run(Path::new(sbb), Path::new(dwellings))
        }
        [cmd, esrm20, regions, output] if cmd == "exposure" => {
            exposure::run(Path::new(esrm20), Path::new(regions), Path::new(output))
        }
        [cmd, gem, regions, output] if cmd == "gem" => {
            gem::run(Path::new(gem), Path::new(regions), Path::new(output))
        }
        [cmd, dir, tifs @ ..] if cmd == "aig-check" => aig_check(Path::new(dir), tifs),
        [cmd, gmted, vs30, chelsa, rivers, output] if cmd == "ground-inputs" => ground_inputs::run(
            Path::new(gmted),
            Path::new(vs30),
            Path::new(chelsa),
            Path::new(rivers),
            Path::new(output),
        ),
        [cmd, grid, usgs, ground, vs30, mw, mode] if cmd == "validate-liquefaction" => {
            validate_liquefaction::run(
                Path::new(grid),
                Path::new(usgs),
                Path::new(ground),
                Path::new(vs30),
                mw.parse()?,
                mode,
            )
        }
        [cmd, usgs, ground] if cmd == "validate-liquefaction-scenario" => {
            validate_liquefaction::run_scenario(Path::new(usgs), Path::new(ground))
        }
        [cmd, ground, vs30, output, rest @ ..]
            if cmd == "liq-static" && (rest.is_empty() || rest == ["--coastal"]) =>
        {
            liq_static::run(
                Path::new(ground),
                Path::new(vs30),
                Path::new(output),
                !rest.is_empty(),
            )
        }
        [cmd, raw, vs30, output] if cmd == "wtd" => {
            wtd::run(Path::new(raw), Path::new(vs30), Path::new(output))
        }
        [cmd, tif, vs30, output] if cmd == "land-cover" => {
            land_cover::run(Path::new(tif), Path::new(vs30), Path::new(output))
        }
        [cmd, grid, usgs, ground, vs30] if cmd == "validate-landslide" => validate_landslide::run(
            Path::new(grid),
            Path::new(usgs),
            Path::new(ground),
            Path::new(vs30),
        ),
        [cmd, usgs, ground] if cmd == "validate-landslide-scenario" => {
            validate_landslide::run_scenario(Path::new(usgs), Path::new(ground))
        }
        [cmd, bathy, cities, output] if cmd == "coast-points" => {
            coast_points::run(Path::new(bathy), Path::new(cities), Path::new(output))
        }
        [cmd, obs, src, mw, dep, strike, dip, rake, setting] if cmd == "validate-tsunami" => {
            let (a, b) = src.split_once(',').context("boylam,enlem")?;
            validate_tsunami::run(
                Path::new(obs),
                (a.parse()?, b.parse()?),
                mw.parse()?,
                dep.parse()?,
                strike.parse()?,
                dip.parse()?,
                rake.parse()?,
                setting,
            )
        }
        [cmd, src, mw, dep, strike, dip, rake, setting, radius, k] if cmd == "tsunami-scenario" => {
            let (a, b) = src.split_once(',').context("boylam,enlem")?;
            tsunami_scenario::run(
                Path::new("data"),
                (a.parse()?, b.parse()?),
                mw.parse()?,
                dep.parse()?,
                strike.parse()?,
                dip.parse()?,
                rake.parse()?,
                setting,
                radius.parse()?,
                k.parse()?,
            )
        }
        [cmd, bathy, src, k, pts @ ..] if cmd == "tsunami-eta" => {
            let pair = |s: &str| -> Result<(f64, f64)> {
                let (a, b) = s.split_once(',').context("boylam,enlem")?;
                Ok((a.trim().parse()?, b.trim().parse()?))
            };
            let pts: Vec<(f64, f64)> = pts.iter().map(|s| pair(s)).collect::<Result<_>>()?;
            tsunami_eta::run(Path::new(bathy), pair(src)?, k.parse()?, &pts)
        }
        [cmd, raw, output] if cmd == "bathy" => bathy::run(Path::new(raw), Path::new(output)),
        [cmd, ground, vs30, output] if cmd == "ls-static" => {
            ls_static::run(Path::new(ground), Path::new(vs30), Path::new(output))
        }
        [cmd, raw, vs30, output] if cmd == "lith" => {
            lith::run(Path::new(raw), Path::new(vs30), Path::new(output))
        }
        [cmd, md75, vs30, output] if cmd == "slope75" => {
            slope75::run(Path::new(md75), Path::new(vs30), Path::new(output))
        }
        [cmd, file, pts @ ..] if cmd == "sras-sample" => {
            let r = seismira_core::raster::Raster::open(std::fs::read(file)?)?;
            for p in pts {
                let (lon, lat) = p.split_once(',').context("nokta: boylam,enlem")?;
                let (lon, lat): (f64, f64) = (lon.parse()?, lat.parse()?);
                let mut line = format!("{p}:");
                for (li, l) in r.layers.iter().enumerate() {
                    let c = r.code_at(li, lon, lat)?;
                    let v = c.and_then(|c| l.value(c));
                    match (c, v) {
                        (_, Some(v)) => line += &format!(" {}={v:.2}", l.name),
                        (Some(c), None) => line += &format!(" {}=#{c}", l.name),
                        _ => line += &format!(" {}=dışarı", l.name),
                    }
                }
                println!("{line}");
            }
            Ok(())
        }
        [cmd, tif, pts @ ..] if cmd == "tif-sample" => {
            let mut t = geotiff::GeoTiff::open(Path::new(tif))?;
            for p in pts {
                let (lon, lat) = p.split_once(',').context("nokta: boylam,enlem")?;
                let v = t.value_at(lon.parse()?, lat.parse()?)?;
                println!("{p}: {v}");
            }
            Ok(())
        }
        [cmd, shp, vs30, ghsl, raster, info] if cmd == "regions" => regions::run(
            Path::new(shp),
            Path::new(vs30),
            Path::new(ghsl),
            Path::new(raster),
            Path::new(info),
        ),
        _ => bail!(
            "kullanım:\n  seismira-data natural-earth <girdi> <çıktı klasörü>\n  \
             seismira-data geonames <cities15000.txt> <admin1CodesASCII.txt> <çıktı.scty>\n  \
             seismira-data vs30 <median.tif> <sd.tif> <çıktı.sras>\n  \
             seismira-data city-sites <şehirler.scty> <vs30.sras> <ghsl.tif> <çıktı.ssit>\n  \
             seismira-data regions <admin1.shp> <vs30.sras> <ghsl.tif> <çıktı.sras> <çıktı.sreg>\n  \
             seismira-data exposure <esrm20 klasörü> <admin1.sras> <çıktı.sexp>\n  \
             seismira-data gem <gem klasörü> <admin1.sras> <çıktı.svul>\n  \
             seismira-data ground-inputs <gmted md30_grd> <vs30.sras> <chelsa bio12.tif> <ne_10m_rivers.shp> <çıktı.sras>\n  \
             seismira-data slope75 <gmted md75_grd> <vs30.sras> <çıktı.sras>\n  \
             seismira-data wtd <wtd_vs30grid.i16> <vs30.sras> <çıktı.sras>\n  \
             seismira-data lith <glim_vs30grid.u8> <vs30.sras> <çıktı.sras>\n  \
             seismira-data validate-landslide <grid.xml> <jessee_2018_model.tif> <data/ground> <vs30.sras>\n  \
             seismira-data validate-landslide-scenario <jessee_2018_model.tif> <data/ground>\n  \
             seismira-data bathy <bathy_1m.i16> <çıktı.sras>\n  \
             seismira-data coast-points <bathy.sras> <cities15000.scty> <çıktı.bin>\n  \
             seismira-data tsunami-eta <bathy.sras> <boylam,enlem> <k> <boylam,enlem>...\n  \
             seismira-data tsunami-scenario <boylam,enlem> <Mw> <derinlik> <doğrultu> <eğim> <kayma açısı> <crust|interface|intraslab> <yarıçap°> <k>\n  \
             seismira-data validate-gem <slab2.sslb>\n  \
             seismira-data validate-tsunami <ncei_runups.json> <boylam,enlem> <Mw> <derinlik> <doğrultu> <eğim> <kayma açısı> <ortam>\n  \
             seismira-data ls-static <data/ground> <vs30.sras> <çıktı.sras>\n  \
             seismira-data land-cover <ESACCI-LC-...-2009-v2.0.7.tif> <vs30.sras> <çıktı.sras>\n  \
             seismira-data liq-static <data/ground> <vs30.sras> <çıktı.sras> [--coastal]\n  \
             seismira-data aig-check <aig klasörü> <geotiff>...\n  \
             seismira-data tif-sample <geotiff> <boylam,enlem>..."
        ),
    }
}

/// GMTED dosya adı: döşeme (sütun, satır) → kök ad. Döşemeler yalnızca
/// satır satır (tek sütun): 0 → `w001001`, 1 → `w001000`, k ≥ 2 →
/// `z001{k−1:03}`. md30'da 2, md75'te 33 dosya; dosya sayısı ve son
/// dosyanın blok sayısı tutuyor, ayrıca GeoTIFF döşemeleriyle
/// karşılaştırarak doğrulandı (`aig-check`, docs/veri.md).
pub fn gmted_tile_name(tx: u32, ty: u32) -> String {
    assert_eq!(tx, 0, "GMTED döşemeleri tek sütun");
    match ty {
        0 => "w001001".into(),
        1 => "w001000".into(),
        k => format!("z001{:03}", k - 1),
    }
}

/// AIG okuyucusunu aynı ızgaradaki GeoTIFF döşemeleriyle karşılaştırır.
fn aig_check(dir: &Path, tifs: &[String]) -> Result<()> {
    let mut aig = aig::Aig::open(dir, gmted_tile_name)?;
    for t in tifs {
        let mut tif = geotiff::GeoTiff::open(Path::new(t))?;
        let (tw, th) = tif.size();
        let dx = (tif.west - aig.west) / aig.cell;
        let dy = (aig.north - tif.north) / aig.cell;
        ensure!(
            (dx - dx.round()).abs() < 1e-3 && (dy - dy.round()).abs() < 1e-3,
            "ızgaralar hizalı değil ({dx}, {dy})"
        );
        let (x0, y0) = (dx.round() as usize, dy.round() as i64);
        let bh = aig.block_h() as i64;
        let ya = y0.div_euclid(bh) * bh;
        let n = ((y0 + i64::from(th)) - ya) as u32;
        let w = aig.width as usize;
        let mut band = vec![0i16; w * n as usize];
        aig.rows(ya as u32, n, &mut band)?;
        let mut trow = vec![0f32; tw as usize * th as usize];
        tif.rows(0, th, &mut trow)?;
        let (mut same, mut diff, mut nd) = (0u64, 0u64, 0u64);
        let mut first = None;
        let mut max_diff = 0f32;
        for y in 0..th as usize {
            let ay = (y0 - ya) as usize + y;
            for x in 0..tw as usize {
                let a = band[ay * w + (x0 + x) % w];
                let g = trow[y * tw as usize + x];
                if g.is_nan() || a == aig::NODATA {
                    nd += 1;
                    if g.is_nan() != (a == aig::NODATA) {
                        diff += 1;
                    }
                } else if f32::from(a) == g {
                    same += 1;
                } else {
                    diff += 1;
                    max_diff = max_diff.max((f32::from(a) - g).abs());
                    first.get_or_insert((x, y, a, g));
                }
            }
        }
        println!(
            "{t}: aynı {same}, farklı {diff} (en büyük {max_diff} m), nodata {nd}; ilk fark \
             {first:?}; CCITT bloğu {}",
            aig.ccitt_blocks
        );
    }
    Ok(())
}

/// Bir istasyonun karşılaştırma satırı. ln PGV'ler: gözlenen, A, B, C.
struct Row {
    code: String,
    obs: f64,
    a: f64,
    b: f64,
    /// ShakeMap Rrup (km).
    rrup: f64,
    /// C (Vs30 haritası verildiyse).
    c: Option<f64>,
    /// Dalma-batmada yay ardı mı (Slab2 verildiyse ve zon bulunduysa).
    backarc: Option<bool>,
}

/// [`compare_shakemap`] sonucu.
struct Compared {
    /// Bizim fay geometrimiz.
    fault: seismira_core::source::Fault,
    rows: Vec<Row>,
    /// ShakeMap'in mesafe vermediği (atlanan) istasyon sayısı.
    missing: usize,
    /// Tek yatay kanalı (bileşen tanımı belirsiz, ör. "UNK") olan istasyon
    /// sayısı; değer olduğu gibi kullanıldı.
    single: usize,
    /// ln(harita Vs30 / istasyon Vs30) ve harita kaynağı sayıları.
    vs_ratio: Vec<f64>,
    vs_src: [usize; 4],
    /// Dalma-batma: Slab2 zonu ve Parker bölgesi.
    slab_zone: Option<String>,
    region: seismira_core::gmpe::psbah20::Region,
}

/// Bir karşılaştırmanın özeti: ortalama ve standart sapma ln(gözlenen /
/// tahmin), ortalama ΔI (JMA aletsel şiddeti), |ΔI| < 1 olanların oranı.
#[derive(Clone, Copy, Debug)]
struct Summary {
    n: usize,
    mean_ln: f64,
    sd_ln: f64,
    mean_di: f64,
    within_one: f64,
}

/// Seçilen karşılaştırma: 0 A, 1 B, 2 C.
fn pick(r: &Row, k: usize) -> Option<f64> {
    match k {
        0 => Some(r.a),
        1 => Some(r.b),
        _ => r.c,
    }
}

/// Satırlardaki `k` karşılaştırmasının özeti.
fn summarize<'a>(rows: impl IntoIterator<Item = &'a Row>, k: usize) -> Option<Summary> {
    use seismira_core::intensity::jma_from_pgv_vector;
    // ΔI için bileşen dönüşümü gerekmez: aynı istasyonda gözlenen ve
    // tahmin edilen PGV aynı oranla çarpılır, farkta birbirini götürür.
    let to_i = |ln_pgv: f64| jma_from_pgv_vector(ln_pgv.exp());
    let pairs: Vec<(f64, f64)> = rows
        .into_iter()
        .filter_map(|r| Some((r.obs, pick(r, k)?)))
        .collect();
    if pairs.len() < 2 {
        return None;
    }
    let res: Vec<f64> = pairs.iter().map(|(o, p)| o - p).collect();
    let di: Vec<f64> = pairs.iter().map(|(o, p)| to_i(*o) - to_i(*p)).collect();
    let n = res.len() as f64;
    let mean = res.iter().sum::<f64>() / n;
    Some(Summary {
        n: res.len(),
        mean_ln: mean,
        sd_ln: (res.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0)).sqrt(),
        mean_di: di.iter().sum::<f64>() / n,
        within_one: di.iter().filter(|x| x.abs() < 1.0).count() as f64 / n,
    })
}

/// ShakeMap istasyon ölçümleriyle karşılaştırma (CLAUDE.md Bölüm 14).
///
/// Gözlenen RotD50 yerine iki yatay bileşenin geometrik ortalaması
/// kullanılır (Boore 2010: ikisi pratikte çok yakın). Tek yatay kanallı
/// istasyonlarda (ör. Puebla 2017 atlas: "UNK") değer olduğu gibi alınır ve
/// ayrıca sayılır. Karşılaştırmalar: (A) ShakeMap'in mesafeleriyle (yalnızca
/// GMPE ve dönüşüm), (B) bizim fay geometrimizle, (C) Vs30 haritası
/// verilirse bizim geometri + bizim Vs30 haritamız (uygulamanın tüm
/// zinciri). A ve B istasyonun ShakeMap Vs30'unu kullanır.
///
/// Model depremin ortamından: sığ kabukta CY14; dalma-batmada Parker vd.
/// (2020), bölge Slab2 verilirse merkez üssünün zonundan (uygulamayla aynı,
/// `Scenario::with_slab`), yoksa küresel. Slab2 verilirse istasyonlar yay
/// önü / yay ardı diye de sınıflanır.
fn compare_shakemap(
    stations: &Path,
    q: &seismira_core::quake::Quake,
    vs30_map: Option<&seismira_core::site::Vs30Map<Vec<u8>>>,
    slab: Option<&seismira_core::slab::Slab2>,
) -> Result<Compared> {
    use seismira_core::gmpe::psbah20::{self, EventType, Region};
    use seismira_core::gmpe::{Imt, cy14};
    use seismira_core::quake::Setting;
    use seismira_core::source::Fault;

    let ours = Fault::from_quake(q);
    let fault = q.fault;
    let event = match q.setting {
        Setting::ShallowCrust => None,
        Setting::Interface => Some(EventType::Interface),
        Setting::Intraslab => Some(EventType::Intraslab),
    };
    let hit = match (event, slab) {
        (Some(_), Some(m)) => m.locate(q.epicenter.0, q.epicenter.1, q.depth_km, q.setting),
        _ => None,
    };
    let region = match (hit, slab) {
        (Some(h), Some(m)) => {
            seismira_core::slab::parker_region(m.code(h.zone), q.epicenter.0, q.epicenter.1)
        }
        _ => Region::Global,
    };
    let (vs_min, vs_max) = match event {
        None => (cy14::VS30_MIN, cy14::VS30_MAX),
        Some(_) => (psbah20::VS30_MIN, psbah20::VS30_MAX),
    };
    let predict = |rrup: f64, rjb: f64, rx: f64, ztor: f64, vs30: f64| match event {
        None => cy14::predict(
            Imt::Pgv,
            &cy14::Input {
                mw: q.mw,
                rrup,
                rjb,
                rx,
                ztor,
                dip: q.dip,
                fault,
                vs30,
            },
        )
        .map(|m| m.mean),
        Some(event) => psbah20::predict(
            Imt::Pgv,
            &psbah20::Input {
                mw: q.mw,
                rrup,
                zhyp: q.depth_km,
                vs30,
                event,
                region,
            },
        )
        .map(|m| m.mean),
    };
    let json: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(stations)?)?;
    let feats = json["features"].as_array().context("features yok")?;
    let mut rows: Vec<Row> = Vec::new();
    let mut vs_ratio: Vec<f64> = Vec::new();
    let mut vs_src = [0usize; 4];
    let mut missing = 0usize;
    let mut single = 0usize;
    for f in feats {
        let p = &f["properties"];
        if p["station_type"] != "seismic" {
            continue;
        }
        let channels: Vec<(&str, f64)> = p["channels"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|c| {
                let name = c["name"].as_str()?;
                let v = c["amplitudes"]
                    .as_array()?
                    .iter()
                    .find(|a| a["name"] == "pgv")?["value"]
                    .as_f64()?;
                Some((name, v))
            })
            .filter(|(n, _)| !n.ends_with('Z'))
            .collect();
        let obs = match channels[..] {
            [(_, h1), (_, h2)] => (h1.ln() + h2.ln()) / 2.0,
            [("UNK", v)] => {
                single += 1;
                v.ln()
            }
            _ => continue,
        };
        let vs30 = p["vs30"].as_f64().context("vs30")?;
        let d = &p["distances"];
        let coords = f["geometry"]["coordinates"].as_array().context("konum")?;
        let site = (
            coords[0].as_f64().context("boylam")?,
            coords[1].as_f64().context("enlem")?,
        );
        let (Some(rrup), Some(rjb)) = (d["rrup"].as_f64(), d["rjb"].as_f64()) else {
            missing += 1; // ShakeMap bazı istasyonlarda mesafe vermiyor
            continue;
        };
        // Yeni ShakeMap sürümleri Rx vermeyebilir: taban bloğu kabul edilir.
        // (Tavan bloğu terimi cos δ ile çarpılır; dik faylarda etkisi ~0.)
        let rx = d["rx"].as_f64().unwrap_or(-rjb);
        // Kabukta ShakeMap kırılması yüzeyden başlıyor (rupture.json):
        // Ztor = 0 (yalnızca CY14 kullanır).
        let a = predict(rrup, rjb, rx, 0.0, vs30.clamp(vs_min, vs_max));
        let od = ours.distances(site);
        let b = predict(
            od.rrup,
            od.rjb,
            od.rx,
            ours.ztor,
            vs30.clamp(vs_min, vs_max),
        );
        let c = match vs30_map {
            Some(m) => {
                let s = m.site(site.0, site.1)?;
                vs_src[s.source as usize] += 1;
                vs_ratio.push((s.vs30 / vs30).ln());
                predict(
                    od.rrup,
                    od.rjb,
                    od.rx,
                    ours.ztor,
                    s.vs30.clamp(vs_min, vs_max),
                )
            }
            None => None,
        };
        let backarc = match (hit, slab) {
            (Some(h), Some(m)) => m.is_backarc(h.zone, site.0, site.1),
            _ => None,
        };
        if let (Some(a), Some(b)) = (a, b) {
            rows.push(Row {
                code: p["code"].as_str().unwrap_or("?").to_string(),
                obs,
                a,
                b,
                rrup,
                c,
                backarc,
            });
        }
    }
    Ok(Compared {
        fault: ours,
        rows,
        missing,
        single,
        vs_ratio,
        vs_src,
        slab_zone: hit.zip(slab).map(|(h, m)| m.code(h.zone).to_string()),
        region,
    })
}

/// `validate-shakemap` komutu: [`compare_shakemap`] sonucunu tablo olarak
/// yazar. Argümanlar: `<stationlist.json> boylam enlem derinlik Mw doğrultu
/// eğim fay(SS|RS|NS) [hiposantr_doğrultu_konumu] [--vs30 <dosya.sras>]
/// [--setting crust|interface|intraslab] [--slab <slab2.sslb>]
/// [--csv <istasyonlar.csv>]`; ya da konum argümanları yerine
/// `--scenario <kimlik>` (hazır senaryo).
fn validate_shakemap(stations: &Path, args: &[String]) -> Result<()> {
    use seismira_core::quake::{FaultType, Quake, Setting};

    let mut args = args.to_vec();
    let mut flag = |name: &str| -> Result<Option<String>> {
        Ok(match args.iter().position(|a| a == name) {
            Some(i) => {
                let v = args
                    .get(i + 1)
                    .with_context(|| format!("{name} değeri"))?
                    .clone();
                args.drain(i..i + 2);
                Some(v)
            }
            None => None,
        })
    };
    let vs30_map = match flag("--vs30")? {
        Some(path) => {
            let bytes = std::fs::read(&path).with_context(|| path.clone())?;
            Some(seismira_core::site::Vs30Map::open(bytes)?)
        }
        None => None,
    };
    let slab = match flag("--slab")? {
        Some(path) => {
            Some(seismira_core::slab::Slab2::load(Path::new(&path)).with_context(|| path.clone())?)
        }
        None => None,
    };
    let setting = match flag("--setting")?.as_deref() {
        None | Some("crust") => Setting::ShallowCrust,
        Some("interface") => Setting::Interface,
        Some("intraslab") => Setting::Intraslab,
        Some(s) => bail!("ortam: {s} (crust|interface|intraslab)"),
    };
    let csv = flag("--csv")?;
    let scenario = flag("--scenario")?;
    let args = args.as_slice();
    if let Some(id) = scenario {
        // Hazır senaryo (data/scenarios/scenarios.json): uygulamanın kullandığı
        // depremin aynısı (elle kırılma boyutu ve odak konumu dahil).
        if !args.is_empty() {
            bail!("--scenario ile konum argümanları verilmez");
        }
        let q = scenario_quake(&id)?;
        return report_shakemap(stations, &q, vs30_map.as_ref(), slab.as_ref(), csv);
    }
    let (base, along) = match args {
        [b @ .., a] if args.len() == 8 => (b, Some(a.parse::<f64>()?)),
        b => (b, None),
    };
    let [lon, lat, depth, mw, strike, dip, fault] = base else {
        bail!(
            "argümanlar: boylam enlem derinlik Mw doğrultu eğim SS|RS|NS [hiposantr_doğrultu_konumu]"
        );
    };
    let fault = match fault.as_str() {
        "SS" => FaultType::StrikeSlip,
        "RS" => FaultType::Reverse,
        "NS" => FaultType::Normal,
        f => bail!("fay tipi: {f}"),
    };
    let mut q = Quake::new((lon.parse()?, lat.parse()?));
    q.depth_km = depth.parse()?;
    q.mw = mw.parse()?;
    q.strike = strike.parse()?;
    q.dip = dip.parse()?;
    q.fault = fault;
    q.setting = setting;
    if let Some(a) = along {
        q.hypo_along = a;
    }
    report_shakemap(stations, &q, vs30_map.as_ref(), slab.as_ref(), csv)
}

/// Hazır senaryonun depremi (uygulamadaki `scenarios::parse` ile aynı alanlar).
fn scenario_quake(id: &str) -> Result<seismira_core::quake::Quake> {
    use seismira_core::quake::{FaultType, Quake, Setting};
    let path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/scenarios/scenarios.json"
    ));
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path)?)?;
    let s = v["scenarios"]
        .as_array()
        .context("scenarios")?
        .iter()
        .find(|s| s["id"] == id)
        .with_context(|| format!("senaryo yok: {id}"))?;
    let num = |k: &str| s[k].as_f64().with_context(|| k.to_string());
    let mut q = Quake::new((num("lon")?, num("lat")?));
    q.mw = num("mw")?;
    q.depth_km = num("depth_km")?;
    q.setting = match s["setting"].as_str() {
        Some("interface") => Setting::Interface,
        Some("intraslab") => Setting::Intraslab,
        _ => Setting::ShallowCrust,
    };
    q.strike = num("strike")?;
    q.dip = num("dip")?;
    q.rake = num("rake")?;
    q.fault = FaultType::from_rake(q.rake);
    q.hypo_along = num("hypo_along")?;
    q.hypo_down = num("hypo_down")?;
    q.rupture_km = Some([num("length_km")?, num("width_km")?]);
    Ok(q)
}

fn report_shakemap(
    stations: &Path,
    q: &seismira_core::quake::Quake,
    vs30_map: Option<&seismira_core::site::Vs30Map<Vec<u8>>>,
    slab: Option<&seismira_core::slab::Slab2>,
    csv: Option<String>,
) -> Result<()> {
    use seismira_core::intensity::jma_from_pgv_vector;
    let q = *q;
    let Compared {
        fault: ours,
        rows,
        missing,
        single,
        vs_ratio,
        vs_src,
        slab_zone,
        region,
    } = compare_shakemap(stations, &q, vs30_map, slab)?;

    let to_i = |ln_pgv: f64| jma_from_pgv_vector(ln_pgv.exp());
    println!(
        "fay (bizim): L = {:.0} km, W = {:.0} km, Ztor = {:.1} km, hiposantr doğrultu konumu {:.2}",
        ours.length, ours.width, ours.ztor, q.hypo_along
    );
    if q.setting != seismira_core::quake::Setting::ShallowCrust {
        println!(
            "Parker vd. (2020), bölge {} (Slab2 zonu: {})",
            region.key(),
            slab_zone.as_deref().unwrap_or("yok")
        );
    }
    println!(
        "{} istasyon (model aralığında); mesafesi eksik {} istasyon atlandı; tek kanallı {}\n",
        rows.len(),
        missing,
        single
    );
    println!("| karşılaştırma | n | ort. ln(göz/tah) | std ln | ort. ΔI (JMA) | |ΔI| < 1 oranı |");
    println!("|---|---|---|---|---|---|");
    let mut comps = vec![
        ("A: ShakeMap mesafeleri", 0),
        ("B: bizim fay geometrimiz", 1),
    ];
    if vs30_map.is_some() {
        comps.push(("C: bizim geometri + bizim Vs30", 2));
    }
    let line = |label: &str, s: Option<Summary>| match s {
        Some(s) => println!(
            "| {label} | {} | {:+.2} | {:.2} | {:+.2} | %{:.0} |",
            s.n,
            s.mean_ln,
            s.sd_ln,
            s.mean_di,
            s.within_one * 100.0
        ),
        None => println!("| {label} | – | – | – | – | – |"),
    };
    for &(label, k) in &comps {
        line(label, summarize(&rows, k));
    }
    if rows.iter().any(|r| r.backarc.is_some()) {
        println!("\nYay önü / yay ardı (Slab2, levha 105 km eş-derinliği):\n");
        println!(
            "| karşılaştırma | n | ort. ln(göz/tah) | std ln | ort. ΔI (JMA) | |ΔI| < 1 oranı |"
        );
        println!("|---|---|---|---|---|---|");
        for (side, want) in [
            ("yay önü", Some(false)),
            ("yay ardı", Some(true)),
            ("sınıflanmadı", None),
        ] {
            let sel: Vec<&Row> = rows.iter().filter(|r| r.backarc == want).collect();
            if sel.is_empty() {
                continue;
            }
            for &(label, k) in &comps {
                line(
                    &format!("{side}, {}", &label[..1]),
                    summarize(sel.iter().copied(), k),
                );
            }
        }
    }
    if !vs_ratio.is_empty() {
        let n = vs_ratio.len() as f64;
        let m = vs_ratio.iter().sum::<f64>() / n;
        let sd = (vs_ratio.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (n - 1.0)).sqrt();
        println!(
            "\nVs30 harita/istasyon: ort. ln oran {m:+.2} (×{:.2}), std {sd:.2}; \
             kaynak: harita {}, komşu {}, referans {}",
            m.exp(),
            vs_src[0],
            vs_src[1],
            vs_src[2]
        );
    }
    // Mesafeye göre (ShakeMap Rrup) ortalama ΔI: uzak alan eğilimi.
    println!(
        "\n| Rrup (km) | n | ort. ΔI (A) | ort. ΔI (B) | ort. ΔI (C) |\n|---|---|---|---|---|"
    );
    for (lo, hi) in [
        (0.0, 25.0),
        (25.0, 50.0),
        (50.0, 100.0),
        (100.0, 150.0),
        (150.0, 200.0),
        (200.0, 300.0),
        (300.0, 500.0),
        (500.0, 1000.0),
    ] {
        let sel: Vec<&Row> = rows
            .iter()
            .filter(|r| r.rrup >= lo && r.rrup < hi)
            .collect();
        if sel.is_empty() {
            continue;
        }
        let mean_di = |k: usize| {
            let v: Vec<f64> = sel
                .iter()
                .filter_map(|r| Some(to_i(r.obs) - to_i(pick(r, k)?)))
                .collect();
            if v.is_empty() {
                "–".to_string()
            } else {
                format!("{:+.2}", v.iter().sum::<f64>() / v.len() as f64)
            }
        };
        println!(
            "| {lo:.0}–{hi:.0} | {} | {} | {} | {} |",
            sel.len(),
            mean_di(0),
            mean_di(1),
            mean_di(2)
        );
    }
    if let Some(path) = csv {
        let mut out = String::from("code,rrup,obs_ln_pgv,a_ln_pgv,b_ln_pgv,c_ln_pgv,backarc\n");
        for r in &rows {
            out += &format!(
                "{},{:.2},{:.4},{:.4},{:.4},{},{}\n",
                r.code,
                r.rrup,
                r.obs,
                r.a,
                r.b,
                r.c.map_or(String::new(), |c| format!("{c:.4}")),
                r.backarc.map_or("", |b| if b { "1" } else { "0" })
            );
        }
        std::fs::write(&path, out).with_context(|| path.clone())?;
        println!("\nistasyon ayrıntısı: {path}");
    }
    Ok(())
}

/// GeoNames şehir dökümünü `.scty` biçimine çevirir (nüfusa göre azalan).
/// Sütunlar: GeoNames readme.txt, "geoname" tablosu (0 tabanlı: 1 ad,
/// 4 enlem, 5 boylam, 7 özellik kodu, 8 ülke, 10 admin1 kodu, 14 nüfus).
fn geonames(cities_txt: &Path, admin1_txt: &Path, output: &Path) -> Result<()> {
    use seismira_core::cities::{City, write};
    use std::collections::HashMap;

    let admin1: HashMap<String, String> = std::fs::read_to_string(admin1_txt)?
        .lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            // GeoNames bazı admin1 adlarına İngilizce tür eki koyar
            // ("Afyonkarahisar Province"); bölge adı olarak gereksiz.
            let name = f.get(1)?;
            let name = ["Province", "Prefecture"]
                .iter()
                .find_map(|sfx| name.strip_suffix(&format!(" {sfx}")))
                .unwrap_or(name);
            Some((f.first()?.to_string(), name.to_string()))
        })
        .collect();
    let mut cities = Vec::new();
    for (n, line) in std::fs::read_to_string(cities_txt)?.lines().enumerate() {
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() < 19 {
            bail!("satır {}: {} sütun", n + 1, f.len());
        }
        let country: [u8; 2] = f[8]
            .as_bytes()
            .try_into()
            .with_context(|| format!("satır {}: ülke kodu '{}'", n + 1, f[8]))?;
        cities.push(City {
            name: f[1].to_string(),
            admin1: admin1
                .get(&format!("{}.{}", f[8], f[10]))
                .cloned()
                .unwrap_or_default(),
            country,
            lat: f[4].parse()?,
            lon: f[5].parse()?,
            population: f[14].parse().unwrap_or(0),
            capital: f[7] == "PPLC",
        });
    }
    cities.sort_by(|a, b| {
        b.population
            .cmp(&a.population)
            .then_with(|| a.name.cmp(&b.name))
    });
    if let Some(dir) = output.parent() {
        std::fs::create_dir_all(dir)?;
    }
    write(
        &cities,
        &mut std::io::BufWriter::new(std::fs::File::create(output)?),
    )?;
    let no_admin = cities.iter().filter(|c| c.admin1.is_empty()).count();
    println!(
        "→ {} ({} şehir, {} şehirde il/eyalet adı yok, {:.1} MB)",
        output.display(),
        cities.len(),
        no_admin,
        std::fs::metadata(output)?.len() as f64 / 1e6
    );
    Ok(())
}

/// Natural Earth katmanlarını ölçek başına bir `.smap` dosyasına çevirir.
fn natural_earth(input: &Path, output: &Path) -> Result<()> {
    std::fs::create_dir_all(output)?;
    for scale in ["110m", "50m", "10m"] {
        let shp = |name: &str| input.join(format!("ne_{scale}_{name}.shp"));
        let layers = vec![
            polygons("land", &shp("land"))?,
            polygons("lakes", &shp("lakes"))?,
            lines("admin0", &shp("admin_0_boundary_lines_land"))?,
            lines("admin1", &shp("admin_1_states_provinces_lines"))?,
        ];
        let path = output.join(format!("ne_{scale}.smap"));
        let mut file = std::io::BufWriter::new(std::fs::File::create(&path)?);
        MapFile { layers }.write(&mut file)?;
        drop(file);
        let size = std::fs::metadata(&path)?.len();
        println!("→ {} ({:.1} MB)", path.display(), size as f64 / 1e6);
    }
    Ok(())
}

/// Tüm şekilleri okur; boş (NullShape) kayıtlar çağıran tarafta atlanır.
fn read_shapes(path: &Path) -> Result<Vec<Shape>> {
    ShapeReader::from_path(path)
        .and_then(|r| r.read())
        .with_context(|| format!("{} okunamadı", path.display()))
}

fn project(p: &Point) -> [f64; 2] {
    let (x, y) = mercator::to_unit(p.x, p.y);
    [x, y]
}

/// Halkanın kapanış noktasını (ilk noktanın tekrarı) atar.
fn open_ring(points: &[Point]) -> &[Point] {
    match points {
        [first, .., last] if first == last => &points[..points.len() - 1],
        _ => points,
    }
}

/// 180° meridyenini geçen kenar sayısı. Natural Earth geometrisi ±180°'de
/// bölünmüş olmalı (CLAUDE.md Bölüm 8); geçen kenar varsa uyarılır.
/// Kutup boyunca uzanan kenarlar (Antarktika'nın kapanışı, enlem ≈ −90°)
/// gerçek bir geçiş değildir; Mercator'da haritanın alt kenarına düşer.
fn antimeridian_crossings(points: &[Point]) -> usize {
    points
        .windows(2)
        .filter(|w| (w[1].x - w[0].x).abs() > 180.0)
        .filter(|w| w[0].y.abs().min(w[1].y.abs()) < 89.9)
        .count()
}

fn polygons(name: &str, path: &Path) -> Result<Layer> {
    let shapes: Vec<Polygon> = read_shapes(path)?
        .into_iter()
        .filter_map(|s| match s {
            Shape::Polygon(p) => Some(Ok(p)),
            Shape::NullShape => None,
            other => Some(Err(anyhow::anyhow!(
                "{name}: beklenmeyen şekil {}",
                other.shapetype()
            ))),
        })
        .collect::<Result<_>>()?;

    let mut vertices: Vec<[f32; 2]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut earcut = earcut::Earcut::new();
    let mut tris: Vec<u32> = Vec::new();
    let (mut parts, mut crossings) = (0usize, 0usize);

    let mut orphan_holes = 0usize;
    for shape in &shapes {
        // Her deliği, gerçekten içinde bulunduğu dış halkaya bağla. Shapefile
        // sırası buna güvenmeye yetmez: Natural Earth 10m karada Hazar Denizi
        // deliği Avrasya'nın hemen ardından gelmez.
        let mut groups: Vec<Vec<&[Point]>> = Vec::new();
        let mut bboxes: Vec<[f64; 4]> = Vec::new();
        let mut holes: Vec<&[Point]> = Vec::new();
        for ring in shape.rings() {
            let pts = open_ring(ring.points());
            crossings += antimeridian_crossings(pts);
            match ring {
                PolygonRing::Outer(_) => {
                    bboxes.push(bbox(pts));
                    groups.push(vec![pts]);
                }
                PolygonRing::Inner(_) => holes.push(pts),
            }
        }
        for hole in holes {
            let Some(probe) = hole.first() else { continue };
            // İç içe dış halkalar varsa en küçük kapsayan seçilir.
            let owner = (0..groups.len())
                .filter(|&i| in_bbox(&bboxes[i], probe) && point_in_ring(groups[i][0], probe))
                .min_by(|&a, &b| bbox_area(&bboxes[a]).total_cmp(&bbox_area(&bboxes[b])));
            match owner {
                Some(i) => groups[i].push(hole),
                None => orphan_holes += 1,
            }
        }
        for group in groups {
            let mut flat: Vec<[f64; 2]> = Vec::new();
            let mut holes: Vec<u32> = Vec::new();
            for (i, ring) in group.iter().enumerate() {
                if i > 0 {
                    holes.push(flat.len() as u32);
                }
                flat.extend(ring.iter().map(project));
            }
            if flat.len() < 3 {
                continue;
            }
            earcut.earcut(flat.iter().copied(), &holes, &mut tris);
            let base = vertices.len() as u32;
            vertices.extend(flat.iter().map(|&[x, y]| [x as f32, y as f32]));
            indices.extend(tris.iter().map(|&i| base + i));
            parts += 1;
        }
    }
    report(
        name,
        path,
        shapes.len(),
        parts,
        vertices.len(),
        indices.len() / 3,
        crossings,
    );
    if orphan_holes > 0 {
        bail!("{name}: hiçbir dış halkanın içinde olmayan {orphan_holes} delik");
    }
    Ok(Layer {
        name: name.to_owned(),
        kind: LayerKind::Triangles,
        vertices,
        indices,
    })
}

fn bbox(pts: &[Point]) -> [f64; 4] {
    pts.iter().fold(
        [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ],
        |b, p| [b[0].min(p.x), b[1].min(p.y), b[2].max(p.x), b[3].max(p.y)],
    )
}

fn bbox_area(b: &[f64; 4]) -> f64 {
    (b[2] - b[0]) * (b[3] - b[1])
}

fn in_bbox(b: &[f64; 4], p: &Point) -> bool {
    p.x >= b[0] && p.x <= b[2] && p.y >= b[1] && p.y <= b[3]
}

/// Işın atma (çift-tek kuralı) ile nokta-halka içerme testi.
fn point_in_ring(ring: &[Point], p: &Point) -> bool {
    let mut inside = false;
    let mut j = ring.len().wrapping_sub(1);
    for i in 0..ring.len() {
        let (a, b) = (&ring[i], &ring[j]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
        j = i;
    }
    inside
}

fn lines(name: &str, path: &Path) -> Result<Layer> {
    let shapes: Vec<Polyline> = read_shapes(path)?
        .into_iter()
        .filter_map(|s| match s {
            Shape::Polyline(p) => Some(Ok(p)),
            Shape::NullShape => None,
            other => Some(Err(anyhow::anyhow!(
                "{name}: beklenmeyen şekil {}",
                other.shapetype()
            ))),
        })
        .collect::<Result<_>>()?;

    let mut vertices: Vec<[f32; 2]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let (mut parts, mut crossings) = (0usize, 0usize);
    for shape in &shapes {
        for part in shape.parts() {
            if part.len() < 2 {
                continue;
            }
            crossings += antimeridian_crossings(part);
            if !indices.is_empty() {
                indices.push(RESTART);
            }
            let base = vertices.len() as u32;
            vertices.extend(part.iter().map(|p| {
                let [x, y] = project(p);
                [x as f32, y as f32]
            }));
            indices.extend(base..vertices.len() as u32);
            parts += 1;
        }
    }
    report(
        name,
        path,
        shapes.len(),
        parts,
        vertices.len(),
        0,
        crossings,
    );
    Ok(Layer {
        name: name.to_owned(),
        kind: LayerKind::LineStrips,
        vertices,
        indices,
    })
}

fn report(
    name: &str,
    path: &Path,
    shapes: usize,
    parts: usize,
    verts: usize,
    tris: usize,
    crossings: usize,
) {
    let file = path.file_name().unwrap_or_default().to_string_lossy();
    print!("{file:48} {name:7} şekil {shapes:5}  parça {parts:6}  köşe {verts:8}");
    if tris > 0 {
        print!("  üçgen {tris:8}");
    }
    if crossings > 0 {
        print!("  UYARI: 180°'yi geçen {crossings} kenar");
    }
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pt(x: f64, y: f64) -> Point {
        Point { x, y }
    }

    /// Bölüm 14 regresyonu: 1999 İzmit (Mw 7.6), USGS ShakeMap istasyonları
    /// (depoda: data/validation/izmit1999). Kayıtlı sonuç (docs/dogrulama.md,
    /// 2026-11-14): A ort. ΔI −0,24, %87; B −0,39, %84 (31 istasyon).
    /// Yer hareketi, şiddet dönüşümü ya da kaynak geometrisi değişirse bu
    /// test düşer: değişiklik bilinçliyse yeni sayılar belgeye yazılıp
    /// buradaki beklenen değerler güncellenir.
    /// 2017 Puebla (Mw 7,1, levha içi): Parker vd. (2020) + Slab2 bölgesi
    /// (Orta Amerika kuzey). docs/dogrulama.md'deki A ve B satırları.
    #[test]
    fn puebla_2017_dogrulamasi_kayitli_degerlerde() {
        use seismira_core::quake::{FaultType, Quake, Setting};
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data");
        let stations = Path::new(root).join("validation/puebla2017/stationlist.json");
        let slab =
            seismira_core::slab::Slab2::load(&Path::new(root).join("slab2/slab2.sslb")).unwrap();
        // USGS: merkez üssü, derinlik, Mww düğüm düzlemi 1.
        let mut q = Quake::new((-98.4887, 18.5499));
        q.depth_km = 48.0;
        q.mw = 7.1;
        q.strike = 299.0;
        q.dip = 44.0;
        q.fault = FaultType::Normal;
        q.setting = Setting::Intraslab;
        let c = compare_shakemap(&stations, &q, None, Some(&slab)).unwrap();
        assert_eq!(c.region, seismira_core::gmpe::psbah20::Region::CamN);
        assert_eq!(c.slab_zone.as_deref(), Some("cam"));
        assert_eq!((c.rows.len(), c.single, c.missing), (29, 29, 0));
        let (a, b) = (
            summarize(&c.rows, 0).unwrap(),
            summarize(&c.rows, 1).unwrap(),
        );
        let close = |x: f64, want: f64| (x - want).abs() < 0.01;
        assert!(close(a.mean_di, 0.20) && close(a.mean_ln, 0.25), "{a:?}");
        assert!(
            close(a.sd_ln, 0.59) && close(a.within_one, 28.0 / 29.0),
            "{a:?}"
        );
        assert!(close(b.mean_di, 0.12) && close(b.mean_ln, 0.15), "{b:?}");
        assert!(
            close(b.sd_ln, 0.59) && close(b.within_one, 28.0 / 29.0),
            "{b:?}"
        );
    }

    #[test]
    fn izmit_1999_dogrulamasi_kayitli_degerlerde() {
        use seismira_core::quake::{FaultType, Quake};
        let stations = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/validation/izmit1999/stationlist.json"
        ));
        // Merkez üssü ve derinlik USGS; doğrultu / eğim USGS moment tensörü.
        let mut q = Quake::new((29.864, 40.748));
        q.depth_km = 17.0;
        q.mw = 7.6;
        q.strike = 85.0;
        q.dip = 81.0;
        q.fault = FaultType::StrikeSlip;
        let c = compare_shakemap(stations, &q, None, None).unwrap();
        assert_eq!(c.rows.len(), 31);
        assert_eq!(c.missing, 0);
        let (a, b) = (
            summarize(&c.rows, 0).unwrap(),
            summarize(&c.rows, 1).unwrap(),
        );
        assert!(summarize(&c.rows, 2).is_none()); // Vs30 haritası verilmedi
        let close = |x: f64, want: f64| (x - want).abs() < 0.01;
        assert!(close(a.mean_di, -0.24) && close(a.mean_ln, -0.31), "{a:?}");
        assert!(
            close(a.sd_ln, 0.66) && close(a.within_one, 27.0 / 31.0),
            "{a:?}"
        );
        // B: fay uzunluğu TMG17 ile sınırlı (2026-11-24): 198 → 171 km;
        // önce −0.39 / −0.49 / 0.66.
        assert!(close(b.mean_di, -0.36) && close(b.mean_ln, -0.45), "{b:?}");
        assert!(
            close(b.sd_ln, 0.69) && close(b.within_one, 26.0 / 31.0),
            "{b:?}"
        );
        // Başlangıç kabul ölçütü (Bölüm 14), A için: ortalama sapma < 0,3
        // birim ve noktaların en az %80'inde fark < 1 birim.
        assert!(a.mean_di.abs() < 0.3 && a.within_one >= 0.8);
    }

    #[test]
    fn nokta_halka_icinde() {
        let square = [pt(0.0, 0.0), pt(10.0, 0.0), pt(10.0, 10.0), pt(0.0, 10.0)];
        assert!(point_in_ring(&square, &pt(5.0, 5.0)));
        assert!(!point_in_ring(&square, &pt(15.0, 5.0)));
        assert!(!point_in_ring(&square, &pt(5.0, -1.0)));
        // İçbükey (U şekli): girintideki nokta dışarıda.
        let u = [
            pt(0.0, 0.0),
            pt(9.0, 0.0),
            pt(9.0, 9.0),
            pt(6.0, 9.0),
            pt(6.0, 3.0),
            pt(3.0, 3.0),
            pt(3.0, 9.0),
            pt(0.0, 9.0),
        ];
        assert!(!point_in_ring(&u, &pt(4.5, 6.0)));
        assert!(point_in_ring(&u, &pt(1.5, 6.0)));
    }
}
