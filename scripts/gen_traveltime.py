#!/usr/bin/env python3
"""ak135 ilk P ve ilk S varış süresi tablosunu üretir (tek seferlik).

Model: ak135 (Kennett, Engdahl & Buland 1995, GJI 122, 108-124).
Hesap: ObsPy TauP (TauPyModel("ak135")), faz listeleri "ttp" (P türü tüm
fazlar) ve "tts" (S türü tüm fazlar); her noktada en erken varış alınır.

Kurulum ve çalıştırma (depo kökünden):
    python3 -m venv .venv
    .venv/bin/pip install obspy==1.5.1
    .venv/bin/python scripts/gen_traveltime.py

Çıktılar (depoya girer, küçük):
    data/traveltime/ak135.ttt        ikili tablo (biçim: seismira-core/src/traveltime.rs)
    data/traveltime/ak135_check.csv  ızgara dışı rastgele noktalarda TauP
                                     değerleri; Rust testleri ara değer
                                     hatasını bununla ölçer (ölçüt < 1 s).
"""

import multiprocessing as mp
import pathlib
import random
import struct
import sys

import numpy as np

ROOT = pathlib.Path(__file__).resolve().parent.parent
OUT = ROOT / "data" / "traveltime"

# Mesafe ekseni (derece): kaynağa yakın eğrilik büyük, ızgara sık.
DIST = np.concatenate([
    np.arange(0.0, 2.0, 0.02),
    np.arange(2.0, 20.0, 0.1),
    np.arange(20.0, 100.0 + 1e-9, 0.25),
])
# Derinlik ekseni (km).
DEPTH = np.concatenate([
    np.arange(0.0, 40.0, 1.0),
    np.arange(40.0, 100.0, 5.0),
    np.arange(100.0, 700.0 + 1e-9, 10.0),
])

_model = None


def _init():
    global _model
    from obspy.taup import TauPyModel
    _model = TauPyModel(model="ak135")


def first_arrivals(depth_km: float, dist_deg: float) -> tuple[float, float]:
    """(ilk P, ilk S) saniye. Kaynak noktasının kendisinde 0."""
    if dist_deg == 0.0 and depth_km == 0.0:
        return 0.0, 0.0
    out = []
    for phases in (["ttp"], ["tts"]):
        arr = _model.get_travel_times(
            source_depth_in_km=depth_km,
            distance_in_degree=dist_deg,
            phase_list=phases,
        )
        out.append(min(a.time for a in arr) if arr else float("nan"))
    return out[0], out[1]


def row(depth_km: float):
    return [first_arrivals(depth_km, float(d)) for d in DIST]


def check_point(args):
    depth_km, dist_deg = args
    p, s = first_arrivals(depth_km, dist_deg)
    return depth_km, dist_deg, p, s


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    print(f"ızgara: {len(DIST)} mesafe × {len(DEPTH)} derinlik", flush=True)
    with mp.Pool(initializer=_init) as pool:
        rows = []
        for i, r in enumerate(pool.imap(row, [float(h) for h in DEPTH])):
            rows.append(r)
            if i % 10 == 0:
                print(f"  derinlik {DEPTH[i]:.0f} km", flush=True)
        tp = np.array([[a[0] for a in r] for r in rows], dtype=np.float32)
        ts = np.array([[a[1] for a in r] for r in rows], dtype=np.float32)
        if np.isnan(tp).any() or np.isnan(ts).any():
            print("HATA: bazı noktalarda varış yok", file=sys.stderr)
            return 1

        rng = random.Random(135)
        pts = [(rng.uniform(0, 700), rng.uniform(0, 100)) for _ in range(3000)]
        # Kaynağa yakın bölgeyi ayrıca yokla (en zor bölge).
        pts += [(rng.uniform(0, 60), rng.uniform(0, 3)) for _ in range(1000)]
        checks = pool.map(check_point, pts)

    with open(OUT / "ak135.ttt", "wb") as f:
        f.write(b"STTT")
        f.write(struct.pack("<III", 1, len(DIST), len(DEPTH)))
        f.write(DIST.astype("<f4").tobytes())
        f.write(DEPTH.astype("<f4").tobytes())
        f.write(tp.astype("<f4").tobytes())  # [derinlik][mesafe]
        f.write(ts.astype("<f4").tobytes())
    with open(OUT / "ak135_check.csv", "w") as f:
        f.write("depth_km,dist_deg,p_s,s_s\n")
        for h, d, p, s in checks:
            f.write(f"{h:.4f},{d:.4f},{p:.4f},{s:.4f}\n")

    import obspy
    print(f"tamam (ObsPy {obspy.__version__}): {OUT}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
