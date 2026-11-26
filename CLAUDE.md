# Seismira: Deprem Simülatörü Proje Rehberi (v4)

## 0. Bu dosya ne?

- Projenin temel kararlarını içerir. Her oturumun başında oku. Bir karar
  değişirse ilgili dosyayı güncelle ve değişikliği kullanıcıya söyle.
- Ayrıntılar docs/ altındadır; ilgili işe başlamadan o belgeyi oku:
  - docs/bilim.md: bilimsel model (Bölüm 7)
  - docs/veri.md: veri kaynakları, lisanslar, paketler (Bölüm 12)
  - docs/dogrulama.md: doğrulama senaryoları, ölçütler, kayıtlar (Bölüm 14)
  - docs/arayuz.md: metin/diller, erişilebilirlik, girdiler ve geçerlilik
    kuralları, harita, bölge vurgusu, arayüz düzeni (Bölüm 3, 6, 8, 9, 10)
- "(doğrula)" ve "kontrol et" işaretleri bilinçlidir: o bilgi hafızadan
  yazıldı. Uygulamadan önce birincil kaynaktan doğrula, sonucu docs/
  altına not et. Doğrulayamadığın şeyi uydurma, kullanıcıya söyle.
- Kütüphane API'lerini ezberden yazma; güncel sürümü docs.rs ve
  crates.io üzerinden kontrol et.

## 1. Kullanıcı hakkında

- Profesyonel programcı değil ama teknik terminolojiye az çok hakim.
  Teknik terim kullanabilirsin; açıklamalar kısa ve net olsun.
- İletişim dili: Türkçe.
- İşletim sistemi: CachyOS (Arch tabanlı), varsayılan kabuk bash.
  Masaüstü: GNOME, Wayland.
- Donanım (2026-11-14 kontrol edildi): Intel Core i5-1135G7 (4 çekirdek /
  8 iş parçacığı), 16 GB RAM, dahili Intel Iris Xe (Vulkan: Mesa
  vulkan-intel). Harici ekran kartı YOK.
- Rust: Arch paketi (rustup yok), clippy + rustfmt kurulu.
- Bu makine referans hedeftir; uygulama burada akıcı çalışmalı.
- Kullanıcıda yalnızca Linux var. Windows ve macOS sürümlerini kendisi
  deneyemez; bu yüzden CI derlemeleri şart.
- Proje adı: **Seismira** (2026-11-14 seçildi). Paket ön eki `seismira-`,
  uygulama ikilisi `seismira`. Depo: GitHub, CI GitHub Actions.

## 2. Proje vizyonu

Dünya haritası üzerinde çalışan, eğitim ve farkındalık amaçlı,
bilimsel olarak ciddi bir deprem simülatörü. Tek senaryo, ama iyi
hesaplanmış ve canlı izlenen bir deprem. Kullanımı basit, içeriği
yüzeysel DEĞİL.

Kullanıcı bir deprem tanımlar ve "Başlat"a basar:
1. Merkez üssünde bir çarpı belirir.
2. P ve S dalgaları farklı renkte halkalar olarak yayılır.
3. Erken uyarı anında, güçlü sarsıntı beklenen il / eyalet / bölge
   sınırları yanıp sönmeye başlar; sol üste bildirim düşer.
4. P dalgası bir şehre ulaşınca şehirde küçük bir işaret çıkar.
5. S dalgası ulaşınca şehrin üstünde JMA Shindo kutusu belirir
   (0, 1, 2, 3, 4, 5-, 5+, 6-, 6+, 7). Bölge sınırı Shindo rengine
   döner ve sarsıntı sürerken yanıp söner.
6. Sarsıntı bitince sıvılaşma ve heyelan riskli alanlar içi boş, kalın
   kırmızı kareler (~25 km) olarak belirir, yumuşak yanıp söner; köşede
   tür harfi (S / H), bölgede ilk karede bildirim (2026-11-23 kullanıcı
   kararı, ayrıntı docs/arayuz.md).
7. Tsunami tehlikesi varsa kıyılar sarı / kırmızı / mor renklenir,
   uyarı yürürlükteyken yanıp söner, varış süreleri gösterilir.
8. Simülasyon bitince, kullanıcı isterse fay kırılma izi gösterilir.
   Varsayılan: kapalı.

Fay kırılmasının ilerleyişi ANİMASYON OLARAK GÖSTERİLMEZ. Hesapta
sonlu fay modeli kullanılır (mesafe ve zamanlama için şart).

Kapsam dışı: can kaybı tahmini (belirsizliği çok yüksek ve hassas bir
konu; kullanıcı isterse ileride eklenir).

Uygulamada açık bir uyarı olmalı: "Bu bir eğitim simülasyonudur.
Gerçek deprem bilgisi için AFAD, Kandilli, JMA veya USGS gibi resmi
kurumları takip edin." Alt çubukta durur; ✕ ile kapatılabilir ama her
açılışta yeniden görünür ve Görünüm menüsünden geri açılır.

## 3. Platform, metin, erişilebilirlik (ayrıntı: docs/arayuz.md)

- Windows, Linux ve macOS'ta BİREBİR AYNI görünüm ve davranış.
- Temel veri uygulamayla gelir; uygulama internetsiz çalışır.
- Tema: koyu, haritayı öne çıkaran, modern ve sade.
- Tüm metin UTF-8, her yazı sistemi doğru görünmeli; Noto yazı tipleri
  gömülü. Koda gömülü arayüz metni YOK (çeviri altyapısı Faz 1'den).
- Yanıp sönme ≤ ~1 Hz, yumuşak (istisna: tsunami kıyıları tam açık/kapalı,
  2026-11-25 kullanıcı kararı); "Hareketi azalt" ayarı; renk tek bilgi
  taşıyıcı olmaz.

## 4. Teknoloji kararları

- Dil: Rust (stable).
- Arayüz: egui + eframe, çizim arka ucu wgpu (Vulkan / DirectX 12 /
  Metal). Eski donanım için OpenGL yedeği wgpu'nun kendi GL arka ucuyla
  sağlanır (WGPU_BACKEND=opengl). eframe'in ayrı glow çizicisi
  KULLANILMAZ: harita, geometriyi GPU'da tutan wgpu paint callback ile
  çizilir ve glow bunu desteklemez (2026-11-14 kararı).
- Paralel hesap: rayon. İptal: AtomicBool ile işbirlikçi iptal.
- Büyük veri okuma: memmap2.
- Coğrafi hesap: saf Rust (ör. geo, geographiclib-rs). PROJ ve GDAL
  UYGULAMAYA girmez; yalnızca veri hazırlama aracında olabilir.
- Harita: Natural Earth vektör verisi, kendimiz çiziyoruz.
- Google Maps ve Apple Maps KULLANILMAZ. openstreetmap.org döşeme
  sunucularından ASLA doğrudan çekilmez (kuralları dağıtılan
  uygulamalar için yasaklıyor).

## 5. Proje yapısı (Cargo workspace)

- crates/seismira-core: simülasyon motoru. Arayüze HİÇBİR bağımlılığı
  yok. (Paket adı `core` OLAMAZ; Rust'ın yerleşik kütüphanesiyle
  çakışır.)
- crates/seismira-app: egui arayüzü. Yalnızca çekirdeği çağırır ve çizer.
- crates/seismira-data: komut satırı aracı. Ham veriyi (shapefile,
  GeoTIFF, CSV) uygulamanın hızlı okuduğu kompakt ikili biçime
  çevirir. Tek seferlik üretimler için yardımcı Python betiği
  kabul edilir (ör. seyahat süresi tablosu).
- data/: data/SOURCES.toml (her veri setinin kaynağı, lisansı, atıf
  metni, "değiştirildi" notu; "Veri Kaynakları" sayfası bundan üretilir)
  ve işlenmiş veriler. Ham (data/raw/) ve büyük üretilmiş (data/map/,
  data/vs30/) veri depoya GİRMEZ: kaynaklar sürüm + SHA256 ile sabitlenir,
  tek komutla yeniden üretilir: `python3 scripts/prepare_data.py`.
  İstisna (fiili durum, 2026-11-15'te yazıya geçirildi): küçük işlenmiş
  dosyalar depoda DURUR (data/traveltime, data/cities, data/regions,
  data/test, data/validation, data/scenarios; ~18 MB), çünkü üretimleri ağır kurulum
  (ObsPy) ya da elle indirilen Vs30 ister ve testler bunları okur.
  Silinirse cities ve regions aynı betikle, traveltime ve test verileri
  kendi betikleriyle (scripts/gen_*.py) yeniden üretilir. Bu dosyalar
  gereksiz yere yeniden üretilip commit'lenmez (her seferinde depo
  ~15 MB büyür).
- assets/: uygulamayla gelen dosyalar (yazı tipleri). CJK yazı tipleri
  depoda değil, aynı betikle indirilir.
- docs/: bilim, veri, doğrulama belgeleri.

## 6–10. Girdiler, harita, bölge vurgusu, arayüz (docs/arayuz.md)

- Sağ panel: konfigürasyon + "Başlat". Sol panel: YALNIZCA bildirim
  akışı (şehir listesi yok, 2026-11-14 kullanıcı kararı). Alt: zaman
  çizgisi + kapatılabilir eğitim uyarısı. Orta: harita (Web Mercator).
- Kullanıcı haritayı serbestçe gezer ve depremi istediği yerden başlatır
  (haritaya tıklayarak). Şehir seçerek konum belirleme YOK.
- Geçerlilik aralığı dışındaki girdi engellenir ya da açıkça uyarılır;
  sessiz ekstrapolasyon YOK.
- Bölge durumları simülasyon zamanının saf fonksiyonudur.

## 7. Bilimsel model (docs/bilim.md)

- GENEL KURAL: hiçbir katsayıyı ezbere yazma. Her formül yayımlanmış
  kaynağından alınır, kod yorumunda künyesi yazılır, makaledeki örnek
  değerlerle birim testi yazılır.
- Referans uygulamalar yalnızca sonuç karşılaştırmak içindir;
  OpenQuake (AGPL) kodu KOPYALANMAZ.
- Özet: sonlu fay (tek dikdörtgen düzlem), ak135 seyahat süreleri
  (tek model), sığ kabukta dört NGA-West2 modelinin (ASK14, BSSA14,
  CB14, CY14) bölgesel eşit ağırlıklı ortalaması (2026-11-24 kullanıcı
  kararı; varsayılan bu, kullanıcı panelden ağırlıkları değiştirip
  Akkar vd. 2014'ü ekleyebilir, 2026-11-27), dalma-batmada Parker vd. (2020), PGV →
  JMA Shindo, analitik belirsizlik (Monte Carlo yok), ESRM20/GEM
  kırılganlık, Zhu vd. (2017) sıvılaşma, Nowicki Jessee vd. (2018)
  heyelan, Okada (1985) + fast marching tsunami.

## 11. Performans kuralları

- Arayüz iş parçacığı ASLA bloklanmaz. Hedef: referans makinede
  sabit 60 FPS.
- Ağır hesap arka planda; sonuçlar kanal üzerinden gelir.
- Kademeli sonuç: önce kaba ızgara, sonra incelen. Kaynağa yakın
  ince, uzakta kaba (çok çözünürlüklü).
- Parametre değişince eski hesap anında iptal edilir.
- Şehir ve bölge olay zamanları başta tek seferde hesaplanır.
- Hiçbir şey animasyon yapmıyorsa yeniden çizim yapılmaz.
- Veri topyekün belleğe alınmaz; yalnızca ilgili bölge okunur.
- Sayılar f32. Depolamada u8/u16 kuantalama. f16 stable Rust'ta
  yoksa `half` crate'i.
- Bellek hedefi: tipik kullanımda 2–3 GB'ı geçmesin.
- CJK glifleri ihtiyaç anında yüklenir.

## 12. Veri ve lisanslar (docs/veri.md)

- Her veri indirilmeden önce lisansı yeniden kontrol edilir; kaynak,
  lisans, atıf ve "değiştirildi" notu data/SOURCES.toml'a yazılır.
- GADM KULLANILMAZ. GEM verisi (CC BY-NC-SA) ayrı paket; ODbL verisiyle
  tek veri tabanında birleştirilmez.
- Temel paket hedefi ~750 MB altı (2026-11-23 kullanıcı kararı: heyelan
  katmanı için 500 MB'tan yükseltildi; v0.1.0 veri paketi 699 MB).

## 13. Lisans

- Kod: GPL-3.0-or-later.
- Veriler kendi lisanslarıyla ayrı durur.
- GEM verisiyle paketlenen sürüm satılamaz. Bağış sorun değil.

## 14. Doğrulama (docs/dogrulama.md)

- Hazır senaryolar = doğrulama depremleri (Kahramanmaraş, İzmit,
  Kumamoto, Ridgecrest, Tōhoku, Noto, Maule, Puebla).
- Sapmalar docs/dogrulama.md'ye yazılır ve kullanıcıya bildirilir.

## 15. Yol haritası

Her fazın sonunda: derle, çalıştır, kullanıcıya ekranda ne görmesi
gerektiğini söyle, onay al, commit at.

- Faz 0: Kurulum. Rust, git, workspace, fmt / clippy / test.
  Donanım kontrolü. Proje adı. Bu dosyayı docs/'a böl. Kullanıcıya
  GitHub (veya Codeberg) deposu isteyip istemediğini sor; varsa
  Linux + Windows + macOS için CI derlemesi kur.
- Faz 1: Pencere, koyu tema, harita (Natural Earth 3 ayrıntı düzeyi,
  ülke ve Admin-1 sınırları), kaydırma, yakınlaştırma, yatay tekrar,
  180° meridyeni. Merkez üssü koyma. Yazı tipi zinciri ve çok dilli
  metin testi. Çeviri altyapısı (TR/EN).
- Faz 2: Sağ panel ve geçerlilik kuralları. ak135 tablosu. Jeodezik
  P/S halkaları. Zaman çizgisi (tüm hızlar). Bildirim akışı.
- Faz 3: Kaynak geometrisi ve mesafe ölçüleri. CY14. Shindo
  dönüşümü (sabit referans zemin). Şehirler, Shindo kutuları, ölçek
  açıklaması. İlk doğrulama (bir kabuk depremi).
- Faz 4: Vs30. Kademeli sarsıntı ızgarası. Bölge kimliği rasterı.
  Erken uyarı modeli. Bölge sınırı vurgusu ve yanıp sönme.
  Şehir ve bölge kartları. MMI. Belirsizlik aralığı.
- Faz 5: Dalma-batma modelleri. Slab2 ile ortam, doğrultu ve eğim
  önerisi. Alt fay zamanlaması (t_tepe). İsteğe bağlı fay izi.
  Doğrulama (Tōhoku, Puebla).
- Faz 6: Bina hasarı: Türkiye ve Avrupa (ESRM20 + İBB).
- Faz 7: Sıvılaşma ve heyelan (önce eksik veri katmanları).
- Faz 8: Tsunami aşama 1 ve 2, kıyı vurgusu.
- Faz 9: Japonya, ABD, Meksika verileri; GEM paketi.
- Faz 10: Kalite ayarları, hazır senaryolar, erişilebilirlik
  kontrolü, veri kaynakları sayfası, üç platform için paketleme.
  Kod imzalama kararı kullanıcıya sorulur (imzasız uygulama Windows
  ve macOS'ta güvenlik uyarısı verir; imzalama ücretlidir).
- Faz 11 (isteğe bağlı): yüksek çözünürlüklü kıyı ve detay paketleri,
  GPU tsunami simülasyonu, kararlı kıta içi model, model ortalaması.

## 16. Çalışma kuralları

- Küçük adımlarla ilerle. Bir adım çalışmadan sonrakine geçme.
- Büyük karar (kütüphane, mimari, kapsam) öncesi kullanıcıya sor.
- Hata çıkarsa önce kendin derleyip çalıştırarak bul.
- Çekirdek crate'e asla arayüz kodu girmez.
- Her bilimsel formül için kaynak + birim testi zorunlu.
- Her faz sonunda kısa bir "ne yaptık, sırada ne var" özeti ver.
