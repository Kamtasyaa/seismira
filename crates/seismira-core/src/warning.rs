//! VARSAYIMSAL deprem erken uyarı sistemi (CLAUDE.md Bölüm 7.3). Çoğu
//! ülkede kamuya açık bir sistem yok; arayüz bunu böyle etiketler.
//!
//! Uyarı ölçütü JMA'nınkidir (doğrulandı, 2026-11-14):
//! - JMA SSS (www.jma.go.jp/jma/kishou/know/faq/faq25.html): "最大震度が
//!   ５弱以上 … と予想された場合に、震度４以上 … が予想される地域を対象に
//!   緊急地震速報（警報）を発表します" — en yüksek tahmini Shindo 5- ve
//!   üstüyse, Shindo 4 ve üstü beklenen bölgelere uyarı.
//! - JMA EEW ayrıntıları (www.jma.go.jp/jma/kishou/know/jishin/eew/shikumi/
//!   shousai.html): "地震波が２点以上の地震観測点で観測され" — en az iki
//!   istasyonda gözlem.
//!
//! Zamanlama modeli (2026-11-24): uyarı anı = P dalgasının, merkez üssünden
//! [`System::station_km`] uzaklıktaki sanal İLK istasyona varışı (JMA'nın
//! "地震検知", algılama) + [`System::latency_s`] (algılamadan ilk rapora).
//! - 4,5 s: JMA, 験震時報 73 (2010) 1–122, Doi, Matsumori, Aikawa & Yokota,
//!   "緊急地震速報の評価・改善に向けた検討", ek tablo "地震検知から第1報までの
//!   時間" (2007-10-01 – 2008-12-31, 44 olay): medyan 4,55 s, çeyrekler
//!   4,0–5,8 s, en az 2,6, en çok 26,7 s; uyarı (警報) yayımlanan 9 olayda
//!   medyan 4,1 s. Süre ikinci istasyonu beklemeyi ve işlemi içerir.
//!   (jma.go.jp/jma/kishou/books/kenshin/vol73p001.pdf; data/raw/refs/misc/.)
//! - 10 km: ilk istasyonun uzaklığı VARSAYIM: K-NET aralığı ~20 km (NIED);
//!   rastgele bir merkez üssünün en yakın istasyonu aralığın yaklaşık yarısı
//!   uzaklıktadır. (Önceki model: 20 km'deki ikinci istasyon + 5 s,
//!   kaynaksız; uyarıyı derinliğe göre ~2 s geç veriyordu.)

use crate::geo;
use crate::intensity::Shindo;
use crate::traveltime::{Phase, TravelTimes};

/// Sanal erken uyarı sistemi.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct System {
    /// İlk (algılayan) istasyonun merkez üssüne uzaklığı (km).
    pub station_km: f64,
    /// Algılamadan ilk rapora geçen süre (s): ikinci istasyonu bekleme +
    /// işlem + iletim.
    pub latency_s: f64,
}

impl System {
    /// Yoğun ağ (varsayılan).
    pub const DENSE: System = System {
        station_km: 10.0,
        latency_s: 4.5,
    };

    /// Uyarının yayımlandığı an (s, deprem anına göre). Seyahat süresi
    /// tablosu bu derinliği kapsamıyorsa `None`.
    pub fn alert_time(&self, tt: &TravelTimes, depth_km: f64) -> Option<f64> {
        let t_p = tt.time(Phase::P, depth_km, geo::km_to_deg(self.station_km))?;
        Some(t_p + self.latency_s)
    }
}

/// Uyarı yayımlanır mı? (tahmini en yüksek Shindo 5- ve üstü)
pub fn issued(predicted_max: Shindo) -> bool {
    predicted_max >= Shindo::S5Lower
}

/// Bölge uyarı kapsamında mı? (tahmini Shindo 4 ve üstü)
pub fn region_covered(region_max: Shindo) -> bool {
    region_max >= Shindo::S4
}

/// Bir sahadaki uyarı süresi: S varışı − uyarı anı. Sıfır ya da negatifse
/// saha "kör bölge"dedir (uyarı sarsıntıdan sonra gelir).
pub fn lead_time(t_s: f64, t_alert: f64) -> f64 {
    t_s - t_alert
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn tt() -> TravelTimes {
        let p = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/traveltime/ak135.ttt"
        );
        TravelTimes::load(Path::new(p)).unwrap()
    }

    #[test]
    fn uyari_ani_mantikli() {
        let t = tt();
        // 10 km derinlik, 10 km uzaklık: P ~ 14 km / ~6 km/s ≈ 2,4 s; + 4,5 s.
        let a = System::DENSE.alert_time(&t, 10.0).unwrap();
        assert!((6.0..8.0).contains(&a), "{a}");
        // Derin depremde P istasyona daha geç ulaşır.
        assert!(System::DENSE.alert_time(&t, 100.0).unwrap() > a);
    }

    #[test]
    fn olcut_ve_kapsam() {
        assert!(!issued(Shindo::S4));
        assert!(issued(Shindo::S5Lower));
        assert!(!region_covered(Shindo::S3));
        assert!(region_covered(Shindo::S4));
    }

    #[test]
    fn kor_bolge() {
        let t = tt();
        let a = System::DENSE.alert_time(&t, 10.0).unwrap();
        // Merkez üssünde S, uyarıdan önce gelir → kör bölge.
        let ts_near = t.time(Phase::S, 10.0, 0.0).unwrap();
        assert!(lead_time(ts_near, a) <= 0.0);
        // 150 km'de onlarca saniye önceden uyarı.
        let ts_far = t.time(Phase::S, 10.0, geo::km_to_deg(150.0)).unwrap();
        assert!(lead_time(ts_far, a) > 20.0);
    }
}
