#!/usr/bin/env python3
"""Dağıtılacak temel veri paketini toplar (Faz 10 adım 7; CLAUDE.md Bölüm 12-13).

Çıktı: dist/seismira-data-<sürüm>.tar.zst (içinde `data/` ağacı ve
`data/MANIFEST.sha256`). Sürüm paketleme iş akışında (.github/workflows/
release.yml) GitHub sürüm eki olarak indirilir ve her platformun ikilisiyle
birleştirilir; uygulama `data/` klasörünü çalıştırılabilir dosyanın yanında
arar (crates/seismira-app/src/paths.rs).

YALNIZCA lisansı dağıtıma uygun dosyalar girer (açık liste, aşağıda); her
satırın lisansı data/SOURCES.toml'dadır. Girmeyenler:
- Yerel data/ground/ls_static.sras (heyelan): vektör GLiM'den, dağıtım izni
  yok (docs/veri.md) → heyelan, litolojisi GLiM 0,5° (CC BY 3.0,
  scripts/gen_glim05.py; doğrulama docs/dogrulama.md) olan bir kopyayla
  YENİDEN üretilir (`ls-static`, geçici zemin klasöründe).
- data/ground/wtd.sras ve onunla üretilmiş liq_static: Fan vd. (2017) yeraltı
  su seviyesi "yalnızca araştırma", izin bekleniyor → sıvılaşma KIYI modeliyle
  (`liq-static --coastal`, yeraltı suyu istemez) yeniden üretilir.
- data/gem/: CC BY-NC-SA, AYRI paket (ticari kullanım yok).
- data/raw, data/validation, data/test, ara katmanlar (inputs30, slope75,
  landcover, lith).

Gerekenler: yerelde üretilmiş veri (python3 scripts/prepare_data.py --ground)
ve `cargo build --release -p seismira-data`.

Çalıştırma (depo kökünden): python3 scripts/package_data.py <sürüm>
"""

import hashlib
import pathlib
import shutil
import subprocess
import sys
import tarfile

ROOT = pathlib.Path(__file__).resolve().parent.parent
DATA = ROOT / "data"

# (yol data/ altında, SOURCES.toml kimliği)
FILES = [
    ("SOURCES.toml", "—"),
    ("map/ne_110m.smap", "natural-earth"),
    ("map/ne_50m.smap", "natural-earth"),
    ("map/ne_10m.smap", "natural-earth"),
    ("regions/admin1.sras", "natural-earth"),
    ("regions/admin1.sreg", "natural-earth + geonames-cities15000"),
    ("regions/admin1.sout", "natural-earth"),
    ("regions/stable.sout", "usgs-strec-stable"),
    ("traveltime/ak135.ttt", "ak135-traveltimes"),
    ("cities/cities15000.scty", "geonames-cities15000"),
    ("cities/cities15000.ssit", "geonames-cities15000 + usgs-global-vs30-2026 + ghsl-built-s-2020"),
    ("vs30/vs30.sras", "usgs-global-vs30-2026"),
    ("slab2/slab2.sslb", "usgs-slab2"),
    ("exposure/esrm20.sexp", "esrm20"),
    ("tsunami/bathy.sras", "etopo-2022-60s"),
    ("tsunami/coast.bin", "etopo-2022-60s + geonames-cities15000"),
    ("scenarios/scenarios.json", "usgs-scenarios"),
    # ODbL: ayrı dosya, başka veriyle birleştirilmez (kullanıcı kararı
    # 2026-11-27: ana pakette, herkese aynı veri).
    ("infra/infra.sinf", "openstreetmap-infra"),
]


def sha256(p: pathlib.Path) -> str:
    h = hashlib.sha256()
    with p.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__)
        return 2
    version = sys.argv[1]
    name = f"seismira-data-{version}"
    out = ROOT / "dist" / name
    if out.exists():
        shutil.rmtree(out)
    (out / "data").mkdir(parents=True)

    missing = [p for p, _ in FILES if not (DATA / p).exists()]
    if missing:
        print("eksik (önce scripts/prepare_data.py):", *missing, sep="\n  ")
        return 1
    for p, _ in FILES:
        dst = out / "data" / p
        dst.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(DATA / p, dst)

    # Sıvılaşma: dağıtılabilir kıyı modeli (yeraltı suyu katmanı olmadan).
    tool = ROOT / "target" / "release" / ("seismira-data" + (".exe" if sys.platform == "win32" else ""))
    if not tool.exists():
        print("önce: cargo build --release -p seismira-data")
        return 1
    liq = out / "data" / "ground" / "liq_static.sras"
    liq.parent.mkdir(parents=True)
    subprocess.run(
        [str(tool), "liq-static", str(DATA / "ground"), str(DATA / "vs30" / "vs30.sras"), str(liq), "--coastal"],
        check=True,
    )

    # Heyelan: vektör GLiM yerine GLiM 0,5° litolojisiyle (dağıtılabilir).
    glim05 = ROOT / "data" / "raw" / "glim05" / "glim05_vs30grid.u8"
    if not glim05.exists():
        print("önce: .venv/bin/python -I scripts/gen_glim05.py")
        return 1
    tmp = ROOT / "dist" / f"{name}-ground-tmp"
    if tmp.exists():
        shutil.rmtree(tmp)
    tmp.mkdir()
    for f in ("inputs30.sras", "slope75.sras", "landcover.sras"):
        (tmp / f).symlink_to(DATA / "ground" / f)
    vs30 = str(DATA / "vs30" / "vs30.sras")
    subprocess.run([str(tool), "lith", str(glim05), vs30, str(tmp / "lith.sras")], check=True)
    subprocess.run(
        [str(tool), "ls-static", str(tmp), vs30, str(out / "data" / "ground" / "ls_static.sras")],
        check=True,
    )
    shutil.rmtree(tmp)

    lines = []
    total = 0
    for f in sorted((out / "data").rglob("*")):
        if f.is_file():
            rel = f.relative_to(out).as_posix()
            lines.append(f"{sha256(f)}  {rel}")
            total += f.stat().st_size
    (out / "data" / "MANIFEST.sha256").write_text("\n".join(lines) + "\n")

    archive = ROOT / "dist" / f"{name}.tar.zst"
    with tarfile.open(archive, "w:zst") as tar:
        tar.add(out / "data", arcname="data")
    print(f"{len(lines)} dosya, {total / 1e6:.0f} MB → {archive.relative_to(ROOT)} ({archive.stat().st_size / 1e6:.0f} MB)")
    print(f"sha256 {sha256(archive)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
