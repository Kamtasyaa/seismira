#!/usr/bin/env python3
"""USGS STREC kararlı kıta içi bölge çokgenlerini indirir ve SHA256 ile doğrular.

Kaynak: github.com/usgs/strec (arşivlenmiş depo), commit 6e13af7f,
strec/data/stable.geojson; lisans: kamu malı (depodaki license.txt,
2026-11-25 okundu). docs/bilim.md 7.4, data/SOURCES.toml "usgs-strec-stable".

Kullanım (depo kökünden):
    python3 scripts/fetch_strec.py
Ardından:
    cargo run --release -p seismira-data -- stable-regions \\
        data/raw/strec/6e13af7f76d52adfeefbd74dbe647705e92db7d0/stable.geojson \\
        data/regions/stable.sout
"""

import hashlib
import pathlib
import sys
import urllib.request

COMMIT = "6e13af7f76d52adfeefbd74dbe647705e92db7d0"
BASE = f"https://raw.githubusercontent.com/usgs/strec/{COMMIT}"
DEST = pathlib.Path(__file__).resolve().parent.parent / "data" / "raw" / "strec" / COMMIT
FILES = [
    ("strec/data/stable.geojson", "a1b11b7192554a9ca13ebb4d999d252a6ef8ab1fc8dd1d500a06906cbe9db3e4"),
    ("license.txt", "e4beb4f2dbf0ef36079633fc051c4407debcefd3caa143cce2fb6e955ffc7b66"),
]


def main() -> int:
    DEST.mkdir(parents=True, exist_ok=True)
    for rel, digest in FILES:
        path = DEST / pathlib.PurePosixPath(rel).name
        if not path.exists() or hashlib.sha256(path.read_bytes()).hexdigest() != digest:
            req = urllib.request.Request(f"{BASE}/{rel}", headers={"User-Agent": "seismira-data"})
            with urllib.request.urlopen(req, timeout=60) as r:
                path.write_bytes(r.read())
        if hashlib.sha256(path.read_bytes()).hexdigest() != digest:
            print(f"{path.name}: SHA256 uyuşmuyor", file=sys.stderr)
            return 1
        print(f"tamam  {path.name}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
