#!/usr/bin/env python3
"""GLiM (Hartmann & Moosdorf 2012) litoloji vektörü → Vs30 ızgarası (30″).

Kullanım (depo kökünden; .venv'de pyogrio, shapely, pyproj, rasterio):
    .venv/bin/python -I scripts/gen_glim.py

Girdi: data/raw/glim/LiMW_GIS_2015.gdb.zip (Hamburg Üniversitesi GLiM
sayfasındaki Dropbox bağlantısı, "LiMW_GIS 2015.gdb.zip", 1 138 691 332 B,
SHA256 aşağıda), açılmış hali data/raw/glim/unz/. Katman GLiM_export,
1 235 259 çokgen, World Eckert IV. `xx` alanı üst sınıf (16 sınıf; Jessee
2018'in kullandığı düzey).

LİSANS: vektör sürümün lisans metni BULUNAMADI (docs/veri.md); izin
istendi. Çıktı yalnızca yerelde geliştirme/doğrulama içindir, pakete ve
depoya girmez.

Rasterleme: hücre MERKEZİ çokgenin içindeyse o sınıf (rasterio
all_touched=False; USGS'in kategorik katmanlar için kullandığı en yakın
komşu örneklemesine karşılık gelir). Izgara: 180°B, 84°K köşeli,
43200 × 16800, 30″ (Vs30 ızgarası).

Çıktı: data/raw/glim/glim_vs30grid.u8, ham u8, satırlar kuzeyden güneye.
Kod = seismira-core `jessee2018::GLIM_CLASSES` sırası (1 ev … 16 wb),
0 = çokgen yok.
"""

import hashlib
import pathlib
import sys

import numpy as np
import pyogrio.raw
import pyproj
import shapely
from rasterio import features
from rasterio.transform import from_origin

ROOT = pathlib.Path(__file__).resolve().parent.parent
SRC = ROOT / "data" / "raw" / "glim"
ZIP = SRC / "LiMW_GIS_2015.gdb.zip"
ZIP_SHA256 = "895be0cde43f68d1fe6a24dafafbde715d9c9893adfea28916a9a69f88cc320c"
GDB = SRC / "unz" / "LiMW_GIS 2015.gdb"
OUT = SRC / "glim_vs30grid.u8"

CLASSES = ["ev", "ig", "mt", "nd", "pa", "pb", "pi", "py", "sc", "sm", "ss",
           "su", "va", "vb", "vi", "wb"]
WEST, NORTH, CELL, W, H = -180.0, 84.0, 1.0 / 120.0, 43200, 16800
LAYER = "GLiM_export"
BATCH = 50_000  # çokgen/parça; tepe bellek ~2–3 GB


def sha256(p):
    h = hashlib.sha256()
    with open(p, "rb") as f:
        for b in iter(lambda: f.read(1 << 22), b""):
            h.update(b)
    return h.hexdigest()


def main():
    if sha256(ZIP) != ZIP_SHA256:
        sys.exit(f"{ZIP}: SHA256 uyuşmuyor")
    if not GDB.exists():
        sys.exit(f"{GDB} yok: önce `unzip {ZIP.name} -d unz` (data/raw/glim içinde)")
    info = pyogrio.read_info(str(GDB), layer=LAYER)
    n = info["features"]
    tr = pyproj.Transformer.from_crs(info["crs"], "EPSG:4326", always_xy=True)
    out = np.zeros((H, W), dtype=np.uint8)
    unknown = 0
    # Bellek: 1,2 milyon çokgen tek seferde okunursa 16 GB RAM yetmiyor
    # (2026-11-22 OOM). Parça parça okunur, her parça doğrudan `out`a yakılır.
    # Çokgenler örtüşmediği için sıra sonucu değiştirmez.
    for k0 in range(0, n, BATCH):
        _, _, geom, (xx,) = pyogrio.raw.read(
            str(GDB), layer=LAYER, columns=["xx"], skip_features=k0, max_features=BATCH
        )
        codes = np.array([CLASSES.index(x) + 1 if x in CLASSES else 0 for x in xx],
                         dtype=np.uint8)
        unknown += int((codes == 0).sum())
        geoms = shapely.from_wkb(geom)
        del geom
        geoms = shapely.transform(
            geoms, lambda c: np.column_stack(tr.transform(c[:, 0], c[:, 1]))
        )
        keep = codes > 0
        features.rasterize(
            zip(geoms[keep], codes[keep]),
            out=out,
            transform=from_origin(WEST, NORTH, CELL, CELL),
            all_touched=False,
        )
        del geoms
        print(f"  {min(n, k0 + BATCH)}/{n} çokgen", flush=True)
    if unknown:
        print(f"bilinmeyen sınıf: {unknown} çokgen (0 kalır)")
    out.tofile(OUT)
    counts = np.bincount(out.ravel(), minlength=17)
    print("hücre sayıları:", {(CLASSES[i - 1] if i else "yok"): int(c) for i, c in enumerate(counts) if c})
    print(f"{OUT}: {OUT.stat().st_size} B")


if __name__ == "__main__":
    main()
