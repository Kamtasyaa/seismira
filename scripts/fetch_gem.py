#!/usr/bin/env python3
"""GEM küresel bina stoku özetleri ve yapısal zafiyet fonksiyonlarını
indirir ve doğrular (Faz 9; ayrı "GEM paketi", docs/veri.md).

Kaynaklar (ikisi de CC BY-NC-SA 4.0, ticari kullanım YOK):
- GEM Global Exposure Model, https://github.com/gem/global_exposure_model
  Künye: Yepes-Estrada, C., Calderon, A., Costa, C., Crowley, H., Dabbeek,
  J., Hoyos, M., Martins, L., Paul, N., Rao, A., Silva, V. (2023). Global
  Building Exposure Model for Earthquake Risk Assessment. Earthquake
  Spectra, doi:10.1177/87552930231194048. Veri: Global Exposure Model
  (2026.0.0), doi:10.5281/zenodo.8117363.
- GEM Global Vulnerability Model, https://github.com/gem/global_vulnerability_model
  Künye (deponun README'sinden): Nafeh, A.M.B., Aljawhari, K., Silva, V.
  (2025). The 2025 Global Vulnerability Model of the GEM Foundation,
  doi:10.5281/zenodo.8391742; Aljawhari, K., Nafeh, A.M.B., Silva, V.
  (2026). A new global vulnerability model for regional seismic risk
  assessments: Part 1 – structural vulnerability. Bull. Earthquake Eng.,
  doi:10.1007/s10518-026-02443-7.

Sürümler SABİTLENMİŞTİR (commit'ler aşağıda). İlk indirmede SHA256 özetleri
data/raw/gem/MANIFEST.sha256 dosyasına yazılır; sonraki çalıştırmalar
dosyaları bu listeye göre doğrular.

İndirilenler (data/raw/gem/, depoya girmez; ~125 MB):
- exposure/World/summaries/Exposure_Summary_Adm1_Taxonomy.csv (Adm1 ×
  sınıf; bina sayısı, yeniden yapım değeri)
- exposure/World/summaries/Vulnerability_mapping_country.csv (sınıf →
  zafiyet fonksiyonu, ağırlık)
- vulnerability/<bölge>/<ülke>/vulnerability_structural.xml ve
  vulnerability_nonstructural.xml (215 ülke; yapısal olmayan ~26 MB,
  2026-11-23'te eklendi)
- her iki deponun LICENSE.txt, README.md dosyaları

Çalıştırma (depo kökünden): python3 scripts/fetch_gem.py
"""

import hashlib
import json
import pathlib
import sys
import urllib.parse
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parent.parent
OUT = ROOT / "data" / "raw" / "gem"
MANIFEST = OUT / "MANIFEST.sha256"

REPOS = {
    # main, 2026-11-23'te görüldü.
    "exposure": ("gem/global_exposure_model", "e2a74b9ff5c85ee8f1ab8bfd571f3662c28dea30"),
    "vulnerability": (
        "gem/global_vulnerability_model",
        "5974372ac3f4a99f25d0649eb030fbe596f23b36",
    ),
}
EXPOSURE_FILES = [
    "LICENSE.txt",
    "README.md",
    "World/summaries/Exposure_Summary_Adm1_Taxonomy.csv",
    "World/summaries/Vulnerability_mapping_country.csv",
]


def get(url: str) -> bytes:
    req = urllib.request.Request(url, headers={"User-Agent": "seismira-data"})
    with urllib.request.urlopen(req, timeout=300) as r:
        return r.read()


def vulnerability_files() -> list[str]:
    repo, sha = REPOS["vulnerability"]
    tree = json.loads(get(f"https://api.github.com/repos/{repo}/git/trees/{sha}?recursive=1"))
    if tree.get("truncated"):
        raise RuntimeError("GitHub ağaç listesi kesik geldi")
    paths = [
        t["path"]
        for t in tree["tree"]
        if t["type"] == "blob"
        and t["path"].endswith(("/vulnerability_structural.xml", "/vulnerability_nonstructural.xml"))
    ]
    return ["LICENSE.txt", "README.md"] + sorted(paths)


def download(key: str, path: str) -> bytes:
    repo, sha = REPOS[key]
    url = f"https://raw.githubusercontent.com/{repo}/{sha}/{urllib.parse.quote(path)}"
    data = get(url)
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
    if known:
        wanted = sorted(known)
        # Eski manifest (yalnızca yapısal): yapısal olmayan dosyalar eklenir.
        if not any(p.endswith("vulnerability_nonstructural.xml") for p in known):
            extra = [f"vulnerability/{p}" for p in vulnerability_files()]
            wanted = sorted(set(wanted) | set(extra))
    else:
        wanted = [f"exposure/{p}" for p in EXPOSURE_FILES]
        wanted += [f"vulnerability/{p}" for p in vulnerability_files()]
    lines = []
    for p in wanted:
        dst = OUT / p
        if not dst.exists():
            dst.parent.mkdir(parents=True, exist_ok=True)
            print(f"indiriliyor: {p}", flush=True)
            key, sub = p.split("/", 1)
            dst.write_bytes(download(key, sub))
        h = hashlib.sha256(dst.read_bytes()).hexdigest()
        if p in known and known[p] != h:
            print(f"SHA256 uyuşmuyor: {p}", file=sys.stderr)
            return 1
        lines.append(f"{h}  {p}")
    if len(lines) != len(known):
        MANIFEST.write_text("\n".join(lines) + "\n")
        print(f"{MANIFEST.relative_to(ROOT)} yazıldı ({len(lines)} dosya)")
    else:
        print(f"{len(lines)} dosya doğrulandı")
    return 0


if __name__ == "__main__":
    sys.exit(main())
