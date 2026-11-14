//! Uygulama klasörlerinin bulunması.
//!
//! Sıra: ortam değişkeni, çalıştırılabilir dosyanın yanındaki klasör,
//! geliştirme sırasında depo kökündeki klasör.

use std::path::PathBuf;

fn find(env: &str, name: &str, dev_fallback: &str) -> PathBuf {
    if let Some(dir) = std::env::var_os(env) {
        return PathBuf::from(dir);
    }
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        let candidate = dir.join(name);
        if candidate.is_dir() {
            return candidate;
        }
    }
    PathBuf::from(dev_fallback)
}

/// Yazı tipleri gibi uygulamayla gelen dosyalar (`assets/`).
pub fn assets_dir() -> PathBuf {
    find(
        "SEISMIRA_ASSETS",
        "assets",
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets"),
    )
}

/// İşlenmiş veri (`data/`): harita dosyaları vb.
pub fn data_dir() -> PathBuf {
    find(
        "SEISMIRA_DATA",
        "data",
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../data"),
    )
}
