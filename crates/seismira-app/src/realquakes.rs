//! Gerçek depremleri internetten çekme (README "Önemli": USGS / EMSC;
//! isteğe bağlı, yalnızca kullanıcı isteyince; uygulama internetsiz çalışır).
//!
//! Kaynaklar ve koşullar (resmi sayfalarından okundu, 2026-11-26; docs/veri.md):
//! - USGS: veriler ABD kamu malı ("USGS-authored or produced data and
//!   information are considered to be in the U.S. Public Domain"), atıf rica
//!   ediliyor ("Credit: U.S. Geological Survey"). Otomatik uygulamalar için
//!   gerçek zamanlı GeoJSON akışları öneriliyor (fdsnws/event/1 sayfası):
//!   `earthquakes/feed/v1.0/summary/4.5_month.geojson`. Moment tensörü olayın
//!   `detail` GeoJSON'undan (ürün "moment-tensor", "nodal-plane-1-*").
//! - EMSC-CSEM SeismicPortal: veriler CC BY 4.0, atıf "Credit: EMSC-CSEM
//!   SeismicPortal, https://www.seismicportal.eu"; FDSN olay servisi
//!   `fdsnws/event/1/query?format=json` (2026-11-26 denendi).
//! - AFAD: koşullar ve API belgesi resmi sitede bulunamadı → EKLENMEDİ.
//!
//! İstekler arka plan iş parçacığında; kimlik bilgisi gönderilmez
//! (User-Agent "seismira").

use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Usgs,
    Emsc,
}

impl Source {
    pub const ALL: [Source; 2] = [Source::Usgs, Source::Emsc];
    pub fn label(self) -> &'static str {
        match self {
            Source::Usgs => "USGS",
            Source::Emsc => "EMSC",
        }
    }
    /// Atıf metni (çeviri anahtarı).
    pub fn credit_key(self) -> &'static str {
        match self {
            Source::Usgs => "real-credit-usgs",
            Source::Emsc => "real-credit-emsc",
        }
    }
}

/// Listedeki bir deprem.
#[derive(Clone, Debug, PartialEq)]
pub struct RealQuake {
    pub source: Source,
    pub id: String,
    /// UNIX zamanı (ms, UTC).
    pub time_ms: i64,
    pub lon: f64,
    pub lat: f64,
    pub depth_km: f64,
    pub mag: f64,
    pub mag_type: String,
    pub place: String,
    /// USGS ayrıntı GeoJSON adresi (moment tensörü için).
    pub detail: Option<String>,
}

impl RealQuake {
    /// Büyüklük türü moment büyüklüğü mü (Mw, Mww, Mwc, Mwr, Mwb…)?
    pub fn is_mw(&self) -> bool {
        self.mag_type.to_ascii_lowercase().starts_with("mw")
    }
}

/// Moment tensörü düğüm düzlemi (doğrultu, eğim, kayma açısı).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NodalPlane {
    pub strike: f64,
    pub dip: f64,
    pub rake: f64,
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(20)))
        .build()
        .into()
}

fn get(url: &str) -> Result<String, String> {
    agent()
        .get(url)
        .header("User-Agent", "seismira")
        .call()
        .map_err(|e| e.to_string())?
        .body_mut()
        .read_to_string()
        .map_err(|e| e.to_string())
}

/// Son 30 günün en az `min_mag` depremleri, yeniden eskiye.
pub fn fetch(source: Source, min_mag: f64) -> Result<Vec<RealQuake>, String> {
    let mut list = match source {
        Source::Usgs => parse_usgs(&get(
            "https://earthquake.usgs.gov/earthquakes/feed/v1.0/summary/4.5_month.geojson",
        )?)?,
        Source::Emsc => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs()) as i64;
            let start = iso_date(now - 30 * 86_400);
            parse_emsc(&get(&format!(
                "https://www.seismicportal.eu/fdsnws/event/1/query?format=json&starttime={start}&minmagnitude={min_mag}&limit=500&orderby=time"
            ))?)?
        }
    };
    list.retain(|q| q.mag >= min_mag);
    list.sort_by_key(|q| std::cmp::Reverse(q.time_ms));
    Ok(list)
}

/// USGS olay ayrıntısı: iki düğüm düzlemi ve (varsa) ShakeMap kırılma
/// modelinin kuadları.
#[derive(Clone, Debug, Default)]
pub struct Detail {
    pub planes: Vec<NodalPlane>,
    pub quads: Vec<Quad>,
}

/// ShakeMap `rupture.json` kuadı: üst ve alt kenar noktaları, derinlikler.
#[derive(Clone, Debug, PartialEq)]
pub struct Quad {
    pub top: Vec<(f64, f64)>,
    pub bottom: Vec<(f64, f64)>,
    pub ztop: f64,
    pub zbot: f64,
}

/// Ayrıntı + ShakeMap kırılma modeli (ikincisi yoksa ya da alınamazsa boş).
pub fn fetch_detail(detail_url: &str) -> Result<Detail, String> {
    let text = get(detail_url)?;
    let v: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let mut d = Detail {
        planes: nodal_planes(&v),
        quads: Vec::new(),
    };
    let rupture_url = v["properties"]["products"]["shakemap"]
        .as_array()
        .and_then(|a| {
            a.iter()
                .max_by_key(|m| m["preferredWeight"].as_i64().unwrap_or(0))
        })
        .and_then(|m| m["contents"]["download/rupture.json"]["url"].as_str())
        .map(str::to_string);
    if let Some(u) = rupture_url {
        match get(&u).and_then(|t| parse_rupture(&t)) {
            Ok(q) => d.quads = q,
            Err(e) => log::warn!("ShakeMap kırılma modeli alınamadı: {e}"),
        }
    }
    Ok(d)
}

/// Tercih edilen moment tensörünün iki düğüm düzlemi.
fn nodal_planes(v: &serde_json::Value) -> Vec<NodalPlane> {
    let Some(best) = v["properties"]["products"]["moment-tensor"]
        .as_array()
        .and_then(|a| {
            a.iter()
                .max_by_key(|m| m["preferredWeight"].as_i64().unwrap_or(0))
        })
    else {
        return Vec::new();
    };
    let p = &best["properties"];
    let num = |k: String| p[k.as_str()].as_str().and_then(|s| s.parse::<f64>().ok());
    (1..=2)
        .filter_map(|i| {
            Some(NodalPlane {
                strike: num(format!("nodal-plane-{i}-strike"))?,
                dip: num(format!("nodal-plane-{i}-dip"))?,
                rake: num(format!("nodal-plane-{i}-rake"))?,
            })
        })
        .collect()
}

/// `rupture.json` (GeoJSON MultiPolygon, köşeler boylam/enlem/derinlik):
/// her halka bir kuad; üst kenar en sığ, alt kenar en derin köşeler
/// (scripts/gen_scenarios.py `quads` ile aynı).
pub fn parse_rupture(text: &str) -> Result<Vec<Quad>, String> {
    let v: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for f in v["features"].as_array().into_iter().flatten() {
        let g = &f["geometry"];
        if g["type"] != "MultiPolygon" {
            continue;
        }
        for poly in g["coordinates"].as_array().into_iter().flatten() {
            for ring in poly.as_array().into_iter().flatten() {
                let pts: Vec<(f64, f64, f64)> = ring
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|p| Some((p[0].as_f64()?, p[1].as_f64()?, p[2].as_f64()?)))
                    .collect();
                // Kapalı halkanın son noktası ilkinin tekrarı.
                let pts = &pts[..pts.len().saturating_sub(1)];
                if pts.len() < 3 {
                    continue;
                }
                let ztop = pts.iter().map(|p| p.2).fold(f64::INFINITY, f64::min);
                let zbot = pts.iter().map(|p| p.2).fold(f64::NEG_INFINITY, f64::max);
                let pick = |z: f64| {
                    pts.iter()
                        .filter(|p| (p.2 - z).abs() < 1e-6)
                        .map(|p| (p.0, p.1))
                        .collect::<Vec<_>>()
                };
                out.push(Quad {
                    top: pick(ztop),
                    bottom: pick(zbot),
                    ztop,
                    zbot,
                });
            }
        }
    }
    Ok(out)
}

/// Kırılma modelinden kurulan fay.
#[derive(Clone, Debug, PartialEq)]
pub enum Geometry {
    /// Çok parçalı iz (≥ 2 parça).
    Trace {
        points: Vec<(f64, f64)>,
        dip: f64,
        width: f64,
        hypo_down: f64,
    },
    /// Tek düzlem.
    Plane {
        strike: f64,
        dip: f64,
        length: f64,
        width: f64,
        hypo_along: f64,
        hypo_down: f64,
    },
}

/// Kuadlardan fay geometrisi ve kullanılacak düğüm düzlemi (doğrultusu
/// kırılmaya en yakın olan). scripts/gen_scenarios.py `build` +
/// `chain_trace` kurallarının aynısı (hazır senaryolarla tutarlı):
/// Mww doğrultusuna 45°'ten uzak kuadlar atlanır; kuadlar eksen boyunca uç
/// uca (kollar atlanır, ≤ 10 km basamak, kapsama ≥ %90, ≥ 2 parça) → iz;
/// olmazsa tek düzlem (uzunluk eksen uzanımı, eğim tek kuadda kuadın
/// kendisinden, yoksa Mww). Kırılma modeli ya da düzlem yoksa `None`.
pub fn rupture_geometry(
    quads: &[Quad],
    planes: &[NodalPlane],
    epicenter: (f64, f64),
    depth: f64,
) -> Option<(Geometry, NodalPlane)> {
    use seismira_core::geo;
    let local = |p: (f64, f64)| {
        let (km, az) = geo::distance_azimuth(epicenter, p);
        let a = az.to_radians();
        (km * a.sin(), km * a.cos())
    };
    let axial = |a: f64, b: f64| {
        let d = (a - b).rem_euclid(180.0);
        d.min(180.0 - d)
    };
    let az =
        |a: (f64, f64), b: (f64, f64)| (b.0 - a.0).atan2(b.1 - a.1).to_degrees().rem_euclid(360.0);
    let quads: Vec<&Quad> = quads.iter().filter(|q| q.top.len() >= 2).collect();
    if quads.is_empty() || planes.is_empty() {
        return None;
    }
    let xy_all: Vec<(f64, f64)> = quads
        .iter()
        .flat_map(|q| q.top.iter().map(|&p| local(p)))
        .collect();
    let axis_all = principal_axis(&xy_all);
    let plane = *planes
        .iter()
        .min_by(|a, b| axial(a.strike, axis_all).total_cmp(&axial(b.strike, axis_all)))?;
    let keep: Vec<&Quad> = quads
        .into_iter()
        .filter(|q| {
            axial(
                az(local(q.top[0]), local(q.top[q.top.len() - 1])),
                plane.strike,
            ) <= 45.0
        })
        .collect();
    if keep.is_empty() {
        return None;
    }
    let xy: Vec<(f64, f64)> = keep
        .iter()
        .flat_map(|q| q.top.iter().map(|&p| local(p)))
        .collect();
    let mut strike = principal_axis(&xy);
    if ((strike - plane.strike + 180.0).rem_euclid(360.0) - 180.0).abs() > 90.0 {
        strike = (strike + 180.0).rem_euclid(360.0);
    }
    let s = (strike.to_radians().sin(), strike.to_radians().cos());
    let proj = |p: (f64, f64)| p.0 * s.0 + p.1 * s.1;
    let (lo, hi) = xy
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &p| {
            (a.min(proj(p)), b.max(proj(p)))
        });
    let length = hi - lo;
    if length < 1.0 {
        return None;
    }
    let ztop = keep.iter().map(|q| q.ztop).fold(f64::INFINITY, f64::min);
    let zbot = keep
        .iter()
        .map(|q| q.zbot)
        .fold(f64::NEG_INFINITY, f64::max);
    let down = ((depth - ztop) / (zbot - ztop).max(1e-6)).clamp(0.0, 1.0);
    if let Some(points) = chain_trace(&keep, &local, strike, length) {
        let dip = plane.dip.clamp(1.0, 90.0);
        return Some((
            Geometry::Trace {
                points,
                dip,
                width: (zbot - ztop) / dip.to_radians().sin(),
                hypo_down: down,
            },
            plane,
        ));
    }
    let mut along = -lo / length;
    let mut dip = plane.dip;
    if keep.len() == 1 && !keep[0].bottom.is_empty() {
        let mean = |v: &[(f64, f64)]| {
            let n = v.len() as f64;
            let (x, y) = v.iter().fold((0.0, 0.0), |(a, b), &p| {
                let q = local(p);
                (a + q.0, b + q.1)
            });
            (x / n, y / n)
        };
        let (tm, bm) = (mean(&keep[0].top), mean(&keep[0].bottom));
        let horiz = (bm.0 - tm.0).hypot(bm.1 - tm.1);
        if horiz > 0.5 {
            dip = (zbot - ztop).atan2(horiz).to_degrees();
            // Eğim yönü doğrultunun sağında (Aki & Richards).
            let cand = (az(tm, bm) - 90.0).rem_euclid(360.0);
            if ((cand - strike + 180.0).rem_euclid(360.0) - 180.0).abs() > 90.0 {
                strike = (strike + 180.0).rem_euclid(360.0);
                along = 1.0 - along;
            }
        }
    }
    let dip = dip.clamp(1.0, 90.0);
    Some((
        Geometry::Plane {
            strike,
            dip,
            length,
            width: (zbot - ztop) / dip.to_radians().sin(),
            hypo_along: along.clamp(0.0, 1.0),
            hypo_down: down,
        },
        plane,
    ))
}

/// Noktaların ana ekseninin azimutu (0–180).
fn principal_axis(xy: &[(f64, f64)]) -> f64 {
    let n = xy.len() as f64;
    let mx = xy.iter().map(|p| p.0).sum::<f64>() / n;
    let my = xy.iter().map(|p| p.1).sum::<f64>() / n;
    let (mut sxx, mut syy, mut sxy) = (0.0, 0.0, 0.0);
    for p in xy {
        sxx += (p.0 - mx).powi(2);
        syy += (p.1 - my).powi(2);
        sxy += (p.0 - mx) * (p.1 - my);
    }
    let theta = 0.5 * (2.0 * sxy).atan2(sxx - syy);
    (90.0 - theta.to_degrees()).rem_euclid(180.0)
}

/// gen_scenarios.py `chain_trace`: kuad üst kenarlarını eksen boyunca uç
/// uca bağlar; koşullar sağlanmazsa `None`.
/// Zincirleme öğesi: eksen üzerindeki başlangıç ve bitiş, noktalar
/// (boylam, enlem) ve yerel km karşılıkları.
type ChainItem = (f64, f64, Vec<(f64, f64)>, Vec<(f64, f64)>);

fn chain_trace(
    keep: &[&Quad],
    local: &impl Fn((f64, f64)) -> (f64, f64),
    strike: f64,
    length: f64,
) -> Option<Vec<(f64, f64)>> {
    const GAP_KM: f64 = 10.0;
    let s = (strike.to_radians().sin(), strike.to_radians().cos());
    let proj = |p: (f64, f64)| p.0 * s.0 + p.1 * s.1;
    let mut items: Vec<ChainItem> = keep
        .iter()
        .map(|q| {
            let mut pts = q.top.clone();
            let mut xy: Vec<(f64, f64)> = pts.iter().map(|&p| local(p)).collect();
            if proj(xy[xy.len() - 1]) < proj(xy[0]) {
                pts.reverse();
                xy.reverse();
            }
            (proj(xy[0]), proj(xy[xy.len() - 1]), pts, xy)
        })
        .collect();
    items.sort_by(|a, b| (b.1 - b.0).total_cmp(&(a.1 - a.0)));
    let mut accepted: Vec<ChainItem> = Vec::new();
    for it in items {
        let own = it.1 - it.0;
        let overlaps = accepted
            .iter()
            .any(|a| it.1.min(a.1) - it.0.max(a.0) > (0.5 * own).min(1.0));
        if !overlaps {
            accepted.push(it);
        }
    }
    accepted.sort_by(|a, b| a.0.total_cmp(&b.0));
    let (mut pts, mut xys) = (accepted[0].2.clone(), accepted[0].3.clone());
    for w in accepted.windows(2) {
        let (a, b) = (&w[0], &w[1]);
        if b.0 < a.1 - 1.0 {
            return None;
        }
        let la = a.3[a.3.len() - 1];
        if (b.3[0].0 - la.0).hypot(b.3[0].1 - la.1) > GAP_KM {
            return None;
        }
        pts.extend_from_slice(&b.2);
        xys.extend_from_slice(&b.3);
    }
    let (mut out, mut oxy) = (vec![pts[0]], vec![xys[0]]);
    for (p, xy) in pts.iter().zip(&xys).skip(1) {
        let l = oxy[oxy.len() - 1];
        if (xy.0 - l.0).hypot(xy.1 - l.1) >= 1.0 {
            out.push(*p);
            oxy.push(*xy);
        }
    }
    if out.len() > 1 {
        let last = out.len() - 1;
        out[last] = pts[pts.len() - 1];
        oxy[last] = xys[xys.len() - 1];
    }
    let extent = proj(oxy[oxy.len() - 1]) - proj(oxy[0]);
    if extent < 0.9 * length || out.len() < 3 || out.len() > seismira_core::quake::MAX_TRACE_POINTS
    {
        return None;
    }
    Some(out)
}

pub fn parse_usgs(text: &str) -> Result<Vec<RealQuake>, String> {
    let v: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let feats = v["features"].as_array().ok_or("features yok")?;
    Ok(feats
        .iter()
        .filter_map(|f| {
            let p = &f["properties"];
            let c = f["geometry"]["coordinates"].as_array()?;
            if p["type"].as_str().is_some_and(|t| t != "earthquake") {
                return None;
            }
            Some(RealQuake {
                source: Source::Usgs,
                id: f["id"].as_str()?.to_string(),
                time_ms: p["time"].as_i64()?,
                lon: c.first()?.as_f64()?,
                lat: c.get(1)?.as_f64()?,
                depth_km: c.get(2)?.as_f64()?,
                mag: p["mag"].as_f64()?,
                mag_type: p["magType"].as_str().unwrap_or("").to_string(),
                place: p["place"].as_str().unwrap_or("").to_string(),
                detail: p["detail"].as_str().map(str::to_string),
            })
        })
        .collect())
}

/// Ayrıntı metninden tercih edilen moment tensörünün iki düğüm düzlemi
/// (testler için; uygulama [`fetch_detail`] kullanır).
#[cfg(test)]
pub fn parse_detail_planes(text: &str) -> Result<Vec<NodalPlane>, String> {
    let v: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    Ok(nodal_planes(&v))
}

pub fn parse_emsc(text: &str) -> Result<Vec<RealQuake>, String> {
    let v: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let feats = v["features"].as_array().ok_or("features yok")?;
    Ok(feats
        .iter()
        .filter_map(|f| {
            let p = &f["properties"];
            if p["evtype"].as_str().is_some_and(|t| !t.starts_with('k')) {
                return None; // yalnızca doğal deprem (ke / kn…)
            }
            Some(RealQuake {
                source: Source::Emsc,
                id: p["unid"].as_str()?.to_string(),
                time_ms: parse_iso_ms(p["time"].as_str()?)?,
                lon: p["lon"].as_f64()?,
                lat: p["lat"].as_f64()?,
                depth_km: p["depth"].as_f64()?,
                mag: p["mag"].as_f64()?,
                mag_type: p["magtype"].as_str().unwrap_or("").to_string(),
                place: p["flynn_region"].as_str().unwrap_or("").to_string(),
                detail: None,
            })
        })
        .collect())
}

/// "YYYY-MM-DD" (UTC) → gün sayısından.
fn iso_date(secs: i64) -> String {
    let (y, m, d) = civil(secs.div_euclid(86_400));
    format!("{y:04}-{m:02}-{d:02}")
}

/// Howard Hinnant "civil_from_days".
fn civil(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

/// Howard Hinnant "days_from_civil".
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// "2026-11-26T02:09:17.2Z" → UNIX ms (UTC; Z ya da saat dilimsiz).
pub fn parse_iso_ms(s: &str) -> Option<i64> {
    let s = s.trim_end_matches('Z');
    let (date, time) = s.split_once('T')?;
    let mut dp = date.split('-').map(|x| x.parse::<i64>().ok());
    let (y, m, d) = (dp.next()??, dp.next()??, dp.next()??);
    let mut tp = time.split(':');
    let h: i64 = tp.next()?.parse().ok()?;
    let mi: i64 = tp.next()?.parse().ok()?;
    let sec: f64 = tp.next().unwrap_or("0").parse().ok()?;
    Some(
        (days_from_civil(y, m, d) * 86_400 + h * 3600 + mi * 60) * 1000
            + (sec * 1000.0).round() as i64,
    )
}

/// UNIX ms → "YYYY-AA-GG SS:DD UTC".
pub fn format_utc(ms: i64) -> String {
    let secs = ms.div_euclid(1000);
    let (y, m, d) = civil(secs.div_euclid(86_400));
    let r = secs.rem_euclid(86_400);
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02} UTC",
        r / 3600,
        r % 3600 / 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Canlı ağ denemesi (CI'da çalışmaz): `SEISMIRA_NET_TEST=1`.
    #[test]
    fn canli_ag() {
        if std::env::var("SEISMIRA_NET_TEST").as_deref() != Ok("1") {
            return;
        }
        for src in Source::ALL {
            let l = fetch(src, 5.0).unwrap();
            eprintln!(
                "{}: {} deprem; ilk {:?}",
                src.label(),
                l.len(),
                l.first().map(|q| (&q.place, q.mag, &q.mag_type))
            );
            assert!(!l.is_empty());
            assert!(l.windows(2).all(|w| w[0].time_ms >= w[1].time_ms));
        }
        // 2023 Pazarcık (us6000jllz): moment tensörü + ShakeMap kırılması →
        // 15 parçalı iz, 227,6° düzlemi.
        let d = fetch_detail(
            "https://earthquake.usgs.gov/earthquakes/feed/v1.0/detail/us6000jllz.geojson",
        )
        .unwrap();
        let (g, p) = rupture_geometry(&d.quads, &d.planes, (37.0143, 37.2256), 10.0).unwrap();
        eprintln!("Pazarcık: düzlem {p:?}, {} kuad", d.quads.len());
        assert!(matches!(g, Geometry::Trace { ref points, .. } if points.len() == 16));
        assert!((p.strike - 227.61).abs() < 0.5);
    }

    /// Yereldeki gerçek ShakeMap dosyası (CI'da yok, varsa): Kahramanmaraş
    /// izi hazır senaryodakiyle (gen_scenarios.py) aynı nokta sayısı, düzlem
    /// 2 (227,6°) seçilir.
    #[test]
    fn kahramanmaras_kirilmasi() {
        let p = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/raw/validation/kahramanmaras2023/rupture.json"
        );
        let Ok(text) = std::fs::read_to_string(p) else {
            return;
        };
        let quads = parse_rupture(&text).unwrap();
        let planes = [
            NodalPlane {
                strike: 317.63,
                dip: 88.71,
                rake: -179.18,
            },
            NodalPlane {
                strike: 227.61,
                dip: 89.18,
                rake: -1.29,
            },
        ];
        let (g, plane) = rupture_geometry(&quads, &planes, (37.0143, 37.2256), 10.0).unwrap();
        assert_eq!(plane.strike, 227.61);
        let Geometry::Trace { points, width, .. } = g else {
            panic!("iz bekleniyordu: {g:?}");
        };
        assert_eq!(points.len(), 16);
        assert!((width - 15.0).abs() < 0.1, "{width}");
    }

    /// Yapay iki kuad (uç uca, doğu-batı) → iz; tek kuad (eğimli) → düzlem.
    #[test]
    fn yapay_kirilma() {
        let q = |x0: f64, x1: f64| Quad {
            top: vec![(x0, 40.0), ((x0 + x1) / 2.0, 40.0), (x1, 40.0)],
            bottom: vec![(x0, 39.9), (x1, 39.9)],
            ztop: 0.0,
            zbot: 15.0,
        };
        let planes = [NodalPlane {
            strike: 90.0,
            dip: 80.0,
            rake: 0.0,
        }];
        let (g, _) =
            rupture_geometry(&[q(30.0, 30.5), q(30.5, 31.0)], &planes, (30.5, 40.0), 8.0).unwrap();
        assert!(
            matches!(g, Geometry::Trace { ref points, .. } if points.len() >= 3),
            "{g:?}"
        );
        let one = Quad {
            top: vec![(30.0, 40.0), (30.4, 40.0)],
            bottom: vec![(30.0, 39.9), (30.4, 39.9)],
            ztop: 1.0,
            zbot: 16.0,
        };
        let (g, _) = rupture_geometry(&[one], &planes, (30.2, 39.95), 8.0).unwrap();
        let Geometry::Plane {
            strike,
            dip,
            length,
            ..
        } = g
        else {
            panic!("düzlem bekleniyordu");
        };
        // Alt kenar güneyde: eğim yönü güney → doğrultu doğu (90°; Aki &
        // Richards: doğrultuya bakınca fay sağa, yani güneye eğimli), eğim
        // atan(15 / 11,1 km) ≈ 53,5°.
        assert!((strike - 90.0).abs() < 1.0, "{strike}");
        assert!((dip - 53.5).abs() < 1.0, "{dip}");
        assert!((length - 34.1).abs() < 0.5, "{length}");
        assert!(rupture_geometry(&[], &planes, (30.0, 40.0), 10.0).is_none());
    }

    #[test]
    fn tarih() {
        let ms = parse_iso_ms("2026-11-26T02:09:17.2Z").unwrap();
        assert_eq!(format_utc(ms), "2026-11-26 02:09 UTC");
        assert_eq!(parse_iso_ms("1970-01-01T00:00:00").unwrap(), 0);
        assert_eq!(iso_date(86_400 * 365), "1971-01-01");
    }

    /// Gerçek yanıt biçimleri (2026-11-26'da alınan örneklerden kısaltılmış).
    #[test]
    fn ayristirma() {
        let usgs = r#"{"type":"FeatureCollection","features":[{"type":"Feature","properties":{"mag":4.6,"place":"97 km SW of Puerto Madero, Mexico","time":1795663828511,"type":"earthquake","magType":"mb","detail":"https://earthquake.usgs.gov/earthquakes/feed/v1.0/detail/us6000udki.geojson"},"geometry":{"type":"Point","coordinates":[-93.126,14.164,10]},"id":"us6000udki"}]}"#;
        let q = &parse_usgs(usgs).unwrap()[0];
        assert_eq!(
            (q.lon, q.lat, q.depth_km, q.mag),
            (-93.126, 14.164, 10.0, 4.6)
        );
        assert!(!q.is_mw());
        let emsc = r#"{"type":"FeatureCollection","features":[{"type":"Feature","geometry":{"type":"Point","coordinates":[178.3265,-35.9894,-5.9]},"id":"20261126_0000050","properties":{"source_id":"2095317","time":"2026-11-26T02:09:17.2Z","flynn_region":"OFF E. COAST OF N. ISLAND, N.Z.","lat":-35.9894,"lon":178.3265,"depth":5.9,"evtype":"ke","mag":5.0,"magtype":"mw","unid":"20261126_0000050"}}]}"#;
        let e = &parse_emsc(emsc).unwrap()[0];
        assert_eq!((e.depth_km, e.mag), (5.9, 5.0));
        assert!(e.is_mw());
        let detail = r#"{"properties":{"products":{"moment-tensor":[{"preferredWeight":1,"properties":{"nodal-plane-1-strike":"10","nodal-plane-1-dip":"20","nodal-plane-1-rake":"90"}},{"preferredWeight":5,"properties":{"nodal-plane-1-strike":"224.5","nodal-plane-1-dip":"89","nodal-plane-1-rake":"-1"}}]}}}"#;
        assert_eq!(
            parse_detail_planes(detail).unwrap()[0],
            NodalPlane {
                strike: 224.5,
                dip: 89.0,
                rake: -1.0
            }
        );
    }
}
