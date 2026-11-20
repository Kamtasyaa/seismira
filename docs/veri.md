# Veri kaynakları, lisanslar, paketler

> CLAUDE.md v3’ten ayrıldı (Faz 0, 2026-11-14). Bölüm numaraları atıflar
> bozulmasın diye korunmuştur. "(doğrula)" işaretleri hâlâ geçerlidir.

## 12. Veri kaynakları, lisanslar, paketler

| Veri | Kaynak | Lisans |
|---|---|---|
| Kıyı, ülke sınırı, göl | Natural Earth | Kamu malı |
| İl / eyalet / bölge sınırları | Natural Earth Admin-1 | Kamu malı |
| Daha ayrıntılı idari sınırlar (isteğe bağlı) | geoBoundaries | Kontrol et. GADM KULLANMA (yeniden dağıtıma izin vermiyor, doğrula) |
| Japonya tahmin bölgeleri (isteğe bağlı) | JMA GIS verisi | Kontrol et |
| Şehirler, nüfus, çok dilli adlar | GeoNames | CC BY 4.0 |
| Seyahat süreleri | ak135, tablo bizim üretimimiz | Model yayımlanmış |
| Dalma-batma geometrisi | USGS Slab2 (Hayes 2018) | Kamu malı (DOĞRULANDI 2026-11-20) |
| Levha sınırları, tektonik bölgeleme | Bird (2003) PB2002 veya USGS STREC | Kontrol et |
| Vs30 | USGS küresel Vs30 | Kontrol et |
| Yükselti, eğim, CTI | Copernicus DEM GLO-90 veya SRTM | Kontrol et |
| Deniz derinliği | GEBCO | Kontrol et |
| Yağış | WorldClim (dağıtımı kısıtlı olabilir) veya CHELSA | Kontrol et |
| Yeraltı su seviyesi | Fan vd. (2013) | Kontrol et |
| Nehirler | HydroSHEDS / HydroRIVERS | Kontrol et |
| Litoloji | GLiM (Hartmann & Moosdorf 2012) | Kontrol et |
| Arazi örtüsü | GlobCover 2009 veya ESA WorldCover | Kontrol et |
| Avrupa + Türkiye bina stoku, kırılganlık, site modeli | ESRM20 (risk.EFEHR) | CC BY 4.0 |
| Diğer ülkeler bina stoku ve kırılganlık | GEM | CC BY-NC-SA 4.0, ticari YOK |
| İstanbul mahalle bazlı bina yaşı | İBB Açık Veri | İBB Açık Veri Lisansı |
| Japonya bina yapım dönemi | e-Stat | CC BY uyumlu |
| Japonya zemin ve heyelan | J-SHIS (NIED) | Kendi sözleşmesi, kontrol et |
| ABD bina envanteri | FEMA Hazus / USACE NSI | Ücretsiz; lisans metnini kontrol et |
| Meksika konut verisi | INEGI | Kaynak göstererek serbest |
| Yazı tipleri | Noto | SIL OFL |
| Yüksek çözünürlüklü kıyı ve detay | OpenStreetMap | ODbL |

Paketler:
- Temel paket (uygulamayla gelir; hedef ~500 MB altı, ölçerek
  doğrula): harita ve idari sınırlar, şehirler, seyahat süresi
  tablosu, ~1 km çözünürlükte küresel ızgaralar, alt örneklenmiş
  batimetri, ESRM20.
- GEM paketi (ayrı indirme): lisansı farklı olduğu için uygulamayla
  birlikte paketlenmez.
- Bölge paketleri (ayrı indirme): yüksek çözünürlüklü ızgaralar,
  kıyı ve ulusal veriler (Türkiye, Japonya, ABD, Meksika...).
- Barındırma önerisi: GitHub Releases veya Zenodo; checksum zorunlu.

Lisans kuralları:
- Her veri indirilmeden önce lisansı yeniden kontrol edilir.
- Türetilmiş veri kaynağının lisansını taşır; CC BY'da "değiştirildi"
  notu düşülür.
- CC BY-NC-SA ile ODbL verisi tek bir türetilmiş veri tabanında
  BİRLEŞTİRİLMEZ; ayrı dosyalarda kalır.
- Lisansı yeniden dağıtıma izin vermeyen veri kullanılmaz; alternatifi
  bulunur ya da kullanıcıya sorulur.

## Uygulama notları: veri hazırlama hattı

- Tek komut: `python3 scripts/prepare_data.py` (depo kökünden). Yazı
  tiplerini ve Natural Earth'ü indirip SHA256 ile doğrular, ardından
  `seismira-data natural-earth` ile `data/map/ne_{110m,50m,10m}.smap`
  dosyalarını üretir.
- Natural Earth sürümü: v5.1.2 (GitHub nvkelso/natural-earth-vector
  etiketi). Lisans 2026-11-14'te yeniden kontrol edildi: kamu malı, atıf
  isteğe bağlı ("Made with Natural Earth.").
- `.smap` biçimi: seismira-core/src/mapfile.rs. Koordinatlar Web Mercator
  birim karesinde f32; kara ve göller önceden üçgenlenmiş, sınırlar
  line strip. Boyutlar: 110m 0.2 MB, 50m 2.0 MB, 10m 17.9 MB.
- Natural Earth geometrisi ±180°'de zaten bölünmüş; araç meridyeni geçen
  kenarları sayar ve uyarır (Antarktika'nın kutup kapanış kenarı hariç).
- 10m Admin-1 sınır dosyasında boş (NullShape) kayıtlar var; atlanır.
- Çokgen delikleri dosyadaki sıraya göre değil, nokta-çokgen testiyle
  içinde bulundukları dış halkaya bağlanır. (Sıraya güvenince 10m karada
  Hazar Denizi kara olarak dolmuştu; 2026-11-14'te düzeltildi.) Sahipsiz
  delik kalırsa araç hata verir.

## Vs30: kaynak kararı (Faz 4, 2026-11-14)

- Kaynak: "The U.S. Geological Survey Global Vs30 Mosaic", USGS veri
  yayını, 2026, doi:10.5066/P1NV6UNM (Heath vd. 2020 sürümünün
  güncellemesi). Lisans: CC0 1.0 (USGS sayfasında doğrulandı,
  2026-11-14). Dosyalar: vs30_mosaic_median_30c.tif (medyan, m/s),
  vs30_mosaic_sd_30c.tif (log standart sapma), version.json.
  Çözünürlük 30″. Özel değerler: 600 okyanus, 601 buz, 602 buzul,
  603 göl.
- ScienceBase betik indirmelerini 403 ile engelliyor. Dosyalar bir kez
  tarayıcıdan indirilip `data/raw/vs30/` altına konur; betik SHA256
  ile doğrular, dosya yoksa indirme talimatını yazar.
- Uygulamaya giden çözünürlük: 30″ (kullanıcı kararı, 2026-11-14).
  Paket bütçesinin büyük kısmını alabilir; ileride DEM / eğim / litoloji
  eklenirken bütçe yeniden değerlendirilir.
- Yedek: apps.usgs.gov/shakemap_geodata/vs30/global_vs30.grd (2020
  tabanlı, 610 MB, betikle inebiliyor, SD katmanı yok). Kullanılmıyor.

## ESRM20 site modeli ve şehir zemini (2026-11-14)

- ESRM20 site modeli (gitlab.seismo.ethz.ch/efehr/esrm20_sitemodel, kod
  GPLv3, veri CC BY 4.0, GFZ Nextcloud) Vs30 olarak USGS küresel
  haritasının Avrupa dilimini (`europe_2020_vs30.hdf5`) kullanıyor; kendi
  büyütmesini eğim + jeolojik çağ ile veriyor (Vs30 değil). Yani ESRM20,
  Vs30'u kıyı ovalarında USGS'den daha iyi VERMEZ. Büyütme modeli Faz 6'da
  ayrıca değerlendirilecek.
- ESRM20'nin "exposure2site" aracı, kaba hücre / idari birim için saha
  değerini GHSL yapılaşma yoğunluğuyla ağırlıklandırıyor (yamaçtaki boş
  hücre yerine yerleşik ova). Gölcük örneği (GeoNames noktası yamaçta,
  466 m/s; kıyı ovası 230–340 m/s) bu sorunun tipik hâli.
- Karar: şehir zemini için GHSL GHS-BUILT-S R2023A, 2020, EPSG:4326 30″
  (JRC, CC BY 4.0, copyright.txt 2026-11-14 okundu). Yalnızca veri
  hazırlamada kullanılır; uygulamaya şehir başına hesaplanmış Vs30 girer.

- Uygulama (2026-11-14): `seismira-data city-sites data/cities/cities15000.scty
  data/vs30/vs30.sras data/raw/ghsl/GHS_BUILT_S_E2020_GLOBE_R2023A_4326_30ss_V1_0.tif
  data/cities/cities15000.ssit` (GHSL: `python3 scripts/fetch_ghsl.py`).
  Pencere 5×5 hücre, yapılaşma ağırlıklı medyan. Sonuç: 34 215 şehrin
  34 164'ü ağırlıklı; ağırlıklı/nokta oranı %10 0,82, medyan 1,00, %90 1,28.
  `.ssit` (411 KB) depoda, `.scty` gibi.
- Sınır (kullanıcıya bildirildi): Gölcük düzelmedi (466 → 447 m/s). GHSL'ye
  göre yapılaşma, haritanın 440–480 m/s verdiği hücrelerde yoğun; kıyı
  şeridi hücreleri çoğunlukla deniz. Sorun Vs30 vekilinin kendisinde
  (eğim yöntemi dar kıyı ovasını yamaçla karıştırıyor). Gerçek çözüm
  jeoloji tabanlı / ölçülmüş Vs30 (ör. Türkiye için ulusal Vs30
  çalışmaları; doğrula) ya da ESRM20 eğim+jeoloji büyütmesi (Faz 6).

## Veri hattı ve CI düzeltmeleri (2026-11-15)

- `scripts/fetch_geonames.py` eklendi; `prepare_data.py` artık eksikse
  şehir listesini (`geonames`) ve şehir zeminini (`city-sites`) de üretir,
  bölge dosyalarının üçünü de (.sras, .sreg, .sout) denetler. Denendi:
  sabitlenmiş 2026-11-14 anlık görüntüsünden üretilen `.scty` ve `.ssit`,
  depodakilerle bayt bayt aynı (SHA256).
- GeoNames dökümü her gün değişir; eski gün sunucudan alınamaz. Ham
  dosyalar kaybolursa betik bugünkü dökümü indirir ve farkı açıkça yazar
  (SOURCES.toml ve `.ssit` güncellenmeli).
- CI: önce test, sonra clippy (test düşse de çalışır); `-D warnings`
  yalnızca clippy adımında. Sebep: CI en yeni kararlı Rust'ı kullanıyor
  (1.100), yerel Arch paketi daha eski (1.99); yeni clippy uyarısı üç commit
  boyunca Windows ve macOS testlerinin hiç çalışmamasına yol açmıştı.

## Slab2 (Faz 5, 2026-11-20)

- Kaynak: Hayes, G. (2018), Slab2 - A Comprehensive Subduction Zone Geometry
  Model, USGS data release, doi:10.5066/F7PV6JNV. Meta veri (Slab2.xml):
  erişim ve kullanım kısıtı "none"; USGS verisi, kamu malı.
- İndirme: ScienceBase betikleri 403 ile engelliyor; kullanıcı ana kaydın
  "Attached Files" bölümünden `Slab2Distribute_Mar2018.tar.gz` (140 MB,
  SHA256 3f53a835…7b1d) dosyasını tarayıcıyla indirdi → `data/raw/slab2/`.
  Arşivde 27 zon, her zon için dep / dip / str / thk / unc ızgaraları
  (GMT NetCDF-4, 0,05° ya da 0,02°).
- İşleme: `.venv/bin/python scripts/gen_slab2.py` (numpy, scipy, h5py) →
  `data/slab2/slab2.sslb` (1,85 MB; yalnızca derinlik, u16 0,05 km, zon
  başına 3° genişletilmiş, levha dışı hücreler en yakın levha derinliğiyle
  dolu + bayrak; zstd). Eğim ve doğrultu uygulamada türetilir: Slab2'nin
  dip / str ızgaraları derinliğin merkezi farkıyla birebir aynı (`--check`:
  26 zonda fark < 0,01°, Sumatra-Java'da medyan 0,2° / 0,8°). Çıktı
  tekrarlanabilir (iki çalıştırma bayt bayt aynı).
- Depoda: `.sslb` ve `data/test/slab2_ref.csv` (324 düğümlük test
  referansı). Gerekçe CLAUDE.md Bölüm 5 istisnası: ham veri elle indirilir,
  testler dosyayı okur. Uygulama dosya yoksa dalma-batmada küresel modelle
  çalışır.

## ESRM20 bina stoku ve kırılganlık (Faz 6, 2026-11-20)

- Kaynak: efehr/esrm20 (EFEHR GitLab), commit b2368bee (2025-01-19), CC BY
  4.0 (LICENSE okundu). `scripts/fetch_esrm20.py` 181 dosyayı (44 ülke ×
  konut/ticari/sanayi + eşleme + kırılganlık XML) indirir, SHA256 listesini
  yazar (673 MB; Fransa konut dosyası Git LFS'te, `lfs=true` ile).
- Bina stoku idari birim düzeyinde; düzey ülkeye göre değişir (Türkiye il,
  Almanya / Fransa / İtalya ilçe ve altı, Andorra / Monaco ülke). Her kayıt
  koordinatıyla Natural Earth Admin-1 bölgesine atanır
  (`seismira-data exposure`); denizde kalan nokta 0,5° içinde aynı ülkenin
  en yakın bölgesine taşınır (1445 nokta). Hiç kara noktası olmayan Monaco
  ülke kısıtsız. Bölgesiz kalan: 372 bina (Meis / Kastellorizo; Yunan
  bölgesi 0,5° içinde yok).
- Sonuç `data/exposure/esrm20.sexp` (0,29 MB, depoda; üretim için 673 MB
  indirme gerekiyor): 1570 bölge, 511 kırılganlık tipi, 142,6 milyon bina.
  Türkiye denetimi: 81 ilin 80'i ESRM20 il toplamlarıyla birebir; Edirne'ye
  komşu ülkeden 17,6 bina düşüyor (sınır noktası). ESRM20 il adlarında yazım
  farkları var (Afyon, "Kinkkale", "Zinguldak"); eşleme koordinatla olduğu
  için etkisiz.
- Kırılganlık XML'i OpenQuake `logncdf` (doğrusal ortalama, standart
  sapma) → medyan, β. ESRM20 xlsx "Final" sayfasıyla denetlendi
  (CR_LDUAL-DUH_H1: 1,388 g, β 0,359). Dört sınır durumunun β'sı her tipte
  aynı (fark ≤ 0,01 denetimi). Şiddet ölçüleri: SA(0.6) 233, SA(1.0) 170,
  PGA 71, SA(0.3) 37 tip.

