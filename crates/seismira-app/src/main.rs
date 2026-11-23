//! Seismira masaüstü uygulaması (egui + eframe).

// Windows sürüm derlemesinde uygulamanın yanında konsol penceresi açılmasın.
// Log yine de dosyaya yazılır (bkz. `init_logging`).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod city_layer;
mod compute;
#[cfg(feature = "screenshot")]
mod devshot;
mod fonts;
mod ground_style;
mod i18n;
mod map;
mod mmi_style;
mod panel;
mod paths;
mod scenarios;
mod shindo_style;
mod sources;
mod text_test;
mod theme;
mod timeline;
mod tsunami;

/// Çoklu örnekleme (MSAA): harita çizgileri ve kıyılar için kenar yumuşatma.
/// Harita çizim hattı da aynı örnek sayısını kullanmak zorundadır.
pub const MSAA: u32 = 4;

use eframe::egui;

fn main() -> eframe::Result {
    init_logging();

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

/// Log hem stderr'e hem de uygulama durum klasöründeki `seismira.log`
/// dosyasına yazılır (her açılışta baştan). Böylece terminal açmadan da
/// log gönderilebilir; Windows sürümünde konsol olmadığı için tek yol budur.
/// Klasör: Linux `~/.local/share/seismira`, macOS
/// `~/Library/Application Support/seismira`, Windows
/// `%APPDATA%\seismira\data` (eframe'in kayıt klasörüyle aynı).
/// Panik iletisi de log'a düşer.
fn init_logging() {
    use std::io::Write;

    struct Tee(Option<std::fs::File>);
    impl Write for Tee {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            // stderr yoksa (Windows konsolsuz) hata yok sayılır.
            let _ = std::io::stderr().write_all(buf);
            if let Some(f) = &mut self.0 {
                let _ = f.write_all(buf);
            }
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            let _ = std::io::stderr().flush();
            if let Some(f) = &mut self.0 {
                let _ = f.flush();
            }
            Ok(())
        }
    }

    let path = eframe::storage_dir("seismira").map(|d| d.join("seismira.log"));
    let file = path.as_ref().and_then(|p| {
        std::fs::create_dir_all(p.parent()?).ok()?;
        std::fs::File::create(p).ok()
    });
    let has_file = file.is_some();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .target(env_logger::Target::Pipe(Box::new(Tee(file))))
        .init();
    match (&path, has_file) {
        (Some(p), true) => log::info!("log dosyası: {}", p.display()),
        _ => log::warn!("log dosyası açılamadı; log yalnızca stderr'de"),
    }

    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        log::error!("panik: {info}");
        default_hook(info);
    }));
}
