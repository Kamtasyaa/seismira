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

### 2026-11-14 — Seyahat süreleri: tablo ↔ TauP

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
