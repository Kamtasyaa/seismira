#!/usr/bin/env python3
"""Yer hareketi modelleri için referans değerler üretir (yalnızca test).

Referans uygulama: pygmm (Kottke, MIT lisansı), CY14 için makalenin
katsayı tablosunu kullanır. Bizim Rust uygulamamız nshmp-lib'in (USGS, CC0)
katsayılarını kullanır; ikisi %1 içinde örtüşmelidir (CLAUDE.md Bölüm 14).

Kurulum ve çalıştırma (depo kökünden):
    .venv/bin/pip install pygmm==0.9.1
    .venv/bin/python scripts/gen_gmpe_ref.py

Çıktı: data/test/cy14_ref.csv
"""

import csv
import itertools
import pathlib

import numpy as np
import pygmm

ROOT = pathlib.Path(__file__).resolve().parent.parent
OUT = ROOT / "data" / "test"

# Bizim periyot seçimimiz (Bölüm 7.4): 0.3, 0.6 (ara değer), 1.0 s; ayrıca
# ara değeri doğrulamak için tablo periyotları 0.5 ve 0.75.
PERIODS = [0.3, 0.5, 0.75, 1.0]


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    rows = []
    mags = [4.5, 5.5, 6.5, 7.5, 8.2]
    dists = [1.0, 5.0, 20.0, 60.0, 150.0, 300.0]
    cases = [
        # (mekanizma, dip, ztor, rx işareti)
        ("SS", 90.0, 0.0, 0),
        ("RS", 40.0, 2.0, 1),
        ("RS", 40.0, 2.0, -1),
        ("NS", 55.0, 5.0, 1),
    ]
    for mag, rrup, (mech, dip, ztor, side), vs30 in itertools.product(
        mags, dists, cases, [760.0, 400.0, 250.0]
    ):
        rjb = float(np.sqrt(max(rrup**2 - ztor**2, 0.0)))
        rx = side * rjb if side != 0 else 0.0
        on_hw = side > 0
        s = pygmm.Scenario(
            mag=mag,
            dist_rup=rrup,
            dist_jb=rjb,
            dist_x=rx,
            dip=dip,
            depth_tor=ztor,
            mechanism=mech,
            v_s30=vs30,
            on_hanging_wall=on_hw,
            vs_source="inferred",
            region="california",
        )
        m = pygmm.ChiouYoungs2014(s)
        row = dict(
            mag=mag, rrup=rrup, rjb=rjb, rx=rx, ztor=ztor, dip=dip, mech=mech,
            vs30=vs30, hw=int(on_hw),
            ln_pga=float(np.log(m.pga)), sd_pga=float(m.ln_std_pga),
            ln_pgv=float(np.log(m.pgv)), sd_pgv=float(m.ln_std_pgv),
        )
        psa = np.atleast_1d(m.interp_spec_accels(PERIODS))
        sds = np.atleast_1d(m.interp_ln_stds(PERIODS))
        for t, v, sd in zip(PERIODS, psa, sds):
            row[f"ln_sa{t}"] = float(np.log(v))
            row[f"sd_sa{t}"] = float(sd)
        rows.append(row)

    with open(OUT / "cy14_ref.csv", "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=list(rows[0].keys()))
        w.writeheader()
        w.writerows(rows)
    print(f"{len(rows)} senaryo, pygmm {pygmm.__version__} → {OUT / 'cy14_ref.csv'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
