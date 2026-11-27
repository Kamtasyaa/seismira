//! Metin ve sayı kutularında sağ tık menüsü: Kes / Kopyala / Yapıştır /
//! Tümünü seç (kullanıcı isteği 2026-11-27). Ctrl+C / Ctrl+X / Ctrl+V
//! eframe'in `clipboard` özelliğiyle çalışır; menü aynı yolu kullanır:
//! kutuya odak verilir ve platforma kes / kopyala / yapıştır isteği
//! gönderilir ([`egui::ViewportCommand::RequestCopy`] vb.), olay bir sonraki
//! karede odaktaki kutuya ulaşır. Seçim yoksa Kes / Kopyala tüm metni alır,
//! Yapıştır tüm metnin yerine geçer (sayı kutusunda beklenen davranış);
//! seçim varsa yalnızca o.

use crate::i18n::Lang;
use eframe::egui::{self, Response, Ui, ViewportCommand, Widget};
use egui::text::{CCursor, CCursorRange};

/// Kutunun metin durumunda seçim yoksa tümünü seçer; odağı verir.
fn prepare(ctx: &egui::Context, id: egui::Id, select_all: bool) {
    let mut st = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
    let has_selection = st
        .cursor
        .char_range()
        .is_some_and(|r| r.primary.index != r.secondary.index);
    if select_all || !has_selection {
        // Uç, metnin uzunluğuna egui tarafından kırpılır (CursorRange::range).
        st.cursor.set_char_range(Some(CCursorRange::two(
            CCursor::new(0),
            CCursor::new(usize::MAX / 4),
        )));
        st.store(ctx, id);
    }
    ctx.memory_mut(|m| m.request_focus(id));
}

pub trait EditMenu {
    /// Sağ tık menüsünü ekler.
    fn edit_menu(self, lang: Lang) -> Self;
}

impl EditMenu for Response {
    fn edit_menu(self, lang: Lang) -> Self {
        let (ctx, id, enabled) = (self.ctx.clone(), self.id, self.enabled());
        if !enabled {
            return self;
        }
        self.context_menu(|ui| {
            let item = |ui: &mut Ui, key: &str, shortcut: &str| {
                ui.add(egui::Button::new(lang.t(key)).shortcut_text(shortcut))
                    .clicked()
            };
            if item(ui, "edit-cut", "Ctrl+X") {
                prepare(&ctx, id, false);
                ctx.send_viewport_cmd(ViewportCommand::RequestCut);
                ui.close();
            }
            if item(ui, "edit-copy", "Ctrl+C") {
                prepare(&ctx, id, false);
                ctx.send_viewport_cmd(ViewportCommand::RequestCopy);
                ui.close();
            }
            if item(ui, "edit-paste", "Ctrl+V") {
                prepare(&ctx, id, false);
                ctx.send_viewport_cmd(ViewportCommand::RequestPaste);
                ui.close();
            }
            ui.separator();
            if item(ui, "edit-select-all", "Ctrl+A") {
                prepare(&ctx, id, true);
                ui.close();
            }
        });
        self
    }
}

/// Bir girdi kutusunu sağ tık menüsüyle sarar (`ui.add(WithMenu(w, lang))`).
pub struct WithMenu<W>(pub W, pub Lang);

impl<W: Widget> Widget for WithMenu<W> {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.add(self.0).edit_menu(self.1)
    }
}

impl<'a> WithMenu<egui::DragValue<'a>> {
    pub fn suffix(self, s: &'static str) -> Self {
        WithMenu(self.0.suffix(s), self.1)
    }
}

/// Geliştirme: `SEISMIRA_CLIPTEST=1` panoya bilinen metni yazar, ~1 s sonra
/// yapıştırma ister ve gelen metni günlüğe yazar (Wayland panosu denemesi).
pub fn self_test(ctx: &egui::Context) {
    use std::sync::atomic::{AtomicU32, Ordering};
    static FRAME: AtomicU32 = AtomicU32::new(0);
    if std::env::var("SEISMIRA_CLIPTEST").as_deref() != Ok("1") {
        return;
    }
    let n = FRAME.fetch_add(1, Ordering::Relaxed);
    ctx.request_repaint_after(std::time::Duration::from_millis(30));
    ctx.input(|i| {
        for e in &i.raw.events {
            match e {
                egui::Event::Paste(t) => log::info!("PANO yapıştırma: {t:?} (kare {n})"),
                egui::Event::Copy => log::info!("PANO Copy olayı (kare {n})"),
                _ => {}
            }
        }
    });
    if n == 40 {
        log::info!("PANO yazılıyor");
        ctx.copy_text("seismira-pano-denemesi".into());
    }
    if n == 80 {
        log::info!("PANO yapıştırma isteniyor");
        ctx.send_viewport_cmd(ViewportCommand::RequestPaste);
    }
    if n == 140 {
        ctx.send_viewport_cmd(ViewportCommand::Close);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::kittest::Queryable;

    /// Sayı kutusunda sağ tık menüsü açılır; "Tümünü seç" kutuyu düzenleme
    /// kipine alır ve odak verir.
    #[test]
    fn sag_tik_menusu() {
        let lang = Lang::Tr;
        let mut h = egui_kittest::Harness::builder()
            .with_size([300.0, 200.0])
            .with_context_setup(move |ctx| {
                crate::fonts::install(ctx, &crate::paths::assets_dir(), lang);
            })
            .build_ui_state(
                move |ui, v: &mut f64| {
                    let l = ui.label("Büyüklük").id;
                    ui.add(WithMenu(egui::DragValue::new(v), lang))
                        .labelled_by(l);
                },
                7.5f64,
            );
        h.run();
        h.get_by_role(egui::accesskit::Role::SpinButton)
            .click_secondary();
        h.run();
        h.run();
        for k in ["edit-cut", "edit-copy", "edit-paste", "edit-select-all"] {
            assert!(h.query_by_label_contains(&lang.t(k)).is_some(), "{k} yok");
        }
        h.get_by_label_contains(&lang.t("edit-select-all")).click();
        h.run();
        h.run();
        let focused = h.ctx.memory(|m| m.focused());
        assert!(focused.is_some(), "odak yok");
        // Menü kapandı.
        assert!(h.query_by_label_contains(&lang.t("edit-copy")).is_none());
    }

    fn harness(lang: Lang) -> egui_kittest::Harness<'static, f64> {
        egui_kittest::Harness::builder()
            .with_size([300.0, 200.0])
            .with_context_setup(move |ctx| {
                crate::fonts::install(ctx, &crate::paths::assets_dir(), lang);
            })
            .build_ui_state(
                move |ui, v: &mut f64| {
                    let l = ui.label("Büyüklük").id;
                    ui.add(WithMenu(
                        egui::DragValue::new(v)
                            .custom_formatter(move |v, _| lang.num(v, 1))
                            .custom_parser(|s| s.trim().replace(',', ".").parse().ok()),
                        lang,
                    ))
                    .labelled_by(l);
                    let _ = ui.button("başka");
                },
                7.5f64,
            )
    }

    fn copied(h: &egui_kittest::Harness<'static, f64>) -> Option<String> {
        h.output()
            .platform_output
            .commands
            .iter()
            .find_map(|c| match c {
                egui::OutputCommand::CopyText(t) => Some(t.clone()),
                _ => None,
            })
    }

    /// Kopyala: platformun RequestCopy karşılığı Copy olayı gelince değer
    /// panoya yazılır.
    #[test]
    fn kopyala_menuden() {
        let lang = Lang::Tr;
        let mut h = harness(lang);
        h.run();
        h.get_by_role(egui::accesskit::Role::SpinButton)
            .click_secondary();
        h.run();
        h.run();
        h.get_by_label_contains(&lang.t("edit-copy")).click();
        h.run();
        // Entegrasyon RequestCopy'yi bir sonraki karede Copy olayına çevirir.
        h.event(egui::Event::Copy);
        let mut got = None;
        for _ in 0..4 {
            h.step();
            got = got.or_else(|| copied(&h));
        }
        assert_eq!(got.as_deref(), Some("7,5"));
    }

    /// Yapıştır: Paste olayı tüm metnin yerine geçer, Enter ile değer olur.
    #[test]
    fn yapistir_menuden() {
        let lang = Lang::Tr;
        let mut h = harness(lang);
        h.run();
        h.get_by_role(egui::accesskit::Role::SpinButton)
            .click_secondary();
        h.run();
        h.run();
        h.get_by_label_contains(&lang.t("edit-paste")).click();
        h.run();
        h.event(egui::Event::Paste("6,8".into()));
        h.run();
        h.key_press(egui::Key::Enter);
        h.run();
        assert!((*h.state() - 6.8).abs() < 1e-9, "{}", h.state());
    }
}
