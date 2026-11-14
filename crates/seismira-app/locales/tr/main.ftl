app-title = Seismira
disclaimer = Bu bir eğitim simülasyonudur. Gerçek deprem bilgisi için AFAD, Kandilli, JMA veya USGS gibi resmi kurumları takip edin.

disclaimer-close = Uyarıyı kapat

menu-view = Görünüm
menu-disclaimer = Eğitim uyarısı
menu-text-test = Yazı testi
menu-language = Dil

panel-settings = Deprem ayarları
button-start = Başlat

panel-notifications = Bildirimler
notifications-empty = Henüz bildirim yok.

timeline = Zaman çizgisi

text-test-title = Çok dilli yazı testi
text-test-note = Sağdan sola yazılarda (Arapça, İbranice) çok kelimeli metinlerin kelime sırası şu an hatalıdır; bilinen sorun, ayrıntı docs/arayuz.md.

map-loading = Harita yükleniyor…
map-missing = Harita verisi bulunamadı. Depo kökünde şu komutu çalıştırın:
epicenter = Merkez üssü
epicenter-hint = Haritaya tıklayarak merkez üssünü seçin. Sürükleyerek kaydırın, tekerlekle yakınlaştırın.

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
ranges-note = Geçerlilik aralıkları geçicidir; yer hareketi modelleri eklendikçe kesinleşecek.
start-needs-epicenter = Başlatmak için önce merkez üssünü seçin.

issue-mw = Bu ortam ve fay tipi için büyüklük { $min } – { $max } aralığında olmalı.
issue-depth = Bu ortam için odak derinliği { $min } – { $max } km aralığında olmalı.
issue-interface = Dalma-batma arayüzünde yalnızca ters fay olabilir.
issue-deep = Derin odak (300 km'den derin): yer hareketi modelleri geçerli değil; şiddet sonuçları kaba tahmin olacak.
issue-geometry = Geçersiz değer (eğim 0–90°, enlem −90–90° olmalı).

# Zaman çizgisi
play = Oynat (Boşluk)
pause = Duraklat (Boşluk)
restart = Başa sar
next-event = Sonraki olaya atla (deprem anı, dalgaların yüzeye çıkışı, 100°'ye ulaşması…)
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
city-p-arrived = P dalgası ulaştı: { $city }
city-shindo = { $city }: Shindo { $shindo }
region-shindo = { $region }: Shindo { $shindo } (en yüksek: { $city })
tip-shindo = Shindo { $s } (aletsel şiddet { $i }; olası aralık { $lo } – { $hi })
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
no-gmpe = Bu tektonik ortam için yer hareketi modeli henüz yok (Faz 5); yalnızca dalgalar gösterilir.
no-gmpe-short = Bu ortam için şiddet hesabı henüz yok.
advanced = İleri seviye
hypo-along = Odak, doğrultu boyunca
hypo-down = Odak, eğim boyunca
rupture-size = Kırılma: { $l } × { $w } km, üst kenar { $ztor } km derinlikte
model-limit = Tahmin sınırı: faya 300 km
