#!/usr/bin/env python3
"""Noto yazı tiplerini indirir ve SHA256 ile doğrular.

Kullanım (depo kökünden):
    python3 scripts/fetch_fonts.py           # eksikleri indir, hepsini doğrula
    python3 scripts/fetch_fonts.py --check   # yalnızca doğrula

CJK dışındaki yazı tipleri depoda bulunur; CJK dosyaları (~23 MB) depoya
girmez, bu betikle assets/fonts/cjk/ altına indirilir.
Lisans: SIL Open Font License 1.1 (assets/fonts/OFL.txt, cjk/OFL.txt).
Bir yazı tipi güncellenecekse URL ve SHA256 birlikte değiştirilir.
"""

import hashlib
import pathlib
import sys
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parent.parent / "assets" / "fonts"

# (assets/fonts altındaki yol, kaynak URL, sha256)
FONTS = [
    ("NotoSansArabic-Regular.ttf", "https://raw.githubusercontent.com/notofonts/notofonts.github.io/main/fonts/NotoSansArabic/hinted/ttf/NotoSansArabic-Regular.ttf",
     "bdff3e5659d67e67def05b33f749683b9376ae819d65d3dd62ac4640b3aaef48"),
    ("NotoSansArmenian-Regular.ttf", "https://raw.githubusercontent.com/notofonts/notofonts.github.io/main/fonts/NotoSansArmenian/hinted/ttf/NotoSansArmenian-Regular.ttf",
     "720df88c332417a235b4d6209d14ec2e2bf4bfe2a954b7453d869ea593bfce1e"),
    ("NotoSansBengali-Regular.ttf", "https://raw.githubusercontent.com/notofonts/notofonts.github.io/main/fonts/NotoSansBengali/hinted/ttf/NotoSansBengali-Regular.ttf",
     "b55c62ee531e3214da6c0701daecea89a52ba42db7d8206b92e6b51f397a3193"),
    ("NotoSansDevanagari-Regular.ttf", "https://raw.githubusercontent.com/notofonts/notofonts.github.io/main/fonts/NotoSansDevanagari/hinted/ttf/NotoSansDevanagari-Regular.ttf",
     "4e3c66638958c3e2ab5d37f47a8deb89fffeb7be9985c665a519bbc7ba762313"),
    ("NotoSansEthiopic-Regular.ttf", "https://raw.githubusercontent.com/notofonts/notofonts.github.io/main/fonts/NotoSansEthiopic/hinted/ttf/NotoSansEthiopic-Regular.ttf",
     "f6f7fc379db9438959a2b0527e7a2cf36ea9c84626d56ec444fff37fc24c3c10"),
    ("NotoSansGeorgian-Regular.ttf", "https://raw.githubusercontent.com/notofonts/notofonts.github.io/main/fonts/NotoSansGeorgian/hinted/ttf/NotoSansGeorgian-Regular.ttf",
     "d3e33254b09e7bb2c5cf0f17e554b80462056c5a107097f258d495168c3a9346"),
    ("NotoSansGujarati-Regular.ttf", "https://raw.githubusercontent.com/notofonts/notofonts.github.io/main/fonts/NotoSansGujarati/hinted/ttf/NotoSansGujarati-Regular.ttf",
     "9b5a7aaeeb649a2e75a49d8b006a1f87db1b61c0df3b001609f4e0725d88dbf6"),
    ("NotoSansGurmukhi-Regular.ttf", "https://raw.githubusercontent.com/notofonts/notofonts.github.io/main/fonts/NotoSansGurmukhi/hinted/ttf/NotoSansGurmukhi-Regular.ttf",
     "658d0207da305a1411c539a8b0bbeda64d4146e54fb4827facddb890b6b90d74"),
    ("NotoSansHebrew-Regular.ttf", "https://raw.githubusercontent.com/notofonts/notofonts.github.io/main/fonts/NotoSansHebrew/hinted/ttf/NotoSansHebrew-Regular.ttf",
     "cdefaf8efd47045f6820928eba84db5bed7557539328952b5f828315485e02ee"),
    ("NotoSansKannada-Regular.ttf", "https://raw.githubusercontent.com/notofonts/notofonts.github.io/main/fonts/NotoSansKannada/hinted/ttf/NotoSansKannada-Regular.ttf",
     "9ad74dc64838c6855b96f671fc08e425a58921b9d0c71712ea79c328a27e6e38"),
    ("NotoSansKhmer-Regular.ttf", "https://raw.githubusercontent.com/notofonts/notofonts.github.io/main/fonts/NotoSansKhmer/hinted/ttf/NotoSansKhmer-Regular.ttf",
     "e66675f2082788f0511a714bef5a1748928294b38c8e286a96ea73a864b5e605"),
    ("NotoSansLao-Regular.ttf", "https://raw.githubusercontent.com/notofonts/notofonts.github.io/main/fonts/NotoSansLao/hinted/ttf/NotoSansLao-Regular.ttf",
     "0a86e5e1ccfe34ca78c43fac6829dc751b42bcc469272a9a55325aae587bfbe7"),
    ("NotoSansMalayalam-Regular.ttf", "https://raw.githubusercontent.com/notofonts/notofonts.github.io/main/fonts/NotoSansMalayalam/hinted/ttf/NotoSansMalayalam-Regular.ttf",
     "c08de7fa8d032a5d6a4d120fb82c78cec60b362a4e73fa26360d89759ff2a7f9"),
    ("NotoSansMath-Regular.ttf", "https://raw.githubusercontent.com/notofonts/notofonts.github.io/main/fonts/NotoSansMath/hinted/ttf/NotoSansMath-Regular.ttf",
     "d51afd5739c7ba6c44fcab35a88160e25dfb69a2d4ad0bd99533f8d894af1f96"),
    ("NotoSansMongolian-Regular.ttf", "https://raw.githubusercontent.com/notofonts/notofonts.github.io/main/fonts/NotoSansMongolian/hinted/ttf/NotoSansMongolian-Regular.ttf",
     "10f9066fe5cedcaa41c17d471bcadc7fb22880ce81ff2c06578671768ff44fbb"),
    ("NotoSansMyanmar-Regular.ttf", "https://raw.githubusercontent.com/notofonts/notofonts.github.io/main/fonts/NotoSansMyanmar/hinted/ttf/NotoSansMyanmar-Regular.ttf",
     "fafce4db400bc0b214907ccdbfb0ad2f18a57bfefd08c8a571830b84088cf2fc"),
    ("NotoSansOriya-Regular.ttf", "https://raw.githubusercontent.com/notofonts/notofonts.github.io/main/fonts/NotoSansOriya/hinted/ttf/NotoSansOriya-Regular.ttf",
     "a16645d056017927406546aa78e4ce15e782fd8783467267b75450453d007415"),
    ("NotoSans-Regular.ttf", "https://raw.githubusercontent.com/notofonts/notofonts.github.io/main/fonts/NotoSans/hinted/ttf/NotoSans-Regular.ttf",
     "478c558ea716033cd60c03438f628dfa75694dcf6b5f6d505a2f05fd2b4f3823"),
    ("NotoSansSinhala-Regular.ttf", "https://raw.githubusercontent.com/notofonts/notofonts.github.io/main/fonts/NotoSansSinhala/hinted/ttf/NotoSansSinhala-Regular.ttf",
     "9e32612d47004552f3125e78648a9e2e7899a216ccd3cefbb93a9b5f4c809feb"),
    ("NotoSansSymbols2-Regular.ttf", "https://raw.githubusercontent.com/notofonts/notofonts.github.io/main/fonts/NotoSansSymbols2/hinted/ttf/NotoSansSymbols2-Regular.ttf",
     "c4a0a80f0041ce4be81e2478faad22776d23edb98ae3f0d19bd37044820ecf9d"),
    ("NotoSansSymbols-Regular.ttf", "https://raw.githubusercontent.com/notofonts/notofonts.github.io/main/fonts/NotoSansSymbols/hinted/ttf/NotoSansSymbols-Regular.ttf",
     "d0e98e9a2c046594c5021437273943be7e79e0fd980fde125279e22302212595"),
    ("NotoSansSyriac-Regular.ttf", "https://raw.githubusercontent.com/notofonts/notofonts.github.io/main/fonts/NotoSansSyriac/hinted/ttf/NotoSansSyriac-Regular.ttf",
     "4440929bf1a47bb50179e8d8495641d208c63d607355e451f29ea2d3e5343290"),
    ("NotoSansTamil-Regular.ttf", "https://raw.githubusercontent.com/notofonts/notofonts.github.io/main/fonts/NotoSansTamil/hinted/ttf/NotoSansTamil-Regular.ttf",
     "3c0a186feb3c63c7f6d63e1511dcdc144e745ae09b98e217c83f3e317974f6f9"),
    ("NotoSansTelugu-Regular.ttf", "https://raw.githubusercontent.com/notofonts/notofonts.github.io/main/fonts/NotoSansTelugu/hinted/ttf/NotoSansTelugu-Regular.ttf",
     "b274780b69d1d23fe84b55e809a152cb2ac5306d33864b1f87622f6971871aae"),
    ("NotoSansThaana-Regular.ttf", "https://raw.githubusercontent.com/notofonts/notofonts.github.io/main/fonts/NotoSansThaana/hinted/ttf/NotoSansThaana-Regular.ttf",
     "7543935bdcef770c9d3dd54222651b29040e5ec82f3bc58542392b2c7a9cbd3c"),
    ("NotoSansThai-Regular.ttf", "https://raw.githubusercontent.com/notofonts/notofonts.github.io/main/fonts/NotoSansThai/hinted/ttf/NotoSansThai-Regular.ttf",
     "61cf814eec46b294d6ea4401ac295d0cecd5207bd2331dcc5a15e7301d30ee44"),
    ("NotoSansTifinagh-Regular.ttf", "https://raw.githubusercontent.com/notofonts/notofonts.github.io/main/fonts/NotoSansTifinagh/hinted/ttf/NotoSansTifinagh-Regular.ttf",
     "8058054786bc572007193988654d1ded342bc2538c2d91c50d1a7238ea1a99cc"),
    ("NotoSerifTibetan-Regular.ttf", "https://raw.githubusercontent.com/notofonts/notofonts.github.io/main/fonts/NotoSerifTibetan/hinted/ttf/NotoSerifTibetan-Regular.ttf",
     "ee97bf3dc56e813651db734c9f35f8f1d41e7e31acf5f7d893e64ad22b292446"),
    ("cjk/NotoSansJP-Regular.otf", "https://raw.githubusercontent.com/notofonts/noto-cjk/main/Sans/SubsetOTF/JP/NotoSansJP-Regular.otf",
     "dff723ba59d57d136764a04b9b2d03205544f7cd785a711442d6d2d085ac5073"),
    ("cjk/NotoSansKR-Regular.otf", "https://raw.githubusercontent.com/notofonts/noto-cjk/main/Sans/SubsetOTF/KR/NotoSansKR-Regular.otf",
     "69975a0ac8472717870aefeab0a4d52739308d90856b9955313b2ad5e0148d68"),
    ("cjk/NotoSansSC-Regular.otf", "https://raw.githubusercontent.com/notofonts/noto-cjk/main/Sans/SubsetOTF/SC/NotoSansSC-Regular.otf",
     "faa6c9df652116dde789d351359f3d7e5d2285a2b2a1f04a2d7244df706d5ea9"),
    ("cjk/NotoSansTC-Regular.otf", "https://raw.githubusercontent.com/notofonts/noto-cjk/main/Sans/SubsetOTF/TC/NotoSansTC-Regular.otf",
     "5bab0cb3c1cf89dde07c4a95a4054b195afbcfe784d69d75c340780712237537"),
]


def sha256(path: pathlib.Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> int:
    check_only = "--check" in sys.argv
    bad = 0
    for rel, url, digest in FONTS:
        dest = ROOT / rel
        if not dest.exists():
            if check_only:
                print(f"EKSİK   {rel}")
                bad += 1
                continue
            dest.parent.mkdir(parents=True, exist_ok=True)
            print(f"indir   {rel}")
            req = urllib.request.Request(url, headers={"User-Agent": "seismira-font-fetch"})
            with urllib.request.urlopen(req, timeout=120) as resp:
                tmp = dest.with_suffix(dest.suffix + ".part")
                tmp.write_bytes(resp.read())
                tmp.replace(dest)
        if sha256(dest) != digest:
            print(f"HATALI  {rel} (SHA256 uyuşmuyor)")
            bad += 1
        else:
            print(f"tamam   {rel}")
    if bad:
        print(f"{bad} dosyada sorun var.")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
