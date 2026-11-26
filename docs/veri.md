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
| Deniz derinliği | NOAA NCEI ETOPO 2022, 60″ (okyanusta GEBCO tabanlı) | CC0-1.0 (NCEI ISO üst verisi, 2026-11-23) |
| Yağış | CHELSA V2.1 klimatolojileri (bio12) | CC0 1.0 (EnviDat, DOĞRULANDI 2026-11-22) |
| Yeraltı su seviyesi | Fan vd. (2017) WTD, USC thredds | "Yalnızca araştırma amaçlı serbest": dağıtım için yazardan İZİN İSTENİYOR (2026-11-22) |
| Nehirler | GMTED2010'dan kendi ürettiğimiz ağ + Natural Earth 10m nehir orta hatları (HydroSHEDS KULLANILMAZ) | Kamu malı türevi (karar 2026-11-22) |
| Litoloji | GLiM (Hartmann & Moosdorf 2012), vektör | Vektör lisansı bulunamadı, izin istendi: yalnızca yerel (2026-11-22) |
| Arazi örtüsü | ESA CCI Land Cover 2009 v2.0.7 (CEDA) | "Any user for any purpose" + atıf (CEDA koşul dosyası, 2026-11-22) |
| Avrupa + Türkiye bina stoku, kırılganlık, site modeli | ESRM20 (risk.EFEHR) | CC BY 4.0 |
| Diğer ülkeler bina stoku ve zafiyet | GEM Global Exposure Model v2026.0.0 (Adm1 özetleri) + Global Vulnerability Model (yapısal), GitHub, commit sabitli | CC BY-NC-SA 4.0, ticari YOK (LICENSE.txt, 2026-11-23); ayrı paket `data/gem/`, depoda değil |
| İstanbul mahalle bazlı bina yaşı | İBB Açık Veri | İBB Açık Veri Lisansı |
| Japonya bina yapım dönemi | e-Stat | CC BY uyumlu |
| Japonya zemin ve heyelan | J-SHIS (NIED) | Kendi sözleşmesi (J-SHIS利用規約); işlenmiş ürün dağıtılabilir, atıf şart (2026-11-23) |
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
  birlikte paketlenmez. Üretim: `python3 scripts/prepare_data.py --gem`
  (scripts/fetch_gem.py ~125 MB ham (yapısal + yapısal olmayan), `seismira-data gem` → data/gem/gem.svul,
  8 MB). Bölge kimlikleri Natural Earth'ten (kamu malı); ODbL verisiyle
  birleştirilmez. Uygulama dosyayı bulursa Avrupa dışı bölge kartlarında
  ortalama hasar oranı gösterir, bulamazsa "GEM paketi kurulu değil" der.
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

## J-SHIS (Japonya zemini), Faz 9 kararı (2026-11-23)

- USGS Vs30 mozaiği Japonya için ZATEN J-SHIS kullanıyor: usgs/
  earthquake-global_vs30 deposu, `Japan/Makefile` (J-SHIS
  `Z-V2-JAPAN-AMP-VS400_M250` şekil dosyası, 250 m, 128–149° D,
  31–46° K; README: "The map of Japan is downloaded from
  http://www.j-shis.bosai.go.jp/"). Bu yüzden J-SHIS ayrıca eklenmedi.
  Daha yeni V3 (若松・松岡 2013) ile fark 30″ çözünürlükte küçük olur;
  gerekirse Faz 11.
- Lisans: J-SHIS利用規約 (https://www.j-shis.bosai.go.jp/agreement), açık
  lisans DEĞİL. 第5条1項: veri düzenlenip işlenebilir, ürün serbestçe
  dağıtılabilir; 第5条2項: verinin olduğu gibi (biçim dönüştürülmüş dahil)
  kopyasının dağıtımı yasak; ürün SATILACAKSA j-shis@bosai.go.jp'ye
  başvurulur. 第4条: atıf (表層地盤データ記述ファイル規約'ndaki kaynaklar).
  Bizim vs30.sras'ımız mozaikten yeniden örneklenmiş, kuantalanmış bir
  üründür (işlenmiş); atıf data/SOURCES.toml'a eklendi.
- Kumamoto şiddet doğrulaması (docs/dogrulama.md Faz 9) Vs30 kaynaklı
  sistematik sapma göstermedi (C − B = −0,02 ΔI).

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


## Faz 7 girdileri: lisans kontrolü (2026-11-22)

- CHELSA V2.1 klimatolojileri: EnviDat paketi `chelsa-climatologies`,
  lisans CC0 1.0 (API `package_show`, 2026-11-22). Zhu 2017 WorldClim
  (1950–2000) yağışıyla kalibre edildi; CHELSA farklı bir ürün, sapma
  doğrulamada (adım 5) ölçülecek.
- HydroSHEDS v1 / HydroRIVERS: lisans (TechDoc v1.4 Ek A) ticari kullanım
  dahil serbest AMA dağıtım yalnızca "en az bu lisans kadar koruyucu" bir
  son kullanıcı sözleşmesiyle, tek başına ürün olarak dağıtım yasak, tersine
  mühendislik yasağı ve WWF'ye karşı tazmin yükümlülüğü var. GPL-3.0 açık
  kaynak dağıtımla uyumsuz sayıldı. KARAR (kullanıcı, 2026-11-22): nehir ağı
  GMTED2010 (kamu malı) yükseklik verisinden akış birikimiyle kendimiz
  üretilir. HydroSHEDS da SRTM'den aynı yöntemle üretildiği için yaklaşım
  benzer; eşik ve sapma belgelenir, USGS Ground Failure ile karşılaştırılır.
  Aynı akış birikimi heyelan modelinin CTI girdisi için de kullanılır
  (HYDRO1k CTI'nin USGS adresi 404).
- Fan vd. yeraltı su seviyesi: `http://thredds-gfnl.usc.es/thredds/catalog/
  GLOBALWTDFTP/annualmeans/` (https çalışmıyor), 5 kıta NetCDF, ~340 MB.
  Dosyadaki sürüm Fan vd. (2017, PNAS 114(40), 10572–10577) "annual mean";
  Zhu 2017'nin kalibre edildiği Fan, Li & Miguez-Macho (2013, Science 339,
  940–943) değil. Global öznitelik: "These data can be used freely for
  research purposes provided that the following source is acknowledged".
  Yeniden dağıtım izni yok. KARAR (kullanıcı, 2026-11-22): veri sorumlusu
  Gonzalo Miguez-Macho'dan (USC) e-postayla izin istenir; yanıt gelene kadar
  veri yalnızca yerelde geliştirme/doğrulama için kullanılır, pakete girmez.
  İzin gelmezse: Zhu 2017 kıyı modeli (wtd istemez) ya da başka WTD verisi.
- Kıyıya uzaklık: kendimiz hesaplarız (USGS NASA Ocean Color ürününü
  kullanmış; taşınmış). Uygulamada okyanus maskesi olarak Vs30 mozaiğinin
  okyanus kodu kullanıldı (Natural Earth yerine; uygulamadaki tüm 30″
  katmanlarla aynı kıyı, 2026-11-22).

## Faz 7 adım 2: girdi katmanları (2026-11-22)

- GMTED2010 indirme: `Grid_ZipFiles/md30_grd.zip` (239 MB) ve
  `md75_grd.zip` (2,9 GB), ESRI ArcInfo ikili ızgara. GeoTIFF döşemeleri
  (`Global_tiles_GMTED/...`) de var ama 7,5″'te ~5 kat büyük; dizin
  listeleri 403, tek tek dosya adresleri çalışıyor. GDAL kullanmamak için
  saf Rust AIG okuyucu yazıldı (crates/seismira-data/src/aig.rs).
  Doğrulama (`seismira-data aig-check`): md30, USGS GeoTIFF döşemeleri
  30N030E (Türkiye–İran) ve 30S060W (G. Amerika) ile hücre hücre aynı
  (8 640 000 / 8 640 000; kuzeyde tek fark aşağıdaki CCITT bloğu).
  CCITT 1 bit bloklar (yalnızca RMin / RMin + 1 değerleri) için çözücü
  yazılmadı, RMin ile doldurulur (hata ≤ 1 m): md30'da 177 450 bloğun 56'sı.
  Dosya adları (döşeme satırı k, kuzeyden): 0 `w001001`, 1 `w001000`,
  k ≥ 2 `z001{k−1}` (md30'da 2, md75'te 33 dosya).
  md75 doğrulaması: 7,5″ GeoTIFF 30N030E ile 138 186 913 hücre aynı, 53 087
  farklı (%0,04), en büyük fark 1 m, hepsi CCITT bloklarında.
  GMTED köşesi 1/7200° (0,5″, ~15 m) kaymış (−180,000139°); Vs30 ızgarasıyla
  aynı sayılır.
  GMTED'de okyanus 0 m (nodata değil); su maskesi Vs30'dan.
- CHELSA bio12 ölçeği: ham değer × 0,1 = mm/yıl. Örnek noktalarla
  doğrulandı: İstanbul 773, Rize 2287, Paris 659, Londra 654 mm. Ham değer
  u16'da 65535'te doyuyor (Cherrapunji 6553,5 mm; gerçek ~11 000); Zhu
  2017 2500 mm'de kırptığı için etkisiz.
- CTI tanımı DOĞRULANDI: USGS "HYDRO1k Documentation ReadMe" (PDF,
  usgs.gov/media/files/hydro-1k-readme), Bölüm 3.2.5: CTI = ln(FA /
  tan(slope)), düz alanda slope = 0,001; Bölüm 3.2.3: FA yukarı havza
  alanı, 1 km hücrede km², tepelerde 0 (hücrenin kendisi hariç). Hafızadaki
  "(FA + 1) × piksel alanı (m²)" tanımı YANLIŞTI, kullanılmadı.
- Lisans kontrolü, litoloji ve arazi örtüsü (2026-11-22, sayfalar okundu):
  - GLiM 0,5° raster (PANGAEA 788537): CC BY 3.0 (açıkça yazıyor). Vektör
    sürüm: Hamburg sayfası ve PANGAEA'da lisans YAZMIYOR; CCGM üzerinden
    sipariş/Dropbox. Jessee 2018 vektörü 250 m'ye rasterlamış; 0,5° (~55 km)
    raster heyelan için çok kaba. Vektör için yazarlardan (Hartmann /
    Moosdorf) yazılı izin gerekebilir: KULLANICIYA SORULACAK.
  - ESA CCI Land Cover (1992–2015 v2.0.7) / C3S (2016+ v2.1.1): sayfalarda
    sürüme bağlı tek bir lisans yok. CDS üç etiket listeliyor ("ESA CCI
    licence", "CC-BY licence", "VITO licence") eşleme vermeden; ESA CCI
    koşulları "any user for any purpose" + atıf diyor, ama aynı sayfadaki
    "Copyrights" paragrafı yeniden dağıtımı ve türev ürünü yasaklıyor (portal
    içeriği için mi, veri için mi belirsiz). Belirsiz: KULLANICIYA SORULACAK.
  - ESA WorldCover 2021 v200 (10 m): CC BY 4.0 (esa-worldcover.org/en/data-access,
    "free of charge, without restriction of use"). Atıf: "© ESA WorldCover
    project 2021 / Contains modified Copernicus Sentinel data (2021)
    processed by ESA WorldCover consortium"; Zanaga vd. 2022,
    doi:10.5281/zenodo.7254221. Sınıfları (11) GlobCover LCCS sınıflarından
    farklı: Jessee katsayıları için sınıf eşlemesi gerekir (yaklaşım
    belgelenmeli, doğrulamada sapma ölçülmeli).
- Fan WTD yerel dönüşüm (2026-11-22; İZİN BEKLENİYOR, yalnızca yerel):
  5 kıta NetCDF4 (toplam 343 MB, SHA256 scripts/gen_fan_wtd.py'de),
  `.venv/bin/python -I scripts/gen_fan_wtd.py` + `seismira-data wtd` →
  data/ground/wtd.sras (169 MB, AYRI dosya; diğer girdilerle karışmaz).
  Bulgular: (1) derinlik dosyada NEGATİF saklı (−1000…0 m), pozitife
  çevrildi; (2) enlem/boylam float32'de birikimli hesaplanmış (son
  değerlerde 0,09 hücre kayma), dizinler ilk değer + sabit adımla
  hesaplandı; ızgara Vs30 ızgarasıyla hücre merkezinde hizalı;
  (3) Vs30'da kara olup WTD'si olmayan 2,28 milyon hücre (adalar, kıyı
  şeridi): genel model uygulanamaz, kıyı modeline düşülür.
  Sağduyu: Adapazarı 3,4 m, İstanbul kıyısı 0,1 m, Sahra 61 m, Los
  Angeles 12 m.
- Üretim: `seismira-data ground-inputs` → data/ground/inputs30.sras
  (depoda değil). Yöntem ve varsayımlar: ground_inputs.rs modül belgesi
  ve docs/bilim.md 7.9.
- Uygulamanın okuduğu tek zemin dosyası: data/ground/liq_static.sras
  (`seismira-data liq-static`, 8 bit, ~136 MB; depoda değil). Dağıtılacak
  sürüm `--coastal` ile (wtd'siz) üretilir; ayrıntı docs/bilim.md 7.9
  "paket boyutu".
- GLiM: PANGAEA 788537 (CC BY 3.0) yalnızca 0,5° raster ve dosya tanıtıcısı
  bozuk ("handle not found"). Jessee 2018 vektör GLiM'i 250 m'ye
  rasterlamış. Vektör sürüm Hamburg Üniversitesi sayfasında CCGM üzerinden
  dağıtılıyor (Dropbox, `LiMW_GIS 2015.gdb.zip`, 1,14 GB); lisansı henüz
  GÖRÜLMEDİ.
- GLiM yerel dönüşüm (2026-11-22; LİSANS GÖRÜLMEDİ, izin istendi, yalnızca
  yerel): `LiMW_GIS 2015.gdb.zip` (SHA256 scripts/gen_glim.py'de) açılır,
  `.venv/bin/python -I scripts/gen_glim.py` (çokgen merkezli rasterleme,
  30″, parça parça okur: tek seferde okumak 16 GB RAM'i aşıp OOM verdi) +
  `seismira-data lith` → data/ground/lith.sras (10 MB, AYRI dosya, kod =
  `jessee2018::GLIM_CLASSES` sırası). Vs30'da kara olup GLiM'i olmayan
  1,80 milyon hücre (kıyı şeridi farkı, karanın ~%0,8'i). Sağduyu:
  Adapazarı su, Kapadokya py, Fuji/Kilauea vb, İstanbul sm, Everest ig.
- ESA CCI Land Cover 2009 v2.0.7 (2026-11-22): CEDA arşivinden
  (dap.ceda.ac.uk/neodc/esacci/land_cover/), 311 791 297 B, SHA256
  SOURCES.toml'da. Koşullar `esacci_landcover_terms_and_conditions.txt`
  (aynı arşiv): "may be used by any user for any purpose", atıf şartı,
  garanti yok. Daha önce belirsiz bulunan "Copyrights" paragrafı CDS
  portalının sayfasındaydı; veri dosyasının kendi koşul metni bunu içermiyor.
  Yıl 2009 seçildi: Jessee 2018 GlobCover 2009 ile kalibre edilmiş.
  `seismira-data land-cover` → data/ground/landcover.sras (35 MB):
  her 30″ hücreye tam 3 × 3 CCI pikseli düşer, çoğunluk sınıfı alınır.
  `tiff` 0.11.3 bu dosyada da "no lzw end code" hatası veriyor;
  toleranslı LZW yedeği u8'e genişletildi ve 300 rastgele noktada rasterio
  ile karşılaştırıldı (298 aynı, 2'si Vs30'da su). Vs30'da kara olup CCI
  kara sınıfı olmayan 1,63 milyon hücre (kıyı şeridi).
  GlobCover 2009 asıl dosyası kullanılamadı: ESA DUE sunucusu kapalı,
  yalnızca Google Earth Engine'de var; koşulları da daha dar ("eğitim
  ve/veya bilimsel amaç"). Sınıf eşlemesi docs/bilim.md 7.9.
- GlobCover 2009: ESA DUE sitesi yanıt vermiyor (2026-11-22 de). Alternatif:
  aynı LCCS sınıf sistemini kullanan ESA CCI Land Cover (lisansı kontrol et).

## Tsunami: derinlik ve kıyı noktaları (Faz 8 adım 1, 2026-11-23)

- GEBCO_2026 (kamu malı, gebco.net "Terms of use" 2026-11-23 okundu) asıl
  adaydı; tüm indirme bağlantıları (BODC dahil) CEDA'ya gidiyor ve CEDA
  2026-11-23'te TLS bağlantısını reddetti (birkaç saat önce ESA CCI aynı
  sunucudan inmişti). Yerine NOAA NCEI ETOPO 2022 60″ yüzey yüksekliği
  (CC0-1.0; okyanus derinliği GEBCO ve diğer kaynaklardan). 1′ tsunami
  varış süresi hesabı için yeterli. scripts/fetch_etopo.py (SHA256).
- Deniz maskesi (scripts/gen_bathy.py): Natural Earth 10m kara ve göl DEĞİL;
  ETOPO > 10 m olan "deniz" hücreleri kara sayılır (2,4 M hücre, çoğu
  Antarktika buz sahanlığı: "surface" ürünü buz yüzeyini verir), 0–10 m
  olanlar 1 m. Bağlı bileşenlerden yalnızca dünya okyanusu (Marmara dahil)
  ve Karadeniz (Azak dahil; İstanbul Boğazı 1′'de kapalı) tutulur; Hazar
  (NE'de kara da göl de değil), Lut Gölü ve kopan küçük lagünler maskelenir.
- `seismira-data bathy` → data/tsunami/bathy.sras (25 MB; log 1–11 000 m,
  251 kod). Sağduyu: Marmara 1270 m, Karadeniz ortası 2220 m, Mariana
  çukuru 10 598 m, Japonya çukuru 7304 m, Hazar ve Lut maskeli.
- `seismira-data coast-points` → data/tsunami/coast.bin (1,9 MB): denize
  bakan 830 248 hücreden ~10 km arayla 96 661 nokta, her birinde derinlik ve
  en yakın şehir (cities15000). Türkiye 787, Japonya 2128 nokta.


## Dağıtım paketi (Faz 10 adım 7, 2026-11-23)

- Kod imzalama YOK (kullanıcı kararı: ücretli). İlk açılış uyarısı için
  packaging/KURULUM.txt (TR/EN): Windows "Yine de çalıştır", macOS
  Gizlilik ve Güvenlik → "Yine de Aç" ya da `xattr -dr com.apple.quarantine`.
- Veri paketi: `python3 scripts/package_data.py <sürüm>` →
  dist/seismira-data-<sürüm>.tar.zst. Açık liste (betikte, her dosyanın
  SOURCES.toml kimliğiyle). v0.1.0: 18 dosya, 711 MB, arşiv 699 MB, sha256
  fb026734…20d3d2 (hedef ~750 MB, CLAUDE.md Bölüm 12). Heyelan dahil:
  litolojisi GLiM 0,5° (CC BY 3.0, scripts/gen_glim05.py) olan kopya,
  `ls-static` ile pakette yeniden üretilir (129 s). Sıvılaşma KIYI modeli
  (`liq-static --coastal`, Fan WTD izni bekleniyor). Girmeyenler: vektör
  GLiM ve Fan WTD türevleri, GEM (ayrı, CC BY-NC-SA), doğrulama / test /
  ham veri.
- Sürüm iş akışı: .github/workflows/release.yml ("v*" etiketi ya da elle).
  İkililer: linux-x86_64 (ubuntu-22.04, glibc 2.35), windows-x86_64,
  macos-arm64, macos-x86_64. Paketleme işi veri paketini `data-v0.1.0`
  sürümünün ekinden indirir, SHA256 + MANIFEST ile doğrular, assets
  (fetch_fonts.py ile CJK dahil) + LICENSE + KURULUM.txt ekler; Linux
  tar.gz, diğerleri zip; TASLAK sürüm oluşturur.
- BEKLEYEN (kullanıcı): veri paketinin `data-v0.1.0` sürümüne yüklenmesi
  (336 MB, herkese açık depo) ve ilk sürüm etiketi.

## USGS STREC kararlı kıta içi bölgeler (2026-11-25)

- Kaynak: github.com/usgs/strec (arşivlenmiş, 2026-09-15), commit
  6e13af7f76d52adfeefbd74dbe647705e92db7d0, `strec/data/stable.geojson`
  (140 549 B, SHA256 a1b11b71…; tek MultiPolygon, 12 çokgen, 26 halka,
  3 465 nokta). Lisans: depodaki license.txt "public domain because it
  contains materials that originally came from the United States
  Geological Survey" (2026-11-25 okundu). README bölgelendirmenin Garcia
  vd. çalışmasına dayandığını söylüyor, künye vermiyor (doğrula).
- İndirme: `python3 scripts/fetch_strec.py` (SHA256); dönüşüm
  `seismira-data stable-regions` → data/regions/stable.sout (17 KB,
  depoda). Sürüm iş akışı bu dosyayı v0.1.0 veri paketinin üstüne ekler.

## Gerçek depremler: USGS ve EMSC (2026-11-26, kullanıcı kararı)

- Kullanıcı ureq 3.5.2 (MIT/Apache-2.0, rustls) ve üç kaynağı seçti; resmi
  sayfalardan koşullar okundu:
  - USGS: "USGS-authored or produced data and information are considered
    to be in the U.S. Public Domain"; atıf rica: "Credit: U.S. Geological
    Survey". fdsnws/event/1 sayfası: otomatik uygulamalar gerçek zamanlı
    GeoJSON akışlarını kullanmalı → `summary/4.5_month.geojson`; moment
    tensörü olay `detail` GeoJSON'undan. Hız sınırı yazılı değil.
  - EMSC: seismicportal.eu/terms.html: veri hizmetlerinden indirilen
    veriler CC BY 4.0; atıf "Credit: EMSC-CSEM SeismicPortal,
    https://www.seismicportal.eu". Site / veritabanının toplu kopyası
    izne tabi (biz yalnızca küçük sorgu yapıyoruz). Uç nokta
    `fdsnws/event/1/query?format=json` 2026-11-26 denendi.
  - AFAD: deprem.afad.gov.tr JavaScript ile oluşuyor; API belgesi ve
    kullanım koşulları bulunamadı (apiv2 parametresiz 400) → EKLENMEDİ;
    kullanıcıya izin e-postası önerildi.
- İstek yalnızca "Listeyi getir" ile; User-Agent "seismira"; kişisel bilgi
  gönderilmez. Canlı test `SEISMIRA_NET_TEST=1` (CI'da çalışmaz): USGS 162,
  EMSC 164 olay (M ≥ 5, 30 gün).
- 2026-11-26: "Senaryoya aktar" USGS olay ayrıntısından ShakeMap
  `download/rupture.json` (tercih ağırlığı en yüksek ShakeMap ürünü) ve
  moment tensörünün iki düğüm düzlemini alır; fay `realquakes::
  rupture_geometry` ile (scripts/gen_scenarios.py ile aynı kural: kollar
  atlanır, ≤ 10 km basamak, kapsama ≥ %90 → çok parçalı iz; yoksa tek
  düzlem); düzlem kırılmaya en yakın doğrultulu olan. Canlı test: Pazarcık
  2 kuad → 16 noktalı iz, 227,6° düzlemi (önce 1. düzlem 317,6° seçiliyordu).
  ShakeMap'i olmayan küçük olaylarda moment tensörü / varsayılan. AFAD'a
  izin e-postası 2026-11-26 gönderildi (kullanıcı).
- 2026-11-27: aynı ShakeMap ürününden `download/stationlist.json` da
  alınır (USGS kamu malı; atıf pencerede). Yalnız aletsel ("seismic")
  istasyonlar, PGV iki yatay kanalın geometrik ortalaması; ShakeMap'in
  işaretlediği (flag ≠ "0") genlikler atlanır. Diske yazılmaz, oturum
  boyunca bellekte (`realquakes::parse_stations`). Canlı test: Pazarcık
  261 istasyon.

