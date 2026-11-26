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

/// USGS ayrıntısından tercih edilen moment tensörünün 1. düğüm düzlemi.
pub fn fetch_nodal_plane(detail_url: &str) -> Result<Option<NodalPlane>, String> {
    parse_usgs_detail(&get(detail_url)?)
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

pub fn parse_usgs_detail(text: &str) -> Result<Option<NodalPlane>, String> {
    let v: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let Some(mts) = v["properties"]["products"]["moment-tensor"].as_array() else {
        return Ok(None);
    };
    // Tercih ağırlığı en yüksek ürün.
    let best = mts
        .iter()
        .max_by_key(|m| m["preferredWeight"].as_i64().unwrap_or(0));
    Ok(best.and_then(|m| {
        let p = &m["properties"];
        let num = |k: &str| p[k].as_str().and_then(|s| s.parse::<f64>().ok());
        Some(NodalPlane {
            strike: num("nodal-plane-1-strike")?,
            dip: num("nodal-plane-1-dip")?,
            rake: num("nodal-plane-1-rake")?,
        })
    }))
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
        // 2023 Pazarcık (us6000jllz) moment tensörü.
        let np = fetch_nodal_plane(
            "https://earthquake.usgs.gov/earthquakes/feed/v1.0/detail/us6000jllz.geojson",
        )
        .unwrap();
        eprintln!("Pazarcık düğüm düzlemi: {np:?}");
        assert!(np.is_some());
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
            parse_usgs_detail(detail).unwrap(),
            Some(NodalPlane {
                strike: 224.5,
                dip: 89.0,
                rake: -1.0
            })
        );
    }
}
