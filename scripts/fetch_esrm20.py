#!/usr/bin/env python3
"""ESRM20 (European Seismic Risk Model 2020) bina stoku ve kırılganlık
dosyalarını indirir ve doğrular (Faz 6).

Kaynak: EFEHR GitLab, https://gitlab.seismo.ethz.ch/efehr/esrm20 (CC BY 4.0).
Künye: Crowley, H., Dabbeek, J., Despotaki, V., Rodrigues, D., Martins, L.,
Silva, V., Romão, X., Pereira, N., Weatherill, G., Danciu, L. (2021).
European Seismic Risk Model (ESRM20). EFEHR Technical Report 002, V1.0.1,
doi:10.7414/EUC-EFEHR-TR002-ESRM20.

Sürüm SABİTLENMİŞTİR (commit aşağıda). İlk indirmede SHA256 özetleri
data/raw/esrm20/<commit>/MANIFEST.sha256 dosyasına yazılır; sonraki
çalıştırmalar dosyaları bu listeye göre doğrular.

İndirilenler (data/raw/esrm20/<commit>/, depoya girmez):
- Exposure/OQ_Exposure_Input_<ülke>.xml ve _Res/_Com/_Ind.csv (44 ülke,
  idari birim düzeyi)
- Vulnerability/esrm20_exposure_vulnerability_mapping.csv
- Vulnerability/fragility_ESRM20_Various_IM.xml
- LICENSE, CREDITS, ESRM20_CITATION.cff

Çalıştırma (depo kökünden): python3 scripts/fetch_esrm20.py
"""

import hashlib
import json
import pathlib
import sys
import urllib.parse
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parent.parent
PROJECT = 269  # efehr/esrm20
COMMIT = "b2368bee1f992a0db9f4db69b42143c19f5486c7"  # main, 2025-01-19
API = f"https://gitlab.seismo.ethz.ch/api/v4/projects/{PROJECT}/repository"
OUT = ROOT / "data" / "raw" / "esrm20" / COMMIT
MANIFEST = OUT / "MANIFEST.sha256"

FIXED = [
    "LICENSE",
    "CREDITS",
    "ESRM20_CITATION.cff",
    "Vulnerability/esrm20_exposure_vulnerability_mapping.csv",
    "Vulnerability/fragility_ESRM20_Various_IM.xml",
]


def get(url: str) -> bytes:
    req = urllib.request.Request(url, headers={"User-Agent": "seismira-data"})
    with urllib.request.urlopen(req, timeout=300) as r:
        return r.read()


def tree() -> list[str]:
    paths, page = [], 1
    while True:
        q = urllib.parse.urlencode(
            {"path": "Exposure", "ref": COMMIT, "per_page": 100, "page": page}
        )
        items = json.loads(get(f"{API}/tree?{q}"))
        if not items:
            return paths
        paths += [i["path"] for i in items if i["type"] == "blob"]
        page += 1


def download(path: str) -> bytes:
    enc = urllib.parse.quote(path, safe="")
    # lfs=true: Git LFS'teki dosyalarda işaretçi yerine içerik döner.
    data = get(f"{API}/files/{enc}/raw?ref={COMMIT}&lfs=true")
    if data.startswith(b"version https://git-lfs"):
        raise RuntimeError(f"{path}: LFS işaretçisi indi, içerik değil")
    return data


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    known = {}
    if MANIFEST.exists():
        for line in MANIFEST.read_text().splitlines():
            h, p = line.split("  ", 1)
            known[p] = h
    wanted = known.keys() if known else FIXED + [
        p for p in tree() if p.endswith((".csv", ".xml")) and "OQ_Exposure_Input_" in p
    ]
    lines = []
    for p in sorted(wanted):
        dst = OUT / p
        if not dst.exists():
            dst.parent.mkdir(parents=True, exist_ok=True)
            print(f"indiriliyor: {p}", flush=True)
            dst.write_bytes(download(p))
        h = hashlib.sha256(dst.read_bytes()).hexdigest()
        if known and known[p] != h:
            print(f"SHA256 uyuşmuyor: {p}", file=sys.stderr)
            return 1
        lines.append(f"{h}  {p}")
    if not known:
        MANIFEST.write_text("\n".join(lines) + "\n")
        print(f"{MANIFEST.relative_to(ROOT)} yazıldı ({len(lines)} dosya)")
    else:
        print(f"{len(lines)} dosya doğrulandı")
    return 0


if __name__ == "__main__":
    sys.exit(main())
