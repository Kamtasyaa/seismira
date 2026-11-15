# Seismira

Dünya haritası üzerinde çalışan, eğitim ve farkındalık amaçlı deprem
simülatörü. Bir deprem tanımlarsınız; P ve S dalgalarının yayılışını,
JMA Shindo şiddetlerini, erken uyarı sürelerini ve tsunami tehlikesini
canlı izlersiniz.

An educational earthquake simulator on a world map: P/S wave propagation,
JMA Shindo intensity, early-warning times and tsunami hazard.

> **Bu bir eğitim simülasyonudur.** Gerçek deprem bilgisi için AFAD,
> Kandilli, JMA veya USGS gibi resmi kurumları takip edin.
>
> **This is an educational simulation.** For real earthquake information,
> follow official agencies such as AFAD, Kandilli, JMA or USGS.

Durum: geliştirme aşamasında (Faz 4: sığ kabuk depremleri, Shindo, bölge
vurgusu, varsayımsal erken uyarı). / Status: early development (phase 4).

## Derleme / Build

```bash
cargo run --release -p seismira-app
```

## Lisans / License

Kod: GPL-3.0-or-later (bkz. `LICENSE`). Veriler kendi lisanslarıyla
dağıtılır; ayrıntı `data/SOURCES.toml` ve `docs/veri.md`.
