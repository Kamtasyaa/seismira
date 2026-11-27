#!/usr/bin/env python3
"""Kritik tesisler (README "Önemli"; kullanıcı kararı 2026-11-27: AYRI dosya,
ODbL, ana veri paketinin içinde) → ara TSV (seismira-data infra ikiliye çevirir).

Kaynak: OpenStreetMap (© OpenStreetMap contributors, ODbL 1.0), Geofabrik
özütleri (.osm.pbf). openstreetmap.org döşeme sunucularından HİÇBİR ŞEY
çekilmez (CLAUDE.md Bölüm 4); yalnızca toplu veri özütü.

Tesis türleri ve OSM etiketleri (SEÇİM, docs/veri.md):
- hastane   amenity=hospital ya da healthcare=hospital (nokta, alan, ilişki)
- köprü     bridge=yes|viaduct|… olmayan; highway=motorway|trunk|primary
            (ve _link'leri) ya da railway=rail üzerindeki yollar (way);
            bridge=viaduct → "viyadük"
- baraj     waterway=dam (çizgi ya da alan); adsız ve 100 m'den kısa
            olanlar atlanır (küçük set / bent)
- trafo     power=substation; gerilim ≥ 66 kV (voltage etiketi) ya da
            substation=transmission; gerilimi bilinmeyen dağıtım trafoları
            atlanır
- gaz       man_made=pipeline, substance=gas|natural_gas (çizgi)

Noktasal tesislerin konumu: alan / çizgilerde düğümlerin sınır kutusu
merkezi (köprü: orta düğüm). Gaz hatları Douglas–Peucker ile ~100 m'ye
sadeleştirilir ve ≤ 50 km'lik parçalara bölünür.

İki geçiş (büyük kıtalarda tüm düğüm konumlarını bellekte tutmamak için):
1) yollar ve ilişkiler: etiket süzgeci, gereken düğüm kimlikleri
2) düğümler: etiketli tesis düğümleri + 1'de toplanan kimliklerin konumu
İlişki (multipolygon) tesisleri: üye yolların düğümleri (aynı geçişte).

Çalıştırma (depo kökünden):
    .venv/bin/python -I scripts/gen_infra.py <girdi.osm.pbf> [...] <çıktı.tsv>
TSV: tür, boylam, enlem, öznitelik (köprü uzunluğu m / gerilim kV / 0),
ad, geometri (gaz: "boy enl;boy enl;…", diğerleri boş).
"""

import math
import sys

import osmium

ROAD = {"motorway", "trunk", "primary", "motorway_link", "trunk_link", "primary_link"}
GAS = {"gas", "natural_gas"}


def kind_of(tags, is_way):
    """Tesis türü ya da None."""
    if tags.get("amenity") == "hospital" or tags.get("healthcare") == "hospital":
        return "hospital"
    if tags.get("waterway") == "dam":
        return "dam"
    if tags.get("power") == "substation":
        v = max_kv(tags.get("voltage", ""))
        if v >= 66 or tags.get("substation") == "transmission":
            return "substation"
        return None
    if is_way:
        b = tags.get("bridge")
        if b and b not in ("no",) and (
            tags.get("highway") in ROAD or tags.get("railway") == "rail"
        ):
            return "viaduct" if b == "viaduct" else "bridge"
        if tags.get("man_made") == "pipeline" and tags.get("substance") in GAS:
            return "gas"
    return None


def max_kv(v):
    best = 0
    for part in v.replace(",", ";").split(";"):
        try:
            best = max(best, int(float(part.strip())) // 1000)
        except ValueError:
            pass
    return best


def name_of(tags):
    for k in ("name", "name:en", "ref"):
        s = tags.get(k)
        if s:
            return s.replace("\t", " ").replace("\n", " ")[:80]
    return ""


def attr_of(kind, tags):
    if kind == "substation":
        return max_kv(tags.get("voltage", ""))
    return 0


KEYS = osmium.filter.KeyFilter(
    "amenity", "healthcare", "waterway", "power", "bridge", "man_made"
)


def haversine(a, b):
    R = 6371.0088
    la1, la2 = math.radians(a[1]), math.radians(b[1])
    dl = math.radians(b[0] - a[0])
    h = math.sin((la2 - la1) / 2) ** 2 + math.cos(la1) * math.cos(la2) * math.sin(dl / 2) ** 2
    return 2 * R * math.asin(min(1.0, math.sqrt(h)))


def simplify(pts, tol_km):
    """Douglas–Peucker (yerel düzlemde, km)."""
    if len(pts) < 3:
        return pts
    lat0 = math.radians(sum(p[1] for p in pts) / len(pts))
    xy = [(p[0] * 111.32 * math.cos(lat0), p[1] * 110.57) for p in pts]
    keep = [False] * len(pts)
    keep[0] = keep[-1] = True
    stack = [(0, len(pts) - 1)]
    while stack:
        i, j = stack.pop()
        (x1, y1), (x2, y2) = xy[i], xy[j]
        dx, dy = x2 - x1, y2 - y1
        L = math.hypot(dx, dy) or 1e-12
        best, bk = -1.0, -1
        for k in range(i + 1, j):
            d = abs(dy * (xy[k][0] - x1) - dx * (xy[k][1] - y1)) / L
            if d > best:
                best, bk = d, k
        if best > tol_km:
            keep[bk] = True
            stack += [(i, bk), (bk, j)]
    return [p for p, k in zip(pts, keep) if k]


def chunks(pts, max_km):
    out, cur, acc = [], [pts[0]], 0.0
    for a, b in zip(pts, pts[1:]):
        acc += haversine(a, b)
        cur.append(b)
        if acc >= max_km:
            out.append(cur)
            cur, acc = [b], 0.0
    if len(cur) > 1:
        out.append(cur)
    return out


def run(path, out):
    # 1. geçiş: yollar ve ilişkiler.
    ways = []  # (tür, etiketler, düğüm kimlikleri)
    rels = []  # (tür, etiketler, üye yol kimlikleri)
    need_nodes = set()
    for o in osmium.FileProcessor(path, osmium.osm.WAY | osmium.osm.RELATION).with_filter(KEYS):
        if o.is_way():
            k = kind_of(o.tags, True)
            if k:
                refs = [n.ref for n in o.nodes]
                ways.append((k, dict(o.tags), refs))
                need_nodes.update(refs)
        elif o.is_relation():
            k = kind_of(o.tags, False)
            if k in ("hospital", "dam", "substation"):
                rels.append((k, dict(o.tags), [m.ref for m in o.members if m.type == "w"]))
    # İlişki üyesi yolların düğümleri (yolların kendisi etiketsiz olabilir).
    member_ways = {w for _, _, ms in rels for w in ms}
    way_nodes = {}
    if member_ways:
        for o in osmium.FileProcessor(path, osmium.osm.WAY).with_filter(
            osmium.filter.IdFilter(member_ways)
        ):
            refs = [n.ref for n in o.nodes]
            way_nodes[o.id] = refs
            need_nodes.update(refs)
    # 2. geçiş: düğümler. Süzgeçler C++ tarafında (Python yalnızca ilgili
    # düğümleri görür): önce etiketli tesis düğümleri, sonra gereken konumlar.
    loc = {}
    points = []
    for o in osmium.FileProcessor(path, osmium.osm.NODE).with_filter(KEYS):
        k = kind_of(o.tags, False)
        if k in ("hospital", "dam", "substation"):
            points.append((k, o.location.lon, o.location.lat, attr_of(k, o.tags), name_of(o.tags), ""))
    if need_nodes:
        for o in osmium.FileProcessor(path, osmium.osm.NODE).with_filter(
            osmium.filter.IdFilter(need_nodes)
        ):
            loc[o.id] = (o.location.lon, o.location.lat)

    def bbox_center(coords):
        xs = [c[0] for c in coords]
        ys = [c[1] for c in coords]
        return (min(xs) + max(xs)) / 2, (min(ys) + max(ys)) / 2

    n_skip = 0
    for k, tags, refs in ways:
        coords = [loc[r] for r in refs if r in loc]
        if len(coords) < 2:
            n_skip += 1
            continue
        length_m = sum(haversine(a, b) for a, b in zip(coords, coords[1:])) * 1000
        name = name_of(tags)
        if k == "gas":
            s = simplify(coords, 0.1)
            for part in chunks(s, 50.0):
                geom = ";".join(f"{x:.5f} {y:.5f}" for x, y in part)
                points.append((k, part[0][0], part[0][1], 0, name, geom))
            continue
        if k == "dam" and not name and length_m < 100:
            continue
        if k in ("bridge", "viaduct"):
            x, y = coords[len(coords) // 2]
            points.append((k, x, y, min(int(length_m), 65535), name, ""))
            continue
        x, y = bbox_center(coords)
        points.append((k, x, y, attr_of(k, tags), name, ""))
    for k, tags, members in rels:
        coords = [loc[r] for w in members for r in way_nodes.get(w, []) if r in loc]
        if not coords:
            n_skip += 1
            continue
        x, y = bbox_center(coords)
        points.append((k, x, y, attr_of(k, tags), name_of(tags), ""))
    with open(out, "a") as f:
        for k, x, y, a, name, geom in points:
            f.write(f"{k}\t{x:.6f}\t{y:.6f}\t{a}\t{name}\t{geom}\n")
    counts = {}
    for p in points:
        counts[p[0]] = counts.get(p[0], 0) + 1
    print(f"{path}: {counts}, eksik geometri {n_skip}")


def main():
    if len(sys.argv) < 3:
        print(__doc__)
        return 2
    *inputs, out = sys.argv[1:]
    open(out, "w").close()
    for p in inputs:
        run(p, out)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
