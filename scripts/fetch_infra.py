#!/usr/bin/env python3
"""Kritik tesis verisini (OSM, ODbL) dünya için üretir: Geofabrik kıta
özütleri sırayla indirilir, `scripts/gen_infra.py` ile süzülür ve (disk
için) silinir; sonunda `seismira-data infra` ikili dosyayı yazar.

Çıktı: data/infra/infra.sinf; ara: data/raw/osm/infra.tsv; her özütün
adı, tarihi (HTTP Last-Modified) ve SHA256'sı data/raw/osm/manifest.txt'ye
yazılır (Geofabrik "latest" günlük güncellenir; aynı dosyanın yeniden
indirilebilmesi garanti değil, docs/veri.md).

Ağır: ~90 GB indirme, referans makinede birkaç saat. Kullanım:
    python3 scripts/fetch_infra.py
"""

import hashlib
import pathlib
import subprocess
import sys
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parent.parent
RAW = ROOT / "data" / "raw" / "osm"
# Geofabrik üst düzey bölgeleri (birbirini kapsamaz; Rusya ayrı).
REGIONS = [
    "antarctica",
    "australia-oceania",
    "central-america",
    "south-america",
    "africa",
    "russia",
    "asia",
    "north-america",
    "europe",
]
UA = {"User-Agent": "seismira-data-prep"}


def sha256(p):
    h = hashlib.sha256()
    with open(p, "rb") as f:
        for c in iter(lambda: f.read(1 << 22), b""):
            h.update(c)
    return h.hexdigest()


def main() -> int:
    RAW.mkdir(parents=True, exist_ok=True)
    tsv = RAW / "infra.tsv"
    manifest = RAW / "manifest.txt"
    done = set()
    if manifest.exists():
        done = {l.split()[0] for l in manifest.read_text().splitlines() if l.strip()}
    py = str(ROOT / ".venv" / "bin" / "python")
    for region in REGIONS:
        part = RAW / f"infra-{region}.tsv"
        if region in done and part.exists():
            print(f"{region}: hazır")
            continue
        url = f"https://download.geofabrik.de/{region}-latest.osm.pbf"
        pbf = RAW / f"{region}-latest.osm.pbf"
        req = urllib.request.Request(url, headers=UA, method="HEAD")
        with urllib.request.urlopen(req) as r:
            modified = r.headers.get("Last-Modified", "?").replace(" ", "_")
        print(f"{region}: indiriliyor ({modified})", flush=True)
        subprocess.run(["curl", "-sSL", "-A", UA["User-Agent"], "-o", str(pbf), url], check=True)
        digest = sha256(pbf)
        subprocess.run([py, "-I", str(ROOT / "scripts" / "gen_infra.py"), str(pbf), str(part)], check=True)
        pbf.unlink()
        with open(manifest, "a") as f:
            f.write(f"{region} {modified} {digest}\n")
    with open(tsv, "w") as out:
        for region in REGIONS:
            out.write((RAW / f"infra-{region}.tsv").read_text())
    (ROOT / "data" / "infra").mkdir(parents=True, exist_ok=True)
    subprocess.run(
        ["cargo", "run", "--release", "-q", "-p", "seismira-data", "--",
         "infra", str(tsv.relative_to(ROOT)), "data/infra/infra.sinf"],
        check=True, cwd=ROOT,
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
