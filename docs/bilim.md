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

### 7.9 Sıvılaşma ve heyelan

- Sıvılaşma: Zhu vd. (2017). Girdiler: PGV, Vs30, yağış, kıyıya
  uzaklık, nehre uzaklık, yeraltı su seviyesi (doğrula).
- Heyelan: Nowicki Jessee vd. (2018). Girdiler: PGV, eğim, litoloji,
  arazi örtüsü, CTI (doğrula).
- Modellerin kalibre edildiği çözünürlük ve veri setleri kaynaktan
  doğrulanır; farklı veri seti kullanılırsa belgelenir.
- Çıktı: hücre başına olasılık; haritada kırmızı tonlu kareler
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
  - Medyan sarsıntıyla hesaplanır (belirsizlik aralığı henüz yok; USGS
    default_stddev ve PGV σ ile analitik σ adım 5'ten sonra).

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
  Coppersmith (1994) (doğrulanmış; M > 7,9 ekstrapolasyon uyarısı sürüyor),
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
- Erken uyarı gecikmesi (5 s) hâlâ VARSAYIM (2026-11-15): JMA sisteminin
  gözden geçirme makalesi (Kodera vd. 2021, Frontiers in Earth Science,
  doi:10.3389/feart.2021.726045) ilk rapor zamanını yalnızca deniz
  depremleri için ve oluş anına göre veriyor (S-net bölgesinde %50 için
  ~22 s); istasyonda algılamadan uyarıya geçen süre için yayımlanmış bir
  sayı bulunamadı.
