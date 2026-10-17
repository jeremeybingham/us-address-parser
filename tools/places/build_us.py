"""Build the national place list (data/places/) from public data.

    python3 tools/places/build_us.py [--src local-data/places]

Reads, from the source directory (checksums in its SHA256SUMS):

  census-gazetteer-2024/2024_Gaz_place_national.txt    places (public domain)
  census-gazetteer-2024/2024_Gaz_cousubs_national.txt  county subdivisions
  census-gazetteer-2024/2024_Gaz_counties_national.txt county centers
  census-place-county-2020/national_place_by_county2020.txt   place -> county
  census-place-county-2020/national_cousub2020.txt            subdivision -> county
  geonames-postal/us/US.txt, geonames-postal/pr/PR.txt        ZIP -> place (CC BY 4.0)

and writes `places.tsv` and `zips.tsv` (`places.toml` is written by hand). The rules
(what is kept, how names are spelled, how a place gets its county) are in
docs/places.md; every choice is also commented where it is made.

The builder stops on anything it cannot explain: a name that is not upper-case
words, two places that the parser's place comparison rule (`place_key`, copied
from crates/doorplate/src/lexicon/place_key.rs) would treat as one, a ZIP whose
place is missing, a state with no places.
"""
import argparse
import csv
import math
import re
import sys
import tomllib
import unicodedata
from collections import Counter, defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "data" / "places"

# 50 states, DC and Puerto Rico. Other territories and the military "states"
# (GeoNames has ZIPs but no Census places for them) are left out of this pass.
STATES = set(
    "AL AK AZ AR CA CO CT DE DC FL GA HI ID IL IN IA KS KY LA ME MD MA MI MN MS MO "
    "MT NE NV NH NJ NM NY NC ND OH OK OR PA PR RI SC SD TN TX UT VT VA WA WV WI WY".split()
)

NAME = re.compile(r"^[A-Z0-9]+( [A-Z0-9]+)*$")
KIND_RANK = {"municipality": 0, "place": 1, "subdivision": 2}

# Gazetteer NAME endings that are the legal/statistical area type, not part of
# the name ("Alachua city"). Longest first. Case matters: the Gazetteer writes
# the type in lower case, so "Carson City" (the name) is kept whole.
PLACE_TYPES = [
    "city and borough", "consolidated government", "unified government",
    "metropolitan government", "metro government", "urban county", "zona urbana",
    "comunidad", "municipality", "corporation", "borough", "village", "town",
    "city", "CDP",
]
# Subdivision types that are dropped like a place's (a "town" in New York is
# called by its bare name in an address) ...
SUB_BARE = ["city", "town", "village", "borough"]
# ... and types kept in the name, because mail writes them ("ROARING BROOK
# TWP"). The bare name and the abbreviation become variants.
SUB_KEPT = {"township": "TWP", "plantation": "PLT", "gore": None, "grant": None,
            "purchase": None, "location": None}
# Census names that differ from the name a curated pack uses for the same
# place (docs/places.md). Without this the pack's EVERGLADES CITY and the
# Census's EVERGLADES would be two places. Keyed by (state, name after
# cleaning).
CENSUS_NAMES = {("FL", "EVERGLADES"): "EVERGLADES CITY"}
# Gazetteer subdivisions that are only statistical or administrative lines.
SUB_SKIP_WORDS = {"CCD", "UT", "precinct", "district", "barrio", "subbarrio",
                  "defined", "unorganized", "reservation", "ward"}


def fail(msg):
    sys.exit(f"build_us: {msg}")


def read_tsv(path, delimiter):
    with path.open(encoding="utf-8-sig", newline="") as f:
        rows = csv.reader(f, delimiter=delimiter)
        header = [h.strip() for h in next(rows)]
        return [dict(zip(header, (v.strip() for v in r))) for r in rows if r]


# ---- names -----------------------------------------------------------------

def norm(s):
    """Upper-case words of letters and digits: accents folded (`Añasco`),
    apostrophes and periods dropped (`O'Fallon`, `St. Louis`), every other
    punctuation mark a space (`Winston-Salem`), `ST`, `FT` and `MT` written
    out (`SAINT`, `FORT`, `MOUNT`), as the Florida pack does."""
    s = unicodedata.normalize("NFKD", s)
    s = "".join(c for c in s if not unicodedata.combining(c))
    s = re.sub(r"[.'’`‘ʻ]", "", s).replace("&", " AND ")
    s = re.sub(r"[^A-Za-z0-9]+", " ", s).upper().split()
    s = [{"ST": "SAINT", "FT": "FORT", "MT": "MOUNT"}.get(w, w) for w in s]
    return " ".join(s)


def load_expand():
    """Spelling -> full word, as build.rs makes it from the suffix and
    directional tables (directionals win for a word that is both)."""
    ex = {}
    for fname, table in (("suffixes.toml", "suffix"), ("directionals.toml", "directional")):
        t = tomllib.loads((ROOT / "data" / fname).read_text(encoding="utf-8"))
        for r in t[table]:
            for f in r["forms"] + r.get("observed", []):
                ex[f] = r["name"]
    return ex


EXPAND = load_expand()


def place_key(name):
    """The parser's place comparison rule (lexicon/place_key.rs)."""
    out = []
    for w in re.split(r"[\s\-/]+", name):
        w = "".join(c for c in w if c.isalnum()).upper()
        if not w:
            continue
        w = {"ST": "SAINT", "FT": "FORT", "MT": "MOUNT"}.get(w) or EXPAND.get(w, w)
        out.append(w)
    return "".join(out)


CONSOLIDATED = {"consolidated government", "unified government",
                "metropolitan government", "metro government", "urban county"}


def place_names(raw, state):
    """A Gazetteer name as (name, variants): `Alachua city` -> `Alachua`;
    `Milford city (balance)` -> `Milford`; `Islamorada, Village of Islands
    village` -> `Islamorada`. A consolidated city-county is called by its
    city (`Athens-Clarke County unified government (balance)` is ATHENS), with
    the full name a variant. Massachusetts writes `Amherst Town` for the city
    form of a town; mail says AMHERST."""
    name = raw.strip()
    balance = name.endswith("(balance)")
    name = re.sub(r"\s*\(balance\)$", "", name).split(",")[0]
    kind = None
    for t in PLACE_TYPES:
        if name.endswith(" " + t) and len(name) > len(t) + 1:
            name, kind = name[: -len(t) - 1], t
            break
    variants = []
    if (kind in CONSOLIDATED or balance) and re.search(r"[-/]", name):
        variants.append(name)
        name = re.split(r"[-/]", name)[0]
    if state == "MA" and name.endswith(" Town"):
        variants.append(name)
        name = name[: -len(" Town")]
    return name, variants


def county_name(raw):
    """County names without the type word (`Autauga County` -> `AUTAUGA`),
    except where the word is part of the name (`Baltimore city`)."""
    n = re.sub(r" (County|Parish|Municipio|Municipality)$", "", raw.strip())
    n = re.sub(r" (City and Borough|Borough|Census Area)$", "", n)
    return norm(n)


# ---- sources ---------------------------------------------------------------

def load_counties(src):
    """GEOID -> (name, lat, lon), from the Gazetteer's counties file."""
    out = {}
    for r in read_tsv(src / "census-gazetteer-2024" / "2024_Gaz_counties_national.txt", "\t"):
        out[r["GEOID"]] = (r["NAME"], float(r["INTPTLAT"]), float(r["INTPTLONG"]))
    return out


def nearest(counties, geoids, lat, lon):
    """The county whose center is nearest the place's internal point. The
    2020 list names every county a place touches but not how much of it lies
    in each; this is the rule for the 1,304 that span several."""
    def d(g):
        if g not in counties:  # Connecticut's counties, replaced in 2022
            return (float("inf"), g)
        _, clat, clon = counties[g]
        dx = (clon - lon) * math.cos(math.radians(lat))
        return ((clat - lat) ** 2 + dx ** 2, g)
    return min(geoids, key=d)


def census_places(src, counties, stats):
    """Gazetteer places: (state, name, kind, county or '', variants)."""
    by_county = defaultdict(set)
    names = {}
    for r in read_tsv(src / "census-place-county-2020" / "national_place_by_county2020.txt", "|"):
        by_county[r["STATEFP"] + r["PLACEFP"]].add(r["STATEFP"] + r["COUNTYFP"])
        names[r["STATEFP"] + r["COUNTYFP"]] = r["COUNTYNAME"]
    rows = []
    for r in read_tsv(src / "census-gazetteer-2024" / "2024_Gaz_place_national.txt", "\t"):
        if r["USPS"] not in STATES:
            stats["places_skipped_state"] += 1
            continue
        geoids = by_county.get(r["GEOID"])
        county = ""
        if geoids:
            if len(geoids) > 1:
                stats["places_multi_county"] += 1
            g = nearest(counties, sorted(geoids), float(r["INTPTLAT"]), float(r["INTPTLONG"]))
            county = county_name(names[g])
        else:
            stats["places_not_in_2020_list"] += 1
        # CDPs (and Puerto Rico's comunidades and zonas urbanas) have no city
        # government: the pack's `place`. Everything else is a municipality.
        kind = "place" if r["LSAD"] in ("57", "55", "62") else "municipality"
        name, variants = place_names(r["NAME"], r["USPS"])
        rows.append((r["USPS"], name, kind, county, [norm(v) for v in variants]))
    return rows


def census_subdivisions(src, counties, stats):
    """Active county subdivisions that mail can be addressed to: townships
    and the like, and the towns that are not Census places (New England, New
    York, Wisconsin). Returns (state, name, variants, county or '')."""
    sub_county = {}
    for r in read_tsv(src / "census-place-county-2020" / "national_cousub2020.txt", "|"):
        sub_county[r["STATEFP"] + r["COUNTYFP"] + r["COUSUBFP"]] = county_name(r["COUNTYNAME"])
    rows = []
    for r in read_tsv(src / "census-gazetteer-2024" / "2024_Gaz_cousubs_national.txt", "\t"):
        if r["USPS"] not in STATES:
            continue
        raw = r["NAME"]
        word = raw.split()[-1]
        if r["FUNCSTAT"] != "A" or word in SUB_SKIP_WORDS or any(
            w in SUB_SKIP_WORDS for w in raw.split()
        ):
            stats["subs_skipped_type"] += 1
            continue
        base, variants = None, []
        if word in SUB_BARE and len(raw) > len(word) + 1:
            base, variants = place_names(raw, r["USPS"])
        elif word in SUB_KEPT and len(raw) > len(word) + 1:
            bare = raw[: -len(word) - 1]
            base = raw
            variants.append(bare)
            if SUB_KEPT[word]:
                variants.append(f"{bare} {SUB_KEPT[word]}")
        else:
            stats["subs_skipped_type"] += 1
            continue
        name = norm(base)
        # Numbered townships ("Township 12") are not names anyone writes.
        if not re.search(r"[A-Z]{2}", name) or re.fullmatch(r"(TOWNSHIP|TOWN) \d+", name):
            stats["subs_skipped_numbered"] += 1
            continue
        county = sub_county.get(r["GEOID"], "")
        rows.append((r["USPS"], name, [norm(v) for v in variants], county))
    return rows


def geonames_zips(src, stats):
    """(zip, state, place, county) rows. GeoNames lists one place per ZIP;
    military ZIPs (no state) and territories other than Puerto Rico are
    skipped. Puerto Rico is its own file with the municipio in column 4."""
    rows = []
    for path, pr in ((src / "geonames-postal" / "us" / "US.txt", False),
                     (src / "geonames-postal" / "pr" / "PR.txt", True)):
        for line in path.read_text(encoding="utf-8").splitlines():
            f = line.split("\t")
            st = "PR" if pr else f[4]
            if st not in STATES:
                stats["zips_skipped_state"] += 1
                continue
            if not re.fullmatch(r"\d{5}", f[1]):
                fail(f"{path.name}: ZIP {f[1]!r}")
            rows.append((f[1], st, f[2], f[3] if pr else f[5]))
    return rows


# ---- build -----------------------------------------------------------------

class Place:
    def __init__(self, state, name, kind, county):
        self.state, self.name, self.kind, self.county = state, name, kind, county
        self.sources, self.variants = set(), set()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--src", default=str(ROOT / "local-data" / "places"))
    ap.add_argument("--out", default=str(OUT))
    args = ap.parse_args()
    src, out = Path(args.src), Path(args.out)
    stats = Counter()
    counties = load_counties(src)

    places = {}  # (state, place_key) -> Place
    merged_spellings = []

    def add(state, raw_name, kind, county, source):
        name = raw_name if NAME.match(raw_name) else norm(raw_name)
        if not NAME.match(name):
            fail(f"{state} {raw_name!r}: not upper-case words after cleaning ({name!r})")
        k = (state, place_key(name))
        p = places.get(k)
        if p is None:
            p = places[k] = Place(state, name, kind, county)
        else:
            if p.name != name:
                merged_spellings.append(f"{state} {p.name} / {name}")
            if KIND_RANK[kind] < KIND_RANK[p.kind]:
                p.kind, p.name = kind, name
            if not p.county:
                p.county = county
        p.sources.add(source)
        return p

    pending_variants = []
    for st, raw, kind, county, variants in census_places(src, counties, stats):
        raw = CENSUS_NAMES.get((st, norm(raw)), raw)
        pending_variants.append((add(st, raw, kind, county, "census"), variants))
    subs = census_subdivisions(src, counties, stats)
    # A subdivision name used by several in a state (Washington township, in
    # 20 counties) has no one county.
    sub_counts = Counter((st, place_key(n)) for st, n, _, _ in subs)
    for st, name, variants, county in subs:
        multi = sub_counts[(st, place_key(name))] > 1
        p = add(st, name, "subdivision", "" if multi else county, "census")
        if multi and p.kind == "subdivision":
            p.county = ""
        pending_variants.append((p, variants))

    zips = {}
    for z, st, raw, county in geonames_zips(src, stats):
        if z in zips:
            stats["zips_duplicate"] += 1
            continue
        p = add(st, norm(raw), "place", county_name(county) if county else "", "geonames")
        zips[z] = p
    stats["places_from_geonames_only"] = sum(
        1 for p in places.values() if p.sources == {"geonames"}
    )

    # Variants last, so a bare subdivision name never takes a place's key.
    for p, variants in pending_variants:
        for v in variants:
            k = (p.state, place_key(v))
            if k in places and places[k] is not p:
                stats["variants_dropped_taken"] += 1
            else:
                p.variants.add(v)
    # A variant two subdivisions share is dropped from both.
    owners = defaultdict(list)
    for p in places.values():
        for v in p.variants:
            owners[(p.state, place_key(v))].append(p)
    for k, ps in owners.items():
        if len(ps) > 1:
            stats["variants_dropped_shared"] += len(ps)
            for p in ps:
                p.variants = {v for v in p.variants if (p.state, place_key(v)) != k}

    # ---- checks --------------------------------------------------------
    seen = {}
    for p in places.values():
        for s in (p.name, *p.variants):
            if not NAME.match(s):
                fail(f"{p.state} {s!r}: not upper-case words")
            k = (p.state, place_key(s))
            if seen.setdefault(k, p) is not p:
                fail(f"{p.state}: {s!r} and {seen[k].name!r} have the same comparison key")
    for z, p in zips.items():
        if places.get((p.state, place_key(p.name))) is not p:
            fail(f"{z}: place {p.name!r} is not in the list")
    missing = STATES - {p.state for p in places.values()}
    if missing:
        fail(f"no places for {sorted(missing)}")
    nozip_states = STATES - {p.state for p in zips.values()}
    if nozip_states:
        fail(f"no ZIPs for {sorted(nozip_states)}")

    # ---- write ---------------------------------------------------------
    out.mkdir(parents=True, exist_ok=True)
    ordered = sorted(places.values(), key=lambda p: (p.state, p.name))
    with (out / "places.tsv").open("w", encoding="utf-8", newline="") as f:
        f.write("state\tname\tcounty\tkind\tsources\tvariants\n")
        for p in ordered:
            f.write("\t".join([p.state, p.name, p.county, p.kind, ";".join(sorted(p.sources)),
                               ";".join(sorted(p.variants))]) + "\n")
    with (out / "zips.tsv").open("w", encoding="utf-8", newline="") as f:
        f.write("zip\tstate\tplace\n")
        for z in sorted(zips):
            f.write(f"{z}\t{zips[z].state}\t{zips[z].name}\n")

    kinds = Counter(p.kind for p in ordered)
    print(f"build_us: {len(ordered)} places ({dict(kinds)}), "
          f"{sum(len(p.variants) for p in ordered)} variants, {len(zips)} ZIPs -> {out}")
    print(f"build_us: {sum(1 for p in ordered if not p.county)} places with no county")
    for k in sorted(stats):
        print(f"build_us: {k} = {stats[k]}")
    print(f"build_us: {len(merged_spellings)} places merged under another spelling "
          f"(first 8: {merged_spellings[:8]})")


if __name__ == "__main__":
    main()
