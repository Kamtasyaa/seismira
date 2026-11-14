#!/usr/bin/env python3
"""Uygulamanın çalışması için gereken tüm veriyi hazırlar.

Kullanım (depo kökünden):
    python3 scripts/prepare_data.py

Adımlar:
1. Yazı tipleri (CJK dahil) indirilir ve doğrulanır.
2. Natural Earth ham verisi indirilir ve doğrulanır.
3. seismira-data ile harita dosyaları (data/map/*.smap) üretilir.

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
    print("Veri hazır.")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except subprocess.CalledProcessError as err:
        print(f"Adım başarısız: {err}", file=sys.stderr)
        sys.exit(1)
