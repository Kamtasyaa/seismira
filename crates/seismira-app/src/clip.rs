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
}
