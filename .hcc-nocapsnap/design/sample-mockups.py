import statistics as st
from PIL import Image
import os
D=os.path.join(os.path.dirname(os.path.abspath(__file__)), "mockups") + "/"
def load(n): return Image.open(D+n).convert("RGB")
def hx(c): return "#%02X%02X%02X"%c
def patch(im,x,y,r=5):
    px=[im.getpixel((i,j)) for i in range(x-r,x+r+1) for j in range(y-r,y+r+1)]
    return tuple(int(st.median(p[k] for p in px)) for k in range(3))
def lum(c):
    f=lambda v:(v/255)/12.92 if v/255<=0.03928 else (((v/255)+0.055)/1.055)**2.4
    return 0.2126*f(c[0])+0.7152*f(c[1])+0.0722*f(c[2])
def glyph(im,box,mode="bright",frac=0.06):
    px=[im.getpixel((i,j)) for i in range(box[0],box[2]) for j in range(box[1],box[3])]
    key=(lambda p:lum(p)) if mode=="bright" else (lambda p:-lum(p)) if mode=="dark" else (lambda p:max(p)-min(p))
    px.sort(key=key,reverse=True); top=px[:max(5,int(len(px)*frac))]
    return tuple(int(st.median(p[k] for p in top)) for k in range(3))
def cr(a,b):
    la,lb=sorted([lum(a),lum(b)],reverse=True); return (la+0.05)/(lb+0.05)
h,p,r,hi,q=load("01-home.webp"),load("02-prepare-which-dish.webp"),load("03-review-plate-as-served.webp"),load("04-history-offline.webp"),load("05-guest-invitation-qr.webp")
S={}
S["bg (home top)"]=patch(h,560,112); S["bg (home margin)"]=patch(h,24,1470); S["bg (QR screen)"]=patch(q,560,30)
S["headline cream"]=glyph(h,(65,190,985,300)); S["kicker text"]=glyph(h,(65,145,570,172))
S["location pill bg"]=patch(h,200,341,3); S["hero-less card: stat card bg"]=patch(h,300,1296)
S["stat label text"]=glyph(h,(183,1362,390,1395)); S["recent card bg"]=patch(h,720,1600)
S["CTA yellow"]=patch(h,150,1175); S["CTA ink"]=glyph(h,(475,1152,755,1202),"dark")
S["QR-ready chip text (green)"]=glyph(h,(385,1690,495,1718),"sat"); S["QR-ready chip bg"]=patch(h,505,1704,2)
S["time text (muted)"]=glyph(h,(308,1750,505,1778)); S["nav active gold"]=patch(h,143,1893,3)
S["nav inactive text"]=glyph(h,(380,1932,466,1958)); S["online dot"]=patch(h,935,60,3)
S["search/input bg"]=patch(p,700,455); S["dish card (unselected)"]=patch(p,900,940); S["dish card (selected)"]=patch(p,900,690)
S["selected border/check gold"]=patch(p,1003,745,2); S["helper text (staff ref)"]=glyph(p,(64,1612,356,1640))
S["review info card bg"]=patch(r,900,1440); S["review kicker"]=glyph(r,(400,168,725,193)); S["review footnote"]=glyph(r,(405,1912,720,1938))
S["offline amber"]=glyph(hi,(957,55,1060,85),"sat"); S["history banner bg"]=patch(hi,900,470); S["history list card bg"]=patch(hi,900,850)
S["waiting chip text (amber)"]=glyph(hi,(452,988,662,1016),"sat"); S["waiting chip bg"]=patch(hi,675,1001,2)
S["last-synced label"]=glyph(hi,(58,655,228,683))
S["QR card cream"]=patch(q,300,812); S["QR module ink"]=patch(q,382,935,3); S["Done yellow"]=patch(q,200,1740); S["concept-QR note"]=glyph(q,(400,1452,725,1478))
for k,v in S.items(): print(f"{k:32s} {hx(v)}")
print("\n--- contrast (measured pairs)")
bg=S["bg (home top)"]
pairs=[("headline cream","bg (home top)"),("kicker text","bg (home top)"),("stat label text","hero-less card: stat card bg"),
("time text (muted)","recent card bg"),("CTA ink","CTA yellow"),("QR-ready chip text (green)","QR-ready chip bg"),
("nav inactive text","bg (home top)"),("nav active gold","bg (home top)"),("helper text (staff ref)","bg (home top)"),
("review kicker","bg (home top)"),("review footnote","bg (home top)"),("offline amber","bg (home top)"),
("waiting chip text (amber)","waiting chip bg"),("last-synced label","bg (home top)"),("QR module ink","QR card cream"),
("concept-QR note","bg (QR screen)"),("hero-less card: stat card bg","bg (home top)")]
for a,b in pairs:
    c=cr(S[a],S[b]); g="AAA" if c>=7 else "AA" if c>=4.5 else "AA-large" if c>=3 else "FAIL"
    print(f"{a:28s} on {b:28s} {c:5.2f}:1 {g}")
brand={"Royal Purple":(0x2D,0x1B,0x4E),"Dark Purple":(0x1A,0x1A,0x2E),"Bamboo Brown":(0x74,0x42,0x10),"Gold":(0xF6,0xE0,0x5E),"Bamboo Green":(0x68,0xD3,0x91),"Muted Gold":(0xB7,0x79,0x1F)}
print("\n--- brand sheet vs mockup")
for n,c in brand.items(): print(f"{n:14s} {hx(c)}")
