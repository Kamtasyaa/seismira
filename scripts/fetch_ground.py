#!/usr/bin/env python3
"""Faz 7 (sıvılaşma / heyelan) girdi verilerini indirir ve SHA256 ile doğrular.

Kullanım (depo kökünden):
    python3 scripts/fetch_ground.py

- GMTED2010 md30 (30″ medyan yükseklik) ve md75 (7,5″), ESRI grid zip:
  USGS EROS, kamu malı (Danielson & Gesch 2011, USGS OFR 2011-1073).
- CHELSA V2.1 bio12 (yıllık yağış 1981–2010, 30″ GeoTIFF): CC0 1.0
  (EnviDat "chelsa-climatologies", 2026-11-22 kontrol edildi).

Ardından:
    cargo run --release -p seismira-data -- ground-inputs \\
        data/raw/gmted/md30/md30_grd data/vs30/vs30.sras \\
        data/raw/chelsa/CHELSA_bio12_1981-2010_V.2.1.tif \\
        data/raw/natural-earth/v5.1.2/ne_10m_rivers_lake_centerlines.shp \\
        data/ground/inputs30.sras
(~7 GB bellek; çıktı depoya girmez.)
"""

import hashlib
import pathlib
import sys
import urllib.request
import zipfile

ROOT = pathlib.Path(__file__).resolve().parent.parent / "data" / "raw"
GMTED = "https://edcintl.cr.usgs.gov/downloads/sciweb1/shared/topo/downloads/GMTED/Grid_ZipFiles"
CHELSA = "https://os.zhdk.cloud.switch.ch/chelsav2/GLOBAL/climatologies/1981-2010/bio"

# (klasör, dosya, adres, sha256, açılacak klasör)
FILES = [
    ("gmted", "md30_grd.zip", f"{GMTED}/md30_grd.zip",
     "146529b3542e420ba5c87db096c089f98bb713bf6d1dc0b4051671fa18634cdb", "md30"),
    ("gmted", "md75_grd.zip", f"{GMTED}/md75_grd.zip",
     "edb880fddb7d4b17c084cd46248ee3cc8ade49f3675045aa0bc3646c0abb4875", "md75"),
    ("chelsa", "CHELSA_bio12_1981-2010_V.2.1.tif", f"{CHELSA}/CHELSA_bio12_1981-2010_V.2.1.tif",
     "070c2309ead1ae703652e87f1ca51ba390c9ddfd9a236944528ae8b1c53a4e9a", None),
]


def sha256(path: pathlib.Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def main() -> int:
    for sub, name, url, digest, unpack in FILES:
        dest = ROOT / sub
        dest.mkdir(parents=True, exist_ok=True)
        path = dest / name
        if not path.exists() or sha256(path) != digest:
            print(f"indiriliyor: {url}", flush=True)
            req = urllib.request.Request(url, headers={"User-Agent": "seismira-data"})
            with urllib.request.urlopen(req) as r, path.open("wb") as f:
                while chunk := r.read(1 << 20):
                    f.write(chunk)
        if sha256(path) != digest:
            print(f"{name}: SHA256 uyuşmuyor (sürüm değişmiş olabilir)", file=sys.stderr)
            return 1
        if unpack and not (dest / unpack).exists():
            with zipfile.ZipFile(path) as z:
                z.extractall(dest / unpack)
        print(f"tamam  {name}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
