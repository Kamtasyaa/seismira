//! Simülasyon: dalga cepheleri ve olaylar, simülasyon zamanının saf
//! fonksiyonu olarak (ileri-geri sarmada tutarlı; CLAUDE.md Bölüm 9).

use crate::quake::Quake;
use crate::traveltime::{Curve, Front, Phase, TravelTimes};

/// Olay türleri (bildirim akışı ve "sonraki olaya atla" için).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventKind {
    /// Deprem oluştu (t = 0).
    Origin,
    /// Derin depremde dalga yüzeye ulaştı (episantrda ilk varış).
    Surfaced(Phase),
    /// Dalga tablonun sınırına (~100°) ulaştı; ötesinde halka çizilmez.
    ReachedLimit(Phase),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Event {
    /// Simülasyon zamanı (s, deprem anından).
    pub t: f64,
    pub kind: EventKind,
    /// Haritada odaklanılacak nokta (boylam, enlem).
    pub location: (f64, f64),
}

/// P dalgası yüzeye bu süreden geç çıkıyorsa (derin deprem) P ve S'nin
/// yüzeye çıkışı bildirilir; sığ depremlerde birkaç saniyelik gecikme
/// bildirim akışını gereksiz kalabalıklaştırır.
const SURFACING_NOTICE_FROM: f64 = 5.0;

pub struct Simulation {
    pub quake: Quake,
    p: Curve,
    s: Curve,
    events: Vec<Event>,
    max_distance: f64,
}

impl Simulation {
    /// Derinlik tablo dışındaysa `None` (girdi denetimi bunu zaten engeller).
    pub fn new(quake: Quake, tt: &TravelTimes) -> Option<Self> {
        let p = tt.curve(Phase::P, quake.depth_km)?;
        let s = tt.curve(Phase::S, quake.depth_km)?;
        let at = quake.epicenter;
        let mut events = vec![Event {
            t: 0.0,
            kind: EventKind::Origin,
            location: at,
        }];
        // P ve S birlikte bildirilir ya da hiçbiri: P yüzeye geç çıkıyorsa.
        let notify_surfacing = p.first_time() >= SURFACING_NOTICE_FROM;
        for (phase, c) in [(Phase::P, &p), (Phase::S, &s)] {
            if notify_surfacing {
                events.push(Event {
                    t: c.first_time(),
                    kind: EventKind::Surfaced(phase),
                    location: at,
                });
            }
            events.push(Event {
                t: c.last_time(),
                kind: EventKind::ReachedLimit(phase),
                location: at,
            });
        }
        events.sort_by(|a, b| a.t.total_cmp(&b.t));
        Some(Self {
            quake,
            p,
            s,
            events,
            max_distance: tt.max_distance(),
        })
    }

    /// Simülasyonun sonu: S cephesinin tablo sınırına ulaştığı an (s).
    pub fn end_time(&self) -> f64 {
        self.s.last_time()
    }

    pub fn max_distance(&self) -> f64 {
        self.max_distance
    }

    /// t anında P ve S cepheleri.
    pub fn fronts(&self, t: f64) -> (Front, Front) {
        (self.p.front(t), self.s.front(t))
    }

    /// Zamana göre sıralı tüm olaylar.
    pub fn events(&self) -> &[Event] {
        &self.events
    }

    /// t'ye kadar gerçekleşmiş olaylar.
    pub fn events_until(&self, t: f64) -> &[Event] {
        let n = self.events.partition_point(|e| e.t <= t);
        &self.events[..n]
    }

    /// t'den sonraki ilk olayın zamanı.
    pub fn next_event_after(&self, t: f64) -> Option<f64> {
        self.events.iter().map(|e| e.t).find(|&et| et > t + 1e-6)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quake::{Quake, Setting};
    use std::path::Path;

    fn tt() -> TravelTimes {
        let p = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/traveltime/ak135.ttt"
        );
        TravelTimes::load(Path::new(p)).unwrap()
    }

    #[test]
    fn sig_deprem_olaylari() {
        let sim = Simulation::new(Quake::new((29.0, 40.7)), &tt()).unwrap();
        let kinds: Vec<_> = sim.events().iter().map(|e| e.kind).collect();
        assert_eq!(
            kinds,
            vec![
                EventKind::Origin,
                EventKind::ReachedLimit(Phase::P),
                EventKind::ReachedLimit(Phase::S),
            ]
        );
        assert_eq!(sim.end_time(), sim.events().last().unwrap().t);
        // S, P'den sonra gelir; t=60 s'de P cephesi S'den ileride.
        let (p, s) = sim.fronts(60.0);
        match (p, s) {
            (Front::At(dp), Front::At(ds)) => assert!(dp > ds && ds > 0.0),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn derin_depremde_yuzeye_cikis_olayi() {
        let mut q = Quake::new((139.0, 28.0));
        q.setting = Setting::Intraslab;
        q.depth_km = 450.0;
        let sim = Simulation::new(q, &tt()).unwrap();
        let surf: Vec<_> = sim
            .events()
            .iter()
            .filter(|e| matches!(e.kind, EventKind::Surfaced(_)))
            .collect();
        assert_eq!(surf.len(), 2);
        assert!(matches!(sim.fronts(10.0), (Front::NotYet, Front::NotYet)));
    }

    #[test]
    fn orta_derinlikte_yuzeye_cikis_bildirilmez() {
        // 25 km: P ~4 s, S ~7 s. Yalnızca S'nin bildirilmesi tutarsız olurdu.
        let mut q = Quake::new((0.0, 0.0));
        q.depth_km = 25.0;
        let sim = Simulation::new(q, &tt()).unwrap();
        assert!(
            !sim.events()
                .iter()
                .any(|e| matches!(e.kind, EventKind::Surfaced(_)))
        );
    }

    #[test]
    fn olaylar_zamana_gore_ve_sonraki_olay() {
        let sim = Simulation::new(Quake::new((0.0, 0.0)), &tt()).unwrap();
        assert_eq!(sim.events_until(0.0).len(), 1);
        let t1 = sim.next_event_after(0.0).unwrap();
        assert!(t1 > 0.0);
        assert_eq!(sim.events_until(t1).len(), 2);
        assert!(sim.next_event_after(sim.end_time()).is_none());
    }
}
