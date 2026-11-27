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

- Moment: M0 = 10^(1.5·Mw + 9.05) N·m (Hanks & Kanamori 1979; aşağıda
  "Moment bağıntısı"; 2026-11-23'te 9.1'den düzeltildi, kodda
  `tsunami::source::moment`).
- Kırılma boyutları: Wells & Coppersmith (1994) alanı; sığ kabuk doğrultu
  atımlıda uzunluk Thingbaijam vd. (2017) uzunluğuyla sınırlı (Bölüm 7.1,
  2026-11-24). Dalma-batma: WC94 + genişlik sınırları (Bölüm 7.1, Faz 5);
  arayüzde uzunluk TMG17 arayüz uzunluğuyla sınırlı (2026-11-25).
- Tek düzlem dikdörtgen: strike, dip, rake, odağın düzlemdeki konumu.
- Kısıtlar: üst kenar yüzeyin üstüne çıkamaz (Ztor ≥ 0). Kabukta
  genişlik sismojenik kalınlıkla sınırlı (~15–25 km, doğrula);
  genişlik doyunca alan korunacak şekilde uzunluk artar.
- Varsayılan eğimler DOĞRULANDI (2026-11-24): doğrultu atımlı 90°, ters
  40°, normal 50° (önce 55°); USGS OFR 2010-1296 sürüm 1.1 Tablo 2,
  Kaklamanos vd. (2011) önerisi (`data/raw/refs/misc/`, SHA256 16413082…).
  Kayma açısı saf mekanizma (0°, +90°, −90°). Arayüzde dip ve strike
  Slab2'den.
- Ortalama kayma D = M0 / (μ·A), μ = 30 GPa (Riquelme vd. 2015 ile aynı;
  `tsunami::source::mean_slip`). Tsunami için.
- Düzlem ~5–10 km'lik alt faylara bölünür. Alt fayın kırılma zamanı =
  odağa uzaklık / kırılma hızı (≈ 0.8·Vs ≈ 2.8 km/s, doğrula).
  Bu yalnızca zamanlama hesabı içindir, ekranda gösterilmez.
  UYGULAMA (Faz 5, 2026-11-20): ayrık alt fay yerine sürekli hali: sahaya
  en yakın düzlem noktası doğrudan bulunur (`source::Fault::peak_path`).
  Kırılma hızı DOĞRULANDI ve fay tipine bağlandı: Vera, Tilmann & Saul
  (2024), JGR Solid Earth 129, e2023JB027260 (CC BY), 2010–2022 tüm
  Mw ≥ 7.5 (56 deprem): ters 2,1 km/s (%56 Vs), normal 2,5 (%60),
  doğrultu atımlı 2,7 (%78). "0.8·Vs" yalnızca doğrultu atımlıya uyuyor.

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
- t_uyarı = P'nin merkez üssünden D = 10 km'deki sanal İLK istasyona
  varışı (JMA "地震検知") + algılamadan ilk rapora geçen süre 4,5 s
  (2026-11-24). Süre DOĞRULANDI: JMA 験震時報 73 (2010) 1–122 (Doi vd.),
  ek tablo "地震検知から第1報までの時間", 2007-10 – 2008-12 arası 44 olay:
  medyan 4,55 s (çeyrekler 4,0–5,8; uyarılı 9 olayda medyan 4,1); ikinci
  istasyonu beklemeyi içerir. D = 10 km VARSAYIM (K-NET aralığı ~20 km'nin
  yarısı). Önceki model: 20 km'deki ikinci istasyon + 5 s (kaynaksız).
  Açık deniz (2026-11-27): sanal ağ KARADA; ilk istasyon en az en yakın
  kara kadar uzakta (`region::nearest_land_km`, 300 km'ye kadar). 2011
  Tōhoku (JMA: oluş 14:46:18,1, algılama +22,1 s, ilk rapor +27,5 s, uyarı
  +30,7 s): en yakın kara 118 km → model uyarısı ~25 s (eski: ~9 s,
  gerçekten 20 s erken). Büyüklük doygunluğu (büyük depremde ilk
  raporların büyüklüğü düşük tahmin etmesi) MODELLENMEDİ: uyarı kapsamı
  nihai tahminle. İleri seviyede:
  yoğun ağ / seyrek ağ / sistem yok (henüz yok).
- Bölge değeri: bölgenin yerleşim hücrelerindeki en yüksek tahmini Shindo
  (docs/arayuz.md "İl değeri").
- Her şehir için iki değer gösterilir: "S − P" ve "erken uyarı
  süresi" (t_S − t_uyarı). Sıfır veya negatifse "kör bölge". (Uyarı
  süresi, uyarı yayımlandıysa ve şehirde Shindo ≥ 4 ise gösterilir.)

Sarsıntı süresi (Faz 4, 2026-11-14): `seismira-core/src/duration.rs`,
Afshari & Stewart (2016) D5-95, havza terimi yok. pygmm 0.9.1 ile 810
karşılaştırmada ln farkı < 1e-6 (`data/test/as16_ref.csv`). t_bitiş =
t_tepe + D5-95.

Alt fay zamanlaması (Faz 5, 2026-11-20): t_tepe = en yakın fay noktasının
kırılma zamanı (düzlem üzerinde odaktan uzaklık / kırılma hızı, 7.1) + o
noktadan sahaya ak135 S süresi (`Scenario::physical_peak`). Gösterilen
Shindo S varışında 1'den başlar, sınıf sınıf artar ve son değere t_tepe'de
ulaşır; adım en az 0,4 s (görsel alt sınır, `RISE_STEP_S`). Adım hücre
başına saklanır (`SiteResult::rise`, `ShakeGrid::rise`, GPU dokusunun b
kanalı); şehir kutusu, ızgara ve bölge özetleri aynı kuralı kullanır.
Tōhoku benzeri senaryoda (geçici 328 km fay) Tokyo son değerine ~T+145 s'de,
kuzey Hokkaidō ~T+165 s'de ulaşıyor. Yükseliş doğrusal bir görsel
yaklaşımdır: aradaki değerler kırılmanın o ana kadarki kısmıyla
hesaplanmaz. Girdiler model aralığına (M 3–7,9, Rrup 0–200 km, Vs30
200–1000) çekilir; süre yalnızca bölge sınırının "yatışmış" durumuna
geçişinde kullanılır (doğrula: makalenin veri aralığı).

### 7.4 Yer hareketi (GMPE)

- Çıktılar: PGA, PGV, SA(0.3 s), SA(0.6 s), SA(1.0 s). Modelde
  olmayan periyot komşu periyotlardan log-log ara değerle bulunur.
- Sığ kabuk (2026-11-24'ten beri): ASK14, BSSA14, CB14, CY14 eşit
  ağırlıklı ortalama, her modelin bölgesel düzeltmeleriyle (bölge:
  merkez üssünün ülkesi). Ayrıntı ve gerekçe: aşağıda "7.4 Dört
  NGA-West2 modelinin ortalaması". (2026-11-24'e kadar yalnız CY14;
  BSSA14'ün derinlik terimi olmadığı için tek başına kullanılmaması
  kuralı ortalamada geçerli değil: diğer üç model derinliğe duyarlı.)
- Dalma-batma arayüzü ve levha içi: Parker vd. (2020); alternatif
  Kuehn vd. (2020). Bölgesel katsayılar konuma göre seçilir.
  (BC Hydro modeli PGV vermediği için kullanılmaz, doğrula.)
- Japonya seçeneği: Morikawa & Fujiwara (2013); aletsel şiddeti
  doğrudan da veriyor olabilir (doğrula).
- Kararlı kıta içi bölgeler: ilk sürümde özel model yok; kabuk
  modeli kullanılır ve "bu bölge için model uygun değil" uyarısı
  çıkar. İleride NGA-East. UYGULANDI (2026-11-25): bölge çokgenleri
  USGS STREC `stable.geojson` (kamu malı; commit 6e13af7f; ShakeMap'in
  tektonik rejim aracı), data/regions/stable.sout. Merkez üssü içerideyse
  ve ortam sığ kabuksa `Issue::StableContinental` (engellemez). Dalma-batma
  ve levha içinde uyarı yok (o modeller kendi ortamları için). Sağduyu
  denetimi (test `tectonic::strec_dosyasi`): Meckering 1968, New Madrid,
  Virginia 2011, Bhuj 2001, Kanada kalkanı içeride; İzmit, Kahramanmaraş,
  Ridgecrest, Kumamoto, Noto dışarıda. Not: STREC Kuzey Denizi'ni de
  kararlı sayıyor, Sahra'yı saymıyor (veri olduğu gibi kullanıldı).
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
- UYGULAMA (Faz 6, 2026-11-20; `seismira-core/src/exposure.rs`,
  `damage.rs`):
  - Veri: ESRM20 (CC BY 4.0) bina stoku 44 ülke, Natural Earth Admin-1
    bölgelerine toplanmış (docs/veri.md); kırılganlık
    `fragility_ESRM20_Various_IM.xml` (511 tip; PGA, SA 0,3 / 0,6 / 1,0 s).
    Yalnızca Avrupa ve Türkiye; diğer yerlerde kart "bina verisi yok" der.
  - Stok bölgenin yerleşim hücrelerine (GHSL ≥ %1, 30″) EŞİT dağılmış
    kabul edilir. Sarsıntı ızgarası adımının iki katıyla örneklenen her
    yerleşim hücresi bölgenin adım² / toplam payını temsil eder; model
    alanı dışındaki hücreler hasarsız. Hücrede dört şiddet ölçüsü
    (`Scenario::ln_motion`; SA 0,6 s tabloda yok, 0,5 ile 0,75 arasında
    log-log ara değer) bölgenin bina karışımına uygulanır.
  - Belirsizlik: P(DS ≥ k) = Φ((ln IM − ln θ_k) / √(β² + φ²)), φ olay içi
    sapma; olası aralık ln IM ± τ. Φ: Numerical Recipes `erfcc` (Abramowitz
    & Stegun tablosuyla test).
  - Hasarsızlık sınırı (OpenQuake `noDamageLimit`, 0,05 g) medyan şiddete
    uygulanır (OpenQuake tek değere uygular).
  - Bileşen: GMPE'ler RotD50; ESRM20 eğrileri ortalama yatay bileşen için
    türetilmiş (Crowley vd. 2021, doğrula); düzeltme yapılmadı.
  - Şehir kartı: şehrin sahasında, bulunduğu bölgenin bina karışımıyla
    hasar ORANI (sayı değil); "ilin bina dağılımıyla" etiketli.
  - Kart gösterimi (kullanıcı kararı 2026-11-20): bina sayısı + yüzde;
    sayılar 2 anlamlı basamağa yuvarlanır.

- UYGULAMA (Faz 9, 2026-11-23; `seismira-core/src/vulnerability.rs`,
  `damage.rs::loss_regions`, `seismira-data/src/gem.rs`): Avrupa ve Türkiye
  DIŞI için GEM paketi (CC BY-NC-SA 4.0, ayrı paket, `data/gem/gem.svul`).
  - Veri: GEM Global Exposure Model v2026.0.0 özetleri
    (`Exposure_Summary_Adm1_Taxonomy.csv`: Adm1 × bina sınıfı; 215 ülke) ve
    GEM Global Vulnerability Model (ülke başına
    `vulnerability_structural.xml`, NRML 0.5; PGA / SA 0,3 / 0,6 / 1,0 s).
    Ayrıntılı (alt il) GEM verisi lisans başvurusu ister; kullanılmadı.
  - Bina eğrisi (2026-11-23): her sınıf için LR = s·LR_yapısal +
    (1 − s)·LR_yapısal olmayan (`vulnerability_nonstructural.xml`, aynı
    sınıf adı ve IMT; iki eğrinin düğüm birleşiminde, `gem` komutunda).
    s = yeniden yapım değerinde yapısal pay, GEM ABD stokundan kullanım
    başına (RES 0,215, COM 0,214, IND 0,157; Hazus maliyet ayrımı); diğer
    ülkeler için VEKİL (VARSAYIM; ABD'de sınıflar arasında 0,14–0,24).
    Eşi olmayan sınıf ve yapısal olmayan dosyası indirilmemiş ülke yalnızca
    yapısal eğriyle kalır. İçerik (contents) kaybı dahil değil.
    Gerekçe: Japonya'da yapısal olmayan kayıp orta şiddette yapısalın 2–13
    katı (PGA 0,3–0,5 g); Kumamoto doğrulaması yalnızca yapısalla ×0,13–0,66
    düşük kalıyordu.
  - Çıktı hasar durumu DEĞİL, ortalama bina KAYIP ORANI (yeniden yapım
    değerine oran, 0–1): zafiyet fonksiyonu ortalaması, OpenQuake
    enterpolasyonuyla (üstten kırp, alttan 0, IML'de doğrusal). Kayıp
    oranının kendi değişkenliği (covLRs) kullanılmaz.
  - Bölge oranı: sınıfların yeniden yapım değeri (`BLDG_REPL_COST_USD`,
    yapısal + yapısal olmayan) ağırlıklı ortalaması. VARSAYIM: yapısal pay
    sınıflar arasında çok değişmez (yalnızca ABD'de ayrı veriliyor:
    0,10–0,61). Sınıf eğrileri bölge başına ölçü başına tek parçalı doğrusal
    eğride birleştirilir (kesin; hücre başına 4 eğri).
  - Örnekleme ve belirsizlik ESRM20 ile aynı: yerleşim hücreleri, φ ile
    log-normal integral (201 nokta, ±5σ), olası aralık ±1τ.
  - Bölge eşlemesi: ISO 3166-2 → kod değişiklikleri tablosu → ülke içinde
    ad → kalan birimler ülkenin stoksuz bölgelerine yerleşim hücresi
    oranında (küresel değerin %13'ü, büyük kısmı Avrupa; Japonya, ABD,
    Meksika, Şili birebir). Sınıf → fonksiyon: ülke eşleme tablosu; ABD ve
    Kanada'da Hazus sınıf adı (RES3A–F → RES3). Fonksiyonu bulunamayan
    değer %0,5 (Türkiye 764 milyar USD, Kanada 457 milyar USD: zafiyet
    dosyasında olmayan sınıflar; Türkiye zaten ESRM20 kullanır).
  - Sonuç: 2016 Kumamoto dizisi (Mw 6,2 + 7,0) Kumamoto ili %2,7 (olası
    1,6–4,4), gözlenen %2,0–10,0; doğrulama: docs/dogrulama.md Faz 9.

- Hasarsızlık sınırı (`noDamageLimit`, 2026-11-27): NRML tanımıyla
  şiddet L'nin altındaysa hasar olasılığı sıfır; OpenQuake bunu her yer
  hareketi değerine uygular. Analitik karşılığı şiddet dağılımı üzerinden
  P(DS ≥ k) = ∫_{ln L}^{∞} Φ((x − ln m_k)/β) N(x; μ, φ²) dx; iki değişkenli
  normal dağılımla kapalı biçim (θ integralli standart özdeşlik, Drezner &
  Wesolowsky 1990 / Genz 2004 biçimi; makaleler görülmedi, özdeşlik ince
  Simpson integraline karşı test edildi, fark < 2·10⁻⁵;
  `exposure::truncated_exceedance`).
  Önceden sınır medyana uygulanıyordu: L'si medyana yakın tiplerde
  (ESRM20'de medyan(hafif)/L en küçük 2,1) sınırda P(hafif+) 0 ↔ 0,15
  sıçraması. Etki (Kahramanmaraş `validate-damage`): ağır+göçme il
  oranları ±0,1 puan, il başına ln(model/gözlenen) −0,52 → −0,50; hasar
  hesabı 350 → 395 ms (arka planda).

### 7.9 Sıvılaşma ve heyelan

- Sıvılaşma: Zhu vd. (2017). Girdiler: PGV, Vs30, yağış, kıyıya
  uzaklık, nehre uzaklık, yeraltı su seviyesi (doğrula).
- Heyelan: Nowicki Jessee vd. (2018). Girdiler: PGV, eğim, litoloji,
  arazi örtüsü, CTI (doğrula).
- Modellerin kalibre edildiği çözünürlük ve veri setleri kaynaktan
  doğrulanır; farklı veri seti kullanılırsa belgelenir.
- Çıktı: hücre başına olasılık; haritada ~25 km'lik kutulara toplanıp içi boş
  kırmızı kareler (gösterim kuralı docs/arayuz.md "kırmızı kareler")
  (yoğunluk = olasılık).
- FAZ 7 ADIM 1 BULGULARI (2026-11-20, uygulanmadı):
  - Künye düzeltmesi: Zhu, Baise & Thompson (2017), "An Updated Geospatial
    Liquefaction Model for Global Application", BSSA 107(3), 1365–1385,
    doi:10.1785/0120160198 (Earthquake Spectra DEĞİL). Kapalı erişim.
  - Referans uygulama: USGS groundfailure (code.usgs.gov/ghsc/esi/
    groundfailure/groundfailure, CC0 / kamu malı), commit fa7bd289
    (2026-08-27), `src/gfail/models/zhu_2017.py`, `jessee_2018.py`,
    `defaultconfigfiles/models/*.ini`.
  - Zhu 2017 (genel model, USGS kodu): X = 8.801 + 0.334·ln(PGV·SF)
    − 1.918·ln(Vs30) + 0.0005408·yağış(mm) − 0.2054·min(dc, dr)(km)
    − 0.0333·wtd(m); SF = 1/(1 + e^(−2(M−6))). Kırpma: yağış 0–2500 mm,
    PGV 0–150 cm/s. Alansal kaplama: 0.4915 / (1 + 42.4·e^(−9.165·P))².
    Maske: eğim 0–5° dışı, Vs30 > 620 m/s, PGV < 3 cm/s, PGA < %10 g.
  - Nowicki Jessee 2018 (USGS kodu): X = −6.30 + 1.65·ln(PGV)
    + 0.06·eğim(°) + litoloji + 0.03·CTI + arazi örtüsü
    + 0.01·ln(PGV)·eğim(°); litoloji ve arazi örtüsü sınıf katsayıları
    girdi dosyalarına gömülü (GLIM_replace, globcover_replace): MAKALE
    TABLOSU GEREKLİ (makale açık erişim ama yayıncı betiğe 403; kullanıcıdan
    PDF istendi). USGS değişikliği: pekişmemiş tortul −3.21 → −1.36
    (makalede −3.22 yazıyor olabilir; doğrula). Kırpma: CTI 0–19, PGV 0–211.
    Alansal kaplama: exp(−7.592 + 5.237P − 3.042P² + 4.035P³). Maske:
    eğim < 2° (ini) / < 5° (USGS sayfası), PGA < %2 g.
  - Girdi verisi durumu (betikle erişim, 2026-11-20): GMTED2010 7,5″
    (USGS edcintl, erişilebilir), CHELSA bio12 yağış (erişilebilir; USGS
    WorldClim kullanmış), HydroRIVERS v1.0 (erişilebilir), Fan 2013 su
    seviyesi (thredds-gfnl.usc.es, erişilebilir), GLiM (PANGAEA dosya
    bağlantısı 500 verdi), GlobCover 2009 (ESA DUE sitesi yanıt vermiyor),
    HYDRO1k CTI (USGS eski adresi 404), NASA kıyı uzaklığı (OB.DAAC'a
    taşınmış). Lisansların HİÇBİRİ henüz kontrol edilmedi.
- JESSEE 2018 MAKALESİNDEN DOĞRULAMA (2026-11-22; PDF kullanıcıdan, JGR
  Earth Surface 123, 1835–1859, Tablo 3 s. 1846, Eş. 8–9; tablo sayfası
  görüntü olarak okundu, işaretler teyitli):
  - Eş. 8: logit(P) = a + b·ln(PGV) + c·eğim + d·litoloji + e·arazi örtüsü
    + f·CTI + g·ln(PGV)·eğim. a = −6.30, b = 1.65, c = 0.06, f = 0.03,
    g = 0.01. PGV cm/s (ShakeMap), eğim derece (GMTED 7,5″ medyan
    yükseklikten, GMT ile), CTI = ln(A / tan α) (Moore 1991, 30″).
    USGS koduyla birebir aynı.
  - Litoloji (GLiM üst sınıflar; "glim75cN" numaraları GLiM kodlarının
    alfabetik sırası: 1 ev, 2 ig, 3 mt … 15 vi, 16 wb): mt −1.87, nd −0.66,
    pa −0.78, pb −1.88, pi −1.61, py −1.05, sc −0.95, sm −1.36, ss −1.92,
    su −3.22, va −1.54, vb −1.50, vi −0.81. Tabloda olmayan ev (evaporit) ve
    ig (buzul) = 0 (R'de ilk düzey referanstır; ev referans, ig eğitim
    verisinde yok sayıldı: VARSAYIM). Su kütleleri (wb) analizden çıkarılmış
    (s. 1840).
  - USGS değişikliği: pekişmemiş tortul (su) −3.22 → −1.36 (kod sabiti
    OLD = −3.21 ile eşleşiyor; makalede −3.22). Gerekçe USGS notunda: düşük
    eğimde temsil edildiği için katsayı aşırı negatif. SEÇİM: USGS'in
    işletimdeki değeri (−1.36) kullanılır, makale değeri belgede kalır.
  - Arazi örtüsü (GlobCover 2009 sınıfları): 14 0.91, 20 0.88, 30 0.78,
    40 0.68, 50 0.30, 60 1.77, 70 1.71, 90 −1.26, 100 1.50, 110 0.68,
    120 1.13, 130 0.79, 140 1.03, 150 0.54, 160 2.34, 180 1.19, 190 0.30,
    200 −0.06, 220 −0.18, 230 −1.08. Tabloda olmayan 11, 170, 210 = 0
    (11 referans düzey; 210 su).
  - Eş. 9 alansal kaplama: LP = exp(−7.592 + 5.237P − 3.042P² + 4.035P³)
    (işaretler PDF metninde kayboluyor; USGS kodu ve Şekil 10 eğrisiyle
    teyit: P = 1'de ~%25,6; Şekil 10 ekseni %0–25).

- FAZ 7 ADIM 2: GİRDİ KATMANLARI (2026-11-22). `seismira-data ground-inputs`
  → data/ground/inputs30.sras (Vs30 ızgarası 30″, ~355 MB, depoda değil;
  4 dk, tepe bellek 8,5 GB). Katmanlar: slope30, cti, dr_km, dc_km,
  precip_mm. Ayrıntı: crates/seismira-data/src/ground_inputs.rs.
  - Akış: öncelikli taşma (Barnes vd. 2014) + D8; çıkışlar okyanus ve
    Vs30 göl kodu (iç havzalarda sahte taşma nehri olmasın diye göller
    havuz; Vs30 göl kodu geniş nehir yataklarını da kapsıyor, bu yüzden
    Amazon gibi havzalar parçalanıyor: en büyük havza 3,08 milyon km²).
  - Nehir: yukarı havza ≥ 10 km² (VARSAYIM, HydroRIVERS ölçütü), iç su
    hücresi ya da Natural Earth 10m nehir orta hattı (büyük nehirlerin
    konumunu sabitler; D8 hattı New Orleans'ta Mississippi'den ~2 km
    kaymıştı). Kara hücrelerinin %17,3'ü; ~38 milyon km, HydroRIVERS'ın
    toplam uzunluğuyla (~35,9 milyon km, aynı eşik) uyumlu. Akarsu yatakları
    düz deltalarda kayabilir (HydroSHEDS gibi akarsu yakma yok; ör. Kahire'de
    Nil'e ~3 km). Adım 5'te USGS ile karşılaştırılacak.
  - CTI: HYDRO1k tanımı (belge doğrulandı, docs/veri.md); FA = 0 için
    FA + 1 hücre VARSAYIMI.
  - Uzaklıklar: 5 × 5 pahlı dönüşüm, 50 km'de kırpılır (0,2 km adım).
  - Eğim: 30″ Horn (slope30, yalnızca CTI'de). Modellerin eğimi 7,5″'ten:
    `seismira-data slope75` → data/ground/slope75.sras (adım 3, aşağıda).
- USGS İŞLETİM AYARLARI (groundfailure `defaultconfigfiles/models/*.ini`,
  main dalı, 2026-11-22 okundu):
  - İki model de aynı eğim dosyasını kullanıyor (`global_grad.tif`, birim
    gradyan, GMTED2010). Jessee: baselayer = slope (model eğim
    çözünürlüğünde çalışır), eğim maskesi 2–90°, PGA ≥ %2 g, maxprob 0,256,
    default_stddev 0,03 (logit). Zhu genel: baselayer = vs30, divfactor 2
    (30″ hücre 2 × 2'ye bölünür, ~15″), eğim maskesi 0–5°, PGV ≥ 3 cm/s,
    PGA ≥ %10 g, Vs30 ≤ 620 m/s, maxprob 0,487, default_stddev 0,05.
    Enterpolasyon: sürekli katmanlar doğrusal, litoloji/arazi örtüsü en yakın.
  - Zhu girdileri USGS'te: yağış WorldClim (mm), dc NASA Ocean Color (km), dr
    HydroSHEDS (km), wtd Fan vd. 2013 (m).
- FAZ 7 ADIM 3: 7,5″ EĞİM ÖZETİ (2026-11-22). Her 30″ hücrenin 16 alt
  hücre eğimi (Horn, md75) sıralanır, 4'lü grupların ortalaması saklanır
  (slope_q0 en düz … slope_q3 en dik, 0–75°, 0,3° adım). Gerekçe: Jessee'de
  eğim ln(PGV) ile çarpım teriminde de var; tek ortalama eğim olasılığı
  bozar. Uygulama: Jessee olasılığı 4 çeyrek üzerinden ortalanır (her çeyrek
  2° maskesini ayrı uygular); Zhu'nun 0–5° maskesi = 5°'yi aşmayan çeyrek
  oranı (USGS'in 15″ alt bölmesine yakın bir alansal kesir).
  - Eksik girdiler: yeraltı su seviyesi (Fan WTD, izin bekleniyor), litoloji
    (GLiM vektör lisansı), arazi örtüsü (GlobCover / ESA CCI lisansı).
    2026-11-22 gece: litoloji (GLiM vektörü, yerel) ve arazi örtüsü (ESA CCI
    2009) hazır; aşağıda.
- HEYELAN ARAZİ ÖRTÜSÜ: CCI → GLOBCOVER EŞLEMESİ (VARSAYIM, 2026-11-22).
  Jessee 2018 katsayıları GlobCover 2009 sınıflarına göre; GlobCover'ın
  kendisi edinilemedi (docs/veri.md), ESA CCI LC 2009 kullanılıyor. İki
  lejant da UN-LCCS'den türetilmiş ama kodlar farklı. Eşleme CCI PUG v2
  Tablo 3-1 ile Jessee Tablo 3'teki GlobCover sınıf adlarının
  karşılaştırılmasından kuruldu (seismira-core
  `jessee2018::cci_to_globcover`, birim testli):
  - Birebir: 10–12 → 14, 20 → 11 (referans, 0), 30 → 20, 40 → 30,
    50 → 40, 61 → 50, 62 → 60, 71 → 70, 72 → 90, 90 → 100, 100 → 110,
    110 → 120, 120–122 → 130, 130 → 140, 150–153 → 150, 160 → 160,
    170 → 170, 180 → 180, 190 → 190, 200–202 → 200, 220 → 220.
  - CCI 80–82 (iğne yapraklı yaprak döken) → 90: GlobCover'da kapalı iğne
    yapraklı yaprak döken sınıfı yok, 90 "açık, yaprak döken ya da her dem
    yeşil".
  - CCI 140 (liken, yosun) → 140 (otsu): GlobCover'da karşılığı yok.
  - CCI 60 ve 70 ("kapalı-açık, >%15"; 2009'da sırasıyla 9,5 ve 13,6
    milyon hücre) GlobCover'da yoğunluğa göre ikiye ayrılmış: 50/60
    (0,30 / 1,77) ve 70/90 (1,71 / −1,26). İlk sürümde iki katsayının
    ortalaması alınmıştı; 2026-11-23 doğrulaması (docs/dogrulama.md, Faz 7
    heyelan) KAPALI eşlemeyi (60 → 50, 70 → 70) açıkça destekledi
    (Kahramanmaraş oran 0,90 / korelasyon 0,94; Noto 0,88 / 0,95; ortalama
    Noto'da 0,28, açık 0,27 / 0,35). Uygulama kapalı eşlemeyi kullanır.
    İyileştirme adayı: ağaç örtüsü yoğunluğu katmanıyla (ör. MODIS VCF) %40
    eşiğinde ayırmak.
    Başka aday: Copernicus CGLS-LC100 2019 (Zenodo 3939050,
    `PROBAV_LC100_global_v3.0.1_2019-nrt_Discrete-Classification-map_EPSG-4326.tif`,
    1 713 345 228 B): kapalı (111–116) / açık (121–126) orman sınıfları
    doğrudan var. 2026-11-22'de indirme yarıda kesildi (1,02 GB, açılamıyor),
    2026-11-23'te silindi; gerekirse yeniden indir, lisansı kontrol et.
  - Su (210) ve veri yok (0) → model uygulanmaz.
- HEYELAN STATİK ÖN HESAP (2026-11-22): `seismira-data ls-static` →
  data/ground/ls_static.sras (365 MB, 2 dk; biçim seismira-core
  `groundfailure::landslide_static`). A = −6,30 + litoloji + 0,03·CTI +
  arazi örtüsü 8 bit (±0,013 logit) + dört 7,5″ eğim çeyreği (slope75
  kodları, ≤ 2° olanlar 0). 106,5 milyon hücre; litolojisi yok 0,92 M,
  arazi örtüsü yok 0,26 M (maskeli). Başvuru sarsıntısında (PGV 50, PGA
  0,3 g) kodlu / tam kaplama toplamı 1,00007.
  - GLiM'den türediği için izin gelene kadar YALNIZCA YEREL.
  - Boyut denemeleri (Faz 10 paketleme için): çeyrek farkları (kayıpsız)
    322 MB; + eğim adımı 0,6° 284 MB; + 0,9° ve A adımı 0,05 246 MB.
    Çeyrekler yerine "aşan çeyrek sayısı + ortalama eğim" REDDEDİLDİ:
    toplam kaplama PGV 10/30/50/100 cm/s'de %7 / %12 / %7 / %1 düşük.
    Kullanıcı (2026-11-22): "çok sıkıştırmaya gerek yok"; kayıpsız biçim
    kaldı.

- FAZ 7 ADIM 4: SIVILAŞMA UYGULAMADA (2026-11-22). seismira-core
  `groundfailure` (Zhu genel + kıyı, Jessee; USGS koduyla birebir birim
  testleri) ve `groundfailure::liquefaction_grid` (ince sarsıntı ızgarasıyla
  aynı hücreler ve aynı temsilci taban hücre). Kararlar:
  - PGV: GMPE'nin RotD50 medyanı Boore & Kishida (2017) ile "iki yatay
    bileşenin büyüğü"ne çevrilir (ShakeMap uzlaşımı; USGS modelleri ShakeMap
    PGV'siyle kalibre). PGA: RotD50 kalır (PGA için dönüşüm katsayısı
    projede yok); yalnızca %10 g eşik maskesinde kullanılır, sonuç biraz
    daha fazla hücre maskelenir (SAPMA, belgelendi).
  - Vs30: haritanın ham değeri (GMPE sınırına çekilmemiş), USGS gibi.
  - Eğim maskesi: 5°'yi aşmayan 7,5″ çeyrek oranı. USGS eğim > 0 koşulu da
    koyuyor (tam düz hücreyi dışlar; muhtemelen su/veri yok için); burada
    0° dahil (SAPMA, düz delta hücreleri dışlanmasın diye).
  - wtd: data/ground/wtd.sras varsa ve hücrede değer varsa genel model,
    yoksa kıyı modeli (hücre bazında). İzmit Mw 7,6 denemesi: 25 276 hücre,
    25 141 genel / 135 kıyı; hesap 85 ms.
  - Kaplama medyan sarsıntıyla ve ayrıca PGV, PGA × exp(±τ) ile
    hesaplanır (e0004fc; heyelan da aynı yolla). Bölge kartındaki km²
    "olası aralığı" bu −1τ / +1τ kaplamalarının toplamıdır; Zhu 2017 / Jessee
    2018 modellerinin kendi belirsizliği (USGS default_stddev) EKLENMEZ.

- FAZ 7: PAKET BOYUTU, STATİK KATMAN (2026-11-22). Zhu 2017 logiti
  X = statik(Vs30, yağış, dc, dr, wtd) + b·ln(PGV·SF) biçiminde ayrılır
  (`zhu2017::static_logit`, `pgv_coef`); statik kısım, düz çeyrek sayısı ve
  model (genel/kıyı) hücre başına önceden hesaplanır: `seismira-data
  liq-static` → data/ground/liq_static.sras (`groundfailure::static_layer`).
  Uygulama yalnızca bu dosyayı okur; ham girdiler (inputs30 + slope75 + wtd
  = 927 MB) yalnızca veri hazırlama ve duyarlılık denemeleri içindir.
  - Niceleme: X doğrusal −9…+6, 251 kod (adım 0,06). X < −9 kırpılır (en
    güçlü sarsıntıda bile P < 6,3·10⁻⁴; kaplama tabandan en çok %1,1 farklı;
    140,7 milyon hücreden 180 718'i). 16 bit ince düzeltme (liq_xf) biçimde
    tanımlı ama YAZILMIYOR: dosyayı 136 → 292 MB büyütüyor.
  - Hata (8 bit): başvuru sarsıntısında (M 7,5, PGV 50 cm/s) dünya toplamı
    oranı 1,00026, tek hücrede en büyük göreli fark %10,6 (düşük X'li
    hücreler; sınıf sınırları ve model belirsizliği yanında küçük).
    Kahramanmaraş uçtan uca: 1177,93 / 1177,81 km² (oran 1,0001).
  - Lisans: wtd'li (genel model) dosya Fan WTD'den türetilmiştir, yalnızca
    yereldir. Dağıtılacak paket `--coastal` ile (her hücre kıyı modeli)
    üretilir; prepare_data.py wtd.sras yoksa bunu kendiliğinden seçer.
    Dağıtılabilir sürüm 129,6 MB (8 bit). Başvuru sarsıntısında kıyı
    modelinin dünya toplamı genel modelinkinin 0,74 katı (model farkı;
    izin gelirse genel modele geçilir).
  - Heyelan (Jessee) eğimi ln(PGV) ile çarpım teriminde taşıdığı için aynı
    yolla tek sayıya indirgenemez; çeyrek başına statik kısım gerekecek
    (heyelan katmanı geldiğinde, ayrı paket olarak düşünülecek).

### 7.10 Tsunami

- Renkler: SARI az tehlikeli, KIRMIZI tehlikeli, MOR çok tehlikeli.
  JMA kategorileri, DOĞRULANDI (2026-11-24, jma.go.jp/jma/kishou/know/
  jishin/joho/tsunamiinfo.html "発表基準"): sarı 津波注意報 0,2 m ≤ h ≤ 1 m,
  kırmızı 津波警報 1 m < h ≤ 3 m, mor 大津波警報 h > 3 m ("高いところで",
  bölgenin en yüksek değeri). 0,2 m altı: uyarı yok.
- Aşama 1, tetikleme: deniz altında veya kıyıya yakın, sığ
  (~100 km'den az), büyük (~Mw 6.5–7 üstü) depremler. Eşikler
  yayımlanmış uyarı ölçütlerinden doğrulanır.
- Aşama 2, kaynak ve varış: Okada (1985) ile sonlu faydan deniz
  tabanı düşey yer değiştirmesi (fay tipi etkisi buradan gelir;
  doğrultu atımlıda küçük çıkar). Derinlik (ETOPO 2022, okyanusta GEBCO
  tabanlı; docs/veri.md) üzerinde √(g·h) hızıyla,
  KAYNAK ALANINDAN başlayan fast marching ile varış süresi haritası.
  Kıyı yüksekliği: Abe'nin ampirik bağıntısı (Abe 1995, doğrula)
  veya Green yasası; "kaba tahmin" etiketli.
- İlk uyarı zamanı: depremden ~3 dakika sonra, DOĞRULANDI (aynı JMA
  sayfası: "地震が発生してから約３分（一部の地震については約２分）を目標に";
  2 dk Japonya yakınında erken uyarı tekniğiyle hızlı çözülen depremler
  için; uygulamada tek değer 3 dk, `ISSUE_S`). Bildirim ve kıyı vurgusu o anda başlar.
- Yalnızca okyanusa bağlı kıyılar; göller ve iç sular maskelenir.
- Aşama 3 (isteğe bağlı): sığ su denklemleri, wgpu compute shader.
- KAYNAK (Faz 8 adım 2, 2026-11-23): IOC Technical Series 135 (2017),
  PTWC Enhanced Products User's Guide for CARIBE-EWS (weather.gov PDF;
  data/raw/papers/PTWC_CARIBE_Users_guide_2017.pdf).
  - Tablo 2 (ilk metin ürünleri; Karayip dışı satırlar): Mw < 6,5 ürün
    yok; deniz altında / kıyıya çok yakın ve Mw 6,5–7,0 → bilgi bildirimi
    ("too small"); karada → bilgi ("too far inland"); derinlik ≥ 100 km ve
    Mw ≥ 7,1 → bilgi ("too deep"); derinlik < 100 km: 7,1–7,5 tehdit (300 km
    içi kıyılar), 7,6–7,8 (1000 km), ≥ 7,9 (varış ≤ 3 saat olan kıyılar).
    Uygulama: seismira-core `tsunami::trigger::assess` (birim testli).
    "Kıyıya çok yakın" tanımı tabloda yok: uygulamada VARSAYIM (adım 6'da
    yazılacak).
  - Zaman çizelgesi (Tablo 1): ilk ürün depremden 3–10 dk sonra. İlk uyarı
    anı için JMA hedefi (~3 dk) DOĞRULANDI (2026-11-24, yukarıda).
  - Kıyı yüksekliği (Bölüm II, RIFT): Green yasası A_kıyı = A_açık ·
    (D_açık / D_kıyı)^¼, D_kıyı = 1 m; açık nokta, kıyı noktasından daha
    derin olan EN YAKIN model noktası; 100 km'den uzaksa kıyı tahmini
    YAPILMAZ. PTWC tehdit eşiği: tahmini kıyı genliği > 0,3 m. Sınırlar:
    ilk sonuçlar kolayca 2 kat değişebilir; küçük adalarda (< 30 km) ve
    resifli kıyılarda Green yasası fazla verir.
- KIYI YÜKSEKLİĞİ (Faz 8 adım 5, 2026-11-23): Abe (1995) kitap bölümü
  kapalı erişim → Riquelme vd. (2015) JGR 120, 6487–6500 (CC BY-NC-ND;
  kullanıcı PDF'i getirdi, data/raw/papers/). seismira-core
  `tsunami::runup`: Eş. (3) R = 2,831 H (H/d)^¼ (1 + H/d)^¼ |b|^(−½)
  (çok eğimli kıyı, son eğim b) ve Eş. (2)'nin kıyı boyunca Lorentz
  azalması f(p) = 1/(1 + (2p/L)²), p = α·y (θ = 90°, makalenin en kötü
  durum önerisi). Kaynak: `tsunami::source::uplift`: düzgün kaymalı fay
  (uygulamanın sarsıntı düzlemi), D = M0/(μA), μ = 30 GPa, Okada uz; H =
  en büyük |uz|. Geçerlilik (makale): dalma-batma yakın alanı; doğrultu
  atımlı, ada, koy, haliç kapsam dışı; ilk dalga. UZAK ALANDA YÜKSEKLİK
  VERİLMEZ (yayımlanmış basit yöntem bulunamadı; PTWC RIFT sığ su modeli
  kullanır = Aşama 3): yalnızca PTWC tehdit bölgesi ve varış süresi.
- TAHMİN ZİNCİRİ (Faz 8 adım 5, 2026-11-23): seismira-core
  `tsunami::forecast::compute`. VARSAYIMLAR (hepsi kodda sabit, adım 7'de
  sınanacak): kaynak hücreleri = en büyük yer değiştirmeyle aynı işaretli
  ve |uz| ≥ %25 H (%10 ve işaretsiz, Tōhoku'da kıyı çökmesini kaynak sayıp
  varışı 0 dk veriyordu); tepe noktası = H'nin %90'ını geçen hücrelerden
  merkez üssüne en yakını (dar tepe 2′ ızgarada ±%10 dalgalanıyor); d =
  tepe noktasının derinliği (≥ 50 m); şelf eğimi b = en yakın kıyı
  noktasından kaynağa doğru 200 m eş derinliğe ortalama eğim (300 km
  içinde; bulunamazsa tepeye kadar ortalama), 5e-4–0,1'e kırpılır; yakın
  alan = doğrultuya dik uzaklık ≤ 300 km ve doğrultu atımlı DEĞİL;
  "deniz altında" (PTWC) = en az bir kaynak hücresi denizde.
  Deneme (`seismira-data tsunami-scenario`, Tōhoku: USGS merkez üssü,
  193°/14°/81°, arayüz, 1′ pencere ± 8°): fay 547 × 196 km, kayma 11,0 m,
  H 4,98 m (143,52°D 38,06°K), d 4179 m, b 0,0057, R0 34,7 m (gözlenen
  en yüksek ~40 m; Riquelme FFM ile 41 m), Ishinomaki kıyısı varış
  16–56 dk; hesap 0,3 s.
- KORUNAKLI KIYI (2026-11-24): Riquelme vd. (2015) koy ve haliçleri kapsam
  dışı sayar. `tsunami::forecast::sheltered`: kıyı noktasından denizdeki en
  yakın kaynak hücresine (|uz| ≥ %25 H; 0,25° kovalarla seyreltilmiş)
  giden büyük çember üzerinde, ilk 5 km atlandıktan sonra ≥ 20 km
  kesintisiz kara varsa kıyı kaynağı doğrudan görmez (körfez içi,
  yarımadanın arkası, karşı kıyı): yakın alan yüksekliği VERİLMEZ, uzak
  alan gibi yalnızca PTWC tehdidi ("dikkat", sarı) ve varış. Ölçüt
  VARSAYIM; 5 km eşiği denendi, Noto'da yarımadanın arka kıyılarını da
  (dalganın dolaştığı Suzu / Iida koyu) korunaklı saydığı için 20 km
  seçildi (docs/dogrulama.md). Hedef tepe noktası değil denizdeki kaynak:
  Noto'da en büyük yükselme karada.
- KÜÇÜK ADA (2026-11-24): `tsunami::forecast::small_land`: kıyı noktasının
  komşu kara hücresinden sınırlı taşma doldurma; kara parçasının doğu-batı
  ve kuzey-güney uzanımının büyüğü < 30 km ise (pencere kenarına
  değmiyorsa) küçük ada (arama 30 km'de durur; süre etkisi yok).
  Komşu kara hücreleri yalnızca küçük adaya ait kıyı noktasına yakın alan
  yüksekliği VERİLMEZ (korunaklı kıyı gibi). Kaynak: Riquelme vd. (2015)
  adaları kapsam dışı sayar; 30 km PTWC'nin (IOC TS 135, Bölüm II) "küçük
  adalarda (< 30 km) kıyı tahmini fazla verir" sınırı.
- UYGULAMA (Faz 8 adım 4, 2026-11-23): varış süresi, seismira-core
  `tsunami::travel` (birinci dereceden fast marching, 4 komşu, iki yönlü
  güncelleme; boylam-enlem ızgarada dx = R cos φ Δλ). Kaynaklar: Sethian
  (1996) PNAS 93(4) 1591–1595 ve Sethian (1999) kitabı (yöntem; formül
  standart eikonal güncellemesi). Test: sabit derinlikte nokta kaynak,
  analitik süreye göre eksende < %1, 45° köşegende +%0,66.
  Gerçek veri denemesi (`seismira-data tsunami-eta`, nokta kaynak 143,0°D
  38,1°K, k × k birleştirilmiş 1′ derinlik): 4′ küresel ızgara (5400 × 2700)
  2,0 s, 2′ (10800 × 5400) 8,7 s (+ 1,4 s derinlik açma). Varış: Hilo 7,96 /
  7,95 sa, Honolulu 7,56 / 7,75, Crescent City 10,05 / 9,85, Şili (Talcahuano
  yakını) 22,5 / 22,6, Sendai kıyısı 1,2 sa. Gözlemlerle karşılaştırma
  adım 7'de (kaynaklı varış verisiyle; burada nokta kaynak).
- UYGULAMA (Faz 8 adım 3, 2026-11-23): Okada (1985) PDF'i NIED'den
  (bosai.go.jp/e/pdf/Okada_1985_BSSA.pdf; data/raw/papers/). seismira-core
  `tsunami::okada85::displacement`: sonlu dikdörtgen kaynak, yüzey (ux, uy,
  uz), Eş. (24)–(30), cos δ = 0 için Eş. (29), tekillik kuralları (i)–(iii)
  (s. 1148). Tablo 2'nin üç sonlu kaynak durumu (x=2, y=3, d=4, δ=70°;
  x=y=0, δ=±90°; L=3, W=2; λ=μ) doğrultu / eğim / açılma için 4 anlamlı
  basamakla tutuyor (ilk denemede, düzeltme gerekmedi). MathWorks
  "okada85" sayfasındaki "üç denklem düzeltmesi" notu gerinim/eğim
  formülleri içindir; yer değiştirmede görülmedi. Gerinim ve eğim
  uygulanmadı (gerek yok). d: fayın ALT kenarının derinliği; uygulamada
  üst kenar derinliği + W sin δ.


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
  uyarılır (engellenmez; `Issue::RuptureSizeExtrapolated`). Yalnız WC94 ile
  20 km genişlikte doğrultu atımlı fay Mw 7.6 → 200 km, 7.8 → 320 km,
  8.0 → 512 km, 8.5 → 1 657 km oluyordu (2026-11-24'e kadar).
- Doğrultu atımlı sığ kabuk (2026-11-24): uzunluk = min(WC94 alanından
  uzunluk, Thingbaijam, Mai & Goda 2017 uzunluğu). Künye: BSSA 107(5),
  2225–2246, doi:10.1785/0120170017; Tablo 1 "Strike-slip": log10 L =
  −2.943 + 0.681·Mw (σ 0.151), log10 W = −0.543 + 0.261·Mw (σ 0.105);
  veri Mw 5.38–8.70, L 6–580 km, W 6.5–50 km. Birincil kaynaktan
  DOĞRULANDI (yazar kabul kopyası, Bristol; kullanıcı tarayıcıdan indirdi).
  20 km genişlikte iki uzunluk Mw ≈ 7.40'ta eşitlenir: altında WC94 aynen,
  üstünde TMG17 (Mw 7.6 → 171 km, 7.8 → 234, 8.0 → 320, 8.5 → 700).
  Genişlik WC94 kolundaki gibi; alan korunmaz. Ekstrapolasyon uyarısı
  doğrultu atımlıda TMG17 aralığına (8.7) göre: girdi sınırı 8.5 olduğu
  için çıkmaz. Neden yalnız TMG17 değil: Mw ~7'de gözlenenden uzun
  (Kumamoto 67 km, Düzce 83 km) ve doğrulamada sapmayı büyüttü
  (docs/dogrulama.md "Doğrultu atımlı fay uzunluğu"). Ters/normal kabuk
  WC94'te kaldı (TMG17 ters fay verisi Mw ≤ 7.69). Arayüz için bkz.
  aşağıdaki madde.
- Dalma-batma arayüzü (2026-11-25): uzunluk = min(WC94 alanından uzunluk,
  TMG17 arayüz uzunluğu), doğrultu atımlıyla aynı yol. Tablo 1
  "Subduction-interface": log10 L = −2.412 + 0.583·Mw (σ 0.107), veri
  Mw 6.68–9.19, L 29.2–1 420 km (yazar kopyası s. 68, birincil kaynaktan
  DOĞRULANDI). Gerekçe: eğim kabuk ters fay varsayılanında (40°) kalırsa
  genişlik 50 / sin 40° ≈ 78 km'de doyuyor, WC94 alanı uzunluğu
  patlatıyordu (Mw 9.0 → 1 379 km, 9.3 → 2 791 km; sonra 684 ve 1 023 km).
  Slab2 eğiminde (~10–20°) WC94 uzunluğu genelde daha kısa ve değişmez
  (Mw 9.0, 14° → 547 km). Yalnız TMG17 kullanılmadı: makale (s. 23)
  ilişkinin 2004 Sumatra'ya (~1 000 km) uyduğunu, Tōhoku'yu (~350 km) uzun
  verdiğini söylüyor. Makale ayrıca küresel ölçekte arayüz genişliği
  doygunluğunun "belirgin olmadığını" yazıyor; Allen & Hayes (2017) 196 km
  sınırı (aşağıda) yine de korunuyor (o makale doygunluğu bulmuş).
  Hazır senaryolar elle boyutlu olduğu için doğrulama sonuçları değişmez.
- Ekstrapolasyon uyarısı (2026-11-25'ten beri tüm ortamlarda): otomatik
  boyutta Mw > 7.9 (WC94 alanı) arayüz ve levha içinde de uyarılır; önce
  yalnızca kabukta çıkıyordu (Bölüm 6 "sessiz ekstrapolasyon yok" ihlali).
- Genişlik = min(√A, sismojenik taban / sin δ); doyunca L = A / W.
  Sismojenik taban: 20 km (VARSAYIM; CLAUDE.md "15–25 km"). Hiposantr
  tabandan derinse taban hiposantr + 1 km'ye iner.
- Hiposantrın düzlemdeki konumu "İleri seviye" ayarı (varsayılan doğrultu
  ve eğim boyunca ortada). Ztor ≥ 0 ve taban kısıtı konumu kaydırabilir.
- Thingbaijam vd. (2017): Faz 3'te erişilen kopya "tüm haklar saklı, metin
  ve veri madenciliği / yapay zekâ yasak" notlu olduğu için kullanılmamıştı.
  2026-11-24: Bristol deposundaki yazar kabul kopyasında (core.ac.uk
  153373648, SHA256 a7baaf67…) böyle bir not YOK ("made available in
  accordance with publisher policies"); yalnızca Tablo 1'deki katsayılar
  (bilimsel olgu) künyeyle kullanıldı, metin kopyalanmadı.
- Levha içi, normal fay (2026-11-25, kullanıcı kararı "b"): otomatik
  boyutta W = min(√A_WC94, W_TMG17), L = min(A / W, L_TMG17); TMG17
  Tablo 1 "Normal-faulting": log10 L = −1.722 + 0.485·Mw, log10 W =
  −0.829 + 0.323·Mw (σ 0.128 ikisi de), veri Mw 5.86–8.39 (yazar kopyası
  s. 68, DOĞRULANDI). Verinin içinde levha içi ve dış yükselim normal
  olayları da var (makale s. 23). Önce genişlik sınırı yoktu: Mw 8.5, eğim
  40°, odak 50 km 182 × 182 km kare, düzlem 0–117 km (yüzeye çıkıyordu);
  şimdi 251 × 82 km. Mw 8.2: ~180 × 66 km (2017 Chiapas Mw 8.2 ~180 ×
  60 km, hafızadan, doğrula). Elle genişlik kırpılmaz.
- Çok parçalı kırılma (2026-11-25, çekirdek; `rupture.rs`): kullanıcı
  izi (en çok 12 parça) üst kenar izidir; parçalar üst kenarlarından
  birleşik dikdörtgenler (ortak genişlik ve Ztor, parçaya özel eğim
  isteğe bağlı). Tanımlar PEER 2013/03 (Ancheta vd.) Bölüm 2.3 ve 4.2:
  uzunluk ve alan toplam, genişlik en büyük, doğrultu uzunlukla ve eğim
  genişlikle ağırlıklı, Ztor en sığ; Rrup ve Rjb en yakın parçaya, Rx
  GC2 T'ye göre (Spudich & Chiou 2015, USGS OFR 2015-1028, Eş. 1–10, tek
  kesintisiz iz). Genişlik: elle verilmişse o, değilse tek düzlemdeki gibi
  √A_WC94, tabana kırpılmış (2026-11-25 düzeltme: önce A / L idi, Mw 7 ve
  326 km izde 3 km genişlik ve uyarısız sonuç verdi). İz uzunluğu
  kullanıcınındır: ekstrapolasyon uyarısı yerine alan oranı (L·W / A_WC94,
  ×4,1 dışı) denetlenir; büyüklüğe göre çok uzun ya da kısa iz uyarılır. Kırılma zamanı:
  hiposantrın iz boyunca konumu merkez üssünün GC2 U koordinatı; bir
  noktaya kırılma yolu = √(ΔU² + Δ(eğim boyunca)²). Tsunami: her parça aynı
  ortalama kaymayla ayrı Okada kaynağı, toplam. BASİTLEŞTİRME: dirseklerde
  derinde boşluk/örtüşme kalır (NGA-West2 yamukla birleştirdi, s. 68).
  Test: düz izi parçalara bölmek tek düzlemle aynı sonucu verir (0,05 km).
- Yönelim (directivity, 2026-11-26): CY14 Eş. 3.6–3.8 (PEER 2013/07
  s. 24) f_D = c8 f_R f_M exp(−c8a (M − c8b)²) ΔDPP, ln y_ref'e (zemin
  teriminden önce; pygmm ile aynı yer); katsayılar nshmp CY14.csv'den
  (c8, c8a, c8b). DPP: PEER 2013/09 Bölüm 6 (Eş. 6.1–6.12; doğrudan nokta,
  E-yolu, ĉ′, E-yolu ortalaması F̄S, %10 hiposantr kuralı, çok parçalı
  toplam), `directivity.rs`. ΔDPP: aynı Rrup'taki ortalamaya göre; ortalama
  2 km ızgara × 2 km Rrup diliminden (örnekleme biçimi raporda yok,
  VARSAYIM). Yalnız sığ kabuk ve Mw > 5,5; dört model ortalamasında yalnız
  CY14'te terim var (diğer üçünde yok). ÖNEMLİ: CY14 katsayılarına göre
  büyük depremde etki uzun periyotta: Mw 7,8'de c8·exp(...) PGV 0,026,
  SA(3 s) 0,14, SA(10 s) 0,22 (× ΔDPP ~ ±1,5); PGA ve kısa periyotta ~0.
  Şiddet (PGV) haritasında etki ±%1 mertebesinde, tepki spektrumunda uzun
  periyotta belirgin. Bölmeye karşı değişmez değil (Eş. 6.12 parça
  başına ĉ′ ve F̄S tabanı): düz izi 3 parçaya bölmek fay yanındaki sahada
  DPP'yi 0,9'a kadar değiştirdi (uzun periyotta ≤ ~%20); yayımlanmış
  yöntem olduğu için değiştirilmedi (BİLİNEN SINIR).
- Levha içi, ters ve doğrultu atımlı: WC94 alanı, genişlik sınırı yok
  (kaynak yok; TMG17 normal satırının verisi değil). Strasser vd. (2010)
  levha içi bağıntısı erişilemedi.
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

### 7.4 Dört NGA-West2 modelinin ortalaması (2026-11-24)

- Karar: kullanıcı (2026-11-24), gerekçe docs/dogrulama.md "Senaryo
  şiddet doğrulaması tamamlama" (Ridgecrest ve Noto ölçüt dışı; pygmm
  deneyinde ortalama |ln| 0,34 → 0,22).
- Kod: `gmpe/ask14.rs`, `bssa14.rs`, `cb14.rs`, `cy14.rs`, ortalama
  `gmpe/ngaw2.rs`; `shaking::Gmm::Crust`.
- Kaynaklar (her modülün başında ayrıntı): denklemler PEER raporları
  2013/04 (ASK), 2013/05 (BSSA), 2013/06 + düzeltme notu (CB), açık
  erişim, `data/raw/refs/ngaw2/` (depoda değil). Katsayılar USGS
  nshmp-lib (CC0; dergi sürümü). Dergi makaleleri görülmedi; raporla
  dergi arasındaki farklar pygmm ve OpenQuake'in ortak biçiminden
  alındı: ASK14 Sa1180, Japonya Vs30 ara değeri, Tayvan min(Vs30, V1);
  BSSA14 f4 yedi periyotta yazarların 2014-07-15 düzeltmesi (nshmp-lib
  eski değerde kalmış; bssa14.csv'de "DEĞİŞTİRİLDİ"). Her model pygmm
  0.9.1 ile < 0,01 ln (BSSA14 972, ASK14 1440, CB14 1080 senaryo).
- Bölge (`CrustRegion`, `region::country_near`): merkez üssünün admin1
  rasterındaki ülkesi; denizdeyse 100 km içindeki en yakın kara. Her
  model kendi bölgelerini ayırır: BSSA14 Δc3 Çin/Türkiye ve
  İtalya/Japonya; ASK14 Tayvan, Çin, Japonya; CB14 Δc20 Japonya/İtalya,
  Çin; CY14 Ek A (2026-11-27): Japonya/İtalya γ × γ_JP_IT (yalnız
  6 < M < 6,9, rapor öyle diyor), Çin γ × γ_WN, Japonya φ1_JP ve σ2_JP;
  pygmm ile 540 karşılaştırma < 0,01 ln. Etki: Kumamoto C +0,15 → +0,15,
  Noto −0,38 → −0,38 (ikisi de M ≥ 7, çarpan dışı; φ1_JP ¼ ağırlıkla).
- Ortalama ln uzayında eşit ağırlık (SEÇİM). σ: karışım varyansı
  σ² = Σw(σᵢ² + (μᵢ − μ̄)²), τ² aynı biçimde (modeller arası fark tüm
  sahalarda ortak olduğu için τ'ya eklenir; SEÇİM).
- Aralık: her model kendi aralığında; Vs30 her modelin aralığına çekilir
  (ASK14 > 1000 m/s'de 1000). Aralık dışı model ortalamadan düşer:
  BSSA14 normal Mw > 7, CB14 normal Mw > 7,5, ters Mw > 8, odak > 20 km
  ya da eğim < 15°. Ortak mesafe sınırı Rrup ≤ 300 km, Vs30 180–1500.
- Havza terimleri kapalı (Z1 / Z2.5 Vs30'dan modellerin kendi
  bağıntılarıyla; CB14'te Z2.5 ≤ 1 km için sığ tortul terimi ÇALIŞIR,
  Eş. 3.20, Z2.5 Vs30'dan Eş. 6.9 / 6.10).
- Performans (referans makine, Kahramanmaraş Mw 7,8): ince ızgara
  355 → 490 ms, hasar 268 → 290 ms (arka planda).

### 7.4 ASB14 ve kullanıcı ağırlıkları (mantık ağacı, 2026-11-27)

- Kullanıcı isteği (README "Eklenecekler": GMPE seçimi + mantık ağacı).
  VARSAYILAN DEĞİŞMEDİ: dört NGA-West2 eşit, ASB14 ağırlığı 0; kullanıcı
  sağ panelde "Yer hareketi modelleri" bölümünden ağırlıkları değiştirir
  (`Quake::gmm_weights`, `gmpe::ngaw2::Weights`). Hazır senaryo ya da
  gerçek deprem seçilince ağırlıklar korunur; rapor ağırlıkları yazar.
- Model: Akkar, Sandıkkaya & Bommer (2014), BEE 12, 359–387,
  doi:10.1007/s10518-013-9461-4; Rjb sürümü (`gmpe/asb14.rs`).
  Denklemler (1)–(3) kabul edilmiş metinden (Imperial Spiral, sha256
  9a2fe3a4…, `data/raw/refs/asb14/`, depoda değil). Katsayılar yazarların
  elektronik ekinden (62 periyot), pygmm 0.9.1 veri dosyası üzerinden;
  makalenin Tablo 3 ve 4.a'sıyla ortak 20 satırda birebir aynı
  (`scripts/gen_asb14.py` denetler). pygmm ile 630 senaryoda < 0,01 ln.
- Düzeltme notu (erratum, doi:10.1007/s10518-013-9508-6; kullanıcı
  indirdi, 2026-11-27 okundu): yalnızca basılı Eş. (2)'deki fazladan "+S"
  terimi silinmeli; elektronik ekin araçlarında hata yok. Uygulamamız
  zaten düzeltilmiş Eş. (2)'yi kullanıyor; değişiklik gerekmedi.
- Bileşen: makale GM_AR (kaydedildiği yönde geometrik ortalama) verir;
  Boore & Kishida (2017) ln(RotD50/GM_AR) "smooth" katsayılarıyla RotD50'ye
  çevrilir (USGS esi-shakelib, kamu malı; PGA +%1,3, PGV +%2,3). Oranın σ'sı
  karesel eklenir (ShakeMap kuralı).
- Aralık: Mw 4–8, odak ≤ 30 km, Rjb ≤ 200 km (yazarlar "dikkatle uzatılabilir"
  diyor; uzatılmadı), Vs30 150–1200 (bu aralığa çekilir), SA 0,01–4 s.
  Aralık dışında ASB14 ortalamadan düşer, kalan modellerle hesaplanır:
  200 km'de ve spektrumda 4 s'de ortalamada küçük bir sıçrama olur (SEÇİM,
  sessiz ekstrapolasyon yerine). Yalnız ASB14 seçiliyse bu sınırların
  ötesi "model dışı".
- Ağırlıklar o sahada geçerli modeller arasında normalleştirilir; σ ve τ
  karışım varyansıyla (yukarıdaki dört model bölümüyle aynı). ASB14'ün
  bölgesel terimi, havza, tavan bloğu ve yönelim terimi yok.
- Doğrulama: docs/dogrulama.md "ASB14 (Akkar vd. 2014) Türkiye
  depremlerinde".

### 7.4 Parker vd. (2020), dalma-batma (Faz 5, 2026-11-20)

- Künye: Parker, Stewart, Boore, Atkinson & Hassani (2020), PEER Report
  2020/03 (rapor metni okundu); dergi sürümü Earthquake Spectra 38(1),
  456-493 (2022) (görülmedi: Boore'un sitesinin sertifikası süresi dolmuş).
  Uygulama: `seismira-core/src/gmpe/psbah20.rs`.
- Katsayılar: USGS nshmp-lib `PSBAH20_interface.csv`, `PSBAH20_slab.csv`
  (CC0), main b808438b. PEER elektronik ekiyle (Tablo E1–E3,
  PSHAB_Electronic_Supplement.zip, sha256 c83bcfae…f521) sütun sütun
  karşılaştırıldı: kullanılan tüm küresel katsayılar aynı, TEK FARK arayüz
  c6'sı (Mc üstü büyüklük eğimi). PGV ve 0.2–0.75 s'de ek c6 > c4 veriyor
  (PGV: 1.844 / 1.336); nshmp-lib bu periyotlarda c6 = c4 kullanıyor (ilk
  sürümü 2021'den beri). Rapor 4.3.2: katsayılar "c6 ≤ c4" kısıtıyla
  bulundu ("ensures slowing of the magnitude-scaling for M > mc"); ekteki
  değerler bu kısıtı ihlal ediyor. OpenQuake (yalnızca değer bakıldı) eki
  izliyor. SEÇİM: nshmp-lib (kısıta uyan). Etki yalnızca arayüzde M > 7.9:
  M9'da PGV farkı ln 0.56 (~1.75 kat, ~0.4 JMA birimi). AÇIK KONU: dergi
  sürümünün tablosu ya da bir düzeltme notu görülünce kesinleşir.
- Seçimler: küresel model (Mc arayüz 7.9, levha içi 7.6; Tablo 4.1);
  epistemik dallar yok (orta dal); havza terimi yok; levha içi derinlik
  terimi (Eş. 4.6) odak derinliğiyle (model Zhyp ile kuruldu; nshmp-lib
  Ztor + 3.12 km kullanır).
- Geçerlilik (rapor 8.2.1): arayüz M 4.5–9.5, Rrup 20–1000 km, Zhyp ≤ 40
  km; levha içi M 4.5–8.5, Rrup 35–1000 km, Zhyp 0–200 km; Vs30 150–2000
  m/s; yalnızca yay önü (forearc) sahalar. Girdi denetimi buna göre
  güncellendi: arayüz derinliği 5–40 km (önce 5–60), levha içi M ≤ 8.5
  (önce 8.0); levha içinde 200 km'den derin odak uyarılır, dalgalar
  gösterilir, şiddet hesaplanmaz (önce 300 km "kaba tahmin" deniyordu,
  ama kaba tahmin üreten bir model yoktu).
- Kısa mesafe (Rrup < 20 / 35 km) BİLİNÇLİ OLARAK hesaplanır: verinin
  alt sınırıdır, fonksiyonel biçim (yakın kaynak doygunluğu h, Eş. 4.4)
  bu bölge için kurulmuştur; nshmp-lib ve pygmm de hesaplar. Büyük arayüz
  depremlerinde kıyı şehirleri bu mesafededir.
- Bölgesel uyarlama (2026-11-20): `psbah20::Region` (küresel, Alaska, Aleut,
  Cascadia, Orta Amerika K/G, Japonya Pasifik / Filipin, Güney Amerika K/G,
  Tayvan D/B). Bölgesel c0, a0, s1, s2 ekteki Tablo E1/E2'den
  (`scripts/gen_psbah20_regional.py` → `gmpe/psbah20_regional.csv`; betik
  ekin küresel / Alaska / Aleut / Cascadia değerlerinin nshmp-lib ile aynı
  olduğunu da denetler), Mc Tablo 4.1'den. Rapor önerileri: a0 Cascadia
  arayüzünde küresel; s2 Orta Amerika'da küresel; s1 yalnızca Japonya ve
  Tayvan'da ayrı; Aleut Adaları NGA-Sub'un Alaska bölgesinde olduğundan a0
  ve s2 Alaska'dan (nshmp-lib Aleut'u desteklemiyor). Orta Amerika arayüz
  Mc'si iki alt bölgede 7.4 (tablo dipnotu; metin çıkarımında "7.41"
  görünen, "7.4" ile dipnot işaretidir). Yeni Zelanda ve listede olmayan
  zonlar küresel.
- Bölgesel testler: pygmm ile küresel + Alaska + Cascadia 34 020
  karşılaştırma (fark ~1e-9, yuvarlama); OpenQuake 3.26.2 (yalnızca çıktı,
  `scripts/gen_psbah20_oq_ref.py`) ile 12 bölgenin hepsi, 9 660
  karşılaştırma, en büyük ln farkı 4.6e-4. Arayüzde M ≤ Mc (c6 farkı).
- Bölge SEÇİMİ (2026-11-20, Slab2 ile): merkez üssünün altındaki Slab2 zonu
  (`slab::Slab2::locate`; birden çok levha üst üsteyse levha yüzeyi odak
  derinliğine en yakın olan; levha içinde odak yüzeyin en çok 10 km
  üstünde olabilir) → `slab::parker_region`. Alt bölge sınırlarını Campbell
  (2020, Earthquake Spectra 36, doi:10.1177/8755293019899957, kapalı erişim)
  tanımlar; görülmedi. Kullanılan sınırlar VARSAYIM:
  - Alaska / Aleut: Unimak Geçidi, 164,5° B (doğusu Alaska).
  - Güney Amerika kuzey / güney: Arica dirseği, 18,3° G. Gerekçe: rapor
    (s. 31) güneyde 13, kuzeyde 2 arayüz depremi sayar; veri tabanı raporu
    (PEER 2020/02, Bölüm 4) depremlerin Şili'de yoğunlaştığını, Peru'dan
    kuzeyde seyrek olduğunu söyler (Peru: 2001 Arequipa, 2007 Pisco).
  - Orta Amerika kuzey / güney: Meksika–Guatemala sınırı, 92,2° B. İki alt
    bölgenin c0, a0, s1, s2'si aynı; yalnızca levha içi Mc (7,40 / 7,60)
    farklı. Puebla 2017 (Mw 7,1) iki Mc'nin de altında, etkisi yok.
  - Tayvan: Ryukyu zonunun 123° D batısı doğu, Manila zonunun 21° K kuzeyi
    batı alt bölgesi; katsayılar ve Mc ikisinde aynı (rapor s. 31).
  - Izu-Bonin → Japonya – Pasifik levhası (aynı dalan levha); Ryukyu-Nankai
    → Japonya – Filipin Denizi levhası.
  - Diğer zonlar (Yeni Zelanda, Hellen, Calabria, Mekran, Endonezya,
    Karayipler...) küresel.
  Duyarlılık: Japonya Pasifik ile küresel model arasında Mw 8, Sendai'de
  ~0,05 şiddet birimi fark (test `slab2_bolge_ve_yay_ardi`); uzak mesafede
  bölgesel a0 (sönümlenme) farkı büyür. Slab2 bulunamazsa ya da merkez
  üssünün altında levha yoksa küresel model; panel bunu söyler.
- Yay ardı (rapor 8.3.2): model yalnızca yay önü sahalar için; yay ardı
  kayıtları veriden çıkarılmış, Japonya'da yay ardında daha güçlü sönümlenme
  gözlenmiş. NGA-Sub yay çizgileri "gözle" çizilmiş ve dosya olarak
  yayımlanmamış (PEER 2020/02, Bölüm 4.8). Rapor bir yay ardı terimi
  (Eş. 8.2: yolun yay ardındaki kısmı için ek sönümlenme a_BA) yalnızca
  "planlıyor"; katsayısı YOK. Tōhoku denemesinde Rusya ve Sahalin'de
  Shindo 1–2 görüldü (kullanıcı ekran görüntüsü, 2026-11-20).
  UYGULAMA (2026-11-20): yay çizgisi = Slab2 levha yüzeyinin 105 km eş-
  derinlik eğrisi. Kaynak: Syracuse & Abers (2006), G³ 7, Q05017,
  doi:10.1029/2005GC001045, özet (makale metni görülmedi; Wiley 403):
  volkanların altındaki levha derinliği H, 500 km'lik yay parçalarında
  72–173 km, küresel ortalama 105 km. Saha, altındaki levha (levha dışında
  en yakın levha hücresi) 105 km'den derinse yay ardıdır
  (`slab::Slab2::is_backarc`). Zonun 3° genişletilmiş ızgarası dışındaki
  sahalar sınıflanmaz. Denetim (test `yay_onu_ve_ardi`): Sendai ve hendeğin
  okyanus tarafı yay önü; Akita, Vladivostok, Yujno-Sahalinsk yay ardı.
  KARAR: yay ardında şiddet YİNE hesaplanır (Parker'ın yay ardı katsayısı
  yok; başka bir modelin terimini eklemek melez model olur), şehir kartında
  "yay ardı: model yay önü verisiyle kuruldu; burada tahmin fazla olabilir"
  notu çıkar (Bölüm 6: sessiz ekstrapolasyon yok). Izgara hücreleri
  işaretlenmez. Tōhoku istasyonlarıyla ölçüldü (docs/dogrulama.md, Faz 5):
  Japon Denizi tarafında model ~%15–30 fazla, yani etki gerçek ama küçük
  (ilk metin "büyük olasılıkla fazla" idi, yumuşatıldı). Akita şehri:
  gözlenen PGV 12, tahmin 20 cm/s. Yay çizgisi Kuzeydoğu Japonya'da gerçek
  volkanik cepheden ~20–40 km batıda (cephenin altında levha 84–90 km).
- Simülasyona bağlama (Faz 5 adım 3, 2026-11-20): `shaking::Gmm` depreme göre
  modeli seçer (kabuk → CY14; arayüz → Parker arayüz; levha içi ≤ 200 km →
  Parker levha içi; daha derin → model yok). Mesafe sınırı ve Vs30 aralığı
  modelden gelir: dalma-batmada 1000 km ve 150–2000 m/s. Izgara alanı da
  1000 km'ye büyür: Mw 9 Tōhoku benzeri senaryoda ince ızgara 1648×1296
  (adım 2 hücre, ~1,9 km), referans makinede kaba 134 ms, ince 424 ms; en
  yüksek bellek ~300 MB (Bölüm 11 içinde).
- Sarsıntı süresi (D5-95), dalma-batma (2026-11-20): Bahrampouri,
  Rodriguez-Marek & Green (2021), Earthquake Spectra 37(2), 903–920,
  doi:10.1177/8755293020970971 (kullanıcının getirdiği yayımlanmış PDF;
  Eş. 7–12, Tablo 1–2 sayfa görüntüsünden doğrulandı). Arayüz ve levha içi
  ayrı katsayılar; KiK-net verisi, M 4–9, dalma-batma Rrup ≤ 500 km.
  δZ1 = 0 (Z1 bilinmiyor). Aralık dışı girdi uca çekilir (yalnızca "sarsıntı
  bitti" görseli). Kabukta Afshari & Stewart (2016) kalır.
  Tablo notu: levha içi m3 üç değer, etiketsiz; kabuk sütunundaki sırayla
  (RS 5,828 / SS 4,231 / NS 5,496) okundu. OpenQuake da aynı eşlemeyi
  kullanıyor (çıktı karşılaştırması). Metin (s. 908) normal faylarda sürenin
  daha kısa olduğunu söylüyor, ama tabloda NS'nin m3'ü RS'ninkine yakın ve
  SS'ninkinden büyük; tablo uygulandı (kabuk sütununda da aynı durum var).
  Etkisi: Mw 9 arayüz, 300 km, Vs30 400: ~108 s (AS16 uca çekilmiş haliyle ~60 s
  kalıyordu); Tōhoku kayıtlarında 100–200 s.
- Fay boyutu, dalma-batma arayüzü (2026-11-20): alan yine Wells &
  Coppersmith (1994) (doğrulanmış; M > 7,9 ekstrapolasyon uyarısı arayüzde
  2026-11-25'e kadar ÇIKMIYORDU, düzeltildi; uzunluk 2026-11-25'ten beri
  TMG17 ile sınırlı, Bölüm 7.1),
  ama genişlik artık arayüzün sismojenik alt sınırıyla doyuyor:
  `source::INTERFACE_BOTTOM_KM` = 50 km. Kaynak: PEER Report 2020/02
  (NGA-Sub veri tabanı), Bölüm 4.4, s. 65–66: Z_BC, "arayüz depremlerinin
  en büyük derinliğinde levha yüzeyinin ortalama derinliği", "50 km in ALK
  and CAM, 55 km in JPN and SAM" (Hayes vd. 2012; Tichelaar & Ruff 1993).
  Bölgeden bağımsız 50 km seçildi. Ek üst sınır (2026-11-20):
  `INTERFACE_MAX_WIDTH_KM` = 196 km, Allen & Hayes (2017) ÖZETİ (USGS
  Publications Warehouse 70189636 meta verisi; tam metin ücretli, görülmedi):
  "the down-dip rupture width for interface earthquakes saturates near 200 km
  (196 km on average)", yaklaşık Mw > 8,6. Sonuç (Tōhoku, eğim 14°, Mw 9,0):
  547 × 196 km (önce 328 × 328); gözlenen ~450–500 × 200 km. Çapraz denetim: nshmp-lib'de
  (CC0) Shaw (2023) arayüz alanı log A = M − 4 olarak kodlu (kullanılmayan
  yardımcı fonksiyon, makale görülmedi); Mw 9'da 100 000 km², WC94 ters
  çözümü 107 000 km² (%7 fark). Levha içinde genişlik sınırı yok (kaynak
  bulunamadı).
  Erişilemeyen birincil kaynaklar (hepsi betik indirmesine kapalı, 2026-11-20):
  Strasser vd. 2010, Allen & Hayes 2017, Shaw 2023 (SSA, kapalı);
  Thingbaijam vd. 2017 (Bristol AAM, 403); Murotani vd. 2008 (EPS, Springer
  JS doğrulaması), 2013 (GRL, Wiley 403; Zisin, J-STAGE bağlantıyı kesiyor).
  PDF gelirse dalma-batmaya özgü alan bağıntısıyla değiştirilecek.
- Test: pygmm 0.9.1 (nshmp-lib'i izler) ile 1620 senaryo × 7 ölçü = 11 340
  karşılaştırma, en büyük ln farkı 5e-15 (docs/dogrulama.md). pygmm'in
  katsayıları da nshmp-lib'den geldiği için bu test formül aktarımını
  doğrular; katsayıların kendisi yukarıdaki ek karşılaştırmasıyla.

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
  ile birlikte yeniden ele alınacak. DURUM (2026-11-20): t_tepe eklendi;
  kutu yine ilk S'te belirir ama son değere fiziksel tepe anında ulaşır,
  bu yüzden etki artık yalnızca kutunun ilk görünme anında. Erken uyarı
  süresi hâlâ ilk S'e göre (uyarının "sarsıntı başlamadan" ölçütüyle
  tutarlı); değiştirilmedi.
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
  (`seismira-core/src/mmi.rs`). Haritada MMI ölçeği seçimi Faz 10'a
  ERTELENDİ (2026-11-20): erken uyarı ve bölge mantığı JMA Shindo
  sınıflarına bağlı; haritayı iki ölçekle boyamak bütün sınıf / renk /
  bildirim hattını ikiye katlar. Kart MMI'yı zaten gösteriyor.
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
- (2026-11-24: aşağıdaki not geçersiz; süre JMA 験震時報 73'ten doğrulandı,
  7.3.) Erken uyarı gecikmesi (5 s) hâlâ VARSAYIM (2026-11-15): JMA sisteminin
  gözden geçirme makalesi (Kodera vd. 2021, Frontiers in Earth Science,
  doi:10.3389/feart.2021.726045) ilk rapor zamanını yalnızca deniz
  depremleri için ve oluş anına göre veriyor (S-net bölgesinde %50 için
  ~22 s); istasyonda algılamadan uyarıya geçen süre için yayımlanmış bir
  sayı bulunamadı.

### Tepki spektrumu, TBDY-2018 ve yapay ivme kaydı (2026-11-26)

- Senaryo spektrumu: `Scenario::spectrum`, depremin modeli (RotD50, %5).
- TBDY-2018 tasarım spektrumu: `tbdy.rs` (Resmî Gazete metni; ayrıntı
  modül notunda ve docs/arayuz.md).
- SDOF tepkisi: `sdof.rs`, Nigam & Jennings (1969) BSSA 59(2) tam çözümü
  (Eş. 6d–6e, 7; taranmış PDF'ten okundu). Test: rezonansta harmonik
  girdi Sa = √(1+4β²)/(2β)·A (%1 içinde), rijit uçta Sa ≈ PGA.
- Yapay kayıt: `synthetic.rs`, Gasparini & Vanmarcke (1976, SIMQKE)
  yaklaşımı; RAPOR OKUNAMADI (açık kopya yok), zarf ve yineleme kuralı bizim
  seçimimiz; doğruluk çıktıdan sınanır (TBDY ZC hedefinde 0,05–4 s en büyük
  spektrum hatası %5,1, ortalama %1,1; D5-95 16,4 s / hedef 15 s). Süre:
  noktada D5-95 medyanı (`Scenario::d595`). TBDY 2.5.1: gerçek kayıtlar
  öncelikli, benzeştirilmiş kayıt koşullu ve en az 11 → TASARIM İÇİN DEĞİL
  (arayüzde ve dosya başlığında). Taban düzeltmesi yalnızca ortalama.

### Toplu senaryo (2026-11-27; PSHA'ya ilk adım)

- Kullanıcı kararı (2026-11-27): önce fay üzerinde toplu senaryo, sonra
  ESHM20 ile Avrupa + Türkiye olasılıksal tehlike.
- `seismira_core::batch`: verilen depremin kırılma izi (çok parçalı iz ya
  da tek düzlemin üst kenarı) fay kabul edilir. Her büyüklükte kırılma
  uzunluğu otomatik (WC94 / TMG17, `Fault::from_quake`), kırılma fay
  boyunca eşit aralıklı konumlara kaydırılır (yüzen kırılma); uzunluk fayı
  aşarsa fayın tamamı (arayüz "*" ile belirtir). Merkez üssü kırılmanın
  ortası; derinlik, eğim, kayma, ortam ve model ağırlıkları verilen
  depremden. Parçaya özel eğimler alt izde kullanılmaz (ortak eğim).
- Birleştirme: her hücrede senaryoların ortalama ve en yüksek JMA şiddeti
  ve PGV'si; EŞİT ağırlık (SEÇİM). OLASILIK DEĞİL: oluş hızı ve
  büyüklük–sıklık dağılımı yok (ESHM20 aşaması). Bir senaryonun model
  sınırı dışındaki kara hücreleri o senaryoda şiddet 0 sayılır.
- Hesap: ortak alan (fayın tamamı en büyük Mw ile kırılsa), adım en çok
  1024 hücre/kenar, artımlı toplam (bellekte yalnız toplamlar). Kahramanmaraş
  hazır senaryosu, Mw 6,5–7,5 / 0,25 / 3 konum: 15 senaryo, 536 × 426 ızgara,
  2,3 s (referans makine).
