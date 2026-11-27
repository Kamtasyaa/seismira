#!/usr/bin/env python3
"""ESHM20 tehlike haritalarını (50 yılda %10 ve %2 aşılma) EFEHR REST
API'sinden indirir → data/raw/eshm20/ (ham JSON) ve data/raw/eshm20/eshm20.csv.

Kaynak: Danciu vd. (2021), The 2020 update of the European Seismic Hazard
Model (ESHM20), EFEHR, doi:10.12686/a15; lisans CC BY 4.0 (depo LICENSE
dosyası, 2026-11-27). API: https://efehr-services.ethz.ch/hazard/api/v1
(model 81), referans kaya (Vs30 800 m/s), dallar üzerinden aritmetik
ortalama (aggregationtype=arithmetic, level=0.5). Izgara 0,1° (boylam
0,0821339 + k·0,1; enlem j·0,1), yalnız kara ve kıyı noktaları.

Katmanlar (SEÇİM): PGA ve SA 0,1 / 0,2 / 0,3 / 0,5 / 1 / 2 / 3 s; yıllık
aşılma olasılığı 0,002103 (50 yılda %10) ve 0,000399999 (50 yılda ≈ %2).
Döşeme 10° × 10°; ilk katmanda boş çıkan döşemeler sonrakilerde sorulmaz
(veri kaybı yok), en çok 3 eşzamanlı istek; kesilirse kaldığı yerden sürer. Sonra: `seismira-data hazard data/raw/eshm20/eshm20.csv
data/hazard/eshm20.shaz`.

Çalıştırma: python3 scripts/fetch_eshm20.py
"""

import json
import pathlib
import sys
import time
import urllib.parse
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parent.parent
RAW = ROOT / "data" / "raw" / "eshm20"
API = "https://efehr-services.ethz.ch/hazard/api/v1/maps/area"
IMTS = ["PGA", "SA[0.10s]", "SA[0.20s]", "SA[0.30s]", "SA[0.50s]", "SA[1.00s]", "SA[2.00s]", "SA[3.00s]"]
POES = [("0.002103", "10in50"), ("0.000399999", "2in50")]
UA = {"User-Agent": "seismira-data-prep"}


def tile(imt, poe, lat0, lon0):
    q = urllib.parse.urlencode({
        "lat_min": lat0, "lon_min": lon0, "lat_max": lat0 + 10, "lon_max": lon0 + 10,
        "modelid": 81, "imt": imt, "poe": poe, "timespan": 1,
        "soiltype": "rock_vs30_800ms-1", "aggregationtype": "arithmetic",
        "aggregationlevel": 0.5,
    })
    for attempt in range(6):
        try:
            with urllib.request.urlopen(urllib.request.Request(f"{API}?{q}", headers=UA), timeout=180) as r:
                return json.load(r)
        except Exception as e:  # ağ hatası: bekle, yeniden dene
            print(f"  {imt} {poe} {lat0},{lon0}: {e} (deneme {attempt + 1})", flush=True)
            time.sleep(10 * (attempt + 1))
    raise SystemExit("indirilemedi")


def layer_dir(imt, tag):
    return RAW / f"{imt.replace('[', '_').replace(']', '')}_{tag}"


def main() -> int:
    RAW.mkdir(parents=True, exist_ok=True)
    tiles = [(lat, lon) for lat in range(20, 80, 10) for lon in range(-40, 50, 10)]
    # İlk katman (PGA, %10) tümüyle indirilir; onda boş çıkan döşemeler
    # (okyanus, model dışı) diğer katmanlarda da boştur, sorulmaz
    # (2026-11-27: sunucu yanıtı ~4 s'den ~46 s'ye çıktı, boş döşemeler dahil).
    first = layer_dir(IMTS[0], POES[0][1])
    jobs = []
    for imt in IMTS:
        for poe, tag in POES:
            d = layer_dir(imt, tag)
            d.mkdir(exist_ok=True)
            for lat0, lon0 in tiles:
                f = d / f"{lat0}_{lon0}.json"
                if f.exists():
                    continue
                ref = first / f"{lat0}_{lon0}.json"
                if d != first and ref.exists() and not json.loads(ref.read_text()).get("points"):
                    f.write_text(json.dumps({"points": [], "skipped": "ilk katmanda boş"}))
                    continue
                jobs.append((imt, poe, tag, lat0, lon0, f))

    def work(job):
        imt, poe, tag, lat0, lon0, f = job
        data = tile(imt, poe, lat0, lon0)
        f.write_text(json.dumps(data))
        print(f"{imt} {tag} {lat0},{lon0}: {len(data.get('points', []))} nokta", flush=True)

    # Sunucu yavaşken en çok 3 eşzamanlı istek (nazik sınır).
    from concurrent.futures import ThreadPoolExecutor

    print(f"{len(jobs)} döşeme indirilecek", flush=True)
    with ThreadPoolExecutor(max_workers=3) as ex:
        list(ex.map(work, jobs))
    # Birleşik CSV: imt, poe etiketi, boylam, enlem, değer (g). Döşeme
    # kenarındaki noktalar iki döşemede de gelebilir: tekrarlar atılır.
    out = RAW / "eshm20.csv"
    n = 0
    with open(out, "w") as w:
        w.write("imt,poe,lon,lat,value_g\n")
        for imt in IMTS:
            for poe, tag in POES:
                d = RAW / f"{imt.replace('[', '_').replace(']', '')}_{tag}"
                seen = set()
                for f in sorted(d.glob("*.json")):
                    for p in json.loads(f.read_text()).get("points", []):
                        k = (round(p["lon"], 4), round(p["lat"], 4))
                        if k in seen:
                            continue
                        seen.add(k)
                        w.write(f"{imt},{poe},{p['lon']:.6f},{p['lat']:.4f},{p['value']:.6g}\n")
                        n += 1
    print(f"{out}: {n} değer")
    return 0


if __name__ == "__main__":
    sys.exit(main())
