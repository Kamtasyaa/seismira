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
çoğu ülkede kamuya açık bir sistem yok):
- t_uyarı = P'nin merkez üssünden D uzaklıktaki sanal istasyona
  varışı + işlem gecikmesi. Varsayılan D = 25 km, gecikme = 5 s
  (varsayım; yayımlanmış erken uyarı performans çalışmalarından
  doğrula). İleri seviyede: yoğun ağ / seyrek ağ / sistem yok.
- Uyarı ölçütü (JMA ölçütü, doğrula): tahmini en yüksek Shindo 5- ve
  üzeriyse, Shindo 4 ve üzeri beklenen bölgelere uyarı verilir.
- Her şehir için iki değer gösterilir: "S − P" ve "erken uyarı
  süresi" (t_S − t_uyarı). Sıfır veya negatifse "kör bölge".

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


## Açık doğrulama notları

- (Faz 0 notu) 7.1'deki moment bağıntısı: M0 = 10^(1.5·Mw + 9.1) N·m,
  IASPEI standart biçimidir. Hanks & Kanamori (1979) özgün bağıntısı
  Mw = (2/3)·log10(M0 [dyn·cm]) − 10.7 olup N·m'de 10^(1.5·Mw + 9.05)
  verir (fark ~%12). Faz 3'te birincil kaynaktan doğrulanıp hangisinin
  kullanılacağı ve künyesi buraya yazılacak.
