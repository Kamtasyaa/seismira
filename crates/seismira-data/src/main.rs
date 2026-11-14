//! Veri hazırlama aracı: ham veriyi (shapefile, GeoTIFF, CSV) uygulamanın
//! hızlı okuduğu kompakt ikili biçime çevirir.

fn main() {
    println!(
        "seismira-data {} (core {})",
        env!("CARGO_PKG_VERSION"),
        seismira_core::VERSION
    );
}
