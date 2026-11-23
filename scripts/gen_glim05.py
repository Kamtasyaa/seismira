#!/usr/bin/env python3
"""GLiM 0,5° raster (CC BY 3.0) → Vs30 ızgarasında ham u8 litoloji.

Kaynak: Hartmann, J., Moosdorf, N. (2012). Global Lithological Map Database
v1.0 (gridded to 0.5° spatial resolution). PANGAEA,
doi:10.1594/PANGAEA.788537, lisans CC BY 3.0 (PANGAEA meta verisinde,
2026-11-23 okundu). Dosya: hdl.handle.net/10013/epic.39939.d001 →
epic.awi.de/id/eprint/31092/1/hartmann-moosdorf_2012.zip (38 670 B).

Amaç: vektör GLiM'in dağıtım izni gelmezse heyelan katmanını DAĞITILABİLİR
yapmak (docs/veri.md). Bu sürüm çok kaba (0,5° ≈ 55 km); etkisi
docs/dogrulama.md "GLiM 0,5°" bölümünde ölçüldü.

Kıyı boşlukları en yakın sınıflı 0,5° hücreyle doldurulur (aşağıda).

Biçim: ESRI ASCII ızgara, 720 × 360, sol alt (−180, −90), NODATA −9999;
değer 1–16, Classnames.txt sırası (su, vb, ss, pb, sm, sc, va, mt, pa, vi,
wb, py, pi, ev, nd, ig). Çıktı kodu jessee2018::GLIM_CLASSES sırası
(scripts/gen_glim.py ile aynı), 0 = veri yok. Örnekleme: 30″ hücre
merkezinin düştüğü 0,5° hücre (en yakın komşu).

Çıktı: data/raw/glim05/glim05_vs30grid.u8 (ham u8, kuzeyden güneye), sonra
    target/release/seismira-data lith data/raw/glim05/glim05_vs30grid.u8 \\
        data/vs30/vs30.sras <çıktı>/lith.sras

Gereken: numpy, scipy (.venv). Çalıştırma: .venv/bin/python -I scripts/gen_glim05.py
"""

import hashlib
import io
import pathlib
import sys
import urllib.request
import zipfile

import numpy as np
from scipy import ndimage

ROOT = pathlib.Path(__file__).resolve().parent.parent
SRC = ROOT / "data" / "raw" / "glim05"
ZIP = SRC / "hartmann-moosdorf_2012.zip"
URL = "https://epic.awi.de/id/eprint/31092/1/hartmann-moosdorf_2012.zip"
ZIP_SHA256 = "43b4ce3276b155d804db8ff9fb227d620b4c35015a4cf564eac4d06d2b69d88e"
OUT = SRC / "glim05_vs30grid.u8"

SOURCE_ORDER = ["su", "vb", "ss", "pb", "sm", "sc", "va", "mt", "pa", "vi",
                "wb", "py", "pi", "ev", "nd", "ig"]
CLASSES = ["ev", "ig", "mt", "nd", "pa", "pb", "pi", "py", "sc", "sm", "ss",
           "su", "va", "vb", "vi", "wb"]
WEST, NORTH, CELL, W, H = -180.0, 84.0, 1.0 / 120.0, 43200, 16800


def main():
    SRC.mkdir(parents=True, exist_ok=True)
    if not ZIP.exists():
        req = urllib.request.Request(URL, headers={"User-Agent": "seismira-data"})
        with urllib.request.urlopen(req, timeout=120) as r:
            ZIP.write_bytes(r.read())
    digest = hashlib.sha256(ZIP.read_bytes()).hexdigest()
    if ZIP_SHA256 and digest != ZIP_SHA256:
        sys.exit(f"{ZIP}: SHA256 uyuşmuyor ({digest})")
    print(f"sha256 {digest}")
    z = zipfile.ZipFile(ZIP)
    names = z.read("Classnames.txt").decode().splitlines()[1:]
    order = [line.split(";")[3].strip('"') for line in names if line.strip()]
    if order != SOURCE_ORDER:
        sys.exit(f"Classnames.txt beklenen sırada değil: {order}")
    text = z.read("glim_wgs84_0point5deg.txt.asc").decode()
    lines = text.splitlines()
    hdr = {l.split()[0].lower(): float(l.split()[1]) for l in lines[:6]}
    assert (hdr["ncols"], hdr["nrows"], hdr["cellsize"]) == (720, 360, 0.5), hdr
    assert (hdr["xllcorner"], hdr["yllcorner"]) == (-180, -90), hdr
    grid = np.loadtxt(io.StringIO("\n".join(lines[6:])), dtype=np.int32)
    assert grid.shape == (360, 720)
    # Kaynak değer (1–16) → bizim kod (1–16); NODATA → 0.
    lut = np.zeros(17, dtype=np.uint8)
    for k, name in enumerate(SOURCE_ORDER, start=1):
        lut[k] = CLASSES.index(name) + 1
    src = np.where(grid > 0, lut[np.clip(grid, 0, 16)], 0).astype(np.uint8)
    # Kıyı boşluğu: 0,5° hücresi denizde kalan kara (Vs30'da kara) hücreleri
    # litolojisiz kalıyordu (Noto: heyelan toplamı ×0,58). Veri yok
    # hücreleri en yakın sınıflı 0,5° hücrenin sınıfını alır; su / kara
    # ayrımı yine Vs30 maskesinden (`seismira-data lith`). Boylamda sarmal
    # için ızgara iki yandan genişletilip doldurulur.
    pad = 60
    wide = np.concatenate([src[:, -pad:], src, src[:, :pad]], axis=1)
    _, (iy, ix) = ndimage.distance_transform_edt(wide == 0, return_indices=True)
    src = wide[iy, ix][:, pad:-pad]
    # 30″ hücre merkezleri → 0,5° hücre (satır 0 = 90° K).
    lat = NORTH - (np.arange(H) + 0.5) * CELL
    lon = WEST + (np.arange(W) + 0.5) * CELL
    rows = np.clip(((90.0 - lat) / 0.5).astype(np.int64), 0, 359)
    cols = np.clip(((lon + 180.0) / 0.5).astype(np.int64), 0, 719)
    out = src[rows][:, cols]
    out.tofile(OUT)
    counts = np.bincount(out.ravel(), minlength=17)
    print(f"→ {OUT.relative_to(ROOT)} ({W}×{H}); veri yok {counts[0]} hücre")
    for k, n in enumerate(CLASSES, start=1):
        print(f"  {n}: {counts[k]}")


if __name__ == "__main__":
    main()
