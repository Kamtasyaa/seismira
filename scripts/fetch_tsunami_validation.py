#!/usr/bin/env python3
"""Faz 8 adım 7: tsunami doğrulama gözlemlerini indirir.

Kullanım (depo kökünden): python3 scripts/fetch_tsunami_validation.py

Kaynak: NOAA NCEI / WDS Global Historical Tsunami Database, HazEL API
(https://www.ngdc.noaa.gov/hazel/hazard-service/api/v1/tsunamis/runups),
doi:10.7289/V5PN93H7. Olaylar: 5413 Tōhoku 2011, 5907 Noto 2024, 4682
Maule 2010. Çıktı data/raw/validation/tsunami/ncei_runups_<id>.json (depoda
değil). Kayıtlar: konum, varış (arrHour/arrMin, travHours/travMins),
ölçülen su yüksekliği (runupHt), ölçüm türü (typeMeasurementId).
"""

import json
import pathlib
import sys
import time
import urllib.request

OUT = pathlib.Path(__file__).resolve().parent.parent / "data" / "raw" / "validation" / "tsunami"
API = "https://www.ngdc.noaa.gov/hazel/hazard-service/api/v1/tsunamis/runups"
EVENTS = {5413: "Tōhoku 2011", 5907: "Noto 2024", 4682: "Maule 2010"}


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    for eid, name in EVENTS.items():
        items, page = [], 1
        while True:
            url = f"{API}?tsunamiEventId={eid}&itemsPerPage=200&page={page}"
            with urllib.request.urlopen(url, timeout=120) as r:
                d = json.load(r)
            items += d["items"]
            if page >= d.get("totalPages", 1):
                break
            page += 1
            time.sleep(0.3)
        (OUT / f"ncei_runups_{eid}.json").write_text(json.dumps(items))
        print(f"{name} ({eid}): {len(items)} kayıt")
    return 0


if __name__ == "__main__":
    sys.exit(main())
