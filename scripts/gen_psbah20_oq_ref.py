#!/usr/bin/env python3
"""Parker vd. (2020) bölgesel modeli için OpenQuake referans değerleri (yalnızca test).

pygmm (nshmp-lib) yalnızca küresel, Alaska ve Cascadia bölgelerini içerir.
Japonya, Güney Amerika, Orta Amerika ve Tayvan için bağımsız tek uygulama
OpenQuake hazardlib'dir (AGPL). Kural (CLAUDE.md Bölüm 7): OpenQuake KODU
KOPYALANMAZ, yalnızca ÇIKTISI karşılaştırılır. Bu betik OpenQuake'i
çalıştırır ve sonuçları yazar; depoda yalnızca sayılar durur.

Bilinen fark: OpenQuake arayüz c6 katsayısını PEER ekinden alır (ek, raporun
c6 ≤ c4 kısıtını ihlal eder); biz nshmp-lib'i izleriz (docs/bilim.md 7.4).
c6 yalnızca M > Mc'de devreye girdiğinden arayüz senaryoları M ≤ Mc ile
sınırlıdır.

Kurulum (Python 3.14'te OpenQuake'in tam kurulumu derlenemiyor; yalnızca
hesap için gerekenler):
    python3 -m venv /tmp/oq && /tmp/oq/bin/pip install --no-deps openquake.engine==3.26.2
    /tmp/oq/bin/pip install numpy scipy shapely h5py toml pandas pyproj psutil \\
        decorator requests numba h3 alpha_shapes pyzmq
    /tmp/oq/bin/python scripts/gen_psbah20_oq_ref.py
fiona (coğrafi dosya okuma) Parker modeli için gerekmez; içe aktarma zinciri
istediği için boş bir modülle karşılanır.

Çıktı: data/test/psbah20_regional_ref.csv
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
import openquake.baselib  # noqa: E402
from openquake.hazardlib.contexts import simple_cmaker  # noqa: E402
from openquake.hazardlib.gsim.parker_2020 import (  # noqa: E402
    ParkerEtAl2020SInter,
    ParkerEtAl2020SSlab,
)

ROOT = pathlib.Path(__file__).resolve().parent.parent
OUT = ROOT / "data" / "test" / "psbah20_regional_ref.csv"

# Bizim bölge adı → (OpenQuake region, saturation_region, arayüz Mc).
REGIONS = {
    "global": (None, None, 7.9),
    "alaska": ("AK", None, 8.6),
    "aleutian": ("AK", "Aleutian", 8.0),
    "cascadia": ("Cascadia", None, 7.7),
    "cam_n": ("CAM", "CAM_N", 7.4),
    "cam_s": ("CAM", "CAM_S", 7.4),
    "japan_pac": ("JP", "JP_Pac", 8.5),
    "japan_phi": ("JP", "JP_Phi", 7.7),
    "sa_n": ("SA", "SA_N", 8.5),
    "sa_s": ("SA", "SA_S", 8.6),
    "taiwan_e": ("TW", "TW_E", 7.1),
    "taiwan_w": ("TW", "TW_W", 7.1),
}
IMTS = ["PGA", "PGV", "SA(0.3)", "SA(1.0)"]
COLS = ["pga", "pgv", "sa0.3", "sa1.0"]
DISTS = [10.0, 40.0, 100.0, 250.0, 600.0]
VS30S = [760.0, 300.0, 180.0]


def run(gsim, mags, zhyp):
    rows = list(itertools.product(mags, DISTS, VS30S))
    cm = simple_cmaker([gsim], IMTS)
    ctx = cm.new_ctx(len(rows))
    ctx.mag = np.array([r[0] for r in rows])
    ctx.rrup = np.array([r[1] for r in rows])
    ctx.vs30 = np.array([r[2] for r in rows])
    ctx.hypo_depth = np.full(len(rows), zhyp)
    ctx.ztor = np.full(len(rows), max(zhyp - 10.0, 0.0))
    ctx.z2pt5 = np.full(len(rows), -999.0)  # havza terimi yok
    mean, sig, _tau, _phi = cm.get_mean_stds([ctx])
    return rows, mean[0], sig[0]


def main() -> int:
    out = []
    for region, (reg, sat, mc_if) in REGIONS.items():
        cases = [
            ("interface", ParkerEtAl2020SInter, [m for m in (4.5, 5.5, 6.5, 7.0, 7.5, 8.0, 8.4) if m <= mc_if], 25.0),
            ("intraslab", ParkerEtAl2020SSlab, [4.5, 5.5, 6.5, 7.3, 8.0], 60.0),
            ("intraslab", ParkerEtAl2020SSlab, [5.5, 7.0], 15.0),
            ("intraslab", ParkerEtAl2020SSlab, [6.0], 120.0),
        ]
        for event, cls, mags, zhyp in cases:
            gsim = cls(region=reg, saturation_region=sat)
            rows, mean, sig = run(gsim, mags, zhyp)
            for k, (m, r, v) in enumerate(rows):
                row = dict(event=event, region=region, mag=m, rrup=r, zhyp=zhyp, vs30=v)
                for j, c in enumerate(COLS):
                    row[f"ln_{c}"] = f"{mean[j][k]:.10g}"
                    row[f"sd_{c}"] = f"{sig[j][k]:.10g}"
                out.append(row)
    with open(OUT, "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=list(out[0].keys()))
        w.writeheader()
        w.writerows(out)
    print(f"{len(out)} senaryo → {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
