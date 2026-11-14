//! Çeviri altyapısı (Fluent, `fluent-templates`).
//!
//! Arayüz metni koda gömülmez: her metin `locales/<dil>/main.ftl` içinde bir
//! anahtarla tanımlanır ve [`Lang::t`] ile okunur. Dosyalar derleme anında
//! programa gömülür.

use fluent_templates::{LanguageIdentifier, Loader, langid, static_loader};

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn keys(lang: &str) -> BTreeSet<String> {
        let path = format!("{}/locales/{lang}/main.ftl", env!("CARGO_MANIFEST_DIR"));
        std::fs::read_to_string(&path)
            .unwrap()
            .lines()
            .filter_map(|l| {
                let (k, _) = l.split_once(" =")?;
                (!k.starts_with(' ') && !k.starts_with('#')).then(|| k.trim().to_string())
            })
            .collect()
    }

    #[test]
    fn tum_diller_ayni_anahtarlara_sahip() {
        let tr = keys("tr");
        let en = keys("en");
        assert!(!tr.is_empty());
        assert_eq!(tr, en, "tr ve en çeviri anahtarları farklı");
    }

    #[test]
    fn her_anahtar_cozulur() {
        for lang in Lang::ALL {
            for key in keys(lang.code()) {
                let text = LOCALES.try_lookup(&lang.id(), &key);
                assert!(text.is_some(), "{lang:?}: {key} çözülemedi");
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
    fn turkce_metin() {
        assert_eq!(Lang::Tr.t("button-start"), "Başlat");
        assert_eq!(Lang::En.t("button-start"), "Start");
    }
}
