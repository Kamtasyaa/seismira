#!/usr/bin/env python3
"""GHSL GHS-BUILT-S R2023A (2020, EPSG:4326, 30″) yapılaşma alanını indirir,
SHA256 ile doğrular ve GeoTIFF'i çıkarır.

Kullanım (depo kökünden):
    python3 scripts/fetch_ghsl.py

Yalnızca veri hazırlamada kullanılır (şehir zemini, data/cities/*.ssit);
uygulamayla dağıtılmaz. Lisans: CC BY 4.0, (c) European Union 1995-2026
(copyright.txt, 2026-11-14 okundu).
"""

import hashlib
import pathlib
import sys
import urllib.request
import zipfile

NAME = "GHS_BUILT_S_E2020_GLOBE_R2023A_4326_30ss_V1_0"
URL = (
    "https://jeodpp.jrc.ec.europa.eu/ftp/jrc-opendata/GHSL/GHS_BUILT_S_GLOBE_R2023A/"
    f"GHS_BUILT_S_E2020_GLOBE_R2023A_4326_30ss/V1-0/{NAME}.zip"
)
SHA256 = "3dda6099f02b45e31548a4f2e75b82249b32c815238065db703b477bdcba8aa2"
DEST = pathlib.Path(__file__).resolve().parent.parent / "data" / "raw" / "ghsl"


def sha256(path: pathlib.Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def main() -> int:
    DEST.mkdir(parents=True, exist_ok=True)
    zip_path = DEST / f"{NAME}.zip"
    if not zip_path.exists() or sha256(zip_path) != SHA256:
        print(f"indiriliyor: {URL}", flush=True)
        req = urllib.request.Request(URL, headers={"User-Agent": "seismira-data"})
        with urllib.request.urlopen(req) as r, zip_path.open("wb") as f:
            while chunk := r.read(1 << 20):
                f.write(chunk)
    if sha256(zip_path) != SHA256:
        print(f"{zip_path.name}: SHA256 uyuşmuyor", file=sys.stderr)
        return 1
    tif = DEST / f"{NAME}.tif"
    if not tif.exists():
        with zipfile.ZipFile(zip_path) as z:
            z.extract(tif.name, DEST)
    print(f"tamam  {tif.name}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
