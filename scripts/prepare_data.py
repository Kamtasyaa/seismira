#!/usr/bin/env python3
"""Uygulamanın çalışması için gereken tüm veriyi hazırlar.

Kullanım (depo kökünden):
    python3 scripts/prepare_data.py            # temel veri
    python3 scripts/prepare_data.py --ground   # + sıvılaşma girdileri (ağır)
    python3 scripts/prepare_data.py --gem      # + GEM paketi (CC BY-NC-SA)

Adımlar:
1. Yazı tipleri (CJK dahil) indirilir ve doğrulanır.
2. Natural Earth ham verisi indirilir ve doğrulanır.
3. seismira-data ile harita dosyaları (data/map/*.smap) üretilir.
4. Vs30 ham dosyaları doğrulanır (elle indirilir, bkz. fetch_vs30.py) ve
   data/vs30/vs30.sras üretilir (~1,5 dk, ~150 MB).
5. Bölge kimliği rasterı (Natural Earth Admin-1, Vs30 ızgarasında) yoksa
   üretilir: data/regions/admin1.sras + .sreg + .sout (~1 dk; depoda da var).
   Kararlı kıta içi bölgeler (USGS STREC, kamu malı): yoksa
   scripts/fetch_strec.py + `seismira-data stable-regions` →
   data/regions/stable.sout (17 KB; depoda da var).
6. Şehir listesi (GeoNames) ve şehir zemini yoksa üretilir:
   data/cities/cities15000.scty + .ssit (depoda da var; GeoNames dökümü her
   gün değiştiği için sabitlenmiş anlık görüntü, bkz. fetch_geonames.py).

Depodaki küçük işlenmiş dosyalar (5 ve 6) silinirse bu betik aynılarını
yeniden üretir; sıradan kullanımda dokunulmaz.

Depoda olup bu betiğin ÜRETMEDİĞİ dosyalar (ağır ya da elle indirilen
kaynak ister): data/traveltime (scripts/gen_traveltime.py, ObsPy),
data/slab2 (scripts/gen_slab2.py, elle indirilen Slab2 arşivi + h5py).
data/exposure/esrm20.sexp: python3 scripts/fetch_esrm20.py (673 MB) +
`seismira-data exposure data/raw/esrm20/<commit> data/regions/admin1.sras
data/exposure/esrm20.sexp`.

7. Tsunami (Faz 8): ETOPO 2022 1′ indirilir (466 MB, scripts/fetch_etopo.py),
   .venv Python'u ile scripts/gen_bathy.py (rasterio, pyogrio, scipy; ~1 dk,
   ~5 GB RAM) ve `seismira-data bathy` → data/tsunami/bathy.sras (25 MB),
   `seismira-data coast-points` → data/tsunami/coast.bin (2 MB).

8. (`--ground`) Zemin yenilmesi girdileri (Faz 7): GMTED2010 + CHELSA
   indirilir (~3,6 GB, scripts/fetch_ground.py), data/ground/inputs30.sras
   (~4 dk, ~8,5 GB RAM) ve data/ground/slope75.sras (~5 dk) üretilir; bunlardan
   uygulamanın okuduğu tek dosya data/ground/liq_static.sras (sıvılaşmanın
   zemin kısmı, `seismira-data liq-static`). Yeraltı su seviyesi
   (data/ground/wtd.sras) ayrı lisanslıdır ve burada ÜRETİLMEZ
   (scripts/gen_fan_wtd.py; docs/veri.md). Varsa liq_static genel modelle
   (yalnızca yerel kullanım), yoksa Zhu 2017 kıyı modeliyle (dağıtılabilir)
   üretilir.
   Heyelan: ESA CCI 2009 arazi örtüsü (aynı indirme) → data/ground/
   landcover.sras (`seismira-data land-cover`); litoloji yoksa GLiM 0,5°
   (CC BY 3.0, .venv Python'u ile scripts/gen_glim05.py; numpy + scipy) →
   data/ground/lith.sras (`seismira-data lith`); sonra data/ground/
   ls_static.sras (`seismira-data ls-static`, ~2 dk). Vektör GLiM'li
   (daha ayrıntılı, yalnızca yerel) litoloji için önce scripts/gen_glim.py
   (elle indirilen arşiv, docs/veri.md); lith.sras varsa dokunulmaz.

9. (`--gem`) GEM paketi (Faz 9; CC BY-NC-SA 4.0, ticari kullanım yok, AYRI
   paket): scripts/fetch_gem.py (~96 MB) + `seismira-data gem` →
   data/gem/gem.svul (2,6 MB, ~10 s). Avrupa dışı bölgelerde ortalama
   yapısal hasar oranı için.

Her adım tekrar çalıştırılabilir; var olan doğru dosyalar yeniden indirilmez.
"""

import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent


def run(*cmd: str) -> None:
    print("$", " ".join(cmd), flush=True)
    subprocess.run(cmd, cwd=ROOT, check=True)


def main() -> int:
    py = sys.executable
    run(py, "scripts/fetch_fonts.py")
    run(py, "scripts/fetch_natural_earth.py")
    run(
        "cargo", "run", "--release", "-q", "-p", "seismira-data", "--",
        "natural-earth", "data/raw/natural-earth/v5.1.2", "data/map",
    )
    run(py, "scripts/fetch_vs30.py")
    vs30 = ROOT / "data" / "vs30" / "vs30.sras"
    if not vs30.exists():
        vs30.parent.mkdir(parents=True, exist_ok=True)
        run(
            "cargo", "run", "--release", "-q", "-p", "seismira-data", "--",
            "vs30", "data/raw/vs30/vs30_mosaic_median_30c.tif",
            "data/raw/vs30/vs30_mosaic_sd_30c.tif", str(vs30.relative_to(ROOT)),
        )
    ghsl = "data/raw/ghsl/GHS_BUILT_S_E2020_GLOBE_R2023A_4326_30ss_V1_0.tif"
    regions = ROOT / "data" / "regions" / "admin1.sras"
    # Üç dosya birlikte üretilir (.sout, .sras ile aynı adla yazılır).
    if not all(regions.with_suffix(e).exists() for e in (".sras", ".sreg", ".sout")):
        run(py, "scripts/fetch_ghsl.py")
        regions.parent.mkdir(parents=True, exist_ok=True)
        run(
            "cargo", "run", "--release", "-q", "-p", "seismira-data", "--",
            "regions", "data/raw/natural-earth/v5.1.2/ne_10m_admin_1_states_provinces.shp",
            str(vs30.relative_to(ROOT)), ghsl,
            "data/regions/admin1.sras", "data/regions/admin1.sreg",
        )
    stable = ROOT / "data" / "regions" / "stable.sout"
    if not stable.exists():
        run(py, "scripts/fetch_strec.py")
        run(
            "cargo", "run", "--release", "-q", "-p", "seismira-data", "--",
            "stable-regions",
            "data/raw/strec/6e13af7f76d52adfeefbd74dbe647705e92db7d0/stable.geojson",
            str(stable.relative_to(ROOT)),
        )
    cities = ROOT / "data" / "cities" / "cities15000.scty"
    if not cities.exists():
        run(py, "scripts/fetch_geonames.py")
        cities.parent.mkdir(parents=True, exist_ok=True)
        run(
            "cargo", "run", "--release", "-q", "-p", "seismira-data", "--",
            "geonames", "data/raw/geonames/2026-11-14/cities15000.txt",
            "data/raw/geonames/2026-11-14/admin1CodesASCII.txt",
            str(cities.relative_to(ROOT)),
        )
    sites = cities.with_suffix(".ssit")
    if not sites.exists():
        run(py, "scripts/fetch_ghsl.py")
        run(
            "cargo", "run", "--release", "-q", "-p", "seismira-data", "--",
            "city-sites", str(cities.relative_to(ROOT)), str(vs30.relative_to(ROOT)),
            ghsl, str(sites.relative_to(ROOT)),
        )
    tsu = ROOT / "data" / "tsunami"
    bathy = tsu / "bathy.sras"
    coast = tsu / "coast.bin"
    if not bathy.exists():
        run(py, "scripts/fetch_etopo.py")
        venv = ROOT / ".venv" / "bin" / "python"
        run(str(venv if venv.exists() else py), "-I", "scripts/gen_bathy.py")
        run(
            "cargo", "run", "--release", "-q", "-p", "seismira-data", "--",
            "bathy", "data/raw/etopo/bathy_1m.i16", str(bathy.relative_to(ROOT)),
        )
    if not coast.exists():
        run(
            "cargo", "run", "--release", "-q", "-p", "seismira-data", "--",
            "coast-points", str(bathy.relative_to(ROOT)),
            str(cities.relative_to(ROOT)), str(coast.relative_to(ROOT)),
        )
    if "--ground" in sys.argv[1:]:
        ground = ROOT / "data" / "ground"
        ground.mkdir(parents=True, exist_ok=True)
        inputs = ground / "inputs30.sras"
        slope = ground / "slope75.sras"
        if not (inputs.exists() and slope.exists()):
            run(py, "scripts/fetch_ground.py")
        if not inputs.exists():
            run(
                "cargo", "run", "--release", "-q", "-p", "seismira-data", "--",
                "ground-inputs", "data/raw/gmted/md30/md30_grd", str(vs30.relative_to(ROOT)),
                "data/raw/chelsa/CHELSA_bio12_1981-2010_V.2.1.tif",
                "data/raw/natural-earth/v5.1.2/ne_10m_rivers_lake_centerlines.shp",
                str(inputs.relative_to(ROOT)),
            )
        if not slope.exists():
            run(
                "cargo", "run", "--release", "-q", "-p", "seismira-data", "--",
                "slope75", "data/raw/gmted/md75/md75_grd", str(vs30.relative_to(ROOT)),
                str(slope.relative_to(ROOT)),
            )
        liq = ground / "liq_static.sras"
        if not liq.exists():
            extra = [] if (ground / "wtd.sras").exists() else ["--coastal"]
            run(
                "cargo", "run", "--release", "-q", "-p", "seismira-data", "--",
                "liq-static", str(ground.relative_to(ROOT)), str(vs30.relative_to(ROOT)),
                str(liq.relative_to(ROOT)), *extra,
            )
        cover = ground / "landcover.sras"
        if not cover.exists():
            run(
                "cargo", "run", "--release", "-q", "-p", "seismira-data", "--",
                "land-cover", "data/raw/esacci/ESACCI-LC-L4-LCCS-Map-300m-P1Y-2009-v2.0.7.tif",
                str(vs30.relative_to(ROOT)), str(cover.relative_to(ROOT)),
            )
        lith = ground / "lith.sras"
        if not lith.exists():
            glim05 = ROOT / "data" / "raw" / "glim05" / "glim05_vs30grid.u8"
            if not glim05.exists():
                venv = ROOT / ".venv" / ("Scripts/python.exe" if sys.platform == "win32" else "bin/python")
                if not venv.exists():
                    print("GLiM 0,5° için .venv gerekli (numpy, scipy): python3 -m venv .venv "
                          "&& .venv/bin/pip install numpy scipy", file=sys.stderr)
                    return 1
                run(str(venv), "-I", "scripts/gen_glim05.py")
            run(
                "cargo", "run", "--release", "-q", "-p", "seismira-data", "--",
                "lith", str(glim05.relative_to(ROOT)), str(vs30.relative_to(ROOT)),
                str(lith.relative_to(ROOT)),
            )
        ls = ground / "ls_static.sras"
        if not ls.exists():
            run(
                "cargo", "run", "--release", "-q", "-p", "seismira-data", "--",
                "ls-static", str(ground.relative_to(ROOT)), str(vs30.relative_to(ROOT)),
                str(ls.relative_to(ROOT)),
            )
    if "--gem" in sys.argv[1:]:
        gem = ROOT / "data" / "gem" / "gem.svul"
        if not gem.exists():
            run(py, "scripts/fetch_gem.py")
            gem.parent.mkdir(parents=True, exist_ok=True)
            run(
                "cargo", "run", "--release", "-q", "-p", "seismira-data", "--",
                "gem", "data/raw/gem", "data/regions/admin1.sras",
                str(gem.relative_to(ROOT)),
            )
    print("Veri hazır.")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except subprocess.CalledProcessError as err:
        print(f"Adım başarısız: {err}", file=sys.stderr)
        sys.exit(1)
