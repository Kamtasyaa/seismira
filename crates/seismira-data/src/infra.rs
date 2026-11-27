//! `infra` komutu: `scripts/gen_infra.py`'nin ara TSV'sinden kritik tesis
//! dosyası (`.sinf`, [`seismira_core::infra`]).
//!
//! Ayıklama (SEÇİM, docs/veri.md "Kritik tesisler"):
//! - 20 m'den kısa köprüler atlanır (çoğu menfez / kısa geçit; Türkiye
//!   özütünde köprülerin %12'si).
//! - İkiz köprüler: aynı türde birbirine 40 m'den yakın köprülerden uzun
//!   olanı tutulur (bölünmüş yolda iki yönün ayrı yolları).
//! - Tekrarlar: kıta özütlerinin sınırda örtüşmesi (aynı tür, ad, ~1 m).

use anyhow::{Context, Result, bail};
use seismira_core::infra::{self, Facility, Kind};
use std::collections::HashMap;
use std::path::Path;

pub fn run(tsv: &Path, out: &Path) -> Result<()> {
    let text = std::fs::read_to_string(tsv).with_context(|| tsv.display().to_string())?;
    let mut all = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() != 6 {
            bail!("{}. satır: {} sütun", i + 1, f.len());
        }
        let kind = Kind::parse(f[0]).with_context(|| format!("tür {}", f[0]))?;
        let lonlat = (f[1].parse::<f64>()?, f[2].parse::<f64>()?);
        let attr = f[3].parse::<u32>()?.min(u32::from(u16::MAX)) as u16;
        let line_pts: Vec<(f32, f32)> = if f[5].is_empty() {
            Vec::new()
        } else {
            f[5].split(';')
                .map(|p| {
                    let (x, y) = p.split_once(' ').context("köşe")?;
                    Ok((x.parse()?, y.parse()?))
                })
                .collect::<Result<_>>()?
        };
        all.push(Facility {
            kind,
            lonlat,
            attr,
            name: f[4].to_string(),
            line: line_pts,
        });
    }
    let n_raw = all.len();
    // Kıta özütleri sınırda örtüşebilir: aynı tür, aynı ad, ~1 m içinde
    // aynı konum tekrar sayılmaz.
    let mut seen = std::collections::HashSet::new();
    all.retain(|f| {
        seen.insert((
            f.kind,
            (f.lonlat.0 * 1e5).round() as i64,
            (f.lonlat.1 * 1e5).round() as i64,
            f.name.clone(),
        ))
    });
    let n_dup = n_raw - all.len();
    let n_in = all.len();
    let is_bridge = |k: Kind| matches!(k, Kind::Bridge | Kind::Viaduct);
    all.retain(|f| !is_bridge(f.kind) || f.attr >= 20);
    let n_short = n_in - all.len();
    // İkiz köprüler: ~40 m'lik ızgarada komşu hücreler.
    let cell = 0.0004;
    let key = |p: (f64, f64)| ((p.0 / cell).floor() as i64, (p.1 / cell).floor() as i64);
    let mut grid: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
    let mut keep = vec![true; all.len()];
    let mut order: Vec<usize> = (0..all.len()).filter(|&i| is_bridge(all[i].kind)).collect();
    order.sort_by(|&a, &b| all[b].attr.cmp(&all[a].attr));
    for i in order {
        let (kx, ky) = key(all[i].lonlat);
        let twin = (-1..=1).any(|dx| {
            (-1..=1).any(|dy| {
                grid.get(&(kx + dx, ky + dy)).is_some_and(|v| {
                    v.iter().any(|&j| {
                        all[j].kind == all[i].kind
                            && seismira_core::geo::distance_km(all[j].lonlat, all[i].lonlat) < 0.04
                    })
                })
            })
        });
        if twin {
            keep[i] = false;
        } else {
            grid.entry((kx, ky)).or_default().push(i);
        }
    }
    let n_twin = keep.iter().filter(|k| !**k).count();
    let kept: Vec<Facility> = all
        .into_iter()
        .zip(keep)
        .filter_map(|(f, k)| k.then_some(f))
        .collect();
    let mut counts: HashMap<Kind, usize> = HashMap::new();
    for f in &kept {
        *counts.entry(f.kind).or_default() += 1;
    }
    let mut w = std::io::BufWriter::new(std::fs::File::create(out)?);
    infra::write(&kept, 10.0, |raw| zstd::bulk::compress(raw, 19), &mut w)?;
    drop(w);
    let mut c: Vec<_> = counts.into_iter().collect();
    c.sort();
    println!(
        "{n_raw} kayıt; tekrar {n_dup}, kısa köprü {n_short}, ikiz köprü {n_twin} atlandı; {} tesis {:?}; {} ({} bayt)",
        kept.len(),
        c,
        out.display(),
        std::fs::metadata(out)?.len()
    );
    Ok(())
}
