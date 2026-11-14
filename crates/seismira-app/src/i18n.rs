//! Çeviri altyapısı (Fluent, `fluent-templates`).
//!
//! Arayüz metni koda gömülmez: her metin `locales/<dil>/main.ftl` içinde bir
//! anahtarla tanımlanır ve [`Lang::t`] ile okunur. Dosyalar derleme anında
//! programa gömülür.

use fluent_bundle::FluentValue;
use fluent_templates::{LanguageIdentifier, Loader, langid, static_loader};
use std::borrow::Cow;
use std::collections::HashMap;

static_loader! {
    static LOCALES = {
        locales: "./locales",
        fallback_language: "en",
        // Fluent, değişkenlerin çevresine Unicode yön yalıtım karakterleri
        // (FSI/PDI) ekler. egui 0.37 bunları işlemediği için kapalı.
        // egui bidi desteği gelince yeniden açılmalı (docs/arayuz.md).
        customise: |bundle| bundle.set_use_isolating(false),
    };
}

/// Arayüz dili.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    Tr,
    En,
}

impl Lang {
    pub const ALL: [Lang; 2] = [Lang::Tr, Lang::En];

    /// BCP 47 kodu ("tr", "en").
    pub fn code(self) -> &'static str {
        match self {
            Lang::Tr => "tr",
            Lang::En => "en",
        }
    }

    /// Dilin kendi dilindeki adı. Dil seçicide her dil kendi adıyla
    /// gösterilir; bu yüzden çeviri dosyasında değil, burada durur.
    pub fn endonym(self) -> &'static str {
        match self {
            Lang::Tr => "Türkçe",
            Lang::En => "English",
        }
    }

    pub fn from_code(code: &str) -> Option<Lang> {
        let primary = code.split(['-', '_']).next()?.to_ascii_lowercase();
        Lang::ALL.into_iter().find(|l| l.code() == primary)
    }

    /// İşletim sisteminin dil ayarından; desteklenmiyorsa İngilizce.
    pub fn from_system() -> Lang {
        sys_locale::get_locale()
            .and_then(|l| Lang::from_code(&l))
            .unwrap_or(Lang::En)
    }

    fn id(self) -> LanguageIdentifier {
        match self {
            Lang::Tr => langid!("tr"),
            Lang::En => langid!("en"),
        }
    }

    /// Çeviri anahtarının metni. Anahtar yoksa İngilizceye, o da yoksa
    /// anahtarın kendisine düşer (eksik çeviri ekranda göze çarpsın diye).
    pub fn t(self, key: &str) -> String {
        LOCALES
            .try_lookup(&self.id(), key)
            .unwrap_or_else(|| format!("⟨{key}⟩"))
    }
}

impl Lang {
    /// Değişkenli çeviri: `{ $ad }` yerlerine verilen metinler konur.
    /// Sayılar önceden [`Lang::num`] ile biçimlenip metin olarak verilir.
    pub fn t_args(self, key: &str, args: &[(&'static str, String)]) -> String {
        let map: HashMap<Cow<'static, str>, FluentValue> = args
            .iter()
            .map(|(k, v)| (Cow::Borrowed(*k), FluentValue::from(v.clone())))
            .collect();
        LOCALES
            .try_lookup_with_args(&self.id(), key, &map)
            .unwrap_or_else(|| format!("⟨{key}⟩"))
    }

    /// Ondalık sayı, dilin ondalık ayracıyla ("41,01" / "41.01").
    pub fn num(self, v: f64, decimals: usize) -> String {
        let s = format!("{v:.decimals$}");
        match self.t("decimal-separator").as_str() {
            "." => s,
            sep => s.replacen('.', sep, 1),
        }
    }

    /// Enlem/boylam: "41,01° K, 28,97° D".
    pub fn lonlat(self, lon: f64, lat: f64) -> String {
        let ns = self.t(if lat >= 0.0 { "hemi-n" } else { "hemi-s" });
        let ew = self.t(if lon >= 0.0 { "hemi-e" } else { "hemi-w" });
        format!(
            "{}° {ns}, {}° {ew}",
            self.num(lat.abs(), 2),
            self.num(lon.abs(), 2)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};

    /// Anahtar → mesajdaki değişken adları (`{ $ad }`).
    fn messages(lang: &str) -> BTreeMap<String, BTreeSet<String>> {
        let path = format!("{}/locales/{lang}/main.ftl", env!("CARGO_MANIFEST_DIR"));
        std::fs::read_to_string(&path)
            .unwrap()
            .lines()
            .filter_map(|l| {
                let (k, v) = l.split_once(" =")?;
                if k.starts_with(' ') || k.starts_with('#') {
                    return None;
                }
                let vars = v
                    .split("{ $")
                    .skip(1)
                    .filter_map(|r| r.split_once(" }").map(|(n, _)| n.to_string()))
                    .collect();
                Some((k.trim().to_string(), vars))
            })
            .collect()
    }

    #[test]
    fn tum_diller_ayni_anahtar_ve_degiskenlere_sahip() {
        let tr = messages("tr");
        let en = messages("en");
        assert!(!tr.is_empty());
        assert_eq!(
            tr, en,
            "tr ve en çeviri anahtarları ya da değişkenleri farklı"
        );
    }

    #[test]
    fn her_anahtar_cozulur() {
        for lang in Lang::ALL {
            for (key, vars) in messages(lang.code()) {
                let args: Vec<(&'static str, String)> = vars
                    .iter()
                    .map(|v| (&*v.clone().leak(), "X".to_string()))
                    .collect();
                let text = lang.t_args(&key, &args);
                assert!(!text.starts_with('⟨'), "{lang:?}: {key} çözülemedi");
                assert!(
                    !text.contains("{$"),
                    "{lang:?}: {key} değişken eksik: {text}"
                );
            }
        }
    }

    #[test]
    fn dil_kodu_ayristirma() {
        assert_eq!(Lang::from_code("tr-TR"), Some(Lang::Tr));
        assert_eq!(Lang::from_code("en_US.UTF-8"), Some(Lang::En));
        assert_eq!(Lang::from_code("de-DE"), None);
    }

    #[test]
    fn koordinat_bicimi() {
        assert_eq!(Lang::Tr.lonlat(28.974, 41.012), "41,01° K, 28,97° D");
        assert_eq!(Lang::En.lonlat(-70.6, -33.45), "33.45° S, 70.60° W");
    }

    #[test]
    fn degiskenli_metin() {
        let s = Lang::Tr.t_args(
            "issue-mw",
            &[("min", Lang::Tr.num(4.0, 1)), ("max", Lang::Tr.num(8.5, 1))],
        );
        assert!(s.contains("4,0") && s.contains("8,5"), "{s}");
    }

    #[test]
    fn turkce_metin() {
        assert_eq!(Lang::Tr.t("button-start"), "Başlat");
        assert_eq!(Lang::En.t("button-start"), "Start");
    }
}
