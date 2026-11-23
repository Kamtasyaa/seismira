//! "Veri Kaynakları ve Lisanslar" sayfası (CLAUDE.md Bölüm 5, 12;
//! docs/arayuz.md): `data/SOURCES.toml`'dan üretilir. Yalnızca doğrulamada
//! kullanılan kayıtlar (`validation_only`) gösterilmez.

use eframe::egui::{self, RichText};

use crate::i18n::Lang;

/// Bir veri seti kaydı (gösterilen alanlar).
#[derive(Clone, Debug)]
pub struct Source {
    pub id: String,
    pub name: String,
    pub url: String,
    pub license: String,
    pub attribution: String,
    /// Türetilmiş ("değiştirildi" notu).
    pub modified: bool,
    /// `false`: dağıtılan pakette yok (yalnızca yerel derleme; izin bekleniyor).
    pub distributed: bool,
}

pub fn load(path: &std::path::Path) -> Result<Vec<Source>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    parse(&text).map_err(|e| format!("{}: {e}", path.display()))
}

fn parse(text: &str) -> Result<Vec<Source>, String> {
    let table: toml::Table = text.parse().map_err(|e: toml::de::Error| e.to_string())?;
    let list = table
        .get("dataset")
        .and_then(toml::Value::as_array)
        .ok_or("'dataset' listesi yok")?;
    let mut out = Vec::new();
    for d in list {
        let s = |k: &str| -> Result<String, String> {
            d.get(k)
                .and_then(toml::Value::as_str)
                .map(str::to_owned)
                .ok_or_else(|| format!("'{k}' yok: {:?}", d.get("id")))
        };
        let b = |k: &str| d.get(k).and_then(toml::Value::as_bool);
        if b("validation_only") == Some(true) {
            continue;
        }
        out.push(Source {
            id: s("id")?,
            name: s("name")?,
            url: s("url")?,
            license: s("license")?,
            attribution: s("attribution")?,
            modified: b("modified").ok_or("'modified' yok")?,
            distributed: b("distributed").unwrap_or(true),
        });
    }
    Ok(out)
}

/// Pencere içeriği.
pub fn ui(ui: &mut egui::Ui, lang: Lang, sources: &Result<Vec<Source>, String>) {
    ui.label(lang.t("sources-intro"));
    ui.add_space(6.0);
    let list = match sources {
        Ok(l) => l,
        Err(e) => {
            ui.colored_label(ui.visuals().error_fg_color, lang.t("sources-missing"));
            ui.label(RichText::new(e).small().weak());
            return;
        }
    };
    egui::ScrollArea::vertical().show(ui, |ui| {
        for s in list {
            ui.push_id(&s.id, |ui| entry(ui, lang, s));
        }
    });
}

fn entry(ui: &mut egui::Ui, lang: Lang, s: &Source) {
    {
        {
            ui.separator();
            ui.label(RichText::new(&s.name).strong());
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(lang.t("sources-license")).weak());
                ui.label(&s.license);
                if s.modified {
                    ui.label(RichText::new(lang.t("sources-modified")).weak());
                }
                if !s.distributed {
                    ui.label(
                        RichText::new(lang.t("sources-not-distributed"))
                            .color(ui.visuals().warn_fg_color),
                    );
                }
            });
            ui.add(egui::Label::new(RichText::new(&s.attribution).small()).selectable(true));
            ui.hyperlink(&s.url);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Depodaki SOURCES.toml okunur; kimlikler tekil, doğrulama kayıtları
    /// süzülür, URL'ler http(s).
    #[test]
    fn depodaki_kaynaklar_okunur() {
        let path = std::path::Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/SOURCES.toml"
        ));
        let list = load(path).unwrap();
        assert!(list.len() >= 20, "{}", list.len());
        let mut ids: Vec<&str> = list.iter().map(|s| s.id.as_str()).collect();
        ids.sort_unstable();
        let n = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), n, "yinelenen kimlik");
        assert!(!list.iter().any(|s| s.id.starts_with("usgs-shakemap-")));
        assert!(list.iter().all(|s| s.url.starts_with("http")));
        assert!(list.iter().any(|s| s.id == "noto-fonts"));
    }
}
