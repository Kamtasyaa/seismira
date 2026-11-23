//! `validate-gem` komutu: GEM zafiyet modelinin bölge (Adm1) ortalama kayıp
//! oranını gözlemle karşılaştırır (Faz 9, CLAUDE.md Bölüm 14).
//!
//! Zincir uygulamayla aynı: bizim fay geometrimiz, Vs30 haritası, bölge
//! rasterı, `damage::loss_regions` (adım = ince adımın iki katı), Slab2 (levha
//! içi deprem için Parker bölgesi). Bir dizideki depremler ayrı hesaplanıp
//! kayıp oranları bağımsızlık varsayımıyla birleştirilir:
//! r = 1 − (1 − r₁)(1 − r₂) (validate-damage ile aynı; hasar birikimi yok).
//!
//! Olay parametreleri: USGS ComCat (merkez üssü, derinlik, Mww düğüm düzlemi;
//! fay izine uyan düzlem), 2026-11-23 sorgulandı.
//!
//! Gözlem: Kumamoto (aşağıda) ve Puebla 2017 eyaletleri (CENAPRED 2018,
//! [`observed_counts`]); Ridgecrest için yalnızca model yazdırılır:
//! - Kumamoto ili konut hasarı, Kumamoto Valiliği Kriz Yönetimi ve Afet
//!   Önleme Dairesi, "平成28年熊本地震等に係る被害状況について 第253報"
//!   (2017-08-10): tam yıkık (全壊) 8 648, yarı (半壊) 34 186, kısmi
//!   (一部破損) 153 557 bina (棟). Not: bazı belediyeler hasar belgesi
//!   başvurusu sayar; apartmanlar çift sayılmış olabilir.
//! - Sınıf → kayıp oranı aralığı: Kabine Ofisi hasar tespit yönergesi
//!   (平成13年 府政防第518号; 2021 baskısı r303shishin_1.pdf): 全壊 ≥ %50,
//!   半壊 %20–50, 2016'da 一部破損 < %20 (準半壊 ayrımı 2020'de geldi).
//!   Alt sınır: 0,5·全壊 + 0,2·半壊; üst sınır: 1·全壊 + 0,5·半壊 + 0,2·一部.
//! - Payda: GEM konut (RES) bina sayısı, JP-43 (Adm1 özeti; 563 634).
//!
//! Karşılaştırma kaba: gözlem konut SAYISI oranı, model tüm kullanımların
//! DEĞER ağırlıklı oranı.

use anyhow::{Context, Result};
use seismira_core::damage::{self, VulnCurves};
use seismira_core::grid::{Base, Domain};
use seismira_core::quake::{FaultType, Quake, Setting};
use seismira_core::region::RegionMap;
use seismira_core::shaking::Scenario;
use seismira_core::site::Vs30Map;
use seismira_core::slab::Slab2;
use seismira_core::traveltime::TravelTimes;
use seismira_core::vulnerability::VulnModel;
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::AtomicBool;

#[allow(clippy::too_many_arguments)]
fn quake(
    lon: f64,
    lat: f64,
    depth: f64,
    mw: f64,
    strike: f64,
    dip: f64,
    rake: f64,
    setting: Setting,
) -> Quake {
    let mut q = Quake::new((lon, lat));
    q.depth_km = depth;
    q.mw = mw;
    q.strike = strike;
    q.dip = dip;
    q.rake = rake;
    q.fault = FaultType::from_rake(rake);
    q.setting = setting;
    q
}

struct Sequence {
    name: &'static str,
    events: Vec<(&'static str, Quake)>,
    /// Yazdırılacak bölgeler (ISO 3166-2, Natural Earth kodu).
    regions: &'static [&'static str],
}

fn sequences() -> Vec<Sequence> {
    use Setting::{Intraslab, ShallowCrust as Crust};
    vec![
        Sequence {
            name: "2016 Kumamoto",
            events: vec![
                (
                    "us20005hzn Mw 6,2 öncü",
                    quake(130.7042, 32.788, 9.0, 6.2, 210.0, 81.0, 164.0, Crust),
                ),
                (
                    "us20005iis Mw 7,0 ana şok",
                    quake(130.7543, 32.7906, 10.0, 7.0, 224.0, 66.0, -151.0, Crust),
                ),
            ],
            regions: &["JP-43", "JP-44", "JP-42", "JP-45", "JP-41", "JP-40"],
        },
        Sequence {
            name: "2019 Ridgecrest",
            events: vec![
                (
                    "ci38443183 Mw 6,4 öncü",
                    quake(-117.5038, 35.7053, 10.5, 6.4, 226.25, 81.76, 1.21, Crust),
                ),
                (
                    "ci38457511 Mw 7,1 ana şok",
                    quake(-117.5993, 35.7695, 8.0, 7.1, 320.79, 83.91, -171.28, Crust),
                ),
            ],
            regions: &["US-CA", "US-NV"],
        },
        Sequence {
            name: "2017 Puebla",
            events: vec![(
                "us2000ar20 Mw 7,1",
                quake(
                    -98.4887, 18.5499, 48.0, 7.1, 299.27, 43.98, -82.1, Intraslab,
                ),
            )],
            regions: &[
                "MX-MOR", "MX-PUE", "MX-DIF", "MX-MEX", "MX-GRO", "MX-OAX", "MX-TLA",
            ],
        },
    ]
}

/// Gözlenen hasar sayıları: (tam, kısmi, hafif, payda). Sınıf → kayıp oranı
/// aralığı Kumamoto'daki Japon tespit eşiklerinden (tam ≥ %50, kısmi
/// %20–50, hafif < %20): alt = 0,5·tam + 0,2·kısmi, üst = tam + 0,5·kısmi +
/// 0,2·hafif. Meksika'da sınıfların yüzde tanımı yok; VARSAYIM aynı bantlar
/// (FONDEN destek tutarları kısmi / tam = 28 000 / 120 000 ≈ 0,23 ile
/// uyumlu; CENAPRED 2018, metin).
fn observed_counts(iso: &str) -> Option<(f64, f64, f64, f64)> {
    Some(match iso {
        // Kumamoto Valiliği 第253報 (2017-08-10): 全壊 / 半壊 / 一部破損 bina;
        // payda GEM konut BİNA sayısı (JP-43).
        "JP-43" => (8_648.0, 34_186.0, 153_557.0, 563_634.0),
        // CENAPRED (2018), "Impacto socioeconómico de los principales
        // desastres ocurridos en México 2017", 19 Eylül depremi, konut
        // (vivienda); payda GEM konut (DWELLINGS).
        // Morelos: Tablo 3.108 (SEDATU), kısmi 9 697, tam 6 104.
        "MX-MOR" => (6_104.0, 9_697.0, 0.0, 824_111.0),
        // Puebla: Tablo 3.135 (FONDEN), kısmi 22 707, tam 5 638.
        "MX-PUE" => (5_638.0, 22_707.0, 0.0, 2_204_238.0),
        // Edo. de México: Tablo 3.151 (SEDATU), tam 2 494 + taşınma 208,
        // kısmi 2 109, hafif 1 249.
        "MX-MEX" => (2_702.0, 2_109.0, 1_249.0, 5_525_475.0),
        // Guerrero (19 Eylül, 19 belediye): 3 928 hasarlı; kısmi 1 443, tam
        // 2 047, kalan 438 hafif sayıldı (metin).
        "MX-GRO" => (2_047.0, 1_443.0, 438.0, 1_231_314.0),
        // Tlaxcala: 34 konut, 23 kısmi, 11 hafif (metin).
        "MX-TLA" => (0.0, 23.0, 11.0, 436_069.0),
        // Ciudad de México: Tablo 3.71 (Plataforma CDMX, Haziran 2018),
        // BİNA (inmueble): kırmızı (yıkım) 529 → tam, yeşil (onarılabilir
        // yapısal) 10 531 → kısmi, sarı (hafif) 3 752. Payda GEM konut BİNA
        // sayısı (345 336; GEM'in CDMX bina sayısı düşük olabilir).
        "MX-DIF" => (529.0, 10_531.0, 3_752.0, 345_336.0),
        _ => return None,
    })
}

/// Gözlenen kayıp oranı aralığı (yukarıdaki kural).
fn observed(iso: &str) -> Option<(f64, f64)> {
    let (total, partial, minor, n) = observed_counts(iso)?;
    Some((
        (0.5 * total + 0.2 * partial) / n,
        (total + 0.5 * partial + 0.2 * minor) / n,
    ))
}

pub fn run(slab_path: &Path) -> Result<()> {
    let data = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../data"));
    let tt = TravelTimes::load(&data.join("traveltime/ak135.ttt"))?;
    let vs30 = Vs30Map::open(std::fs::read(data.join("vs30/vs30.sras"))?)?;
    let regions = RegionMap::open(std::fs::read(data.join("regions/admin1.sras"))?)?;
    let infos = seismira_core::region::read_infos(&mut std::io::BufReader::new(
        std::fs::File::open(data.join("regions/admin1.sreg"))?,
    ))?;
    let model = VulnModel::read(&mut std::io::BufReader::new(
        std::fs::File::open(data.join("gem/gem.svul")).context("data/gem/gem.svul")?,
    ))?;
    let curves = VulnCurves::new(&model);
    let slab = Slab2::load(slab_path).with_context(|| slab_path.display().to_string())?;
    let iso_to_id: HashMap<&str, u16> = infos
        .iter()
        .enumerate()
        .map(|(i, r)| (r.iso.as_str(), (i + 1) as u16))
        .collect();
    let g = vs30.grid();
    let base = Base {
        west: g.west,
        north: g.north,
        cell: g.cell,
    };

    for seq in sequences() {
        println!("\n## {}", seq.name);
        let mut per_event: Vec<HashMap<u16, [f64; 3]>> = Vec::new();
        for (name, q) in seq.events {
            let t = std::time::Instant::now();
            let scenario = Scenario::with_slab(q, &tt, Some(&slab));
            let domain = Domain::for_scenario(&scenario, base);
            let win = vs30.window(domain.col0, domain.row0, domain.w, domain.h)?;
            let ids = regions.window(domain.col0, domain.row0, domain.w, domain.h)?;
            let step = (domain.fine_step() * 2).max(2);
            let cancel = AtomicBool::new(false);
            let l = damage::loss_regions(
                &scenario,
                &curves,
                &domain,
                step,
                &ids,
                Some(&win),
                &cancel,
                |_| false,
            )
            .context("iptal")?;
            println!("{name}: {} bölge ({} ms)", l.len(), t.elapsed().as_millis());
            per_event.push(l.into_iter().map(|r| (r.id, r.ratio)).collect());
        }
        println!(
            "\n| Bölge | model ortalama kayıp oranı % (olası) | gözlenen % | model / gözlenen |"
        );
        println!("|---|---|---|---|");
        for iso in seq.regions {
            let id = *iso_to_id
                .get(iso)
                .with_context(|| format!("{iso} bölgesi yok"))?;
            let mut keep = [1.0; 3];
            for e in &per_event {
                if let Some(r) = e.get(&id) {
                    for k in 0..3 {
                        keep[k] *= 1.0 - r[k];
                    }
                }
            }
            let [m, lo, hi] = keep.map(|k| (1.0 - k) * 100.0);
            let (obs, ratio) = match observed(iso) {
                Some((a, b)) => (
                    format!("{:.3}–{:.3}", a * 100.0, b * 100.0),
                    format!("×{:.1}–{:.1}", m / (b * 100.0), m / (a * 100.0)),
                ),
                None => ("—".into(), "—".into()),
            };
            let name = &infos[id as usize - 1].name_en;
            println!("| {iso} {name} | {m:.2} ({lo:.2}–{hi:.2}) | {obs} | {ratio} |");
        }
    }
    Ok(())
}
