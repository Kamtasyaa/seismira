#!/usr/bin/env python3
"""ETOPO 2022 (1′) + Natural Earth deniz maskesi → ham derinlik ızgarası.

Kullanım (depo kökünden; .venv'de rasterio, pyogrio, shapely):
    .venv/bin/python -I scripts/gen_bathy.py
Sonra: `seismira-data bathy data/raw/etopo/bathy_1m.i16 data/tsunami/bathy.sras`.

Girdi:
- data/raw/etopo/ETOPO_2022_v1_60s_N90W180_surface.tif (NOAA NCEI ETOPO 2022,
  60″, yüzey yüksekliği, EGM2008; CC0-1.0; SHA256 aşağıda). İndirme:
  https://www.ngdc.noaa.gov/mgg/global/relief/ETOPO2022/data/60s/60s_surface_elev_gtif/
- data/raw/natural-earth/v5.1.2/ne_10m_land.shp ve ne_10m_lakes.shp.

Deniz = Natural Earth karası DEĞİL ve gölü DEĞİL (hücre merkezi;
rasterio all_touched=False), sonra 8-komşulu bağlı bileşenlerden yalnızca
DÜNYA OKYANUSU (en büyük bileşen; Marmara dahil) ve KARADENİZ (Azak dahil;
İstanbul Boğazı 1′'de kapalı olduğu için ayrı bileşen) tutulur
(docs/bilim.md 7.10: "yalnızca okyanusa bağlı kıyılar"). Hazar (NE'de kara
da göl de değil, 154 608 hücre), Lut Gölü ve 1′'de kopan küçük kıyı
lagünleri maskelenir.

ETOPO'nun deniz hücresinde verdiği yükseklik ≥ 0 ise (2,8 milyon hücre,
%79'u Antarktika): > 10 m → kara sayılır (buz sahanlığı yüzeyi: ETOPO
"surface" buz yüzeyini verir; ya da kıyı çizgisi farkı / falez), 0–10 m →
1 m derinlikte sığ deniz (kıyı çizgisi farkı).

Çıktı: data/raw/etopo/bathy_1m.i16, ham int16 (küçük endian), 21600 × 10800,
satırlar kuzeyden güneye, 180°B / 90°K köşeli, 1′ hücre. Değer: deniz
hücresinde derinlik (m, ≥ 1), kara / göl / buz sahanlığı -1.
"""

import hashlib
import pathlib
import sys

import numpy as np
from scipy import ndimage
import pyogrio.raw
import rasterio
import shapely
from rasterio import features

ROOT = pathlib.Path(__file__).resolve().parent.parent
TIF = ROOT / "data" / "raw" / "etopo" / "ETOPO_2022_v1_60s_N90W180_surface.tif"
TIF_SHA256 = "9d27d4b8ea8e76977e2988bca667d7c8fa68b927355feffcddd6b4875a7fd08e"
NE = ROOT / "data" / "raw" / "natural-earth" / "v5.1.2"
OUT = ROOT / "data" / "raw" / "etopo" / "bathy_1m.i16"
W, H = 21600, 10800
ICE_SHELF_M = 10.0
BLACK_SEA = (34.0, 43.0)  # Karadeniz ortası (boylam, enlem)


def sha256(p):
    h = hashlib.sha256()
    with open(p, "rb") as f:
        for b in iter(lambda: f.read(1 << 22), b""):
            h.update(b)
    return h.hexdigest()


def mask(shp, transform):
    _, _, geom, _ = pyogrio.raw.read(str(shp), columns=[])
    geoms = shapely.from_wkb(geom)
    return features.rasterize(
        ((g, 1) for g in geoms),
        out_shape=(H, W),
        transform=transform,
        fill=0,
        all_touched=False,
        dtype="uint8",
    )


def main():
    if sha256(TIF) != TIF_SHA256:
        sys.exit(f"{TIF}: SHA256 uyuşmuyor")
    with rasterio.open(TIF) as d:
        assert (d.width, d.height) == (W, H), (d.width, d.height)
        t = d.transform
        assert abs(t.a - 1 / 60) < 1e-12 and t.c == -180.0 and t.f == 90.0, t
        elev = d.read(1)
    land = mask(NE / "ne_10m_land.shp", t)
    land |= mask(NE / "ne_10m_lakes.shp", t)
    sea = land == 0
    high = sea & (elev > ICE_SHELF_M)
    print(f"denizde ETOPO > {ICE_SHELF_M} m (kara sayıldı): {int(high.sum())}")
    sea &= ~high
    print(f"deniz hücresi: {int(sea.sum())} / {W * H}")
    shallow = sea & (elev >= 0)
    print(f"ETOPO'da 0–{ICE_SHELF_M} m olan deniz hücresi (1 m): {int(shallow.sum())}")
    lab, _ = ndimage.label(sea, structure=np.ones((3, 3), dtype=bool))
    sizes = np.bincount(lab.ravel())
    sizes[0] = 0
    ocean = int(np.argmax(sizes))
    black = int(lab[int((90 - BLACK_SEA[1]) * 60), int((BLACK_SEA[0] + 180) * 60)])
    assert black not in (0, ocean), "Karadeniz noktası ayrı bir deniz bileşeninde değil"
    keep = (lab == ocean) | (lab == black)
    print(
        f"bileşen: okyanus {sizes[ocean]}, Karadeniz {sizes[black]}; "
        f"maskelenen {int((sea & ~keep).sum())} hücre (Hazar, göller, lagünler)"
    )
    sea &= keep
    del lab
    wet_land = (~sea) & (elev < 0)
    print(f"ETOPO'da < 0 olan kara/göl hücresi (kara sayıldı): {int(wet_land.sum())}")
    depth = np.where(sea, np.clip(-elev, 1, 32000), -1).astype("<i2")
    del elev, land
    depth.tofile(OUT)
    print(f"{OUT}: {OUT.stat().st_size} B; en derin {int(depth.max())} m")


if __name__ == "__main__":
    main()
