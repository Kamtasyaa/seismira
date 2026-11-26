//! Yönelim (CY14 ΔDPP) doğrulaması: ShakeMap istasyon kayıtlarının SA(0,3),
//! SA(1,0), SA(3,0) değerleriyle, yönelimli ve yönelimsiz tahminin
//! kalıntıları (docs/dogrulama.md).
//!
//! Gözlem: iki yatay kanalın (HN/HH/BH E-N ya da 1-2) geometrik ortalaması
//! (RotD50'ye yakın; Boore 2010'a göre RotD50/GMxy ≈ 1,0–1,06, iki hesapta
//! da aynı, karşılaştırmayı etkilemez). Zemin: istasyonun ShakeMap Vs30'u
//! (harita hatası karışmasın). Yalnızca Rrup ≤ 70 km (CY14 yönelim etki
//! alanı). Ölçüler: kalıntı ortalaması ve std (ln), yönelimsiz kalıntının
//! ΔDPP'ye eğimi (yönelim gerçekse pozitif), yönelimli std'nin düşüp
//! düşmediği.

use anyhow::{Context, Result};
use seismira_core::gmpe::Imt;
use seismira_core::shaking::Scenario;
use std::path::Path;

pub fn run(stations: &Path, id: &str) -> Result<()> {
    let q = crate::scenario_quake(id)?;
    let tt = seismira_core::traveltime::TravelTimes::load(Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/traveltime/ak135.ttt"
    )))?;
    let region = crate::crust_region_at(q.epicenter)?;
    let sc = Scenario::new(q, &tt).with_crust_region(region);
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(stations)?)?;
    let feats = v["features"].as_array().context("features")?;
    println!(
        "Yönelim doğrulaması: {id}, Mw {}, {} parça; istasyon Vs30, Rrup ≤ 70 km",
        q.mw,
        sc.fault.segments.len()
    );
    println!(
        "| ölçü | n | ΔDPP aralığı | yönelimsiz ort. / std | yönelimli (CY14 ¼) ort. / std | dört modele ort. / std | kalıntı–ΔDPP eğimi (r) |"
    );
    println!("|---|---|---|---|---|---|---|");
    for (name, t) in [("SA(0.3)", 0.3), ("SA(1.0)", 1.0), ("SA(3.0)", 3.0)] {
        let key = format!("sa({t:.1})");
        let mut rows = Vec::new();
        for f in feats {
            let p = &f["properties"];
            if p["station_type"] != "seismic" {
                continue;
            }
            let Some(vs30) = p["vs30"].as_f64() else {
                continue;
            };
            let c = f["geometry"]["coordinates"]
                .as_array()
                .context("koordinat")?;
            let site = (c[0].as_f64().unwrap_or(0.0), c[1].as_f64().unwrap_or(0.0));
            // Yatay kanallar: adı Z ile bitmeyenler; değer %g.
            let mut lns = Vec::new();
            for ch in p["channels"].as_array().into_iter().flatten() {
                let n = ch["name"].as_str().unwrap_or("");
                if n.ends_with('Z') {
                    continue;
                }
                for a in ch["amplitudes"].as_array().into_iter().flatten() {
                    if a["name"] == key.as_str()
                        && let Some(x) = a["value"].as_f64()
                        && x > 0.0
                    {
                        lns.push((x / 100.0).ln());
                    }
                }
            }
            if lns.len() < 2 {
                continue;
            }
            let obs = lns.iter().sum::<f64>() / lns.len() as f64;
            let d = sc.fault.distances(site);
            if d.rrup > 70.0 {
                continue;
            }
            let mut d0 = d;
            d0.dpp = 0.0;
            let v = sc.clamp_vs30(vs30);
            let (Some(with), Some(without)) = (
                sc.ln_motion(Imt::Sa(t), &d, v),
                sc.ln_motion(Imt::Sa(t), &d0, v),
            ) else {
                continue;
            };
            // Deney: CY14 terimi dört modele de uygulansaydı (ortalamada
            // CY14'ün ağırlığı ¼; tam terim = 4 × fark).
            let full = without.mean + 4.0 * (with.mean - without.mean);
            rows.push((d.dpp, obs - without.mean, obs - with.mean, obs - full));
        }
        let n = rows.len() as f64;
        if rows.is_empty() {
            continue;
        }
        let mean = |k: usize| rows.iter().map(|r| [r.0, r.1, r.2, r.3][k]).sum::<f64>() / n;
        let std = |k: usize, m: f64| {
            (rows
                .iter()
                .map(|r| ([r.0, r.1, r.2, r.3][k] - m).powi(2))
                .sum::<f64>()
                / (n - 1.0))
                .sqrt()
        };
        let (m0, m1, m2, m3) = (mean(0), mean(1), mean(2), mean(3));
        let (s0, s1, s2, s3) = (std(0, m0), std(1, m1), std(2, m2), std(3, m3));
        let cov = rows.iter().map(|r| (r.0 - m0) * (r.1 - m1)).sum::<f64>() / (n - 1.0);
        let slope = cov / (s0 * s0);
        let r = cov / (s0 * s1);
        let (lo, hi) = rows
            .iter()
            .fold((f64::MAX, f64::MIN), |(a, b), r| (a.min(r.0), b.max(r.0)));
        println!(
            "| {name} | {} | {lo:+.2}…{hi:+.2} | {m1:+.3} / {s1:.3} | {m2:+.3} / {s2:.3} | {m3:+.3} / {s3:.3} | {slope:+.3} ({r:+.2}) |",
            rows.len()
        );
    }
    Ok(())
}
