//! Zaman çizgisi: simülasyon saati, oynat/duraklat, sarma, hızlar
//! (CLAUDE.md Bölüm 10).

use eframe::egui::{self, Key, Modifiers, RichText};
use seismira_core::sim::Simulation;

use crate::i18n::Lang;

/// Oynatma hızları (simülasyon saniyesi / duvar saniyesi). Tsunami saatler
/// sürdüğü için 3600x'e kadar.
pub const SPEEDS: [f64; 5] = [1.0, 10.0, 60.0, 600.0, 3600.0];

/// ⏪ / ⏩ ve ← / → adımı (s); Shift ile [`BIG_STEP`].
const STEP: f64 = 10.0;
const BIG_STEP: f64 = 60.0;

/// Tek karede ilerletilecek en uzun duvar süresi (s). Pencere bir süre
/// donarsa simülasyon sıçramasın.
const MAX_FRAME_DT: f64 = 0.1;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Clock {
    /// Simülasyon zamanı (s, deprem anından).
    pub t: f64,
    pub playing: bool,
    /// [`SPEEDS`] içindeki indeks.
    pub speed: usize,
}

impl Default for Clock {
    fn default() -> Self {
        Self {
            t: 0.0,
            playing: false,
            speed: 0,
        }
    }
}

impl Clock {
    /// Zamanda adım atar (sarma); [0, end] içinde kalır.
    pub fn seek_by(&mut self, delta: f64, end: f64) {
        self.t = (self.t + delta).clamp(0.0, end.max(0.0));
    }

    fn toggle_play(&mut self, end: f64) {
        if !self.playing && self.t >= end {
            self.t = 0.0; // sondaysa baştan oynat
        }
        self.playing = !self.playing;
    }

    /// Duvar saatinde geçen `dt` kadar ilerletir; sonda durur.
    pub fn advance(&mut self, dt: f64, end: f64) {
        if !self.playing {
            return;
        }
        self.t += dt.clamp(0.0, MAX_FRAME_DT) * SPEEDS[self.speed];
        if self.t >= end {
            self.t = end;
            self.playing = false;
        }
    }
}

/// "1:02:03" ya da "02:03".
pub fn format_hms(t: f64) -> String {
    let s = t.max(0.0).floor() as u64;
    let (h, m, s) = (s / 3600, (s / 60) % 60, s % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m:02}:{s:02}")
    }
}

pub fn ui(ui: &mut egui::Ui, lang: Lang, clock: &mut Clock, sim: Option<&Simulation>) {
    let t = |k| lang.t(k);
    let end = sim.map_or(0.0, Simulation::end_time);
    // Klavye: metin kutusu odakta değilse. Kaydırıcıdan önce tüketilir ki
    // odaktaki kaydırıcı aynı tuşu ikinci kez işlemesin.
    if sim.is_some() && !ui.ctx().egui_wants_keyboard_input() {
        ui.input_mut(|i| {
            for (key, sign) in [(Key::ArrowLeft, -1.0), (Key::ArrowRight, 1.0)] {
                if i.consume_key(Modifiers::SHIFT, key) {
                    clock.seek_by(sign * BIG_STEP, end);
                } else if i.consume_key(Modifiers::NONE, key) {
                    clock.seek_by(sign * STEP, end);
                }
            }
            if i.consume_key(Modifiers::NONE, Key::Space) {
                clock.toggle_play(end);
            }
        });
    }

    ui.add_enabled_ui(sim.is_some(), |ui| {
        ui.horizontal(|ui| {
            if ui.button("⏮").on_hover_text(t("restart")).clicked() {
                clock.t = 0.0;
            }
            if ui.button("⏪").on_hover_text(t("step-back")).clicked() {
                clock.seek_by(-STEP, end);
            }
            let (icon, tip) = if clock.playing {
                ("⏸", t("pause"))
            } else {
                ("▶", t("play"))
            };
            if ui.button(icon).on_hover_text(tip).clicked() {
                clock.toggle_play(end);
            }
            if ui.button("⏩").on_hover_text(t("step-forward")).clicked() {
                clock.seek_by(STEP, end);
            }
            let next = sim.and_then(|s| s.next_event_after(clock.t));
            if ui
                .add_enabled(next.is_some(), egui::Button::new("⏭"))
                .on_hover_text(t("next-event"))
                .clicked()
                && let Some(n) = next
            {
                clock.t = n;
            }

            let label = lang.t_args("time-elapsed", &[("t", format_hms(clock.t))]);
            ui.label(RichText::new(label).monospace());

            // Hız seçimi sağda, kaydırıcı arada kalan genişlikte.
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                for i in (0..SPEEDS.len()).rev() {
                    ui.selectable_value(&mut clock.speed, i, format!("{}x", SPEEDS[i]));
                }
                ui.label(RichText::new(t("speed")).weak());
                ui.spacing_mut().slider_width = (ui.available_width() - 16.0).max(60.0);
                let slider = egui::Slider::new(&mut clock.t, 0.0..=end.max(1.0)).show_value(false);
                if ui.add(slider).dragged() {
                    clock.playing = false;
                }
            });
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saat_bicimi() {
        assert_eq!(format_hms(0.0), "00:00");
        assert_eq!(format_hms(125.9), "02:05");
        assert_eq!(format_hms(3723.0), "1:02:03");
    }

    #[test]
    fn sarma_sinirlarda_kalir() {
        let mut c = Clock::default();
        c.seek_by(-10.0, 100.0);
        assert_eq!(c.t, 0.0);
        c.seek_by(95.0, 100.0);
        c.seek_by(10.0, 100.0);
        assert_eq!(c.t, 100.0);
        // Sondayken oynat: baştan başlar.
        c.toggle_play(100.0);
        assert!(c.playing && c.t == 0.0);
    }

    #[test]
    fn saat_ilerler_ve_sonda_durur() {
        let mut c = Clock {
            t: 0.0,
            playing: true,
            speed: 2, // 60x
        };
        c.advance(0.05, 100.0);
        assert!((c.t - 3.0).abs() < 1e-9);
        // Uzun kare: en fazla 0.1 s duvar zamanı sayılır.
        c.advance(5.0, 100.0);
        assert!((c.t - 9.0).abs() < 1e-9);
        c.speed = 4;
        c.advance(0.1, 100.0);
        assert_eq!(c.t, 100.0);
        assert!(!c.playing);
        // Duraklatılmışken ilerlemez.
        c.t = 10.0;
        c.advance(0.1, 100.0);
        assert_eq!(c.t, 10.0);
    }
}
