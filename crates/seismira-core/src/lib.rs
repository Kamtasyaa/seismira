//! Deprem simülasyon çekirdeği.
//!
//! Bu crate arayüzden tamamen bağımsızdır: egui, eframe, wgpu veya
//! herhangi bir pencere / çizim kütüphanesine bağımlılık EKLENMEZ.

pub mod cities;
pub mod component;
pub mod damage;
pub mod duration;
pub mod exposure;
pub mod geo;
pub mod gmpe;
pub mod grid;
pub mod groundfailure;
pub mod intensity;
pub mod mapfile;
pub mod mercator;
pub mod mmi;
pub mod quake;
pub mod raster;
pub mod region;
pub mod shaking;
pub mod sim;
pub mod site;
pub mod slab;
pub mod source;
pub mod traveltime;
pub mod tsunami;
pub mod warning;

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
