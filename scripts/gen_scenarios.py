#!/usr/bin/env python3
"""Hazır senaryolar (Faz 10; CLAUDE.md Bölüm 14) → data/scenarios/scenarios.json.

Her senaryo bir doğrulama depremidir. Kaynaklar (2026-11-23 sorgulandı):
- USGS ComCat olay sayfası (fdsnws/event/1/query?eventid=...): merkez üssü,
  odak derinliği, büyüklük, Mww moment tensörü düğüm düzlemi (kayma açısı,
  kuad eğimi çıkarılamazsa eğim).
- USGS ShakeMap kırılma modeli (`download/rupture.json`, tercih edilen
  ShakeMap ürünü; her dosyanın `metadata.reference` alanındaki yayımlanmış
  modele dayanır, çıktıya yazılır).

Tek dikdörtgen düzleme indirgeme (uygulamanın kaynak modeli tek düzlem):
- Parçalar: ShakeMap kuadları. Her kuadın üst kenarı (en sığ köşeler).
  Doğrultusu (üst kenarın uçlarından) ana şokun Mww düzleminin doğrultusuna
  (mod 180) 45°'ten uzak kuadlar atlanır (Ridgecrest: Mw 6,4 öncü
  şokun dik fayı).
- Ana eksen: kalan üst kenar noktalarının yerel düzlemde (azimutal eşit
  uzaklık, merkez üssü çevresinde) ana bileşeni. Doğrultu, eksen yönlerinden
  Mww doğrultusuna yakın olanı.
- Uzunluk L: noktaların eksen üzerindeki uzanımı. Odak doğrultu konumu:
  merkez üssünün izdüşümü (0 = doğrultunun başlangıç ucu).
- Eğim: eğimli tek kuadlı modellerde kuadın kendi eğimi (üst ve alt kenar
  arasındaki yatay kayma ve derinlik farkından); düşey ya da çok parçalı
  modellerde Mww eğimi. Genişlik W = (z_alt − z_üst) / sin(eğim). Odak eğim
  konumu = (derinlik − z_üst) / (z_alt − z_üst), [0, 1]'e kırpılır.

Bu, gerçek çok parçalı kırılmanın KABA eşdeğeridir: düzlem merkez üssünden
geçer (gerçek iz merkez üssünden km'lerce uzakta olabilir, ör.
Kahramanmaraş'ta merkez üssü Narlı fayında), eğrilik ve parça atlamaları
yoktur.

Çalıştırma (depo kökünden): python3 scripts/gen_scenarios.py
"""

import json
import math
import pathlib
import sys
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parent.parent
OUT = ROOT / "data" / "scenarios" / "scenarios.json"
R_KM = 6371.0088

# (kimlik, USGS olay kimliği, ortam)
EVENTS = [
    ("kahramanmaras2023", "us6000jllz", "crust"),
    ("izmit1999", "usp0009d4z", "crust"),
    ("kumamoto2016", "us20005iis", "crust"),
    ("ridgecrest2019", "ci38457511", "crust"),
    ("tohoku2011", "official20110311054624120_30", "interface"),
    ("noto2024", "us6000m0xl", "crust"),
    ("maule2010", "official20100227063411530_30", "interface"),
    ("puebla2017", "us2000ar20", "intraslab"),
]


def get_json(url: str):
    req = urllib.request.Request(url, headers={"User-Agent": "seismira-data"})
    with urllib.request.urlopen(req, timeout=120) as r:
        return json.load(r)


def local_xy(origin, p):
    """Azimutal eşit uzaklık: (boylam, enlem) → (x doğu, y kuzey) km."""
    lo0, la0 = map(math.radians, origin)
    lo, la = map(math.radians, p)
    dl = lo - lo0
    h = math.sin((la - la0) / 2) ** 2 + math.cos(la0) * math.cos(la) * math.sin(dl / 2) ** 2
    d = 2 * R_KM * math.asin(math.sqrt(h))
    az = math.atan2(
        math.sin(dl) * math.cos(la),
        math.cos(la0) * math.sin(la) - math.sin(la0) * math.cos(la) * math.cos(dl),
    )
    return d * math.sin(az), d * math.cos(az)


def azimuth(a, b):
    return math.degrees(math.atan2(b[0] - a[0], b[1] - a[1])) % 360.0


def ang_diff_axial(a, b):
    """İki doğrultu arasındaki açı, yönsüz (mod 180), 0–90."""
    d = abs((a - b) % 180.0)
    return min(d, 180.0 - d)


def quads(rupture):
    """Her kuad için (üst kenar [(lon, lat)], z_üst, z_alt, alt kenar)."""
    out = []
    for f in rupture["features"]:
        g = f["geometry"]
        if g["type"] != "MultiPolygon":
            continue
        # Parçalar ayrı çokgen ya da aynı çokgenin ayrı halkaları olabilir.
        for ring in (r[:-1] for poly in g["coordinates"] for r in poly):
            ztop = min(p[2] for p in ring)
            zbot = max(p[2] for p in ring)
            top = [(p[0], p[1]) for p in ring if abs(p[2] - ztop) < 1e-6]
            bot = [(p[0], p[1]) for p in ring if abs(p[2] - zbot) < 1e-6]
            out.append((top, ztop, zbot, bot))
    return out


def preferred(products):
    return max(products, key=lambda p: p.get("preferredWeight", 0))


def build(sid, eid, setting):
    ev = get_json(f"https://earthquake.usgs.gov/fdsnws/event/1/query?eventid={eid}&format=geojson")
    p = ev["properties"]
    lon, lat, depth = ev["geometry"]["coordinates"]
    mt = [
        m
        for m in p["products"]["moment-tensor"]
        if m["properties"].get("derived-magnitude-type", "").lower() == "mww"
    ]
    mtp = preferred(mt)["properties"]
    planes = [
        tuple(float(mtp[f"nodal-plane-{i}-{k}"]) for k in ("strike", "dip", "rake"))
        for i in (1, 2)
    ]
    sm = next(s for s in p["products"]["shakemap"] if "download/rupture.json" in s["contents"])
    rup = get_json(sm["contents"]["download/rupture.json"]["url"])
    ref = rup.get("metadata", {}).get("reference", "")
    qs = quads(rup)

    # Mww düğüm düzlemi: kuadların genel doğrultusuna yakın olan.
    pts_all = [pt for q in qs for pt in q[0]]
    xy_all = [local_xy((lon, lat), pt) for pt in pts_all]
    ax_all = principal_axis(xy_all)
    plane = min(planes, key=lambda pl: ang_diff_axial(pl[0], ax_all))
    mt_strike, mt_dip, rake = plane

    def quad_strike(q):
        return azimuth(local_xy((lon, lat), q[0][0]), local_xy((lon, lat), q[0][-1]))

    keep = [q for q in qs if len(q[0]) >= 2 and ang_diff_axial(quad_strike(q), mt_strike) <= 45.0]
    skipped = len(qs) - len(keep)
    xy = [local_xy((lon, lat), pt) for q in keep for pt in q[0]]
    axis = principal_axis(xy)
    strike = axis
    # Eksen yönünü Mww doğrultusuna yakın seç (yönlü).
    if abs((strike - mt_strike + 180) % 360 - 180) > 90:
        strike = (strike + 180) % 360
    s = (math.sin(math.radians(strike)), math.cos(math.radians(strike)))
    proj = [x * s[0] + y * s[1] for x, y in xy]
    lo, hi = min(proj), max(proj)
    length = hi - lo
    along = (0.0 - lo) / length  # merkez üssü yerel orijinde

    ztop = min(q[1] for q in keep)
    zbot = max(q[2] for q in keep)
    dip = mt_dip
    dip_from = "Mww"
    if len(keep) == 1 and keep[0][3]:
        top, _, _, bot = keep[0]
        # Üst kenar ortası → alt kenar ortası yatay uzaklık.
        tm = [sum(c) / len(top) for c in zip(*[local_xy((lon, lat), pt) for pt in top])]
        bm = [sum(c) / len(bot) for c in zip(*[local_xy((lon, lat), pt) for pt in bot])]
        horiz = math.hypot(bm[0] - tm[0], bm[1] - tm[1])
        if horiz > 0.5:
            dip = math.degrees(math.atan2(zbot - ztop, horiz))
            dip_from = "ShakeMap kuadı"
            # Doğrultu: eğim yönü doğrultunun sağında (Aki & Richards).
            dip_az = azimuth(tm, bm)
            cand = (dip_az - 90.0) % 360
            if abs((cand - strike + 180) % 360 - 180) > 90:
                strike = (strike + 180) % 360
                along = 1.0 - along
    width = (zbot - ztop) / math.sin(math.radians(dip))
    down = min(max((depth - ztop) / (zbot - ztop), 0.0), 1.0)
    return {
        "id": sid,
        "usgs_id": eid,
        "setting": setting,
        "lon": round(lon, 4),
        "lat": round(lat, 4),
        "depth_km": round(depth, 1),
        "mw": round(float(p["mag"]), 1),
        "strike": round(strike, 1),
        "dip": round(dip, 1),
        "rake": round(rake, 1),
        "hypo_along": round(min(max(along, 0.0), 1.0), 3),
        "hypo_down": round(down, 3),
        "length_km": round(length, 0),
        "width_km": round(width, 0),
        "time": p["time"],
        "source": {
            "event": f"USGS ComCat {eid}, Mww düğüm düzlemi {mt_strike:.0f}/{mt_dip:.0f}/{rake:.0f}",
            "rupture": f"USGS ShakeMap ({sm['source']} v{sm['properties'].get('version')}) rupture.json: {ref.strip()}",
            "dip_from": dip_from,
            "skipped_quads": skipped,
        },
    }


def principal_axis(xy):
    """Noktaların ana ekseninin azimutu (0–180)."""
    n = len(xy)
    mx = sum(p[0] for p in xy) / n
    my = sum(p[1] for p in xy) / n
    sxx = sum((p[0] - mx) ** 2 for p in xy)
    syy = sum((p[1] - my) ** 2 for p in xy)
    sxy = sum((p[0] - mx) * (p[1] - my) for p in xy)
    theta = 0.5 * math.atan2(2 * sxy, sxx - syy)  # x ekseninden
    az = (90.0 - math.degrees(theta)) % 180.0
    return az


def main() -> int:
    out = [build(*e) for e in EVENTS]
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps({"version": 1, "scenarios": out}, ensure_ascii=False, indent=1) + "\n")
    for s in out:
        print(
            f"{s['id']:18} Mw {s['mw']} {s['setting']:9} {s['strike']:6.1f}/{s['dip']:4.1f}/{s['rake']:6.1f} "
            f"L {s['length_km']:5.0f} W {s['width_km']:4.0f} along {s['hypo_along']:.2f} down {s['hypo_down']:.2f} "
            f"(eğim: {s['source']['dip_from']}, atlanan kuad {s['source']['skipped_quads']})"
        )
    print(f"→ {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
