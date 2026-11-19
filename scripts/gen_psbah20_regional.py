#!/usr/bin/env python3
"""Parker vd. (2020) bölgesel katsayılarını PEER elektronik ekinden çıkarır.

Kaynak: PEER Report 2020/03 elektronik eki (Tablo E1 arayüz, E2 levha içi),
https://apps.peer.berkeley.edu/publications/peer_reports/reports_2020/PSHAB_Electronic_Supplement.zip
İndirilen arşiv data/raw/peer/ altında tutulur (depoya girmez) ve SHA256 ile
doğrulanır.

Çıktı: crates/seismira-core/src/gmpe/psbah20_regional.csv (depoda). Yalnızca
bölgeye göre değişen katsayılar yazılır: c0, a0, s1, s2. Bölgeye göre
değişmeyen katsayılar nshmp-lib dosyalarından gelir (psbah20_*.csv). Rapor
önerileri (Bölüm 4.3, 5.2.2, 8.2.2):
- a0: Alaska (Aleut Adaları dahil), Orta Amerika ve Meksika, Japonya,
  Güney Amerika, Tayvan bölgesel; Cascadia arayüzünde küresel (ekte NA),
  levha içinde Cascadia.
- s2: Alaska (Aleut dahil), Cascadia, Japonya, Güney Amerika, Tayvan
  bölgesel; Orta Amerika ve Meksika küresel. s1 yalnızca Japonya ve
  Tayvan'da ayrı, diğerlerinde s1 = s2.

Ayrıca ekteki küresel, Alaska, Aleut ve Cascadia değerlerinin nshmp-lib
dosyalarıyla aynı olduğu denetlenir (bilinen tek fark arayüz c6'sı, burada
kullanılmaz).

Çalıştırma (depo kökünden): python3 scripts/gen_psbah20_regional.py
"""

import csv
import hashlib
import io
import pathlib
import urllib.request
import zipfile

ROOT = pathlib.Path(__file__).resolve().parent.parent
URL = (
    "https://apps.peer.berkeley.edu/publications/peer_reports/reports_2020/"
    "PSHAB_Electronic_Supplement.zip"
)
SHA256 = "c83bcfae6effd15ff3afcc09c9cf543a2189ecf2961d4ef3d3d2b4273e57f521"
RAW = ROOT / "data" / "raw" / "peer" / "PSHAB_Electronic_Supplement.zip"
GMPE = ROOT / "crates" / "seismira-core" / "src" / "gmpe"
OUT = GMPE / "psbah20_regional.csv"

# Bölge → (c0 sütunu, a0 sütunu, s1 sütunu, s2 sütunu), olay tipine göre.
# Sütun yoksa küresel. Bölge adları Rust'taki `Region` ile aynı.
INTERFACE = {
    "global": ("Global_c0", "Global_a0", "Global_s2", "Global_s2"),
    "alaska": ("Alaska_c0", "Alaska_a0", "Alaska_s2", "Alaska_s2"),
    "aleutian": ("Aleutian_c0", "Alaska_a0", "Alaska_s2", "Alaska_s2"),
    # Arayüzde Cascadia a0 ekte NA; rapor 8.2.2 küresel a0'ı önerir.
    "cascadia": ("Cascadia_c0", "Global_a0", "Cascadia_s2", "Cascadia_s2"),
    "cam_n": ("CAM_N_c0", "CAM_a0", "Global_s2", "Global_s2"),
    "cam_s": ("CAM_S_c0", "CAM_a0", "Global_s2", "Global_s2"),
    "japan_pac": ("Japan_Pac_c0", "Japan_a0", "Japan_s1", "Japan_s2"),
    "japan_phi": ("Japan_Phi_c0", "Japan_a0", "Japan_s1", "Japan_s2"),
    "sa_n": ("SA_N_c0", "SA_a0", "SA_s2", "SA_s2"),
    "sa_s": ("SA_S_c0", "SA_a0", "SA_s2", "SA_s2"),
    "taiwan_e": ("Taiwan_E_c0", "Taiwan_a0", "Taiwan_s1", "Taiwan_s2"),
    "taiwan_w": ("Taiwan_W_c0", "Taiwan_a0", "Taiwan_s1", "Taiwan_s2"),
}
# Levha içinde Orta Amerika, Japonya ve Tayvan alt bölgeleri ortak sabitli
# (rapor 4.3.4); köşe büyüklükleri yine alt bölgeye göre (Tablo 4.1, Rust'ta).
SLAB = {
    "global": ("Global_c0", "Global_a0", "Global_s2", "Global_s2"),
    "alaska": ("Alaska_c0", "Alaska_a0", "Alaska_s2", "Alaska_s2"),
    "aleutian": ("Aleutian_c0", "Alaska_a0", "Alaska_s2", "Alaska_s2"),
    "cascadia": ("Cascadia_c0", "Cascadia_a0", "Cascadia_s2", "Cascadia_s2"),
    "cam_n": ("CAM_c0", "CAM_a0", "Global_s2", "Global_s2"),
    "cam_s": ("CAM_c0", "CAM_a0", "Global_s2", "Global_s2"),
    "japan_pac": ("Japan_c0", "Japan_a0", "Japan_s1", "Japan_s2"),
    "japan_phi": ("Japan_c0", "Japan_a0", "Japan_s1", "Japan_s2"),
    "sa_n": ("SA_N_c0", "SA_a0", "SA_s2", "SA_s2"),
    "sa_s": ("SA_S_c0", "SA_a0", "SA_s2", "SA_s2"),
    "taiwan_e": ("Taiwan_c0", "Taiwan_a0", "Taiwan_s1", "Taiwan_s2"),
    "taiwan_w": ("Taiwan_c0", "Taiwan_a0", "Taiwan_s1", "Taiwan_s2"),
}


def fetch() -> bytes:
    if not RAW.exists():
        RAW.parent.mkdir(parents=True, exist_ok=True)
        print(f"indiriliyor: {URL}")
        req = urllib.request.Request(URL, headers={"User-Agent": "seismira-data"})
        with urllib.request.urlopen(req) as r:
            RAW.write_bytes(r.read())
    data = RAW.read_bytes()
    got = hashlib.sha256(data).hexdigest()
    if got != SHA256:
        raise SystemExit(f"SHA256 uyuşmuyor: {got}")
    return data


def table(z: zipfile.ZipFile, name: str, skip: int) -> dict[str, dict[str, str]]:
    path = next(n for n in z.namelist() if n.endswith(name))
    text = z.read(path).decode("utf-8-sig").replace("\r", "")
    rows = list(csv.reader(io.StringIO(text)))[skip:]
    header = [h.strip() for h in rows[0]]
    out = {}
    for row in rows[1:]:
        if not row or not row[0].strip():
            continue
        out[period_key(row[0])] = dict(zip(header, (v.strip() for v in row)))
    return out


def period_key(p: str) -> str:
    p = p.strip()
    return {"-1": "PGV", "0": "PGA"}.get(p, repr(float(p)))


def nshmp(name: str) -> dict[str, dict[str, str]]:
    lines = [
        l for l in (GMPE / name).read_text().splitlines()
        if l.strip() and not l.lstrip().startswith("#")
    ]
    rows = list(csv.reader(lines))
    header = [h.strip() for h in rows[0]]
    return {
        (r[0].strip() if r[0].strip() in ("PGA", "PGV") else repr(float(r[0]))): dict(
            zip(header, (v.strip() for v in r))
        )
        for r in rows[1:]
    }


def check(ours: dict, supp: dict, label: str) -> None:
    cols = ["Global_c0", "Alaska_c0", "Aleutian_c0", "Cascadia_c0", "Global_a0",
            "Alaska_a0", "Global_s2", "Alaska_s2", "Cascadia_s2"]
    for t, row in ours.items():
        for c in cols:
            a, b = float(row[c]), float(supp[t][c])
            if abs(a - b) > 1e-9:
                raise SystemExit(f"{label} {t} {c}: nshmp {a} / ek {b}")


def main() -> int:
    z = zipfile.ZipFile(io.BytesIO(fetch()))
    e1 = table(z, "Table_E1_Interface_Coefficients.csv", 0)
    e2 = table(z, "Table_E2_Slab_Coefficients.csv", 1)
    n_int, n_slab = nshmp("psbah20_interface.csv"), nshmp("psbah20_slab.csv")
    check(n_int, e1, "arayüz")
    check(n_slab, e2, "levha içi")

    rows = []
    for event, supp, ours, mapping in [
        ("interface", e1, n_int, INTERFACE),
        ("intraslab", e2, n_slab, SLAB),
    ]:
        # Yalnızca nshmp-lib dosyasında olan periyotlar (ortak ızgara).
        for t in ours:
            for region, cols in mapping.items():
                vals = [supp[t][c] for c in cols]
                if any(v in ("", "NA") for v in vals):
                    raise SystemExit(f"{event} {t} {region}: eksik {cols} {vals}")
                rows.append([event, region, t, *vals])

    with open(OUT, "w", newline="") as f:
        f.write(
            "# Parker vd. (2020) bölgesel katsayıları (c0, a0, s1, s2). Kaynak: PEER Report\n"
            "# 2020/03 elektronik eki, Tablo E1 (arayüz) ve E2 (levha içi),\n"
            f"# PSHAB_Electronic_Supplement.zip sha256 {SHA256[:12]}…\n"
            "# Değiştirilmedi; sütun seçimi ve eşleme scripts/gen_psbah20_regional.py.\n"
        )
        w = csv.writer(f, lineterminator="\n")
        w.writerow(["event", "region", "T", "c0", "a0", "s1", "s2"])
        w.writerows(rows)
    print(f"{len(rows)} satır → {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
