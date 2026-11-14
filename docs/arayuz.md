# Arayüz, harita ve çizim

> CLAUDE.md v3’ten ayrıldı (Faz 0, 2026-11-14). Bölüm numaraları atıflar
> bozulmasın diye korunmuştur. "(doğrula)" işaretleri hâlâ geçerlidir.

## 3. Platform, metin, erişilebilirlik

- Windows, Linux ve macOS'ta BİREBİR AYNI görünüm ve davranış.
- Temel veri uygulamayla gelir; uygulama internetsiz çalışır.
- Tema: koyu, haritayı öne çıkaran, modern ve sade.

### 3.1 Metin ve diller

- Tüm metin UTF-8. Her yazı sistemi sorunsuz görünmeli: Latin, Kiril,
  Yunan, Arap, İbrani, Devanagari ve diğer Hint yazıları, Tay, Çince,
  Japonca, Korece vb.
- Yazı tipleri gömülür: Noto ailesiyle yedek (fallback) zinciri.
- Çince, Japonca ve Korece aynı kod noktalarını farklı gliflerle
  çizer; CJK yazı tipi sırası arayüz diline göre değişmeli.
- KRİTİK TEST (Faz 1): seçilen egui sürümü karmaşık yazı
  şekillendirmeyi (Arapça birleşme, sağdan sola yazım, Devanagari
  birleşik harfleri) doğru yapıyor mu? Yapmıyorsa şekillendirme
  kütüphanesi (ör. cosmic-text, parley, rustybuzz) entegre et.
- Çeviri altyapısı FAZ 1'DEN İTİBAREN kurulur (Fluent veya eşdeğeri).
  Koda gömülü arayüz metni yazılmaz. Başlangıç: Türkçe ve İngilizce.
- Şehir ve bölge adları kullanıcının dilinde gösterilebilir
  (GeoNames alternatif adları, Natural Earth yerelleştirilmiş adları).

### 3.2 Erişilebilirlik

- Yanıp sönen hiçbir öğe saniyede 3'ten fazla parlamaz (ışığa duyarlı
  epilepsi riski, WCAG 2.3.1). Bizim sınırımız: yaklaşık 1 Hz,
  yumuşak geçişli.
- "Hareketi azalt" ayarı: tüm yanıp sönmeler kapanır, yerine sabit
  kalın çizgi gelir.
- Renk tek bilgi taşıyıcı olmaz: Shindo kutusunda rakam, tsunami
  kategorisinde etiket bulunur.


## 6. Kullanıcı girdileri (sağ panel)

Ana ayarlar:
- Hazır senaryolar (Bölüm 14'teki gerçek depremler)
- Konum: haritaya tıklayarak veya enlem/boylam girerek. Kullanıcı
  haritayı serbestçe gezip depremi istediği yerden başlatır; şehir
  listesinden konum seçme yoktur (2026-11-14 kullanıcı kararı).
- Büyüklük (Mw)
- Odak derinliği (km)
- Tektonik ortam: sığ kabuk / dalma-batma arayüzü / levha içi
  (konuma göre otomatik öneri; kullanıcı değiştirebilir)
- Fay tipi: doğrultu atımlı / normal / ters
- Fay doğrultusu (strike), eğimi (dip), kayma açısı (rake).
  Varsayılanlar fay tipinden ve dalma-batmada Slab2'den gelir.
- Ayar sırasında haritada doğrultuyu gösteren ince bir önizleme
  çizgisi olur (bu, sonuçtaki kırılma izi değildir).

"İleri seviye" (katlanabilir):
- Odağın fay üzerindeki konumu, kırılma yönü
- Kırılma uzunluğu ve genişliği (varsayılan: otomatik)
- Zemin etkisi açık/kapalı
- Erken uyarı sistemi varsayımı (Bölüm 7.3)

Görünüm:
- Şiddet ölçeği: Shindo / MMI / ikisi birlikte
- Katmanlar: bölge sınırı vurgusu, sıvılaşma, heyelan, tsunami
- Simülasyon sonunda fay izini göster (varsayılan: kapalı)
- Hareketi azalt

Geçerlilik kuralları (yaklaşık değerler; her modelin yayımlanmış
aralığından doğrula, kodda sabit olarak tut):

| Ortam | Mw | Odak derinliği | Not |
|---|---|---|---|
| Sığ kabuk | 4.0 – ~8.5 (ters/normal ~8.0) | 0 – ~35 km | |
| Dalma-batma arayüzü | ~4.5 – 9.5 | ~5 – ~60 km | yalnızca ters fay |
| Levha içi | ~4.5 – ~8.0 | ~30 – ~300 km | |
| Derin odak | | > ~300 km | modeller geçersiz, "kaba tahmin" etiketi |

Aralık dışı girdi engellenir ya da açıkça uyarılır. Sessiz
ekstrapolasyon yapılmaz.


## 8. Harita ve çizim

- İzdüşüm: Web Mercator, ±85° ile sınırlı, yatayda dünya tekrar eder.
- 180° meridyeni: geometri veri hazırlamada ±180°'de bölünür.
  Pasifik'i geçen halka ve tsunami doğru çizilmeli.
- Dalga halkaları ekran dairesi DEĞİLDİR: merkez üssünden jeodezik
  olarak hesaplanan noktaların izdüşümüdür.
- egui yalnızca dışbükey çokgen doldurur. Kara çokgenleri veri
  hazırlamada üçgenlenir (earcut) ve hazır mesh olarak saklanır.
- Uygulama (Faz 1): geometri `.smap` dosyalarından GPU tamponlarına bir
  kez yüklenir (egui-wgpu paint callback, map/gpu.rs). Her karede
  yalnızca merkez/ölçek/renk uniform'u değişir. Yatay tekrar instancing
  ile (her örnek bir dünya kopyası). Çizgiler 1 piksel (LineStrip,
  primitive restart); kalın/yanıp sönen sınırlar Faz 4'te ayrıca çizilir.
  MSAA 4x.
- Ayrıntı düzeyi eşikleri (dünya genişliği, point; görsel ayar):
  < 1500 → 110m, < 5000 → 50m, üstü → 10m. İl/eyalet sınırları 5000'den
  itibaren 10m verisinden çizilir (50m Admin-1 yalnızca birkaç ülkeyi
  kapsar).
- Ayrıntı düzeyi: Natural Earth 1:110m, 1:50m, 1:10m yakınlaştırmaya
  göre. Şehir ölçeğinde kıyı kaba kalır; yüksek çözünürlüklü kıyı
  bölge paketlerinde (OSM kara çokgenleri, ODbL).
- Izgara katmanları enlem/boylam ızgarasında hesaplanır, Mercator'a
  göre yeniden örneklenerek tek doku olarak çizilir.
- Sınırlar Natural Earth'ün varsayılan gösterimidir, siyasi görüş
  bildirmez; Veri Kaynakları sayfasında belirtilir.

## 9. Bölge sınırı vurgusu (il / eyalet / bölge)

Birim: birinci düzey idari bölge (Türkiye'de il, ABD ve Meksika'da
eyalet, Japonya'da prefektörlük, Fransa ve İtalya'da bölge).

Her bölgenin durumu simülasyon zamanının saf fonksiyonudur (ileri-geri
sarmada tutarlı kalır):
- Uyarı: t ≥ t_uyarı, bölge uyarı ölçütünü sağlıyor ve S henüz
  ulaşmadı → sınır BEYAZ yanıp söner.
- Sarsıntı: bölgedeki ilk hücreye S ulaştı → sınır, o ana kadar
  bölgede ulaşılan en yüksek Shindo'nun renginde yanıp söner; değer
  arttıkça renk güncellenir.
- Yatışmış: bölgedeki sarsıntı bitti → yanıp sönme durur, sınır sabit
  renkte kalır, bölge çok hafif aynı renkle dolar.
- Nihai en yüksek Shindo'su 3'ün altında kalan bölgeler yanıp sönmez;
  yalnızca ince sabit renkli sınır alır (eşik ayarlanabilir).
- Tsunami ayrı kanaldır: uyarı yürürlükteyken kıyı çizgisi (deniz
  tarafında kalın bant) sarı / kırmızı / mor yanıp söner. Böylece
  Shindo renkleriyle karışmaz.

Yanıp sönme kuralları:
- Yumuşak opaklık dalgası (~%35–%100), yaklaşık 1 Hz, asla 2 Hz üstü.
- Duvar saatine bağlıdır, simülasyon hızından bağımsızdır.
- Tüm öğeler aynı fazda yanıp söner.
- Hızlı oynatmada bir durum çok kısa sürerse görsel en az ~1 saniye
  ekranda kalır (yalnızca sunum; simülasyon durumu değişmez).
- "Hareketi azalt" açıkken yanıp sönme yok, sabit kalın çizgi var.

Uygulama notları:
- Veri hazırlamada, sarsıntı ızgarasıyla hizalı bir bölge kimliği
  rasterı (u16) üretilir; bölge değerleri buradan toplanır.
- Çizimde yalnızca opaklık değişir, geometri yeniden üretilmez.
- Dünya ölçeğinde yalnızca etkilenen bölgelerin sınırı çizilir.
- Bölgeye tıklayınca bölge kartı açılır.
- Japonya'da isteğe bağlı: JMA'nın kendi şiddet ve tsunami tahmin
  bölgeleri.

## 10. Arayüz düzeni

- Orta: harita.
- SAĞ panel: konfigürasyon (Bölüm 6) + "Başlat".
- SOL ÜST: bildirim akışı, zaman sırasıyla. Örnekler:
  "Erken uyarı: Kocaeli, Sakarya, İstanbul",
  "P dalgası İstanbul'a ulaştı", "İzmit: Shindo 6+",
  "Tsunami uyarısı: Marmara kıyısı (KIRMIZI)".
  Öneme göre renklenir; tıklanınca harita oraya gider.
- SOL panel yalnızca bildirim akışıdır. (v3'teki "şehir listesi"
  2026-11-14'te kullanıcı kararıyla kaldırıldı; şehir bilgileri haritada
  şehre tıklanınca açılan şehir kartında ve bildirimlerde gösterilir.)
- ALT: zaman çizgisi. Oynat/duraklat, ileri-geri sar, "sonraki olaya
  atla". Hızlar: 1x, 10x, 60x, 600x, 3600x (tsunami saatler sürer).
- Şehir kartı: PGA/PGV, Vs30 ve zemin sınıfı, Shindo + MMI ve olası
  aralık, tahmini hasar, S − P, erken uyarı süresi.
- Bölge kartı: en yüksek Shindo, tahmini hasar (bölge düzeyi),
  tsunami kategorisi ve varış süresi.
- Köşede ölçek açıklaması (Shindo renkleri JMA paletinden; tsunami
  sarı/kırmızı/mor).
- Şehirler nüfusa ve yakınlaştırmaya göre gösterilir.
- "Veri Kaynakları ve Lisanslar" sayfası (zorunlu).
- Kalite ayarı: Düşük / Orta / Yüksek / Ultra (ızgara çözünürlüğü
  değişir; akıcılık hiçbir ayarda bozulmaz).


## Doğrulama notu: egui 0.37 karmaşık yazı testi (2026-11-14)

Yöntem: egui_kittest (wgpu, ekran dışı) ile Noto yazı tipleri yüklenip
örnek metinler PNG'ye çizildi, aynı metinler `pango-view` ile referans
olarak çizilip karşılaştırıldı.

| Yazı sistemi | egui 0.37 sonucu |
|---|---|
| Latin (Türkçe İ/ı/ş/ğ), Kiril, Yunan | Doğru; fi/fl bitişik harfleri ve kerning var (harfrust, PR #8031) |
| Devanagari birleşik harfleri (क्ष, त्र, प्र) | Doğru |
| Tay (üst/alt işaretler) | Doğru (satır kaydırma sözlüğü yok; kısa etiketlerde sorun değil) |
| Çince, Japonca, Korece | Doğru (yazı tipi sırası dile göre ayarlanmalı, Bölüm 3.1) |
| Arapça / İbranice tek kelime | Doğru (harfler bitişik, sağdan sola) |
| Arapça / İbranice çok kelime | YANLIŞ: kelimeler soldan sağa diziliyor ("تل أبيب" → "أبيب تل") |
| Arapça-Hint rakamları (٧٫٨) | YANLIŞ: ters sırada veya hiç çizilmiyor |
| Karışık yön (Arapça + Latin) | YANLIŞ |

Neden: epaint kaynak kodunda bidi (UAX #9) yok; `unicode.rs` içinde
"TODO: heed bidi characters" notu var. Şekillendirme kelime kelime
yapılıyor, kelimeler hep soldan sağa diziliyor.

Durum: egui deposunda tam RTL/bidi desteği için açık PR'lar var (#8836,
#8818, #8822, #8577; 2026-11 itibarıyla incelenmemiş).

Karar (2026-11-14): egui 0.37 ile devam. Arayüz dilleri şimdilik TR/EN
(soldan sağa), bu yüzden sorun Faz 3'e (şehir adları kullanıcının dilinde)
kadar görünmez. Faz 3 başında egui'nin bidi desteği yeniden kontrol edilir;
gelmemişse `unicode-bidi` ile kelime sırasını düzelten geçici bir katman
yazılır. Tam çözüm (parley) son çaredir. Sağdan sola örnekleri regresyon
testi olarak depoda tutulur.

## Geçerlilik kuralları: uygulama durumu (Faz 2, 2026-11-14)

Bölüm 6 tablosu `seismira-core/src/quake.rs` içinde sabit olarak duruyor
ve henüz DOĞRULANMADI. Kural: Mw ve derinlik aralık dışıysa, arayüzde
ters fay dışı seçilmişse ya da değer geçersizse Başlat kapalıdır (⛔).
300 km'den derin odak engellenmez ama "kaba tahmin" uyarısı verilir (⚠).
Faz 3'te CY14, Faz 5'te Parker vd. (2020) eklenirken her modelin
yayımlanmış aralığı makaleden alınıp bu sabitler güncellenecek.
Varsayılan eğim/kayma açıları (doğrultu atımlı 90°/0°, normal 55°/−90°,
ters 40°/90°) da doğrulanacak değerlerdir.

Doğrultu önizlemesi şimdilik yalnızca YÖN gösterir (ekranda sabit
uzunluk); kırılma uzunluğu Faz 3'te kaynak geometrisiyle gelecek.
Taslak değişince (haritaya tıklama dahil) çalışan simülasyon iptal edilir.

## Shindo kutusunun kademeli yükselişi (2026-11-14, kullanıcı isteği)

S dalgası şehre ulaşınca kutu son değeri hemen göstermez: 1'den başlar,
her `RISE_STEP_S` = 0,4 s'de bir sınıf artar (1 → 2 → 3 → 4 → 5- …) ve son
değerde durur. Güçlü sarsıntı daha çok adım, yani daha geç tepe demektir
(7 için ~3,2 s). Simülasyon zamanının saf fonksiyonudur
(`SiteResult::shindo_at`); geri sarma ve hızlandırma tutarlıdır. Bölge
bildirimi, kutunun son değere vardığı anda (`t_peak`) düşer. Şehir kartı
tahmin olarak her zaman son değeri gösterir. Bu GÖRSEL bir kabuldür; Faz 5'te
alt fay zamanlaması (t_tepe) ile fiziksel tepe zamanına bağlanacak.

Kutusu komşu kutuyla çakıştığı için çizilmeyen şehirlerde (kalabalık
azaltma) nokta o anki Shindo renginde çizilir; değer fare ile kartta
okunur. Kart, kutunun ya da noktanın üstüne gelince açılır.

## Sarsıntı alanı (ızgara) katmanı (Faz 4 adım 2, 2026-11-14)

- Hesap: `seismira-core/src/grid.rs`. Alan, Rrup ≤ 300 km eğrisinin sınır
  kutusu; hücreler Vs30 haritasının 30″ hücrelerine hizalı (yeniden
  örnekleme yok). Önce 8 kat kaba, sonra ince geçiş; ikisi de arka planda,
  iptal edilebilir. Su / buz ve model dışı hücreler boş.
- Ölçüm (referans makine, rayon 8 iş parçacığı): Mw 7.5 Marmara 1077×667
  hücre ince geçiş ~130–180 ms, kaba ~2–40 ms; Mw 8: 1577×697, ~200 ms.
  Bu yüzden Bölüm 11'deki "kaynağa yakın ince, uzakta kaba" ayrımına şimdilik
  gerek olmadı: tüm alan Vs30 çözünürlüğünde. Kenar > 2048 hücreyse adım
  büyür (GPU doku sınırı; GL yedeği dahil).
- Çizim: ızgara bir kez Rg32Float dokuya yüklenir (son sınıf, S varışı);
  shader ekran noktasını Mercator'dan enlem/boylama çevirip hücreyi okur
  ve şehir kutularıyla aynı kademeli yükselişi uygular (zamanın saf
  fonksiyonu; her karede yalnızca uniform güncellenir). Kara ve göllerin
  üstünde, sınırların altında; opaklık %70 (görsel ayar; %45'te sarılar
  koyu zeminle zeytin rengine dönüyordu).
- Görünüm menüsü → "Sarsıntı alanı" ile kapatılabilir.
- Bilinen: açık renkli (4, 5-) alanlar üstünde şehir adları zor okunuyor.
