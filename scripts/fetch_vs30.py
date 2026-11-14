#!/usr/bin/env python3
"""USGS Küresel Vs30 Mozaiği (2026) ham dosyalarını SHA256 ile doğrular.

Kullanım (depo kökünden):
    python3 scripts/fetch_vs30.py

ScienceBase betik indirmelerini engelliyor (HTTP 403, 2026-11-14), bu
yüzden dosyalar bir kez tarayıcıdan indirilir. Eksik ya da farklıysa bu
betik ne yapılacağını yazar ve hata koduyla çıkar.
Lisans: CC0 1.0 (doi:10.5066/P1NV6UNM, 2026-11-14 kontrol edildi).
"""

import hashlib
import pathlib
import sys

PAGE = "https://www.sciencebase.gov/catalog/item/67be4ac3d34e8876fcbfbd89"
DEST = pathlib.Path(__file__).resolve().parent.parent / "data" / "raw" / "vs30"

FILES = [
    ("vs30_mosaic_median_30c.tif", "00bd29b764f91cd085433e0c2ac07fd0c3607fe5ba5e3ebe98ae3afc08ee8cb3"),
    ("vs30_mosaic_sd_30c.tif", "4e752c232ff0af65d2619683c7a3d16bd2c7191575b2768c0460c271290a4392"),
    ("version.json", "1c22b760b6764169b53ecacca730e052af4daa7daeb98b396301c28a5fda005a"),
]


def sha256(path: pathlib.Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def main() -> int:
    bad = []
    for name, digest in FILES:
        path = DEST / name
        if not path.exists():
            bad.append(f"{name}: yok")
        elif sha256(path) != digest:
            bad.append(f"{name}: SHA256 uyuşmuyor (farklı sürüm olabilir)")
        else:
            print(f"tamam  {name}")
    if bad:
        print("\nVs30 ham dosyaları eksik ya da farklı:", file=sys.stderr)
        for b in bad:
            print(f"  - {b}", file=sys.stderr)
        print(
            f"\nTarayıcıdan indirin: {PAGE}\n"
            f"ve şu klasöre koyun: {DEST}\n"
            "(Farklı sürüm indirildiyse lisans ve SHA256 bu betikte ve "
            "data/SOURCES.toml'da güncellenmeli.)",
            file=sys.stderr,
        )
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
