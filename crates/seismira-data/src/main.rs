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
//! ```

use anyhow::{Context, Result, bail};
use seismira_core::mapfile::{Layer, LayerKind, MapFile, RESTART};
use seismira_core::mercator;
use shapefile::{Point, Polygon, PolygonRing, Polyline, Shape, ShapeReader};
use std::path::Path;

mod city_sites;
mod geotiff;
mod regions;
mod vs30;

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
             seismira-data regions <admin1.shp> <vs30.sras> <ghsl.tif> <çıktı.sras> <çıktı.sreg>"
        ),
    }
}

/// Bir istasyonun karşılaştırma satırı: kod, gözlenen, A, B (ln PGV),
/// ShakeMap Rrup (km), C (ln PGV; Vs30 haritası verildiyse).
type Row = (String, f64, f64, f64, f64, Option<f64>);

/// [`compare_shakemap`] sonucu.
struct Compared {
    /// Bizim fay geometrimiz.
    fault: seismira_core::source::Fault,
    rows: Vec<Row>,
    /// ShakeMap'in mesafe vermediği (atlanan) istasyon sayısı.
    missing: usize,
    /// ln(harita Vs30 / istasyon Vs30) ve harita kaynağı sayıları.
    vs_ratio: Vec<f64>,
    vs_src: [usize; 4],
}

/// Bir karşılaştırmanın özeti: ortalama ve standart sapma ln(gözlenen /
/// tahmin), ortalama ΔI (JMA aletsel şiddeti), |ΔI| < 1 olanların oranı.
#[derive(Clone, Copy, Debug)]
struct Summary {
    mean_ln: f64,
    sd_ln: f64,
    mean_di: f64,
    within_one: f64,
}

/// Satırlardaki `k` karşılaştırmasının (0: A, 1: B, 2: C) özeti.
fn summarize(rows: &[Row], k: usize) -> Option<Summary> {
    use seismira_core::intensity::jma_from_pgv_rotd50;
    let to_i = |ln_pgv: f64| jma_from_pgv_rotd50(ln_pgv.exp());
    let pick = |r: &Row| match k {
        0 => Some(r.2),
        1 => Some(r.3),
        _ => r.5,
    };
    let res: Vec<f64> = rows.iter().filter_map(|r| Some(r.1 - pick(r)?)).collect();
    let di: Vec<f64> = rows
        .iter()
        .filter_map(|r| Some(to_i(r.1) - to_i(pick(r)?)))
        .collect();
    if res.len() < 2 {
        return None;
    }
    let n = res.len() as f64;
    let mean = res.iter().sum::<f64>() / n;
    Some(Summary {
        mean_ln: mean,
        sd_ln: (res.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0)).sqrt(),
        mean_di: di.iter().sum::<f64>() / n,
        within_one: di.iter().filter(|x| x.abs() < 1.0).count() as f64 / n,
    })
}

/// ShakeMap istasyon ölçümleriyle karşılaştırma (CLAUDE.md Bölüm 14).
///
/// Gözlenen RotD50 yerine iki yatay bileşenin geometrik ortalaması
/// kullanılır (Boore 2010: ikisi pratikte çok yakın). Karşılaştırmalar:
/// (A) ShakeMap'in mesafeleriyle (yalnızca GMPE ve dönüşüm), (B) bizim fay
/// geometrimizle, (C) Vs30 haritası verilirse bizim geometri + bizim Vs30
/// haritamız (uygulamanın tüm zinciri). A ve B istasyonun ShakeMap
/// Vs30'unu kullanır.
fn compare_shakemap(
    stations: &Path,
    q: &seismira_core::quake::Quake,
    vs30_map: Option<&seismira_core::site::Vs30Map<Vec<u8>>>,
) -> Result<Compared> {
    use seismira_core::gmpe::{Imt, cy14};
    use seismira_core::source::Fault;

    let ours = Fault::from_quake(q);
    let fault = q.fault;
    let json: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(stations)?)?;
    let feats = json["features"].as_array().context("features yok")?;
    let mut rows: Vec<Row> = Vec::new();
    let mut vs_ratio: Vec<f64> = Vec::new();
    let mut vs_src = [0usize; 4];
    let mut missing = 0usize;
    for f in feats {
        let p = &f["properties"];
        if p["station_type"] != "seismic" {
            continue;
        }
        let horiz: Vec<f64> = p["channels"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|c| matches!(c["name"].as_str(), Some(n) if !n.ends_with('Z')))
            .filter_map(|c| {
                c["amplitudes"]
                    .as_array()?
                    .iter()
                    .find(|a| a["name"] == "pgv")?["value"]
                    .as_f64()
            })
            .collect();
        if horiz.len() != 2 {
            continue;
        }
        let obs = (horiz[0].ln() + horiz[1].ln()) / 2.0;
        let vs30 = p["vs30"].as_f64().context("vs30")?;
        let d = &p["distances"];
        let coords = f["geometry"]["coordinates"].as_array().context("konum")?;
        let site = (
            coords[0].as_f64().context("boylam")?,
            coords[1].as_f64().context("enlem")?,
        );
        let predict = |rrup: f64, rjb: f64, rx: f64, ztor: f64, vs30: f64| {
            cy14::predict(
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
            .map(|m| m.mean)
        };
        let (Some(rrup), Some(rjb)) = (d["rrup"].as_f64(), d["rjb"].as_f64()) else {
            missing += 1; // ShakeMap bazı istasyonlarda mesafe vermiyor
            continue;
        };
        // Yeni ShakeMap sürümleri Rx vermeyebilir: taban bloğu kabul edilir.
        // (Tavan bloğu terimi cos δ ile çarpılır; dik faylarda etkisi ~0.)
        let rx = d["rx"].as_f64().unwrap_or(-rjb);
        // ShakeMap kırılması yüzeyden başlıyor (rupture.json): Ztor = 0.
        let a = predict(rrup, rjb, rx, 0.0, vs30);
        let od = ours.distances(site);
        let b = predict(od.rrup, od.rjb, od.rx, ours.ztor, vs30);
        let c = match vs30_map {
            Some(m) => {
                let s = m.site(site.0, site.1)?;
                vs_src[s.source as usize] += 1;
                vs_ratio.push((s.vs30 / vs30).ln());
                let v = s.vs30.clamp(cy14::VS30_MIN, cy14::VS30_MAX);
                predict(od.rrup, od.rjb, od.rx, ours.ztor, v)
            }
            None => None,
        };
        if let (Some(a), Some(b)) = (a, b) {
            rows.push((
                p["code"].as_str().unwrap_or("?").to_string(),
                obs,
                a,
                b,
                rrup,
                c,
            ));
        }
    }
    Ok(Compared {
        fault: ours,
        rows,
        missing,
        vs_ratio,
        vs_src,
    })
}

/// `validate-shakemap` komutu: [`compare_shakemap`] sonucunu tablo olarak
/// yazar. Argümanlar: `<stationlist.json> boylam enlem derinlik Mw doğrultu
/// eğim fay(SS|RS|NS) [hiposantr_doğrultu_konumu] [--vs30 <dosya.sras>]`.
fn validate_shakemap(stations: &Path, args: &[String]) -> Result<()> {
    use seismira_core::intensity::jma_from_pgv_rotd50;
    use seismira_core::quake::{FaultType, Quake};

    let mut args = args.to_vec();
    let vs30_map = match args.iter().position(|a| a == "--vs30") {
        Some(i) => {
            let path = args.get(i + 1).context("--vs30 dosya yolu")?.clone();
            args.drain(i..i + 2);
            let bytes = std::fs::read(&path).with_context(|| path.clone())?;
            Some(seismira_core::site::Vs30Map::open(bytes)?)
        }
        None => None,
    };
    let args = args.as_slice();
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
    if let Some(a) = along {
        q.hypo_along = a;
    }
    let Compared {
        fault: ours,
        rows,
        missing,
        vs_ratio,
        vs_src,
    } = compare_shakemap(stations, &q, vs30_map.as_ref())?;

    let to_i = |ln_pgv: f64| jma_from_pgv_rotd50(ln_pgv.exp());
    println!(
        "fay (bizim): L = {:.0} km, W = {:.0} km, Ztor = {:.1} km, hiposantr doğrultu konumu {:.2}",
        ours.length, ours.width, ours.ztor, q.hypo_along
    );
    println!(
        "{} istasyon (model aralığında); mesafesi eksik {} istasyon atlandı\n",
        rows.len(),
        missing
    );
    println!("| karşılaştırma | ort. ln(göz/tah) | std ln | ort. ΔI (JMA) | |ΔI| < 1 oranı |");
    println!("|---|---|---|---|---|");
    let mut comps = vec![
        ("A: ShakeMap mesafeleri", 0),
        ("B: bizim fay geometrimiz", 1),
    ];
    if vs30_map.is_some() {
        comps.push(("C: bizim geometri + bizim Vs30", 2));
    }
    for (label, k) in comps {
        let Some(s) = summarize(&rows, k) else {
            println!("| {label} | – | – | – | – |");
            continue;
        };
        println!(
            "| {label} | {:+.2} | {:.2} | {:+.2} | %{:.0} |",
            s.mean_ln,
            s.sd_ln,
            s.mean_di,
            s.within_one * 100.0
        );
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
    ] {
        let sel: Vec<_> = rows.iter().filter(|r| r.4 >= lo && r.4 < hi).collect();
        if sel.is_empty() {
            continue;
        }
        let n = sel.len() as f64;
        let a = sel.iter().map(|r| to_i(r.1) - to_i(r.2)).sum::<f64>() / n;
        let b = sel.iter().map(|r| to_i(r.1) - to_i(r.3)).sum::<f64>() / n;
        let c: Vec<f64> = sel
            .iter()
            .filter_map(|r| Some(to_i(r.1) - to_i(r.5?)))
            .collect();
        let c = if c.is_empty() {
            "–".to_string()
        } else {
            format!("{:+.2}", c.iter().sum::<f64>() / c.len() as f64)
        };
        println!(
            "| {lo:.0}–{hi:.0} | {} | {a:+.2} | {b:+.2} | {c} |",
            sel.len()
        );
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
        let c = compare_shakemap(stations, &q, None).unwrap();
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
        assert!(close(b.mean_di, -0.39) && close(b.mean_ln, -0.49), "{b:?}");
        assert!(
            close(b.sd_ln, 0.66) && close(b.within_one, 26.0 / 31.0),
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
