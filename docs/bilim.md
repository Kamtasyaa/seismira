# Bilimsel model

> CLAUDE.md v3’ten ayrıldı (Faz 0, 2026-11-14). Bölüm numaraları atıflar
> bozulmasın diye korunmuştur. "(doğrula)" işaretleri hâlâ geçerlidir.

## 7. Bilimsel model

GENEL KURAL: hiçbir katsayıyı ezbere yazma. Her formül yayımlanmış
kaynağından alınır, kod yorumunda künyesi yazılır, makaledeki örnek
değerlerle birim testi yazılır.

Referans uygulamalar (yalnızca sonuç karşılaştırmak için): USGS
nshmp-lib, USGS groundfailure, USGS ShakeMap (kamu malı, doğrula);
OpenQuake hazardlib (AGPL: KOD KOPYALANMAZ, yalnızca çıktı
karşılaştırılır). Katsayılar makalelerin elektronik eklerinden alınır.

### 7.1 Kaynak geometrisi

- Moment: M0 = 10^(1.5·Mw + 9.1) N·m (Hanks & Kanamori 1979).
- Kırılma boyutları: kabuk için Wells & Coppersmith (1994) veya
  Leonard (2010); dalma-batma için Strasser vd. (2010). Alternatif
  tek kaynak: Thingbaijam vd. (2017).
- Tek düzlem dikdörtgen: strike, dip, rake, odağın düzlemdeki konumu.
- Kısıtlar: üst kenar yüzeyin üstüne çıkamaz (Ztor ≥ 0). Kabukta
  genişlik sismojenik kalınlıkla sınırlı (~15–25 km, doğrula);
  genişlik doyunca alan korunacak şekilde uzunluk artar.
- Varsayılanlar (doğrula): doğrultu atımlı dip 90°, rake 0° veya 180°;
  normal dip ~55°, rake −90°; ters dip ~40°, rake 90°; arayüzde dip
  ve strike Slab2'den.
- Ortalama kayma D = M0 / (μ·A), μ ≈ 30 GPa (doğrula). Tsunami için.
- Düzlem ~5–10 km'lik alt faylara bölünür. Alt fayın kırılma zamanı =
  odağa uzaklık / kırılma hızı (≈ 0.8·Vs ≈ 2.8 km/s, doğrula).
  Bu yalnızca zamanlama hesabı içindir, ekranda gösterilmez.

### 7.2 Seyahat süreleri

- TEK model: ak135 (Kennett, Engdahl & Buland 1995). Sabit hızlı
  yakın alan modeli KULLANILMAZ (iki model arasında halka sıçrar).
- Tablo çevrimdışı üretilir (TauP / ObsPy): episantr uzaklığı
  0–100°, derinlik 0–700 km, ilk P ve ilk S varışı. Uygulamada
  çift doğrusal ara değer.
- Halka yarıçapı = verilen süre için tablodan ters arama. Derin
  depremde halka yüzeyde gecikmeyle belirir.
- ~100° ötesinde halka çizilmez (P gölge bölgesi). Tahmini şiddetin
  hissedilme eşiğinin altına düştüğü mesafeden sonra halkalar soluk
  çizilir.

### 7.3 Zamanlama ve erken uyarı

Her nokta (şehir, ızgara hücresi) için:
- t_P: odaktan ilk P varışı.
- t_S: odaktan ilk S varışı. (Kırılma S'den yavaş olduğu için ilk S
  her zaman odaktan gelir; halkalarla tutarlıdır.)
- t_tepe: en yakın alt fayın kırılma zamanı + o alt faydan S süresi.
  Shindo değeri t_S'de belirir, t_tepe'de nihai değerine ulaşır.
  (M9'da fayın uzak ucundaki şehir için fark dakikaları bulur.)
- t_bitiş: t_tepe + sarsıntı süresi (kabuk için Afshari & Stewart
  2016; dalma-batma için uygun model bul, doğrula).

Erken uyarı (VARSAYIMSAL sistem; arayüzde böyle etiketlenir, çünkü
çoğu ülkede kamuya açık bir sistem yok). Uygulama: `seismira-core/src/warning.rs`
(Faz 4, 2026-11-14).
- Uyarı ölçütü (JMA, DOĞRULANDI 2026-11-14): "最大震度が５弱以上 … と予想
  された場合に、震度４以上 … が予想される地域を対象に" (jma.go.jp
  faq25): tahmini en yüksek Shindo 5- ve üstüyse, Shindo 4 ve üstü
  beklenen bölgelere uyarı. Ayrıca "地震波が２点以上の地震観測点で観測され"
  (JMA EEW ayrıntıları): en az iki istasyonda gözlem.
- t_uyarı = P'nin merkez üssünden D uzaklıktaki sanal (ikinci) istasyona
  varışı + işlem gecikmesi. D = 20 km: K-NET aralığı ~20 km (NIED; ilk
  kurulumda ~25 km); bu aralıkta ikinci en yakın istasyon tipik olarak bu
  mesafede ya da daha yakındadır (önceki taslakta 25 km idi). Gecikme =
  5 s: JMA gecikmeyi sayı olarak vermiyor ("可能な限り素早く"); VARSAYIM,
  yayımlanmış performans çalışmalarından doğrulanacak. İleri seviyede:
  yoğun ağ / seyrek ağ / sistem yok (henüz yok).
- Bölge değeri: bölgenin yerleşim hücrelerindeki en yüksek tahmini Shindo
  (docs/arayuz.md "İl değeri").
- Her şehir için iki değer gösterilir: "S − P" ve "erken uyarı
  süresi" (t_S − t_uyarı). Sıfır veya negatifse "kör bölge". (Uyarı
  süresi, uyarı yayımlandıysa ve şehirde Shindo ≥ 4 ise gösterilir.)

Sarsıntı süresi (Faz 4, 2026-11-14): `seismira-core/src/duration.rs`,
Afshari & Stewart (2016) D5-95, havza terimi yok. pygmm 0.9.1 ile 810
karşılaştırmada ln farkı < 1e-6 (`data/test/as16_ref.csv`). t_bitiş =
t_tepe + D5-95 (t_tepe şimdilik kademeli yükselişin tepe anı; Faz 5'te alt
fay zamanlaması). Girdiler model aralığına (M 3–7,9, Rrup 0–200 km, Vs30
200–1000) çekilir; süre yalnızca bölge sınırının "yatışmış" durumuna
geçişinde kullanılır (doğrula: makalenin veri aralığı).

### 7.4 Yer hareketi (GMPE)

- Çıktılar: PGA, PGV, SA(0.3 s), SA(0.6 s), SA(1.0 s). Modelde
  olmayan periyot komşu periyotlardan log-log ara değerle bulunur.
- Sığ kabuk: CY14 (Chiou & Youngs 2014). Rrup, Rjb, Rx, Ztor ve dip
  kullanır; derinlik sonucu etkiler. BSSA14 TEK BAŞINA KULLANILMAZ
  (derinlik terimi yok, derinlik ayarı etkisiz kalır). İleride:
  ASK14, BSSA14, CB14, CY14 eşit ağırlıklı ortalama (USGS yaklaşımı).
- Dalma-batma arayüzü ve levha içi: Parker vd. (2020); alternatif
  Kuehn vd. (2020). Bölgesel katsayılar konuma göre seçilir.
  (BC Hydro modeli PGV vermediği için kullanılmaz, doğrula.)
- Japonya seçeneği: Morikawa & Fujiwara (2013); aletsel şiddeti
  doğrudan da veriyor olabilir (doğrula).
- Kararlı kıta içi bölgeler: ilk sürümde özel model yok; kabuk
  modeli kullanılır ve "bu bölge için model uygun değil" uyarısı
  çıkar. İleride NGA-East.
- Modelin mesafe sınırının ötesi "model dışı" işaretlenir,
  gösterilmez.
- Bileşen tanımı: modeller RotD50 verir; şiddet dönüşümünün
  beklediği tanım farklıysa dönüşüm uygulanır (doğrula).

### 7.5 Zemin etkisi

- Vs30: USGS küresel mozaik varsayılan; Avrupa ve Türkiye için
  ESRM20 site modeli; Japonya için J-SHIS.
- Havza derinliği (Z1.0): modelin kendi Vs30 ilişkisinden varsayılan.

#### Saha koşulu: uygulama (Faz 4, 2026-11-14)

- Kaynak: USGS Küresel Vs30 Mozaiği 2026 (docs/veri.md), 30″, medyan ve
  log standart sapma. `seismira-core/src/site.rs`.
- Saklama: u8, medyan 80–2500 m/s logaritmik (en büyük kuantalama
  hatası %0,7), SD 0–1,25 doğrusal (adım 0,005). Ham değer aralığı
  83,6–2197 m/s ve SD 0,10–0,525; hiçbir değer kırpılmadı.
- Şehirler: veri hazırlamada 5×5 hücrelik pencerede yapılaşma (GHSL)
  ağırlıklı medyan (ESRM20 exposure2site yaklaşımı, Weatherill vd. 2023);
  pencerede yapılaşma yoksa aşağıdaki nokta kuralı.
- Hücre su / buz / buzul ise (kıyı şehirleri) ≤ 3 hücre (≈ 2,8 km)
  içinde en yakın kara hücresi; yoksa referans 400 m/s. Kaynak şehir
  kartında yazılır (harita / en yakın kara / referans).
- CY14 aralığı 180–1500 m/s. Dışındaki harita değeri uca çekilir ve
  kartta "model sınırına çekildi" diye AÇIKÇA yazılır (Bölüm 6: sessiz
  ekstrapolasyon yok). Neden kırpma, "model dışı" değil: yumuşak deltalar
  (< 180) en çok sarsılan yerler; onları boş bırakmak yanıltıcı olur.
  CY14'ün doğrusal olmayan zemin terimi düşük Vs30'da doygunluğa
  yaklaştığı için kırpmanın etkisi sınırlıdır (doğrula: 150 m/s'de
  kırpılmış ve kırpılmamış tahmin farkı).
- Vs30 belirsizliği (SD katmanı) şimdilik okunur ama σ'ya eklenmez;
  Faz 4 belirsizlik adımında ele alınacak.
- Performans: Vs30 yalnızca yer hareketi hesaplanacak sahalarda (Rrup ≤
  300 km) okunur; dosya belleğe eşlenir, döşemeler (2°×2°) gerektikçe
  açılır ve önbelleğe alınır.

### 7.6 Şiddet dönüşümü

- JMA Shindo: PGV'den aletsel şiddet (Midorikawa vd. 1999 veya
  Fujimoto & Midorikawa 2005; geçerlilik aralığına dikkat, doğrula).
  Sınıflar: 0 (<0.5), 1 (0.5–1.5), 2 (1.5–2.5), 3 (2.5–3.5),
  4 (3.5–4.5), 5- (4.5–5.0), 5+ (5.0–5.5), 6- (5.5–6.0),
  6+ (6.0–6.5), 7 (≥6.5).
- MMI: Worden vd. (2012).

### 7.7 Belirsizlik

- Monte Carlo ile çoklu senaryo ÜRETİLMEZ.
- GMPE standart sapması analitik kullanılır (log-normal). "En olası
  Shindo" yanında "olası aralık" gösterilir.

### 7.8 Bina hasarı

- Bina stoku (exposure) → taksonomi → kırılganlık eğrisi.
- Her eğrinin istediği şiddet ölçüsü veri dosyasından okunur
  (tipik: PGA, SA(0.3), SA(0.6), SA(1.0); doğrula).
- Varsayılan kural: kırılganlık eğrileri ESRM20 ve GEM'den gelir.
  Ulusal veriler (İBB, e-Stat, Hazus, INEGI) yalnızca stok dağılımını
  iyileştirir. Ulusal bir eğri kullanılacaksa kaynağı ve lisansı
  belgelenir.
- ESRM20 taksonomisi tasarım seviyesini zaten içerir (yönetmeliksiz /
  düşük / orta / yüksek; doğrula). İBB'nin üç dönemi (1980 öncesi,
  1980–2000, 2000 sonrası) yalnızca İstanbul'da mahalle düzeyinde
  dağılımı inceltmek için kullanılır; eşleme tablosu docs/veri.md'ye
  yazılır.
- Yönetmelik eşikleri (eşleme için): Türkiye 1975, 1998, 2007, 2018;
  Japonya 1981 ve 2000; ABD Hazus sınıfları; Meksika 1985 sonrası.
- Çözünürlük: açık veriler çoğunlukla il / eyalet düzeyinde. Asıl
  sonuç BÖLGE düzeyinde verilir. Şehir düzeyi, "şehir bulunduğu
  bölgenin yapı dağılımını taşır" varsayımıyla indirgenir ve böyle
  etiketlenir.
- Beklenen hasar oranları, kırılganlık eğrisinin log-normal şiddet
  dağılımı üzerinden integraliyle bulunur. Aralık, olay-arası terimin
  ±1 standart sapma kaydırılmasıyla verilir.
- Hasar durumları: hafif / orta / ağır / göçme. Tüm sonuçlar
  "tahmini" etiketlidir.

### 7.9 Sıvılaşma ve heyelan

- Sıvılaşma: Zhu vd. (2017). Girdiler: PGV, Vs30, yağış, kıyıya
  uzaklık, nehre uzaklık, yeraltı su seviyesi (doğrula).
- Heyelan: Nowicki Jessee vd. (2018). Girdiler: PGV, eğim, litoloji,
  arazi örtüsü, CTI (doğrula).
- Modellerin kalibre edildiği çözünürlük ve veri setleri kaynaktan
  doğrulanır; farklı veri seti kullanılırsa belgelenir.
- Çıktı: hücre başına olasılık; haritada kırmızı tonlu kareler
  (yoğunluk = olasılık).

### 7.10 Tsunami

- Renkler: SARI az tehlikeli, KIRMIZI tehlikeli, MOR çok tehlikeli.
  JMA kategorileriyle uyumlu eşik önerisi (doğrula): sarı 0.2–1 m,
  kırmızı 1–3 m, mor 3 m üstü. 0.2 m altı: uyarı yok.
- Aşama 1, tetikleme: deniz altında veya kıyıya yakın, sığ
  (~100 km'den az), büyük (~Mw 6.5–7 üstü) depremler. Eşikler
  yayımlanmış uyarı ölçütlerinden doğrulanır.
- Aşama 2, kaynak ve varış: Okada (1985) ile sonlu faydan deniz
  tabanı düşey yer değiştirmesi (fay tipi etkisi buradan gelir;
  doğrultu atımlıda küçük çıkar). GEBCO üzerinde √(g·h) hızıyla,
  KAYNAK ALANINDAN başlayan fast marching ile varış süresi haritası.
  Kıyı yüksekliği: Abe'nin ampirik bağıntısı (Abe 1995, doğrula)
  veya Green yasası; "kaba tahmin" etiketli.
- İlk uyarı zamanı: depremden ~3 dakika sonra (JMA hedefi, doğrula;
  ayarlanabilir). Bildirim ve kıyı vurgusu o anda başlar.
- Yalnızca okyanusa bağlı kıyılar; göller ve iç sular maskelenir.
- Aşama 3 (isteğe bağlı): sığ su denklemleri, wgpu compute shader.


## Uygulama notları

### 7.2 Seyahat süreleri (Faz 2)

- Tablo: `scripts/gen_traveltime.py`, ObsPy 1.5.1 TauP, model "ak135",
  faz listeleri "ttp" ve "tts"; her noktada en erken varış. Çıktı
  `data/traveltime/ak135.ttt` (depoda; ObsPy kurulumu ağır olduğu için
  tablo hazır tutulur).
- Izgara: mesafe 0–2° (0.02° adım), 2–20° (0.1°), 20–100° (0.25°);
  derinlik 0–40 km (1 km), 40–100 km (5 km), 100–700 km (10 km).
- Doğrulama: ızgara dışı 4000 rastgele noktada (1000'i kaynağa yakın
  0–3°, 0–60 km) TauP değeri `ak135_check.csv`'ye yazılır; Rust testi
  çift doğrusal ara değerin en büyük hatasını ölçer (ölçüt < 1 s,
  Bölüm 14). Sonuç aşağıda.
- Halka yarıçapı: sabit derinlik için t(Δ) eğrisinde ters arama
  (doğrusal). İlk varış mesafeyle azalmaz; ara değer gürültüsüne karşı
  eğri kümülatif maksimumla düzlenir.
- Yüzey geometrisi: halka noktaları WGS84 üzerinde jeodezik (Karney 2013,
  geographiclib-rs). Episantr açısı ↔ km dönüşümü 6371 km yarıçapla
  (ObsPy `kilometers2degrees` uzlaşımı; ak135 yarıçapı da 6371 km).
  Not: Bu uzlaşım jeosentrik enlem düzeltmesi yapmaz; etkisi en fazla
  ~0.2° enlem (~birkaç saniye varış). Gerekirse ileride düzeltilir.

### 7.1 Kaynak geometrisi (Faz 3)

- Alan: Wells & Coppersmith (1994) Tablo 2A, tüm fay tipleri,
  M = 4.07 + 0.98·log10(A) bağıntısının tersi (USGS nshmp-lib de alanı bu
  tersten hesaplar; `RuptureScaling.lengthCa08`, kamu malı). Birincil
  makaleden DOĞRULANDI (2026-11-15, s. 990, Tablo 2A, "All" satırı): 148
  deprem, a = 4.07 ± 0.06, b = 0.98 ± 0.03, s = 0.24, r = 0.95, büyüklük
  aralığı 4.8–7.9, alan aralığı 2.2–5 184 km².
- Veri aralığının üstü (Mw > 7.9) ekstrapolasyondur ve arayüzde AÇIKÇA
  uyarılır (engellenmez; `Issue::RuptureSizeExtrapolated`). 20 km genişlikle
  doğrultu atımlıda: Mw 7.6 → 200 km, 7.8 → 320 km, 8.0 → 512 km, 8.5 →
  1 657 km. İzmit'te uzun (gözlenen ~145 km), Kahramanmaraş'ta uyumlu
  (docs/dogrulama.md, B). Mw 8 üstü için genişlik doygunluğunu gözeten bir
  bağıntı (ör. Leonard 2010) AÇIK KONU; kaynaktan doğrulanmadan eklenmez.
- Genişlik = min(√A, sismojenik taban / sin δ); doyunca L = A / W.
  Sismojenik taban: 20 km (VARSAYIM; CLAUDE.md "15–25 km"). Hiposantr
  tabandan derinse taban hiposantr + 1 km'ye iner.
- Hiposantrın düzlemdeki konumu "İleri seviye" ayarı (varsayılan doğrultu
  ve eğim boyunca ortada). Ztor ≥ 0 ve taban kısıtı konumu kaydırabilir.
- Thingbaijam vd. (2017): erişilebilen kopya "tüm haklar saklı, metin ve
  veri madenciliği / yapay zekâ yasak" notlu olduğu için kullanılmadı.
- Dalma-batma ve levha içi ölçekleme (Strasser vd. 2010) Faz 5'te.
- Mesafe ölçüleri NGA-West2 tanımlarıyla; saha, merkez üssü çevresinde
  azimutal eşit uzaklık düzlemine taşınır (300 km içinde hatası ihmal
  edilebilir). Testler: dikey/eğimli fay, tavan/taban bloğu, analitik
  değerler (source.rs).

### 7.4 CY14 (Faz 3)

- Künye: Chiou & Youngs (2014), Earthquake Spectra 30(3), 1117-1153.
- Katsayılar: USGS nshmp-lib `CY14.csv` (CC0), 2026-11-14 alındı.
  Fonksiyonel biçim nshmp-lib `ChiouYoungs_2014.java` temel modeline göre
  (USGS havza ölçeklemesi, CyberShake, PRVI hariç).
- Seçimler: RotD50; Kaliforniya/küresel temel model (bölgesel düzeltme
  yok); yönelim terimi yok; havza terimi 0 (ΔZ1 = 0); Vs30 "tahmini".
  Fay tipi (ters/normal terimleri) kullanıcının seçtiği tipten alınır.
  Kayma açısı elle değiştirilirse tip CY14'ün bayrak tanımına göre seçilir
  (2026-11-15): ters 30° ≤ λ ≤ 150°, normal −120° ≤ λ ≤ −60°, kalanı
  doğrultu atımlı (`FaultType::from_rake`; OpenQuake hazardlib ve
  wltcwpf/GMPE uygulamalarıyla teyit edildi, makale metni görülmedi).
  Öncesinde kayma açısı kutusu hesaba hiç girmiyordu. (Eski not: nshmp-lib
  45–135° / −135–−45° kullanır; doğrulanmadı.)
- Geçerlilik: Mw 3.5–8.5 doğrultu atımlı, 3.5–8.0 ters/normal; Rrup ≤ 300
  km; Ztor 0–20 km; Vs30 180–1500 m/s. pygmm de aynı sınırlarla uyarıyor.
  Bölüm 6'daki kabuk sınırları (4.0–8.5 / 8.0) bununla uyumlu.
- Test: pygmm 0.9.1 ile 1944 karşılaştırma, fark 0 (docs/dogrulama.md).
- Faz 3–4 arası zemin sabit 400 m/s idi; Faz 4'ten itibaren Vs30
  haritası (Bölüm 7.5 "Saha koşulu: uygulama").

### 7.6 Şiddet dönüşümü (Faz 3)

- Midorikawa, Fujimoto & Muramatsu (1999), Journal of Social Safety
  Science 1, 51-56, Tablo 2 (J-STAGE'den tam metin okundu):
  - I = 0~7: I = 2.54 + 1.82·log10(PGV), σ = 0.19 ← KULLANILAN
  - I = 4~7: I = 2.68 + 1.72·log10(PGV), σ = 0.21 (JMA erken uyarısı;
    Iwakiri vd. 2011, EPS 63, 57-69, Eş. 6 ile teyit)
  - PGV: iki yatay bileşenin vektörel bileşkesinin tepesi (水平2成分を
    合成した最大値) = RotD100.
- Iwakiri vd. (2011): 4~7 bağıntısı ~2.5'in altında uygun olmayabilir;
  bu yüzden tüm aralığı kapsayan 0~7 uydurması seçildi.
- RotD100/RotD50 (PGV) = 1.23: VARSAYIM, DOĞRULANMADI. Shahi & Baker
  (2014) Tablo 1 yalnızca spektral ivme verir (0.5–1 s'de 1.23–1.24).
  Boore & Kishida (2017, BSSA) PGV oranını veriyor ancak erişilemedi
  (yazar sitesinin sertifikası süresi dolmuş). Etkisi ≈ +0.16 şiddet.
  Kullanıcıdan makaleyi tarayıcıda açması istenebilir.
- Shindo eşikleri ve yuvarlama: JMA "計測震度の算出方法" (2026-11-14):
  alt sınır dahil / üst sınır hariç; üçüncü ondalık yuvarlanır, ikincisi
  atılır.
- Shindo renkleri: JMA HPColorGuide (Temmuz 2020) Tablo 2-2:
  7 (180,0,104), 6+ (165,0,33), 6- (255,40,0), 5+ (255,153,0),
  5- (255,230,0), 4 (250,230,150), 3 (0,65,255), 2 (0,170,255),
  1 (242,242,255).
- Faz 8 için not: aynı kılavuz Tablo 1-2 tsunami renkleri: 大津波警報
  (200,0,255) mor, 津波警報 (255,40,0) kırmızı, 津波注意報 (250,245,0)
  sarı.
- Belirsizlik: σ_I = √((1.82·σ_lnPGV / ln10)² + 0.19²); "olası aralık"
  ±1σ.

## Açık doğrulama notları

- (Faz 0 notu) 7.1'deki moment bağıntısı: M0 = 10^(1.5·Mw + 9.1) N·m,
  IASPEI standart biçimidir. Hanks & Kanamori (1979) özgün bağıntısı
  Mw = (2/3)·log10(M0 [dyn·cm]) − 10.7 olup N·m'de 10^(1.5·Mw + 9.05)
  verir (fark ~%12). Faz 3'te birincil kaynaktan doğrulanıp hangisinin
  kullanılacağı ve künyesi buraya yazılacak.

## İnceleme notları (2026-11-15)

- İlk S varışı ve güçlü sarsıntı: ak135'in ilk S'i ~150 km'den sonra kabuk
  altından gelen dalgadır. 10 km derinlikte ilk S ile 3,5 km/s doğrudan S
  arasındaki fark: 150 km'de 0,3 s, 200 km'de 3,3 s, 300 km'de 9,6 s
  (ölçüm, tablo). Shindo kutusu ve erken uyarı süresi ilk S'e göre
  hesaplandığı için uzakta kutu bu kadar erken belirir, uyarı süresi bu
  kadar kısa görünür. Tek model kararının (7.2) sonucu; Faz 5'te t_tepe
  ile birlikte yeniden ele alınacak.
- Erken uyarı kapsamı: Marmara Mw 7.5'te 42 bölgenin 41'i kapsamda. Sebep
  "en yüksek hücre" istatistiği DEĞİL: yaygın değerle (yerleşim
  hücrelerinin %10'u) de 41 / 41 çıkıyor (Ljubljana Mw 7.5: 301 → 298).
  Model, 300 km sınırına kadar Shindo 4 öngörüyor; Kahramanmaraş
  istasyonları (200–300 km'de gözlem tahminden yüksek) bununla çelişmiyor.
  Kapsam ölçütü değiştirilmedi.
- CY14 Ztor sınırı (20 km) girdi denetimine bağlandı: sığ kabukta üst
  kenar 20 km'den derinse (küçük ve derin deprem, ör. Mw 5 / 22 km)
  Başlat engellenir (`Issue::ZtorOutOfRange`). Öncesinde denetim geçiyor,
  hiçbir sahada şiddet çıkmıyordu.
- RotD100 / RotD50 (PGV) DOĞRULANDI ve sabit 1,23 kaldırıldı (2026-11-15):
  Boore & Kishida (2017) katsayıları USGS ShakeMap'in (CC0) veri
  dosyalarından alındı (makalenin kendisine yine erişilemedi). Oran
  büyüklük ve mesafeye bağlı: Rrup 1–300 km'de 1,19–1,29, fay üzerinde
  1,37; eski sabite göre fark −0,03…+0,04 şiddet birimi
  (`seismira-core/src/component.rs`). 7.6'daki "VARSAYIM" notu geçersiz.
- MMI eklendi (2026-11-15): Worden vd. (2012), PGV'den, mesafe / büyüklük
  terimleriyle; katsayılar ShakeMap `wgrw12.py` (CC0), makale görülmedi.
  Girdi iki yatay bileşenin büyüğü (Boore & Kishida 2017 ile RotD50'den).
  σ = 0,63. Şehir kartında Shindo'nun altında gösterilir
  (`seismira-core/src/mmi.rs`). Henüz yok: haritada MMI ölçeği seçimi.
- Zemin (Vs30) belirsizliği, 7.7 (2026-11-15): haritanın log standart
  sapması "olası aralık"a EKLENMEZ. Gerekçe: CY14'ün σ'sı Vs30'u
  ölçülmemiş ("inferred") sahalar için zaten büyütülmüştür (Eş. 13'teki
  σ3 terimi; kodda bu dal kullanılıyor); harita SD'sini üstüne eklemek
  aynı belirsizliği iki kez saymak olur. Bunun yerine etkisi ayrı
  hesaplanıp şehir kartında gösterilir: Vs30·e^(±SD) için aletsel şiddet
  farkının yarısı (`SiteResult::jma_vs30_effect`; yalnızca SD'si olan
  sahalarda, yani şehirlerde; ızgara hücrelerinde hesaplanmaz).
- Moment bağıntısı (Faz 0 notunun durumu, 2026-11-15): Hanks & Kanamori
  (1979) özetinden ve NEIC büyüklük formülleri tablosundan teyit edildi:
  M = (2/3)·log10(M0) − 10.7, M0 dyn·cm. N·m cinsinden
  M0 = 10^(1.5·Mw + 9.05). 7.1'deki "9.1" sabiti IASPEI biçimidir ve
  birincil kaynaktan DOĞRULANMADI. Kodda moment henüz kullanılmıyor
  (yalnızca AS16 süre modeli kendi 16.05 dyn·cm sabitiyle, ki H&K ile
  aynıdır). Karar: Faz 8'de (Okada, ortalama kayma) H&K tanımı (9.05)
  kullanılacak; 7.1'deki satır o zaman düzeltilecek.
- Erken uyarı gecikmesi (5 s) hâlâ VARSAYIM (2026-11-15): JMA sisteminin
  gözden geçirme makalesi (Kodera vd. 2021, Frontiers in Earth Science,
  doi:10.3389/feart.2021.726045) ilk rapor zamanını yalnızca deniz
  depremleri için ve oluş anına göre veriyor (S-net bölgesinde %50 için
  ~22 s); istasyonda algılamadan uyarıya geçen süre için yayımlanmış bir
  sayı bulunamadı.
