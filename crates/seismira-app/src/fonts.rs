//! Yazı tipi zinciri.
//!
//! - Yalnızca uygulamayla gelen Noto yazı tipleri kullanılır; sistem yazı
//!   tipleri KULLANILMAZ (üç platformda birebir aynı görünüm, CLAUDE.md
//!   Bölüm 3).
//! - Noto Sans (Latin, Kiril, Yunan) programa gömülüdür; uygulama her
//!   durumda metin çizebilir.
//! - Diğer yazı sistemlerinin yazı tipleri `assets/fonts/` altından,
//!   o yazıdan ilk karakter ekrana geldiğinde yüklenir (egui `FontProvider`).
//! - CJK sırası arayüz diline göre belirlenir: aynı Han karakteri Japonca,
//!   Çince ve Korece'de farklı glifle çizilir (Bölüm 3.1).

use eframe::egui::{
    self, FallbackRequest, FontData, FontDefinitions, FontFamily, FontInsert, FontPriority,
    FontProvider, InsertFontFamily,
};
use skrifa::{FontRef, MetadataProvider as _};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use std::sync::{Arc, Mutex};

use crate::i18n::Lang;

static NOTO_SANS: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/fonts/NotoSans-Regular.ttf"
));

/// CJK dışındaki yazı tipleri, yedek sırasıyla. Yazı sistemleri birbiriyle
/// çakışmadığı için sıraları önemsizdir; sembol ve matematik yazı tipleri
/// sonda durur ki bir yazı sisteminin kendi glifi öncelikli olsun.
const SCRIPT_FONTS: &[&str] = &[
    "NotoSansArabic-Regular.ttf",
    "NotoSansHebrew-Regular.ttf",
    "NotoSansDevanagari-Regular.ttf",
    "NotoSansBengali-Regular.ttf",
    "NotoSansGurmukhi-Regular.ttf",
    "NotoSansGujarati-Regular.ttf",
    "NotoSansOriya-Regular.ttf",
    "NotoSansTamil-Regular.ttf",
    "NotoSansTelugu-Regular.ttf",
    "NotoSansKannada-Regular.ttf",
    "NotoSansMalayalam-Regular.ttf",
    "NotoSansSinhala-Regular.ttf",
    "NotoSansThai-Regular.ttf",
    "NotoSansLao-Regular.ttf",
    "NotoSansKhmer-Regular.ttf",
    "NotoSansMyanmar-Regular.ttf",
    "NotoSansArmenian-Regular.ttf",
    "NotoSansGeorgian-Regular.ttf",
    "NotoSansEthiopic-Regular.ttf",
    "NotoSansThaana-Regular.ttf",
    "NotoSansSyriac-Regular.ttf",
    "NotoSansMongolian-Regular.ttf",
    "NotoSansTifinagh-Regular.ttf",
    "NotoSerifTibetan-Regular.ttf",
    "NotoSansSymbols-Regular.ttf",
    "NotoSansSymbols2-Regular.ttf",
    "NotoSansMath-Regular.ttf",
];

/// CJK yazı tiplerinin arayüz diline göre sırası.
fn cjk_order(lang: Lang) -> [&'static str; 4] {
    // Japonca, Çince ve Korece arayüz dilleri eklendiğinde her biri kendi
    // bölge yazı tipini başa alacak. Şimdilik (TR/EN) Japonca biçimler
    // varsayılandır: JMA Shindo ölçeği ve Japonya senaryoları öne çıkıyor.
    match lang {
        Lang::Tr | Lang::En => [
            "cjk/NotoSansJP-Regular.otf",
            "cjk/NotoSansKR-Regular.otf",
            "cjk/NotoSansSC-Regular.otf",
            "cjk/NotoSansTC-Regular.otf",
        ],
    }
}

/// Yazı tiplerini kurar. Dil değişince yeniden çağrılır (CJK sırası için).
pub fn install(ctx: &egui::Context, assets: &Path, lang: Lang) {
    let mut defs = FontDefinitions::empty();
    defs.font_data.insert(
        "NotoSans".to_owned(),
        Arc::new(FontData::from_static(NOTO_SANS)),
    );
    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        defs.families.insert(family, vec!["NotoSans".to_owned()]);
    }
    ctx.set_fonts(defs);
    ctx.set_font_providers(vec![Arc::new(BundledFonts::new(
        assets.join("fonts"),
        lang,
    ))]);
}

/// `assets/fonts/` içinden ihtiyaç anında yazı tipi bulan sağlayıcı.
struct BundledFonts {
    dir: PathBuf,
    order: Vec<&'static str>,
    /// Okunmuş dosyalar; okunamayanlar `None` olarak hatırlanır.
    cache: Mutex<HashMap<&'static str, Option<Arc<Vec<u8>>>>>,
}

impl BundledFonts {
    fn new(dir: PathBuf, lang: Lang) -> Self {
        let mut order = SCRIPT_FONTS.to_vec();
        order.extend(cjk_order(lang));
        Self {
            dir,
            order,
            cache: Mutex::new(HashMap::new()),
        }
    }

    fn load(&self, rel: &'static str) -> Option<Arc<Vec<u8>>> {
        let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        cache
            .entry(rel)
            .or_insert_with(|| {
                let path = self.dir.join(rel);
                match std::fs::read(&path) {
                    Ok(bytes) => {
                        log::debug!("yazı tipi yüklendi: {}", path.display());
                        Some(Arc::new(bytes))
                    }
                    Err(err) => {
                        log::warn!("yazı tipi okunamadı: {} ({err})", path.display());
                        None
                    }
                }
            })
            .clone()
    }
}

fn has_glyph(bytes: &[u8], ch: char) -> bool {
    FontRef::from_index(bytes, 0)
        .map(|f| f.charmap().map(ch).is_some())
        .unwrap_or(false)
}

impl FontProvider for BundledFonts {
    fn font_for(&self, request: &FallbackRequest<'_>) -> Option<FontInsert> {
        let ch = request.cluster.chars().next()?;
        for &rel in &self.order {
            let Some(bytes) = self.load(rel) else {
                continue;
            };
            if has_glyph(&bytes, ch) {
                return Some(FontInsert {
                    name: format!("bundled:{rel}"),
                    data: FontData::from_blob(bytes, 0),
                    families: vec![InsertFontFamily {
                        family: request.family.clone(),
                        priority: FontPriority::Lowest,
                    }],
                });
            }
        }
        log::debug!("hiçbir yazı tipinde yok: {:?}", request.cluster);
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn listedeki_tum_yazi_tipleri_depoda() {
        let dir = crate::paths::assets_dir().join("fonts");
        for rel in SCRIPT_FONTS {
            let bytes = std::fs::read(dir.join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"));
            assert!(FontRef::from_index(&bytes, 0).is_ok(), "{rel} okunamadı");
        }
    }

    #[test]
    fn yazi_sistemleri_dogru_dosyaya_duser() {
        let fonts = BundledFonts::new(crate::paths::assets_dir().join("fonts"), Lang::Tr);
        let find = |ch: char| {
            fonts
                .order
                .iter()
                .copied()
                .find(|rel| fonts.load(rel).is_some_and(|b| has_glyph(&b, ch)))
        };
        assert_eq!(find('ز'), Some("NotoSansArabic-Regular.ttf"));
        assert_eq!(find('א'), Some("NotoSansHebrew-Regular.ttf"));
        assert_eq!(find('क'), Some("NotoSansDevanagari-Regular.ttf"));
        assert_eq!(find('ก'), Some("NotoSansThai-Regular.ttf"));
        assert_eq!(find('ა'), Some("NotoSansGeorgian-Regular.ttf"));
        // Gömülü Noto Sans, Latin/Kiril/Yunan'ı kapsar.
        let noto = FontRef::from_index(NOTO_SANS, 0).unwrap();
        for ch in ['İ', 'ş', 'ğ', 'Ж', 'Σ'] {
            assert!(noto.charmap().map(ch).is_some(), "{ch}");
        }
    }
}
