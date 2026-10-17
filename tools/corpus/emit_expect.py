"""Write key/slug/usps/reso expectations into corpus cases from the specs
(docs/key-format.md, docs/reso-mapping.md, Pub 28), using only each case's
hand-labeled `expect`. Never reads emitter output.

    python3 tools/corpus/emit_expect.py FILE.jsonl ...          add missing ones
    python3 tools/corpus/emit_expect.py --check FILE.jsonl ...  compare stored ones
"""
import json, re, sys, collections

SKIP_WARN = {"possibly_truncated","ambiguous_street_number","multiple_addresses","invalid_zip","unparsed_remainder","ambiguous_token"}

def plain(s):
    out=[]
    for k,c in enumerate(s):
        if c=='.':
            if k>0 and s[k-1].isdigit() and k+1<len(s) and s[k+1].isdigit(): out.append(c)
            continue
        if c in ",'’\";:()": continue
        out.append(c.upper())
    return " ".join("".join(out).split())

def join(*p): return " ".join(x for x in p if x)

def special(e):
    if e.get("po_box"): return "PO BOX "+plain(e["po_box"])
    if e.get("rural_route"):
        r=e["rural_route"]; s=f'{r["kind"]} {plain(r["route"])}'
        return s+(f' BOX {plain(r["box"])}' if r.get("box") else "")
    if e.get("military"):
        m=e["military"]; s=f'{m["kind"]} {plain(m["number"])}'
        return s+(f' BOX {plain(m["box"])}' if m.get("box") else "")
    if e.get("general_delivery"): return "GENERAL DELIVERY"
    return ""

def name_text(e):
    """The street's name; for a highway, the highway and any name words after
    the route (`HIGHWAY 66 FRONTAGE`, docs/key-format.md)."""
    nm=plain(e["street_name"]) if e.get("street_name") else ""
    if e.get("highway"):
        h=e["highway"]; return join(h.get("state",""),h["prefix"],plain(h["route"]),nm)
    return nm

def has_street(e): return bool(e.get("street_name") or e.get("highway"))

def dual(e):
    """A street and a PO Box in one record (Pub 28 section 215)."""
    return has_street(e) and bool(e.get("po_box"))

def unit_text(t,n):
    n=plain(n) if n else ""
    if t and n: return f"{t} {n}"
    if t: return t
    if n: return f"# {n}"
    return ""

def units(e):
    return join(unit_text(e.get("unit_type"),e.get("unit_number")),
                unit_text(e.get("second_unit_type"),e.get("second_unit_number")))

def street_line(e):
    return join(plain(e["street_number"]) if e.get("street_number") else "",
                e.get("street_dir_prefix",""),name_text(e),e.get("street_suffix",""),
                e.get("street_dir_suffix",""),units(e))

def line1(e):
    """The USPS delivery line. A dual address is the PO Box alone, without
    the street's units (section 215)."""
    if dual(e): return special(e)
    sp=special(e)
    return join(sp,units(e)) if sp else street_line(e)

def city_of(e): return e.get("military_post_office") or e.get("city","")

def zipfull(e):
    z=e.get("zip5","")
    return z+("-"+e["zip4"] if z and e.get("zip4") else "")

def last_line(e): return join(city_of(e),e.get("state",""),zipfull(e))

def keyseg(s):
    s=" ".join(s.replace(".","").replace("'","").upper().split())
    return "".join(c if (c.isascii() and (c.isalnum() or c in " /-")) else "".join("%%%02X"%b for b in c.encode()) for c in s)

def key(e,unit):
    st,z=e.get("state",""),e.get("zip5","")
    pre="k1:" if st and z else "k1n:"
    # A dual address keys on the street (docs/key-format.md).
    if e.get("po_box") and not has_street(e): body=["P",e["po_box"],st,z]
    elif e.get("rural_route"):
        r=e["rural_route"]; body=["R",r["kind"],r["route"],r.get("box",""),st,z]
    elif e.get("military"):
        m=e["military"]; body=["M",m["kind"],m["number"],m.get("box",""),e.get("military_post_office",""),st,z]
    elif e.get("general_delivery"): body=["G",st,z]
    elif e.get("street_name") or e.get("highway"):
        body=["S",e.get("street_number",""),e.get("street_dir_prefix",""),name_text(e),e.get("street_suffix",""),e.get("street_dir_suffix",""),st,z]
        if unit: body+=[e.get("unit_number",""),e.get("second_unit_number","")]
    else: return None
    return pre+body[0]+":"[:0]+"|".join([body[0]]+[keyseg(x) for x in body[1:]])[1:] if False else pre+"|".join([body[0]]+[keyseg(x) for x in body[1:]])

def slug(e):
    # A dual address follows the street, with its units (docs/key-format.md, Slug).
    l1=street_line(e) if dual(e) else line1(e)
    text=join(l1,city_of(e),e.get("state",""),e.get("zip5","")).replace("#"," unit ")
    return re.sub(r"[^a-z0-9]+","-",text.lower()).strip("-")

def reso(e):
    r={}
    # A dual address uses the street; the box is not emitted
    # (docs/reso-mapping.md).
    sp="" if dual(e) else special(e)
    num=e.get("street_number","") if not sp else ""
    if num: r["StreetNumber"]=plain(num)
    m=re.match(r"\d+",num or "")
    if m and len(m.group())<=10: r["StreetNumericNum"]=m.group()
    if m and len(m.group())<=10: r["StreetNumberNumeric"]=m.group(); r.pop("StreetNumericNum")
    if e.get("street_dir_prefix") and not sp: r["StreetDirPrefix"]=e["street_dir_prefix"]
    nm=sp or name_text(e)
    if nm: r["StreetName"]=nm
    if e.get("street_suffix") and not sp: r["StreetSuffix"]=e["street_suffix"]
    if e.get("street_dir_suffix") and not sp: r["StreetDirSuffix"]=e["street_dir_suffix"]
    def ru(t,n):
        n=plain(n) if n else ""
        if t and n: return f"{t} {n}"
        if t: return t
        if n: return "#"+n
        return ""
    u=join(ru(e.get("unit_type"),e.get("unit_number")),ru(e.get("second_unit_type"),e.get("second_unit_number")))
    if u: r["UnitNumber"]=u
    c=city_of(e)
    if c: r["City"]=c
    if e.get("state"): r["StateOrProvince"]=e["state"]
    if e.get("zip5"): r["PostalCode"]=e["zip5"]
    if e.get("zip4"): r["PostalCodePlus4"]=e["zip4"]
    ua=", ".join(x for x in [street_line(e) if dual(e) else line1(e),c,join(e.get("state",""),zipfull(e))] if x)
    if ua: r["UnparsedAddress"]=ua
    return r

def eligible(c):
    e=c["expect"]
    if set(c.get("warnings",[]))&SKIP_WARN or e.get("unparsed"): return None
    if "placeholder" in c.get("warnings",[]): return "placeholder"
    if not (e.get("street_name") or e.get("highway") or special(e)): return None
    return "ok"

def add(c, with_reso):
    kind=eligible(c)
    if not kind: return False
    e=c["expect"]; new={}
    if kind=="placeholder":
        new={"key":"none","key_unit":"none","slug":"none","usps":{}}
    else:
        new["key"]=key(e,False); new["key_unit"]=key(e,True); new["slug"]=slug(e)
        u={"line1":line1(e)}
        if (l:=last_line(e)): u["last_line"]=l
        new["usps"]=u
        if with_reso: new["reso"]=reso(e)
    changed=False
    out=collections.OrderedDict()
    for k,v in c.items():
        if k=="note": 
            for nk,nv in new.items():
                if nk not in c: out[nk]=nv; changed=True
            new={}
        out[k]=v
    for nk,nv in new.items():
        if nk not in c: out[nk]=nv; changed=True
    c.clear(); c.update(out)
    return changed

def computed(c):
    """The expectations this script derives for a case, or {} if none."""
    kind=eligible(c)
    if kind!="ok": return {}
    e=c["expect"]
    u={"line1":line1(e)}
    if (l:=last_line(e)): u["last_line"]=l
    out={"key":key(e,False),"key_unit":key(e,True),"slug":slug(e),"usps":u}
    if e.get("state") and e.get("zip5"): out["reso"]=reso(e)
    return out

def check(paths):
    """--check: every stored expectation must equal what the rules give.
    Run it after changing a rule here or in the docs it follows."""
    n=bad=0
    for path in paths:
        for l in open(path,encoding="utf-8"):
            if not l.strip(): continue
            c=json.loads(l)
            for k,v in computed(c).items():
                if k in c:
                    n+=1
                    if c[k]!=v:
                        bad+=1; print(f"{c['id']} {k}:\n  stored   {c[k]}\n  computed {v}")
    print(f"compared {n} expectations, {bad} differ")
    return bad==0

if __name__=="__main__":
    if sys.argv[1:2]==["--check"]:
        sys.exit(0 if check(sys.argv[2:]) else 1)
    n=0
    for path in sys.argv[1:]:
        lines=open(path,encoding="utf-8").read().splitlines()
        res=[]
        for l in lines:
            if not l.strip(): res.append(l); continue
            c=json.loads(l,object_pairs_hook=collections.OrderedDict)
            e=c["expect"]
            with_reso = bool(e.get("state") and e.get("zip5"))
            if add(c,with_reso): n+=1
            res.append(json.dumps(c,ensure_ascii=False))
        open(path,"w",encoding="utf-8").write("\n".join(res)+"\n")
    print("cases changed:",n)
