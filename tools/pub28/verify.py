#!/usr/bin/env python3
"""Compare data/*.toml with the live Pub 28 tables (docs/design.md item 44).

    python3 tools/pub28/verify.py --cache DIR [--fetch] [--emitter]

Reads Appendix C1 (suffixes), C2 (units), B (states, directionals, military
"states") from DIR and compares them with data/suffixes.toml, units.toml,
states.toml and directionals.toml. With --fetch it downloads the pages into DIR
first (a browser User-Agent is needed: pe.usps.com sends other clients a
"page not found" page). DIR is outside the repository: the pages are USPS
content, not ours. Exits 1 if anything differs. Formatting to Pub 28 is not
validation; this checks our tables against the publication, nothing more.
"""
import argparse, sys, time, tomllib, urllib.request
from html.parser import HTMLParser
from pathlib import Path

BASE = "https://pe.usps.com/text/pub28/"
PAGES = ["pub28apc_002", "pub28apc_003", "28apb", "28apf"]
UA = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36"
ROOT = Path(__file__).resolve().parents[2]


class Tables(HTMLParser):
    def __init__(self):
        super().__init__()
        self.tables, self.stack, self.cell = [], [], None

    def handle_starttag(self, tag, attrs):
        if tag == "table":
            self.stack.append([])
        elif tag == "tr" and self.stack:
            self.stack[-1].append([])
        elif tag in ("td", "th") and self.stack and self.stack[-1]:
            self.cell = []
        elif tag == "br" and self.cell is not None:
            self.cell.append(" ")

    def handle_endtag(self, tag):
        if tag in ("td", "th") and self.cell is not None and self.stack and self.stack[-1]:
            # Appendix C1 has a literal "\u2009BYU" typo in the page source.
            text = "".join(self.cell).replace("\\u2009", "")
            self.stack[-1][-1].append(" ".join(text.split()))
            self.cell = None
        elif tag == "table" and self.stack:
            self.tables.append(self.stack.pop())

    def handle_data(self, data):
        if self.cell is not None:
            self.cell.append(data)


def tables(path):
    p = Tables()
    p.feed(path.read_text(encoding="utf8", errors="replace"))
    return p.tables


def fetch(cache):
    cache.mkdir(parents=True, exist_ok=True)
    for page in PAGES:
        req = urllib.request.Request(BASE + page + ".htm", headers={"User-Agent": UA})
        (cache / f"{page}.htm").write_bytes(urllib.request.urlopen(req, timeout=60).read())
        time.sleep(0.5)


def load(name):
    return tomllib.loads((ROOT / "data" / name).read_text())


problems = []


def bad(msg):
    problems.append(msg)
    print("DIFF:", msg)


def check_suffixes(cache):
    rows = [r for r in max(tables(cache / "pub28apc_002.htm"), key=len)][1:]
    groups, cur = [], None
    for r in rows:
        if len(r) == 3 and r[2]:
            cur = {"abbr": r[2], "spell": set(), "primary": {r[0]}}
            groups.append(cur)
        if cur is None:
            continue
        cur["spell"].update(c for c in r if c)
    c1 = {}
    for g in groups:
        c1.setdefault(g["abbr"], set()).update(g["spell"])
    ours = {s["abbr"]: s for s in load("suffixes.toml")["suffix"]}
    print(f"suffixes: C1 {len(c1)} abbreviations, ours {len(ours)}")
    for abbr in sorted(set(c1) - set(ours)):
        bad(f"suffix {abbr} is in C1 and not in suffixes.toml")
    for abbr in sorted(set(ours) - set(c1)):
        bad(f"suffix {abbr} is in suffixes.toml and not in C1")
    for abbr in sorted(set(c1) & set(ours)):
        have = set(ours[abbr]["forms"]) | set(ours[abbr].get("observed", []))
        for sp in sorted(c1[abbr] - have):
            # C1 lists some spellings under two abbreviations (MDW); look elsewhere
            if not any(sp in set(o["forms"]) | set(o.get("observed", [])) for o in ours.values()):
                bad(f"suffix spelling {sp} (C1 row {abbr}) is not in suffixes.toml")
        for sp in sorted(set(ours[abbr]["forms"]) - c1[abbr]):
            bad(f"suffix {abbr}: form {sp} is not listed by C1 under that row (should it be observed?)")


def check_units(cache):
    rows = max(tables(cache / "pub28apc_003.htm"), key=len)[1:]
    c2 = {}
    for r in rows:
        if len(r) == 2 and r[1] and not r[0].startswith("*"):
            c2[r[1].rstrip("*")] = (r[0].upper(), r[1].endswith("**"))
    ours = {u["abbr"]: u for u in load("units.toml")["unit"]}
    print(f"units: C2 {len(c2)} designators, ours {len(ours)}")
    for abbr, (name, no_range) in sorted(c2.items()):
        u = ours.get(abbr)
        if u is None:
            bad(f"unit {abbr} ({name}) is in C2 and not in units.toml")
            continue
        if u["range_required"] == no_range:
            bad(f"unit {abbr}: range_required is {u['range_required']} but C2 {'does not require' if no_range else 'requires'} a range")
        if name not in (u["name"], *u["forms"]):
            bad(f"unit {abbr}: C2 name {name} is not in our name or forms")
    for abbr in sorted(set(ours) - set(c2)):
        print(f"  note: unit {abbr} is not in C2 (expected: PMB from section 285)")


def check_states(cache):
    ts = tables(cache / "28apb.htm")
    st = next(t for t in ts if t and t[0][:1] == ["State/Possession"])[1:]
    dr = next(t for t in ts if t and t[0][:1] == ["Geographic Directional"])[1:]
    mil = next(t for t in ts if t and t[0][:1] and t[0][0].startswith("Military"))[1:]
    states = {s["code"]: s for s in load("states.toml")["state"]}
    print(f"states: B {len(st)} codes + {len(mil)} military, ours {len(states)}")
    for name, code in st:
        s = states.get(code)
        if s is None:
            bad(f"state {code} ({name}) is in Appendix B and not in states.toml")
        elif name.upper() != s["name"] and name.upper() not in s.get("aliases", []):
            print(f"  note: {code}: B says {name!r}, we say {s['name']!r}")
    for name, code in mil:
        if code not in states:
            bad(f"military {code} ({name}) is not in states.toml")
    covered = {c for _, c in st} | {c for _, c in mil}
    for code in sorted(set(states) - covered):
        bad(f"states.toml has {code}, which Appendix B does not list")
    dirs = {d["abbr"]: d for d in load("directionals.toml")["directional"]}
    print(f"directionals: B {len(dr)}, ours {len(dirs)}")
    for name, abbr in dr:
        d = dirs.get(abbr)
        if d is None or d["name"] != name.upper():
            bad(f"directional {name} = {abbr} differs from directionals.toml")
    for abbr in sorted(set(dirs) - {a for _, a in dr}):
        bad(f"directionals.toml has {abbr}, which Appendix B does not list")


# Appendix F examples that this library deliberately does not standardize.
F_SKIP = {
    # SR is STATE ROAD or STATE ROUTE (Appendix F maps it to both), so the base
    # tables leave it to state packs (docs/usps-pub28-notes.md).
    "SR MM": "SR is left to state packs",
}


def check_appendix_f(cache):
    import re

    import doorplate

    t = max(tables(cache / "28apf.htm"), key=len)
    n = 0
    for row in t[1:]:
        if len(row) < 2 or not row[0] or not row[1]:
            continue
        example = re.sub(r"\s*\(excessive characters\)", "", row[0]).strip()
        n += 1
        if example in F_SKIP:
            continue
        want = f"123 {row[1].strip()}"
        got = doorplate.parse(f"123 {example}, Tampa FL 33602").to_usps()["line1"]
        if got != want:
            bad(f"Appendix F: {example!r} should be {want!r}, got {got!r}")
    print(f"appendix F: {n} examples, {len(F_SKIP)} skipped")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--cache", required=True, type=Path)
    ap.add_argument("--fetch", action="store_true")
    ap.add_argument("--emitter", action="store_true", help="also check Appendix F through the Python package")
    a = ap.parse_args()
    if a.fetch:
        fetch(a.cache)
    for fn in (check_suffixes, check_units, check_states):
        fn(a.cache)
    if a.emitter:
        check_appendix_f(a.cache)
    print(f"{len(problems)} difference(s)")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
