//! Deprem simülasyon çekirdeği.
//!
//! Bu crate arayüzden tamamen bağımsızdır: egui, eframe, wgpu veya
//! herhangi bir pencere / çizim kütüphanesine bağımlılık EKLENMEZ.

pub mod geo;
pub mod mapfile;
pub mod mercator;
pub mod quake;
pub mod sim;
pub mod traveltime;

/// Çekirdek sürümü (Cargo.toml'dan).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surum_bos_degil() {
        assert!(!VERSION.is_empty());
    }
}
