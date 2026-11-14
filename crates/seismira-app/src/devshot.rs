//! Geliştirme aracı: gerçek pencerenin (GPU çizimi dahil) ekran görüntüsü.
//!
//! ```text
//! SEISMIRA_SCREENSHOT=/tmp/a.png SEISMIRA_VIEW=lon,lat,ölçek \
//!     cargo run -p seismira-app --features screenshot
//! ```
//! Harita hazır olduktan birkaç kare sonra görüntü kaydedilir ve uygulama
//! kapanır. Yalnızca `screenshot` özelliğiyle derlenir.

use eframe::egui;
use std::sync::atomic::{AtomicU32, Ordering};

static FRAMES_READY: AtomicU32 = AtomicU32::new(0);

pub fn tick(ctx: &egui::Context, map_ready: bool) {
    let Some(path) = std::env::var_os("SEISMIRA_SCREENSHOT") else {
        return;
    };
    if !map_ready {
        ctx.request_repaint();
        return;
    }
    // Birkaç kare bekle: yazı tipleri ve ince harita yüklensin.
    let n = FRAMES_READY.fetch_add(1, Ordering::Relaxed);
    if n < 30 {
        ctx.request_repaint_after(std::time::Duration::from_millis(30));
        return;
    }
    if n == 30 {
        let ctx2 = ctx.clone();
        ctx.request_screenshot(move |img| {
            let [w, h] = img.size;
            let buf = image::RgbaImage::from_raw(w as u32, h as u32, img.as_raw().to_vec())
                .expect("görüntü boyutu");
            match buf.save(&path) {
                Ok(()) => log::info!("ekran görüntüsü: {}", path.to_string_lossy()),
                Err(e) => log::error!("ekran görüntüsü kaydedilemedi: {e}"),
            }
            ctx2.send_viewport_cmd(egui::ViewportCommand::Close);
            ctx2.request_repaint();
        });
    }
}
