app-title = Seismira
disclaimer = Bu bir eğitim simülasyonudur. Gerçek deprem bilgisi için AFAD, Kandilli, JMA veya USGS gibi resmi kurumları takip edin.

disclaimer-close = Uyarıyı kapat

menu-view = Görünüm
menu-disclaimer = Eğitim uyarısı
menu-shake-grid = Sarsıntı alanı
menu-fault-trace = Simülasyon sonunda fay izi
menu-liquefaction = Sıvılaşma riski (sarsıntı bitince)
menu-landslide = Heyelan riski (sarsıntı bitince)
menu-tsunami = Tsunami uyarıları
menu-reduce-motion = Hareketi azalt
menu-quality = Kalite
quality-low = Düşük (en çok 512 hücre, hızlı)
quality-medium = Orta (en çok 1024 hücre)
quality-high = Yüksek (en çok 2048 hücre, varsayılan)
quality-ultra = Ultra (2048 hücre, hasar tam çözünürlükte)
quality-note = Sarsıntı ızgarasının ve hasar hesabının çözünürlüğü. Hesap arka planda yapılır; açık simülasyon yeniden hesaplanır.
menu-text-test = Yazı testi
menu-language = Dil

panel-settings = Deprem ayarları
button-start = Başlat

panel-notifications = Bildirimler
notifications-empty = Henüz bildirim yok.

text-test-title = Çok dilli yazı testi
text-test-note = Sağdan sola yazılarda (Arapça, İbranice) çok kelimeli metinlerin kelime sırası şu an hatalıdır; bilinen sorun, ayrıntı docs/arayuz.md.

map-loading = Harita yükleniyor…
map-missing = Harita verisi bulunamadı. Depo kökünde şu komutu çalıştırın:

# Sayı ve koordinat biçimi
decimal-separator = ,
hemi-n = K
hemi-s = G
hemi-e = D
hemi-w = B

# Deprem ayarları
setting = Tektonik ortam
setting-crust = Sığ kabuk
setting-interface = Dalma-batma arayüzü
setting-intraslab = Levha içi
fault = Fay tipi
fault-ss = Doğrultu atımlı
fault-normal = Normal
fault-reverse = Ters
mw = Büyüklük (Mw)
depth = Odak derinliği (km)
lat = Enlem
lon = Boylam
strike = Doğrultu (°)
dip = Eğim (°)
rake = Kayma açısı (°)
fault-geometry = Fay geometrisi
ranges-note = Geçerlilik aralıkları yer hareketi modellerinin yayımlanmış aralıklarından gelir (kabuk: CY14, dalma-batma: Parker vd. 2020).
start-needs-epicenter = Başlatmak için önce merkez üssünü seçin.

issue-mw = Bu ortam ve fay tipi için büyüklük { $min } – { $max } aralığında olmalı.
issue-depth = Bu ortam için odak derinliği { $min } – { $max } km aralığında olmalı.
issue-interface = Dalma-batma arayüzünde yalnızca ters fay olabilir.
issue-deep = Odak { $max } km'den derin: yer hareketi modeli bu derinlikte geçerli değil. Dalgalar gösterilir, şiddet hesaplanmaz.
issue-geometry = Geçersiz değer (eğim 0–90°, enlem −90–90° olmalı).
issue-ztor = Kırılmanın üst kenarı { $ztor } km derinlikte; yer hareketi modeli en fazla { $max } km için geçerli. Odak derinliğini azaltın ya da büyüklüğü artırın.
issue-rupture-size = Kırılma boyutu bağıntısı Mw { $max } değerine kadar veriyle destekleniyor; daha büyük depremde fay uzunluğu ve genişliği kaba tahmindir.
issue-rupture-invalid = Kırılma uzunluğu 1–{ $l } km, genişliği 1–{ $w } km olmalı.
issue-rupture-clamped = Kırılma genişliği sismojenik tabana sığmıyor; { $w } km'ye kırpıldı.
issue-rupture-area = Kırılma alanı bu büyüklük için olağan değerin { $r } katı (Wells ve Coppersmith 1994, ±2,5σ dışında); sonuçlar beklenenden farklı olabilir.

# Zaman çizgisi
play = Oynat (Boşluk)
pause = Duraklat (Boşluk)
restart = Başa sar
next-event = Sonraki bildirime atla (erken uyarı, bölge Shindo değerleri, P dalgasının şehirlere ulaşması…)
step-back = 10 saniye geri (←; Shift+← 1 dakika)
step-forward = 10 saniye ileri (→; Shift+→ 1 dakika)
speed = Hız
time-elapsed = T+{ $t }

# Dalgalar ve olaylar
wave-p = P dalgası
wave-s = S dalgası
event-origin = Deprem: M{ $mw }, { $depth } km derinlikte
event-surfaced-p = P dalgası yüzeye ulaştı
event-surfaced-s = S dalgası yüzeye ulaştı
event-limit-p = P dalgası { $deg }°'ye ulaştı; ötesi P gölge bölgesi
event-limit-s = S dalgası { $deg }°'ye ulaştı; simülasyon sonu
traveltime-missing = Seyahat süresi tablosu bulunamadı (data/traveltime/ak135.ttt).

# Şiddet ve şehirler (Faz 3)
shindo-legend = Shindo (JMA)
percent = %{ $v }
city-p-arrived = P dalgası ulaştı: { $city }
city-shindo = { $city }: Shindo { $shindo }
region-shindo = { $region }: Shindo { $shindo } (en yüksek: { $city })
tip-shindo = Shindo { $s } (aletsel şiddet { $i }; olası aralık { $lo } – { $hi })
tip-mmi = MMI { $roman } ({ $v }; olası aralık { $lo } – { $hi })
tip-pga = PGA: %{ $v } g
tip-pgv = PGV: { $v } cm/s
tip-arrivals = P: T+{ $p }, S: T+{ $s } (S − P: { $sp } s)
tip-distance = Faya uzaklık: { $d } km
tip-outside-model = Bu mesafede yer hareketi modeli geçerli değil.
tip-vs30-map = Zemin: Vs30 ≈ { $v } m/s (USGS haritası)
tip-vs30-nearby = Zemin: Vs30 ≈ { $v } m/s (en yakın kara hücresi)
tip-vs30-built = Zemin: Vs30 ≈ { $v } m/s (USGS haritası, yerleşim ağırlıklı)
tip-vs30-reference = Zemin: referans Vs30 = { $v } m/s (harita değeri yok)
tip-vs30-clamped = Hesapta model sınırına çekildi: { $v } m/s
tip-vs30-sd = Zemin belirsizliği: Vs30 ×/÷ { $f } → şiddet ±{ $d }
no-gmpe = Bu derinlikte yer hareketi modeli yok; yalnızca dalgalar gösterilir.
no-gmpe-short = Bu deprem için şiddet hesabı yok.
advanced = İleri seviye
hypo-along = Odak, doğrultu boyunca
hypo-down = Odak, eğim boyunca
rupture-size = Kırılma: { $l } × { $w } km, üst kenar { $ztor } km derinlikte
rupture-manual = Kırılma boyutunu elle gir
rupture-length = Kırılma uzunluğu
rupture-width = Kırılma genişliği
model-limit = Tahmin sınırı: faya { $km } km

# Erken uyarı (Faz 4)
early-warning = Erken uyarı (varsayımsal sistem): { $regions }
early-warning-more = { $regions } ve { $n } bölge daha
tip-lead-time = Erken uyarı süresi: { $s } s
tip-blind-zone = Kör bölge: uyarı sarsıntıdan sonra geliyor

# Bölge kartı ve simülasyonu kapatma (Faz 4)
button-stop = Simülasyonu kapat
running-hint = Simülasyon açıkken haritada bir ile tıklayınca bölge kartı açılır. Yeni merkez üssü seçmek için önce simülasyonu kapatın.
region-max = En yüksek: Shindo { $s }
region-max-city = En yüksek: Shindo { $s } ({ $city })
region-typical = Yaygın: Shindo { $s } (yerleşim alanının en az %10'u)
region-arrivals = İlk P: T+{ $p }, ilk S: T+{ $s }
region-lead = Erken uyarı: bölgedeki ilk S'ten { $s } s önce
region-liquefaction = Sıvılaşma: ~{ $km2 } km² etkilenen alan (olası { $lo }–{ $hi })
region-landslide = Heyelan: ~{ $km2 } km² etkilenen alan (olası { $lo }–{ $hi })
region-tsunami = Tsunami: { $cat }, { $h }, ilk dalga { $eta }
ground-letter-liquefaction = S
ground-letter-landslide = H
ground-name-liquefaction = Sıvılaşma riski
ground-name-landslide = Heyelan riski
ground-legend-note = Kare: ~{ $km } km'lik alan; USGS sarı uyarı eşiği
ground-notice = [{ $letter }] { $name }: { $region }
ground-notice-unnamed = [{ $letter }] { $name }
region-blind = Erken uyarı: kör bölge (uyarı ilk S'ten sonra geliyor)
region-not-covered = Erken uyarı kapsamı dışında (Shindo 4'ün altı)

## Slab2 levha geometrisi ve Parker vd. (2020) bölgesi (Faz 5)
slab-none = Slab2'ye göre bu noktanın altında dalma-batma levhası yok; burada dalma-batma depremi beklenmeyebilir (Slab2 her zonu kapsamaz, ör. Kıbrıs yayı). Hesap küresel katsayılarla, yay önü / yay ardı ayrımı yapılmadan yapılır.
slab-missing = Slab2 verisi bulunamadı; dalma-batma modeli küresel katsayılarla hesaplanır.
slab-info = Levha ({ $zone }, Slab2): yüzey { $d } km, doğrultu { $strike }°, eğim { $dip }°
slab-region = Parker vd. (2020) bölgesi: { $region }
slab-apply = Levha geometrisini uygula
slab-too-deep = Levha burada { $d } km derinde; arayüz depremi en çok { $max } km derinlikte olabilir.
slab-above = Odak, levha yüzeyinin ({ $d } km) üstünde; levha içi deprem levhanın içinde olmalı.
tip-backarc = Yay ardı: model yay önü verisiyle kuruldu; burada tahmin fazla olabilir.
parker-region-global = küresel
parker-region-alaska = Alaska
parker-region-aleutian = Aleut Adaları
parker-region-cascadia = Cascadia
parker-region-cam_n = Orta Amerika ve Meksika (kuzey)
parker-region-cam_s = Orta Amerika ve Meksika (güney)
parker-region-japan_pac = Japonya, Pasifik levhası
parker-region-japan_phi = Japonya, Filipin Denizi levhası
parker-region-sa_n = Güney Amerika (kuzey)
parker-region-sa_s = Güney Amerika (güney)
parker-region-taiwan_e = Tayvan (kuzeydoğu)
parker-region-taiwan_w = Tayvan (güneybatı)
slab-zone-alu = Alaska-Aleut
slab-zone-cal = Calabria
slab-zone-cam = Orta Amerika
slab-zone-car = Karayipler
slab-zone-cas = Cascadia
slab-zone-cot = Cotabato
slab-zone-hal = Halmahera
slab-zone-hel = Helen Yayı
slab-zone-him = Himalaya
slab-zone-hin = Hindukuş
slab-zone-izu = Izu-Bonin
slab-zone-ker = Kermadec-Tonga
slab-zone-kur = Kamçatka-Kuril-Japonya
slab-zone-mak = Mekran
slab-zone-man = Manila Hendeği
slab-zone-mue = Muertos Çukuru
slab-zone-pam = Pamir
slab-zone-phi = Filipinler
slab-zone-png = Yeni Gine
slab-zone-puy = Puysegur
slab-zone-ryu = Ryukyu-Nankai
slab-zone-sam = Güney Amerika
slab-zone-sco = Scotia Denizi
slab-zone-sol = Solomon Adaları
slab-zone-sul = Sulawesi
slab-zone-sum = Sumatra-Cava
slab-zone-van = Vanuatu

## Tahmini bina hasarı (Faz 6, ESRM20)
region-damage-head = Tahmini bina hasarı (ESRM20, ilde { $n } bina):
region-damage-heavy = Ağır hasar veya göçme: ~{ $n } bina (%{ $p }; olası %{ $lo }–{ $hi })
region-damage-heavy-none = Ağır hasar veya göçme: beklenmiyor (1 binadan az)
region-damage-moderate = Orta hasar: ~{ $n } bina (%{ $p })
region-damage-nodata = Bu bölge için bina verisi yok.
region-damage-nodata-nogem = Bu bölge için bina verisi yok (ESRM20 yalnızca Avrupa ve Türkiye; dünyanın geri kalanı için GEM paketi kurulu değil).
region-loss-head = Tahmini yapısal hasar (GEM, bölgedeki binaların değer ağırlıklı ortalaması):
region-loss-ratio = Ortalama hasar oranı: %{ $p } (olası %{ $lo }–{ $hi })
region-loss-none = Ortalama hasar oranı: ihmal edilebilir
tip-damage = Tahmini ağır hasar veya göçme: %{ $p } (olası %{ $lo }–{ $hi })
tip-damage-note = (ilin bina dağılımıyla; şehrin kendi bina verisi değil)

# Tsunami (Faz 8)
tsunami-cat-major = Büyük tsunami uyarısı
tsunami-cat-warning = Tsunami uyarısı
tsunami-cat-advisory = Tsunami dikkat bildirimi
tsunami-issued = { $cat }: { $regions }
tsunami-heights = Tahmini en yüksek tırmanma (kaba tahmin): { $list }
tsunami-info-small = Tsunami bilgisi: deprem tsunami için küçük, tehdit beklenmiyor (PTWC ölçütü)
tsunami-info-deep = Tsunami bilgisi: deprem çok derin (≥ 100 km), tehdit beklenmiyor (PTWC ölçütü)
tsunami-info-inland = Tsunami bilgisi: deprem karada, tehdit beklenmiyor (PTWC ölçütü)
tsunami-arrived = Tsunami ulaştı: { $region } { $h }
tsunami-height-suffix = (~{ $m } m)
tsunami-legend-title = Tsunami (JMA kategorileri, depremden ~3 dk sonra)
tsunami-legend-major = Büyük tsunami uyarısı: > 3 m
tsunami-legend-warning = Tsunami uyarısı: 1–3 m
tsunami-legend-advisory = Dikkat: 0,2–1 m ya da tehdit bölgesi (yükseklik yok)
tsunami-legend-note = Yükseklikler kaba tahmindir (yakın alan tırmanması, Riquelme vd. 2015)
tsunami-tip = { $place }: { $cat }, { $h }, ilk dalga { $eta }
tsunami-no-height = yükseklik tahmin edilmedi
tsunami-height = tahmini ~{ $m } m
tsunami-unnamed = Kıyı

# Hazır senaryolar (Faz 10)
scenario = Hazır senaryo
scenario-custom = Özel (elle)
scenario-note = USGS olay kaydı ve ShakeMap kırılma modelinden tek düzlemli kaba eşdeğer; gerçek kırılma birden çok parçalı olabilir. Değerleri değiştirirsen senaryo "Özel" olur.
scenario-kahramanmaras2023 = 2023 Kahramanmaraş (Pazarcık), Mw 7,8 — yalnızca ilk ana şok
scenario-izmit1999 = 1999 İzmit (Gölcük), Mw 7,6
scenario-kumamoto2016 = 2016 Kumamoto (Japonya), Mw 7,0
scenario-ridgecrest2019 = 2019 Ridgecrest (ABD), Mw 7,1
scenario-tohoku2011 = 2011 Tōhoku (Japonya), Mw 9,1
scenario-noto2024 = 2024 Noto Yarımadası (Japonya), Mw 7,5
scenario-maule2010 = 2010 Maule (Şili), Mw 8,8
scenario-puebla2017 = 2017 Puebla (Meksika), Mw 7,1
