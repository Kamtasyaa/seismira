//! Seismira masaüstü uygulaması (egui + eframe).

mod app;
mod fonts;
mod i18n;
mod text_test;
mod theme;

use eframe::egui;

fn native_options(renderer: eframe::Renderer) -> eframe::NativeOptions {
    eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(i18n::Lang::from_system().t("app-title"))
            .with_app_id("seismira")
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([900.0, 600.0]),
        renderer,
        // Yalnızca uygulamayla gelen yazı tipleri (bkz. fonts.rs).
        system_font_fallback: false,
        ..Default::default()
    }
}

fn run(renderer: eframe::Renderer) -> eframe::Result {
    log::info!("çizim arka ucu: {renderer:?}");
    eframe::run_native(
        "seismira",
        native_options(renderer),
        Box::new(|cc| Ok(Box::new(app::SeismiraApp::new(cc)))),
    )
}

fn main() -> eframe::Result {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // Varsayılan wgpu (Vulkan / DirectX 12 / Metal). Açılamazsa eski
    // donanım için OpenGL (glow) yedeği. SEISMIRA_RENDERER=glow ile zorlanır.
    let forced = std::env::var("SEISMIRA_RENDERER").ok();
    if forced.as_deref() == Some("glow") {
        return run(eframe::Renderer::Glow);
    }
    match run(eframe::Renderer::Wgpu) {
        Err(err) if forced.is_none() => {
            log::warn!("wgpu açılamadı ({err}); OpenGL ile deneniyor");
            run(eframe::Renderer::Glow)
        }
        other => other,
    }
}
