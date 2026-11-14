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
| Dalma-batma geometrisi | USGS Slab2 | Kontrol et |
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
