//! Seismira masaüstü uygulaması (egui + eframe).

mod app;
mod city_layer;
mod compute;
#[cfg(feature = "screenshot")]
mod devshot;
mod fonts;
mod i18n;
mod map;
mod panel;
mod paths;
mod shindo_style;
mod text_test;
mod theme;
mod timeline;

/// Çoklu örnekleme (MSAA): harita çizgileri ve kıyılar için kenar yumuşatma.
/// Harita çizim hattı da aynı örnek sayısını kullanmak zorundadır.
pub const MSAA: u32 = 4;

use eframe::egui;

fn main() -> eframe::Result {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // Çizim: wgpu (Vulkan / DirectX 12 / Metal). Bunlar yoksa wgpu kendi
    // OpenGL arka ucuna düşer; WGPU_BACKEND=opengl ile zorlanabilir.
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(i18n::Lang::from_system().t("app-title"))
            .with_app_id("seismira")
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([900.0, 600.0]),
        multisampling: MSAA as u16,
        // Yalnızca uygulamayla gelen yazı tipleri (bkz. fonts.rs).
        system_font_fallback: false,
        ..Default::default()
    };
    eframe::run_native(
        "seismira",
        options,
        Box::new(|cc| Ok(Box::new(app::SeismiraApp::new(cc)))),
    )
}
