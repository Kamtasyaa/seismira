#!/usr/bin/env python3
"""Natural Earth v5.1.2 vektör katmanlarını indirir ve SHA256 ile doğrular.

Kullanım (depo kökünden):
    python3 scripts/fetch_natural_earth.py           # eksikleri indir, doğrula
    python3 scripts/fetch_natural_earth.py --check   # yalnızca doğrula

Ham veri depoya girmez (data/raw/ .gitignore'da). İşlenmiş harita dosyaları
seismira-data aracıyla üretilir (bkz. docs/veri.md).
Lisans: kamu malı (https://www.naturalearthdata.com/about/terms-of-use/,
2026-11-14 kontrol edildi).
"""

import hashlib
import pathlib
import sys
import urllib.request

VERSION = "v5.1.2"
BASE = f"https://raw.githubusercontent.com/nvkelso/natural-earth-vector/{VERSION}"
DEST = pathlib.Path(__file__).resolve().parent.parent / "data" / "raw" / "natural-earth" / VERSION

# (dosya adı, depodaki yol, sha256)
FILES = [
    ("ne_10m_admin_0_boundary_lines_land.dbf", "10m_physical/ne_10m_admin_0_boundary_lines_land.dbf", "103d9b00b684b0380d8ded830d24a1406ed28cbb6c1f4dc3df21c50cac615403"),
    ("ne_10m_admin_0_boundary_lines_land.prj", "10m_physical/ne_10m_admin_0_boundary_lines_land.prj", "a02a27b1d1982c8516d83398e85a3c8b1aef1713c13ef4d84d7bde17430c07c4"),
    ("ne_10m_admin_0_boundary_lines_land.shp", "10m_physical/ne_10m_admin_0_boundary_lines_land.shp", "b6811b9e2f6ed40b31622e6d261210c5e6afded9b7ad71977d3658293f56373f"),
    ("ne_10m_admin_0_boundary_lines_land.shx", "10m_physical/ne_10m_admin_0_boundary_lines_land.shx", "e4128caba567cd9e0b11f3a2a0f0b18dfc8f0bb3cd03f2535f750887d6b86fe5"),
    ("ne_10m_admin_1_states_provinces.dbf", "10m_cultural/ne_10m_admin_1_states_provinces.dbf", "445a8a9bea889634faf0af18081830df0b05b8471fc6af8dc42aecdd7a71bba1"),
    ("ne_10m_admin_1_states_provinces.prj", "10m_cultural/ne_10m_admin_1_states_provinces.prj", "a02a27b1d1982c8516d83398e85a3c8b1aef1713c13ef4d84d7bde17430c07c4"),
    ("ne_10m_admin_1_states_provinces.shp", "10m_cultural/ne_10m_admin_1_states_provinces.shp", "c6f5c8b4b1320d9417033762419c6df1eb423989cd880fba78ea0b1e3522cbe4"),
    ("ne_10m_admin_1_states_provinces.shx", "10m_cultural/ne_10m_admin_1_states_provinces.shx", "37a9e2bc79ed31d3bdea3cb62d928f77281a1c88d645cd33430231c75dbcf350"),
    ("ne_10m_admin_1_states_provinces_lines.dbf", "10m_cultural/ne_10m_admin_1_states_provinces_lines.dbf", "b524fd1baf59386cd4bca0d6df642ddb02be9cdc4d69f1f1d109885acf21d9ed"),
    ("ne_10m_admin_1_states_provinces_lines.prj", "10m_cultural/ne_10m_admin_1_states_provinces_lines.prj", "a02a27b1d1982c8516d83398e85a3c8b1aef1713c13ef4d84d7bde17430c07c4"),
    ("ne_10m_admin_1_states_provinces_lines.shp", "10m_cultural/ne_10m_admin_1_states_provinces_lines.shp", "c89b8545fdb03a764df9de2efeadf0d6735169d94b48a315b5e233780e073ab5"),
    ("ne_10m_admin_1_states_provinces_lines.shx", "10m_cultural/ne_10m_admin_1_states_provinces_lines.shx", "ba65a7072e06b5454bca8c0a527978333571f3ce45094f006a60f18f8509db10"),
    ("ne_10m_lakes.dbf", "10m_physical/ne_10m_lakes.dbf", "0edaa87f76c2221c986f0796ce0b54b1c418e9032afba46f8e302edc41193495"),
    ("ne_10m_lakes.prj", "10m_physical/ne_10m_lakes.prj", "a02a27b1d1982c8516d83398e85a3c8b1aef1713c13ef4d84d7bde17430c07c4"),
    ("ne_10m_lakes.shp", "10m_physical/ne_10m_lakes.shp", "11064fa55ed91b56a0d86733643c1a5bfcc137fdf044e83f471978a5b86617e1"),
    ("ne_10m_lakes.shx", "10m_physical/ne_10m_lakes.shx", "a11ffb02e862b6c4ec8c015e6e0c70502f21bc71dcd79312b8dbc676f7e4c557"),
    ("ne_10m_land.dbf", "10m_physical/ne_10m_land.dbf", "e422520372e65c6fb8220888580e5331d04699bcd71c27e53ad42e241a531cbd"),
    ("ne_10m_land.prj", "10m_physical/ne_10m_land.prj", "a02a27b1d1982c8516d83398e85a3c8b1aef1713c13ef4d84d7bde17430c07c4"),
    ("ne_10m_land.shp", "10m_physical/ne_10m_land.shp", "4cad3a49bc75c1a4c2f3d7efae04f2f8e63151c96764b2658effabf524331fa6"),
    ("ne_10m_land.shx", "10m_physical/ne_10m_land.shx", "f3b02a190c3020eeafd0f48de982619826b57ecd853cbc68e4fc049652891a04"),
    ("ne_110m_admin_0_boundary_lines_land.dbf", "110m_physical/ne_110m_admin_0_boundary_lines_land.dbf", "373d7a150b9f8dcb6ec918b52eea0f0361ad40ef433602a1b9f59b4de668daec"),
    ("ne_110m_admin_0_boundary_lines_land.prj", "110m_physical/ne_110m_admin_0_boundary_lines_land.prj", "3259f0e55290a82b1350646f604e8a7ee1e2136c0320a40fad838ab40819fff8"),
    ("ne_110m_admin_0_boundary_lines_land.shp", "110m_physical/ne_110m_admin_0_boundary_lines_land.shp", "d19f784f92626816c064f878571f61bc43ec7bfc407666ce6a450d6e757afff0"),
    ("ne_110m_admin_0_boundary_lines_land.shx", "110m_physical/ne_110m_admin_0_boundary_lines_land.shx", "849899c47c51c6edd04750307b21cb109ae4c7e8ca3c5fee8606f8646dbe2787"),
    ("ne_110m_admin_1_states_provinces_lines.dbf", "110m_cultural/ne_110m_admin_1_states_provinces_lines.dbf", "8c7ff0617356e2b86cf68a14a9f1515c410b59694af751a8e098d4342887be90"),
    ("ne_110m_admin_1_states_provinces_lines.prj", "110m_cultural/ne_110m_admin_1_states_provinces_lines.prj", "98aaf3d1c0ecadf1a424a4536de261c3daf4e373697cb86c40c43b989daf52eb"),
    ("ne_110m_admin_1_states_provinces_lines.shp", "110m_cultural/ne_110m_admin_1_states_provinces_lines.shp", "7688e1823fc43c63f1505998a89b9fd2479428e154d4e2b4670c21df1be5d898"),
    ("ne_110m_admin_1_states_provinces_lines.shx", "110m_cultural/ne_110m_admin_1_states_provinces_lines.shx", "2b6510b39cc322a68053907c82a5705953ba3916b6f6178ef632f63424510422"),
    ("ne_110m_lakes.dbf", "110m_physical/ne_110m_lakes.dbf", "ee65c43bdb14463f9a31eda58f7245e3ae0796ac606a67bea0e8386080878cbf"),
    ("ne_110m_lakes.prj", "110m_physical/ne_110m_lakes.prj", "a02a27b1d1982c8516d83398e85a3c8b1aef1713c13ef4d84d7bde17430c07c4"),
    ("ne_110m_lakes.shp", "110m_physical/ne_110m_lakes.shp", "e65b633578c284776b452bcea07126747d3d6484a507ca43866097059a389dfd"),
    ("ne_110m_lakes.shx", "110m_physical/ne_110m_lakes.shx", "954c018f338248ba6ec6fde3cfe0f8742a4bdb2f886fe888baf6d7b5433c39ab"),
    ("ne_110m_land.dbf", "110m_physical/ne_110m_land.dbf", "db7cf6d2de2811df09bd7fcc6f243ab78a715b83571a0cb7b36b4e2af3297caa"),
    ("ne_110m_land.prj", "110m_physical/ne_110m_land.prj", "3259f0e55290a82b1350646f604e8a7ee1e2136c0320a40fad838ab40819fff8"),
    ("ne_110m_land.shp", "110m_physical/ne_110m_land.shp", "8689e6932b8e370e2ca4587cf3ba21e460b1235db37b6ed3c172c35b4a6088de"),
    ("ne_110m_land.shx", "110m_physical/ne_110m_land.shx", "2719254764a70262a34333581d582d503b8af5d6626e6da4eb2b5f86e7316faa"),
    ("ne_50m_admin_0_boundary_lines_land.dbf", "50m_physical/ne_50m_admin_0_boundary_lines_land.dbf", "3b817d0866bc0e90032831ad0630125bf1fa3dcc7040bedc379605b9e7d59543"),
    ("ne_50m_admin_0_boundary_lines_land.prj", "50m_physical/ne_50m_admin_0_boundary_lines_land.prj", "3259f0e55290a82b1350646f604e8a7ee1e2136c0320a40fad838ab40819fff8"),
    ("ne_50m_admin_0_boundary_lines_land.shp", "50m_physical/ne_50m_admin_0_boundary_lines_land.shp", "16950c1695e39b2721f0c19ecd3b4be6a13565c6c8cbe61b2b834694984a9c40"),
    ("ne_50m_admin_0_boundary_lines_land.shx", "50m_physical/ne_50m_admin_0_boundary_lines_land.shx", "8d1bf4bd9bf7f1bb3c9d0167d56f1b748f04c4ceab5663688990aecd61955cd0"),
    ("ne_50m_admin_1_states_provinces_lines.dbf", "50m_cultural/ne_50m_admin_1_states_provinces_lines.dbf", "782eebfb5e5032d6ea5e47c0f698b6c336a624a987a87ff599988d18875478c0"),
    ("ne_50m_admin_1_states_provinces_lines.prj", "50m_cultural/ne_50m_admin_1_states_provinces_lines.prj", "98aaf3d1c0ecadf1a424a4536de261c3daf4e373697cb86c40c43b989daf52eb"),
    ("ne_50m_admin_1_states_provinces_lines.shp", "50m_cultural/ne_50m_admin_1_states_provinces_lines.shp", "c117e2f2d2776e2d0504f32be833bbbd9fb7b4683cf70e2298b5cc385a25b883"),
    ("ne_50m_admin_1_states_provinces_lines.shx", "50m_cultural/ne_50m_admin_1_states_provinces_lines.shx", "9f5009330cce9a5da1070123266caeebef17d2007a2c3ee60da14b8cadcdc606"),
    ("ne_50m_lakes.dbf", "50m_physical/ne_50m_lakes.dbf", "9c70f3ef0f7175f97ab4a8bc4d249449e95936b1eb75efb5db893c66c80dcdb7"),
    ("ne_50m_lakes.prj", "50m_physical/ne_50m_lakes.prj", "a02a27b1d1982c8516d83398e85a3c8b1aef1713c13ef4d84d7bde17430c07c4"),
    ("ne_50m_lakes.shp", "50m_physical/ne_50m_lakes.shp", "0b94256e53f0a252dde49b9c57a7a1461454ac6e248c1fd02874d826dc469254"),
    ("ne_50m_lakes.shx", "50m_physical/ne_50m_lakes.shx", "8f90881410c4c26d36a976d9663f24e37965a05d34e26f1bf1cb8d609855e7b0"),
    ("ne_50m_land.dbf", "50m_physical/ne_50m_land.dbf", "d082add78c82e01b9bf422df3143ad6fbe43ca1bcb798d12a334f9e001b67bc0"),
    ("ne_50m_land.prj", "50m_physical/ne_50m_land.prj", "98aaf3d1c0ecadf1a424a4536de261c3daf4e373697cb86c40c43b989daf52eb"),
    ("ne_50m_land.shp", "50m_physical/ne_50m_land.shp", "140d1419f7ace66bf0ea8ceb068cab544d413170f4e8c48e0a11bde6c978b319"),
    ("ne_50m_land.shx", "50m_physical/ne_50m_land.shx", "c1b93ee40fe83d6322197251517c141c2c91970915ac61d7bf7a4821fc2a68d7"),
]


def main() -> int:
    check_only = "--check" in sys.argv
    bad = 0
    DEST.mkdir(parents=True, exist_ok=True)
    for name, rel, digest in FILES:
        dest = DEST / name
        if not dest.exists():
            if check_only:
                print(f"EKSİK   {name}")
                bad += 1
                continue
            print(f"indir   {name}")
            req = urllib.request.Request(f"{BASE}/{rel}", headers={"User-Agent": "seismira-data-fetch"})
            with urllib.request.urlopen(req, timeout=300) as resp:
                tmp = dest.with_suffix(dest.suffix + ".part")
                tmp.write_bytes(resp.read())
                tmp.replace(dest)
        if hashlib.sha256(dest.read_bytes()).hexdigest() != digest:
            print(f"HATALI  {name} (SHA256 uyuşmuyor)")
            bad += 1
    print(f"{len(FILES) - bad}/{len(FILES)} dosya doğru.")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
