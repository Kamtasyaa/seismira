//! Zemin yenilmesi katmanlarının (sıvılaşma, heyelan) renk ölçekleri ve
//! açıklama kutuları (Bölüm 2.6: "kırmızı tonlu kareler", yoğunluk =
//! olasılık).
//!
//! Gösterilen değer USGS Ground Failure ile aynı: hücre alanının etkilenen
//! oranı (alansal kaplama). Sınıf sınırları USGS'in görüntüleme ayarından
//! (`defaultconfigfiles/models/*.ini`, `display_options.lims.model`):
//! sıvılaşma (`zhu_2017_general.ini`) 0,005 … 0,5, heyelan
//! (`jessee_2018.ini`) 0,002 … 0,5 (main dalı, 2026-11-22 okundu).
//! Renk tek bilgi taşıyıcı değil (Bölüm 3): açıklama kutusunda her tonun
//! yüzde değeri ve katmanın adı yazar; iki katman ayrı ton ailesinde
//! (sıvılaşma pembe-kırmızı, heyelan turuncu-kahve).

use crate::i18n::Lang;
use eframe::egui::{self, Align2, Color32, FontId, Rect, vec2};

/// Bir katmanın renk ölçeği.
pub struct Palette {
    /// Sınıfların alt sınırları (alan oranı) ve renkleri; ilk sınırın
    /// altındaki hücreler çizilmez.
    pub classes: [(f32, Color32); 6],
}

impl Palette {
    /// Çizilen en küçük oran.
    pub fn min(&self) -> f32 {
        self.classes[0].0
    }
}

/// Sıvılaşma: açık pembeden koyu kırmızıya. Koyu harita zemininde ve
/// Shindo renklerinin üstünde ayırt edilsin diye doygun ve opak.
pub const LIQUEFACTION: Palette = Palette {
    classes: [
        (0.005, Color32::from_rgb(0xFF, 0xC2, 0xC2)),
        (0.01, Color32::from_rgb(0xFF, 0x8F, 0x8F)),
        (0.02, Color32::from_rgb(0xFF, 0x55, 0x55)),
        (0.05, Color32::from_rgb(0xE5, 0x1C, 0x1C)),
        (0.10, Color32::from_rgb(0xB0, 0x00, 0x0C)),
        (0.20, Color32::from_rgb(0x6E, 0x00, 0x10)),
    ],
};

/// Heyelan: açık şeftaliden koyu kahveye (sıvılaşmadan ayrı ton ailesi).
pub const LANDSLIDE: Palette = Palette {
    classes: [
        (0.002, Color32::from_rgb(0xFF, 0xD9, 0xA8)),
        (0.01, Color32::from_rgb(0xFF, 0xB0, 0x5E)),
        (0.02, Color32::from_rgb(0xF5, 0x86, 0x22)),
        (0.05, Color32::from_rgb(0xCC, 0x5A, 0x00)),
        (0.10, Color32::from_rgb(0x94, 0x3B, 0x00)),
        (0.20, Color32::from_rgb(0x5C, 0x24, 0x00)),
    ],
};

/// Sıvılaşmanın çizilen en küçük oranı (bölge özeti de bunu kullanır).
pub const MIN_COVERAGE: f32 = LIQUEFACTION.classes[0].0;

/// Bir oranı yüzde metnine çevirir ("%0,5" / "0.5%").
fn percent(lang: Lang, v: f32) -> String {
    // f32 sınırları (0,01 → 0,9999…) tam yüzdeye yuvarlansın.
    let p = (f64::from(v) * 1000.0).round() / 10.0;
    let n = if p < 1.0 {
        lang.num(p, 1)
    } else {
        lang.num(p, 0)
    };
    lang.t_args("percent", &[("v", n)])
}

/// Açıklama kutusunun dikey yeri (piksel): `slot` 0 Shindo açıklamasının
/// hemen üstü, 1 onun da üstü (iki katman birlikte gösterilince).
const SLOT_H: f32 = 56.0;

/// Açıklama kutusu: Shindo açıklamasının üstünde, sağ altta.
pub fn draw_legend(ui: &egui::Ui, rect: Rect, lang: Lang, title: &str, pal: &Palette, slot: u32) {
    let painter = ui.painter_at(rect);
    let sw = vec2(30.0, 12.0);
    let width = pal.classes.len() as f32 * sw.x;
    // Shindo açıklamasının yüksekliği (city_layer::draw_legend): kutu +
    // başlık + model sınırı notu + kenar boşlukları.
    let below = 12.0 + crate::shindo_style::BOX_SIZE.y + 18.0 + 22.0 + 14.0 + slot as f32 * SLOT_H;
    let galley = painter.layout_no_wrap(
        title.to_owned(),
        FontId::proportional(12.0),
        ui.visuals().text_color(),
    );
    // Kutu başlığa da sığsın; sağ kenar Shindo açıklamasıyla hizalı.
    let width_all = width.max(galley.size().x);
    let origin = rect.right_bottom() - vec2(12.0 + width_all, below + sw.y + 16.0);
    let bg = Rect::from_min_max(
        origin - vec2(6.0, 20.0),
        origin + vec2(width_all + 6.0, sw.y + 18.0),
    );
    painter.rect_filled(bg, 4.0, crate::theme::MAP_BG.gamma_multiply(0.85));
    painter.galley(
        origin - vec2(0.0, 4.0 + galley.size().y),
        galley,
        ui.visuals().text_color(),
    );
    let origin = origin + vec2(width_all - width, 0.0);
    for (k, (lo, color)) in pal.classes.iter().enumerate() {
        let r = Rect::from_min_size(origin + vec2(k as f32 * sw.x, 0.0), sw);
        painter.rect_filled(r.shrink2(vec2(1.0, 0.0)), 1.0, *color);
        painter.text(
            r.center_bottom() + vec2(0.0, 2.0),
            Align2::CENTER_TOP,
            percent(lang, *lo),
            FontId::proportional(10.5),
            ui.visuals().weak_text_color(),
        );
    }
}

/// Blok piramidinin düzey sayısı: düzey k'de bir kare 2^k × 2^k hücre.
/// Shader, ekranda bir kare en az 8 piksel olacak düzeyi seçer
/// (map/ground.wgsl, `MIN_PX`).
pub const LEVELS: u32 = 7;

/// Blok piramidi: her düzeyde (renk sınıfı ya da −1, belirme anı, 0, 0).
/// Blok değeri, bloğun içindeki hücrelerin kaplama ORTALAMASI (eşit alanlı
/// sayılır; blok = bloğun etkilenen alan oranı); belirme anı, kaplaması
/// sıfırdan büyük hücrelerde sarsıntının en erken bittiği an. Izgara
/// 2^(LEVELS−1)'in katına tamamlanır (dolgu kaplaması 0); döner:
/// (genişlik, yükseklik, düzeyler).
pub fn pyramid(
    pal: &Palette,
    coverage: &[f32],
    t_end: &[f32],
    w: u32,
    h: u32,
) -> (u32, u32, Vec<Vec<[f32; 4]>>) {
    let m = 1u32 << (LEVELS - 1);
    let (pw, ph) = (w.div_ceil(m) * m, h.div_ceil(m) * m);
    // Düzey 0: (kaplama, an).
    let mut cur: Vec<(f32, f32)> = vec![(0.0, f32::INFINITY); (pw * ph) as usize];
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) as usize;
            let c = coverage[i];
            if c > 0.0 && t_end[i].is_finite() {
                cur[(y * pw + x) as usize] = (c, t_end[i]);
            }
        }
    }
    let texel = |(c, t): (f32, f32)| match pal.classes.iter().rposition(|(lo, _)| c >= *lo) {
        Some(k) if t.is_finite() => [k as f32, t, 0.0, 0.0],
        _ => [-1.0, f32::INFINITY, 0.0, 0.0],
    };
    let mut out = vec![cur.iter().copied().map(texel).collect::<Vec<_>>()];
    let (mut lw, mut lh) = (pw, ph);
    for _ in 1..LEVELS {
        let (nw, nh) = (lw / 2, lh / 2);
        let mut next = vec![(0.0, f32::INFINITY); (nw * nh) as usize];
        for y in 0..nh {
            for x in 0..nw {
                let mut sum = 0.0;
                let mut t = f32::INFINITY;
                for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    let (c, tt) = cur[((2 * y + dy) * lw + 2 * x + dx) as usize];
                    sum += c;
                    if c > 0.0 {
                        t = t.min(tt);
                    }
                }
                next[(y * nw + x) as usize] = (sum / 4.0, t);
            }
        }
        out.push(next.iter().copied().map(texel).collect());
        cur = next;
        (lw, lh) = (nw, nh);
    }
    (pw, ph, out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn piramit_ortalama_ve_en_erken_an() {
        // 3 × 1 ızgara: 0,2 (t 10), 0 , 0,06 (t 5).
        let (pw, ph, lv) = pyramid(&LIQUEFACTION, &[0.2, 0.0, 0.06], &[10.0, 20.0, 5.0], 3, 1);
        assert_eq!((pw, ph), (64, 64));
        assert_eq!(lv.len(), LEVELS as usize);
        // Düzey 0: hücreler ayrı.
        assert_eq!(lv[0][0], [5.0, 10.0, 0.0, 0.0]);
        assert_eq!(lv[0][1][0], -1.0);
        assert_eq!(lv[0][2], [3.0, 5.0, 0.0, 0.0]);
        // Düzey 1: (0,2 + 0 + 0 + 0) / 4 = 0,05 → sınıf 3; an 10.
        assert_eq!(lv[1][0], [3.0, 10.0, 0.0, 0.0]);
        // (0,06 + 0·3) / 4 = 0,015 → sınıf 1; an 5.
        assert_eq!(lv[1][1], [1.0, 5.0, 0.0, 0.0]);
        // Düzey 2: (0,2 + 0,06) / 16 = 0,01625 → sınıf 1, en erken an 5.
        assert_eq!(lv[2][0], [1.0, 5.0, 0.0, 0.0]);
        assert_eq!(lv[6].len(), 1);
    }
}
