# Doğrulama

> CLAUDE.md v3’ten ayrıldı (Faz 0, 2026-11-14). Bölüm numaraları atıflar
> bozulmasın diye korunmuştur. "(doğrula)" işaretleri hâlâ geçerlidir.

## 14. Doğrulama

Gerçek depremler (aynı zamanda "hazır senaryolar"):
- 2023 Kahramanmaraş (Mw 7.8 ve 7.5), kabuk
- 1999 İzmit / Gölcük (Mw 7.6), kabuk
- 2016 Kumamoto (Mw 7.0), kabuk, Japonya
- 2019 Ridgecrest (Mw 7.1), kabuk, ABD
- 2011 Tōhoku (Mw 9.0), arayüz, tsunami
- 2024 Noto Yarımadası (Mw 7.5), kabuk, tsunami
- 2010 Maule, Şili (Mw 8.8), arayüz, tsunami
- 2017 Puebla, Meksika (Mw 7.1), levha içi

Karşılaştırma kaynakları: USGS ShakeMap ızgaraları, JMA gözlenen
şiddetleri ve tsunami uyarı kayıtları, AFAD ve NIED istasyon verileri.

Başlangıç kabul ölçütleri (ilk ölçümden sonra birlikte gözden
geçirilir):
- Şiddet: noktaların en az %80'inde fark 1.0 birimden az; ortalama
  sapma 0.3 birimden az.
- Seyahat süreleri: referans ak135 değerlerinden sapma 1 s'den az.
- Tsunami: kıyı bölgelerinin çoğunda kategori JMA'nın verdiğiyle
  aynı; varış süresi hatası %20'den az.
- GMPE birim testleri: referans uygulamayla fark %1'den az.
Sapmalar docs/dogrulama.md'ye yazılır ve kullanıcıya bildirilir.


## Doğrulama kayıtları

### 2026-11-14 — GMPE birim testi: CY14 ↔ pygmm

- Referans: pygmm 0.9.1 (Kottke, MIT), makale katsayı tablosu; bizim
  katsayılar nshmp-lib CY14.csv (CC0). `scripts/gen_gmpe_ref.py` →
  `data/test/cy14_ref.csv`: 360 senaryo (Mw 4.5–8.2, Rrup 1–300 km, SS/RS/NS,
  tavan/taban bloğu, Vs30 250/400/760), PGA, PGV, SA(0.3, 0.5, 0.75, 1.0).
- Sonuç: 1944 karşılaştırmada en büyük |Δln| = 0.00000, σ farkı %0.000.
  Ölçüt (%1) sağlandı. İki bağımsız katsayı kaynağı da birebir aynı.

### 2026-11-20 — GMPE birim testi: Parker vd. (2020) ↔ pygmm

- Referans: pygmm 0.9.1 `ParkerEtAl2020` (küresel, `epistemic=False`).
  `scripts/gen_gmpe_ref.py` → `data/test/psbah20_ref.csv`: 1620 senaryo
  (arayüz M 4.5–9.1; levha içi M 4.5–8.4 × Zhyp 15/40/57/120 km; Rrup
  1–1000 km; Vs30 180–1800), PGA, PGV, SA(0.3, 0.5, 0.6, 0.75, 1.0).
- Sonuç: 11 340 karşılaştırmada en büyük |Δln| = 5e-15, σ farkı 2e-16.
  Ölçüt (%1) sağlandı. pygmm katsayıları da nshmp-lib'den; katsayıların
  PEER ekiyle karşılaştırması ve tek sapma (arayüz c6) docs/bilim.md 7.4.

### 2026-11-20 — Parker vd. (2020) bölgesel model ↔ pygmm ve OpenQuake

- pygmm 0.9.1: küresel, Alaska, Cascadia; 4860 senaryo, 34 020 karşılaştırma,
  en büyük |Δln| 5e-9 (referans 10 basamağa yuvarlı).
- OpenQuake 3.26.2 (AGPL; yalnızca çıktı): 12 bölgenin hepsi, arayüz M ≤ Mc,
  levha içi Zhyp 15/60/120 km; 2415 senaryo, 9660 karşılaştırma, en büyük
  |Δln| 4.6e-4, σ farkı 7e-11. `scripts/gen_psbah20_oq_ref.py` →
  `data/test/psbah20_regional_ref.csv`. Ölçüt (%1) sağlandı.



- 4000 ızgara dışı nokta; en büyük ara değer hatası P 0.094 s, S 0.135 s.
  Ölçüt (< 1 s) sağlandı. (Ayrıntı: docs/bilim.md 7.2.)

### 2026-11-14 — İlk şiddet doğrulaması: 1999 İzmit (Mw 7.6), kabuk

- Veri: USGS ShakeMap (atlas, usp0009d4z) `stationlist.json`, kamu malı,
  `data/validation/izmit1999/`. 38 aletsel istasyon; model aralığında
  (Rrup ≤ 300 km) 31 istasyon.
- Gözlenen RotD50 yerine iki yatay bileşenin geometrik ortalaması.
  Şiddet farkı (ΔI), gözlenen ve tahmin edilen PGV'nin aynı dönüşümle
  (Midorikawa vd. 1999) JMA aletsel şiddetine çevrilmesiyle; bileşen oranı
  varsayımı (1.23) iki tarafta da aynı olduğu için ΔI'yı etkilemez.
- Komut: `seismira-data validate-shakemap data/validation/izmit1999/stationlist.json 29.864 40.748 17 7.6 85 81 SS [hiposantr_konumu]`
  (merkez üssü ve derinlik USGS; doğrultu/eğim USGS moment tensörü düğüm
  düzlemi 2).

| Karşılaştırma | ort. ln(göz/tah) | std ln | ort. ΔI | \|ΔI\| < 1 |
|---|---|---|---|---|
| A: ShakeMap mesafeleri (yalnız GMPE + dönüşüm) | −0.31 | 0.66 | −0.24 | %87 |
| B: bizim geometri, hiposantr ortada (varsayılan) | −0.49 | 0.66 | −0.39 | %84 |
| B: bizim geometri, hiposantr doğrultunun %25'inde | −0.38 | 0.70 | −0.30 | %84 |

- Değerlendirme: A, başlangıç ölçütlerini (ort. sapma < 0.3, %80'i 1
  birimden az) sağlıyor. Saçılım CY14'ün σ'sıyla uyumlu. Model ortalamada
  ~%27 fazla tahmin ediyor; ShakeMap'in kendi GMPE'si de bu olay için
  negatif sapma bulmuş (PGV ln_bias ≈ −0.16): İzmit'in olaylar arası terimi
  negatif. B'de ek sapma, varsayılan geometrinin (hiposantr ortada, L ≈ 198
  km) fayı İstanbul'a doğru uzatmasından; gerçek kırılma çoğunlukla doğuya
  ilerledi ve istasyonların çoğu İstanbul'da. Gerçeğe yakın hiposantr
  konumuyla ölçüt sınırına geliyor.
- 2026-11-24 güncellemesi: ShakeMap'in İzmit gözlemleri NGA-West2'deki aynı
  kayıtlardan sistematik ~%37 düşük çıktı; NGA verisiyle sapma pozitif ve
  küçük (aşağıda "NGA-West2 kayıtlarıyla CY14").
- Açık konular: (1) WC94 alan bağıntısı İzmit için gerçekten uzun fay veriyor
  (198 km; gözlenen ~145 km). (2) Hazır senaryolarda (Faz 10) gerçek
  hiposantr konumu ve doğrultu verilecek. (3) Vs30 burada istasyonun kendi
  değeri; uygulamada Faz 4'e kadar sabit 400 m/s.


#### İzmit uzak alan (kullanıcı gözlemi üzerine, 2026-11-14)

Kullanıcı, 300 km'ye yakın şehirlerde (Burdur, Afyon çevresi) Shindo 4
görülmesini sorguladı. Aletsel istasyonlarla mesafeye göre (ShakeMap
mesafeleri ve istasyon Vs30'u ile CY14; I = Midorikawa 0~7):

| Rrup | İstasyonlar | I gözlenen | I tahmin |
|---|---|---|---|
| 119–145 km | BTS, ERG, KUT | 4.55–4.58 | 3.94–4.42 |
| 157–344 km | TKR, BLK, AFY, USK, TOS, CNK, MNS, BRN, DNZ, AYD | 2.99–4.09 (çoğu 3.0–3.6) | 3.16–4.14 (çoğu 3.6–4.1) |
| 472 km (model dışı) | TKT | 1.04 | 3.32 |

Mesafe aralıklarına göre İzmit (A / B): 0–25 km −0.62 / −0.72; 25–50
−0.40 / −0.50; 50–100 −0.05 / −0.29; 100–150 +0.34 / +0.16; 150–200
−0.45 / −0.63 (n=2); 200–300 −0.20 / −0.24 (n=5).

İlk yorum ("Türkiye'de uzak alanda sistematik fazla tahmin") 2023
Kahramanmaraş verisiyle ÇÜRÜTÜLDÜ (aşağıda): orada 200–300 km'de model
gözlemin altında kalıyor. İzmit'teki fark olaya özgü (negatif olay terimi)
ve az istasyona dayanıyor. Bölgesel model değişikliğine gerek görülmedi.

### 2026-11-14 — Şiddet doğrulaması: 2023 Kahramanmaraş (Mw 7.8), kabuk

- Veri: USGS ShakeMap (us, sürüm 19, us6000jllz) `stationlist.json`,
  kamu malı; 5 MB olduğu için depoda değil: `data/raw/validation/kahramanmaras2023/`
  (URL: https://earthquake.usgs.gov/product/shakemap/us6000jllz/us/1756921940993/download/stationlist.json).
- 262 aletsel istasyon; model aralığında 191. Bu ShakeMap sürümü Rx
  vermiyor; taban bloğu kabul edildi (dip 89°, tavan bloğu terimi ~0).
- Komut: `seismira-data validate-shakemap <stationlist.json> 37.0143 37.2256 10 7.8 228 89 SS`
  (merkez üssü USGS, doğrultu/eğim USGS moment tensörü düğüm düzlemi 2).

| Karşılaştırma | ort. ln(göz/tah) | std ln | ort. ΔI | \|ΔI\| < 1 |
|---|---|---|---|---|
| A: ShakeMap mesafeleri | +0.19 | 0.57 | +0.15 | %96 |
| B: bizim geometri (hiposantr ortada, L = 320 km) | +0.22 | 0.64 | +0.17 | %93 |

| Rrup (km) | n | ort. ΔI (A) | ort. ΔI (B) |
|---|---|---|---|
| 0–25 | 34 | +0.18 | +0.46 |
| 25–50 | 11 | +0.11 | −0.00 |
| 50–100 | 25 | +0.18 | +0.15 |
| 100–150 | 27 | +0.03 | +0.02 |
| 150–200 | 20 | +0.12 | +0.11 |
| 200–300 | 74 | +0.20 | +0.15 |

- Değerlendirme: A ve B başlangıç ölçütlerini sağlıyor. Model hafifçe
  az tahmin ediyor (Kahramanmaraş pozitif olay terimi; İzmit negatifti).
  B'de 0–25 km'deki +0.46, tek düzlemli merkezlenmiş fayın gerçek çok
  parçalı (~350 km, iki yönlü) kırılmayı temsil etmemesinden.

## Faz 4: Vs30 haritasıyla tüm zincir (C), 2026-11-14

C = bizim fay geometrimiz (varsayılan, hiposantr ortada) + bizim Vs30
haritamız (USGS 2026 mozaiği, `data/vs30/vs30.sras`, model aralığına
kırpılmış). A ve B istasyonun ShakeMap Vs30'unu kullanır (yukarıdakilerle
aynı). Komut: yukarıdaki komutlara `--vs30 data/vs30/vs30.sras` eklenir.

| Olay | ort. ln(göz/tah) | std ln | ort. ΔI | \|ΔI\| < 1 | B'deki ΔI |
|---|---|---|---|---|---|
| İzmit 1999 (31 ist.) | −0.55 | 0.61 | −0.44 | %81 | −0.39 |
| Kahramanmaraş 2023 (191 ist.) | +0.10 | 0.67 | +0.08 | %91 | +0.17 |

Harita Vs30'u / istasyon Vs30'u (ShakeMap stationlist):

| Olay | ort. ln oran | std ln | kaynak (harita / komşu / referans) |
|---|---|---|---|
| İzmit 1999 | −0.09 (×0.91) | 0.32 | 33 / 2 / 0 |
| Kahramanmaraş 2023 | −0.26 (×0.77) | 0.40 | 261 / 1 / 0 |

- Değerlendirme: İki olay da ölçüt içinde (%80 üstü |ΔI| < 1). İzmit'te
  harita biraz daha yumuşak zemin veriyor → biraz daha fazla tahmin
  (−0.39 → −0.44; asıl neden yine olay terimi ve geometri, bkz. yukarı),
  saçılım azaldı (0.66 → 0.61). Kahramanmaraş'ta sapma yarıya indi
  (+0.17 → +0.08), saçılım biraz arttı (0.64 → 0.67).
- SAPMA NOTU (kullanıcıya bildirildi): Kahramanmaraş istasyonlarında
  harita Vs30'u ShakeMap istasyon Vs30'undan ortalama %23 düşük. ShakeMap
  istasyon Vs30'u kısmen ölçülmüş (AFAD saha karakterizasyonu) olabilir;
  harita topoğrafik eğim vekiline dayanıyor (doğrula: Türkiye bölgesinde
  2026 mozaiğine hangi bölgesel harita girdi). Faz 6'da ESRM20 site
  modeli ile Türkiye için yeniden karşılaştırılacak.

## Otomatik regresyon testi (2026-11-15)

İzmit 1999 karşılaştırmasının A ve B satırları artık `cargo test` ile
denetleniyor (`seismira-data`, `izmit_1999_dogrulamasi_kayitli_degerlerde`):
31 istasyon, A ort. ΔI −0,24 / %87, B −0,39 / %84 (±0,01). Yer hareketi,
şiddet dönüşümü ya da kaynak geometrisi değişirse test düşer; değişiklik
bilinçliyse yeni sayılar buraya yazılıp testteki beklenen değerler
güncellenir. C (Vs30 haritası) ve Kahramanmaraş elle çalıştırılır; verileri
depoda değil. Komutun çıktısı aynı (yeniden çalıştırıldı, tablo birebir).

## Faz 5: dalma-batma doğrulaması (2026-11-20)

Araç: `seismira-data validate-shakemap` artık `--setting interface|intraslab`
ve `--slab data/slab2/slab2.sslb` alır (Parker vd. 2020; bölge, uygulamadaki
gibi merkez üssünün Slab2 zonundan) ve istasyonları yay önü / yay ardı diye
ayırır (`--csv` ile istasyon ayrıntısı). A, B, C tanımları yukarıdaki gibi.

### 2011 Tōhoku (USGS Mw 9.1), arayüz

- Veri: USGS ShakeMap atlas `stationlist.json` (4 MB, depoda değil:
  `data/raw/validation/tohoku2011/`, URL ve SHA256 SOURCES.toml'da); 1204
  istasyon, model aralığında (Rrup ≤ 1000 km) 1189. Kırılma (ShakeMap,
  Suzuki vd. 2011): ~36,0–39,9° K, 10–50 km derinlik.
- Komut: `seismira-data validate-shakemap <stationlist.json> 142.373 38.297 29 9.1 193 14 RS --setting interface --slab data/slab2/slab2.sslb --vs30 data/vs30/vs30.sras`
  (merkez üssü ve derinlik USGS; doğrultu / eğim USGS W-faz düğüm düzlemi).
- Bölge Japonya – Pasifik (Slab2 zonu kur). Bizim fay: Mw 9,1'de 657 × 207 km.

| Karşılaştırma | ort. ln(göz/tah) | std ln | ort. ΔI | \|ΔI\| < 1 |
|---|---|---|---|---|
| A: ShakeMap mesafeleri, bölgesel (Japonya Pasifik) | −0.01 | 0.47 | −0.01 | %99 |
| A: ShakeMap mesafeleri, KÜRESEL model (karşılaştırma) | +0.43 | 0.59 | +0.34 | %93 |
| B: bizim geometri | −0.31 | 0.51 | −0.24 | %96 |
| C: bizim geometri + bizim Vs30 | −0.38 | 0.50 | −0.30 | %96 |
| B, Mw 9,0 (uygulamadaki senaryo; fay 519 × 207 km) | −0.09 | 0.49 | −0.07 | %99 |
| C, Mw 9,0 | −0.16 | 0.48 | −0.13 | %98 |
| B, Mw 9,0, 196 km genişlik sınırıyla (fay 547 × 196 km) | −0.12 | 0.49 | −0.09 | %99 |
| C, Mw 9,0, 196 km sınırıyla | −0.19 | 0.48 | −0.15 | %98 |
| B, Mw 9,1, 196 km sınırıyla (fay 692 × 196 km) | −0.33 | 0.52 | −0.26 | %96 |
| C, Mw 9,1, 196 km sınırıyla | −0.40 | 0.51 | −0.32 | %95 |

SAPMA (kullanıcıya bildirildi, 2026-11-20): Allen & Hayes (2017) genişlik
sınırı (196 km) eklenince fay uzadı; Mw 9,1'de C'nin ortalama sapması −0,32
ile başlangıç ölçütünü (< 0,3) az farkla aşıyor. Sebep sınır değil, alan:
WC94 (veri M ≤ 7,9) Mw 9,1'de 136 000 km² veriyor, gerçek kırılma ~450 × 200
km. Dalma-batmaya özgü alan bağıntısı (Allen & Hayes 2017 tam metni ücretli)
ya da hazır senaryoda gerçek fay geometrisi düzeltir. Mw 9,0 senaryosu
ölçüt içinde.

| Rrup (km) | n | ΔI (A) | ΔI (B) | ΔI (C) |
|---|---|---|---|---|
| 50–100 | 113 | +0.10 | +0.00 | −0.01 |
| 100–150 | 144 | −0.05 | −0.18 | −0.19 |
| 150–200 | 147 | −0.08 | −0.25 | −0.27 |
| 200–300 | 189 | −0.06 | −0.31 | −0.36 |
| 300–500 | 408 | −0.10 | −0.43 | −0.48 |
| 500–1000 | 188 | +0.25 | +0.04 | −0.11 |

Yay önü / yay ardı (A): yay önü 419 istasyon +0.03, yay ardı 770 istasyon
−0.03 (ΔI). Bölgelere göre (ln, A): Tōhoku Pasifik kıyısı yay önü −0.15,
Tōhoku Japon Denizi tarafı yay ardı −0.32; Akita şehri (AKT010) gözlenen
PGV 12 cm/s, tahmin 20 cm/s.

- Değerlendirme: ölçütler sağlandı (Mw 9,1 C'de ort. ΔI −0,30 sınırda).
  BÖLGESELLEŞTİRME ÇOK ÖNEMLİ: küresel modelle A +0,34 / std 0,59, Japonya
  Pasifik katsayılarıyla −0,01 / 0,47.
- B / C'deki fazla tahmin (Mw 9,1) 150–500 km'de büyüyor: bizim fay (WC94
  alanı, M > 7,9 ekstrapolasyon) 657 km uzunlukta, gerçek kırılma ~450 km;
  merkezlenmiş fay güneyde Kanto'ya fazla uzanıyor. Dalma-batmaya özgü alan
  bağıntısı (PDF bekleniyor) ve hazır senaryoda gerçek fay geometrisi bunu
  düzeltir. Mw 9,0'da (alan %20 küçük) sapma −0,07'ye iniyor.
- Yay ardı: Japon Denizi tarafında model ~%15–30 fazla (ln −0.17 farkı,
  ~0,1–0,4 Shindo birimi); Parker raporunun "Japonya'da yay ardında daha
  güçlü sönümlenme" gözlemiyle aynı yönde, ama küçük. Kart notu buna göre
  yumuşatıldı ("tahmin fazla olabilir").
- Yay çizgisi denetimi: Slab2'ye göre Kuzeydoğu Japonya volkanik cephesinin
  (Iwate, Kurikoma, Zao, Azuma, Nasu) altında levha 84–90 km derinlikte;
  küresel ortalama 105 km eşiği yayı orada ~20–40 km fazla batıda çiziyor
  (sınırdaki birkaç istasyon yay önü sayılıyor). Zon başına eşik için
  Syracuse & Abers (2006) tablosu gerekir (görülmedi).

### 2017 Puebla (Mw 7.1), levha içi

- Veri: USGS ShakeMap atlas `stationlist.json`, `data/validation/puebla2017/`
  (depoda, 480 KB). 29 UNAM istasyonu, hepsi TEK kanallı ("UNK"; bileşen
  tanımı belirtilmemiş, değer olduğu gibi kullanıldı; büyük yatay bileşense
  gözlenen ~%10 fazla görünür). Mexico City istasyonları bu listede yok
  (USGS us v13 listesinde de aynı 29 istasyon).
- Komut: `seismira-data validate-shakemap data/validation/puebla2017/stationlist.json -98.4887 18.5499 48 7.1 299 44 NS --setting intraslab --slab data/slab2/slab2.sslb --vs30 data/vs30/vs30.sras`
  (USGS Mww düğüm düzlemi 1; düzlem 2 (108° / 47°) neredeyse aynı sonuç).
- Bölge Orta Amerika kuzey (Slab2 zonu cam). Fay 35 × 35 km, Ztor 35,8 km.

| Karşılaştırma | ort. ln(göz/tah) | std ln | ort. ΔI | \|ΔI\| < 1 |
|---|---|---|---|---|
| A: ShakeMap mesafeleri | +0.25 | 0.59 | +0.20 | %97 |
| B: bizim geometri | +0.15 | 0.59 | +0.12 | %97 |
| C: bizim geometri + bizim Vs30 | +0.00 | 0.56 | +0.00 | %97 |

- Değerlendirme: ölçütler sağlandı. Model hafifçe az tahmin ediyor
  (tek kanallı gözlem büyük bileşense bunun bir kısmı ondandır). 500 km
  ötesinde 3 istasyonda +0,8 (az veri).
- Otomatik test: `puebla_2017_dogrulamasi_kayitli_degerlerde` (A ve B
  satırları, ±0,01; Vs30 haritası CI'da yok).

### 2026-11-20 — Süre modeli birim testi: Bahrampouri vd. (2021) ↔ OpenQuake

- Referans: OpenQuake 3.26.2 `BahrampouriEtAldm2021SInter` / `SSlab`
  (AGPL; yalnızca çıktı, `scripts/gen_duration_sub_ref.py` →
  `data/test/duration_sub_ref.csv`): 768 senaryo (arayüz RS; levha içi
  RS/SS/NS; M 4–9; Rrup 5–500 km; Vs30 150–1200).
- Sonuç: en büyük |Δln D| = 4.6e-5 (%0,005; OpenQuake'in δZ1 yuvarlaması),
  σ aynı. Ölçüt (%1) sağlandı. Elle denetim: arayüz M 4, Rrup 5 km, Vs30 150
  → ln D 2,6465 (OpenQuake 2,646588).

## Faz 6: bina hasarı doğrulaması, 2023 Kahramanmaraş (2026-11-20)

- Komut: `seismira-data validate-damage data/validation/kahramanmaras2023/sbb_hasar_2023-03-06.csv data/raw/esrm20/exposure_repo/Exposure_Model_Turkey_Res.csv`
  (ESRM20 konut sayısı için efehr/esrm20_exposure, commit 92dba863,
  sha256 59e046a4…2434; depoda değil).
- Model: uygulamanın zinciri (bizim fay, Vs30 haritası, ESRM20, CY14). İki
  ana şok (USGS Mw 7,8 Pazarcık 227,6°/89°; Mw 7,5 Elbistan 277,2°/78°) ayrı
  hesaplanıp il oranları 1 − (1 − f₁)(1 − f₂) ile birleştirildi; artçılar
  yok. Aralık: ±1τ (iki olayda aynı yönde).
- Gözlem: SBB "2023 Kahramanmaraş ve Hatay Depremleri Raporu" (Mart 2023),
  Tablo 20 (ÇŞİDB, 6 Mart 2023): acil yıkılacak + ağır hasarlı + yıkık
  KONUT; payda ESRM20 konut sayısı. Ülke düzeyinde bina: Tablo 12 (232 632
  bina) / Tablo 11 MAKS (2 618 697 bina).

| İl | ESRM20 bina | MAKS bina | model ağır+göçme % (olası) | gözlenen konut % | model / gözlenen |
|---|---|---|---|---|---|
| Adana | 278602 | 451117 | 5.1 (2.5–9.5) | 0.6 | 9.07 |
| Adıyaman | 55418 | 120496 | 24.7 (15.3–36.7) | 58.8 | 0.42 |
| Diyarbakır | 119669 | 225679 | 0.3 (0.1–0.8) | 3.2 | 0.08 |
| Elazığ | 57241 | 123713 | 1.6 (0.6–3.6) | 11.5 | 0.14 |
| Gaziantep | 227079 | 305683 | 8.7 (5.2–13.7) | 5.4 | 1.62 |
| Kahramanmaraş | 124454 | 243153 | 18.4 (11.8–27.1) | 41.9 | 0.44 |
| Malatya | 90980 | 178987 | 23.2 (15.0–33.4) | 50.2 | 0.46 |
| Hatay | 198681 | 406849 | 22.3 (15.4–30.7) | 55.1 | 0.40 |
| Kilis | 16222 | 37312 | 5.0 (2.6–9.0) | 8.5 | 0.59 |
| Osmaniye | 75168 | 143080 | 7.8 (3.9–14.0) | 11.0 | 0.71 |
| Şanlıurfa | 181902 | 382628 | 1.1 (0.4–2.7) | 1.2 | 0.97 |

- 11 il toplamı: model %10,2 (bina), gözlenen %8,9 (bina, ülke toplamı) /
  %17,3 (konut). İl başına ln(model/gözlenen konut) ortalama −0,55 (×0,57),
  std 1,24.
- Değerlendirme (kullanıcıya bildirildi):
  - Bina düzeyinde toplam UYUMLU (%10,2 / %8,9). Kabul ölçütü bu iş için
    tanımlı değildi; ilk ölçüm olarak kaydedildi.
  - Konut oranı bina oranının ~2 katı (çöken çok katlı binalarda konut
    çok): ağır etkilenen dört ilde (Hatay, Kahramanmaraş, Adıyaman,
    Malatya) model / gözlenen konut 0,40–0,46, bina oranına çevrilince
    (÷ ~1,95) ~0,8–0,9.
  - Uzak illerde saçılım büyük: Adana ×9 fazla, Elazığ / Diyarbakır ×0,1
    az. Sebep büyük ölçüde geometri: tek düzlem, merkez üssünde ortalanmış
    320 km fay; gerçek kırılma ~350 km, çift yönlü, Elbistan kırılması
    Malatya'ya uzanıyor. Hazır senaryoda (Faz 10) gerçek fay izi verilmeli.
  - SAPMA: ESRM20 bina sayıları MAKS'ın ~yarısı (11 il: 1,43 / 2,62
    milyon). Kartlardaki "~N bina" sayıları büyük olasılıkla düşük; oranlar
    etkilenmez. ESRM20 Türkiye stoku 2011 sayımına dayanıyor olabilir
    (doğrula: ESRM20 teknik rapor, Türkiye bölümü).


## Faz 7: sıvılaşma doğrulaması, 2023 Kahramanmaraş (2026-11-22)

Referans: USGS Ground Failure ürünü, us6000jllz sürüm 14 (2023-04-14),
Zhu vd. (2017) genel model, ShakeMap v17 ile (info.json). İndirme: ComCat
`ground-failure` ürünü (`zhu_2017_general_model.tif`, 15″, alansal kaplama)
ve `includesuperseded=true` ile ShakeMap v17 `grid.xml` (2′ aralık). Ham
veri `data/raw/validation/kahramanmaras2023/groundfailure/` (depoda değil).
USGS'in raporladığı toplam tehlike (hagg0.1g, PGA ≥ %10 g hücrelerde
Σ kaplama × alan): **569,7 km²**.

Ölçü: aynı toplam tehlike (bizim 30″ hücrelerimizde; USGS 15″ haritası 2 × 2
ortalanır, maskeli NaN = 0). Yöntem sınaması: USGS haritasının bizim
hücrelerimizdeki toplamı 558,8 km² (raporlanan değerden %2 az: hücre
eşlemesi ve alan hesabı).

1. Sarsıntı = USGS ShakeMap v17 (çift doğrusal enterpolasyon), zemin
   katmanları bizim (`seismira-data validate-liquefaction … 7.8 <kip>`):

   | Vs30 | Bizim (km²) | Oran | Korelasyon | Aynı sınıf | ln oran (ikisi ≥ %0,5) |
   |---|---|---|---|---|---|
   | ShakeMap SVEL | 634,8 | 1,14 | 0,777 | %76,8 | +0,12 |
   | Bizim Vs30 (USGS 2026 mozaiği) | 921,3 | 1,65 | 0,883 | %75,6 | +0,59 |

   Aynı Vs30 ile fark %14: yağış (CHELSA 1981–2010 / WorldClim 1950–2000),
   nehre ve kıyıya uzaklık (GMTED akış ağı + NE / HydroSHEDS, NASA), su
   seviyesi (Fan 2017 / Fan 2013), eğim (7,5″ çeyrek / 15″ enterpolasyon).
   Asıl fark Vs30: 2026 mozaiği bölgede ShakeMap'in Vs30'undan daha düşük
   (yumuşak zemin); uygulama yeni haritayı kullanır (bilinçli seçim).
2. Uçtan uca (`validate-liquefaction-scenario`): bizim sarsıntı modelimiz
   (CY14, tek düzlemli fay: USGS düğüm düzlemi 227,6° / 89,2°), bizim Vs30,
   uygulamadaki zincirin aynısı (adım 1). **Bizim 1177,8 km², USGS aynı
   alanda 589,7 km², oran 2,00.** ≥ %0,5 hücre: bizim 19 717, USGS 10 990.
   Ayrışım (çarpımsal): Vs30 ×1,45, diğer zemin katmanları ×1,14, sarsıntı
   modeli ×1,28 (istasyon verisiyle düzeltilmiş ShakeMap'e göre; tek
   düzlemli fay ve medyan GMPE; Faz 6 hasar doğrulamasıyla aynı yön).

SAPMA (kullanıcıya bildirildi, 2026-11-22): Kahramanmaraş'ta sıvılaşma
toplam tehlikesi USGS'in ~2 katı. Katsayılar değiştirilmedi (formüller
yayımlanmış kaynaktan, CLAUDE.md Bölüm 7). Model belirsizliği zaten büyük:
USGS'in kendi σ'sı (beta dağılımı) ve Zhu 2017'nin olay bazında
saçılımı (doğrula: makale kapalı erişim). İleride bakılabilecek: Vs30
haritasının bölgesel farkı; tek düzlem yerine çok parçalı fay.

### 2024 Noto (Mw 7,5) ve duyarlılık (2026-11-22)

USGS Ground Failure us6000m0xl sürüm 10, ShakeMap v10 (1′ aralık); USGS
toplam tehlike 777,2 km² (bizim hücrelerde 776,9: yöntem birebir).

| Vs30 | Bizim (km²) | Oran | Korelasyon | Aynı sınıf |
|---|---|---|---|---|
| ShakeMap SVEL | 1059,5 | 1,36 | 0,805 | %50,9 |
| Bizim Vs30 | 1286,6 | 1,66 | 0,788 | %47,7 |

Zemin katmanlarımız iki depremde de yukarı sapıyor (aynı Vs30 ile ×1,14 ve
×1,36). Duyarlılık (`SEISMIRA_DIAG_DR_ADD`, nehre uzaklığa sabit ekleme,
SVEL ile):

| dr + | Kahramanmaraş oran | Noto oran |
|---|---|---|
| 0 | 1,14 | 1,36 |
| 0,5 km | 0,94 | 1,25 |
| 1 km | 0,78 | 1,15 |
| 2 km | 0,51 | 0,96 |

Sonuç: nehre uzaklık en duyarlı girdi (−0,2054/km, ve çoğu hücrede
min(dc, dr) = dr). Nehir ağının tanımı (bizde 10 km² eşiği + Vs30 göl kodu +
Natural Earth hatları) DOĞRULANMAMIŞ bir varsayım; Zhu 2017'nin HydroSHEDS
ağını hangi eşikle kullandığı makalede olmalı (BSSA, kapalı erişim; Zhu 2015
Earthquake Spectra da kapalı, OpenAlex 2026-11-22). Tek bir kaydırma iki
depremi birlikte tutturmadığı için ayar YAPILMADI; makale gelince tanım
uygulanıp bu tablo yeniden üretilecek. Kullanıcıdan PDF istendi.

## Faz 7: heyelan doğrulaması, Kahramanmaraş 2023 ve Noto 2024 (2026-11-23)

Referans: USGS Ground Failure, Nowicki Jessee vd. (2018) modeli
(`jessee_2018_model.tif`, 7,5″, alansal kaplama). us6000jllz sürüm 14
(ShakeMap v17), us6000m0xl sürüm 10 (ShakeMap v10); ComCat `ground-failure`
ürününden. USGS'in raporladığı toplam tehlike: Kahramanmaraş **370 km²**
(± 19), Noto **89 km²** (± 59). Ham veri `data/raw/validation/*/groundfailure/`
(depoda değil). Araç: `seismira-data validate-landslide` (ham katmanlar,
ShakeMap sarsıntısı) ve `validate-landslide-scenario` (uçtan uca).

Ölçü: PGA ≥ %10 g hücrelerde Σ kaplama × alan (bizim 30″ hücrelerde; USGS
7,5″ haritası 4 × 4 ortalanır, NaN = 0). Yöntem sınaması: USGS haritasının
bizim hücrelerdeki toplamı 391,4 / 95,0 km² (raporlanandan %6 / %7 fazla;
tüm hücrelerle 485,8 / 120,8, yani USGS'in ölçüsü PGA ≥ %10 g'ye uyuyor).

1. Sarsıntı = ShakeMap, zemin katmanları bizim (GLiM vektör → 30″, ESA CCI
   2009 → GlobCover eşlemesi, CTI, 7,5″ eğim çeyrekleri). CCI 60/70
   (yoğunluğu belirsiz orman) eşlemesine duyarlılık:

   | CCI 60/70 eşlemesi | K.maraş oran | K.maraş korelasyon | Noto oran | Noto korelasyon |
   |---|---|---|---|---|
   | iki katsayının ortalaması (ilk sürüm) | 0,86 | 0,908 | 0,28 | 0,836 |
   | **kapalı (50 / 70; seçilen)** | **0,90** | **0,936** | **0,88** | **0,945** |
   | açık (60 / 90) | 0,89 | 0,864 | 0,27 | 0,346 |

   Sonuç: GlobCover 2009 bu ormanları çoğunlukla kapalı sınıflamış; kapalı
   eşleme iki depremde de USGS'i %10–12 içinde ve 0,94–0,95 korelasyonla
   yeniden üretiyor. Kalan küçük fark: GLiM sürümü / rasterleme (USGS 250 m),
   CTI (HYDRO1k / bizim GMTED akış birikimi), eğim (USGS 7,5″ doğrudan / bizim
   çeyrek ortalaması), arazi örtüsü (GlobCover / CCI).
2. Uçtan uca (Kahramanmaraş; bizim CY14 sarsıntısı, tek düzlemli fay, USGS
   düğüm düzlemi 227,6° / 89,2°; uygulamanın `ls_static.sras` dosyası):
   **bizim 337,2 km² (tüm hücreler), USGS aynı alanda 487,8 km², oran
   0,69.** Zemin ×0,90 (adım 1) → sarsıntı modeli ×~0,77. Sıvılaşmada
   sarsıntı ×1,28 YUKARI idi (ovalar); heyelanda AŞAĞI (dağlık, sert zemin).
   İkisi de tek düzlemli fay + istasyon verisi olmayan medyan tahminden;
   katsayılar değiştirilmedi.

SAPMA (kullanıcıya bildirildi, 2026-11-23): uçtan uca heyelan toplamı
USGS'in ~0,7 katı (zemin katmanları tek başına 0,9).

### GLiM 0,5° (CC BY 3.0) ile heyelan, vektör GLiM'in yerine (2026-11-23)

Amaç: vektör GLiM'in dağıtım izni gelmezse heyelan katmanını dağıtılabilir
yapmak. `scripts/gen_glim05.py` (PANGAEA 788537, 0,5°, en yakın komşu;
kıyıda denize düşen 0,5° hücreler en yakın sınıflı hücreyle doldurulur) +
`seismira-data lith`; diğer zemin katmanları aynı. Ölçü yukarıdaki adım 1
ile aynı (ShakeMap sarsıntısı, PGA ≥ %10 g):

| Litoloji | K.maraş oran | K.maraş korelasyon | Noto oran | Noto korelasyon |
|---|---|---|---|---|
| vektör GLiM (30″; izin bekliyor) | 0,90 | 0,936 | 0,88 | 0,945 |
| 0,5°, dolgusuz | 1,07 | 0,898 | 0,58 | 0,730 |
| **0,5°, kıyı dolgulu** | **1,09** | **0,904** | **0,96** | **0,952** |

Sonuç: dolgulu 0,5° sürüm iki depremde vektör sürümle aynı düzeyde
(toplamda USGS'in %4–9 içinde, korelasyon 0,90–0,95). Eğim ve sarsıntı
baskın; litoloji 55 km'lik ölçekte çoğunlukla aynı. Sınır: yalnızca iki
deprem; küçük ölçekli litoloji değişimi (ör. dar alüvyon vadileri) kaba
sürümde kaybolur. Dağıtılan pakete alınması kullanıcı kararına bağlı
(ls_static ~365 MB, paket hedefi ~500 MB).

## Faz 8: tsunami doğrulaması, Tōhoku 2011, Noto 2024, Maule 2010 (2026-11-23)

Gözlemler: NOAA NCEI / WDS Global Historical Tsunami Database (HazEL API,
doi:10.7289/V5PN93H7; scripts/fetch_tsunami_validation.py → data/raw/
validation/tsunami/, depoda değil): olay 5413 (6429 kayıt), 5907 (537),
4682 (692). Ölçüm türü kodları API'de tanımlı değil; kayıt adlarından
çıkarıldı: 2 mareograf, 3 DART (BPR), 4 / 5 saha araştırması, 1 görgü.
Kaynak parametreleri: USGS ComCat merkez üssü / derinlik ve Mww moment
tensörü düğüm düzlemi (Tōhoku Mw 9,1, 192/8/78, arayüz; Noto Mw 7,5,
49/41/103, kabuk; Maule Mw 8,8, 17/13/108, arayüz). Araç: `seismira-data
validate-tsunami` (uygulamayla aynı pencere kuralları: bölgesel 1′, Mw ≥
7,9'da küresel 4′).

Varış (tür 2 ve 3, `travHours/travMins`; model: gözleme 15 km içindeki en
yakın deniz hücresi):

| Deprem | < 1000 km: medyan model/gözlem, ±%20 içinde | ≥ 1000 km |
|---|---|---|
| Tōhoku | 0,87 (0,80–1,00), %63 (79 gözlem) | 0,95 (0,90–0,98), **%95** (191) |
| Noto | 1,02 (0,93–1,30), %50 (14 / 31) | modelde karşılığı yok (6; pencere dışı, Mw < 7,9) |
| Maule | 0,91 (0,36–1,17), %40 (5) | 0,99 (0,97–1,00), **%95** (133) |

Uzak alanda ölçüt (%20) sağlanıyor. Yakın alanda SAĞLANMIYOR: model biraz
erken (Tōhoku medyan 0,87); bazı yakın mareograf "varışları" (Miyako 2 dk,
Kamaishi 4 dk, Valparaíso 3 dk) büyük olasılıkla eş-sismik su seviyesi
değişimi ya da ilk küçük hareket; kaynak alanı varsayımı (|uz| ≥ %25 H) ve
1′ kıyı derinliği de etkili. Değiştirilmedi.

Yükseklik (tür 2 ve 4, `runupHt`; model: yakın alan tırmanması olan kıyı
noktası ≤ 20 km). JMA kategorisi bölgenin EN YÜKSEK değerine göre verildiği
için asıl ölçü bölge bazında (NCEI `area`) en yüksek / en yüksek:

| Deprem | R0 / gözlenen en yüksek | Bölge bazında aynı kategori | Nokta bazında aynı / ort. oran |
|---|---|---|---|
| Tōhoku | 34,3 / 38,6 m | 8 / 14 (%57) | %77 / ×5,2 |
| Noto | 13,8 / 6,6 m | 9 / 13 (%69) | %53 / ×3,4 |
| Maule | 16,1 / 17,2 m | 5 / 8 (%62) | %93 / ×1,6 |

Ölçüt ("bölgelerin çoğunda aynı") üçünde de sağlanıyor. SAPMALAR
(kullanıcıya bildirildi, 2026-11-23): (1) model fay uçlarının ötesinde fazla
veriyor (Tōhoku: Sahalin 5,2 / 0,2 m, Kuril 7,3 / 2,3, Hokkaido 17 / 6,7):
Riquelme Eş. 2'nin Lorentz azalması yavaş; Okada profili (makalenin FFM
sürümü) denendi ama fay ucunun hemen ötesindeki yüksek gözlemleri sıfırlıyor,
makaledeki analitik f(p) korundu. (2) Korunaklı körfezlerde çok fazla
(Tokyo, Kanagawa, Toyama: 12–18 m / 1,4–2,8 m): makale koyları kapsam dışı
sayar; kıyı noktası "koy içi" ayrımı yok. (3) Noto (kabuk içi, sığ deniz,
d = 78 m) en yüksekte ×2: Riquelme'nin geçerlilik alanı (dalma-batma) dışı.
Nokta bazında model ×1,6–5 fazla: formül EN YÜKSEK tırmanmayı verir; tek
bir noktadaki tırmanma bunun altında kalır (beklenen).


## Faz 9: GEM ortalama kayıp oranı, Kumamoto 2016, Ridgecrest 2019, Puebla 2017 (2026-11-23)

- Komut: `seismira-data validate-gem data/slab2/slab2.sslb` (2 s; GEM paketi
  `data/gem/gem.svul` gerekir, CI'da yok). Zincir uygulamayla aynı (bizim fay,
  Vs30, bölge rasterı, `damage::loss_regions`; dizilerde oranlar
  1 − (1 − r₁)(1 − r₂) ile birleşir). Olaylar USGS ComCat, Mww düğüm düzlemi:
  Kumamoto us20005hzn (Mw 6,2; 210/81/164) + us20005iis (Mw 7,0; 224/66/−151),
  Ridgecrest ci38443183 (Mw 6,4; 226/82/1) + ci38457511 (Mw 7,1; 321/84/−171),
  Puebla us2000ar20 (Mw 7,1, levha içi; 299/44/−82).
- Gözlem, Kumamoto: Kumamoto Valiliği 第253報 (2017-08-10) konut hasarı
  全壊 8 648 / 半壊 34 186 / 一部破損 153 557 bina. Sınıf → kayıp oranı:
  Kabine Ofisi hasar tespit yönergesi (府政防第518号; 全壊 ≥ %50, 半壊
  %20–50, 2016'da 一部破損 < %20). Alt sınır 0,5·全壊 + 0,2·半壊, üst sınır
  1·全壊 + 0,5·半壊 + 0,2·一部; payda GEM konut bina sayısı (563 634).
- Gözlem, Meksika: resmî eyalet tablosu BULUNAMADI; yalnızca haber
  kaynaklı, tarihe göre değişen konut sayıları (Expansión 2017-10-05 Edomex
  7 565 etkilenen / 1 800 tam kayıp; Puebla ilk sayım ~12 500 etkilenen;
  CDMX 5 765 (2 273 tam) ile 12 000–22 000 arası). Payda GEM konut
  (DWELLINGS). Bunlar KABA büyüklük kontrolüdür, ölçüt değildir.

| Bölge | model % (olası ±1τ) | gözlenen % | model / gözlenen |
|---|---|---|---|
| Kumamoto ili (JP-43) | 1,32 (0,70–2,38) | 2,0–10,0 | ×0,13–0,66 |
| Ōita, Nagasaki, Fukuoka… | ≤ 0,01 | (küçük) | — |
| California (US-CA) | 0,02 (0,01–0,03) | — | — |
| Morelos | 5,7 (1,7–15) | — | — |
| Puebla | 1,9 (0,6–5,5) | etkilenen ~0,6 | kayıp oranında ×3+ fazla |
| Ciudad de México | 1,1 (0,3–3,9) | etkilenen 0,2–0,7 | kayıp oranında ×1,6+ fazla |
| Edo. de México | 0,21 (0,05–0,85) | etkilenen 0,14, tam kayıp 0,03 | ×1,5–14 fazla |

Değerlendirme ve SAPMALAR (kullanıcıya bildirildi, 2026-11-23):
1. Kumamoto: model gözlenen aralığın ALTINDA (×0,13–0,66); +1τ ancak alt
   sınıra değiyor. Olası nedenler: (a) GEM YAPISAL zafiyet fonksiyonu, Japon
   "損害割合" ise çatı, duvar kaplaması gibi yapısal olmayan kısımları da
   sayar; (b) Japon tespiti cömert (yardım hakkı için başvuru), 一部破損
   çok hafif hasarı da içerir (alt sınır buna duyarsız); (c) 全壊/半壊
   sayıları bina değil başvuru bazlı olabilir (çift sayım); (d) bizim
   sarsıntımız (Kumamoto'nun ayrı şiddet doğrulaması henüz yok).
2. Meksika: model eyalet düzeyinde birkaç kat FAZLA görünüyor, ama gözlem
   sayıları zayıf. Sıralama tutuyor (Morelos > Puebla > CDMX > Edomex,
   haberlerde de en çok etkilenen Morelos ve Puebla).
3. Ridgecrest: Adm1 = eyalet olduğu için anlamlı karşılaştırma yok; model
   California geneline ihmal edilebilir oran veriyor (beklenen; seyrek
   nüfuslu bölge).
4. Ayar YAPILMADI. GEM çıktısı kartta "ortalama hasar oranı" olarak kalır;
   büyüklük mertebesi doğru, faktör 2–5 belirsizlik var. İyileştirme
   adayları: yapısal olmayan zafiyet (GEM'de varsa
   `vulnerability_nonstructural`), Meksika için CENAPRED "Impacto
   socioeconómico" 2017 raporu (sonraki oturumda indirilecek), Kumamoto
   şiddet doğrulaması (NIED K-NET / JMA).

### Ek (aynı gün): yapısal olmayan kayıp ve Kumamoto şiddeti

Kumamoto sapmasının nedenini ayırmak için iki kontrol yapıldı.

1. Şiddet (neden d): USGS ShakeMap atlas istasyon listesi
   (`data/validation/kumamoto2016/`, 346 NIED K-NET/KiK-net istasyonu, iki
   yatay kanal), ana şok Mw 7,0, 224/66, SS:
   `seismira-data validate-shakemap data/validation/kumamoto2016/stationlist.json 130.7543 32.7906 10 7.0 224 66 SS --vs30 data/vs30/vs30.sras`

   | Karşılaştırma | n | ort. ln(göz/tah) | std ln | ort. ΔI | \|ΔI\| < 1 |
   |---|---|---|---|---|---|
   | A: ShakeMap mesafeleri | 305 | −0,01 | 0,52 | −0,01 | %98 |
   | B: bizim geometri | 305 | −0,06 | 0,55 | −0,04 | %97 |
   | C: bizim geometri + bizim Vs30 | 305 | −0,08 | 0,55 | −0,06 | %97 |

   Ölçütler sağlandı. Yakın alanda model düşük: Rrup 0–25 km ΔI +0,30 (11
   istasyon), 25–50 km +0,13 (C). Hasarın yoğunlaştığı yerde PGV ~×1,2–1,4
   az; kayıp farkının küçük bir kısmı.
2. Yapısal olmayan kayıp (neden a): GEM `vulnerability_nonstructural.xml`
   eğrileri bina eğrisine katıldı (docs/bilim.md 7.8; şimdilik yalnızca
   Japonya ve Meksika dosyaları indirildi; aynı gün 215 ülkenin tümü
   indirildi, gem.svul 8 MB, eşi olmayan sınıf 0).

   | Bölge | yalnızca yapısal % | bina (yapısal + yapısal olmayan) % | gözlenen % |
   |---|---|---|---|
   | Kumamoto ili | 1,32 (0,70–2,38) | 2,68 (1,60–4,42) | 2,0–10,0 |
   | Morelos | 5,72 | 6,03 (2,05–15,35) | — |
   | Puebla | 1,94 | 2,20 (0,76–5,74) | etkilenen ~0,6 |
   | Ciudad de México | 1,14 | 1,51 (0,59–4,03) | etkilenen 0,2–0,7 |
   | Edo. de México | 0,21 | 0,39 (0,14–1,11) | etkilenen 0,14 |
   | California | 0,02 | 0,04 (0,02–0,07) | — |

   Kumamoto artık gözlenen aralığın İÇİNDE (alt sınıra yakın). Meksika'da
   yapısal olmayan eğri yapısala yakın olduğundan değişim küçük; model
   haber kaynaklı sayılara göre hâlâ birkaç kat fazla (gözlem zayıf).
   CENAPRED "Impacto socioeconómico 2017" raporu (cenapred.gob.mx ve
   cenapred.unam.mx, 415-IMPACTO_SOCIOECONOMICO_2017.PDF) betiğe 403
   veriyor; kullanıcıdan tarayıcıyla indirmesi istendi. Ayar YAPILMADI.

### Ek: Meksika gözlemi CENAPRED raporuyla (2026-11-23)

Kaynak: CENAPRED (2018), "Impacto socioeconómico de los principales
desastres ocurridos en la República Mexicana en 2017" (608 s.; CENAPRED
sitesi betiğe ve tarayıcıya 403 verdi, kullanıcı Internet Archive
kopyasını indirdi: web.archive.org/web/20251011110709/https://www.cenapred.unam.mx/es/Publicaciones/archivos/415-IMPACTO_SOCIOECONOMICO_2017.PDF;
yerelde data/raw/validation/puebla2017/, sha256 83ff9e4bf66fe53f…). 19 Eylül
depremi konut hasarı: Morelos Tablo 3.108 (SEDATU; kısmi 9 697, tam
6 104), Puebla Tablo 3.135 (FONDEN; 22 707 / 5 638), Edo. de México
Tablo 3.151 (SEDATU; tam 2 494 + taşınma 208, kısmi 2 109, hafif 1 249),
Guerrero metin (3 928; kısmi 1 443, tam 2 047), Tlaxcala metin (34),
Ciudad de México Tablo 3.71 (BİNA: kırmızı 529, yeşil 10 531, sarı 3 752).
Sınıf → kayıp oranı: Kumamoto'daki bantlar (VARSAYIM; FONDEN destek
tutarı kısmi / tam = 28 000 / 120 000 ≈ 0,23 ile uyumlu). Payda GEM konut
(CDMX'te GEM bina). Komut `seismira-data validate-gem` (gözlem kodda).

| Bölge | model % (olası) | gözlenen % | model / gözlenen |
|---|---|---|---|
| Kumamoto | 2,68 (1,60–4,42) | 1,98–10,0 | ×0,3–1,4 |
| Ciudad de México (bina) | 1,51 (0,59–4,03) | 0,69–1,90 | ×0,8–2,2 |
| Guerrero | 0,32 (0,10–0,97) | 0,11–0,23 | ×1,4–3,0 |
| Puebla | 2,20 (0,76–5,74) | 0,33–0,77 | ×2,9–6,6 |
| Morelos | 6,03 (2,05–15,4) | 0,61–1,33 | ×4,5–10 |
| Edo. de México | 0,39 (0,14–1,11) | 0,03–0,07 | ×5,4–12 |
| Tlaxcala | 0,86 (0,30–2,66) | ~0,002 | ×~300 |

SAPMA (kullanıcıya bildirildi, 2026-11-23): Meksika'nın orta eyaletlerinde
model resmî sayılara göre ×3–10 FAZLA (−1τ bile Morelos'ta gözlenenin
üstünde); CDMX ve Kumamoto aralıkta. Olası nedenler: (1) FONDEN / SEDATU
sayıları yalnızca afet ilan edilen belediyeler ve desteğe başvuran
konutlar (eksik sayım; Tlaxcala'da 34 konut bunu düşündürüyor); (2) GEM
Meksika zafiyet eğrileri (ağırlıkla donatılı / çerçeveli yığma) bu
deprem için fazla kırılgan olabilir; (3) sarsıntı: Puebla UNAM
istasyonlarında sapma yok (C +0,00), ama CDMX dışında yakın alan
istasyonu az. Ayar YAPILMADI; bölge kartındaki GEM oranı "büyüklük
mertebesi" olarak okunmalı.

## Faz 10: hazır senaryoların şiddet doğrulaması (2026-11-23)

Komut: `seismira-data validate-shakemap <stationlist.json> --scenario <kimlik>
--vs30 data/vs30/vs30.sras --slab data/slab2/slab2.sslb` (uygulamanın hazır
senaryosuyla birebir aynı deprem: ShakeMap kırılma modelinden L, W, eğim,
odak konumu; scripts/gen_scenarios.py). Karşılaştırma C (bizim geometri +
bizim Vs30), ortalama ΔI ve |ΔI| < 1 oranı; "önce" = varsayılan geometri
(WC94 boyutu, odak ortada; yukarıdaki bölümler).

| Deprem (istasyon) | Senaryo fayı | Önce C | Senaryo C |
|---|---|---|---|
| Tōhoku Mw 9,1 (1189) | 435 × 191 km, odak %46 | −0,32, %95 | **−0,18, %99** |
| İzmit (31) | 142 × 20 km, odak %31 (doğuya) | −0,44, %81 | **−0,32, %84** |
| Kumamoto (308) | 26 × 14 km, odak SW ucunda | −0,06, %97 | −0,04, %98 |
| Kahramanmaraş Mw 7,8 (191) | 268 × 15 km, odak %57 | +0,08, %91 | +0,07, %92 |
| Puebla (29) | 23 × 16 km, Ztor 46 km | +0,00, %97 | +0,07, %100 |

Sonuç: gerçek kırılma boyutu ve odak konumu büyük ve tek yönlü
kırılmalarda belirgin iyileştirme getiriyor (Tōhoku, İzmit), diğerlerinde
fark küçük. İzmit hâlâ ortalama ölçütünün (|ΔI| < 0,3) biraz dışında:
A satırı (ShakeMap mesafeleri) da −0,24, yani sapmanın çoğu GMPE'nin bu
olaydaki olaylar arası teriminden (ShakeMap'in kendi biası da negatif).
Kumamoto ve Puebla için ShakeMap kırılma modeli kısa (26 / 23 km); daha
uzun yayımlanmış modeller var, değiştirilmedi.

## NGA-West2 kayıtlarıyla CY14: İzmit 1999 ve Düzce 1999 (2026-11-24)

- Veri: PEER NGA-West2 flatfile, RotD50 d050 public sürüm (Ancheta vd.
  2014, Earthquake Spectra 30(3), doi:10.1193/070913EQS197M), kayıt
  gerektirmeyen kamuya açık dosya. Yeniden dağıtım izni sorulmuş, yanıt yok:
  ham dosya `data/raw/validation/nga/` altında, depoya GİRMEZ (dolayısıyla
  `cargo test`te değil). Türkiye satırları `turkey_d050.csv`.
- Komut: `seismira-data validate-nga <turkey_d050.csv> "Kocaeli, Turkey"
  --scenario izmit1999` ve `... "Duzce, Turkey"` (Düzce'nin hazır senaryosu
  yok: NGA odak parametreleri + otomatik WC94 boyutu, yani uygulamada elle
  girilen depremin aynısı). Vs30 haritası `data/vs30/vs30.sras`.
- A: NGA'nın kendi Mw, Rrup/Rjb/Rx, Ztor, eğim ve ölçülmüş Vs30 değeri
  (yalnız CY14). B: bizim geometri + ölçülmüş Vs30. C: bizim geometri +
  bizim Vs30 (uygulamadaki sonuç). Rrup > 300 km ve PGV'siz kayıtlar atlandı.

| Olay (kayıt) | A ln / ΔI / %<1 | B ln / ΔI | C ln / ΔI / %<1 | Vs30 harita/ölçülmüş |
|---|---|---|---|---|
| İzmit Mw 7,51 (28) | +0,24 / +0,19 / %100 | +0,08 / +0,07 | +0,10 / +0,08 / %100 | ×1,02 (std 0,34) |
| Düzce Mw 7,14 (26) | −0,29 / −0,23 / %92 | −0,40 / −0,31 | −0,32 / −0,25 / %100 | ×1,12 (std 0,35) |

(std ln: İzmit A 0,41, C 0,49; Düzce A 0,61, C 0,60.)

- **İzmit'teki negatif sapma gözlem verisinden kaynaklanıyor.** ShakeMap
  atlas `stationlist.json` ile NGA-West2'nin aynı istasyonlarında
  (27 eşleşme, ≤ 2 km) ShakeMap'in yatay PGV ortalaması NGA RotD50'den
  sistematik olarak düşük: ort. ln −0,46 (×0,63), std 0,31. Bileşen
  tanımı farkı bunu açıklamaz: ShakeMap'in BÜYÜK yatay bileşeni bile
  27 istasyonun 25'inde NGA RotD50'nin altında (ort. ln −0,33); büyük
  bileşen ortalamada RotD50'den büyüktür (Boore & Kishida 2017, `component.rs`).
  Nedeni belirlenmedi (atlasın eski/farklı işlenmiş kayıtları olabilir;
  doğrulanmadı). NGA-West2 kayıtları CY14'ün kendi geliştirme verisi olduğu
  ve tek tek işlendiği için daha güvenilir kabul edildi.
- Sonuç: NGA verisiyle İzmit senaryosu ölçüt İÇİNDE (C: ΔI +0,08, %100).
  Faz 10 tablosundaki −0,32 sonucu ShakeMap gözlemine özgüdür. Model
  değiştirilmedi; senaryo geometrisi (B) A'ya göre sapmayı küçültüyor.
- Düzce: ortalama ölçüt içinde (|ΔI| < 0,3, C −0,25), ama yakın alanda
  (Rrup < 20 km, 11 kayıt) model fazla tahmin ediyor (A −0,62 ln). Bunun
  çoğu "Lamont" geçici istasyonlarından (Rrup 0–25 km, ln −0,6 ile −1,6;
  ör. Lamont 1058 Rrup 0,2 km, PGV 14,6 cm/s). Aynı mesafedeki Düzce,
  Bolu ve IRIGM istasyonları modele yakın ya da üstünde. Uzak alanda
  (100–300 km, 10 kayıt) sapma ~0. Model değiştirilmedi.
- Vs30 haritası bu istasyonlarda ortalamada doğru (×1,02 / ×1,12), ama tek
  tek istasyonlarda ln std ~0,35 (ör. İzmit istasyonu 811 → 492 m/s,
  Ambarlı 175 → 344 m/s); C'nin istasyon saçılımı bu yüzden biraz büyük.

## Doğrultu atımlı fay uzunluğu: WC94 ↔ Thingbaijam vd. (2017) (2026-11-24)

Otomatik geometri (elle boyut YOK; odak konumu senaryodaki gibi), Vs30
haritasıyla (C). Komut: `validate-shakemap <istasyonlar> --vs30
data/vs30/vs30.sras <boylam enlem derinlik Mw doğrultu eğim SS odak>` ve
`validate-nga ... "Duzce, Turkey"`. Hücre: L (km) / C ort. ΔI.

| Olay | Önce (WC94) | Yalnız TMG17 L, W | Seçilen: min(WC94, TMG17) |
|---|---|---|---|
| İzmit Mw 7,6 (ShakeMap, 31) | 198 / −0,41 | 171 / −0,37 | 171 / −0,37 |
| Kahramanmaraş Mw 7,8 (191 → 185) | 320 / +0,05 | 234 / +0,14 | 234 / +0,14 |
| Kumamoto Mw 7,0 (308) | 46 / −0,08 | 67 / −0,15 | 46 / −0,08 |
| Düzce Mw 7,14 (NGA, 26) | 62 / −0,25 | 83 / −0,33 | 62 / −0,25 |

- Kahramanmaraş'ta fay kısalınca 6 istasyon CY14'ün 300 km sınırının
  dışına çıktı (n 191 → 185). Gerçek kırılma ~300 km'yi aşıyor (senaryo:
  268 km), yani TMG17 burada kısa; sapma ölçüt içinde kaldı.
- Mw ≥ 8 doğrultu atımlı için doğrulama verisi yok; değişikliğin asıl
  amacı ekstrapolasyonu kaldırmak (Mw 8,5: 1 657 → 700 km; TMG17 verisinin
  en uzun kırılması 580 km).
- İzmit regresyon testi (`izmit_1999_dogrulamasi_kayitli_degerlerde`) B
  satırı güncellendi: −0,39 / −0,49 → −0,36 / −0,45 (doğrultu 85°, odak
  ortada). Hazır senaryolar elle boyut kullandığı için etkilenmez.

## Senaryo şiddet doğrulaması tamamlama: Ridgecrest, Noto, Maule (2026-11-24)

Veri (USGS ShakeMap `stationlist.json`, kamu malı; `data/raw/validation/`,
depoda değil):
- Ridgecrest ci38457511: `us` sürüm 7 (ShakeMap 4, 1129 sismik istasyon;
  SHA256 a67a51b1…). Bölge ağının `ci` v14 dosyası eski biçimde (mesafe ve
  `station_type` yok), `cgs` v10 da öyle; `atlas` v1 ile `us` v7 ortak 924
  istasyonda PGV medyan ln farkı 0,00 (ortalama −0,17, birkaç uç değer).
- Noto us6000m0xl: `us` sürüm 10 (404 sismik; SHA256 223350c8…).
- Maule: `atlas` (35 sismik; SHA256 b307eba0…). Kanallar tek "peak"
  kanalı (H1/H2 yok). 2026-11-24'ten beri `compare_shakemap` bunu "UNK"
  gibi tek kanal olarak, değeri olduğu gibi okur (bileşen tanımı ShakeMap
  kılavuzunda doğrulanamadı: sayfa 403). Sonuç (Parker vd. 2020, bölge
  sa_s, senaryo geometrisi; 28 istasyon): A +0,48, B +0,44, **C +0,40 ΔI,
  %96 < 1 birim** (model az tahmin; 25–150 km'de +0,36…+0,59). "peak" büyük
  yatay bileşen ise Boore & Kishida (2017) oranı (M 8,8, 75 km: ln 0,12)
  kadar düşer → C ≈ +0,31. İki yorumda da ortalama ölçütünün biraz
  dışında; model değiştirilmedi.

Komut: `validate-shakemap <stationlist> --scenario <id> --vs30
data/vs30/vs30.sras --slab data/slab2/slab2.sslb`. C satırı, ort. ΔI / %<1:

| Deprem (istasyon) | A | C | Mesafeye göre C |
|---|---|---|---|
| Ridgecrest (683) | −0,41 / %88 | **−0,40 / %88** | her mesafede −0,1 … −0,7 (çoğu −0,35…−0,45) |
| Noto (351) | −0,67 / %70 | **−0,75 / %65** | 0–100 km −0,03…+0,04; 150–200 −0,84; 200–300 −1,11 |

İki olay da ÖLÇÜT DIŞI (ort. |ΔI| > 0,3; Noto %80 koşulunu da sağlamıyor).
A ≈ C: sapma geometri ya da Vs30'dan değil, yer hareketi modelinden (CY14)
geliyor. CY14'ün Japonya/İtalya anelastik düzeltmesi yalnızca 6 < M < 6,9
için tanımlı (pygmm uygulaması), Noto'yu etkilemez.

### Deney: dört NGA-West2 modeli (karar desteği)

`scripts/compare_ngaw2_models.py` (pygmm 0.9.1; yalnızca karşılaştırma):
ShakeMap mesafeleri + istasyon Vs30 (A tipi), ln(gözlem/tahmin) ortalaması.
"r": modelin bölgesel sürümü (Japonya: hepsi; Türkiye: yalnızca BSSA14'te
açık bölge, diğerleri temel model; Kaliforniya: temel). "ort4r": dört
bölgesel modelin eşit ağırlıklı ortalaması (ln uzayında).

| Olay | CY14 | BSSA14r | ASK14r | CB14r | ort4r |
|---|---|---|---|---|---|
| Kahramanmaraş (192) | +0,19 | −0,15 | +0,02 | +0,29 | +0,09 |
| İzmit (31, ShakeMap gözlemi) | −0,31 | −0,62 | −0,45 | −0,34 | −0,43 |
| Kumamoto (308) | −0,01 | +0,20 | +0,29 | +0,41 | +0,22 |
| Ridgecrest (684) | −0,52 | −0,16 | −0,60 | −0,25 | −0,38 |
| Noto (351) | −0,84 | −0,24 | −0,27 | −0,12 | −0,37 |

(İzmit ShakeMap gözlemi NGA'dan ~0,46 ln düşük; NGA'ya göre CY14 ≈ +0,15,
ort4r ≈ +0,03.) Ortalama |sapma| CY14 0,34 → ort4r 0,22; en kötü olay
(Noto) −0,84 → −0,37, uzak alan (200–300 km) −1,29 → −0,65. Kumamoto
kötüleşiyor (−0,01 → +0,22; uzak alan +0,03 → +0,54): Japonya'nın
bölgesel sönümlenmesi Kyushu'da gözlemden hızlı, orta Honshu'da yavaş.
Sonuç: model ortalaması (Faz 11 maddesi) en büyük sapmaları belirgin
küçültüyor; karar kullanıcıya soruldu.

## Dört NGA-West2 modeli uygulamada: doğrulama tekrarı (2026-11-24)

Model: `shaking::Gmm::Crust` (ASK14 + BSSA14 + CB14 + CY14, bölgesel;
docs/bilim.md "7.4 Dört NGA-West2 modelinin ortalaması"). Bölge merkez
üssünün ülkesinden (Türkiye, Japonya; ABD → temel model). Tüm doğrulama
araçları (`validate-shakemap`, `validate-nga`, `validate-damage`,
`validate-gem`, sıvılaşma / heyelan uçtan uca) uygulamayla aynı modeli ve
bölgeyi kullanır. Senaryo komutu: `validate-shakemap <stationlist>
--scenario <id> --vs30 data/vs30/vs30.sras --slab data/slab2/slab2.sslb`.

C (uygulamanın tam zinciri), ort. ΔI (JMA) / |ΔI| < 1 oranı:

| Deprem (kayıt) | Yalnız CY14 | Dört model | Mesafeye göre (dört model) |
|---|---|---|---|
| Kahramanmaraş (191) | +0,07 / %92 | **+0,01 / %94** | 0–25 km +0,30; 25–300 km −0,11…+0,05 |
| İzmit, NGA-West2 (28) | +0,08 / %100 | **−0,05 / %100** | — |
| İzmit, ShakeMap atlas (31) | −0,32 / %84 | −0,46 / %77 | (gözlem ~0,46 ln düşük, yukarıda) |
| Kumamoto (308) | −0,04 / %98 | +0,15 / %98 | 200–300 km +0,44 |
| Ridgecrest (683) | −0,40 / %88 | **−0,28 / %89** | 25–50 km −0,80 (8 ist.); diğer −0,16…−0,48 |
| Noto (351) | −0,75 / %65 | **−0,38 / %87** | 0–150 km −0,09…+0,11; 150–200 −0,45; 200–300 −0,61 |
| Düzce, NGA-West2 (26) | −0,25 / %100 | −0,34 / %88 | (yakın alan Lamont istasyonları) |

- Ortalama |ΔI| (İzmit NGA ile, 6 olay): 0,27 → 0,20; en kötü olay
  0,75 → 0,38. Ölçüt (|ΔI| < 0,3): önce 4 / 6, şimdi 4 / 6 (Ridgecrest
  girdi, Düzce çıktı); Noto ve Düzce sınırın biraz dışında.
- Noto'da yakın ve orta alan çok iyi; uzak alan (150–300 km, orta
  Honşu) hâlâ fazla tahmin. Kumamoto'da uzak alan (Kyushu) bu kez az
  tahmin: Japonya'nın bölgesel sönümlenmesi iki bölgede farklı; tek
  "Japonya" katsayısı ikisini birden yakalayamıyor. Model değiştirilmedi.
- Dalma-batma (Tōhoku, Puebla, Maule) etkilenmez (Parker vd. 2020).
- Hasar (Kahramanmaraş, `validate-damage`, aynı gün öncesi/sonrası): il
  ortalaması ln(model/gözlenen) −0,77 → −0,78 (×0,46), 11 il toplamı
  %9,1 → %8,4 (gözlenen bina %8,9). GEM (Kumamoto ili kayıp oranı): 2,68 →
  2,36 (gözlenen aralık 1,98–10,0); Ridgecrest, Puebla değişmedi.
- Sıvılaşma uçtan uca (Kahramanmaraş): oran 1,94 (önceki kayıt 2,00).
  Heyelan uçtan uca: oran 0,66 (önceki kayıt 0,69; arada GLiM 0,5°'ye
  geçildiği için önceki sayı birebir karşılaştırılabilir değil).
- Regresyon testi `izmit_1999_dogrulamasi_kayitli_degerlerde` yeni
  değerlerle güncellendi; ortalama ölçütü bu (ShakeMap atlas) gözlem
  setiyle artık aranmıyor, %80 koşulu korunuyor.

## Tsunami: korunaklı kıyılar (2026-11-24)

Değişiklik: kaynağı doğrudan görmeyen kıyılara (≥ 20 km kara engeli)
yakın alan yüksekliği verilmez, PTWC tehdidindeyse "dikkat" (sarı)
(docs/bilim.md 7.10 "KORUNAKLI KIYI"). `validate-tsunami` artık bu
kıyıları uygulamadaki kategorisiyle karşılaştırmaya katar (yükseklik
oranına katmaz). Bölge bazında kategori farkı (|gözlem − model|) toplamı:

| Deprem | Bölge aynı (önce → sonra) | Kategori farkı toplamı | Nokta bazında aynı | Korunaklı nokta |
|---|---|---|---|---|
| Tōhoku | 8 → 8 / 14 | 9 → 7 | %77 → %77 | 291 |
| Noto | 9 → 9 / 13 | 4 → 4 | %53 → %52 | 546 |
| Maule | 5 → 5 / 8 | 3 → 3 | %93 → %93 | 211 |

- Tōhoku: Sahalin 5,2 m (mor) → sarı (gözlem 0,2 m), Kanagawa 17,3 m
  (mor) → sarı (gözlem 1,8 m; bir kategori az), Japon Denizi kıyısı
  (Yamagata, Sakata 0,35 m) sarı. "TOKYO" bölgesinde model 17,9 → 11,0 m:
  kalan değer açık okyanustaki İzu adalarından (Hachijō); iç körfez
  (Harumi 1,6–2,8 m) korunaklı. NCEI'nin "YAMAGATA" etiketli 2,6 m'lik
  Hamanaka kaydı aslında Hokkaido'da (43,1°K 145,1°D): veri etiketi hatası.
- Noto: Fukui, Hyogo, Kyoto korunaklı ve PTWC tehdit bölgesinde (sarı;
  gözlem 0,3–0,6 m, uyumlu; Fukui önce bir kategori fazlaydı). Tottori ve
  Yamaguchi korunaklı ama tehdit bölgesi dışında (0; gözlem 0,6 / 0,2 m):
  Tottori önce uyumluydu, şimdi bir kategori az.
  Toyama hâlâ fazla (11,8 / 1,4 m): körfez Noto'ya açık, sorun kıyı
  boyunca azalma (Lorentz) ve kıyı şekli; korunma değil.
- Maule: Coquimbo korunaklı (sarı; gözlem 1,3 m, bir kategori az).
- 5 km eşik denemesi: Tōhoku nokta bazında %73, Noto %25 (yarımadanın
  arka kıyıları korunaklı sayıldı) → reddedildi. Tepe noktasını hedef
  almak: Noto %16 (tepe karada) → reddedildi.

## Hasar doğrulaması: Pazarcık hazır senaryo geometrisiyle (2026-11-24)

`validate-damage` artık Pazarcık ana şokunu uygulamanın hazır senaryosuyla
kurar (268 × 15 km, odak %57; ShakeMap kırılma modelinden), Elbistan
otomatik boyutla (146 km). Model: dört NGA-West2 ortalaması, Türkiye
bölgesi. Önce (aynı gün, otomatik 234 km, odak ortada) → sonra:

| İl | model / gözlenen (önce) | (sonra) |
|---|---|---|
| Adana | 9,12 | 9,33 |
| Adıyaman | 0,37 | 0,36 |
| Diyarbakır | 0,05 | 0,12 |
| Elazığ | 0,06 | 0,16 |
| Gaziantep | 1,42 | 1,44 |
| Kahramanmaraş | 0,44 | 0,46 |
| Malatya | 0,24 | 0,47 |
| Hatay | 0,31 | 0,33 |
| Kilis | 0,62 | 0,74 |
| Osmaniye | 0,67 | 0,67 |
| Şanlıurfa | 1,01 | 1,18 |

- İl başına ln(model/gözlenen): −0,78 (×0,46), std 1,44 → **−0,50 (×0,60),
  std 1,18**. 11 il toplamı %8,4 → %9,5 (gözlenen bina %8,9).
- Kalan sapma: en çok hasar gören illerde (Hatay, Adıyaman, Kahramanmaraş,
  Malatya) model gözlenenin ~0,35–0,5'i; Adana'da ×9. Sarsıntı tarafı
  istasyonlarla uyumlu (Kahramanmaraş C ΔI +0,01), dolayısıyla fark büyük
  ölçüde hasar zincirinde (ESRM20 kırılganlık / bina envanteri, gözlenenin
  konut ölçeği). Model değiştirilmedi.
- Ölçek denemesi (aynı gün): gözlem KONUT, model BİNA oranı. `validate-
  damage` artık ESRM20 Türkiye konut envanterinin DWELLINGS sütununu tip
  eşlemesiyle kırılganlık tiplerine dağıtıp aynı zinciri "konut stoku"yla
  da çalıştırır (konut binası ve konut ağırlıklı sütunlar). Sonuç: konut
  ağırlıklı il başına ×0,61 (std 1,27), 11 il %8,8 konut (gözlenen %17,3);
  bina oranıyla neredeyse aynı (×0,60). HİPOTEZ ÇÜRÜDÜ: ESRM20'de çok katlı
  betonarme sınıflar belirgin daha kırılgan olmadığından ağırlık sonucu
  değiştirmiyor. Kalan fark kırılganlık / yapı kalitesi tarafında
  (en çok hasar gören illerde ×0,33–0,43, Adana ×9–14); yayımlanmış bir
  bölgesel düzeltme bulunmadan değiştirilmedi.

## Tsunami: küçük adalar (2026-11-24)

Küçük ada (< 30 km) kıyılarına yakın alan yüksekliği verilmez (docs/bilim.md
7.10 "KÜÇÜK ADA"). Korunaklı kıyı adımına göre:

| Deprem | Bölge aynı | Kategori farkı toplamı | Nokta bazında aynı | Küçük ada noktası |
|---|---|---|---|---|
| Tōhoku | 8 → 9 / 14 | 7 → 5 | %77 → %77 | 66 |
| Noto | 9 → 8 / 13 | 4 → 5 | %52 → %52 | 15 |
| Maule | 5 → 5 / 8 | 3 → 3 | %93 → %91 | 25 |

- Tōhoku: İzu-Ōshima 13,4 m (mor) → sarı (gözlem 0,7 m, aynı); TOKYO
  bölgesinde Hachijō vb. adalar → sarı (gözlem en yüksek 2,8 m iç körfezde;
  bir kategori az).
- Noto: Shimane gözlemleri Oki adalarında; küçük ada sayıldı ve PTWC
  tehdit bölgesi dışında → 0 (gözlem 0,3 m; önce aynıydı).
- Üç depremde toplam kategori farkı 14 → 13 (korunaklı kıyıdan önce 16).

## Platform denemeleri (v0.1.0)

### macOS x86_64 (2026-11-23, kullanıcı)

- Makine: kendini "16 inç MacBook Pro, 10. nesil Intel" olarak tanıtıyor.
  Gerçek 16 inç MacBook Pro (2019) 9. nesil Intel ve AMD GPU ile geldiği
  için bu bir Hackintosh. Ekran kartı WhateverXe ile çalışıyor (Intel
  Xe-LP için WhateverGreen'den türetilmiş Lilu eklentisi; Apple'ın Xe
  sürücüsü olmadığından uyarlanmış sürücü paketi kurar,
  https://github.com/b00t0x/WhateverXe/wiki). Kullanıcıya göre başka
  uygulamalarda da pencere bozulmaları oluyor.
- 2. çalıştırma: tüm hesaplar log'da sorunsuz (şehirler, ızgara, tsunami,
  sıvılaşma, heyelan, hasar); görüntü normal.
- 1. çalıştırma (kayıtlı pencere durumu yokken, pencere elle büyütüldü):
  harita sağ paneli örttü, panel pencere dışına taştı. Fare hareketi ve
  yeniden boyutlandırmadan sonra da düzelmedi. Yeniden üretilemedi.
  Panel yerleşim kodu (egui 0.37) boyutları doğru sınırlıyor; sorunun
  Metal yüzeyi ile pencere boyutu uyumsuzluğundan (GPU sürücüsü / yama)
  kaynaklandığı düşünülüyor. Kodda değişiklik yapılmadı. Gerçek bir
  Mac'te görülürse yeniden ele alınacak.
- DÜZELTME (2026-11-24): aynı görüntü Linux'ta da çıktı (kullanıcı, hazır
  senaryo "Kahramanmaraş" seçildikten sonra; ekran görüntüsünde başlık
  "arları"). Sürücü değil, YERLEŞİM HATASI: sağa yaslı ayar panelinde bir
  öğe panelden geniş olunca içerik sola taşıyor, egui sol kısmı kırpıyor.
  Taşanlar: senaryo kutusundaki uzun seçili ad (260 px'te +264 px),
  "Tektonik ortam" / "Fay tipi" kutuları (dalma-batmada +51 px), "İleri
  seviye"deki etiket + kaydırıcı satırları (+87 px). Çözüm: kutular hücre
  genişliğinde ve metin "…" ile kısaltılır; kaydırıcıların etiketi üstte,
  genişliği kalan alana göre. Regresyon testi `panel_icerigi_tasmaz`
  (8 senaryo × TR/EN × 260 / 300 px, "İleri seviye" açık). macOS'taki
  bozulma büyük olasılıkla aynı hataydı (ilk açılışta panel 300 px).
- Windows: gerçek bilgisayarda denenmedi. VirtualBox Windows 11 VM'de
  (2026-11-23, v0.1.1 taslağı) exe açıldı, pencere içi siyah kaldı; log
  incelenmedi (VM'de gerçek GPU yok, sonuç gerçek Windows'u temsil
  etmez). 2026-11-24 kullanıcı kararı: Windows testi bırakıldı; README ve
  KURULUM.txt'ye "Windows'ta denenmedi" notu ve GL yedeği talimatı
  eklendi. Windows yalnızca CI'da (derleme + testler) doğrulanıyor.

## Çok parçalı fay izi: hazır senaryolar (2026-11-25)

Hazır senaryolardan kırılması ShakeMap'te parça parça çizilmiş olanlar artık
çok parçalı iz kullanıyor (scripts/gen_scenarios.py `chain_trace`, çekirdek
`rupture.rs`, bilim.md 7.1): Kahramanmaraş 15 parça / 285 km (Narlı kolu
atlandı, merkez üssü ana ize 18 km), İzmit 15 parça / 150 km (7 kuad, ≤ 10 km
basamaklar köprülendi), Ridgecrest 25 parça / 51 km (4 kol kuadı atlandı).
Diğer beş senaryo tek düzlem (tek kuad). Komut: `validate-shakemap
<stationlist> --scenario <id> --vs30 data/vs30/vs30.sras --slab
data/slab2/slab2.sslb [--no-trace]` (`--no-trace`: eski tek düzlem). Dört
NGA-West2 ortalaması (2026-11-24'ten beri), C satırı (bizim geometri + Vs30):

| Deprem (istasyon) | Tek düzlem ort. ΔI / std ln / %<1 | Çok parçalı iz |
|---|---|---|
| Kahramanmaraş (191/192) | +0,01 / 0,63 / %94 | −0,02 / **0,58** / **%96** |
| İzmit (31) | −0,46 / 0,65 / %77 | −0,44 / **0,60** / %77 |
| Ridgecrest (683/684) | −0,28 / 0,83 / %89 | −0,27 / 0,83 / %89 |

B satırı (bizim geometri, ShakeMap Vs30) Kahramanmaraş'ta +0,10 → +0,07: A
satırına (ShakeMap'in kendi mesafeleri, +0,07) eşitlendi, yani mesafeler artık
ShakeMap'in sonlu fayıyla tutarlı. NGA-West2 İzmit (28 kayıt, iz ile): C
ΔI −0,03, std 0,44, %100. Sonuç: dağılım küçüldü, ortalama değişmedi; iz
gerçeğe daha yakın. (Yukarıdaki Faz 10 tablosu CY14 tek model dönemindendir.)

## Yönelim (CY14 ΔDPP) doğrulaması: istasyon SA kayıtları (2026-11-26)

Komut: `seismira-data validate-directivity <stationlist.json> <senaryo>`
(`validate_directivity.rs`). Gözlem: iki yatay kanalın geometrik ortalaması;
zemin: istasyonun ShakeMap Vs30'u; Rrup ≤ 70 km (CY14 yönelim alanı); hazır
senaryo geometrisi (çok parçalı iz dahil). Kalıntı = ln(göz/tah). "Eğim":
yönelimsiz kalıntının ΔDPP'ye doğrusal eğimi (r korelasyon); yönelim gerçek
bir etkiyse pozitif. "Dört modele": deney, CY14 terimi dört NGA-West2
modeline uygulansaydı (ortalamada CY14 ağırlığı ¼, tam terim 4 × fark;
Watson-Lamprey, PEER 2017/04'ün önerdiği yön, rapor okunmadı).

| Olay (n, SA) | yönelimsiz std | CY14 (uygulanan) | dört modele | eğim (r) |
|---|---|---|---|---|
| Kahramanmaraş (51) SA 1 s | 0,750 | 0,743 | 0,725 | +0,23 (+0,43) |
| Kahramanmaraş (51) SA 3 s | 0,740 | 0,725 | 0,694 | +0,18 (+0,34) |
| İzmit (21) SA 1 s | 0,886 | 0,884 | 0,879 | +0,29 (+0,31) |
| İzmit (21) SA 3 s | 0,614 | 0,615 | 0,628 | +0,04 (+0,06) |
| Ridgecrest (25) SA 3 s | 0,728 | 0,730 | 0,751 | −0,02 (−0,03) |
| Kumamoto (63) SA 3 s | 0,571 | 0,551 | 0,503 | +0,27 (+0,51) |
| Noto (32) SA 3 s | 0,482 | 0,495 | 0,540 | −0,23 (−0,39) |

Sonuç: DPP üç olayda (Kahramanmaraş, İzmit SA 1 s, Kumamoto) gerçek
kalıntılarla pozitif ilişkili (yönelimi yakalıyor); Ridgecrest'te ilişki yok,
Noto'da (karmaşık ters fay, tek düzlem eşdeğeri) ters. Uygulanan etki
(yalnız CY14, ¼ ağırlık) std'yi en çok %3 düşürür. Dört modele yaymak iki
olayda iyileştirir (Kahramanmaraş −%6, Kumamoto −%12), üçünde kötüleştirir;
beş olay ortalamasında SA 3 s std değişimi ≈ 0. KARAR: yayımlanmış hal
(yalnız CY14) korunur; dört modele yayma kanıt yetersiz (kullanıcıya
bildirildi). PGV / şiddete etki ±%1.

## ASB14 (Akkar vd. 2014) Türkiye depremlerinde (2026-11-27)

Komut: `SEISMIRA_GMM_WEIGHTS=<ASK14,BSSA14,CB14,CY14,ASB14> seismira-data
validate-shakemap … --scenario <id> --vs30 data/vs30/vs30.sras --slab
data/slab2/slab2.sslb` ve `validate-nga … --scenario izmit1999` / `"Duzce,
Turkey"`. C (uygulamanın tam zinciri), ort. ΔI (JMA) / |ΔI| < 1 oranı (n).
ΔI > 0: model az tahmin ediyor.

| Deprem (gözlem) | Dört NGA-West2 (varsayılan) | Yalnız ASB14 | NGA + ASB14 yarı yarıya |
|---|---|---|---|
| Kahramanmaraş Mw 7,8, ShakeMap | −0,02 / %96 (192) | +0,24 / %93 (117, ≤ 200 km) | +0,05 / %95 (192) |
| İzmit, ShakeMap atlas | −0,44 / %77 (31) | −0,25 / %92 (26) | −0,35 / %87 (31) |
| İzmit, NGA-West2 kayıtları | −0,03 / %100 (28) | +0,16 / %100 (23) | +0,06 / %100 (28) |
| Düzce, NGA-West2 kayıtları | −0,34 / %88 (26) | −0,14 / %100 (26) | −0,24 / %100 (26) |

- Mesafeye göre (Kahramanmaraş, aynı istasyonlar): ASB14 0–25 km +0,46
  (NGA +0,13), 25–200 km +0,10…+0,21 (NGA −0,05…+0,05). ASB14 bu Mw 7,8
  depremde yakın alanı belirgin az tahmin ediyor; veri seti Mw 7,6'ya kadar
  (makale), yani bu deprem modelin veri aralığının üstünde.
- İzmit'te iki gözlem seti ters yönde: ShakeMap atlasıyla ASB14 daha iyi,
  NGA-West2 kayıtlarıyla dört model daha iyi (ShakeMap atlası NGA'dan ~%37
  düşük; yukarıdaki "NGA-West2 kayıtlarıyla CY14" bölümü). Düzce'de ASB14
  daha iyi.
- Sonuç: tek başına ASB14 de dört model de bu dört karşılaştırmada açık
  üstünlük göstermiyor; yarı yarıya karışım ortalama |ΔI|'yı 0,21'den
  (dört model) 0,18'e indiriyor ama Kahramanmaraş'ı biraz kötüleştiriyor.
  KARAR: varsayılan DEĞİŞMEDİ (2026-11-24 kullanıcı kararı); ASB14
  kullanıcı seçimi olarak sunuldu. Değişiklik kullanıcıya bildirildi.

## Hasar: il stokunun GHSL ağırlığıyla dağıtılması (deney, 2026-11-27)

Soru (inceleme 2026-11-24): ESRM20 Türkiye stoku il düzeyinde (81 nokta);
uygulama il stokunu ilin yerleşim hücrelerine EŞİT dağıtıyor. Yoğun kent
merkezi ile seyrek kırsal aynı payı alıyor. Hipotez: GHSL yapılaşma alanıyla
ağırlık Adana'daki fazla tahmini (×7) ve Hatay'daki az tahmini (×0,38)
düzeltir.

Komut: `SEISMIRA_GHSL=data/raw/ghsl/GHS_BUILT_S_E2020_GLOBE_R2023A_4326_30ss_V1_0.tif
seismira-data validate-damage …` (`damage::regions_weighted`, hücre ağırlığı
= GHSL m², il toplamı ilin tüm yerleşim hücrelerinden); `SEISMIRA_GHSL_QUANT=1`
ağırlığı 3 bitlik logaritmik düzeylere yuvarlar (bölge rasterının boş
bitlerine sığdırma denemesi). noDamageLimit düzeltmesinden sonra (eb42e01).

| | eşit (uygulama) | GHSL | GHSL 3 bit |
|---|---|---|---|
| il başına ln(model/gözlenen) ort. / std | −0,50 / 1,07 | −0,49 / 1,04 | −0,49 / 1,04 |
| Adana model/gözlenen | 7,37 | 7,23 | 7,17 |
| Hatay | 0,38 | 0,39 | 0,40 |
| Adıyaman | 0,40 | 0,46 | 0,46 |
| Malatya | 0,33 | 0,38 | 0,38 |
| Gaziantep | 1,11 | 1,01 | 1,03 |
| 11 il model (gözlenen bina %8,9) | %9,7 | %10,0 | %10,1 |

Sonuç: etki küçük (ortalama +0,01, std −0,03); Adana ve Hatay sapmaları il
içi dağılımdan kaynaklanmıyor (olası nedenler: bina stoku / kırılganlık
ayrımı, Hatay'da 20 Şubat artçısı, Adana'da gözlenen konut paydası).
KARAR: uygulama değiştirilmedi (veri biçimi değişikliğine değmez); deney
kipi doğrulama aracında kalır.
