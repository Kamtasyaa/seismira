//! `gem` komutu: GEM küresel bina stoku özetleri + yapısal zafiyet
//! fonksiyonları → `.svul` (Faz 9; docs/bilim.md 7.8, docs/veri.md).
//!
//! Girdi: `scripts/fetch_gem.py` ile indirilen sabitlenmiş klasör
//! (`exposure/World/summaries/Exposure_Summary_Adm1_Taxonomy.csv`,
//! `exposure/World/summaries/Vulnerability_mapping_country.csv`,
//! `vulnerability/<bölge>/<ülke>/vulnerability_structural.xml`) ve bölge
//! kimliği rasterı (`data/regions/admin1.sras` + `.sreg`).
//!
//! - Stok satırları (Adm1 × bina sınıfı × yerleşim türü) toplanır; yerleşim
//!   türleri (TOTAL/URBAN/RURAL...) çakışmaz (ülke Adm1 özetleriyle aynı
//!   toplamı verdiği Çin ve ABD'de denetlendi, 2026-11-23).
//! - Ağırlık: `BLDG_REPL_COST_USD` (yapısal + yapısal olmayan yeniden yapım
//!   değeri). Yapısal pay (`COST_STRUCTURAL_USD`) yalnızca ABD ve
//!   topraklarında dolu; tutarlılık için her yerde toplam değer kullanılır.
//!   Çıktı bir ORAN olduğundan yalnızca sınıflar arası göreli ağırlık önemli
//!   (VARSAYIM: yapısal pay sınıftan sınıfa çok değişmez; ABD'de 0,10–0,61).
//! - Bölge eşlemesi sırasıyla: (1) ISO 3166-2 kodu (Natural Earth aynı kodu
//!   birden çok bölgeye verdiyse önce ada göre, sonra yerleşim hücresi
//!   oranında), (2) kod değişiklikleri tablosu [`ALIASES`], (3) aynı ülkede
//!   ad (büyük/küçük harf, aksan ve noktalama yok sayılır), (4) kalanlar
//!   ülkenin hiç stok almamış bölgelerine (hepsi almışsa tüm bölgelerine)
//!   yerleşim hücresi oranında dağıtılır.
//! - Sınıf → zafiyet fonksiyonu: ülke eşleme tablosu (aynı satır birden çok
//!   kez yazılmışsa ağırlıklar toplanıp 1'e ölçeklenir). ABD, Kanada ve
//!   ABD topraklarında eşleme yok; `TAXONOMY_HAZUS` doğrudan fonksiyon
//!   adıdır. Hazus RES3A–F alt sınıflarının fonksiyonu yoksa RES3 kullanılır.
//! - Aynı içerikli fonksiyonlar (ülkeler arası) tek kayda indirilir.

use anyhow::{Context, Result, bail};
use seismira_core::exposure::FragImt;
use seismira_core::region::{RegionInfo, RegionMap};
use seismira_core::vulnerability::{VulnFunction, VulnModel, VulnRegion};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

/// GEM Adm1 kodu → Natural Earth kodu (+ aynı kodlu bölgeler arasında ad).
/// Natural Earth'ün eski ya da farklı ISO 3166-2 kodları; .sreg'den bakıldı.
const ALIASES: &[(&str, &str, Option<&str>)] = &[
    ("MX-CMX", "MX-DIF", None),
    ("CO-DC", "CO-CUN", Some("Bogotá")),
    ("IN-UK", "IN-UT", None),
    ("IN-OD", "IN-OR", None),
    ("IN-CG", "IN-CT", None),
    ("IN-TS", "IN-TG", None),
    ("ZA-KZN", "ZA-NL", None),
    ("ZA-GP", "ZA-GT", None),
    ("TW-NWT", "TW-TPQ", None),
];

/// Ülke adı (GEM `NAME_0`) → zafiyet klasörü adı, adlar farklıysa.
const FOLDER_ALIASES: &[(&str, &str)] = &[
    ("CPV", "Cape_Verde"),
    ("CIV", "Ivory_Coast"),
    ("FSM", "Micronesia"),
    ("TUR", "Turkey"),
];

/// Karşılaştırma için ad: küçük harf, aksansız, yalnızca harf ve rakam.
fn norm(s: &str) -> String {
    s.nfkd()
        .filter(|c| !is_combining_mark(*c) && c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn between<'a>(s: &'a str, open: &str, close: &str) -> Option<(&'a str, &'a str)> {
    let a = s.find(open)? + open.len();
    let b = s[a..].find(close)? + a;
    Some((&s[a..b], &s[b + close.len()..]))
}

/// NRML 0.5 `vulnerabilityModel` (yalnızca kimlik, IMT, IML, ortalama LR).
pub fn read_vulnerability(xml: &str) -> Result<Vec<VulnFunction>> {
    let mut out = Vec::new();
    let mut rest = xml;
    while let Some(i) = rest.find("<vulnerabilityFunction ") {
        rest = &rest[i..];
        let end = rest
            .find("</vulnerabilityFunction>")
            .context("kapanmamış vulnerabilityFunction")?;
        let block = &rest[..end];
        rest = &rest[end..];
        let (id, _) = between(block, "id=\"", "\"").context("id")?;
        let (head, after) = between(block, "<imls", ">").context("imls")?;
        let (imt, _) = between(head, "imt=\"", "\"").context("imt")?;
        let imt = FragImt::parse(imt).with_context(|| format!("{id}: bilinmeyen IMT {imt}"))?;
        let (iml, _) = between(after, "", "</imls>").context("imls sonu")?;
        let (lr, _) = between(block, "<meanLRs>", "</meanLRs>").context("meanLRs")?;
        let nums = |s: &str| -> Result<Vec<f32>> {
            s.split_whitespace()
                .map(|x| x.parse::<f32>().map_err(Into::into))
                .collect()
        };
        let (iml, mean_lr) = (nums(iml)?, nums(lr)?);
        if iml.len() != mean_lr.len() || iml.len() < 2 || !iml.windows(2).all(|w| w[0] < w[1]) {
            bail!("{id}: geçersiz eğri");
        }
        out.push(VulnFunction {
            name: id.to_string(),
            imt,
            iml,
            mean_lr,
        });
    }
    Ok(out)
}

/// Fonksiyon içeriği anahtarı (tekilleştirme için).
fn content_key(f: &VulnFunction) -> (u8, Vec<u32>) {
    let bits = f
        .iml
        .iter()
        .chain(&f.mean_lr)
        .map(|v| v.to_bits())
        .collect();
    (f.imt as u8, bits)
}

struct Regions<'a> {
    infos: &'a [RegionInfo],
    built: &'a HashMap<u16, u32>,
    by_iso: HashMap<&'a str, Vec<u16>>,
    by_country: HashMap<[u8; 2], Vec<u16>>,
}

impl<'a> Regions<'a> {
    fn new(infos: &'a [RegionInfo], built: &'a HashMap<u16, u32>) -> Self {
        let mut by_iso: HashMap<&str, Vec<u16>> = HashMap::new();
        let mut by_country: HashMap<[u8; 2], Vec<u16>> = HashMap::new();
        for (i, r) in infos.iter().enumerate() {
            let id = (i + 1) as u16;
            by_iso.entry(r.iso.as_str()).or_default().push(id);
            by_country.entry(r.country).or_default().push(id);
        }
        Self {
            infos,
            built,
            by_iso,
            by_country,
        }
    }

    fn info(&self, id: u16) -> &RegionInfo {
        &self.infos[usize::from(id) - 1]
    }

    fn built(&self, id: u16) -> f64 {
        f64::from(self.built.get(&id).copied().unwrap_or(0))
    }

    /// Bölgeler arasında yerleşim hücresi oranında paylar (hiç yerleşim yoksa
    /// eşit).
    fn shares(&self, ids: &[u16]) -> Vec<(u16, f64)> {
        let total: f64 = ids.iter().map(|&i| self.built(i)).sum();
        ids.iter()
            .map(|&i| {
                let w = if total > 0.0 {
                    self.built(i) / total
                } else {
                    1.0 / ids.len() as f64
                };
                (i, w)
            })
            .collect()
    }

    /// Aday kümesini ada göre daraltır (eşleşen yoksa küme aynen kalır).
    fn by_name(&self, ids: &[u16], name: &str) -> Vec<u16> {
        let n = norm(name);
        let hit: Vec<u16> = ids
            .iter()
            .copied()
            .filter(|&i| norm(&self.info(i).name_en) == n)
            .collect();
        if hit.is_empty() { ids.to_vec() } else { hit }
    }

    /// GEM Adm1 birimi → bölge payları; `None`: eşlenemedi (havuza).
    fn resolve(&self, id1: &str, name1: &str) -> Option<Vec<(u16, f64)>> {
        if let Some(ids) = self.by_iso.get(id1) {
            return Some(self.shares(&self.by_name(ids, name1)));
        }
        if let Some(&(_, iso, name)) = ALIASES.iter().find(|a| a.0 == id1)
            && let Some(ids) = self.by_iso.get(iso)
        {
            let ids = name.map_or(ids.clone(), |n| self.by_name(ids, n));
            return Some(self.shares(&ids));
        }
        let cc = country_code(id1)?;
        let n = norm(name1);
        let hit: Vec<u16> = self
            .by_country
            .get(&cc)?
            .iter()
            .copied()
            .filter(|&i| norm(&self.info(i).name_en) == n)
            .collect();
        (!hit.is_empty()).then(|| self.shares(&hit))
    }
}

/// "JP-23" → b"JP".
fn country_code(id1: &str) -> Option<[u8; 2]> {
    let p = id1.split('-').next()?.as_bytes();
    (p.len() == 2).then(|| [p[0], p[1]])
}

/// Bölge stoku birikimi: bölge → (fonksiyon → (değer, bina)).
type Stock = BTreeMap<u16, HashMap<u16, (f64, f64)>>;

fn add(stock: &mut Stock, shares: &[(u16, f64)], funcs: &[(u16, f64)], cost: f64, n: f64) {
    for &(r, s) in shares {
        let e = stock.entry(r).or_default();
        for &(f, w) in funcs {
            let x = e.entry(f).or_default();
            x.0 += cost * s * w;
            x.1 += n * s * w;
        }
    }
}

pub fn run(gem: &Path, regions: &Path, output: &Path) -> Result<()> {
    let infos = seismira_core::region::read_infos(&mut std::io::BufReader::new(
        std::fs::File::open(regions.with_extension("sreg"))?,
    ))?;
    let map = RegionMap::open(std::fs::read(regions)?)?;
    let built = crate::exposure::built_cells(&map)?;
    let reg = Regions::new(&infos, &built);
    let summaries = gem.join("exposure/World/summaries");

    // Zafiyet klasörleri: normalize ad → yol.
    let mut folders: HashMap<String, PathBuf> = HashMap::new();
    for region in std::fs::read_dir(gem.join("vulnerability"))? {
        let region = region?.path();
        if !region.is_dir() {
            continue;
        }
        for c in std::fs::read_dir(&region)? {
            let c = c?.path();
            if c.join("vulnerability_structural.xml").exists() {
                folders.insert(norm(&c.file_name().unwrap().to_string_lossy()), c);
            }
        }
    }

    // Ülke sınıf eşlemesi: (ID_0, sınıf) → [(fonksiyon adı, ağırlık)].
    let mut mapping: HashMap<(String, String), Vec<(String, f64)>> = HashMap::new();
    let mut rdr = csv::Reader::from_path(summaries.join("Vulnerability_mapping_country.csv"))?;
    let h = rdr.headers()?.clone();
    let col = |n: &str| h.iter().position(|x| x == n).context(n.to_string());
    let (c0, ct, cv, cw) = (
        col("ID_0")?,
        col("TAXONOMY")?,
        col("VUL_MAPPING")?,
        col("WEIGHT")?,
    );
    for rec in rdr.records() {
        let rec = rec?;
        let list = mapping
            .entry((rec[c0].to_string(), rec[ct].to_string()))
            .or_default();
        let w: f64 = rec[cw].parse()?;
        match list.iter_mut().find(|x| x.0 == rec[cv]) {
            Some(x) => x.1 += w,
            None => list.push((rec[cv].to_string(), w)),
        }
    }
    let mut renormalized = 0usize;
    for list in mapping.values_mut() {
        let s: f64 = list.iter().map(|x| x.1).sum();
        if (s - 1.0).abs() > 1e-6 {
            renormalized += 1;
            list.iter_mut().for_each(|x| x.1 /= s);
        }
    }

    let mut functions: Vec<VulnFunction> = Vec::new();
    let mut by_content: HashMap<(u8, Vec<u32>), u16> = HashMap::new();
    // ID_0 → (fonksiyon adı → genel sıra); ilk karşılaşmada okunur.
    let mut country_funcs: HashMap<String, HashMap<String, u16>> = HashMap::new();

    let mut stock: Stock = BTreeMap::new();
    // Eşlenemeyen birimler: ülke kodu → (fonksiyon → (değer, bina)).
    let mut pools: BTreeMap<[u8; 2], HashMap<u16, (f64, f64)>> = BTreeMap::new();
    let mut pooled_units: BTreeMap<[u8; 2], std::collections::BTreeSet<String>> = BTreeMap::new();
    let (mut total, mut unmapped, mut lost) = (0.0f64, 0.0f64, 0.0f64);
    let mut unmapped_by: BTreeMap<String, f64> = BTreeMap::new();

    let mut rdr = csv::Reader::from_path(summaries.join("Exposure_Summary_Adm1_Taxonomy.csv"))?;
    let h = rdr.headers()?.clone();
    let col = |n: &str| h.iter().position(|x| x == n).context(n.to_string());
    let (c0, cn0, c1, cn1, ct, chz, cb, cc) = (
        col("ID_0")?,
        col("NAME_0")?,
        col("ID_1")?,
        col("NAME_1")?,
        col("TAXONOMY")?,
        col("TAXONOMY_HAZUS")?,
        col("BUILDINGS")?,
        col("BLDG_REPL_COST_USD")?,
    );
    for rec in rdr.records() {
        let rec = rec?;
        let (id0, id1) = (&rec[c0], &rec[c1]);
        let cost: f64 = rec[cc].parse()?;
        let n: f64 = rec[cb].parse()?;
        if cost <= 0.0 && n <= 0.0 {
            continue;
        }
        total += cost;
        if !country_funcs.contains_key(id0) {
            let key = FOLDER_ALIASES
                .iter()
                .find(|a| a.0 == id0)
                .map_or_else(|| norm(&rec[cn0]), |a| norm(a.1));
            let mut names = HashMap::new();
            if let Some(dir) = folders.get(&key) {
                let xml = std::fs::read_to_string(dir.join("vulnerability_structural.xml"))?;
                for f in read_vulnerability(&xml).with_context(|| dir.display().to_string())? {
                    let idx = match by_content.get(&content_key(&f)) {
                        Some(&i) => i,
                        None => {
                            let i = u16::try_from(functions.len())
                                .context("65535'ten fazla fonksiyon")?;
                            by_content.insert(content_key(&f), i);
                            functions.push(f.clone());
                            i
                        }
                    };
                    names.insert(f.name, idx);
                }
            } else {
                println!("  {id0} ({}): zafiyet klasörü yok", &rec[cn0]);
            }
            country_funcs.insert(id0.to_string(), names);
        }
        let names = &country_funcs[id0];
        let funcs: Option<Vec<(u16, f64)>> =
            match mapping.get(&(id0.to_string(), rec[ct].to_string())) {
                Some(list) => list
                    .iter()
                    .map(|(v, w)| names.get(v).map(|&i| (i, *w)))
                    .collect(),
                None if !rec[chz].is_empty() => {
                    let hz = &rec[chz];
                    names
                        .get(hz)
                        .or_else(|| {
                            // RES3A–F → RES3.
                            let base = hz.strip_suffix(|c: char| ('A'..='F').contains(&c))?;
                            base.ends_with("RES3").then(|| names.get(base)).flatten()
                        })
                        .map(|&i| vec![(i, 1.0)])
                }
                None => None,
            };
        let Some(funcs) = funcs else {
            unmapped += cost;
            *unmapped_by.entry(id0.to_string()).or_default() += cost;
            continue;
        };
        match reg.resolve(id1, &rec[cn1]) {
            Some(shares) => add(&mut stock, &shares, &funcs, cost, n),
            None => match country_code(id1) {
                Some(cc) if reg.by_country.contains_key(&cc) => {
                    let p = pools.entry(cc).or_default();
                    for &(f, w) in &funcs {
                        let x = p.entry(f).or_default();
                        x.0 += cost * w;
                        x.1 += n * w;
                    }
                    pooled_units.entry(cc).or_default().insert(id1.to_string());
                }
                _ => lost += cost,
            },
        }
    }

    // Havuzlar: ülkenin stok almamış bölgelerine (yoksa tümüne).
    let mut pooled = 0.0;
    for (cc, pool) in &pools {
        let all = &reg.by_country[cc];
        let empty: Vec<u16> = all
            .iter()
            .copied()
            .filter(|i| !stock.contains_key(i))
            .collect();
        let target = if empty.is_empty() { all } else { &empty };
        let shares = reg.shares(target);
        let pool_cost: f64 = pool.values().map(|x| x.0).sum();
        pooled += pool_cost;
        for (&f, &(c, n)) in pool {
            add(&mut stock, &shares, &[(f, 1.0)], c, n);
        }
        let units = &pooled_units[cc];
        println!(
            "  {}: {} birim eşlenemedi ({:.1} milyar USD) → {} bölgeye ({}) dağıtıldı",
            String::from_utf8_lossy(cc),
            units.len(),
            pool_cost / 1e9,
            target.len(),
            if empty.is_empty() {
                "tüm bölgeler"
            } else {
                "stoksuz bölgeler"
            }
        );
    }

    let model = VulnModel {
        functions,
        regions: stock
            .into_iter()
            .map(|(id, m)| {
                let mut entries: Vec<(u16, f32, f32)> = m
                    .into_iter()
                    .map(|(f, (c, n))| (f, c as f32, n as f32))
                    .collect();
                entries.sort_by_key(|e| e.0);
                VulnRegion {
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
    model.write(&mut w)?;
    drop(w);
    for (c, v) in &unmapped_by {
        println!(
            "  {c}: fonksiyonu bulunamayan sınıf {:.1} milyar USD",
            v / 1e9
        );
    }
    println!(
        "\n→ {} ({} bölge, {} fonksiyon, {:.2} MB)\ntoplam {:.1} trilyon USD; ülke içinde \
         dağıtılan %{:.2}; bölgesiz %{:.3}; fonksiyonsuz %{:.3}; eşleme ağırlığı \
         yeniden ölçeklenen {renormalized}",
        output.display(),
        model.regions.len(),
        model.functions.len(),
        std::fs::metadata(output)?.len() as f64 / 1e6,
        total / 1e12,
        pooled / total * 100.0,
        lost / total * 100.0,
        unmapped / total * 100.0,
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ad_normallesir() {
        assert_eq!(norm("Ciudad de México"), "ciudaddemexico");
        assert_eq!(norm("Khorāsān-e Raẕavī"), "khorasanerazavi");
        assert_eq!(norm("Talo'fo'fo"), "talofofo");
    }

    #[test]
    fn zafiyet_xml_okunur() {
        let xml = r#"<vulnerabilityModel>
<vulnerabilityFunction id="A/B/H:1" dist="BT">
<imls imt="SA(0.3)" > 0.05 0.1 0.2 </imls>
<meanLRs>1e-08 0.5 0.99 </meanLRs>
<covLRs>1e-08 1 1 </covLRs>
</vulnerabilityFunction>
<vulnerabilityFunction id="C" dist="BT">
<imls imt="PGA">0.1 1</imls>
<meanLRs>0 1</meanLRs>
</vulnerabilityFunction>
</vulnerabilityModel>"#;
        let f = read_vulnerability(xml).unwrap();
        assert_eq!(f.len(), 2);
        assert_eq!(f[0].name, "A/B/H:1");
        assert_eq!(f[0].imt, FragImt::Sa03);
        assert_eq!(f[0].iml, vec![0.05, 0.1, 0.2]);
        assert_eq!(f[0].mean_lr[1], 0.5);
        assert_eq!(f[1].imt, FragImt::Pga);
    }

    #[test]
    fn ulke_kodu() {
        assert_eq!(country_code("JP-23"), Some(*b"JP"));
        assert_eq!(country_code("XKX"), None);
    }
}
