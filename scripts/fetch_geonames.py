#!/usr/bin/env python3
"""GeoNames cities15000 + admin1CodesASCII dökümünü indirir ve doğrular.

Kullanım (depo kökünden):
    python3 scripts/fetch_geonames.py

GeoNames dökümleri HER GÜN güncellenir; eski bir günün dosyası sunucudan
alınamaz. Bu yüzden depodaki `data/cities/cities15000.scty`, aşağıdaki
SHA256 değerleriyle sabitlenmiş 2026-11-14 anlık görüntüsünden üretilmiştir
(data/SOURCES.toml "geonames-cities15000"). Betik:
- dosyalar yerinde ve SHA256 tutuyorsa: "tamam" der;
- dosyalar yoksa: bugünkü dökümü indirir ve SHA256'nın sabitlenmiş değerden
  FARKLI olduğunu açıkça yazar (şehir listesi biraz farklı çıkar; bu durumda
  SOURCES.toml'daki anlık görüntü tarihi ve SHA256 güncellenmeli, `.ssit`
  yeniden üretilmelidir).
Lisans: CC BY 4.0 (https://download.geonames.org/export/dump/readme.txt,
2026-11-14 kontrol edildi).
"""

import hashlib
import io
import pathlib
import sys
import urllib.request
import zipfile

BASE = "https://download.geonames.org/export/dump"
SNAPSHOT = "2026-11-14"
DEST = pathlib.Path(__file__).resolve().parent.parent / "data" / "raw" / "geonames" / SNAPSHOT

# (dosya adı, sabitlenmiş sha256)
FILES = [
    ("cities15000.zip", "f274b5e5f2cff6211cb10cc5b348254a6e00dde9449244d18c0f542beb7581e5"),
    ("admin1CodesASCII.txt", "2e4867f5397acb056ea26377601b57af37cb4c81399770225018e9e439e91ad3"),
]


def main() -> int:
    DEST.mkdir(parents=True, exist_ok=True)
    changed = False
    for name, digest in FILES:
        dest = DEST / name
        if not dest.exists():
            print(f"indir   {name}", flush=True)
            req = urllib.request.Request(f"{BASE}/{name}", headers={"User-Agent": "seismira-data"})
            with urllib.request.urlopen(req, timeout=300) as resp:
                tmp = dest.with_suffix(dest.suffix + ".part")
                tmp.write_bytes(resp.read())
                tmp.replace(dest)
        if hashlib.sha256(dest.read_bytes()).hexdigest() == digest:
            print(f"tamam   {name}")
        else:
            print(f"FARKLI  {name} (sabitlenmiş {SNAPSHOT} anlık görüntüsü değil)")
            changed = True
    txt = DEST / "cities15000.txt"
    if not txt.exists():
        with zipfile.ZipFile(io.BytesIO((DEST / "cities15000.zip").read_bytes())) as z:
            z.extract(txt.name, DEST)
    if changed:
        print(
            "\nUYARI: GeoNames dökümü sabitlenmiş anlık görüntüden farklı. Üretilecek\n"
            "şehir listesi depodakinden farklı olur; data/SOURCES.toml'daki tarih ve\n"
            "SHA256 ile bu betikteki değerler güncellenmeli, cities15000.ssit yeniden\n"
            "üretilmelidir.",
            file=sys.stderr,
        )
    return 0


if __name__ == "__main__":
    sys.exit(main())
