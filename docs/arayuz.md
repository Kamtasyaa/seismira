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

#### Erişilebilirlik denetimi (Faz 10 adım 6, 2026-11-23)

Ölçüt: WCAG 2.2 AA (metin ≥ 4,5:1, grafik öğe ≥ 3:1; göreli parlaklık
formülüyle hesaplandı, egui 0.37 koyu tema değerleri kaynağından).

| Öğe | Önce | Sonra | Değişiklik |
|---|---|---|---|
| Panel metni | 5,2 | 8,0 | egui gri 140 → 175 (theme.rs) |
| Soluk metin (panel / pencere / açıklama) | 2,6–2,8 | 4,6–5,4 | %60 alfa → gri 135 |
| Şehir adı / kara | 3,7 | 6,9 | ×0,85 kaldırıldı |
| Shindo 2 kutu yazısı | 2,6 (beyaz) | 8,2 (siyah) | yazı rengi en yüksek kontrastla (shindo_style::text_on) |
| Shindo 6-, MMI IX kutu yazısı | 3,8 / 4,0 | 5,6 / 5,3 | aynı kural |
| Koyu kutular / kara (Shindo 3, 6+, 7, MMI X) | 1,4–2,3 | açık kenar | kenar rengi dolgunun kara kontrastına göre |

- Değişmeyenler (uygun bulundu): eğitim uyarısı 9,3; engel / uyarı
  metinleri 6,4 / 9,3; bağlantı 5,8; tsunami renkleri kara üstünde ≥ 3,5;
  yanıp sönme 1 Hz: bölge ve zemin kareleri kosinüs (yumuşak), tsunami
  kıyıları açık/kapalı (2026-11-25 kullanıcı kararı: periyodun %60'ı tam
  görünür, %40'ı kapalı; 1 Hz, ışığa duyarlılık sınırı olan saniyede 3
  yanıp sönmenin altında). "Hareketi azalt" üçünü de sabitler.
- Renk tek taşıyıcı: tsunami şeridinin kalınlığı kategoriye göre (dikkat
  3, uyarı 4,5, büyük uyarı 6 point; açıklamada da). Shindo / MMI
  kutularında etiket; zemin karelerinde S / H harfi.
- Klavye: merkez üssü enlem / boylam kutularıyla girilebilir (fare
  gerekmez); menüler egui + AccessKit (eframe `accesskit` özelliği açık).
  Harita gezinmesi fare / dokunmatik ister (bilinen sınır).
- Yazı boyutu: eframe Ctrl + / Ctrl − ile yakınlaştırma (varsayılan açık).
- Test: `shindo_style::tests::kutu_yazilari_okunur` (tüm Shindo ve MMI
  kutularında ≥ 4,5:1).
- Ekran görüntüsüyle kontrol: YAPILMADI (ekran kilitliydi), sonra.


## 6. Kullanıcı girdileri (sağ panel)

Ana ayarlar:
- Hazır senaryolar (Bölüm 14'teki gerçek depremler). UYGULANDI (Faz 10
  adım 2, 2026-11-23): panelin en üstünde açılır liste; seçince taslak
  dolar, harita merkez üssüne gider, altında kaynak notu çıkar; bir değer
  elle değişince "Özel (elle)" olur. Veri: data/scenarios/scenarios.json
  (scripts/gen_scenarios.py: USGS ComCat merkez üssü/derinlik/Mww düzlemi
  + ShakeMap rupture.json'dan tek düzlemli eşdeğer L, W, eğim, odak
  konumu; ayrıntı betiğin başında). Kahramanmaraş: yalnızca Mw 7,8 ana şok
  (uygulama tek deprem).
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
  UYGULAMA (2026-11-20): dalma-batma ortamı seçilip merkez üssü konunca
  panel Slab2 zonunu, levha yüzeyinin derinliğini, doğrultusunu, eğimini ve
  Parker bölgesini gösterir. Arayüz depreminde "Levha geometrisini uygula"
  düğmesi doğrultu, eğim, odak derinliği (= levha yüzeyi) ve kayma açısını
  (90°) doldurur; kendiliğinden UYGULANMAZ (kullanıcının girdiğini ezmemek
  için). Levha 40 km'den derinse düğme yerine uyarı çıkar; levha içinde
  odak levha yüzeyinin 10 km'den fazla üstündeyse uyarı çıkar. Merkez
  üssünün altında levha yoksa sarı UYARI (gri not değil; kullanıcı
  denemesi 2026-11-20, İyon Denizi'nde Mw 9 arayüz): orada dalma-batma
  depremi beklenmeyebilir, hesap küresel katsayılarla. Engellenmez, çünkü
  Slab2 her zonu kapsamaz (Kıbrıs, Cebelitarık yayları yok).
- Ayar sırasında haritada doğrultuyu gösteren ince bir önizleme
  çizgisi olur (bu, sonuçtaki kırılma izi değildir).

"İleri seviye" (katlanabilir):
- Odağın fay üzerindeki konumu, kırılma yönü
- Kırılma uzunluğu ve genişliği (varsayılan: otomatik). UYGULANDI (Faz 10
  adım 1, 2026-11-23): İleri seviye → "Kırılma boyutunu elle gir"; açılınca
  o anki otomatik değerlerle başlar. Girdi 1–1500 × 1–250 km (dışı
  engellenir); genişlik tabana sığmazsa kırpılır ve uyarılır; alan WC94'ün
  ×4,1 dışındaysa (±2,5σ) uyarılır; elle boyutta "Mw 7,9 üstü boyut
  ekstrapolasyonu" uyarısı gösterilmez. Hazır senaryolar gerçek boyutu
  bununla verir.
- Zemin etkisi açık/kapalı
- Erken uyarı sistemi varsayımı (Bölüm 7.3)

Görünüm:
- Şiddet ölçeği: Shindo / MMI / ikisi birlikte (Faz 10'a ertelendi,
  2026-11-20; MMI şimdilik şehir kartında). UYGULANDI (Faz 10 adım 5,
  2026-11-23): Görünüm → Şiddet ölçeği: JMA Shindo (varsayılan) / MMI
  (kaydedilir). MMI seçilince harita ızgarası, şehir kutuları (Roma
  rakamı) ve sağ alttaki ölçek MMI olur: hücre / şehir değeri Worden vd.
  (2012), büyük bileşen PGV'den (mesafe katsayısı 0); renkler USGS
  ShakeMap ArcGIS servisi katman "mi" (crates/seismira-app/src/mmi_style.rs).
  I gösterilmez, II–III aynı renk. Kademeli yükselme Shindo ile aynı anda
  tamamlanır. "İkisi birlikte" YAPILMADI (kutular iki kat yer kaplar).
  Geliştirme: `SEISMIRA_SCALE=mmi`.
  DEĞİŞTİ (2026-11-24, kullanıcı şikâyeti "ölçekler karışık"): MMI
  seçiliyken bildirimler, bölge sınırı / dolgu renkleri, şehir kartı
  (kutu + ilk satır MMI, Shindo satırı ikinci) ve bölge kartı da MMI
  yazar. Bölgenin en yüksek ve yaygın MMI'si ızgara hücrelerinin büyük
  bileşen PGV'sinden (en yüksek PGV; seçilen hücrelerin %10'unun ulaştığı
  PGV), ızgarayla aynı Worden vd. (2012) dönüşümüyle; kademeli yükselme
  Shindo ile aynı anda tamamlanır. Ölçekten BAĞIMSIZ kalanlar (JMA
  ölçütü): erken uyarının yayımlanması ve kapsamı ("Shindo 4'ün altı"
  yazısı), hangi bölge / şehir bildiriminin akışa gireceği (Shindo 5- ve
  üstü + en güçlü 3) ve turuncu vurgu, yanıp sönme eşiği (Shindo 3). MMI
  için yayımlanmış bir uyarı eşiği seçilmedi; seçim ölçüte göre, gösterim
  ölçeğe göre yapılır.
- Katmanlar: bölge sınırı vurgusu, sıvılaşma, heyelan, tsunami
- Simülasyon sonunda fay izini göster (varsayılan: kapalı). UYGULANDI
  (Faz 5, 2026-11-20): Görünüm menüsünde, kaydedilir; zaman çizgisinin
  sonunda (S cephesi 100°'ye ulaşınca, ⏭ ile) hesapta kullanılan sonlu fay
  çizilir: üst kenar kalın beyaz, yüzey izdüşümü ince. İlerleyiş animasyonu
  yok. Simülasyondan önceki soluk önizleme ayrıdır (kırılma izi değildir).
- Hareketi azalt

Geçerlilik kuralları (yaklaşık değerler; her modelin yayımlanmış
aralığından doğrula, kodda sabit olarak tut):

| Ortam | Mw | Odak derinliği | Not |
|---|---|---|---|
| Sığ kabuk | 4.0 – ~8.5 (ters/normal ~8.0) | 0 – ~35 km | |
| Dalma-batma arayüzü | 4.5 – 9.5 | ~5 – 40 km | yalnızca ters fay |
| Levha içi | 4.5 – 8.5 | ~30 – 200 km | |
| Derin odak | | > 200 km | model yok: uyarı, dalgalar gösterilir, şiddet hesaplanmaz |

Dalma-batma satırları Parker vd. (2020) raporundan DOĞRULANDI
(2026-11-20, docs/bilim.md); alt derinlik sınırları (5 / 30 km) modelden
değil, ortam sınıflandırmasından gelen yaklaşık değerlerdir.

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
  UYGULAMA (Faz 6): "Tahmini bina hasarı (ESRM20, ilde N bina)", "Ağır
  hasar veya göçme: ~X bina (%p; olası %a–b)", "Orta hasar: ~Y bina (%q)".
  Şehir kartında yalnızca oran, "ilin bina dağılımıyla" notuyla. Hasar
  sonucu bölge özetinden sonra ayrı mesajla gelir (Mw 7,8'de ~25 ms).
- Köşede ölçek açıklaması (Shindo renkleri JMA paletinden; tsunami
  sarı/kırmızı/mor).
- Şehirler nüfusa ve yakınlaştırmaya göre gösterilir.
- "Veri Kaynakları ve Lisanslar" sayfası (zorunlu). UYGULANDI (Faz 10
  adım 4, 2026-11-23): Yardım → "Veri kaynakları ve lisanslar" penceresi,
  data/SOURCES.toml'dan (uygulama klasöründe gelir) üretilir: ad, lisans,
  "değiştirildi", "dağıtılan pakette yok" (distributed = false), atıf
  (seçilebilir), bağlantı. `validation_only = true` kayıtlar (yalnızca
  doğrulama verisi) gösterilmez. Giriş metninde kodun lisansı
  (GPL-3.0-or-later). EKSİK: kayıt adları ve atıflar dosyada Türkçe /
  özgün dilde; İngilizce arayüzde de öyle görünür (ileride `name_en`).
  Geliştirme: `SEISMIRA_SOURCES=1` pencereyi açık başlatır.
- Kalite ayarı: Düşük / Orta / Yüksek / Ultra (ızgara çözünürlüğü
  değişir; akıcılık hiçbir ayarda bozulmaz). UYGULANDI (Faz 10 adım 3,
  2026-11-23): Görünüm → Kalite (kaydedilir; varsayılan Yüksek). İnce
  ızgaranın en büyük kenarı 512 / 1024 / 2048 / 2048 hücre (taban 30″;
  küçük depremde hepsi tam çözünürlük), bina hasarı örnekleme adımı ince
  adımın 4 / 2 / 2 / 1 katı. Açık simülasyonda değişince arka plan hesabı
  yeniden başlar (saat korunur). Tōhoku Mw 9,1 (referans makine): ince
  ızgara 0,27 / 0,33 / 0,46 / 0,45 s, hasar 0,11 / 0,14 / 0,25 / 0,71 s.
  Geliştirme: `SEISMIRA_QUALITY=low|medium|high|ultra`.


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
Varsayılan eğim/kayma açıları (doğrultu atımlı 90°/0°, normal 50°/−90°,
ters 40°/90°): eğimler 2026-11-24'te USGS OFR 2010-1296 Tablo 2'den
doğrulandı (docs/bilim.md 7.1).

Ek denetimler (2026-11-15), yalnızca sığ kabuk:
- ⛔ Kırılmanın üst kenarı (Ztor) 20 km'den derinse (CY14 ve CB14 sınırı): Başlat
  kapalı. Derinlik aralığı (0–35 km) bunu tek başına yakalamaz; Ztor
  büyüklüğe de bağlıdır (Mw 5 / 22 km engellenir, Mw 7 / 35 km geçer).
- ⚠ Mw > 7,9: kırılma boyutu bağıntısı veri aralığının dışında
  (docs/bilim.md 7.1); uyarılır, engellenmez.

"Sonraki olaya atla" (⏭) bildirim akışındaki her şeye atlar: çekirdek
olayları, erken uyarı, bölge Shindo bildirimleri, P'nin şehirlere varışı.
(Eskiden yalnızca çekirdek olaylarına, yani deprem anından doğrudan P'nin
100°'ye varışına, ~13 dakikaya atlıyordu.)

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

## Bölge kimliği rasterı ve bölge bildirimleri (Faz 4 adım 3, 2026-11-14)

- Kaynak: Natural Earth 1:10m Admin-1 çokgenleri (v5.1.2, 4596 bölge;
  `name_tr`, `name_en`, `iso_3166_2`). `seismira-data regions`, Vs30
  ızgarasında (30″) tarama çizgisi dolgusu: hücre merkezi çokgende ise o
  bölge (çift-tek kuralı). Hücre merkezi içermeyen adacıklar temsil
  noktasının hücresini alır. Çıktı `data/regions/admin1.sras` (iki bayt
  katmanı, 3,3 MB) + `admin1.sreg` (159 KB), depoda.
- Doğrulama (elle): Gölcük → Kocaeli (TR-41), Fatih → İstanbul (TR-34),
  Kawasaki → Kanagawa (JP-14), Los Angeles → Kaliforniya (US-CA), Marmara
  Denizi → yok.
- Toplama (`region::summarize`): bölge başına en yüksek son Shindo, ilk
  S varışı, en yüksek değere ilk ulaşma anı (kademeli yükselişle) ve konum.
  Bildirimler bundan üretilir; ad arayüz dilinde (İstanbul / Istanbul).
  "En yüksek: şehir" yalnızca şehrin Shindo'su bölgeninkine eşitse yazılır.
- Fransa ve İtalya (kullanıcı kararı, 2026-11-14): Natural Earth Admin-1
  olduğu gibi kullanılır (Fransa'da département, İtalya'da provincia);
  Bölüm 9'daki "bölge" düzeyinin yerine geçer. Türkiye'deki il boyutuna
  daha yakın, bildirimler daha yerel.
- İl değeri (kullanıcı kararı, 2026-11-14): en yüksek Shindo YERLEŞİM
  hücrelerinden (GHSL yapılaşma alanı ≥ hücrenin %1'i, ~7 000 m²; seçim).
  İlk S varışı tüm hücrelerden. Gerekçe: JMA bölge değeri istasyonlardan
  gelir, istasyonlar yerleşim yerlerindedir. Ölçüm (Marmara Mw 7.5,
  29,784 D, 40,748 K, doğrultu 84°), tüm hücre → yerleşim: Kütahya 5- → 4
  (5- olan 2 / ~18 bin hücrenin hiçbiri yerleşim değil), Bolu 5+ → 5-,
  Düzce 6- → 5+; Kocaeli / Sakarya / Yalova 6+, İstanbul 5+ değişmedi.
  Ankara 5- kaldı (2 799 yerleşim hücresinden 1'i 5-; gerçek bir yerleşim
  yerinin yumuşak zemini, istasyon davranışıyla uyumlu). Raster 8,3 MB
  (~10 milyon yerleşim hücresi, karanın %4,6'sı).

## Erken uyarı ve bölge sınırı vurgusu (Faz 4 adım 4, 2026-11-14)

- Bildirim: "Erken uyarı (varsayımsal sistem): Kocaeli, Sakarya, … ve N bölge
  daha" (t_uyarı'da; kapsamdaki bölgeler en güçlüden). Şehir kartında
  "Erken uyarı süresi: X s" ya da "Kör bölge".
- Sınırlar: etkilenen bölgelerin Natural Earth Admin-1 halkaları
  (`data/regions/admin1.sout`, 6,4 MB, ilk depremde arka planda ~120 ms'de
  açılır) GPU'ya bir kez yüklenir; her parça ekran ölçeğinde kalınlığı
  olan bir dörtgen (kare uçlu). Her karede yalnızca bölge başına renk /
  kalınlık / opaklık (uniform dilimi) güncellenir.
- Kullanıcı kararı (2026-11-15): Bölüm 9'daki "sınır yanıp söner" yerine
  İL ALANI Shindo renginde düşük kontrastla yanıp söner; sınır yüksek
  kontrastlı (koyu kontur + renkli çizgi) ve sabittir.
- Dolgu: Admin-1 halkaları (dış + delik bayrağı, `.sout` v2) etkilenen
  bölgeler için arka planda earcut ile üçgenlenir; kara, göller ve ızgaranın
  üstünde, sınır katmanlarının altında çizilir.
- İl rengi (kullanıcı kararı 2026-11-15): bildirim JMA gibi EN YÜKSEK
  değeri yazar; il dolgusu ve sınırı YAYGIN değerle boyanır: yerleşim
  hücrelerinin en az %10'unun ulaştığı en yüksek sınıf (`TYPICAL_FRACTION`,
  seçim), aynı kademeli yükselişle (sınıf k, hücrelerin %10'u k'ya
  ulaştığında görünür). Örnek (28,552 D 40,598 K, Mw 7.0, doğrultu 0°):
  Bursa'da 1 952 yerleşim hücresinin 7'si 6- (fayın güney ucundaki
  Zeytinbağı–Kurşunlu kıyısı) → bildirim "Bursa: 6-", il rengi 5-.
  Yanıp sönme kararı (nihai en yüksek ≥ 3) en yüksek değere bakar.
- Durum → görünüm:
  - Uyarı: beyaz sınır 2 pt; beyaz dolgu %4–%14 yanıp söner. Yalnızca P
    dalgasının ulaştığı bölgelerde (bölgedeki ilk P varışı; kullanıcı
    kararı 2026-11-15: kapsam 41 ile kadar çıkabiliyor, uzaktakiler
    gereksiz yanmasın). Bildirimdeki uyarı listesi değişmedi.
  - Sarsıntı: o ana kadarki en yüksek Shindo rengi; sınır 2,5 pt, dolgu
    %8–%30 yanıp söner. Nihai en yüksek < 3 ise yalnızca 1,5 pt sınır.
  - Yatışmış (bölgedeki en geç t_bitiş geçti): nihai Shindo rengi, sınır
    2 pt, dolgu %12 sabit (Bölüm 9 "çok hafif aynı renkle dolar").
  - Sınırların altında siyah %75 kontur (+2,5 pt). Güçlü olan üstte.
- Yanıp sönme: duvar saatine bağlı, 1 Hz, kosinüs. Görünüm → "Hareketi
  azalt": dolgu orta opaklıkta sabit (ayar kaydedilir).
- Hızlı oynatmada en az ~1 s görünme kuralı (2026-11-15): oynatma
  sürerken bir bölgenin uyarı ve sarsıntı görseli en az 1 duvar saniyesi
  ekranda kalır ve aşamalar sırayla gösterilir (uyarı → sarsıntı →
  yatışmış); bölge uyarıdan hiç geçmiyorsa o aşama atlanır. Yalnızca
  sunum: simülasyon durumu zamanın saf fonksiyonu olarak kalır; geri
  sarmada ve duraklatılmışken gerçek durum hemen gösterilir
  (`advance_shown`, `MIN_SHOW_S`). Örnek: 600x hızda ~40 s süren sarsıntı
  0,07 s yerine 1 s görünür. (Bölge kartı aşağıda.)
- Bölge sayısı sınırı (2026-11-15): bir karede en fazla 1 024 bölge
  vurgulanır; aşılırsa en ZAYIF olanlar atlanır. Ölçüm: Ljubljana Mw 8,0'da
  392 bölge (Natural Earth'te Slovenya 193, Birleşik Krallık 232 birim).
  Eski sınır 256 bölgeydi ve aşıldığında en güçlü bölgelerin renkli sınırı
  kesiliyordu. Uniform verisi tek seferde yazılır.
- Gözlem: Marmara Mw 7.5'te uyarı kapsamı 41 bölge (Bulgaristan ve
  Yunanistan dahil; Shindo 4 tahmini geniş). JMA ölçütüyle tutarlı ama
  dünya ölçeğinde çok beyaz çizgi var.

## Sağdan sola yazı: Faz 3 kontrolü (2026-11-15, gecikmeli)

- egui 0.37.0 hâlâ en yeni sürüm (crates.io); bidi desteği gelmedi.
- Uygulamada şu an sağdan sola metin GÖRÜNMÜYOR: 34 215 şehir adı ve il /
  eyalet adı ile 4 596 bölge adının (name_tr, name_en) tamamı Latin
  harfli (dosyalar tarandı). Sorun yalnızca "Yazı testi" penceresinde
  duruyor. Bu yüzden `unicode-bidi` geçici katmanı yazılmadı; şehir adları
  kullanıcının dilinde gösterilmeye başlanırsa (GeoNames alternatif adları)
  yeniden ele alınacak.

## Bölge kartı ve haritaya tıklama (Faz 4, 2026-11-15)

- Simülasyon açıkken haritada bir bölgeye tıklamak bölge kartını açar;
  aynı bölgeye ya da sonucu olmayan bir yere (deniz, model dışı) tıklamak
  kapatır. Kart tıklanan noktaya bağlı durur (harita kayınca birlikte
  kayar). İçerik: ad (arayüz dilinde), ISO 3166-2 kodu, en yüksek Shindo
  (şehrin değeri bölgeninkine eşitse şehir adıyla), yaygın Shindo, bölgedeki
  ilk P ve ilk S varışı, erken uyarı (ilk S'e göre süre / kör bölge / kapsam
  dışı). Değerler tahminin son hâlidir (şehir kartı gibi). Tahmini hasar
  Faz 6'da, tsunami Faz 8'de eklenecek.
- KARAR DEĞİŞİKLİĞİ: eskiden haritaya her tıklama merkez üssünü taşıyıp
  çalışan simülasyonu iptal ediyordu (yanlışlıkla tıklama simülasyonu
  siliyordu ve bölge kartıyla çakışıyordu). Artık simülasyon açıkken
  tıklama merkez üssünü DEĞİŞTİRMEZ. Yeni merkez üssü için sağ paneldeki
  "Simülasyonu kapat" düğmesi (Başlat'ın yanında, yalnızca simülasyon
  açıkken görünür) ya da paneldeki herhangi bir ayarı değiştirmek yeter.
  Simülasyon yokken tıklama eskisi gibi merkez üssünü seçer.
- Geliştirme: `SEISMIRA_REGION=boylam,enlem` kartı tıklamasız açar.
- Şehir kartına eklenenler: MMI satırı (Worden vd. 2012) ve zemin
  belirsizliği satırı ("Vs30 ×/÷ f → şiddet ±d"; docs/bilim.md 7.7).

## Zemin yenilmesi: kırmızı kareler (2026-11-23 kullanıcı kararı)

- Kullanıcı bir referans video gösterdi (Kahramanmaraş 2023 temalı bir
  erken uyarı canlandırması): riskli alanlar İÇİ BOŞ, kalın kırmızı
  çerçeveli sabit boyutlu karelerle gösteriliyor, sıvılaşma ve heyelan aynı
  kareyle, fark bildirimde ("Landslide in Hatay"). İlk sürümdeki ısı
  haritası (hücre başına 6 tonlu dolu kareler, GPU blok piramidi) KALDIRILDI;
  kullanıcı "önerdiğin gibi yap" dedi.
- Kutular: ~25 km (compute::BOX_KM); boylamda Δλ, enlemde Δφ = Δλ·cos φ₀
  (alanın orta enlemi; Web Mercator'da kare). Kutunun değeri USGS'in
  "aggregate hazard" ölçüsü: Σ kaplama × hücre alanı (km²).
- Kare kuralı: kutu USGS Ground Failure'ın olay düzeyi SARI uyarı eşiğini
  tek başına geçmeli (groundfailure `utilities.py::get_alert`: sıvılaşma
  10 km², heyelan 1 km²; 2026-11-23 okundu) VE tür başına en çok etkilenen
  alanlı 6 kutu (compute::MAX_BOXES) gösterilir. Gerekçe (ölçüm,
  2026-11-23): yalnızca sarı eşik Kahramanmaraş Mw 7,8'de 30 + 33 kare
  (halı görünümü); turuncu eşik (100 / 10 km²) Kahramanmaraş'ta 2 + 5 ama
  Kumamoto'da 0 + 0, Noto'da 1 + 0 kare veriyordu. Bu bir GÖSTERİM kararıdır;
  bölge kartındaki km² değerleri kesilmez.
- Kare, kutuda kaplaması çizim eşiğini (sıvılaşma %0,5, heyelan %0,2; USGS
  görüntüleme ayarı) geçen hücrelerde sarsıntının İLK bittiği anda (t_end)
  belirir; sonra simülasyon boyunca kalır.
- Çizim: kalın kırmızı çerçeve (altında koyu kontrast çizgisi), köşede tür
  harfi (TR S / H, EN LQ / LS; sıvılaşma sol üst, heyelan sağ üst köşede,
  aynı kutuda ikisi olunca çakışmasın). Renk tek bilgi taşıyıcı değil.
  1 Hz yumuşak yanıp sönme (opaklık %35–100, kosinüs); "Hareketi azalt"ta
  sabit. Referans videodaki sert, düzensiz yanıp sönme bilinçli olarak
  YAPILMADI (Bölüm 3 kuralı).
- Bildirim: bir bölgede (tür başına) ilk kare belirince "[S] Sıvılaşma
  riski: Hatay". Bölge rasterında olmayan kutuda (kıyı) kutuya en yakın
  şehrin ili.
- Açıklama: Shindo açıklamasının üstünde, kare varken: "□ S Sıvılaşma
  riski / □ H Heyelan riski / Kare: ~25 km'lik alan; USGS sarı uyarı eşiği".
- Görünüm menüsü: "Sıvılaşma riski", "Heyelan riski" (yalnızca
  `data/ground/ls_static.sras` varsa), ikisi de varsayılan açık, kaydedilir.
- Bölge kartı: "Sıvılaşma: ~X km² etkilenen alan (tahmini)" satırı (USGS'in
  "aggregate hazard" ölçüsü: bölgedeki Σ kaplama × hücre alanı), bölgede ilk
  kare belirdikten sonra ve katman açıkken. İzmit Mw 7,6 denemesinde Sakarya
  ~110 km² (olası 100–120).
- "Olası" aralık yalnızca sarsıntının olaylar arası belirsizliğini (ln PGV,
  ln PGA ± 1τ; hasarla aynı yöntem) yansıtır ve dardır, çünkü Zhu modelinde
  PGV'nin etkisi zayıftır (ln PGV katsayısı 0,334). Girdi verisi ve modelin
  kendi belirsizliği DAHİL DEĞİL: Kahramanmaraş doğrulamasında USGS'le fark
  ~2 kat (docs/dogrulama.md). Kullanıcıya bildirildi; gerekirse kart metni
  "yalnızca sarsıntı belirsizliği" diye netleştirilebilir.

## Tsunami (Faz 8 adım 6, 2026-11-23)

- Hesap ayrı iş parçacığında (crates/seismira-app/src/tsunami.rs): 1′
  bölgesel pencere (PTWC yarıçapı + 2°, en çok ±14°; Mw ≥ 7,9'da ±8°) ile
  yakın alan tırmanması ve varış; Mw ≥ 7,9'da ek olarak küresel 4′ varış
  (Tōhoku: bölgesel 0,5 s, küresel +6 s). Simülasyon sonu = max(S cephesi
  sonu, son tsunami varışı + 10 dk); zaman çizgisi buna uzar.
- Uyarı anı: depremden 180 s sonra (JMA "約3分" hedefi, 気象庁
  津波警報・注意報 sayfası 2026-11-23 okundu; PTWC ilk ürünü 3–10 dk).
- Kategoriler (aynı JMA sayfası): 大津波警報 > 3 m (mor 200,0,255),
  津波警報 1–3 m (kırmızı 255,40,0), 津波注意報 0,2–1 m (sarı 250,245,0;
  JMA HPColorGuide renkleri). Yakın alanda yükseklik = Riquelme tırmanma
  tahmini (JMA'nın "kıyıda tsunami yüksekliği" değil, tırmanma; kaba
  tahmin olarak etiketli). Uzak alanda (PTWC tehdit bölgesi, yükseklik
  yok) ve doğrultu atımlıda: sarı.
- Haritada kıyı ŞERİDİ (kullanıcı kararı 2026-11-23: noktalar yerine
  yekpare bant): kıyı noktaları (~10 km arayla) her biri en yakın iki
  komşusuna (≤ 22 km) kalın çizgiyle bağlanır (`tsunami::coast_links`,
  sonuç gelince bir kez); koyu kenarlı, kategori renginde, iki farklı
  kategorili nokta arasında renk parçanın ortasında değişir. Noktalar
  kıyı boyunca sıralı olmadığından dar körfezlerde karşı kıyıya bağlanma
  görülebilir; 22 km'den uzun boşluk açık kalır. 1 Hz yumuşak yanıp söner
  (ground_style::pulse), "Hareketi azalt"ta sabit. Fareyle şeride
  gelince en yakın noktanın ipucu: yer (şehir, il), kategori, tahmini
  yükseklik ya da "yükseklik tahmin edilmedi", ilk dalga anı.
- Açıklama sol altta: üç kategori + "yükseklikler kaba tahmindir".
- Bildirimler: 180 s'de kategori başına il listesi ("Büyük tsunami
  uyarısı: Iwate, Miyagi, …"), referans videodaki gibi en yüksek 6 yerin
  listesi ("Tahmini en yüksek tırmanma: Ishinomaki (35 m), …"); her ile ilk
  dalga ulaşınca "Tsunami ulaştı: Miyagi (~35 m)". Mw 6,5–7,0 / derin /
  karada: tek bilgi bildirimi (PTWC ölçütü). ⏭ tsunami olaylarına da atlar.
- Bölge kartı: "Tsunami: <kategori>, tahmini ~X m (ya da yükseklik tahmin
  edilmedi), ilk dalga T+…" (uyarı anından sonra). Kıyı noktası denizde
  olduğu için bölgesi çevresindeki (0, 2, 5, 10 km; 8 yön) ilk kimlikli kara
  hücresinden (crates/seismira-app/src/tsunami.rs `by_region`).
- Görünüm menüsü → "Tsunami uyarıları" (veri varsa; varsayılan açık,
  kaydedilir): kapalıyken kıyı noktaları, açıklama, bildirimler ve bölge
  kartı satırı gizlenir (hesap yine yapılır, zaman çizgisi yine uzar).

## Kendi verin ve dışa aktarma (2026-11-26, README kritik listesi)

- Menü "Veri": "Kendi verini yükle…" ve "Sonuçları dışa aktar…" pencereleri
  (`app/user_ui.rs`, `userdata.rs`, `export.rs`).
- Dosya seçme: yol kutusu (her platform) + pencereye sürükle-bırak. winit
  0.30.13'te dosya bırakma yalnızca X11, Windows, macOS'ta var; Wayland'de
  YOK (kaynakta denetlendi). 2026-11-26 kullanıcı onayı: "Dosya seç…"
  düğmesi sistemin dosya penceresini açar (`rfd` 0.18.0, MIT; Linux'ta
  xdg-desktop-portal, GTK bağımlılığı yok; pencere ana iş parçacığında
  açılır, sonuç `pollster` ile yardımcı iş parçacığında beklenir). Linux
  paketinde portal arka ucu (ya da Zenity) gerekir; GNOME/KDE'de hazır.
- Türler (uzantıdan): .geojson/.json fay izi (en uzun çizgi; > 32 nokta
  Visvalingam ile 32'ye), .tif/.tiff Vs30 (tek bant, coğrafi koordinat;
  100–3000 m/s dışı veri yok; en çok 10⁸ hücre; en yakın hücre), .csv/.txt
  envanter (başlıklı; boylam/enlem/bina sütun adları TR/EN; ; ayraçta
  ondalık virgül). Okuma arka planda; hata çeviri anahtarıyla gösterilir.
- Vs30 kapsadığı KARA hücrelerinde (deniz NaN kalır) ve şehirlerde
  uygulamanın Vs30'u yerine geçer (şehir kartında "senin yüklediğin
  harita"); bina hasarı da bunu kullanır. Sıvılaşma / heyelan statik
  katmanları önceden hesaplı olduğundan DEĞİŞMEZ (bilinen sınır).
- Envanter: her nokta için şiddet ve beklenen ağır hasar + göçme =
  bina sayısı × noktanın ilindeki ESRM20 karışımının oranı (şehirlerle
  aynı yöntem; yalnızca Türkiye ve Avrupa). Haritada mavi kareler (≤ 5 000
  nokta çizilir), Vs30 kapsamı mavi kesikli çerçeve.
- Dışa aktarma: `~/Seismira/seismira-<UTC tarih-saat>/`: üç GeoTIFF (JMA
  aletsel şiddet, PGV cm/s, PGA g; f32, nodata −9999, EPSG:4326; elle
  yazılır, rasterio/GDAL 3 ile doğrulandı), şehir ve bölge CSV'si, fay ve
  zemin yenilmesi GeoJSON'u, envanter CSV'si, BENIOKU_README.txt (eğitim
  uyarısı, birimler). Yazma arka planda; "Klasörü aç" (xdg-open / explorer
  / open).

## Tepki spektrumu ve TBDY-2018 (2026-11-26, README "Önemli")

- Veri → "Tepki spektrumu…"; simülasyon açıkken haritada SAĞ TIK noktayı
  seçer (sol tık bölge kartı olarak kalır). Haritada turuncu elmas.
- Spektrum: `Scenario::spectrum` (çekirdek), 0,02–6 s'de 46 logaritmik
  periyot + PGA (T = 0), depremin modeli (kabukta dört NGA-West2 ortalaması,
  dalma-batmada Parker vd. 2020), RotD50, %5 sönüm; medyan ve ±1σ (toplam
  σ). Zemin: kullanıcı Vs30 → USGS → referans. Model sınırı dışı: yok.
- TBDY-2018 (`seismira_core::tbdy`, Resmî Gazete metninden): kullanıcı S_S
  ve S_1 girer (AFAD tdth.afad.gov.tr; harita uygulamada YOK), zemin sınıfı
  Vs30'dan (Tablo 16.1; sınır değerde üst sınıf, VARSAYIM) ya da elle;
  F_S, F_1 Tablo 2.1–2.2 doğrusal ara değer; S_ae(T) Denk. 2.2–2.3, T_L 6 s.
  ZF: çizilmez. Uyarı metni: senaryo tahmini (tek olay) ile olasılıksal
  tasarım spektrumu aynı şey değildir; harita katsayıları geometrik
  ortalama, tahmin RotD50.
- Grafik kütüphanesiz (egui çizici): doğrusal eksen, 2 / 4 / 6 s seçimi,
  imleçte değer. CSV: ~/Seismira/spektrum-<UTC>.csv.


## Dizüstü / dokunmatik yüzey uyumu (2026-11-26, kullanıcı isteği)

- İki parmak kaydırma (tıklamadan) = haritayı her yöne kaydır; fare
  tekerleği = yakınlaştır (önceki davranış); iki parmak sıkıştırma
  (pinch) = yakınlaştır (macOS, Windows hassas dokunmatik yüzey: Ctrl +
  kaydırma olarak gelir); Ctrl + iki parmak / tekerlek = yakınlaştır
  (her yerde). Haritanın sağ kenarında + / − düğmeleri; klavyede + / −.
- Ayrım (`map/mod.rs`): egui-winit kaynak bilgisini yalnızca macOS'ta
  veriyor; Wayland'de tekerlek ayrık "satır" (LineDelta), dokunmatik
  yüzey sürekli "nokta" (PixelDelta) birimiyle gelir (winit 0.30.13
  wayland/seat/pointer kaynağı); Windows'ta dokunmatik yüzey kesirli
  satır, tekerlek tam sayı satır.
- BİLİNEN SINIR: winit 0.30.13'te sıkıştırma hareketi (PinchGesture)
  yalnızca macOS / iOS'ta; Linux'ta (Wayland ve X11) iletilmiyor → Linux
  dizüstünde yakınlaştırma Ctrl + iki parmak ya da + / − ile. eframe
  0.37 winit 0.30'a bağlı; ileride winit yükseltilince eklenebilir.
