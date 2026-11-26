#!/usr/bin/env python3
"""Akkar, Sandıkkaya & Bommer (2014) katsayı dosyasını üretir:
crates/seismira-core/src/gmpe/asb14.csv.

Kaynaklar (ikisi de birleştirilir, değerler DEĞİŞTİRİLMEZ):
1. ASB14 Rjb modeli, 62 periyot + PGA + PGV: pygmm 0.9.1 (Kottke, MIT)
   `pygmm/data/akkar-sandikkaya-bommer-2014-dist_jb.csv` ("Coefficients
   recieved on 2016-03-17", yazarların elektronik eki). Makalenin Tablo 3
   ve Tablo 4.a'sı (Imperial Spiral kabul edilmiş metni, sha256 9a2fe3a4…)
   ile ortak 22 satırda birebir aynı (bu betik denetler).
2. Bileşen oranı ln(RotD50 / GM_AR): Boore & Kishida (2017), USGS
   esi-shakelib (ABD kamu malı)
   `conversions/imc/data/nga-w2.lmR1M2_D50GMAR.4paper…csv`, "smooth"
   sütunları (ShakeMap'in kullandığı), sha256 cc3c38c5….

Çalıştırma (depo kökünden):
    .venv/bin/python -I scripts/gen_asb14.py
"""

import csv
import hashlib
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
BK17 = (
    ROOT
    / "data/raw/refs/bk17/"
    "nga-w2.lmR1M2_D50GMAR.4paper.RrupMin_0000.RrupMax_0400.Mmin_2.00.Mmax_9.00.csv"
)
PAPER_TXT = ROOT / "data/raw/refs/asb14/asb14.txt"  # pdftotext -layout asb14.pdf
OUT = ROOT / "crates/seismira-core/src/gmpe/asb14.csv"


def pygmm_file() -> pathlib.Path:
    import pygmm

    return pathlib.Path(pygmm.__file__).parent / "data/akkar-sandikkaya-bommer-2014-dist_jb.csv"


def read_asb14(path: pathlib.Path) -> list[dict]:
    lines = path.read_text().splitlines()
    header = lines[2].lstrip("#").split(",")
    rows, seen = [], set()
    for line in lines[3:]:
        v = dict(zip(header, map(float, line.split(","))))
        if v["period"] in seen:  # PGV satırı dosyada iki kez
            continue
        seen.add(v["period"])
        rows.append(v)
    return rows


def check_paper(rows: list[dict]) -> int:
    """Tablo 4.a ile karşılaştırır; denetlenen satır sayısı."""
    by = {r["period"]: r for r in rows}
    txt = PAPER_TXT.read_text().splitlines()
    a = next(i for i, l in enumerate(txt) if "Table 4.a" in l)
    b = next(i for i, l in enumerate(txt) if "Table 4.b" in l)
    keys = ["a_1", "a_3", "a_4", "a_8", "a_9", "b_1", "b_2", "sd_within", "sd_between", "sd_total"]
    n = 0
    for line in txt[a:b]:
        f = line.split()
        if len(f) != 11:
            continue
        p = {"PGA": 0.0, "PGV": -1.0}.get(f[0])
        if p is None:
            try:
                p = float(f[0])
            except ValueError:
                continue
        vals = [float(x.replace(",", ".")) for x in f[1:]]  # metinde "0,39551" yazım hatası
        for k, x in zip(keys, vals):
            if abs(by[p][k] - x) > 1e-9:
                sys.exit(f"Tablo 4.a farkı: T={p} {k} makale {x} pygmm {by[p][k]}")
        n += 1
    return n


def main() -> int:
    src = pygmm_file()
    rows = read_asb14(src)
    n = check_paper(rows)
    if n < 20:
        sys.exit(f"Tablo 4.a'dan yalnızca {n} satır okundu")
    bk = {}
    with BK17.open() as f:
        for r in csv.DictReader(f):
            bk[float(r["per"])] = r
    sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
    with OUT.open("w", newline="") as f:
        f.write(
            "# Akkar, Sandıkkaya & Bommer (2014) Rjb modeli katsayıları; scripts/gen_asb14.py üretti.\n"
            f"# ASB14: pygmm 0.9.1 (MIT) akkar-sandikkaya-bommer-2014-dist_jb.csv sha256 {sha(src)[:16]}…\n"
            "#   (yazarların elektronik eki; makale Tablo 3 ve 4.a ile ortak satırlarda aynı).\n"
            "# bk_*: Boore & Kishida (2017) ln(RotD50/GM_AR), USGS esi-shakelib (kamu malı)\n"
            f"#   nga-w2.lmR1M2_D50GMAR.4paper…csv sha256 {sha(BK17)[:16]}…, smooth sütunları.\n"
            "# T: -1 = PGV, 0 = PGA. Değerler değiştirilmedi.\n"
        )
        w = csv.writer(f, lineterminator="\n")
        cols = ["a_1", "a_2", "a_3", "a_4", "a_5", "a_6", "a_7", "a_8", "a_9", "c_1",
                "v_con", "v_ref", "c", "n", "b_1", "b_2", "sd_within", "sd_between", "sd_total"]
        w.writerow(["T", *[c.replace("_", "") for c in cols], "bk_c0", "bk_r1", "bk_m1", "bk_m2", "bk_sigma"])
        for r in sorted(rows, key=lambda r: r["period"]):
            b = bk[r["period"]]
            p = r["period"]
            w.writerow([
                "PGV" if p == -1 else "PGA" if p == 0 else f"{p:g}",
                *[f"{r[c]:g}" for c in cols],
                b["c0smooth"], b["r1smooth"], b["m1smooth"], b["m2smooth"], b["sigma"],
            ])
    print(f"{OUT.relative_to(ROOT)}: {len(rows)} satır; makale Tablo 4.a ile {n} satır aynı")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
