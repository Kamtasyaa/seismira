//! MMI (Değiştirilmiş Mercalli Şiddeti) renkleri ve etiketleri, haritada
//! "MMI" ölçeği seçildiğinde (Görünüm → Şiddet ölçeği; docs/arayuz.md).
//!
//! Renkler: USGS ShakeMap ArcGIS servisi `eq/sm_ShakeMap30DaySignificant`,
//! katman 5 "mi" (ShakeMap MMI), `drawingInfo.renderer.classBreakInfos`
//! (2026-11-23 okundu): I < 2 saydam; II–III (191,204,255); IV
//! (128,255,255); V (122,255,147); VI (255,255,0); VII (255,200,0); VIII
//! (255,145,0); IX (255,0,0); X+ (128,0,0). Sınıf sınırları tam sayılar
//! (ör. IV: 4 ≤ MMI < 5).

use eframe::egui::Color32;

/// Haritada gösterilen şiddet ölçeği (Görünüm → Şiddet ölçeği). Bölge
/// uyarıları ve bildirimler her zaman JMA Shindo'dur (erken uyarı ölçütü).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Scale {
    #[default]
    Shindo,
    Mmi,
}

impl Scale {
    pub fn code(self) -> &'static str {
        match self {
            Self::Shindo => "shindo",
            Self::Mmi => "mmi",
        }
    }

    pub fn from_code(s: &str) -> Option<Self> {
        [Self::Shindo, Self::Mmi]
            .into_iter()
            .find(|x| x.code() == s)
    }
}

/// Kademeli gösterim: Shindo `now` / `fin` ilerlemesine göre MMI sınıfı
/// (aynı anda tamamlanır; ızgaradaki yükselme kuralıyla aynı oran).
pub fn class_at(final_class: u8, shindo_now: usize, shindo_final: usize) -> u8 {
    if shindo_now >= shindo_final || shindo_final == 0 {
        return final_class;
    }
    let steps = f64::from(final_class.saturating_sub(1));
    (1.0 + (steps * shindo_now as f64 / shindo_final as f64).floor()) as u8
}

/// MMI sınıfı 1–10 (I–X): `floor(mmi)`, 1–10'a çekilir.
pub fn class(mmi: f64) -> u8 {
    mmi.floor().clamp(1.0, 10.0) as u8
}

/// Sınıfın rengi; I için `None` (gösterilmez).
pub fn color(class: u8) -> Option<Color32> {
    Some(match class {
        0 | 1 => return None,
        2 | 3 => Color32::from_rgb(191, 204, 255),
        4 => Color32::from_rgb(128, 255, 255),
        5 => Color32::from_rgb(122, 255, 147),
        6 => Color32::from_rgb(255, 255, 0),
        7 => Color32::from_rgb(255, 200, 0),
        8 => Color32::from_rgb(255, 145, 0),
        9 => Color32::from_rgb(255, 0, 0),
        _ => Color32::from_rgb(128, 0, 0),
    })
}

/// Roma rakamı.
pub fn label(class: u8) -> &'static str {
    ["I", "II", "III", "IV", "V", "VI", "VII", "VIII", "IX", "X"]
        [usize::from(class.clamp(1, 10)) - 1]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn siniflar_ve_renkler() {
        assert_eq!(class(0.5), 1);
        assert_eq!(class(3.99), 3);
        assert_eq!(class(4.0), 4);
        assert_eq!(class(12.0), 10);
        assert_eq!(color(1), None);
        assert_eq!(color(2), color(3));
        assert_eq!(color(6), Some(Color32::from_rgb(255, 255, 0)));
        assert_eq!(label(8), "VIII");
    }
}
