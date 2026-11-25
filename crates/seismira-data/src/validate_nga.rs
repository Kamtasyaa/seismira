//! NGA-West2 kayıtlarıyla CY14 doğrulaması (yalnızca sonuç karşılaştırma).
//!
//! Kaynak: PEER NGA-West2 "Updated_NGA_West2_Flatfile_RotD50_d050_public_version"
//! (Ancheta vd. 2014, Earthquake Spectra 30(3) 989–1005, doi:10.1193/070913EQS197M).
//! Ham dosya `data/raw/validation/nga/` altında durur, depoya girmez
//! (yeniden dağıtım izni sorulmuş, yanıt yok; bkz. docs/dogrulama.md).
//!
//! Üç karşılaştırma, hata kaynaklarını ayırmak için:
//! - A: NGA'nın kendi Mw, mesafe (Rrup, Rjb, Rx), Ztor, eğim, mekanizma ve
//!   ölçülmüş Vs30 değeri → yalnızca CY14 uygulaması ve olayın kendi terimi.
//! - B: bizim fay geometrimiz (senaryo ya da NGA odak parametreleriyle
//!   otomatik boyut) + ölçülmüş Vs30 → geometri etkisi.
//! - C: bizim geometri + bizim Vs30 haritası → uygulamadaki sonuç.
//!
//! Gözlem RotD50 PGV (cm/s). Model de RotD50 verir (CY14), dolayısıyla
//! bileşen dönüşümü gerekmez.

use anyhow::{Context, Result, bail};
use seismira_core::gmpe::{Imt, ngaw2};
use seismira_core::intensity::jma_from_pgv_vector;
use seismira_core::quake::{FaultType, Quake};
use seismira_core::rupture::Rupture;
use seismira_core::site::Vs30Map;
use std::path::Path;

struct Record {
    station: String,
    site: (f64, f64),
    pgv: f64,
    vs30: f64,
    rrup: f64,
    rjb: f64,
    rx: f64,
}

struct Event {
    mw: f64,
    strike: f64,
    dip: f64,
    rake: f64,
    hypo: (f64, f64, f64),
    ztor: f64,
    length: f64,
    width: f64,
}

/// Sütun adları (flatfile başlığı, satır sonları boşluğa çevrilmiş).
const COLS: [&str; 18] = [
    "Earthquake Name",
    "Station Name",
    "Earthquake Magnitude",
    "Strike (deg)",
    "Dip (deg)",
    "Rake Angle (deg)",
    "Hypocenter Latitude (deg)",
    "Hypocenter Longitude (deg)",
    "Hypocenter Depth (km)",
    "Depth to Top Of Fault Rupture Model",
    "Fault Rupture Length for Calculation of Ry (km)",
    "Fault Rupture Width (km)",
    "Joyner-Boore Dist. (km)",
    "ClstD (km)",
    "Rx",
    "Vs30 (m/s) selected for analysis",
    "Station Latitude",
    "Station Longitude",
];
const PGV_COL: &str = "PGV (cm/sec)";

fn read(path: &Path, event: &str) -> Result<(Event, Vec<Record>, usize)> {
    let mut rdr = csv::Reader::from_path(path).with_context(|| path.display().to_string())?;
    let head: Vec<String> = rdr
        .headers()?
        .iter()
        .map(|h| h.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect();
    let idx = |name: &str| {
        head.iter()
            .position(|h| h == name)
            .with_context(|| format!("sütun yok: {name}"))
    };
    let ix: Vec<usize> = COLS.iter().map(|c| idx(c)).collect::<Result<_>>()?;
    let pgv_ix = idx(PGV_COL)?;
    let mut ev = None;
    let mut recs = Vec::new();
    let mut no_pgv = 0;
    for row in rdr.records() {
        let row = row?;
        if row.get(ix[0]) != Some(event) {
            continue;
        }
        let num = |k: usize| -> Result<f64> {
            let s = row.get(ix[k]).unwrap_or("").trim();
            s.parse::<f64>()
                .with_context(|| format!("{}: {s:?}", COLS[k]))
        };
        if ev.is_none() {
            ev = Some(Event {
                mw: num(2)?,
                strike: num(3)?,
                dip: num(4)?,
                rake: num(5)?,
                hypo: (num(7)?, num(6)?, num(8)?),
                ztor: num(9)?,
                length: num(10)?,
                width: num(11)?,
            });
        }
        let pgv: f64 = row.get(pgv_ix).unwrap_or("").trim().parse()?;
        if pgv <= 0.0 {
            no_pgv += 1; // flatfile'da -999: değer yok
            continue;
        }
        recs.push(Record {
            station: row.get(ix[1]).unwrap_or("").to_string(),
            site: (num(17)?, num(16)?),
            pgv,
            vs30: num(15)?,
            rrup: num(13)?,
            rjb: num(12)?,
            rx: num(14)?,
        });
    }
    let Some(ev) = ev else {
        bail!("olay bulunamadı: {event:?}")
    };
    Ok((ev, recs, no_pgv))
}

pub fn run(flatfile: &Path, event: &str, scenario: Option<&str>) -> Result<()> {
    let (ev, recs, no_pgv) = read(flatfile, event)?;
    let data = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../data"));
    let vs30_map = Vs30Map::open(std::fs::read(data.join("vs30/vs30.sras"))?)?;

    let nga_fault = FaultType::from_rake(ev.rake);
    let q = match scenario {
        Some(id) => crate::scenario_quake(id)?,
        None => {
            let mut q = Quake::new((ev.hypo.0, ev.hypo.1));
            q.mw = ev.mw;
            q.depth_km = ev.hypo.2;
            q.strike = ev.strike;
            q.dip = ev.dip;
            q.rake = ev.rake;
            q.fault = nga_fault;
            q
        }
    };
    let ours = Rupture::from_quake(&q);
    let region = crate::crust_region_at((ev.hypo.0, ev.hypo.1))?;
    let clamp = |v: f64| v.clamp(ngaw2::VS30_MIN, ngaw2::VS30_MAX);
    // Dört NGA-West2 modelinin ortalaması (uygulamayla aynı). Odak derinliği
    // ve genişlik: A'da NGA'nın, B/C'de bizim fayımızınki.
    #[allow(clippy::too_many_arguments)]
    let cy = |mw, rrup, rjb, rx, ztor, dip, fault, vs30, zhyp, width| {
        ngaw2::predict(
            Imt::Pgv,
            &ngaw2::Input {
                mw,
                rrup,
                rjb,
                rx,
                ztor,
                zhyp,
                dip,
                width,
                fault,
                vs30: clamp(vs30),
                region,
            },
        )
        .map(|m| m.mean)
    };

    println!("NGA-West2: {event} (kabuk bölgesi {region:?})");
    println!(
        "  NGA: Mw {:.2}, doğrultu/eğim/kayma {}/{}/{}, Ztor {:.1} km, L × W = {:.0} × {:.0} km",
        ev.mw, ev.strike, ev.dip, ev.rake, ev.ztor, ev.length, ev.width
    );
    println!(
        "  bizim ({}): Mw {:.2}, doğrultu/eğim {:.0}/{:.0}, Ztor {:.1} km, L × W = {:.0} × {:.0} km",
        scenario.unwrap_or("NGA odak parametreleri, otomatik boyut"),
        q.mw,
        q.strike,
        q.dip,
        ours.ztor,
        ours.length,
        ours.width
    );

    // (istasyon, Rrup_NGA, ln gözlem, [A, B, C] ln tahmin)
    let mut rows: Vec<(&Record, f64, [Option<f64>; 3])> = Vec::new();
    let mut vs_ratio = Vec::new();
    let mut out_of_range = 0;
    for r in &recs {
        if r.rrup > ngaw2::MAX_RRUP_KM {
            out_of_range += 1;
            continue;
        }
        let a = cy(
            ev.mw, r.rrup, r.rjb, r.rx, ev.ztor, ev.dip, nga_fault, r.vs30, ev.hypo.2, ev.width,
        );
        let d = ours.distances(r.site);
        let b = cy(
            q.mw, d.rrup, d.rjb, d.rx, ours.ztor, q.dip, q.fault, r.vs30, q.depth_km, ours.width,
        );
        let site = vs30_map.site(r.site.0, r.site.1)?;
        vs_ratio.push((site.vs30 / r.vs30).ln());
        let c = cy(
            q.mw, d.rrup, d.rjb, d.rx, ours.ztor, q.dip, q.fault, site.vs30, q.depth_km, ours.width,
        );
        rows.push((r, r.pgv.ln(), [a, b, c]));
    }
    println!(
        "  {} kayıt; PGV'siz {no_pgv}, Rrup > {:.0} km {out_of_range} kayıt atlandı\n",
        rows.len(),
        ngaw2::MAX_RRUP_KM
    );

    let to_i = |ln: f64| jma_from_pgv_vector(ln.exp());
    let stats = |sel: &[&(&Record, f64, [Option<f64>; 3])], k: usize| {
        let res: Vec<(f64, f64)> = sel
            .iter()
            .filter_map(|(_, obs, p)| p[k].map(|p| (obs - p, to_i(*obs) - to_i(p))))
            .collect();
        let n = res.len() as f64;
        if n < 2.0 {
            return None;
        }
        let m = res.iter().map(|r| r.0).sum::<f64>() / n;
        let sd = (res.iter().map(|r| (r.0 - m).powi(2)).sum::<f64>() / (n - 1.0)).sqrt();
        let di = res.iter().map(|r| r.1).sum::<f64>() / n;
        let within = res.iter().filter(|r| r.1.abs() < 1.0).count() as f64 / n;
        Some((res.len(), m, sd, di, within))
    };
    let labels = [
        "A: NGA mesafe + ölçülmüş Vs30",
        "B: bizim geometri + ölçülmüş Vs30",
        "C: bizim geometri + bizim Vs30",
    ];
    println!("| karşılaştırma | n | ort. ln(göz/tah) | std ln | ort. ΔI (JMA) | |ΔI| < 1 oranı |");
    println!("|---|---|---|---|---|---|");
    let all: Vec<_> = rows.iter().collect();
    for (k, label) in labels.iter().enumerate() {
        match stats(&all, k) {
            Some((n, m, sd, di, w)) => println!(
                "| {label} | {n} | {m:+.2} | {sd:.2} | {di:+.2} | %{:.0} |",
                w * 100.0
            ),
            None => println!("| {label} | – | – | – | – | – |"),
        }
    }
    let n = vs_ratio.len() as f64;
    if n > 1.0 {
        let m = vs_ratio.iter().sum::<f64>() / n;
        let sd = (vs_ratio.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (n - 1.0)).sqrt();
        println!(
            "\nVs30 harita/ölçülmüş: ort. ln oran {m:+.2} (×{:.2}), std {sd:.2}",
            m.exp()
        );
    }

    println!("\n| Rrup NGA (km) | n | ort. ln A | ort. ln B | ort. ln C |\n|---|---|---|---|---|");
    for (lo, hi) in [(0.0, 20.0), (20.0, 50.0), (50.0, 100.0), (100.0, 300.0)] {
        let sel: Vec<_> = rows
            .iter()
            .filter(|r| (lo..hi).contains(&r.0.rrup))
            .collect();
        if sel.is_empty() {
            continue;
        }
        let mean = |k: usize| {
            let v: Vec<f64> = sel
                .iter()
                .filter_map(|(_, o, p)| p[k].map(|p| o - p))
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
            mean(0),
            mean(1),
            mean(2)
        );
    }

    println!(
        "\n| istasyon | Rrup | Vs30 göz/harita | PGV göz | ln A | ln B | ln C |\n|---|---|---|---|---|---|---|"
    );
    let f = |v: Option<f64>, o: f64| v.map_or("–".into(), |p| format!("{:+.2}", o - p));
    for (r, obs, p) in &rows {
        let site = vs30_map.site(r.site.0, r.site.1)?;
        println!(
            "| {} | {:.1} | {:.0}/{:.0} | {:.1} | {} | {} | {} |",
            r.station,
            r.rrup,
            r.vs30,
            site.vs30,
            r.pgv,
            f(p[0], *obs),
            f(p[1], *obs),
            f(p[2], *obs)
        );
    }
    Ok(())
}
