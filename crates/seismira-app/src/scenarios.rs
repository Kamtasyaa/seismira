//! Hazır senaryolar (Faz 10; CLAUDE.md Bölüm 14): doğrulama depremleri,
//! `data/scenarios/scenarios.json` (scripts/gen_scenarios.py; USGS ComCat +
//! ShakeMap kırılma modelinden; çok parçalı modeller `trace` olarak, diğerleri
//! tek düzlemli eşdeğer, kaynak notu dosyada).

use seismira_core::quake::{FaultType, Quake, Setting, Trace};

/// Bir hazır senaryo. Adı çeviri anahtarından: `scenario-<id>`.
#[derive(Clone, Debug)]
pub struct Preset {
    pub id: String,
    pub quake: Quake,
}

impl Preset {
    pub fn name_key(&self) -> String {
        format!("scenario-{}", self.id)
    }
}

pub fn load(path: &std::path::Path) -> Result<Vec<Preset>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    parse(&text).map_err(|e| format!("{}: {e}", path.display()))
}

fn parse(text: &str) -> Result<Vec<Preset>, String> {
    let v: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    if v["version"] != 1 {
        return Err(format!("desteklenmeyen sürüm {}", v["version"]));
    }
    let list = v["scenarios"].as_array().ok_or("'scenarios' yok")?;
    list.iter()
        .map(|s| {
            let num = |k: &str| s[k].as_f64().ok_or_else(|| format!("'{k}' yok"));
            let id = s["id"].as_str().ok_or("'id' yok")?.to_string();
            let setting = match s["setting"].as_str() {
                Some("crust") => Setting::ShallowCrust,
                Some("interface") => Setting::Interface,
                Some("intraslab") => Setting::Intraslab,
                other => return Err(format!("{id}: ortam {other:?}")),
            };
            let mut q = Quake::new((num("lon")?, num("lat")?));
            q.mw = num("mw")?;
            q.depth_km = num("depth_km")?;
            q.setting = setting;
            q.strike = num("strike")?;
            q.dip = num("dip")?;
            q.rake = num("rake")?;
            q.fault = FaultType::from_rake(q.rake);
            q.hypo_along = num("hypo_along")?;
            q.hypo_down = num("hypo_down")?;
            q.rupture_km = Some([num("length_km")?, num("width_km")?]);
            // Çok parçalı iz (varsa): uzunluk ve doğrultu izden; genişlik
            // `width_km`.
            if let Some(tr) = s["trace"].as_array() {
                let pts = tr
                    .iter()
                    .map(|p| Some((p[0].as_f64()?, p[1].as_f64()?)))
                    .collect::<Option<Vec<_>>>()
                    .ok_or_else(|| format!("{id}: iz noktası okunamadı"))?;
                q.trace = Some(Trace::new(&pts).ok_or_else(|| format!("{id}: iz nokta sayısı"))?);
            }
            Ok(Preset { id, quake: q })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Depodaki dosya okunur, her senaryo geçerlilik denetiminden engelsiz
    /// geçer ve çevirisi iki dilde de vardır.
    #[test]
    fn depodaki_senaryolar_gecerli() {
        let path = std::path::Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/scenarios/scenarios.json"
        ));
        let presets = load(path).unwrap();
        assert_eq!(presets.len(), 8);
        // Çok parçalı iz: Kahramanmaraş (USGS Wald sonlu fay izi, 15 parça).
        let km = presets
            .iter()
            .find(|p| p.id == "kahramanmaras2023")
            .unwrap();
        let r = seismira_core::rupture::Rupture::from_quake(&km.quake);
        assert!(r.segments.len() >= 10, "{}", r.segments.len());
        assert!((270.0..300.0).contains(&r.length), "{}", r.length);
        for p in &presets {
            let issues = seismira_core::quake::validate(&p.quake);
            assert!(issues.iter().all(|i| !i.blocks()), "{}: {issues:?}", p.id);
            for lang in ["tr", "en"] {
                let ftl = std::fs::read_to_string(format!(
                    "{}/locales/{lang}/main.ftl",
                    env!("CARGO_MANIFEST_DIR")
                ))
                .unwrap();
                assert!(
                    ftl.lines()
                        .any(|l| l.starts_with(&format!("{} =", p.name_key()))),
                    "{lang}: {} çevirisi yok",
                    p.name_key()
                );
            }
        }
    }
}
