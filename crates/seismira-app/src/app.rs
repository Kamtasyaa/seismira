//! Ana pencere ve panel düzeni (CLAUDE.md Bölüm 10).

use eframe::egui::{self, Align2, Color32, FontId, RichText, Stroke};
use seismira_core::intensity::Shindo;
use seismira_core::sim::{Event, EventKind, Simulation};
use seismira_core::slab::Slab2;
use seismira_core::traveltime::{Front, Phase, TravelTimes};
use seismira_core::{geo, mercator, quake};
use std::path::PathBuf;
use std::sync::Arc;

use crate::city_layer::{self, CityLayer};
use crate::compute::{self, CityResults};
use crate::map::{MapWidget, view::MapView};
use crate::timeline::{self, Clock};
use crate::{fonts, i18n::Lang, panel, paths, shindo_style, text_test, theme};

/// Dalga halkası renkleri. P (önce gelen, zayıf sarsıntı) soğuk, S (güçlü
/// sarsıntı) sıcak renk. Renk tek bilgi taşıyıcı değildir: haritada
/// açıklama kutusu var (Bölüm 3.2).
const P_COLOR: Color32 = Color32::from_rgb(0x5A, 0xB0, 0xFF);
const S_COLOR: Color32 = Color32::from_rgb(0xFF, 0x8A, 0x45);
/// Halka başına jeodezik nokta sayısı.
const RING_POINTS: usize = 256;

const STORAGE_LANG: &str = "lang";
const STORAGE_REDUCE_MOTION: &str = "reduce_motion";
const STORAGE_FAULT_TRACE: &str = "fault_trace";
const STORAGE_GROUND_FAILURE: &str = "ground_failure";
const STORAGE_LANDSLIDE: &str = "landslide";
const STORAGE_TSUNAMI: &str = "tsunami";
const STORAGE_QUALITY: &str = "quality";
const STORAGE_SCALE: &str = "intensity_scale";

/// Bölge sınırı yanıp sönme frekansı (Hz). Bölüm 3 / 9: ≤ ~1 Hz, yumuşak.
const BLINK_HZ: f64 = 1.0;

/// Bölgelerin bu karedeki görünümü (Bölüm 9; 2026-11-15 kullanıcı
/// isteğiyle: il ALANI düşük kontrastla yanıp söner, sınır yüksek
/// kontrastlı ve sabittir). `t` simülasyon zamanı, `wall` duvar saati
/// (yanıp sönme simülasyon hızından bağımsız). Dönen ikinci değer: yanıp
/// sönen bölge var mı (sürekli çizim gerekir).
/// Bir bölge görselinin ekranda kalacağı en kısa süre (duvar saniyesi).
/// Bölüm 9: hızlı oynatmada bir durum çok kısa sürerse görsel en az ~1 s
/// ekranda kalır (yalnızca sunum; simülasyon durumu değişmez).
const MIN_SHOW_S: f64 = 1.0;

/// Bölgenin EKRANDA gösterilen aşaması: 0 sessiz, 1 uyarı, 2 sarsıntı,
/// 3 yatışmış; `since`: bu aşamanın ekrana geldiği duvar saati.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Shown {
    stage: u8,
    since: f64,
}

/// Gösterilen aşamayı gerçek aşamaya (`actual`, simülasyon zamanının saf
/// fonksiyonu) doğru ilerletir. `hold` açıkken (oynatma sürerken) uyarı ve
/// sarsıntı görseli en az [`MIN_SHOW_S`] ekranda kalır ve aşamalar sırayla
/// gösterilir; bölge uyarı aşamasından hiç geçmiyorsa (`passes_warning`
/// değil) o aşama atlanır. Geri sarmada ve duraklatılmışken gerçek aşama
/// hemen gösterilir.
fn advance_shown(shown: Shown, actual: u8, passes_warning: bool, wall: f64, hold: bool) -> Shown {
    if actual == shown.stage {
        return shown;
    }
    if !hold || actual < shown.stage {
        return Shown {
            stage: actual,
            since: wall,
        };
    }
    // İleri: sessiz aşama beklemez, diğerleri en az MIN_SHOW_S görünür.
    if shown.stage != 0 && wall - shown.since < MIN_SHOW_S {
        return shown;
    }
    let mut next = shown.stage + 1;
    if next == 1 && actual > 1 && !passes_warning {
        next = 2;
    }
    Shown {
        stage: next,
        since: wall,
    }
}

fn region_styles(
    results: &[compute::RegionResult],
    alert: Option<&Alert>,
    (t, wall): (f64, f64),
    reduce_motion: bool,
    shown: &mut std::collections::HashMap<seismira_core::region::RegionId, Shown>,
    hold: bool,
    (scale, mw): (crate::mmi_style::Scale, f64),
) -> (Vec<crate::map::RegionStyle>, bool) {
    use seismira_core::region::RegionState;
    // 0–1 arası yumuşak dalga (kosinüs); "Hareketi azalt" açıkken ortada sabit.
    let pulse = if reduce_motion {
        0.5
    } else {
        (0.5 + 0.5 * (std::f64::consts::TAU * BLINK_HZ * wall).cos()) as f32
    };
    // Dolgu opaklığı: yanıp sönen %8–%30, yatışmış %12 sabit (Bölüm 9
    // "çok hafif aynı renkle dolar"). Görsel ayar.
    let blink_fill = |c: Color32| c.gamma_multiply(0.08 + 0.22 * pulse);
    let mut out = Vec::new();
    let mut blinking = false;
    for r in results {
        let s = &r.summary;
        // Yaygın değerin seçili ölçekteki karşılığı (renk bununla).
        let typical = region_level(scale, mw, s.typical, s.pgv_typical);
        let t_alert = alert.filter(|a| a.regions.contains(&s.id)).map(|a| a.t);
        let end = s.t_end.is_finite().then_some(s.t_end);
        // Sunum: gerçek durum → en az ~1 s görünen aşama (Bölüm 9).
        let actual = s.state(t, t_alert, end);
        let actual_stage = match actual {
            RegionState::Quiet => 0,
            RegionState::Warning => 1,
            RegionState::Shaking(_) => 2,
            RegionState::Settled(_) => 3,
        };
        // Uyarı, sarsıntı başlamadan önce (P ulaştıktan sonra) görünüyor mu?
        let passes_warning =
            t_alert.is_some_and(|a| a.max(s.t_first_p) < s.class_times[Shindo::S1 as usize]);
        let entry = shown.entry(s.id).or_insert(Shown {
            stage: actual_stage,
            since: wall,
        });
        *entry = advance_shown(*entry, actual_stage, passes_warning, wall, hold);
        // Gösterilen aşama gerçeğin gerisindeyse çizim sürmeli.
        blinking |= entry.stage != actual_stage;
        let visible = match entry.stage {
            0 => RegionState::Quiet,
            1 => RegionState::Warning,
            2 => RegionState::Shaking(s.typical_at(t).unwrap_or(Shindo::S1)),
            _ => RegionState::Settled(s.typical),
        };
        // (sıra, sınır rengi, kalınlık, dolgu, yanıp sönüyor mu)
        let style = match visible {
            RegionState::Quiet => None,
            // Uyarıda beyaz dolgu daha da hafif (%4–%14): kapsam geniş
            // olabilir (ör. 41 il) ve tüm haritayı grileştirmesin.
            RegionState::Warning => Some((
                0u8,
                Color32::WHITE,
                2.0,
                Color32::WHITE.gamma_multiply(0.04 + 0.10 * pulse),
                true,
            )),
            RegionState::Shaking(now) => {
                let c = typical.rising(now, s.typical).color();
                // Nihai en yüksek Shindo'su 3'ün altındaki bölgeler yanıp sönmez.
                if s.max >= Shindo::S3 {
                    Some((1 + now as u8, c, 2.5, blink_fill(c), true))
                } else {
                    Some((1 + now as u8, c, 1.5, Color32::TRANSPARENT, false))
                }
            }
            RegionState::Settled(max) => {
                let c = typical.color();
                let fill = if max >= Shindo::S3 {
                    c.gamma_multiply(0.12)
                } else {
                    Color32::TRANSPARENT
                };
                Some((1 + max as u8, c, 2.0, fill, false))
            }
        };
        if let Some((order, line, width, fill, blink)) = style {
            blinking |= blink && !reduce_motion;
            out.push((
                order,
                crate::map::RegionStyle {
                    id: s.id,
                    line,
                    line_width: width,
                    fill,
                },
            ));
        }
    }
    // Güçlü olan üstte (ortak sınırlarda).
    out.sort_by_key(|(o, _)| *o);
    (out.into_iter().map(|(_, s)| s).collect(), blinking)
}

/// Bölge değerinin seçili ölçekteki karşılığı: Shindo ızgaradan, MMI
/// aynı hücrelerin büyük bileşen PGV'sinden (Worden vd. 2012; mesafe
/// katsayısı sıfır, Rrup etkisiz; ızgara gösterimiyle aynı kural).
fn region_level(
    scale: crate::mmi_style::Scale,
    mw: f64,
    shindo: Shindo,
    pgv: f32,
) -> crate::mmi_style::Level {
    let mmi = if pgv.is_finite() && pgv > 0.0 {
        seismira_core::mmi::mmi_from_pgv_larger(f64::from(pgv), mw, 10.0)
    } else {
        1.0
    };
    crate::mmi_style::Level::of(scale, shindo, mmi)
}

/// Şehir bildirimleri (bildirim akışında yer alır).
#[derive(Clone, Copy, Debug)]
enum CityEventKind {
    PArrived,
    Shindo(Shindo),
}

#[derive(Clone, Copy, Debug)]
struct CityEvent {
    t: f64,
    city: usize,
    /// Bölgenin (il/eyalet) en kalabalık şehri; bildirimde bölge adı yerine
    /// kullanılır (admin1 adları GeoNames'te İngilizce ve bazen boş).
    region_main: usize,
    kind: CityEventKind,
}

/// Varsayımsal erken uyarı (Bölüm 7.3; `seismira_core::warning`).
#[derive(Clone, Debug)]
struct Alert {
    t: f64,
    /// Uyarı kapsamındaki bölgeler, en güçlüden başlayarak.
    regions: Vec<seismira_core::region::RegionId>,
}

/// Uyarı yayımlanır mı, hangi bölgelere? (JMA ölçütü)
fn early_warning(
    results: &[compute::RegionResult],
    tt: &seismira_core::traveltime::TravelTimes,
    depth_km: f64,
) -> Option<Alert> {
    use seismira_core::warning;
    let max = results.iter().map(|r| r.summary.max).max()?;
    if !warning::issued(max) {
        return None;
    }
    let t = warning::System::DENSE.alert_time(tt, depth_km)?;
    let mut covered: Vec<_> = results
        .iter()
        .filter(|r| warning::region_covered(r.summary.max))
        .collect();
    covered.sort_by(|a, b| {
        b.summary
            .max
            .cmp(&a.summary.max)
            .then(a.summary.t_first_s.total_cmp(&b.summary.t_first_s))
    });
    Some(Alert {
        t,
        regions: covered.iter().map(|r| r.summary.id).collect(),
    })
}

/// Bölge Shindo bildirimi (ızgaradan; bölge kimliği rasterı ile).
#[derive(Clone, Copy, Debug)]
struct RegionEvent {
    /// Bölgenin en yüksek Shindo'ya ulaştığı an.
    t: f64,
    region: seismira_core::region::RegionId,
    shindo: Shindo,
    /// Bölgenin en yüksek PGV'si (MMI gösterimi için).
    pgv: f32,
    best_city: Option<usize>,
    at: (f64, f64),
}

/// Bölge bildirimleri: en yüksek Shindo'su 5- ve üstü olanlar ile en güçlü
/// 3 bölge (daha düşükler haritada görünür). Kimlik sırasından bağımsız.
fn region_events(results: &[compute::RegionResult]) -> Vec<RegionEvent> {
    let mut v: Vec<_> = results
        .iter()
        .filter(|r| r.summary.max >= Shindo::S1)
        .collect();
    v.sort_by(|a, b| {
        b.summary
            .max
            .cmp(&a.summary.max)
            .then(a.summary.t_peak.total_cmp(&b.summary.t_peak))
    });
    v.iter()
        .enumerate()
        .take_while(|(k, r)| *k < 3 || r.summary.max >= Shindo::S5Lower)
        .map(|(_, r)| RegionEvent {
            t: r.summary.t_peak,
            region: r.summary.id,
            shindo: r.summary.max,
            pgv: r.summary.pgv_max,
            best_city: r.best_city,
            at: r.summary.at,
        })
        .collect()
}

/// Bildirim akışına girecek şehir olayları. Kalabalığı önlemek için il /
/// eyalet (GeoNames admin1) başına toplanır (Bölüm 10 örnekleri de bölge
/// düzeyindedir; GeoNames büyük şehirlerin ilçelerini ayrı şehir sayar):
/// - P varışı: hissedilen bölgelerin en kalabalık şehri, nüfusu ≥ 1 milyon
///   ya da başkentse; en erken 10 bölge.
/// - Shindo: bölgedeki en yüksek Shindo; 5- ve üstü bölgeler ile en güçlü
///   3 bölge (daha düşükler haritada ve şehir kartında görünür). Olay
///   zamanı, o şehrin kutusunun son değere vardığı an (S varışından sonra
///   kademeli yükselme; fiziksel tepe anı t_tepe, `SiteResult::rise`).
fn city_events(layer: &CityLayer, results: &CityResults) -> Vec<CityEvent> {
    use std::collections::HashMap;
    let jma = |i: usize| results.sites[i].motion.map(|m| m.jma).unwrap_or(f64::MIN);
    // bölge → (en kalabalık şehir, en güçlü sarsılan şehir)
    let mut regions: HashMap<([u8; 2], &str), (usize, usize)> = HashMap::new();
    for (i, r) in results.sites.iter().enumerate() {
        if !r.motion.is_some_and(|m| m.shindo() >= Shindo::S1) {
            continue;
        }
        let c = &layer.cities[i];
        let key = (
            c.country,
            if c.admin1.is_empty() {
                c.name.as_str()
            } else {
                c.admin1.as_str()
            },
        );
        let e = regions.entry(key).or_insert((i, i));
        if c.population > layer.cities[e.0].population {
            e.0 = i;
        }
        if jma(i) > jma(e.1) {
            e.1 = i;
        }
    }
    let mut out = Vec::new();
    let mut big: Vec<usize> = regions
        .values()
        .map(|&(main, _)| main)
        .filter(|&i| layer.cities[i].population >= 1_000_000 || layer.cities[i].capital)
        .collect();
    let tp = |i: usize| results.sites[i].t_p.unwrap_or(f64::MAX);
    big.sort_by(|&a, &b| tp(a).total_cmp(&tp(b)));
    for &i in big.iter().take(10) {
        if let Some(t) = results.sites[i].t_p {
            out.push(CityEvent {
                t,
                city: i,
                region_main: i,
                kind: CityEventKind::PArrived,
            });
        }
    }
    let mut strong: Vec<(usize, usize)> = regions.values().copied().collect();
    strong.sort_by(|a, b| jma(b.1).total_cmp(&jma(a.1)));
    for (k, &(main, best)) in strong.iter().enumerate() {
        let r = &results.sites[best];
        let Some(m) = r.motion else { continue };
        if k >= 3 && m.shindo() < Shindo::S5Lower {
            break;
        }
        if let Some(t) = r.t_peak() {
            out.push(CityEvent {
                t,
                city: best,
                region_main: main,
                kind: CityEventKind::Shindo(m.shindo()),
            });
        }
    }
    out.sort_by(|a, b| a.t.total_cmp(&b.t));
    out
}

pub struct SeismiraApp {
    lang: Lang,
    assets: PathBuf,
    show_text_test: bool,
    /// "Veri Kaynakları ve Lisanslar" penceresi.
    show_sources: bool,
    sources: Result<Vec<crate::sources::Source>, String>,
    /// Eğitim uyarısı. Kapatılabilir ama her açılışta yeniden görünür
    /// (bilerek kaydedilmez); Görünüm menüsünden geri açılır.
    show_disclaimer: bool,
    /// Sarsıntı alanı (ızgara) katmanı görünsün mü (Görünüm menüsü).
    show_grid: bool,
    /// "Hareketi azalt": yanıp sönme yok, sabit kalın çizgi (Bölüm 3).
    reduce_motion: bool,
    /// Simülasyon sonunda modelin fay kırılma izi (Bölüm 2.8; varsayılan
    /// kapalı, Görünüm menüsü, kaydedilir).
    show_fault_trace: bool,
    /// Kalite ayarı (Görünüm menüsü; kaydedilir).
    quality: compute::Quality,
    /// Haritadaki şiddet ölçeği (Görünüm menüsü; kaydedilir).
    scale: crate::mmi_style::Scale,
    /// Sıvılaşma katmanı (Bölüm 2.6; varsayılan açık, Görünüm menüsü,
    /// kaydedilir).
    show_ground: bool,
    /// Heyelan katmanı (Bölüm 2.6; varsayılan açık, Görünüm menüsü,
    /// kaydedilir; veri yoksa menüde görünmez).
    show_landslide: bool,
    /// Tsunami katmanı ve bildirimleri (Bölüm 2.7; varsayılan açık,
    /// Görünüm menüsü, kaydedilir).
    show_tsunami: bool,
    map: MapWidget,
    /// Sağ paneldeki taslak deprem.
    draft: panel::Draft,
    /// Hazır senaryolar (data/scenarios/scenarios.json; yoksa boş).
    presets: Vec<crate::scenarios::Preset>,
    /// Çalışan simülasyon; taslak değişince iptal edilir.
    sim: Option<Simulation>,
    clock: Clock,
    travel_times: Result<Arc<TravelTimes>, String>,
    cities: Result<CityLayer, String>,
    /// Vs30 haritası; yoksa referans zemin kullanılır (kartta belirtilir).
    vs30: Option<Arc<compute::Vs30>>,
    /// Slab2 levha geometrisi (~2 MB, açılışta okunur; zonlar ilk
    /// kullanımda açılır). Yoksa dalma-batmada küresel model.
    slab: Option<Arc<Slab2>>,
    /// Kararlı kıta içi bölgeler (STREC; yoksa uyarı çıkmaz, günlükte not).
    stable: Option<seismira_core::tectonic::StableRegions>,
    /// ESRM20 bina stoku ve kırılganlık (~0,3 MB, açılışta okunur).
    exposure: Option<Arc<seismira_core::exposure::Exposure>>,
    /// GEM paketi (isteğe bağlı, CC BY-NC-SA; data/gem/gem.svul).
    vuln: Option<Arc<seismira_core::damage::VulnCurves>>,
    /// Son hesabın tahmini bina hasarı.
    damage: Option<compute::DamageResults>,
    /// Zemin yenilmesi girdileri (`data/ground/`); yoksa katman gösterilmez.
    ground: Option<Arc<compute::Ground>>,
    /// Tsunami verisi (`data/tsunami/`); yoksa tsunami hesaplanmaz.
    tsunami_data: Option<Arc<crate::tsunami::TsunamiData>>,
    /// Son hesabın tsunami tahmini.
    tsunami: Option<crate::tsunami::TsunamiResult>,
    /// Bölge kartı için bölge başına tsunami özeti: (en yüksek kategori,
    /// en yüksek tahmin, ilk varış).
    tsunami_regions: std::collections::HashMap<
        seismira_core::region::RegionId,
        (crate::tsunami::Category, Option<f64>, f64),
    >,
    /// Son hesabın zemin yenilmesi sonuçları ([`compute::GroundKind`]
    /// sırası): bölge özetleri (kimliğe göre sıralı; bölge kartı) ve
    /// haritadaki kırmızı kareler (belirme anına göre sıralı).
    ground_failure: [(Vec<compute::RegionGroundFailure>, Vec<compute::GroundBox>); 2],
    /// Bölge kimliği rasterı (il / eyalet); yoksa bölge bildirimleri
    /// GeoNames admin1 adlarıyla şehirlerden üretilir.
    regions: Option<Arc<compute::Regions>>,
    region_events: Vec<RegionEvent>,
    /// Bölge sonuçları (durum ve vurgu için) ve varsayımsal erken uyarı.
    region_results: Vec<compute::RegionResult>,
    alert: Option<Alert>,
    /// Şehir hesabı (arka planda) ve sonuçları.
    job: Option<compute::Job>,
    results: Option<CityResults>,
    ranked: Vec<usize>,
    city_events: Vec<CityEvent>,
    /// İmlecin altındaki nokta (boylam, enlem).
    hover: Option<(f64, f64)>,
    /// Açık bölge kartı: bölge ve tıklanan nokta (kart oraya bağlı durur).
    region_card: Option<(seismira_core::region::RegionId, (f64, f64))>,
    /// Bölgelerin ekranda gösterilen aşaması (en az ~1 s görünme kuralı).
    region_shown: std::collections::HashMap<seismira_core::region::RegionId, Shown>,
}

/// Başlangıç görünümü. Geliştirme için `SEISMIRA_VIEW=boylam,enlem,ölçek`.
fn initial_view() -> MapView {
    let parsed = std::env::var("SEISMIRA_VIEW").ok().and_then(|s| {
        let v: Vec<f64> = s.split(',').filter_map(|x| x.trim().parse().ok()).collect();
        (v.len() == 3).then(|| (v[0], v[1], v[2]))
    });
    // Varsayılan: Avrupa, Afrika ve Asya'nın batısı görünür.
    let (lon, lat, scale) = parsed.unwrap_or((30.0, 30.0, 1800.0));
    let (x, y) = mercator::to_unit(lon, lat);
    MapView::new([x, y], scale)
}

impl SeismiraApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let lang = cc
            .storage
            .and_then(|s| s.get_string(STORAGE_LANG))
            .and_then(|code| Lang::from_code(&code))
            .unwrap_or_else(Lang::from_system);
        let assets = paths::assets_dir();
        log::info!("veri klasörü: {}", assets.display());
        theme::apply(&cc.egui_ctx);
        fonts::install(&cc.egui_ctx, &assets, lang);
        // Küçük tablo (~0.5 MB): açılışta, ilk kareden önce okunur.
        let tt_path = paths::data_dir().join("traveltime").join("ak135.ttt");
        let travel_times = TravelTimes::load(&tt_path).map(Arc::new).map_err(|e| {
            log::error!("{}: {e}", tt_path.display());
            format!("{}: {e}", tt_path.display())
        });
        // Şehir listesi (~1.3 MB) de açılışta okunur.
        let cities = CityLayer::load(&paths::data_dir().join("cities").join("cities15000.scty"));
        if let Err(e) = &cities {
            log::error!("{e}");
        }
        // Vs30 (~150 MB) belleğe eşlenir; açılışta yalnızca başlık okunur.
        let vs30 = compute::load_vs30(&paths::data_dir().join("vs30").join("vs30.sras"))
            .inspect_err(|e| log::warn!("Vs30 haritası yok, referans zemin kullanılacak: {e}"))
            .ok();
        let ground = vs30.as_ref().and_then(|v| {
            compute::Ground::load(&paths::data_dir().join("ground"), v)
                .inspect_err(|e| log::warn!("zemin yenilmesi girdileri yok: {e}"))
                .ok()
        });
        let tsunami_data = crate::tsunami::TsunamiData::load(&paths::data_dir().join("tsunami"))
            .inspect_err(|e| log::warn!("tsunami verisi yok: {e}"))
            .ok();
        let regions = compute::Regions::load(&paths::data_dir().join("regions"))
            .inspect_err(|e| log::warn!("bölge verisi yok: {e}"))
            .ok();
        let exposure_path = paths::data_dir().join("exposure").join("esrm20.sexp");
        let exposure = std::fs::File::open(&exposure_path)
            .and_then(|f| seismira_core::exposure::Exposure::read(&mut std::io::BufReader::new(f)))
            .map(Arc::new)
            .inspect_err(|e| {
                log::warn!("{}: {e} (bina hasarı gösterilmez)", exposure_path.display())
            })
            .ok();
        let gem_path = paths::data_dir().join("gem").join("gem.svul");
        let vuln = std::fs::File::open(&gem_path)
            .and_then(|f| {
                seismira_core::vulnerability::VulnModel::read(&mut std::io::BufReader::new(f))
            })
            .map(|m| Arc::new(seismira_core::damage::VulnCurves::new(&m)))
            .inspect_err(|e| {
                log::info!(
                    "{}: {e} (GEM paketi yok; Avrupa dışı hasar gösterilmez)",
                    gem_path.display()
                )
            })
            .ok();
        let presets =
            crate::scenarios::load(&paths::data_dir().join("scenarios").join("scenarios.json"))
                .inspect_err(|e| log::warn!("hazır senaryolar yok: {e}"))
                .unwrap_or_default();
        let slab = Slab2::load(&paths::data_dir().join("slab2").join("slab2.sslb"))
            .map(Arc::new)
            .inspect_err(|e| log::warn!("Slab2 yok, dalma-batmada küresel model: {e}"))
            .ok();
        // 17 KB: arayüz iş parçacığında okunması sorun değil.
        let stable_path = paths::data_dir().join("regions").join("stable.sout");
        let stable = std::fs::File::open(&stable_path)
            .and_then(|f| {
                seismira_core::tectonic::StableRegions::read(&mut std::io::BufReader::new(f))
            })
            .inspect_err(|e| log::warn!("{}: {e}", stable_path.display()))
            .ok();
        Self {
            lang,
            assets,
            show_text_test: false,
            show_sources: std::env::var("SEISMIRA_SOURCES").is_ok_and(|v| v == "1"),
            sources: crate::sources::load(&paths::data_dir().join("SOURCES.toml"))
                .inspect_err(|e| log::error!("veri kaynakları okunamadı: {e}")),
            show_disclaimer: true,
            show_grid: true,
            reduce_motion: cc
                .storage
                .and_then(|s| s.get_string(STORAGE_REDUCE_MOTION))
                .is_some_and(|v| v == "1"),
            show_fault_trace: cc
                .storage
                .and_then(|s| s.get_string(STORAGE_FAULT_TRACE))
                .is_some_and(|v| v == "1"),
            quality: cc
                .storage
                .and_then(|s| s.get_string(STORAGE_QUALITY))
                .and_then(|v| compute::Quality::from_code(&v))
                .unwrap_or_default(),
            scale: cc
                .storage
                .and_then(|s| s.get_string(STORAGE_SCALE))
                .and_then(|v| crate::mmi_style::Scale::from_code(&v))
                .unwrap_or_default(),
            show_ground: cc
                .storage
                .and_then(|s| s.get_string(STORAGE_GROUND_FAILURE))
                .is_none_or(|v| v == "1"),
            show_landslide: cc
                .storage
                .and_then(|s| s.get_string(STORAGE_LANDSLIDE))
                .is_none_or(|v| v == "1"),
            show_tsunami: cc
                .storage
                .and_then(|s| s.get_string(STORAGE_TSUNAMI))
                .is_none_or(|v| v == "1"),
            map: MapWidget::new(paths::data_dir(), initial_view()),
            draft: panel::Draft::default(),
            presets,
            sim: None,
            clock: Clock::default(),
            travel_times,
            cities,
            vs30,
            slab,
            stable,
            exposure,
            vuln,
            damage: None,
            ground,
            ground_failure: Default::default(),
            tsunami_data,
            tsunami: None,
            tsunami_regions: Default::default(),
            regions,
            region_events: Vec::new(),
            region_results: Vec::new(),
            alert: None,
            job: None,
            results: None,
            ranked: Vec::new(),
            city_events: Vec::new(),
            hover: None,
            region_card: None,
            region_shown: Default::default(),
        }
        .with_demo(&cc.egui_ctx)
    }

    /// Geliştirme: `SEISMIRA_DEMO=boylam,enlem,Mw,derinlik,saniye[,doğrultu]` ile depremi
    /// (ve isteğe bağlı `SEISMIRA_DEMO_SETTING=interface[,eğim]`)
    /// başlatıp verilen anda duraklatır (ekran görüntüsü testleri için).
    fn with_demo(mut self, ctx: &egui::Context) -> Self {
        // `SEISMIRA_QUALITY=low|medium|high|ultra`: kalite (kayıtlı ayarı
        // ezer; ölçüm için).
        if let Some(q) = std::env::var("SEISMIRA_QUALITY")
            .ok()
            .and_then(|v| compute::Quality::from_code(&v))
        {
            self.quality = q;
        }
        // `SEISMIRA_SCALE=shindo|mmi`: haritadaki şiddet ölçeği.
        if let Some(s) = std::env::var("SEISMIRA_SCALE")
            .ok()
            .and_then(|v| crate::mmi_style::Scale::from_code(&v))
        {
            self.scale = s;
        }
        // `SEISMIRA_PRESET=kimlik`: hazır senaryoyu seçili açar (panel
        // yerleşimi ekran görüntüsü için; başlatmaz).
        if let Ok(id) = std::env::var("SEISMIRA_PRESET")
            && let Some(p) = self.presets.iter().find(|p| p.id == id)
        {
            self.draft.quake = p.quake;
            self.draft.has_epicenter = true;
            self.draft.preset = Some(p.id.clone());
        }
        let Some(v) = std::env::var("SEISMIRA_DEMO").ok().map(|s| {
            s.split(',')
                .filter_map(|x| x.trim().parse::<f64>().ok())
                .collect::<Vec<_>>()
        }) else {
            return self;
        };
        let (strike, v) = match v[..] {
            [a, b, c, d, e, strike] => (strike, vec![a, b, c, d, e]),
            _ => (0.0, v),
        };
        if let [lon, lat, mw, depth, t] = v[..] {
            let q = &mut self.draft.quake;
            q.strike = strike;
            q.epicenter = (lon, lat);
            q.mw = mw;
            q.depth_km = depth;
            if depth > quake::limits::CRUST_DEPTH.1 {
                q.setting = quake::Setting::Intraslab;
            }
            // `SEISMIRA_DEMO_SETTING=interface[,eğim]`: dalma-batma arayüzü
            // (ters fay; eğim verilmezse ters fayın varsayılanı).
            if let Ok(s) = std::env::var("SEISMIRA_DEMO_SETTING") {
                let mut it = s.split(',');
                if it.next() == Some("interface") {
                    q.setting = quake::Setting::Interface;
                    q.fault = quake::FaultType::Reverse;
                    let (dip, rake) = q.fault.default_dip_rake();
                    q.dip = it.next().and_then(|d| d.trim().parse().ok()).unwrap_or(dip);
                    q.rake = rake;
                }
            }
            self.draft.has_epicenter = true;
            // `SEISMIRA_DEMO_TRACE=1`: fay izi açık (kayıtlı ayara dokunmadan).
            if std::env::var("SEISMIRA_DEMO_TRACE").is_ok_and(|v| v == "1") {
                self.show_fault_trace = true;
            }
            self.start_sim(ctx);
            self.clock.t = t;
            self.clock.playing = false;
        }
        self
    }

    /// Geliştirme: `SEISMIRA_CARD=şehir adı` o şehrin kartını imleç olmadan
    /// gösterir (ekran görüntüsü testleri için).
    fn dev_card(&self, rect: egui::Rect) -> Option<(usize, egui::Pos2)> {
        let name = std::env::var("SEISMIRA_CARD").ok()?;
        let layer = self.cities.as_ref().ok()?;
        let i = layer.cities.iter().position(|c| c.name == name)?;
        let c = &layer.cities[i];
        Some((i, self.map.to_screen(rect, (c.lon, c.lat))))
    }

    fn set_lang(&mut self, ctx: &egui::Context, lang: Lang) {
        if lang != self.lang {
            self.lang = lang;
            fonts::install(ctx, &self.assets, lang);
        }
    }

    fn menu_bar(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        let t = |k| lang.t(k);
        let mut new_lang = self.lang;
        egui::MenuBar::new().ui(ui, |ui| {
            ui.label(RichText::new(t("app-title")).strong());
            ui.separator();
            ui.menu_button(t("menu-view"), |ui| {
                ui.checkbox(&mut self.show_disclaimer, t("menu-disclaimer"));
                ui.checkbox(&mut self.show_grid, t("menu-shake-grid"));
                ui.checkbox(&mut self.show_fault_trace, t("menu-fault-trace"));
                ui.checkbox(&mut self.show_ground, t("menu-liquefaction"));
                if self.ground.as_ref().is_some_and(|g| g.has_landslide()) {
                    ui.checkbox(&mut self.show_landslide, t("menu-landslide"));
                }
                if self.tsunami_data.is_some() {
                    ui.checkbox(&mut self.show_tsunami, t("menu-tsunami"));
                }
                ui.checkbox(&mut self.reduce_motion, t("menu-reduce-motion"));
                ui.menu_button(t("menu-scale"), |ui| {
                    use crate::mmi_style::Scale;
                    ui.radio_value(&mut self.scale, Scale::Shindo, t("scale-shindo"));
                    ui.radio_value(&mut self.scale, Scale::Mmi, t("scale-mmi"));
                    ui.label(RichText::new(t("scale-note")).small().weak());
                });
                ui.menu_button(t("menu-quality"), |ui| {
                    let before = self.quality;
                    for q in compute::Quality::ALL {
                        ui.radio_value(&mut self.quality, q, t(q.key()));
                    }
                    ui.label(RichText::new(t("quality-note")).small().weak());
                    if self.quality != before {
                        // Açık simülasyon yeni kaliteyle yeniden hesaplanır
                        // (saat ve oynatma durumu korunur).
                        let ctx = ui.ctx().clone();
                        self.spawn_job(&ctx);
                    }
                });
                ui.checkbox(&mut self.show_text_test, t("menu-text-test"));
            });
            ui.menu_button(t("menu-language"), |ui| {
                for lang in Lang::ALL {
                    ui.radio_value(&mut new_lang, lang, lang.endonym());
                }
            });
            ui.menu_button(t("menu-help"), |ui| {
                if ui.button(t("menu-sources")).clicked() {
                    self.show_sources = true;
                }
            });
        });
        let ctx = ui.ctx().clone();
        self.set_lang(&ctx, new_lang);
    }
}

impl SeismiraApp {
    fn map_area(&mut self, ui: &mut egui::Ui, frame: &eframe::Frame) {
        let lang = self.lang;
        let rect = ui.max_rect();
        if self.sim.is_some() {
            let wall = ui.input(|i| i.time);
            let (styles, blinking) = region_styles(
                &self.region_results,
                self.alert.as_ref(),
                (self.clock.t, wall),
                self.reduce_motion,
                &mut self.region_shown,
                self.clock.playing,
                (self.scale, self.sim.as_ref().map_or(0.0, |s| s.quake.mw)),
            );
            self.map.set_region_styles(styles);
            if blinking {
                ui.ctx().request_repaint();
            }
        }
        self.map.set_grid_time(
            self.sim
                .as_ref()
                .filter(|_| self.show_grid)
                .map(|_| self.clock.t),
        );
        let mw = self.sim.as_ref().map_or(0.0, |s| s.quake.mw);
        self.map.set_intensity_scale(self.scale, mw);
        let out = self.map.ui(ui, frame, crate::MSAA);
        if let Some(pts) = &mut self.draft.drawing {
            // Fay izi çizimi: tık nokta ekler, çift tık / Enter bitirir,
            // Esc iptal, Backspace son noktayı siler (metin kutusu odakta
            // değilken).
            let typing = ui.ctx().memory(|m| m.focused().is_some());
            let (enter, esc, back) = ui.input(|i| {
                (
                    i.key_pressed(egui::Key::Enter),
                    i.key_pressed(egui::Key::Escape),
                    i.key_pressed(egui::Key::Backspace),
                )
            });
            if out.double_clicked || (enter && !typing) {
                // Çift tıkın ikinci tıkı nokta eklemez: ilk tık son noktadır.
                if self.draft.finish_drawing() {
                    self.cancel_sim();
                }
            } else if esc && !typing {
                self.draft.drawing = None;
            } else {
                if back && !typing {
                    pts.pop();
                }
                if let Some(c) = out.clicked
                    && pts.len() < seismira_core::quake::MAX_TRACE_POINTS
                {
                    pts.push(c);
                }
            }
        } else if let Some(c) = out.clicked {
            if self.sim.is_some() {
                // Simülasyon sürerken tıklama onu bozmaz: bölge kartını açar
                // ya da kapatır (Bölüm 9). Yeni merkez üssü için önce
                // "Simülasyonu kapat".
                self.toggle_region_card(c);
            } else {
                self.draft.quake.epicenter = c;
                self.draft.has_epicenter = true;
                // Merkez üssü değişti: artık hazır senaryo değil.
                self.draft.preset = None;
                self.cancel_sim();
            }
        }
        self.hover = out.hover;

        let felt = self.results.as_ref().and_then(|r| r.felt_radius_deg);
        if let Some(sim) = &self.sim {
            let (p, s) = sim.fronts(self.clock.t);
            let center = sim.quake.epicenter;
            for (front, color) in [(s, S_COLOR), (p, P_COLOR)] {
                if let Front::At(deg) = front
                    && deg > 0.0
                {
                    // Hissedilme sınırının ötesinde soluk (Bölüm 7.2).
                    let color = if felt.is_some_and(|f| deg > f) {
                        color.gamma_multiply(0.35)
                    } else {
                        color
                    };
                    let pts = geo::ring(center, deg, RING_POINTS);
                    self.map
                        .draw_lonlat_line(ui, rect, &pts, Stroke::new(2.0, color));
                }
            }
        } else if let Some(pts) = &self.draft.drawing {
            self.draw_trace_preview(ui, rect, pts, out.hover);
        } else if self.draft.has_epicenter {
            self.draw_fault(ui, rect, &self.draft.quake, false);
        }
        // Simülasyon bittiyse ve istendiyse modelin fay kırılma izi.
        if let Some(sim) = &self.sim
            && self.show_fault_trace
            && self.clock.t >= sim.end_time()
        {
            self.draw_fault(ui, rect, &sim.quake, true);
        }

        // Model geçerlilik sınırı (kesikli): ötesinde şiddet tahmini yok.
        if self.sim.is_some()
            && let Some(limit) = self.results.as_ref().and_then(|r| r.model_limit.as_ref())
        {
            self.map.draw_lonlat_dashed(
                ui,
                rect,
                limit,
                Stroke::new(1.5, city_layer::MODEL_LIMIT_COLOR),
            );
        }

        // Şehirler, P işaretleri ve Shindo kutuları.
        let hover_pos = ui
            .input(|i| i.pointer.hover_pos())
            .filter(|p| rect.contains(*p));
        let mut hovered_city = None;
        if let Ok(layer) = &self.cities {
            let sim_state = match (&self.sim, &self.results) {
                (Some(_), Some(r)) => Some((r, self.ranked.as_slice(), self.clock.t)),
                _ => None,
            };
            hovered_city = layer
                .draw(
                    ui, rect, &self.map, sim_state, P_COLOR, hover_pos, self.scale,
                )
                .hovered;
        }

        if let Some(sim) = &self.sim {
            self.map.draw_epicenter(ui, rect, sim.quake.epicenter);
            self.legend(ui, rect);
            let gmm = seismira_core::shaking::Gmm::for_quake(&sim.quake);
            if let (Some(_), Some(gmm)) = (&self.results, gmm) {
                let limit = lang.t_args("model-limit", &[("km", lang.num(gmm.max_rrup_km(), 0))]);
                let title = match self.scale {
                    crate::mmi_style::Scale::Shindo => lang.t("shindo-legend"),
                    crate::mmi_style::Scale::Mmi => lang.t("mmi-legend"),
                };
                city_layer::draw_legend(ui, rect, &title, Some(&limit), self.scale);
            }
            self.draw_ground_failure(ui, rect);
            self.draw_tsunami(ui, rect);
        } else if self.draft.has_epicenter {
            self.map
                .draw_epicenter(ui, rect, self.draft.quake.epicenter);
        }
        // Geliştirme: `SEISMIRA_REGION=boylam,enlem` o noktadaki bölgenin
        // kartını tıklama olmadan açar (ekran görüntüsü testleri için).
        if self.region_card.is_none()
            && let Some(v) = std::env::var("SEISMIRA_REGION").ok().map(|s| {
                s.split(',')
                    .filter_map(|x| x.trim().parse::<f64>().ok())
                    .collect::<Vec<_>>()
            })
            && let [lon, lat] = v[..]
        {
            self.toggle_region_card((lon, lat));
        }
        if let Some((id, at)) = self.region_card {
            self.draw_region_card(ui, rect, id, self.map.to_screen(rect, at));
        }
        if let (Some(i), Some(p)) = (hovered_city, hover_pos) {
            self.city_card(ui, rect, i, p);
        } else if let Some((i, p)) = self.dev_card(rect) {
            self.city_card(ui, rect, i, p);
        }

        let painter = ui.painter_at(rect);
        let weak = ui.visuals().weak_text_color();
        if !self.map.is_ready() {
            let msg = if self.map.errors().is_empty() {
                lang.t("map-loading")
            } else {
                format!(
                    "{}\n\npython3 scripts/prepare_data.py",
                    lang.t("map-missing")
                )
            };
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                msg,
                FontId::proportional(15.0),
                weak,
            );
        }
        // Sol altta imlecin koordinatı.
        if let Some((lon, lat)) = self.hover {
            let galley = painter.layout_no_wrap(
                lang.lonlat(lon, lat),
                FontId::proportional(13.0),
                ui.visuals().text_color(),
            );
            let pos = rect.left_bottom() + egui::vec2(10.0, -10.0 - galley.size().y);
            let bg = egui::Rect::from_min_size(pos, galley.size()).expand(4.0);
            painter.rect_filled(bg, 4.0, theme::MAP_BG.gamma_multiply(0.85));
            painter.galley(pos, galley, weak);
        }
    }
}

/// Harita üstünde bilgi kartı: satırlar ve (varsa) sol üstte şiddet kutusu
/// (seçili ölçekte).
/// Kart `at` noktasının sağ altına, sığmazsa öbür yanına yerleşir.
fn paint_card(
    ui: &egui::Ui,
    rect: egui::Rect,
    at: egui::Pos2,
    lines: Vec<(String, Color32)>,
    level: Option<crate::mmi_style::Level>,
) {
    let text = ui.visuals().text_color();
    let painter = ui.painter_at(rect);
    let font = FontId::proportional(13.0);
    let galleys: Vec<_> = lines
        .into_iter()
        .map(|(t, c)| painter.layout_no_wrap(t, font.clone(), c))
        .collect();
    let w = galleys.iter().map(|g| g.size().x).fold(0.0, f32::max);
    let h: f32 = galleys.iter().map(|g| g.size().y + 2.0).sum();
    let badge = level.map_or(0.0, |l| shindo_style::box_size(l.label()).x + 6.0);
    let mut origin = at + egui::vec2(14.0, 14.0);
    let size = egui::vec2(w + badge + 16.0, h + 12.0);
    if origin.x + size.x > rect.right() {
        origin.x = at.x - 14.0 - size.x;
    }
    if origin.y + size.y > rect.bottom() {
        origin.y = at.y - 14.0 - size.y;
    }
    let card = egui::Rect::from_min_size(origin, size);
    painter.rect_filled(card, 6.0, ui.visuals().window_fill);
    painter.rect_stroke(
        card,
        6.0,
        ui.visuals().window_stroke,
        egui::StrokeKind::Inside,
    );
    if let Some(l) = level {
        let size = shindo_style::box_size(l.label());
        shindo_style::draw_labeled_box(
            &painter,
            card.left_top() + egui::vec2(8.0, 8.0) + size / 2.0,
            l.color(),
            l.label(),
        );
    }
    let mut y = card.top() + 6.0;
    for g in galleys {
        let gh = g.size().y;
        painter.galley(egui::pos2(card.left() + 8.0 + badge, y), g, text);
        y += gh + 2.0;
    }
}

fn quake_has_gmpe(q: &quake::Quake) -> bool {
    seismira_core::shaking::has_gmpe(q)
}

impl SeismiraApp {
    /// Fay düzlemi: üst kenar ve eğimli fayda yüzey izdüşümü.
    /// - Önizleme (`trace = false`, simülasyondan önce): ince ve soluk.
    ///   Bölüm 6: bu, sonuçtaki kırılma izi DEĞİLDİR.
    /// - Kırılma izi (`trace = true`, simülasyon sonunda, isteğe bağlı;
    ///   Bölüm 2.8): hesapta kullanılan sonlu fay, belirgin. İlerleyiş
    ///   animasyonu YOK (Bölüm 2).
    fn draw_fault(&self, ui: &egui::Ui, rect: egui::Rect, quake: &quake::Quake, trace: bool) {
        let rupture = seismira_core::rupture::Rupture::from_quake(quake);
        let (outline, edge) = if trace {
            (
                Stroke::new(1.5, Color32::from_rgba_unmultiplied(255, 255, 255, 150)),
                Stroke::new(3.0, Color32::WHITE),
            )
        } else {
            (
                Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 60)),
                Stroke::new(1.5, Color32::from_rgba_unmultiplied(255, 255, 255, 170)),
            )
        };
        let densify = |a: (f64, f64), b: (f64, f64)| -> Vec<(f64, f64)> {
            let mut b = b;
            if (b.0 - a.0).abs() > 180.0 {
                b.0 -= 360.0 * (b.0 - a.0).signum();
            }
            (0..=24)
                .map(|k| {
                    let f = f64::from(k) / 24.0;
                    (a.0 + (b.0 - a.0) * f, a.1 + (b.1 - a.1) * f)
                })
                .collect()
        };
        // Her parçanın yüzey izdüşümü (eğimliyse) ve üst kenar izi.
        for f in &rupture.segments {
            if f.dip < 89.0 {
                let c = f.surface_outline_lonlat();
                let mut ring = Vec::new();
                for k in 0..4 {
                    ring.extend(densify(c[k], c[(k + 1) % 4]));
                }
                self.map.draw_lonlat_line(ui, rect, &ring, outline);
            }
        }
        let tr = rupture.trace_lonlat();
        let mut line = Vec::new();
        for w in tr.windows(2) {
            line.extend(densify(w[0], w[1]));
        }
        self.map.draw_lonlat_line(ui, rect, &line, edge);
    }

    /// Çizilmekte olan fay izi: noktalar, aralarındaki çizgi ve son noktadan
    /// imlece kesikli lastik çizgi.
    fn draw_trace_preview(
        &self,
        ui: &egui::Ui,
        rect: egui::Rect,
        pts: &[(f64, f64)],
        hover: Option<(f64, f64)>,
    ) {
        let edge = Stroke::new(2.5, Color32::WHITE);
        if pts.len() >= 2 {
            self.map.draw_lonlat_line(ui, rect, pts, edge);
        }
        if let (Some(&last), Some(h)) = (pts.last(), hover) {
            self.map.draw_lonlat_dashed(
                ui,
                rect,
                &[last, h],
                Stroke::new(1.5, Color32::from_white_alpha(170)),
            );
        }
        let painter = ui.painter_at(rect);
        for &p in pts {
            let c = self.map.to_screen(rect, p);
            painter.circle_filled(c, 5.0, Color32::from_black_alpha(160));
            painter.circle_filled(c, 3.5, Color32::WHITE);
        }
        // İmleç hareket ettikçe lastik çizgi güncellensin.
        if hover.is_some() {
            ui.ctx().request_repaint();
        }
    }

    /// Şehrin nihai şiddeti seçili ölçekte (sonuç yoksa `None`).
    fn city_level(&self, i: usize) -> Option<crate::mmi_style::Level> {
        let m = self.results.as_ref()?.sites[i].motion?;
        Some(crate::mmi_style::Level::of(self.scale, m.shindo(), m.mmi))
    }

    /// Şehir kartı (fare üzerindeyken): Shindo ve olası aralık, PGA, PGV,
    /// varış zamanları, S − P. Tam şehir kartı Faz 4'te.
    fn city_card(&self, ui: &egui::Ui, rect: egui::Rect, i: usize, at: egui::Pos2) {
        let lang = self.lang;
        let Ok(layer) = &self.cities else { return };
        let c = &layer.cities[i];
        let mut lines: Vec<(String, Color32)> = Vec::new();
        let text = ui.visuals().text_color();
        let weak = ui.visuals().weak_text_color();
        let place = if c.admin1.is_empty() {
            String::from_utf8_lossy(&c.country).into_owned()
        } else {
            format!("{}, {}", c.admin1, String::from_utf8_lossy(&c.country))
        };
        lines.push((c.name.clone(), text));
        lines.push((place, weak));
        let mut level = None;
        if let (Some(sim), Some(res)) = (&self.sim, &self.results) {
            let r = &res.sites[i];
            match r.motion {
                Some(m) => {
                    let (lo, hi) = m.shindo_range();
                    level = Some(crate::mmi_style::Level::of(self.scale, m.shindo(), m.mmi));
                    // Seçili ölçeğin satırı önce (kutuyla aynı ölçek).
                    let first = lines.len();
                    lines.push((
                        lang.t_args(
                            "tip-shindo",
                            &[
                                ("s", m.shindo().to_string()),
                                ("i", lang.num(seismira_core::intensity::round_jma(m.jma), 1)),
                                ("lo", lo.to_string()),
                                ("hi", hi.to_string()),
                            ],
                        ),
                        text,
                    ));
                    let (mmi_lo, mmi_hi) = m.mmi_range();
                    let mmi_line = (
                        lang.t_args(
                            "tip-mmi",
                            &[
                                ("roman", seismira_core::mmi::roman(m.mmi).to_owned()),
                                ("v", lang.num(m.mmi, 1)),
                                ("lo", seismira_core::mmi::roman(mmi_lo).to_owned()),
                                ("hi", seismira_core::mmi::roman(mmi_hi).to_owned()),
                            ],
                        ),
                        text,
                    );
                    match self.scale {
                        crate::mmi_style::Scale::Shindo => lines.push(mmi_line),
                        crate::mmi_style::Scale::Mmi => lines.insert(first, mmi_line),
                    }
                    lines.push((
                        lang.t_args("tip-pga", &[("v", lang.num(m.pga.median() * 100.0, 1))]),
                        text,
                    ));
                    lines.push((
                        lang.t_args("tip-pgv", &[("v", lang.num(m.pgv.median(), 1))]),
                        text,
                    ));
                }
                None if quake_has_gmpe(&sim.quake) => {
                    lines.push((lang.t("tip-outside-model"), weak))
                }
                None => lines.push((lang.t("no-gmpe-short"), weak)),
            }
            if let (Some(tp), Some(ts)) = (r.t_p, r.t_s) {
                lines.push((
                    lang.t_args(
                        "tip-arrivals",
                        &[
                            ("p", timeline::format_hms(tp)),
                            ("s", timeline::format_hms(ts)),
                            ("sp", lang.num(ts - tp, 1)),
                        ],
                    ),
                    text,
                ));
            }
            // Erken uyarı süresi (uyarı yayımlandıysa ve şehirde Shindo ≥ 4).
            if let (Some(alert), Some(ts), Some(m)) = (&self.alert, r.t_s, r.motion)
                && seismira_core::warning::region_covered(m.shindo())
            {
                let lead = seismira_core::warning::lead_time(ts, alert.t);
                lines.push(if lead > 0.0 {
                    (
                        lang.t_args("tip-lead-time", &[("s", lang.num(lead, 1))]),
                        text,
                    )
                } else {
                    (lang.t("tip-blind-zone"), text)
                });
            }
            lines.push((
                lang.t_args("tip-distance", &[("d", lang.num(r.dist.rrup, 0))]),
                weak,
            ));
            if r.motion.is_some() {
                use seismira_core::site::Vs30Source;
                let v = lang.num(r.site.vs30, 0);
                let key = match r.site.source {
                    Vs30Source::Map => "tip-vs30-map",
                    Vs30Source::Nearby => "tip-vs30-nearby",
                    Vs30Source::Reference => "tip-vs30-reference",
                    Vs30Source::Built => "tip-vs30-built",
                };
                lines.push((lang.t_args(key, &[("v", v)]), weak));
                if r.backarc {
                    lines.push((lang.t("tip-backarc"), text));
                }
                if let Some([m, lo, hi]) = self
                    .damage
                    .as_ref()
                    .and_then(|d| d.cities.get(i).copied().flatten())
                {
                    let heavy = |d: [f64; 4]| d[2] + d[3];
                    lines.push((
                        lang.t_args(
                            "tip-damage",
                            &[
                                ("p", fmt_percent(lang, heavy(m))),
                                ("lo", fmt_percent(lang, heavy(lo))),
                                ("hi", fmt_percent(lang, heavy(hi))),
                            ],
                        ),
                        text,
                    ));
                    lines.push((lang.t("tip-damage-note"), weak));
                }
                if let (Some(sd), Some(effect)) = (r.site.vs30_ln_sd, r.jma_vs30_effect) {
                    lines.push((
                        lang.t_args(
                            "tip-vs30-sd",
                            &[("f", lang.num(sd.exp(), 2)), ("d", lang.num(effect, 1))],
                        ),
                        weak,
                    ));
                }
                if (r.vs30_used - r.site.vs30).abs() > 0.5 {
                    lines.push((
                        lang.t_args("tip-vs30-clamped", &[("v", lang.num(r.vs30_used, 0))]),
                        weak,
                    ));
                }
            }
        }
        paint_card(ui, rect, at, lines, level);
    }

    /// `t`'den sonraki ilk bildirimin zamanı ("sonraki olaya atla"):
    /// çekirdek olayları, erken uyarı, bölge ve şehir bildirimleri.
    fn next_event_after(&self, t: f64) -> Option<f64> {
        let sim = self.sim.as_ref()?;
        sim.next_event_after(t)
            .into_iter()
            .chain(self.alert.iter().map(|a| a.t))
            .chain(self.region_events.iter().map(|e| e.t))
            .chain(self.city_events.iter().map(|e| e.t))
            .chain(
                self.tsunami
                    .iter()
                    .filter(|_| self.show_tsunami)
                    .flat_map(|ts| {
                        std::iter::once(crate::tsunami::ISSUE_S)
                            .chain(ts.region_arrivals().into_iter().map(|r| r.t))
                    }),
            )
            .filter(|&et| et > t + 1e-6)
            .min_by(f64::total_cmp)
    }

    /// Tıklanan noktadaki bölgenin kartını açar; aynı bölgeye ya da sonucu
    /// olmayan bir yere (deniz, model dışı) tıklanınca kapatır.
    fn toggle_region_card(&mut self, lonlat: (f64, f64)) {
        let id = self
            .regions
            .as_ref()
            .and_then(|r| r.map.id_at(lonlat.0, lonlat.1).ok())
            .filter(|id| self.region_results.iter().any(|r| r.summary.id == *id));
        self.region_card = match (id, self.region_card) {
            (Some(id), Some((open, _))) if id == open => None,
            (Some(id), _) => Some((id, lonlat)),
            (None, _) => None,
        };
    }

    /// Bölge kartı (Bölüm 10): en yüksek ve yaygın Shindo, varışlar, erken
    /// uyarı. Değerler tahminin son hâlidir (şehir kartı gibi). Tahmini
    /// hasar Faz 6'da, tsunami Faz 8'de eklenecek.
    fn draw_region_card(
        &self,
        ui: &egui::Ui,
        rect: egui::Rect,
        id: seismira_core::region::RegionId,
        at: egui::Pos2,
    ) {
        let lang = self.lang;
        let (Some(regions), Some(res)) = (
            &self.regions,
            self.region_results.iter().find(|r| r.summary.id == id),
        ) else {
            return;
        };
        let Some(info) = regions.info(id) else { return };
        let s = &res.summary;
        let text = ui.visuals().text_color();
        let weak = ui.visuals().weak_text_color();
        let name = match lang {
            Lang::Tr => info.name_tr.clone(),
            Lang::En => info.name_en.clone(),
        };
        let mut lines = vec![(name, text), (info.iso.clone(), weak)];
        let mw = self.sim.as_ref().map_or(0.0, |s| s.quake.mw);
        let max = region_level(self.scale, mw, s.max, s.pgv_max);
        let typical = region_level(self.scale, mw, s.typical, s.pgv_typical);
        // "En yüksek: şehir" yalnızca şehrin değeri bölgeninkine eşitse
        // (bölge değeri ızgaradan gelir; bildirimlerle aynı kural).
        let city = self
            .cities
            .as_ref()
            .ok()
            .zip(res.best_city)
            .filter(|&(_, i)| self.city_level(i) == Some(max))
            .map(|(l, i)| l.cities[i].name.clone());
        lines.push((
            match city {
                Some(city) => {
                    lang.t_args("region-max-city", &[("s", max.text(lang)), ("city", city)])
                }
                None => lang.t_args("region-max", &[("s", max.text(lang))]),
            },
            text,
        ));
        lines.push((
            lang.t_args("region-typical", &[("s", typical.text(lang))]),
            text,
        ));
        lines.push((
            lang.t_args(
                "region-arrivals",
                &[
                    ("p", timeline::format_hms(s.t_first_p)),
                    ("s", timeline::format_hms(s.t_first_s)),
                ],
            ),
            text,
        ));
        if let Some(alert) = &self.alert {
            lines.push(if !alert.regions.contains(&id) {
                (lang.t("region-not-covered"), weak)
            } else {
                let lead = seismira_core::warning::lead_time(s.t_first_s, alert.t);
                if lead > 0.0 {
                    (
                        lang.t_args("region-lead", &[("s", lang.num(lead, 1))]),
                        text,
                    )
                } else {
                    (lang.t("region-blind"), text)
                }
            });
        }
        // Tsunami: bölgenin kıyısındaki en yüksek kategori, tahmin, ilk varış.
        if self.show_tsunami
            && self.clock.t >= crate::tsunami::ISSUE_S
            && let Some(&(cat, h, eta)) = self.tsunami_regions.get(&id)
        {
            let h = h.map_or_else(
                || lang.t("tsunami-no-height"),
                |m| {
                    lang.t_args(
                        "tsunami-height",
                        &[("m", lang.num(m, if m < 10.0 { 1 } else { 0 }))],
                    )
                },
            );
            lines.push((
                lang.t_args(
                    "region-tsunami",
                    &[
                        ("cat", lang.t(cat.key())),
                        ("h", h),
                        ("eta", timeline::format_hms(eta)),
                    ],
                ),
                text,
            ));
        }
        // Sıvılaşma ve heyelan: bölgede ilk kare belirdiyse (katman açıksa).
        for kind in compute::GroundKind::ALL {
            let (show, key) = match kind {
                compute::GroundKind::Liquefaction => (self.show_ground, "region-liquefaction"),
                compute::GroundKind::Landslide => (self.show_landslide, "region-landslide"),
            };
            let regions = &self.ground_failure[kind as usize].0;
            if show
                && let Ok(k) = regions.binary_search_by_key(&id, |r| r.id)
                && self.clock.t >= regions[k].t_first
            {
                let fmt = |km2: f64| {
                    if km2 < 10.0 {
                        lang.num(km2, 1)
                    } else {
                        lang.num(km2, 0)
                    }
                };
                let [m, lo, hi] = regions[k].km2;
                lines.push((
                    lang.t_args(key, &[("km2", fmt(m)), ("lo", fmt(lo)), ("hi", fmt(hi))]),
                    text,
                ));
            }
        }
        match self.damage.as_ref().and_then(|d| d.region(id)) {
            Some(d) if d.buildings > 0.0 => {
                let heavy = |x: &[f64; 4]| x[2] + x[3];
                let pct = |x: f64| fmt_percent(lang, x / d.buildings);
                lines.push((
                    lang.t_args("region-damage-head", &[("n", fmt_count(lang, d.buildings))]),
                    weak,
                ));
                let h = heavy(&d.median);
                lines.push((
                    if h < 1.0 {
                        lang.t("region-damage-heavy-none")
                    } else {
                        lang.t_args(
                            "region-damage-heavy",
                            &[
                                ("n", fmt_count(lang, h)),
                                ("p", pct(h)),
                                ("lo", pct(heavy(&d.low))),
                                ("hi", pct(heavy(&d.high))),
                            ],
                        )
                    },
                    text,
                ));
                if d.median[1] >= 1.0 {
                    lines.push((
                        lang.t_args(
                            "region-damage-moderate",
                            &[("n", fmt_count(lang, d.median[1])), ("p", pct(d.median[1]))],
                        ),
                        text,
                    ));
                }
            }
            _ => match self.damage.as_ref().and_then(|d| d.loss(id)) {
                Some(l) if l.cost > 0.0 => {
                    let pct = |x: f64| fmt_percent(lang, x);
                    let [m, lo, hi] = l.ratio;
                    lines.push((lang.t("region-loss-head"), weak));
                    lines.push((
                        if m < 0.0005 {
                            lang.t("region-loss-none")
                        } else {
                            lang.t_args(
                                "region-loss-ratio",
                                &[("p", pct(m)), ("lo", pct(lo)), ("hi", pct(hi))],
                            )
                        },
                        text,
                    ));
                }
                _ if self.damage.is_some() => {
                    lines.push((
                        lang.t(if self.vuln.is_some() {
                            "region-damage-nodata"
                        } else {
                            "region-damage-nodata-nogem"
                        }),
                        weak,
                    ));
                }
                _ => {}
            },
        }
        paint_card(ui, rect, at, lines, Some(max));
    }

    /// Simülasyonun sonu: S cephesi ya da son tsunami varışı (+10 dk).
    fn sim_end(&self) -> Option<f64> {
        let sim = self.sim.as_ref()?;
        let tsu = self
            .tsunami
            .as_ref()
            .and_then(crate::tsunami::TsunamiResult::last_eta)
            .map_or(0.0, |t| t + 600.0);
        Some(sim.end_time().max(tsu))
    }

    /// Tsunami kıyı şeridi, ipucu ve açıklama (Bölüm 2.7): uyarı anından
    /// (depremden ~3 dk sonra) itibaren, JMA renkleriyle, 1 Hz TAM yanıp
    /// söner (açık/kapalı, [`crate::ground_style::blink`]; "Hareketi
    /// azalt"ta sabit). Komşu kıyı noktaları kalın
    /// bir bantla birleştirilir (kullanıcı isteği, 2026-11-23); iki farklı
    /// kategorili nokta arasında renk parçanın ortasında değişir.
    fn draw_tsunami(&self, ui: &egui::Ui, rect: egui::Rect) {
        use crate::tsunami::{Category, ISSUE_S};
        let Some(ts) = &self.tsunami else {
            return;
        };
        if !self.show_tsunami || self.clock.t < ISSUE_S || ts.coast.is_empty() {
            return;
        }
        let lang = self.lang;
        let (alpha, next_toggle) =
            crate::ground_style::blink(ui.input(|i| i.time), self.reduce_motion);
        let painter = ui.painter_at(rect);
        let hover = ui
            .input(|i| i.pointer.hover_pos())
            .filter(|p| rect.contains(*p));
        let mut best: Option<(f32, &crate::tsunami::CoastNote)> = None;
        // Kalınlık kategoriye göre (renk tek bilgi taşıyıcı olmasın;
        // Faz 10 adım 6): dikkat 3, uyarı 4,5, büyük uyarı 6 point.
        let inner = |c: Category| match c {
            Category::Advisory => 3.0,
            Category::Warning => 4.5,
            Category::Major => 6.0,
        };
        const OUTER: f32 = 9.0;
        let shadow = Color32::from_black_alpha((150.0 * alpha) as u8);
        let ll = |i: u32| {
            let c = &ts.coast[i as usize];
            (c.lon, c.lat)
        };
        // Parçalar (ekran uçları, iki ucun kategorisi), görünen kopyalarda.
        let segs: Vec<([egui::Pos2; 2], Category, Category)> = ts
            .links
            .iter()
            .flat_map(|&(a, b)| {
                let (ca, cb) = (ts.coast[a as usize].category, ts.coast[b as usize].category);
                self.map
                    .segment_positions(rect, ll(a), ll(b))
                    .into_iter()
                    .map(move |s| (s, ca, cb))
            })
            .collect();
        let points: Vec<(egui::Pos2, &crate::tsunami::CoastNote)> = ts
            .coast
            .iter()
            .flat_map(|c| {
                self.map
                    .screen_positions(rect, (c.lon, c.lat))
                    .into_iter()
                    .map(move |p| (p, c))
            })
            .collect();
        // Önce koyu kenar (bütün bant), sonra renkler; düşük kategoriler
        // önce, yüksekler üstte kalsın. Uçlardaki daireler eklemleri
        // yuvarlatır ve bağlantısız noktaları da gösterir.
        for (s, _, _) in &segs {
            painter.line_segment(*s, Stroke::new(OUTER, shadow));
        }
        for (p, _) in &points {
            painter.circle_filled(*p, OUTER / 2.0, shadow);
        }
        for cat in [Category::Advisory, Category::Warning, Category::Major] {
            let stroke = Stroke::new(inner(cat), cat.color().gamma_multiply(alpha));
            for &([p, q], ca, cb) in &segs {
                let m = p.lerp(q, 0.5);
                if ca == cat {
                    painter.line_segment([p, if cb == cat { q } else { m }], stroke);
                }
                if cb == cat && ca != cat {
                    painter.line_segment([m, q], stroke);
                }
            }
            for &(p, c) in points.iter().filter(|(_, c)| c.category == cat) {
                painter.circle_filled(p, inner(cat) / 2.0, stroke.color);
                if let Some(h) = hover {
                    let d = h.distance(p);
                    if d < 8.0 && best.is_none_or(|(bd, _)| d < bd) {
                        best = Some((d, c));
                    }
                }
            }
        }
        // Açıklama: sol altta.
        let font = FontId::proportional(12.0);
        let rows: Vec<(Option<Category>, String)> = vec![
            (None, lang.t("tsunami-legend-title")),
            (Some(Category::Major), lang.t("tsunami-legend-major")),
            (Some(Category::Warning), lang.t("tsunami-legend-warning")),
            (Some(Category::Advisory), lang.t("tsunami-legend-advisory")),
            (None, lang.t("tsunami-legend-note")),
        ];
        let galleys: Vec<_> = rows
            .iter()
            .enumerate()
            .map(|(k, (_, t))| {
                let c = if k == rows.len() - 1 {
                    ui.visuals().weak_text_color()
                } else {
                    ui.visuals().text_color()
                };
                painter.layout_no_wrap(t.clone(), font.clone(), c)
            })
            .collect();
        let w = galleys.iter().map(|g| g.size().x).fold(0.0, f32::max) + 26.0;
        let h = galleys.len() as f32 * 17.0 + 10.0;
        let bg = egui::Rect::from_min_size(
            egui::pos2(rect.left() + 12.0, rect.bottom() - 48.0 - h),
            egui::vec2(w, h),
        );
        painter.rect_filled(bg, 4.0, theme::MAP_BG.gamma_multiply(0.85));
        let mut y = bg.top() + 5.0;
        for ((color, _), g) in rows.iter().zip(galleys) {
            let x = bg.left() + 6.0;
            if let Some(c) = color {
                let (a, b) = (egui::pos2(x, y + 8.0), egui::pos2(x + 12.0, y + 8.0));
                painter.line_segment([a, b], Stroke::new(OUTER, Color32::from_black_alpha(150)));
                painter.line_segment([a, b], Stroke::new(inner(*c), c.color()));
            }
            painter.galley(egui::pos2(x + 18.0, y), g, Color32::WHITE);
            y += 17.0;
        }
        // İpucu: en yakın nokta.
        if let Some((_, c)) = best
            && let Some(hp) = hover
        {
            let place = c
                .place
                .as_ref()
                .map_or_else(|| lang.t("tsunami-unnamed"), |(n, r)| format!("{n} ({r})"));
            let height = c.height.map_or_else(
                || lang.t("tsunami-no-height"),
                |m| {
                    lang.t_args(
                        "tsunami-height",
                        &[("m", lang.num(m, if m < 10.0 { 1 } else { 0 }))],
                    )
                },
            );
            let text = lang.t_args(
                "tsunami-tip",
                &[
                    ("place", place),
                    ("cat", lang.t(c.category.key())),
                    ("h", height),
                    ("eta", timeline::format_hms(c.eta_s)),
                ],
            );
            let g = painter.layout_no_wrap(text, font, Color32::WHITE);
            let r = egui::Rect::from_min_size(hp + egui::vec2(14.0, 10.0), g.size()).expand(5.0);
            painter.rect_filled(r, 4.0, theme::MAP_BG.gamma_multiply(0.95));
            painter.galley(r.min + egui::vec2(5.0, 5.0), g, Color32::WHITE);
        }
        // Yalnızca açılma/kapanma anında yeniden çiz (Bölüm 11).
        if next_toggle.is_finite() {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_secs_f64(next_toggle));
        }
    }

    /// Bu türün katmanı Görünüm menüsünde açık mı?
    fn ground_shown(&self, kind: compute::GroundKind) -> bool {
        match kind {
            compute::GroundKind::Liquefaction => self.show_ground,
            compute::GroundKind::Landslide => self.show_landslide,
        }
    }

    /// Türün harfi ve adı (kare köşesi, açıklama, bildirim).
    fn ground_label(&self, kind: compute::GroundKind) -> (String, String) {
        let lang = self.lang;
        match kind {
            compute::GroundKind::Liquefaction => (
                lang.t("ground-letter-liquefaction"),
                lang.t("ground-name-liquefaction"),
            ),
            compute::GroundKind::Landslide => (
                lang.t("ground-letter-landslide"),
                lang.t("ground-name-landslide"),
            ),
        }
    }

    /// Belirmiş kırmızı kareler ve açıklaması (Bölüm 2.6).
    fn draw_ground_failure(&self, ui: &egui::Ui, rect: egui::Rect) {
        let now = self.clock.t;
        let alpha = crate::ground_style::pulse(ui.input(|i| i.time), self.reduce_motion);
        let mut legend = Vec::new();
        let labels: Vec<(String, String)> = compute::GroundKind::ALL
            .iter()
            .map(|&k| self.ground_label(k))
            .collect();
        let mut boxes = Vec::new();
        for kind in compute::GroundKind::ALL {
            if !self.ground_shown(kind) {
                continue;
            }
            let letter = labels[kind as usize].0.as_str();
            let n0 = boxes.len();
            boxes.extend(
                self.ground_failure[kind as usize]
                    .1
                    .iter()
                    .take_while(|b| b.t <= now)
                    .map(|b| {
                        (
                            b.bounds,
                            alpha,
                            letter,
                            kind == compute::GroundKind::Landslide,
                        )
                    }),
            );
            if boxes.len() > n0 {
                legend.push(labels[kind as usize].clone());
            }
        }
        if boxes.is_empty() {
            return;
        }
        self.map.draw_ground_boxes(ui, rect, &boxes);
        crate::ground_style::draw_legend(ui, rect, self.lang, &legend);
        if !self.reduce_motion {
            ui.ctx().request_repaint();
        }
    }

    fn cancel_sim(&mut self) {
        self.region_card = None;
        self.job = None; // Drop → arka plan hesabı iptal
        self.map.set_grid(None);
        self.ground_failure = Default::default();
        self.tsunami = None;
        self.tsunami_regions.clear();
        self.map.set_region_lines(None);
        self.map.set_region_styles(Vec::new());
        self.region_shown.clear();
        self.results = None;
        self.ranked.clear();
        self.city_events.clear();
        self.region_events.clear();
        self.region_results.clear();
        self.damage = None;
        self.alert = None;
        self.sim = None;
        self.clock = Clock {
            speed: self.clock.speed,
            ..Clock::default()
        };
    }

    fn start_sim(&mut self, ctx: &egui::Context) {
        let Ok(tt) = &self.travel_times else {
            return;
        };
        let tt = tt.clone();
        if quake::validate(&self.draft.quake)
            .iter()
            .any(|i| i.blocks())
        {
            log::warn!("geçersiz girdiyle başlatma engellendi");
            return;
        }
        self.cancel_sim();
        self.sim = Simulation::new(self.draft.quake, &tt);
        self.spawn_job(ctx);
        self.clock = Clock {
            t: 0.0,
            playing: self.sim.is_some(),
            speed: self.clock.speed,
        };
    }

    /// Açık simülasyonun arka plan hesabını (yeniden) başlatır; eski iş
    /// düşürülünce iptal edilir, yeni sonuçlar geldikçe eskilerin yerine
    /// geçer.
    fn spawn_job(&mut self, ctx: &egui::Context) {
        let Ok(tt) = &self.travel_times else {
            return;
        };
        let tt = tt.clone();
        if let Some(sim) = &self.sim
            && let Ok(layer) = &self.cities
        {
            let data = compute::Datasets {
                cities: layer.cities.clone(),
                city_sites: layer.sites.clone(),
                tt,
                vs30: self.vs30.clone(),
                regions: self.regions.clone(),
                slab: self.slab.clone(),
                exposure: self.exposure.clone(),
                vuln: self.vuln.clone(),
                ground: self.ground.clone(),
                tsunami: self.tsunami_data.clone(),
                quality: self.quality,
            };
            self.job = Some(compute::Job::spawn(sim.quake, data, ctx.clone()));
        }
    }

    /// Sağ üstte dalga renk açıklaması.
    fn legend(&self, ui: &egui::Ui, rect: egui::Rect) {
        let painter = ui.painter_at(rect);
        let font = FontId::proportional(13.0);
        let mut y = rect.top() + 12.0;
        let right = rect.right() - 12.0;
        for (key, color) in [("wave-p", P_COLOR), ("wave-s", S_COLOR)] {
            let galley =
                painter.layout_no_wrap(self.lang.t(key), font.clone(), ui.visuals().text_color());
            let text_pos = egui::pos2(right - galley.size().x, y);
            let line_y = y + galley.size().y / 2.0;
            let bg = egui::Rect::from_min_max(
                egui::pos2(text_pos.x - 30.0, y),
                egui::pos2(right, y + galley.size().y),
            )
            .expand(4.0);
            painter.rect_filled(bg, 4.0, theme::MAP_BG.gamma_multiply(0.85));
            painter.line_segment(
                [
                    egui::pos2(text_pos.x - 26.0, line_y),
                    egui::pos2(text_pos.x - 6.0, line_y),
                ],
                Stroke::new(2.0, color),
            );
            painter.galley(text_pos, galley, ui.visuals().text_color());
            y += 24.0;
        }
    }

    fn event_text(&self, sim: &Simulation, e: &Event) -> String {
        let lang = self.lang;
        let limit = || lang.num(sim.max_distance(), 0);
        match e.kind {
            EventKind::Origin => lang.t_args(
                "event-origin",
                &[
                    ("mw", lang.num(sim.quake.mw, 1)),
                    ("depth", lang.num(sim.quake.depth_km, 0)),
                ],
            ),
            EventKind::Surfaced(Phase::P) => lang.t("event-surfaced-p"),
            EventKind::Surfaced(Phase::S) => lang.t("event-surfaced-s"),
            EventKind::ReachedLimit(Phase::P) => lang.t_args("event-limit-p", &[("deg", limit())]),
            EventKind::ReachedLimit(Phase::S) => lang.t_args("event-limit-s", &[("deg", limit())]),
        }
    }

    /// Sol panel: bildirim akışı, en yenisi üstte. Tıklanınca harita oraya gider.
    fn notifications(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        ui.add_space(6.0);
        ui.heading(lang.t("panel-notifications"));
        let Some(sim) = &self.sim else {
            ui.label(RichText::new(lang.t("notifications-empty")).weak());
            return;
        };
        // Çekirdek olayları ve şehir olayları tek akışta, zamana göre.
        struct Note {
            t: f64,
            text: String,
            color: Color32,
            badge: Option<crate::mmi_style::Level>,
            at: (f64, f64),
        }
        let mw = sim.quake.mw;
        let now = self.clock.t;
        let mut notes: Vec<Note> = sim
            .events_until(now)
            .iter()
            .map(|e| Note {
                t: e.t,
                text: self.event_text(sim, e),
                color: match e.kind {
                    EventKind::Origin => Color32::from_rgb(0xFF, 0x6B, 0x6B),
                    EventKind::Surfaced(_) => ui.visuals().text_color(),
                    EventKind::ReachedLimit(_) => ui.visuals().weak_text_color(),
                },
                badge: None,
                at: e.location,
            })
            .collect();
        if let Ok(layer) = &self.cities {
            for e in self.city_events.iter().filter(|e| e.t <= now) {
                let c = &layer.cities[e.city];
                let (text, badge) = match e.kind {
                    CityEventKind::PArrived => (
                        lang.t_args("city-p-arrived", &[("city", c.name.clone())]),
                        None,
                    ),
                    CityEventKind::Shindo(s) => {
                        let level = self
                            .city_level(e.city)
                            .unwrap_or(crate::mmi_style::Level::Shindo(s));
                        let main = &layer.cities[e.region_main];
                        // Bölge adı: il/eyalet (GeoNames admin1); yoksa
                        // bölgenin en kalabalık şehri.
                        let region = if c.admin1.is_empty() {
                            main.name.clone()
                        } else {
                            c.admin1.clone()
                        };
                        let text = if region == c.name {
                            lang.t_args(
                                "city-shindo",
                                &[("city", c.name.clone()), ("shindo", level.text(lang))],
                            )
                        } else {
                            lang.t_args(
                                "region-shindo",
                                &[
                                    ("region", region),
                                    ("city", c.name.clone()),
                                    ("shindo", level.text(lang)),
                                ],
                            )
                        };
                        (text, Some((level, s)))
                    }
                };
                notes.push(Note {
                    t: e.t,
                    text,
                    // Vurgu: bildirim seçimiyle aynı Shindo ölçütü.
                    color: if badge.is_some_and(|(_, s)| s >= Shindo::S5Lower) {
                        Color32::from_rgb(0xFF, 0x9A, 0x6B)
                    } else {
                        ui.visuals().text_color()
                    },
                    badge: badge.map(|(l, _)| l),
                    at: (c.lon, c.lat),
                });
            }
        }
        if let Some(regions) = &self.regions {
            for e in self.region_events.iter().filter(|e| e.t <= now) {
                let Some(info) = regions.info(e.region) else {
                    continue;
                };
                let region = match lang {
                    Lang::Tr => info.name_tr.clone(),
                    Lang::En => info.name_en.clone(),
                };
                let level = region_level(self.scale, mw, e.shindo, e.pgv);
                // "En yüksek: şehir" yalnızca şehrin değeri bölgenin en
                // yüksek değerine eşitse (bölge değeri ızgaradan gelir).
                let city = self
                    .cities
                    .as_ref()
                    .ok()
                    .zip(e.best_city)
                    .filter(|&(_, i)| self.city_level(i) == Some(level))
                    .map(|(l, i)| l.cities[i].name.clone())
                    .filter(|c| *c != region);
                let text = match city {
                    Some(city) => lang.t_args(
                        "region-shindo",
                        &[
                            ("region", region),
                            ("city", city),
                            ("shindo", level.text(lang)),
                        ],
                    ),
                    None => lang.t_args(
                        "city-shindo",
                        &[("city", region), ("shindo", level.text(lang))],
                    ),
                };
                notes.push(Note {
                    t: e.t,
                    text,
                    color: if e.shindo >= Shindo::S5Lower {
                        Color32::from_rgb(0xFF, 0x9A, 0x6B)
                    } else {
                        ui.visuals().text_color()
                    },
                    badge: Some(level),
                    at: e.at,
                });
            }
        }
        if let (Some(alert), Some(regions)) = (&self.alert, &self.regions)
            && alert.t <= now
        {
            const SHOWN: usize = 6;
            let names: Vec<String> = alert
                .regions
                .iter()
                .take(SHOWN)
                .filter_map(|&id| regions.info(id))
                .map(|i| match lang {
                    Lang::Tr => i.name_tr.clone(),
                    Lang::En => i.name_en.clone(),
                })
                .collect();
            let mut list = names.join(", ");
            if alert.regions.len() > SHOWN {
                list = lang.t_args(
                    "early-warning-more",
                    &[
                        ("regions", list),
                        ("n", (alert.regions.len() - SHOWN).to_string()),
                    ],
                );
            }
            notes.push(Note {
                t: alert.t,
                text: lang.t_args("early-warning", &[("regions", list)]),
                color: Color32::WHITE,
                badge: None,
                at: sim.quake.epicenter,
            });
        }
        // Tsunami: uyarı (~3 dk) ve il başına ilk varış.
        if let Some(ts) = self.tsunami.as_ref().filter(|_| self.show_tsunami) {
            use crate::tsunami::{Category, ISSUE_S};
            use seismira_core::tsunami::trigger::{Assessment, Reason};
            if now >= ISSUE_S {
                match ts.assessment {
                    Assessment::Information(r) => notes.push(Note {
                        t: ISSUE_S,
                        text: lang.t(match r {
                            Reason::TooSmall => "tsunami-info-small",
                            Reason::TooDeep => "tsunami-info-deep",
                            Reason::Inland => "tsunami-info-inland",
                        }),
                        color: ui.visuals().text_color(),
                        badge: None,
                        at: sim.quake.epicenter,
                    }),
                    Assessment::Threat { .. } => {
                        let arrivals = ts.region_arrivals();
                        for cat in [Category::Major, Category::Warning, Category::Advisory] {
                            let names: Vec<&str> = arrivals
                                .iter()
                                .filter(|r| r.category == cat)
                                .map(|r| r.region.as_str())
                                .collect();
                            if names.is_empty() {
                                continue;
                            }
                            const SHOWN: usize = 6;
                            let mut list = names[..names.len().min(SHOWN)].join(", ");
                            if names.len() > SHOWN {
                                list = lang.t_args(
                                    "early-warning-more",
                                    &[("regions", list), ("n", (names.len() - SHOWN).to_string())],
                                );
                            }
                            notes.push(Note {
                                t: ISSUE_S,
                                text: lang.t_args(
                                    "tsunami-issued",
                                    &[("cat", lang.t(cat.key())), ("regions", list)],
                                ),
                                color: cat.color(),
                                badge: None,
                                at: sim.quake.epicenter,
                            });
                        }
                        // Videodaki gibi yükseklik listesi: en yüksek 6 yer.
                        let mut seen = std::collections::HashSet::new();
                        let mut tops: Vec<&crate::tsunami::CoastNote> = ts
                            .coast
                            .iter()
                            .filter(|c| c.height.is_some() && c.place.is_some())
                            .collect();
                        tops.sort_by(|a, b| b.height.unwrap().total_cmp(&a.height.unwrap()));
                        let list: Vec<String> = tops
                            .into_iter()
                            .filter(|c| seen.insert(c.place.as_ref().unwrap().0.clone()))
                            .take(6)
                            .map(|c| {
                                let m = c.height.unwrap();
                                format!(
                                    "{} ({} m)",
                                    c.place.as_ref().unwrap().0,
                                    lang.num(m, if m < 10.0 { 1 } else { 0 })
                                )
                            })
                            .collect();
                        if !list.is_empty() {
                            notes.push(Note {
                                t: ISSUE_S,
                                text: lang.t_args("tsunami-heights", &[("list", list.join(", "))]),
                                color: ui.visuals().text_color(),
                                badge: None,
                                at: sim.quake.epicenter,
                            });
                        }
                        for r in arrivals.into_iter().filter(|r| r.t <= now) {
                            let h = r.height.map_or_else(String::new, |m| {
                                lang.t_args(
                                    "tsunami-height-suffix",
                                    &[("m", lang.num(m, if m < 10.0 { 1 } else { 0 }))],
                                )
                            });
                            notes.push(Note {
                                t: r.t,
                                text: lang
                                    .t_args("tsunami-arrived", &[("region", r.region), ("h", h)]),
                                color: r.category.color(),
                                badge: None,
                                at: r.at,
                            });
                        }
                    }
                    Assessment::None => {}
                }
            }
        }
        // Kırmızı kareler: her bölgede (tür başına) ilk karenin belirdiği an.
        for kind in compute::GroundKind::ALL {
            if !self.ground_shown(kind) {
                continue;
            }
            let (letter, name) = self.ground_label(kind);
            let mut seen = std::collections::HashSet::new();
            for b in self.ground_failure[kind as usize]
                .1
                .iter()
                .take_while(|b| b.t <= now)
            {
                let center = (
                    (b.bounds[0] + b.bounds[2]) / 2.0,
                    (b.bounds[1] + b.bounds[3]) / 2.0,
                );
                // Bölge rasterında yoksa (ör. kıyı hücreleri): kutuya en
                // yakın şehrin ili / eyaleti.
                let region = self
                    .regions
                    .as_ref()
                    .and_then(|r| r.info(b.region))
                    .map(|i| match lang {
                        Lang::Tr => i.name_tr.clone(),
                        Lang::En => i.name_en.clone(),
                    })
                    .or_else(|| {
                        let layer = self.cities.as_ref().ok()?;
                        let d2 = |c: &seismira_core::cities::City| {
                            let dx = (c.lon - center.0) * center.1.to_radians().cos();
                            let dy = c.lat - center.1;
                            dx * dx + dy * dy
                        };
                        let c = layer.cities.iter().min_by(|a, b| d2(a).total_cmp(&d2(b)))?;
                        Some(if c.admin1.is_empty() {
                            c.name.clone()
                        } else {
                            c.admin1.clone()
                        })
                    });
                // Bölge başına (tür içinde) yalnızca ilk kare bildirilir.
                if !seen.insert(region.clone()) {
                    continue;
                }
                let text = match region {
                    Some(region) => lang.t_args(
                        "ground-notice",
                        &[
                            ("letter", letter.clone()),
                            ("name", name.clone()),
                            ("region", region),
                        ],
                    ),
                    None => lang.t_args(
                        "ground-notice-unnamed",
                        &[("letter", letter.clone()), ("name", name.clone())],
                    ),
                };
                notes.push(Note {
                    t: b.t,
                    text,
                    color: crate::ground_style::RED,
                    badge: None,
                    at: center,
                });
            }
        }
        notes.sort_by(|a, b| b.t.total_cmp(&a.t)); // en yenisi üstte
        let mut focus = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            for n in &notes {
                let time = lang.t_args("time-elapsed", &[("t", timeline::format_hms(n.t))]);
                let r = egui::Frame::group(ui.style())
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(RichText::new(time).small().weak().monospace());
                        ui.horizontal_top(|ui| {
                            if let Some(l) = n.badge {
                                let (rect, _) = ui.allocate_exact_size(
                                    shindo_style::box_size(l.label()),
                                    egui::Sense::hover(),
                                );
                                shindo_style::draw_labeled_box(
                                    ui.painter(),
                                    rect.center(),
                                    l.color(),
                                    l.label(),
                                );
                            }
                            // Kalan genişlikte alt satıra geçsin.
                            ui.vertical(|ui| {
                                ui.add(
                                    egui::Label::new(RichText::new(&n.text).color(n.color)).wrap(),
                                );
                            });
                        });
                    })
                    .response
                    .interact(egui::Sense::click());
                if r.clicked() {
                    focus = Some(n.at);
                }
            }
        });
        if let Some(loc) = focus {
            self.map.center_on(loc);
        }
    }
}

impl eframe::App for SeismiraApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let lang = self.lang;
        let t = |k| lang.t(k);

        egui::Panel::top("menu").show(ui, |ui| {
            ui.add_space(3.0);
            self.menu_bar(ui);
            ui.add_space(1.0);
        });

        // Alt: zaman çizgisi, altında (kapatılabilir) eğitim uyarısı.
        egui::Panel::bottom("bottom")
            .resizable(false)
            .show(ui, |ui| {
                ui.add_space(4.0);
                let next = self.next_event_after(self.clock.t);
                let end = self.sim_end();
                timeline::ui(ui, lang, &mut self.clock, end, next);
                if self.show_disclaimer {
                    ui.separator();
                    egui::Sides::new().shrink_left().show(
                        ui,
                        |ui| {
                            ui.label(
                                RichText::new(format!("⚠ {}", t("disclaimer")))
                                    .color(theme::WARN_TEXT),
                            );
                        },
                        |ui| {
                            if ui
                                .small_button("✕")
                                .on_hover_text(t("disclaimer-close"))
                                .clicked()
                            {
                                self.show_disclaimer = false;
                            }
                        },
                    );
                }
                ui.add_space(2.0);
            });

        egui::Panel::right("settings")
            .default_size(300.0)
            .size_range(260.0..=420.0)
            .show(ui, |ui| {
                ui.add_space(6.0);
                ui.heading(t("panel-settings"));
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let mut issues = quake::validate(&self.draft.quake);
                    if self.draft.has_epicenter
                        && let Some(stable) = &self.stable
                    {
                        seismira_core::tectonic::check(&self.draft.quake, stable, &mut issues);
                    }
                    let can_run = self.draft.has_epicenter
                        && self.travel_times.is_ok()
                        && !issues.iter().any(|i| i.blocks());
                    let running = self.sim.is_some();
                    let slab = self.slab.as_deref();
                    match panel::ui(
                        ui,
                        lang,
                        &mut self.draft,
                        &self.presets,
                        slab,
                        &issues,
                        can_run,
                        running,
                    ) {
                        panel::Action::Changed | panel::Action::Stop => self.cancel_sim(),
                        panel::Action::Preset => {
                            self.cancel_sim();
                            self.map.center_on(self.draft.quake.epicenter);
                        }
                        panel::Action::Start => {
                            let ctx = ui.ctx().clone();
                            self.start_sim(&ctx);
                        }
                        panel::Action::None => {}
                    }
                    if self.travel_times.is_err() {
                        ui.colored_label(ui.visuals().error_fg_color, t("traveltime-missing"));
                    }
                });
            });

        egui::Panel::left("left")
            .default_size(280.0)
            .size_range(220.0..=420.0)
            .show(ui, |ui| self.notifications(ui));

        // Arka plan hesabının sonuçları (şehirler, kaba / ince ızgara).
        if let Some(job) = &self.job {
            let (msgs, done) = job.poll();
            for msg in msgs {
                match msg {
                    compute::JobMsg::Cities(results) => {
                        if let Ok(layer) = &self.cities {
                            self.ranked = layer.rank_for_boxes(&results);
                            self.city_events = city_events(layer, &results);
                            // Bölge verisi varsa Shindo bildirimleri bölge
                            // özetinden gelir (JobMsg::Regions).
                            if self.regions.is_some() {
                                self.city_events
                                    .retain(|e| matches!(e.kind, CityEventKind::PArrived));
                            }
                        }
                        self.results = Some(results);
                    }
                    compute::JobMsg::Grid(grid) => self.map.set_grid(Some(grid)),
                    compute::JobMsg::Regions(r, lines) => {
                        self.map.set_region_lines(lines);
                        self.region_events = region_events(&r);
                        self.alert = match (&self.travel_times, &self.sim) {
                            (Ok(tt), Some(sim)) => early_warning(&r, tt, sim.quake.depth_km),
                            _ => None,
                        };
                        self.region_results = r;
                    }
                    compute::JobMsg::Damage(d) => self.damage = Some(d),
                    compute::JobMsg::GroundFailure {
                        kind,
                        regions,
                        boxes,
                    } => self.ground_failure[kind as usize] = (regions, boxes),
                    compute::JobMsg::Tsunami(r) => {
                        self.tsunami_regions = self
                            .regions
                            .as_ref()
                            .map(|reg| crate::tsunami::by_region(&r, &reg.map))
                            .unwrap_or_default();
                        self.tsunami = Some(r);
                    }
                }
            }
            if done {
                self.job = None;
            }
        }

        // Simülasyon saati: duvar saatine göre ilerler; oynarken sürekli çizim.
        if let Some(end) = self.sim_end() {
            let dt = f64::from(ui.input(|i| i.stable_dt));
            self.clock.advance(dt, end);
            if self.clock.playing {
                ui.ctx().request_repaint();
            }
        }

        egui::CentralPanel::no_frame().show(ui, |ui| self.map_area(ui, frame));

        let mut open = self.show_text_test;
        egui::Window::new(t("text-test-title"))
            .open(&mut open)
            .default_pos([240.0, 48.0])
            .default_size([520.0, 700.0])
            .vscroll(true)
            .show(ui.ctx(), |ui| {
                ui.label(RichText::new(t("text-test-note")).small().weak());
                ui.separator();
                egui::Grid::new("samples")
                    .num_columns(2)
                    .spacing([16.0, 6.0])
                    .show(ui, |ui| {
                        for (name, text) in text_test::SAMPLES {
                            ui.label(RichText::new(*name).weak());
                            ui.label(RichText::new(*text).size(18.0));
                            ui.end_row();
                        }
                    });
            });
        self.show_text_test = open;

        let mut open = self.show_sources;
        egui::Window::new(t("sources-title"))
            .open(&mut open)
            .default_pos([300.0, 60.0])
            .default_size([620.0, 640.0])
            .show(ui.ctx(), |ui| crate::sources::ui(ui, lang, &self.sources));
        self.show_sources = open;

        #[cfg(feature = "screenshot")]
        // Arka plan hesabı (ızgara, bölgeler) bitmeden görüntü alınmasın.
        crate::devshot::tick(ui.ctx(), self.map.is_ready() && self.job.is_none());
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        storage.set_string(STORAGE_LANG, self.lang.code().to_owned());
        storage.set_string(
            STORAGE_REDUCE_MOTION,
            if self.reduce_motion { "1" } else { "0" }.to_owned(),
        );
        storage.set_string(
            STORAGE_FAULT_TRACE,
            if self.show_fault_trace { "1" } else { "0" }.to_owned(),
        );
        storage.set_string(
            STORAGE_GROUND_FAILURE,
            if self.show_ground { "1" } else { "0" }.to_owned(),
        );
        storage.set_string(
            STORAGE_LANDSLIDE,
            if self.show_landslide { "1" } else { "0" }.to_owned(),
        );
        storage.set_string(
            STORAGE_TSUNAMI,
            if self.show_tsunami { "1" } else { "0" }.to_owned(),
        );
        storage.set_string(STORAGE_QUALITY, self.quality.code().to_owned());
        storage.set_string(STORAGE_SCALE, self.scale.code().to_owned());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::Harness;

    #[test]
    fn bolge_gorseli_en_az_bir_saniye_kalir() {
        let at = |stage, since| Shown { stage, since };
        // Sessizden uyarıya hemen geçilir (sessiz aşama beklemez).
        let s = advance_shown(at(0, 10.0), 3, true, 10.0, true);
        assert_eq!(s, at(1, 10.0));
        // Uyarı 1 s dolmadan değişmez; dolunca sarsıntıya, sonra yatışmışa.
        assert_eq!(advance_shown(s, 3, true, 10.9, true), s);
        let s = advance_shown(s, 3, true, 11.0, true);
        assert_eq!(s, at(2, 11.0));
        assert_eq!(advance_shown(s, 3, true, 11.5, true), s);
        assert_eq!(advance_shown(s, 3, true, 12.1, true), at(3, 12.1));
        // Uyarıdan hiç geçmeyen bölge doğrudan sarsıntıyı gösterir.
        assert_eq!(advance_shown(at(0, 0.0), 3, false, 5.0, true), at(2, 5.0));
        // Gerçek durum uyarıysa uyarı gösterilir (atlanmaz).
        assert_eq!(advance_shown(at(0, 0.0), 1, false, 5.0, true), at(1, 5.0));
        // Geri sarma ve duraklatılmışken: gerçek aşama hemen.
        assert_eq!(advance_shown(at(3, 5.0), 1, true, 5.1, true), at(1, 5.1));
        assert_eq!(advance_shown(at(1, 5.0), 3, true, 5.1, false), at(3, 5.1));
        // Aynı aşamada başlangıç anı korunur.
        assert_eq!(advance_shown(at(2, 5.0), 2, true, 9.0, true), at(2, 5.0));
    }

    /// Ekran görüntüsü: `cargo test -p seismira-app -- --ignored ekran`
    /// Çıktı: target/screenshots/*.png. GPU gerektirdiği için CI'da çalışmaz.
    #[test]
    #[ignore]
    fn ekran_goruntusu() {
        let out = PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../target/screenshots"
        ));
        std::fs::create_dir_all(&out).unwrap();
        for (lang, text_test) in [(Lang::Tr, false), (Lang::Tr, true), (Lang::En, true)] {
            let mut harness = Harness::builder()
                .with_size([1280.0, 800.0])
                .wgpu()
                .build_eframe(|cc| {
                    let mut app = SeismiraApp::new(cc);
                    app.set_lang(&cc.egui_ctx, lang);
                    app.show_text_test = text_test;
                    app
                });
            harness.run();
            let img = harness.render().expect("render");
            let name = format!(
                "app_{}_{}.png",
                lang.code(),
                if text_test { "yazi" } else { "bos" }
            );
            img.save(out.join(&name)).unwrap();
        }
    }
}

/// Tahmini bina sayısı: 2 anlamlı basamağa yuvarlanır (≥ 100), binlik
/// ayraçla ("12 000"). Yanlış kesinlik izlenimi vermemek için.
fn fmt_count(lang: Lang, n: f64) -> String {
    let r = if n >= 100.0 {
        let mag = 10f64.powi(n.log10().floor() as i32 - 1);
        (n / mag).round() * mag
    } else {
        n.round()
    };
    let digits = format!("{}", r as u64);
    let sep = match lang {
        Lang::Tr => '.',
        Lang::En => ',',
    };
    let mut out = String::new();
    for (k, ch) in digits.chars().enumerate() {
        if k > 0 && (digits.len() - k) % 3 == 0 {
            out.push(sep);
        }
        out.push(ch);
    }
    out
}

/// Oran (0–1) → yüzde metni: %10 altı bir ondalık, üstü tam sayı; %0,1
/// altı "<0,1".
fn fmt_percent(lang: Lang, x: f64) -> String {
    let p = x * 100.0;
    if p < 0.05 {
        format!("<{}", lang.num(0.1, 1))
    } else if p < 10.0 {
        lang.num(p, 1)
    } else {
        lang.num(p, 0)
    }
}

#[cfg(test)]
mod damage_fmt_tests {
    use super::*;

    #[test]
    fn sayi_ve_yuzde_bicimi() {
        assert_eq!(fmt_count(Lang::Tr, 12_345.0), "12.000");
        assert_eq!(fmt_count(Lang::En, 1_234_567.0), "1,200,000");
        assert_eq!(fmt_count(Lang::Tr, 87.4), "87");
        assert_eq!(fmt_count(Lang::Tr, 156.0), "160");
        assert_eq!(fmt_percent(Lang::Tr, 0.0312), "3,1");
        assert_eq!(fmt_percent(Lang::En, 0.256), "26");
        assert_eq!(fmt_percent(Lang::Tr, 0.0001), "<0,1");
    }
}
