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

