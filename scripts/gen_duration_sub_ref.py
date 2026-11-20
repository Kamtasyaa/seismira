#!/usr/bin/env python3
"""Bahrampouri vd. (2021) dalma-batma süre modeli için OpenQuake referans
değerleri (yalnızca test).

Kural (CLAUDE.md Bölüm 7): OpenQuake KODU KOPYALANMAZ, yalnızca ÇIKTISI
karşılaştırılır. Bizim uygulama makaleden (Eş. 7–12, Tablo 1–2) yazıldı;
bu betik bağımsız uygulamanın sayılarını üretir.

Seçimler bizimkiyle aynı: z1pt0 = −999 (OpenQuake bunu Vs30'dan tahmin
edilen Z1 sayar, δZ1 = 0). Fay tipi kayma açısıyla: ters 90°, doğrultu
atımlı 0°, normal −90°.

Kurulum: scripts/gen_psbah20_oq_ref.py başlığındaki gibi (OpenQuake 3.26.2).
Çalıştırma: <oq-venv>/bin/python scripts/gen_duration_sub_ref.py
Çıktı: data/test/duration_sub_ref.csv
"""

import csv
import itertools
import pathlib
import sys
import types

fiona = types.ModuleType("fiona")
fiona_crs = types.ModuleType("fiona.crs")
fiona_crs.CRS = object
fiona.crs = fiona_crs
sys.modules.setdefault("fiona", fiona)
sys.modules.setdefault("fiona.crs", fiona_crs)

import numpy as np  # noqa: E402
from openquake.hazardlib.contexts import simple_cmaker  # noqa: E402
from openquake.hazardlib.gsim.bahrampouri_2021_duration import (  # noqa: E402
    BahrampouriEtAldm2021SInter,
    BahrampouriEtAldm2021SSlab,
)

ROOT = pathlib.Path(__file__).resolve().parent.parent
OUT = ROOT / "data" / "test" / "duration_sub_ref.csv"

MAGS = [4.0, 5.0, 5.5, 6.5, 7.5, 8.0, 8.5, 9.0]
DISTS = [5.0, 50.0, 150.0, 200.0, 300.0, 500.0]
VS30S = [150.0, 300.0, 600.0, 1200.0]
MECHS = {"RS": 90.0, "SS": 0.0, "NS": -90.0}


def run(gsim, mech):
    rows = list(itertools.product(MAGS, DISTS, VS30S))
    cm = simple_cmaker([gsim], ["RSD595"])
    ctx = cm.new_ctx(len(rows))
    ctx.mag = np.array([r[0] for r in rows])
    ctx.rrup = np.array([r[1] for r in rows])
    ctx.vs30 = np.array([r[2] for r in rows])
    ctx.z1pt0 = np.full(len(rows), -999.0)
    ctx.rake = np.full(len(rows), MECHS[mech])
    mean, sig, _tau, _phi = cm.get_mean_stds([ctx])
    return rows, mean[0][0], sig[0][0]


def main() -> int:
    out = []
    for event, cls, mechs in (
        ("interface", BahrampouriEtAldm2021SInter, ["RS"]),
        ("intraslab", BahrampouriEtAldm2021SSlab, ["RS", "SS", "NS"]),
    ):
        for mech in mechs:
            rows, mean, sig = run(cls(), mech)
            for k, (m, r, v) in enumerate(rows):
                out.append(
                    dict(event=event, mech=mech, mag=m, rrup=r, vs30=v,
                         ln_d595=f"{mean[k]:.10g}", sigma=f"{sig[k]:.10g}")
                )
    with open(OUT, "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=list(out[0].keys()))
        w.writeheader()
        w.writerows(out)
    print(f"{len(out)} senaryo → {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
