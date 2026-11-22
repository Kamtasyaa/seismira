#!/usr/bin/env python3
"""Fan vd. (2017) yıllık ortalama yeraltı su seviyesi (WTD) → Vs30 ızgarası.

Kullanım (depo kökünden, h5py + numpy gerekir; .venv'de var):
    .venv/bin/python -I scripts/gen_fan_wtd.py
    cargo run --release -p seismira-data -- wtd \\
        data/raw/fan_wtd/wtd_vs30grid.i16 data/vs30/vs30.sras data/ground/wtd.sras

LİSANS: dosyalarda "can be used freely for research purposes" yazıyor;
yeniden dağıtım izni YOK (veri sorumlusundan izin istendi, docs/veri.md).
İzin gelene kadar çıktı yalnızca yerelde geliştirme/doğrulama içindir,
pakete ve depoya girmez.

Kaynak: http://thredds-gfnl.usc.es/thredds/fileServer/GLOBALWTDFTP/annualmeans/
<KITA>_WTD_annualmean.nc (https çalışmıyor). Izgara 30″, hücre merkezleri
Vs30 ızgarasının (180°B, 84°K köşeli, 43200 × 16800) merkezleriyle aynı.
Dosyada derinlik negatif saklı (−1000…0 m); burada pozitife çevrilir.
Çıktı: ham int16 (küçük-endian), satırlar kuzeyden güneye, değer = WTD
santimetre (0–32767, yani en çok 327,67 m'ye kırpılır), veri yok = -32768.
"""

import hashlib
import pathlib
import sys

import h5py
import numpy as np

ROOT = pathlib.Path(__file__).resolve().parent.parent
SRC = ROOT / "data" / "raw" / "fan_wtd"
OUT = SRC / "wtd_vs30grid.i16"

FILES = {
    "AFRICA": "236c015084e1fab214b000c3b83cae969b3b031f644ba458323d9ede2cc7f301",
    "EURASIA": "de3b5f35bee5fb39ba65d2d37fbbbf9deda2b1e30c8a95c8c74c7ef6e3c7fdda",
    "NAMERICA": "d586a5342939252a0081c15d2a7493d9bca2d08ad84e06132be7ed9f21d0c3a9",
    "OCEANIA": "26921944929d66957c3fb3918ed32bb7b1b19f836416e4fea0f8b20cec0b2051",
    "SAMERICA": "09d2e6cbf5157271e6b2dc1bc670aef14a126fa24e6e73786ffcd04db2a5a55a",
}

WEST, NORTH, CELL, W, H = -180.0, 84.0, 1.0 / 120.0, 43200, 16800
NODATA = -32768


def sha256(path: pathlib.Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def main() -> int:
    grid = np.full((H, W), NODATA, dtype=np.int16)
    for name, digest in FILES.items():
        path = SRC / f"{name}_WTD_annualmean.nc"
        if not path.exists() or sha256(path) != digest:
            print(f"{path.name}: yok ya da SHA256 uyuşmuyor", file=sys.stderr)
            return 1
        with h5py.File(path, "r") as f:
            lat = f["lat"][:].astype(np.float64)
            lon = f["lon"][:].astype(np.float64)
            v = f["WTD"]
            scale = float(v.attrs["scale_factor"][0])
            offset = float(v.attrs["add_offset"][0])
            raw = v[0, :, :]
            mask = f["mask"][:, :] == 1
        # Koordinatlar float32'de birikimli hesaplanmış (son değerlerde 0,09
        # hücreye varan kayma); ızgara düzenli: ilk değer + sabit adım.
        r0 = (NORTH - lat[0]) / CELL - 0.5
        c0 = (lon[0] - WEST) / CELL - 0.5
        assert abs(r0 - round(r0)) < 0.01 and abs(c0 - round(c0)) < 0.01, "ızgara hizasız"
        step_lat = np.sign(lat[1] - lat[0])
        rows = round(r0) - (step_lat * np.arange(lat.size)).astype(np.int64)
        cols = (round(c0) + np.arange(lon.size)) % W
        assert abs((NORTH - lat[-1]) / CELL - 0.5 - rows[-1]) < 0.1, "enlem adımı"
        assert abs((lon[-1] - WEST) / CELL - 0.5 - (round(c0) + lon.size - 1)) < 0.1, "boylam adımı"
        ok_r = (rows >= 0) & (rows < H)
        # Dosyada derinlik negatif saklı (yüzey 0, aşağısı eksi; −1000…0 m).
        wtd_m = -(raw.astype(np.float64) * scale + offset)
        cm = np.clip(np.rint(wtd_m * 100.0), 0, 32767).astype(np.int16)
        cm[~mask] = NODATA
        sub = cm[ok_r, :]
        r = rows[ok_r]
        # Kıtalar çakışırsa yalnızca geçerli değer yazılır.
        for i, rr in enumerate(r):
            line = sub[i]
            good = line != NODATA
            grid[rr, cols[good]] = line[good]
        valid = int(mask.sum())
        print(f"{name}: {raw.shape}, geçerli {valid}, WTD aralığı "
              f"{wtd_m[mask].min():.2f}–{wtd_m[mask].max():.2f} m", flush=True)
    grid.astype("<i2").tofile(OUT)
    print(f"{OUT}: {OUT.stat().st_size / 1e6:.0f} MB, geçerli hücre {(grid != NODATA).sum()}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
