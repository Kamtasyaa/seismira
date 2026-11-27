# Sürüm notları / Release notes

Yayımlanmış sürümlerin notları GitHub Releases sayfasındadır. Bu dosya bir
sonraki sürümün TASLAĞIDIR; yayımlarken GitHub'a aynen aktarılır.

## v0.1.4 (TASLAK, yayımlanmadı)

Yayım öncesi yapılacaklar:
- Tam ESHM20 dosyası (`data/hazard/eshm20.shaz`) üretildi (2026-11-27,
  16 katman, 1,8 MB). Dünya kritik tesis verisi (`data/infra/infra.sinf`)
  Avrupa işlenince üretilecek; yeni veri paketi data-v0.2.0 (boyut sınırı
  yok, CLAUDE.md Bölüm 12); release.yml DATA_TAG / DATA_FILE /
  DATA_SHA256 paket yüklenince güncellenecek.
- Kullanıcıya sorulacak: GEM verisinin aynı pakete girmesi (paket satılamaz
  olur, CLAUDE.md Bölüm 13).

İmzasız derleme; kurulum ve ilk açılış uyarısı için arşivdeki KURULUM.txt.
Unsigned build; see KURULUM.txt in the archive. **Yeni veri paketi gerekir /
requires the new data package.**

**Değişiklikler (v0.1.3'ten bu yana)**

Yeni özellikler
- Kritik tesisler: hastane, köprü, viyadük, baraj, trafo merkezi ve doğalgaz
  hatlarında tahmini sarsıntı; türe göre özet, sıralı liste, CSV, haritada
  işaretler (Veri → Kritik tesisler; OpenStreetMap, ODbL)
- Toplu senaryo: bir fay üzerinde büyüklük aralığında çok sayıda deprem,
  ortalama ve en kötü şiddet haritası (Veri → Toplu senaryo)
- Tehlike haritası (ESHM20): 50 yılda %10 ve %2 aşılma, PGA ve spektral
  ivme; tepki spektrumu penceresinde eş tehlike spektrumu (Görünüm →
  Tehlike haritası; Avrupa ve Türkiye, CC BY 4.0)
- Yer hareketi modelleri: Akkar, Sandıkkaya ve Bommer (2014) Avrupa–Orta
  Doğu modeli ve model ağırlıkları (sağ panel)
- Gerçek depremlerde ShakeMap istasyon kayıtlarıyla karşılaştırma (Veri →
  Gözlemlerle karşılaştır) ve ShakeMap kırılma modelinden fay
- Açık tema (Görünüm → Tema)
- Metin ve sayı kutularında Kes / Kopyala / Yapıştır / Tümünü seç (sağ tık
  ve Ctrl+C / Ctrl+V); Veri Kaynakları sayfasındaki bağlantılar açılır

İyileştirmeler ve düzeltmeler
- Erken uyarı: açık deniz depremlerinde ilk istasyon en yakın karada (2011
  Tōhoku'da uyarı JMA kaydına yakın)
- CY14 modelinin Japonya, İtalya ve Çin ayarları
- Bina hasarı: "hasarsızlık sınırı" belirsizlikle birlikte uygulanır
  (sınırda yapay sıçrama yok)
- Sağ tuş / dokunmatik yüzeyde iki parmakla yakınlaştırmada imleç yerinde
  kalır, hız ayarlandı
- Açılış daha hızlı (hasar eğrileri arka planda hazırlanır)
- Harita işaretleri şehir kutularının ve açıklamaların altında

**Changes since v0.1.3**

New features
- Critical facilities: estimated shaking at hospitals, bridges, viaducts,
  dams, substations and gas pipelines; summary by type, sorted list, CSV,
  map markers (Data → Critical facilities; OpenStreetMap, ODbL)
- Batch scenarios: many earthquakes over a magnitude range on one fault,
  mean and worst-case intensity maps (Data → Batch scenarios)
- Hazard map (ESHM20): 10% and 2% in 50 years, PGA and spectral
  acceleration; uniform hazard spectra in the response spectrum window
  (View → Hazard map; Europe and Türkiye, CC BY 4.0)
- Ground-motion models: Akkar, Sandıkkaya & Bommer (2014) for Europe and
  the Middle East, with model weights (right panel)
- Recent earthquakes: comparison with ShakeMap station records (Data →
  Compare with observations) and faults from the ShakeMap rupture model
- Light theme (View → Theme)
- Cut / Copy / Paste / Select all in text and number fields (right click
  and Ctrl+C / Ctrl+V); links in the Data Sources page open

Improvements and fixes
- Early warning: for offshore earthquakes the first station is on the
  nearest land (2011 Tōhoku close to the JMA record)
- Regional terms of the CY14 model for Japan, Italy and China
- Building damage: the "no damage limit" is applied together with the
  uncertainty (no artificial jump at the limit)
- Right-drag / two-finger zoom keeps the cursor in place, speed tuned
- Faster start-up (damage curves prepared in the background)
- Map markers drawn below city boxes and legends
