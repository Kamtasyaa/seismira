#!/usr/bin/env python3
"""Sarsıntı süresi modeli için referans değerler üretir (yalnızca test).

Referans uygulama: pygmm 0.9.1 (Kottke, MIT lisansı), Afshari & Stewart
(2016) D5-95. Havza terimi yok (Z1.0 verilmez; CY14 ile tutarlı).

Çalıştırma (depo kökünden):
    .venv/bin/python scripts/gen_duration_ref.py

Çıktı: data/test/as16_ref.csv (mw, rrup, vs30, mech, ln_d595, sigma)
"""

import csv
import itertools
import pathlib

import numpy as np
import pygmm

ROOT = pathlib.Path(__file__).resolve().parent.parent
OUT = ROOT / "data" / "test" / "as16_ref.csv"

MW = [4.0, 5.0, 5.2, 5.5, 6.0, 6.5, 7.0, 7.4, 7.9]
RRUP = [1.0, 10.0, 30.0, 50.0, 100.0, 200.0]
VS30 = [200.0, 300.0, 400.0, 600.0, 1000.0]
MECH = ["SS", "RS", "NS"]


def main() -> int:
    rows = []
    for mw, r, v, mech in itertools.product(MW, RRUP, VS30, MECH):
        s = pygmm.Scenario(mag=mw, dist_rup=r, v_s30=v, mechanism=mech)
        m = pygmm.AfshariStewart2016(s)
        d = float(np.asarray(m.duration.D_5t95))
        sd = float(np.asarray(m.std_err.D_5t95))
        rows.append((mw, r, v, mech, np.log(d), sd))
    with OUT.open("w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["mw", "rrup", "vs30", "mech", "ln_d595", "sigma"])
        for row in rows:
            w.writerow([row[0], row[1], row[2], row[3], f"{row[4]:.8f}", f"{row[5]:.6f}"])
    print(f"{len(rows)} satır → {OUT}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
