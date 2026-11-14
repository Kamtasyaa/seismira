#!/usr/bin/env python3
"""Uygulamanın çalışması için gereken tüm veriyi hazırlar.

Kullanım (depo kökünden):
    python3 scripts/prepare_data.py

Adımlar:
1. Yazı tipleri (CJK dahil) indirilir ve doğrulanır.
2. Natural Earth ham verisi indirilir ve doğrulanır.
3. seismira-data ile harita dosyaları (data/map/*.smap) üretilir.
4. Vs30 ham dosyaları doğrulanır (elle indirilir, bkz. fetch_vs30.py) ve
   data/vs30/vs30.sras üretilir (~1,5 dk, ~150 MB).

Her adım tekrar çalıştırılabilir; var olan doğru dosyalar yeniden indirilmez.
"""

import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent


def run(*cmd: str) -> None:
    print("$", " ".join(cmd), flush=True)
    subprocess.run(cmd, cwd=ROOT, check=True)


def main() -> int:
    py = sys.executable
    run(py, "scripts/fetch_fonts.py")
    run(py, "scripts/fetch_natural_earth.py")
    run(
        "cargo", "run", "--release", "-q", "-p", "seismira-data", "--",
        "natural-earth", "data/raw/natural-earth/v5.1.2", "data/map",
    )
    run(py, "scripts/fetch_vs30.py")
    vs30 = ROOT / "data" / "vs30" / "vs30.sras"
    if not vs30.exists():
        vs30.parent.mkdir(parents=True, exist_ok=True)
        run(
            "cargo", "run", "--release", "-q", "-p", "seismira-data", "--",
            "vs30", "data/raw/vs30/vs30_mosaic_median_30c.tif",
            "data/raw/vs30/vs30_mosaic_sd_30c.tif", str(vs30.relative_to(ROOT)),
        )
    print("Veri hazır.")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except subprocess.CalledProcessError as err:
        print(f"Adım başarısız: {err}", file=sys.stderr)
        sys.exit(1)
