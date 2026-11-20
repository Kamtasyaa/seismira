#!/usr/bin/env python3
"""USGS Slab2 derinlik ızgaralarını uygulamanın okuduğu `.sslb` biçimine çevirir.

Kaynak: Hayes, G. (2018). Slab2 - A Comprehensive Subduction Zone Geometry
Model. U.S. Geological Survey data release, doi:10.5066/F7PV6JNV.
Arşiv (Slab2Distribute_Mar2018.tar.gz) ScienceBase'den TARAYICIYLA indirilir
(betik indirmeleri 403 ile engelleniyor) ve data/raw/slab2/ altına konur.

Yalnızca derinlik (`*_slab2_dep_*.grd`) saklanır. Eğim ve doğrultu uygulamada
derinliğin merkezi farkından türetilir: Slab2'nin kendi dip / str ızgaraları
tam olarak bu türevdir (27 zonun 26'sında fark < 0,01°, Sumatra-Java'da
medyan 0,2° / 0,8°; ölçüm 2026-11-20, `--check`).

Gereken paketler: numpy, scipy, h5py (.grd dosyaları NetCDF-4 / HDF5).
Çalıştırma (depo kökünden):
    .venv/bin/python scripts/gen_slab2.py           # data/slab2/slab2.sslb
    .venv/bin/python scripts/gen_slab2.py --check   # türev doğrulaması

Biçim (little-endian):
    "SSLB"  u32 sürüm (1)  u32 zon sayısı  f32 derinlik birimi (km)
    her zon: 4 bayt kod (ASCII, sıfırla doldurulmuş), f64 batı boylamı
             (ilk sütunun düğümü, derece; 180'i aşabilir), f64 güney enlemi
             (ilk satırın düğümü), f64 adım (derece), u32 genişlik,
             u32 yükseklik, u64 konum, u32 uzunluk, 4 bayt boşluk
    veri: her zon için zstd çerçevesi, genişlik × yükseklik adet u16,
          satırlar güneyden kuzeye. Değer: alt 15 bit derinlik / birim;
          bit 15 = "levha dışı" (değer en yakın levha hücresinden
          kopyalanmıştır; yay önü / yay ardı ayrımında kullanılır).

Her zon PAD derece genişletilir; levha dışı hücreler en yakın levha
hücresinin derinliğiyle doldurulur (indis uzayında öklid uzaklığı).

Ayrıca test için data/test/slab2_ref.csv yazılır: her zondan rastgele
seçilmiş levha düğümlerinde ham derinlik, eğim ve doğrultu.
"""

import argparse
import hashlib
import io
import pathlib
import struct
import sys
import tarfile
from compression import zstd

import h5py
import numpy as np
from scipy import ndimage

ROOT = pathlib.Path(__file__).resolve().parent.parent
RAW = ROOT / "data" / "raw" / "slab2" / "Slab2Distribute_Mar2018.tar.gz"
SHA256 = "3f53a835bfff6196eb89912deed131855fc525dc5e1332f2c5459b1e3b9f7b1d"
OUT = ROOT / "data" / "slab2" / "slab2.sslb"
REF = ROOT / "data" / "test" / "slab2_ref.csv"

VERSION = 1
UNIT_KM = 0.05  # 15 bit → en çok 1638 km
PAD_DEG = 3.0
OUTSIDE = 0x8000
HEADER = struct.Struct("<4sIIf")
ZONE = struct.Struct("<4sdddIIQI4x")
REF_PER_ZONE = 12
# Slab2'nin eğim / doğrultu türevinde kullandığı yarıçap (--check ile
# doğrulandı; 6371,0088 ile fark < 0,01°).
EARTH_R_KM = 6371.0


def sha256(path: pathlib.Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def read_grids(kind: str) -> dict[str, tuple[np.ndarray, np.ndarray, np.ndarray]]:
    """Arşivden `kind` (dep/dip/str) ızgaralarını okur: kod → (x, y, z)."""
    out = {}
    with tarfile.open(RAW, "r:gz") as tar:
        for m in tar.getmembers():
            name = pathlib.PurePosixPath(m.name).name
            if not (m.isfile() and name.endswith(".grd") and f"_slab2_{kind}_" in name):
                continue
            data = tar.extractfile(m).read()
            with h5py.File(io.BytesIO(data), "r") as h:
                out[name[:3]] = (h["x"][:], h["y"][:], h["z"][:])
    return dict(sorted(out.items()))


def derived_dip_strike(x, y, depth):
    """Merkezi farkla eğim ve doğrultu (Slab2 uzlaşımı: eğim doğrultunun
    sağına). Uygulamadaki türev (`seismira-core/src/slab.rs`) ile aynı."""
    step = np.deg2rad(x[1] - x[0])
    dx = step * EARTH_R_KM * np.cos(np.deg2rad(y))[:, None]
    dy = step * EARTH_R_KM
    gx = np.full_like(depth, np.nan)
    gy = np.full_like(depth, np.nan)
    gx[:, 1:-1] = (depth[:, 2:] - depth[:, :-2]) / (2 * dx)
    gy[1:-1, :] = (depth[2:, :] - depth[:-2, :]) / (2 * dy)
    dip = np.degrees(np.arctan(np.hypot(gx, gy)))
    strike = (np.degrees(np.arctan2(gx, gy)) - 90.0) % 360.0
    return dip, strike


def check() -> int:
    dep, dip, st = read_grids("dep"), read_grids("dip"), read_grids("str")
    for code, (x, y, z) in dep.items():
        cdip, cst = derived_dip_strike(x, y, -z.astype(np.float64))
        d, s = dip[code][2], st[code][2]
        m = np.isfinite(cdip) & np.isfinite(d) & (d > 3.0)
        e_d = np.abs(cdip[m] - d[m])
        e_s = np.abs((cst[m] - s[m] + 180.0) % 360.0 - 180.0)
        print(
            f"{code}: {m.sum():6d} düğüm, eğim farkı medyan {np.median(e_d):.2f}° "
            f"p95 {np.percentile(e_d, 95):.2f}°, doğrultu farkı medyan "
            f"{np.median(e_s):.2f}° p95 {np.percentile(e_s, 95):.2f}°"
        )
    return 0


def encode_zone(x, y, z):
    step = float(x[1] - x[0])
    assert np.allclose(np.diff(x), step) and np.allclose(np.diff(y), step), "düzensiz ızgara"
    assert y[0] < y[-1], "satırlar güneyden kuzeye olmalı"
    pad = int(round(PAD_DEG / step))
    depth = np.clip(-z.astype(np.float64), 0.0, None)
    h, w = depth.shape
    full = np.full((h + 2 * pad, w + 2 * pad), np.nan)
    full[pad : pad + h, pad : pad + w] = depth
    inside = np.isfinite(full)
    # En yakın levha hücresinin indisleri.
    _, (iy, ix) = ndimage.distance_transform_edt(~inside, return_indices=True)
    filled = full[iy, ix]
    q = np.rint(filled / UNIT_KM).astype(np.int64)
    assert q.max() < OUTSIDE
    code = q.astype(np.uint16) | np.where(inside, 0, OUTSIDE).astype(np.uint16)
    west = float(x[0]) - pad * step
    south = float(y[0]) - pad * step
    return west, south, step, code


def write_ref(dep, dip, st) -> None:
    rng = np.random.default_rng(20261120)
    rows = []
    for code, (x, y, z) in dep.items():
        d, s = dip[code][2], st[code][2]
        ok = np.isfinite(z) & np.isfinite(d) & np.isfinite(s)
        # Kenar düğümleri hariç (türev iki komşu ister).
        ok[[0, -1], :] = False
        ok[:, [0, -1]] = False
        nb = np.isfinite(z)
        ok[1:-1, 1:-1] &= nb[:-2, 1:-1] & nb[2:, 1:-1] & nb[1:-1, :-2] & nb[1:-1, 2:]
        idx = np.argwhere(ok)
        for r, c in idx[rng.choice(len(idx), size=min(REF_PER_ZONE, len(idx)), replace=False)]:
            rows.append(
                (code, float(x[c]), float(y[r]), -float(z[r, c]), float(d[r, c]), float(s[r, c]))
            )
    REF.parent.mkdir(parents=True, exist_ok=True)
    with REF.open("w") as f:
        f.write("# Slab2 ham düğüm değerleri (scripts/gen_slab2.py): zon, boylam, enlem, derinlik km, eğim, doğrultu\n")
        f.write("zone,lon,lat,depth_km,dip,strike\n")
        for row in rows:
            f.write("{},{:.4f},{:.4f},{:.4f},{:.4f},{:.4f}\n".format(*row))
    print(f"{REF.relative_to(ROOT)}: {len(rows)} nokta")


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true", help="türetilen eğim/doğrultuyu Slab2 ile karşılaştır")
    args = ap.parse_args()
    if not RAW.exists():
        print(
            f"{RAW.relative_to(ROOT)} yok. https://doi.org/10.5066/F7PV6JNV sayfasından "
            "Slab2Distribute_Mar2018.tar.gz dosyasını tarayıcıyla indirip oraya koyun.",
            file=sys.stderr,
        )
        return 1
    got = sha256(RAW)
    if got != SHA256:
        print(f"SHA256 uyuşmuyor: {got}", file=sys.stderr)
        return 1
    if args.check:
        return check()

    dep = read_grids("dep")
    zones = [(code, *encode_zone(x, y, z)) for code, (x, y, z) in dep.items()]
    frames = [zstd.compress(c.astype("<u2").tobytes(), level=19) for *_, c in zones]
    offset = HEADER.size + ZONE.size * len(zones)
    out = io.BytesIO()
    out.write(HEADER.pack(b"SSLB", VERSION, len(zones), UNIT_KM))
    for (code, west, south, step, c), frame in zip(zones, frames):
        h, w = c.shape
        out.write(ZONE.pack(code.encode().ljust(4, b"\0"), west, south, step, w, h, offset, len(frame)))
        offset += len(frame)
    for frame in frames:
        out.write(frame)
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_bytes(out.getvalue())
    print(f"{OUT.relative_to(ROOT)}: {len(zones)} zon, {OUT.stat().st_size / 1e6:.2f} MB")
    write_ref(dep, read_grids("dip"), read_grids("str"))
    return 0


if __name__ == "__main__":
    sys.exit(main())
