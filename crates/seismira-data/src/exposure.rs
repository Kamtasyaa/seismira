//! `exposure` komutu: ESRM20 bina stoku + kırılganlık → `.sexp` (Faz 6).
//!
//! Girdi: `scripts/fetch_esrm20.py` ile indirilen sabitlenmiş ESRM20 klasörü
//! (`Exposure/OQ_Exposure_Input_<ülke>_{Res,Com,Ind}.csv`,
//! `Vulnerability/esrm20_exposure_vulnerability_mapping.csv`,
//! `Vulnerability/fragility_ESRM20_Various_IM.xml`) ve bölge kimliği rasterı
//! (`data/regions/admin1.sras`).
//!
//! - Her bina kaydı koordinatıyla (ESRM20 idari biriminin temsil noktası)
//!   Natural Earth Admin-1 bölgesine atanır. Nokta denizde kalırsa (kıyı ve
//!   ada birimleri) 0,5°'ye kadar en yakın bölge aranır, ama YALNIZCA o
//!   ülkenin bölgeleri arasında (ülke: dosyadaki doğrudan isabetlerin bina
//!   ağırlıklı çoğunluğu). Böylece ör. Meis'in (Yunanistan) binaları
//!   Antalya'ya taşınmaz.
//! - Bina tipi (GEM taksonomisi) eşleme tablosuyla ESRM20 kırılganlık
//!   tiplerine ağırlıklarıyla dağıtılır.
//! - Kırılganlık XML'i OpenQuake `logncdf` biçimindedir (doğrusal ortalama ve
//!   standart sapma); medyan ve β'ya çevrilir. Dört sınır durumunun β'sı
//!   aynı olmalı (ESRM20 xlsx'te tek β); en büyük fark 0,01'i aşarsa hata.

use anyhow::{Context, Result, bail};
use seismira_core::exposure::{Exposure, FragImt, Fragility, N_DS, RegionStock};
use seismira_core::region::RegionMap;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

/// Okunmakta olan eğri: ad, şiddet ölçüsü, hasarsızlık sınırı, (ortalama,
/// standart sapma) çiftleri.
type Partial = (String, Option<FragImt>, f32, Vec<(f64, f64)>);

fn attr<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let key = format!("{name}=\"");
    let start = line.find(&key)? + key.len();
    let end = line[start..].find('"')? + start;
    Some(&line[start..end])
}

/// OpenQuake NRML 0.5 `fragilityModel` (her öğe ayrı satırda; ESRM20
/// dosyasının biçimi). Sınır durumu sırası `<limitStates>` satırından.
pub fn read_fragility(xml: &str) -> Result<Vec<Fragility>> {
    let mut out = Vec::new();
    let mut cur: Option<Partial> = None;
    for line in xml.lines().map(str::trim) {
        if line.starts_with("<limitStates>") {
            let ls = line
                .trim_start_matches("<limitStates>")
                .trim_end_matches("</limitStates>");
            if ls.split_whitespace().collect::<Vec<_>>()
                != ["slight", "moderate", "extensive", "complete"]
            {
                bail!("beklenmeyen sınır durumları: {ls}");
            }
        } else if line.starts_with("<fragilityFunction") {
            if attr(line, "format") != Some("continuous") || attr(line, "shape") != Some("logncdf")
            {
                bail!("desteklenmeyen eğri: {line}");
            }
            let id = attr(line, "id").context("id")?.to_string();
            cur = Some((id, None, 0.0, Vec::new()));
        } else if line.starts_with("<imls") {
            let c = cur.as_mut().context("imls dışarıda")?;
            let imt = attr(line, "imt").context("imt")?;
            c.1 = Some(FragImt::parse(imt).with_context(|| format!("şiddet ölçüsü {imt}"))?);
            c.2 = attr(line, "noDamageLimit")
                .context("noDamageLimit")?
                .parse()?;
        } else if line.starts_with("<params") {
            let c = cur.as_mut().context("params dışarıda")?;
            c.3.push((
                attr(line, "mean").context("mean")?.parse()?,
                attr(line, "stddev").context("stddev")?.parse()?,
            ));
        } else if line.starts_with("</fragilityFunction") {
            let (name, imt, no_damage, params) = cur.take().context("kapanış")?;
            if params.len() != N_DS {
                bail!("{name}: {} sınır durumu", params.len());
            }
            let mb: Vec<(f64, f64)> = params
                .iter()
                .map(|&(m, s)| Fragility::from_mean_stddev(m, s))
                .collect();
            let betas: Vec<f64> = mb.iter().map(|x| x.1).collect();
            let (lo, hi) = betas
                .iter()
                .fold((f64::MAX, f64::MIN), |(a, b), &x| (a.min(x), b.max(x)));
            if hi - lo > 0.01 {
                bail!("{name}: β sınır durumlarında farklı ({betas:?})");
            }
            let beta = betas.iter().sum::<f64>() / N_DS as f64;
            out.push(Fragility {
                name,
                imt: imt.context("imls yok")?,
                median: [0, 1, 2, 3].map(|k| mb[k].0 as f32),
                beta: beta as f32,
                no_damage,
            });
        }
    }
    Ok(out)
}

/// Denizde kalan nokta için 0,5°'ye kadar halkalarla en yakın bölge;
/// `country(id)` doğru olan bölgeler kabul edilir.
fn region_near(
    map: &RegionMap<Vec<u8>>,
    lon: f64,
    lat: f64,
    country: impl Fn(u16) -> bool,
) -> Result<u16> {
    let step = 0.01;
    for ring in 1..=50 {
        let r = f64::from(ring) * step;
        let n = 8 * ring;
        for k in 0..n {
            let a = f64::from(k) / f64::from(n) * std::f64::consts::TAU;
            let id = map.id_at(
                lon + r * a.cos() / lat.to_radians().cos().max(0.2),
                lat + r * a.sin(),
            )?;
            if id != 0 && country(id) {
                return Ok(id);
            }
        }
    }
    Ok(0)
}

/// Bölge başına yerleşim hücresi sayısı (tüm raster, 512 satırlık şeritler
/// hâlinde; ~22 MB / şerit).
pub fn built_cells(map: &RegionMap<Vec<u8>>) -> Result<HashMap<u16, u32>> {
    let g = map.grid();
    let mut built: HashMap<u16, u32> = HashMap::new();
    let band = 512u32;
    for row0 in (0..g.height).step_by(band as usize) {
        let h = band.min(g.height - row0);
        for raw in map.window(0, i64::from(row0), g.width, h)? {
            let id = raw & seismira_core::region::ID_MASK;
            if id != 0 && raw & seismira_core::region::BUILT_BIT != 0 {
                *built.entry(id).or_default() += 1;
            }
        }
    }
    Ok(built)
}

pub fn run(esrm20: &Path, regions: &Path, output: &Path) -> Result<()> {
    let infos = seismira_core::region::read_infos(&mut std::io::BufReader::new(
        std::fs::File::open(regions.with_extension("sreg"))?,
    ))?;
    let country_of = |id: u16| infos[usize::from(id) - 1].country;
    let vul = esrm20.join("Vulnerability");
    let fragility = read_fragility(&std::fs::read_to_string(
        vul.join("fragility_ESRM20_Various_IM.xml"),
    )?)?;
    let index: HashMap<&str, u16> = fragility
        .iter()
        .enumerate()
        .map(|(i, f)| (f.name.as_str(), i as u16))
        .collect();
    let mut mapping: HashMap<String, Vec<(u16, f64)>> = HashMap::new();
    let mut rdr = csv::Reader::from_path(vul.join("esrm20_exposure_vulnerability_mapping.csv"))?;
    for rec in rdr.records() {
        let rec = rec?;
        let t = *index
            .get(&rec[1])
            .with_context(|| format!("eşlemede bilinmeyen tip {}", &rec[1]))?;
        mapping
            .entry(rec[0].to_string())
            .or_default()
            .push((t, rec[2].parse()?));
    }
    for (tax, list) in &mapping {
        let w: f64 = list.iter().map(|x| x.1).sum();
        if (w - 1.0).abs() > 1e-6 {
            bail!("{tax}: ağırlık toplamı {w}");
        }
    }
    let map = RegionMap::open(std::fs::read(regions)?)?;
    let mut stock: BTreeMap<u16, HashMap<u16, f64>> = BTreeMap::new();
    let (mut total, mut lost, mut unmapped, mut moved) = (0.0f64, 0.0f64, 0.0f64, 0usize);
    let mut files: Vec<_> = std::fs::read_dir(esrm20.join("Exposure"))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "csv"))
        .collect();
    files.sort();
    // Ülke → dosyalar ("OQ_Exposure_Input_<ülke>_<Res|Com|Ind>.csv").
    let mut by_country: BTreeMap<String, Vec<std::path::PathBuf>> = BTreeMap::new();
    for p in files {
        let stem = p.file_stem().unwrap().to_string_lossy().to_string();
        let c = stem
            .trim_start_matches("OQ_Exposure_Input_")
            .rsplit_once('_')
            .context("dosya adı")?
            .0
            .to_string();
        by_country.entry(c).or_default().push(p);
    }
    for (country, paths) in &by_country {
        // (boylam, enlem, taksonomi, bina) kayıtları.
        let mut rows: Vec<(f64, f64, String, f64)> = Vec::new();
        for path in paths {
            let mut rdr = csv::Reader::from_path(path)?;
            let h = rdr.headers()?.clone();
            let col = |n: &str| {
                h.iter()
                    .position(|x| x == n)
                    .with_context(|| format!("{}: {n} sütunu", path.display()))
            };
            let (c_lon, c_lat, c_tax, c_n) =
                (col("lon")?, col("lat")?, col("taxonomy")?, col("number")?);
            for rec in rdr.records() {
                let rec = rec?;
                rows.push((
                    rec[c_lon].parse()?,
                    rec[c_lat].parse()?,
                    rec[c_tax].to_string(),
                    rec[c_n].parse()?,
                ));
            }
        }
        // Doğrudan isabetler ve ülkenin (bina ağırlıklı) çoğunluk kodu.
        let mut ids: HashMap<(u64, u64), u16> = HashMap::new();
        let mut votes: HashMap<[u8; 2], f64> = HashMap::new();
        for (lon, lat, _, n) in &rows {
            let key = (lon.to_bits(), lat.to_bits());
            let id = match ids.get(&key) {
                Some(&id) => id,
                None => {
                    let id = map.id_at(*lon, *lat)?;
                    ids.insert(key, id);
                    id
                }
            };
            if id != 0 {
                *votes.entry(country_of(id)).or_default() += n;
            }
        }
        // Hiçbir nokta karaya düşmediyse (ör. Monaco: 30″ ızgarada hücresi
        // yok) ülke kısıtı olmadan en yakın bölge.
        let code = votes
            .iter()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .map(|(c, _)| *c);
        if code.is_none() {
            println!("  {country}: hiçbir nokta karada değil, ülke kısıtsız en yakın bölge");
        }
        let mut country_total = 0.0;
        for (lon, lat, tax, n) in &rows {
            total += n;
            country_total += n;
            let key = (lon.to_bits(), lat.to_bits());
            let mut id = ids[&key];
            if id == 0 {
                id = region_near(&map, *lon, *lat, |r| {
                    code.is_none_or(|c| country_of(r) == c)
                })?;
                ids.insert(key, id);
                moved += 1;
            }
            if id == 0 {
                lost += n;
                continue;
            }
            let Some(list) = mapping.get(tax) else {
                unmapped += n;
                continue;
            };
            let e = stock.entry(id).or_default();
            for &(t, w) in list {
                *e.entry(t).or_default() += n * w;
            }
        }
        let code = code.map_or("?".into(), |c| String::from_utf8_lossy(&c).to_string());
        println!("{country} ({code}): {country_total:.0} bina");
    }
    let built = built_cells(&map)?;
    let no_built = stock.keys().filter(|id| !built.contains_key(id)).count();
    if no_built > 0 {
        println!(
            "  {no_built} bölgede yerleşim hücresi yok (stok yine yazılır, hasar hesaplanmaz)"
        );
    }
    let exposure = Exposure {
        fragility,
        regions: stock
            .into_iter()
            .map(|(id, m)| {
                let mut entries: Vec<(u16, f32)> =
                    m.into_iter().map(|(t, n)| (t, n as f32)).collect();
                entries.sort_by_key(|e| e.0);
                RegionStock {
                    id,
                    built_cells: built.get(&id).copied().unwrap_or(0),
                    entries,
                }
            })
            .collect(),
    };
    if let Some(dir) = output.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut w = std::io::BufWriter::new(std::fs::File::create(output)?);
    exposure.write(&mut w)?;
    drop(w);
    println!(
        "\n→ {} ({} bölge, {} tip, {:.2} MB)\ntoplam {:.0} bina; bölgesiz {:.0} (%{:.3}); \
         eşlenmeyen tip {:.0}; denizden en yakın bölgeye taşınan nokta kaydı {moved}",
        output.display(),
        exposure.regions.len(),
        exposure.fragility.len(),
        std::fs::metadata(output)?.len() as f64 / 1e6,
        total,
        lost,
        lost / total * 100.0,
        unmapped
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kirilganlik_xml_okunur() {
        let xml = r#"<limitStates>slight moderate extensive complete</limitStates>
<fragilityFunction format="continuous" id="CR_LDUAL-DUH_H1" shape="logncdf">
<imls imt="PGA" maxIML="5" minIML="1e-10" noDamageLimit="0.05"/>
<params ls="slight" mean="1.481" stddev="0.55"/>
<params ls="moderate" mean="2.36" stddev="0.877"/>
<params ls="extensive" mean="2.917" stddev="1.083"/>
<params ls="complete" mean="3.331" stddev="1.237"/>
</fragilityFunction>"#;
        let f = read_fragility(xml).unwrap();
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].imt, FragImt::Pga);
        // ESRM20 xlsx "Final": medyanlar 1.388, 2.213, 2.735, 3.123; β 0.3594.
        let want = [1.388_092_7, 2.212_655_1, 2.734_727_1, 3.122_533_1];
        for (m, w) in f[0].median.iter().zip(want) {
            assert!((f64::from(*m) - w).abs() < 3e-3, "{:?}", f[0]);
        }
        assert!((f64::from(f[0].beta) - 0.359_425).abs() < 2e-3);
        assert!((f[0].no_damage - 0.05).abs() < 1e-9);
    }
}
