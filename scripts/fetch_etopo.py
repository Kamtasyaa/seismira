#!/usr/bin/env python3
"""Faz 8 (tsunami) derinlik verisini indirir ve SHA256 ile doğrular.

Kullanım (depo kökünden):
    python3 scripts/fetch_etopo.py

NOAA NCEI ETOPO 2022, 60″ yüzey yüksekliği (GeoTIFF, 466 MB). Lisans:
CC0-1.0 (NCEI ISO üst verisi, 2026-11-23 kontrol edildi). Atıf: NOAA
National Centers for Environmental Information. 2022: ETOPO 2022 15
Arc-Second Global Relief Model. doi:10.25921/fd45-gt74.

GEBCO_2026 (kamu malı) asıl adaydı; dağıtım sunucusu (CEDA) 2026-11-23'te
TLS bağlantısını reddetti. ETOPO 2022'nin okyanus derinliği GEBCO tabanlıdır.
"""

import hashlib
import pathlib
import sys
import urllib.request

OUT = pathlib.Path(__file__).resolve().parent.parent / "data" / "raw" / "etopo"
URL = (
    "https://www.ngdc.noaa.gov/mgg/global/relief/ETOPO2022/data/60s/"
    "60s_surface_elev_gtif/ETOPO_2022_v1_60s_N90W180_surface.tif"
)
NAME = "ETOPO_2022_v1_60s_N90W180_surface.tif"
SHA256 = "9d27d4b8ea8e76977e2988bca667d7c8fa68b927355feffcddd6b4875a7fd08e"


def sha256(p: pathlib.Path) -> str:
    h = hashlib.sha256()
    with open(p, "rb") as f:
        for b in iter(lambda: f.read(1 << 22), b""):
            h.update(b)
    return h.hexdigest()


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    p = OUT / NAME
    if p.exists() and sha256(p) == SHA256:
        print(f"{NAME}: tamam")
        return 0
    print(f"{NAME} indiriliyor (466 MB)…")
    tmp = p.with_suffix(".part")
    urllib.request.urlretrieve(URL, tmp)
    if sha256(tmp) != SHA256:
        print(f"{NAME}: SHA256 uyuşmuyor", file=sys.stderr)
        return 1
    tmp.rename(p)
    return 0


if __name__ == "__main__":
    sys.exit(main())
