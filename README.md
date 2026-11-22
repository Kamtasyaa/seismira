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

Durum: geliştirme aşamasında (Faz 7). Şu an çalışanlar: sığ kabuk ve
dalma-batma depremleri (sonlu fay, CY14 / Parker 2020), P/S halkaları,
JMA Shindo ve MMI, il/eyalet vurgusu, varsayımsal erken uyarı, tahmini
bina hasarı (ESRM20; Türkiye ve Avrupa), sıvılaşma tehlikesi (Zhu 2017),
isteğe bağlı fay izi. Sırada: heyelan, tsunami.

Status: early development (phase 7). Working: crustal and subduction
earthquakes (finite fault, CY14 / Parker 2020), P/S wavefronts, JMA Shindo
and MMI, regional highlighting, hypothetical early warning, estimated
building damage (ESRM20; Türkiye and Europe), liquefaction hazard (Zhu
2017), optional fault trace. Next: landslides, tsunami.

## Derleme / Build

```bash
python3 scripts/prepare_data.py   # veri (bir kez) / data (once)
cargo run --release -p seismira-app
```

Zemin yenilmesi katmanı için (büyük indirme, ~3,6 GB) / for the ground
failure layer (large download): `python3 scripts/prepare_data.py --ground`.

## Veri talepleri / Data requests

Bazı veri setleri açıkça yeniden dağıtılabilir değil. Kurumlardan izin ya
da veri istendi; yanıt gelene kadar bu veriler yalnızca yerelde
(doğrulama için) kullanılır, uygulamayla dağıtılmaz.

Some datasets are not openly redistributable. Permission or data was
requested from the institutions below; until they reply, such data is used
locally (for validation) only and is not shipped with the app.

| Kurum / Institution | Ne için / Purpose | Durum / Status |
|---|---|---|
| PEER, UC Berkeley (NGA-West2) | Ölçülmüş kayıtlarla CY14 doğrulaması (İzmit, Düzce) / CY14 validation with recorded ground motions | Kayıt izni verildi; yeniden dağıtım sorusu yanıtsız / Registration approved; redistribution question unanswered |
| Universidade de Santiago de Compostela (Fan vd. yeraltı su seviyesi) | Sıvılaşma (Zhu 2017 genel model) / Liquefaction (general model) | İzin istendi; yanıt bekleniyor (şimdilik dağıtılan sürümde kıyı modeli) / Requested; pending (shipped build uses the coastal model) |
| Universität Hamburg (GLiM litoloji) | Heyelan (Nowicki Jessee 2018) / Landslides | İzin isteniyor / Being requested |
| Kandilli Rasathanesi, Düzce Üniv., İTÜ, ODTÜ EERC, DEÜ | Türkiye verileri (kayıtlar, zemin) / Turkish data (records, site) | İstendi; yanıt bekleniyor / Requested; pending |
| Japonya'daki üniversite ve kurumlar / Universities and institutions in Japan | Japonya verileri (Faz 9) / Japanese data (phase 9) | İsteniyor (2026-11) / Being requested |

Ayrıntılar ve lisanslar / Details and licences: `docs/veri.md`,
`data/SOURCES.toml`.

## Lisans / License

Kod: GPL-3.0-or-later (bkz. `LICENSE`). Veriler kendi lisanslarıyla
dağıtılır; ayrıntı `data/SOURCES.toml` ve `docs/veri.md`.
