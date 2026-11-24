//! `validate-damage` komutu: 2023 Kahramanmaraş depremlerinde il bazında
//! tahmini ağır hasar / göçme oranını resmî hasar tespitiyle karşılaştırır
//! (Faz 6, CLAUDE.md Bölüm 14).
//!
//! Uygulamanın zinciriyle aynı: bizim fay geometrimiz, Vs30 haritası, ESRM20
//! bina stoku, `damage::regions` (adım = ince adımın iki katı). İki ana şok
//! (USGS: Mw 7,8 Pazarcık, Mw 7,5 Elbistan) ayrı hesaplanır ve il başına
//! ağır hasar ORANLARI bağımsızlık varsayımıyla birleştirilir:
//! f = 1 − (1 − f₁)(1 − f₂). Artçılar (ör. 20 Şubat Mw 6,4 Hatay) yok.
//!
//! Gözlem: SBB (Mart 2023) Tablo 20, ÇŞİDB hasar tespiti, KONUT sayısı
//! (acil yıkılacak + ağır hasarlı + yıkık). Oranın paydası ESRM20 konut
//! sayısı (aynı ilin konut stoku; resmî toplam konut sayısı raporda yok).
//! Bina sayısı karşılaştırması: SBB Tablo 11 (MAKS) ile ESRM20 bina toplamı.

use anyhow::{Context, Result};
use seismira_core::damage::{self, RegionDamage};
use seismira_core::exposure::Exposure;
use seismira_core::grid::{Base, Domain};
use seismira_core::quake::{FaultType, Quake};
use seismira_core::region::RegionMap;
use seismira_core::shaking::Scenario;
use seismira_core::site::Vs30Map;
use seismira_core::traveltime::TravelTimes;
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::AtomicBool;

/// USGS olay sayfaları (us6000jllz, us6000jlqa): merkez üssü, derinlik,
/// Mww düğüm düzlemi (fay izine uyan düzlem).
///
/// Pazarcık (2026-11-24): uygulamanın hazır senaryosu (ShakeMap kırılma
/// modelinden 268 × 15 km, odak doğrultunun %57'sinde), çünkü otomatik boyut
/// (TMG17 ile 234 km, odak ortada) gerçek kırılmanın güney ucunu (Hatay)
/// kapsamıyor. Elbistan otomatik boyutla (146 km; gözlenen ~150 km).
fn events() -> Result<Vec<(&'static str, Quake)>> {
    let mk = |lon, lat, depth, mw, strike, dip| {
        let mut q = Quake::new((lon, lat));
        q.depth_km = depth;
        q.mw = mw;
        q.strike = strike;
        q.dip = dip;
        q.fault = FaultType::StrikeSlip;
        q.rake = 0.0;
        q
    };
    Ok(vec![
        (
            "Mw 7,8 Pazarcık (hazır senaryo)",
            crate::scenario_quake("kahramanmaras2023")?,
        ),
        (
            "Mw 7,5 Elbistan",
            mk(37.1962, 38.0106, 7.4, 7.5, 277.2, 77.9),
        ),
    ])
}

/// (bölge kimliği → hasar) bir deprem için.
fn run_event(
    q: Quake,
    tt: &TravelTimes,
    vs30: &Vs30Map<Vec<u8>>,
    regions: &RegionMap<Vec<u8>>,
    exposure: &Exposure,
) -> Result<HashMap<u16, RegionDamage>> {
    let scenario = Scenario::new(q, tt).with_crust_region(crate::crust_region_at(q.epicenter)?);
    let g = vs30.grid();
    let base = Base {
        west: g.west,
        north: g.north,
        cell: g.cell,
    };
    let domain = Domain::for_scenario(&scenario, base);
    let win = vs30.window(domain.col0, domain.row0, domain.w, domain.h)?;
    let ids = regions.window(domain.col0, domain.row0, domain.w, domain.h)?;
    let step = (domain.fine_step() * 2).max(2);
    let cancel = AtomicBool::new(false);
    let out = damage::regions(
        &scenario,
        exposure,
        &domain,
        step,
        &ids,
        Some(&win),
        &cancel,
    )
    .context("iptal")?;
    Ok(out.into_iter().map(|d| (d.id, d)).collect())
}

pub fn run(sbb_csv: &Path, esrm20_turkey_res: &Path) -> Result<()> {
    let data = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../data"));
    let tt = TravelTimes::load(&data.join("traveltime/ak135.ttt"))?;
    let vs30 = Vs30Map::open(std::fs::read(data.join("vs30/vs30.sras"))?)?;
    let regions = RegionMap::open(std::fs::read(data.join("regions/admin1.sras"))?)?;
    let infos = seismira_core::region::read_infos(&mut std::io::BufReader::new(
        std::fs::File::open(data.join("regions/admin1.sreg"))?,
    ))?;
    let exposure = Exposure::read(&mut std::io::BufReader::new(std::fs::File::open(
        data.join("exposure/esrm20.sexp"),
    )?))?;
    let iso_to_id: HashMap<&str, u16> = infos
        .iter()
        .enumerate()
        .map(|(i, r)| (r.iso.as_str(), (i + 1) as u16))
        .collect();

    // ESRM20 konut sayısı (il; NAME_1 yazımı farklı olabildiği için
    // koordinatla bölgeye atanır, `exposure` komutuyla aynı).
    let mut dwellings: HashMap<u16, f64> = HashMap::new();
    let mut rdr = csv::Reader::from_path(esrm20_turkey_res)?;
    let h = rdr.headers()?.clone();
    let col = |n: &str| h.iter().position(|x| x == n).context(n.to_string());
    let (c_lon, c_lat, c_dw) = (col("LONGITUDE")?, col("LATITUDE")?, col("DWELLINGS")?);
    for rec in rdr.records() {
        let rec = rec?;
        let id = regions.id_at(rec[c_lon].parse()?, rec[c_lat].parse()?)?;
        *dwellings.entry(id).or_default() += rec[c_dw].parse::<f64>()?;
    }

    // Olay başına ağır hasar oranı (medyan, −1τ, +1τ).
    let mut per_event: Vec<HashMap<u16, [f64; 3]>> = Vec::new();
    for (name, q) in events()? {
        let t = std::time::Instant::now();
        let d = run_event(q, &tt, &vs30, &regions, &exposure)?;
        println!("{name}: {} bölge ({} ms)", d.len(), t.elapsed().as_millis());
        per_event.push(
            d.into_iter()
                .filter(|(_, r)| r.buildings > 0.0)
                .map(|(id, r)| {
                    let f = |x: &[f64; 4]| RegionDamage::heavy(x) / r.buildings;
                    (id, [f(&r.median), f(&r.low), f(&r.high)])
                })
                .collect(),
        );
    }
    let combined = |id: u16| -> [f64; 3] {
        let mut keep = [1.0; 3];
        for e in &per_event {
            if let Some(f) = e.get(&id) {
                for k in 0..3 {
                    keep[k] *= 1.0 - f[k];
                }
            }
        }
        keep.map(|k| 1.0 - k)
    };

    println!(
        "\n| İl | ESRM20 bina | MAKS bina | model ağır+göçme % (olası) | gözlenen konut % | model / gözlenen |"
    );
    println!("|---|---|---|---|---|---|");
    let (mut m_sum, mut b_sum, mut maks_sum, mut obs_sum, mut dw_sum) = (0.0, 0.0, 0.0, 0.0, 0.0);
    let mut log_ratios = Vec::new();
    let mut rdr = csv::ReaderBuilder::new()
        .comment(Some(b'#'))
        .from_path(sbb_csv)?;
    for rec in rdr.deserialize::<(String, String, f64, f64, f64)>() {
        let (iso, il, maks, obs_dw, _orta) = rec?;
        let id = *iso_to_id
            .get(iso.as_str())
            .with_context(|| format!("{iso} bölgesi yok"))?;
        let stock = exposure.region(id).context("stok yok")?;
        let b = stock.buildings();
        let dw = dwellings.get(&id).copied().unwrap_or(f64::NAN);
        let [m, lo, hi] = combined(id);
        let obs = obs_dw / dw;
        m_sum += m * b;
        b_sum += b;
        maks_sum += maks;
        obs_sum += obs_dw;
        dw_sum += dw;
        log_ratios.push((m / obs).ln());
        println!(
            "| {il} | {b:.0} | {maks:.0} | {:.1} ({:.1}–{:.1}) | {:.1} | {:.2} |",
            m * 100.0,
            lo * 100.0,
            hi * 100.0,
            obs * 100.0,
            m / obs
        );
    }
    let n = log_ratios.len() as f64;
    let mean = log_ratios.iter().sum::<f64>() / n;
    let sd = (log_ratios.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0)).sqrt();
    println!(
        "\n11 il: model {:.1} % ({:.0} bina / {:.0} ESRM20 bina); gözlenen konut {:.1} % \
         ({:.0} / {:.0} ESRM20 konut); gözlenen bina (ülke toplamı, SBB Tablo 12) 232632 / {:.0} \
         MAKS bina = {:.1} %",
        m_sum / b_sum * 100.0,
        m_sum,
        b_sum,
        obs_sum / dw_sum * 100.0,
        obs_sum,
        dw_sum,
        maks_sum,
        232_632.0 / maks_sum * 100.0
    );
    println!(
        "il başına ln(model / gözlenen): ortalama {mean:+.2} (×{:.2}), std {sd:.2}",
        mean.exp()
    );
    Ok(())
}
