//! Deprem simülasyon çekirdeği.
//!
//! Bu crate arayüzden tamamen bağımsızdır: egui, eframe, wgpu veya
//! herhangi bir pencere / çizim kütüphanesine bağımlılık EKLENMEZ.

pub mod cities;
pub mod geo;
pub mod gmpe;
pub mod intensity;
pub mod mapfile;
pub mod mercator;
pub mod quake;
pub mod raster;
pub mod shaking;
pub mod sim;
pub mod site;
pub mod source;
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
