"""Deney (karar desteği, 2026-11-24): NGA-West2 dört modelin (CY14, BSSA14, ASK14, CB14; pygmm 0.9.1)
ShakeMap istasyonlarında ln artıkları (A tipi: ShakeMap mesafeleri, istasyon Vs30). Çalıştırma: .venv/bin/python -I scripts/compare_ngaw2_models.py
Yalnızca sonuç karşılaştırma; uygulama kodu değildir. Sonuç özeti docs/dogrulama.md."""
import json, math, sys, warnings, statistics as st
import pygmm
warnings.filterwarnings("ignore")
import logging; logging.disable(logging.WARNING)

EV = {  # id: (stationlist, Mw, dip, mech, ztor, region)
 "kahramanmaras2023": ("data/raw/validation/kahramanmaras2023/stationlist.json", 7.8, 89.2, "SS", 0.0, "turkey"),
 "izmit1999": ("data/validation/izmit1999/stationlist.json", 7.6, 81.0, "SS", 0.0, "turkey"),
 "kumamoto2016": ("data/validation/kumamoto2016/stationlist.json", 7.0, 71.5, "SS", 0.0, "japan"),
 "ridgecrest2019": ("data/raw/validation/ridgecrest2019/stationlist.json", 7.1, 83.9, "SS", 0.0, "california"),
 "noto2024": ("data/raw/validation/noto2024/stationlist.json", 7.5, 38.1, "RS", 0.0, "japan"),
}
BINS=[(0,50),(50,100),(100,200),(200,300)]

def stations(path):
    for f in json.load(open(path))["features"]:
        p=f["properties"]
        if p.get("station_type")!="seismic": continue
        v=[]
        for c in p["channels"]:
            if c["name"].endswith("Z"): continue
            for a in c["amplitudes"]:
                if a["name"]=="pgv" and isinstance(a["value"],(int,float)) and a["value"]>0: v.append(a["value"])
        if len(v)!=2: continue
        d=p.get("distances",{})
        if d.get("rrup") is None or d.get("rjb") is None: continue
        vs=p.get("vs30")
        if not isinstance(vs,(int,float)): continue
        yield (sum(map(math.log,v))/2, d["rrup"], d["rjb"], d.get("rx", -d["rjb"]) if d.get("rx") is not None else -d["rjb"], min(max(vs,180),1000))

MODELS=[("CY14",pygmm.ChiouYoungs2014,False),("CY14r",pygmm.ChiouYoungs2014,True),
        ("BSSA14",pygmm.BooreStewartSeyhanAtkinson2014,False),("BSSA14r",pygmm.BooreStewartSeyhanAtkinson2014,True),
        ("ASK14",pygmm.AbrahamsonSilvaKamai2014,False),("ASK14r",pygmm.AbrahamsonSilvaKamai2014,True),
        ("CB14",pygmm.CampbellBozorgnia2014,False),("CB14r",pygmm.CampbellBozorgnia2014,True)]

for ev,(path,M,dip,mech,ztor,reg) in EV.items():
    rows=[s for s in stations(path) if s[1]<=300]
    res={m[0]:[] for m in MODELS}
    for obs,rrup,rjb,rx,vs in rows:
        for name,cls,regional in MODELS:
            r = reg if regional else ("california" if name.startswith("CY") else "global")
            if reg=="turkey" and regional:
                # Türkiye: yalnızca BSSA14'te açık bölge; diğerlerinde temel model.
                r = "turkey" if name.startswith("BSSA") else ("california" if name.startswith("CY") else "global")
            kw=dict(mag=M,dist_rup=max(rrup,0.1),dist_jb=rjb,dist_x=rx,depth_tor=ztor,dip=dip,mechanism=mech,v_s30=vs,region=r,width=20.0)
            try:
                pred=cls(pygmm.Scenario(**kw)).pgv
                res[name].append((rrup,obs-math.log(pred)))
            except Exception as e:
                pass
    print(f"## {ev} (n={len(rows)}, bölge={reg})")
    print("| model | ort ln | " + " | ".join(f"{a}-{b}" for a,b in BINS)+" |")
    for name,_,_ in MODELS:
        r=res[name]
        if not r: print(f"| {name} | – |"); continue
        bins=[]
        for a,b in BINS:
            x=[v for d,v in r if a<=d<b]; bins.append(f"{st.mean(x):+.2f}" if x else "–")
        print(f"| {name} | {st.mean(v for _,v in r):+.2f} | "+" | ".join(bins)+" |")
    ens=[]
    # eşit ağırlıklı ortalama (bölgesel sürümler)
    by=list(zip(*[res[n] for n in ("CY14r","BSSA14r","ASK14r","CB14r")]))
    if by:
        e=[(t[0][0], st.mean(v for _,v in t)) for t in by]
        bins=[]
        for a,b in BINS:
            x=[v for d,v in e if a<=d<b]; bins.append(f"{st.mean(x):+.2f}" if x else "–")
        print(f"| ort4r | {st.mean(v for _,v in e):+.2f} | "+" | ".join(bins)+" |")
    print()
