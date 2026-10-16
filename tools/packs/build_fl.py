"""Build the Florida pack's place and ZIP tables (data/packs/fl/) from
auction-tracker's checked place list.

    python3 tools/packs/build_fl.py <auction-tracker>/packages/radar/src/radar/data

Reads `cities_fl.json` and checks it against the two source files beside it,
`census_fl_places_2024.txt` (Census 2024 Gazetteer, public domain) and
`geonames_fl_zip.txt` (GeoNames postal codes, CC BY 4.0). Writes
`places.tsv` and `zips.tsv`; `pack.toml` and `docs/packs/fl.md` are written
by hand. The export is faithful: names, variants and ZIPs are copied, not
edited. build.rs drops variants that the place comparison rule already
covers (`ALTAMONTE SPGS`) and rejects anything that leads to two places.
"""
import csv, json, re, sys
from pathlib import Path

OUT = Path(__file__).resolve().parents[2] / "data" / "packs" / "fl"
KINDS = {"municipality", "place"}
SOURCES = {"original", "census", "geonames", "observed"}
NAME = re.compile(r"^[A-Z0-9]+( [A-Z0-9]+)*$")
# Census LSAD words at the end of a Gazetteer NAME ("Alachua city").
# Places whose name differs from the Gazetteer's: the postal name, or the
# Census name plus its LSAD word.
CENSUS_NAME = {"EVERGLADES CITY": "EVERGLADES"}
LSAD = re.compile(r" (city|town|village|CDP|municipality)$")


def fail(msg):
    sys.exit(f"build_fl: {msg}")


def letters(s):
    """Compare names on letters and digits only: `OPA-LOCKA` is `OPA LOCKA`,
    `ST. CLOUD` is `SAINT CLOUD`, `McIntosh` is `MC INTOSH`."""
    return re.sub(r"[^A-Z0-9]", "", s.upper().replace("ST. ", "SAINT "))


def census_names(path):
    """Gazetteer names, with the LSAD word dropped and without it after a
    comma (`Islamorada, Village of Islands village`), to each place's GEOID."""
    with path.open(encoding="utf-8") as f:
        rows = list(csv.DictReader(f, delimiter="\t"))
    out = {}
    for r in rows:
        name = LSAD.sub("", r["NAME"])
        for n in (name, name.split(",")[0]):
            out[letters(n)] = r["GEOID"]
    return out


def geonames_zips(path):
    out = {}
    with path.open(encoding="utf-8") as f:
        for line in f:
            r = line.rstrip("\n").split("\t")
            if r[4] != "FL":
                fail(f"geonames row not in FL: {r[:5]}")
            out[r[1]] = r[2].upper()
    return out


def main(src):
    src = Path(src)
    places = json.loads((src / "cities_fl.json").read_text(encoding="utf-8"))
    census = census_names(src / "census_fl_places_2024.txt")
    geonames = geonames_zips(src / "geonames_fl_zip.txt")

    # A variant that is the name of another Census place is that place, not
    # a misspelling (BELLVIEW, Escambia, listed for BELLEVIEW, Marion).
    dropped = []
    for p in places:
        own = census.get(letters(CENSUS_NAME.get(p["city"], p["city"])))
        keep = [v for v in p["variants"] if census.get(letters(v), own) == own]
        dropped += [f"{v} (for {p['city']})" for v in p["variants"] if v not in keep]
        p["variants"] = keep

    main_zip, shared = {}, {}
    for p in places:
        name = p["city"]
        for s in [name, *p["variants"]]:
            if not NAME.match(s):
                fail(f"{name}: {s!r} is not upper-case words")
        if p["kind"] not in KINDS:
            fail(f"{name}: kind {p['kind']!r}")
        if not p["sources"] or not set(p["sources"]) <= SOURCES:
            fail(f"{name}: sources {p['sources']}")
        if "geonames" in p["sources"] and not p["zips"]:
            fail(f"{name}: a GeoNames place with no ZIP")
        for z in p["zips"]:
            if z in main_zip:
                fail(f"{z}: main ZIP of {main_zip[z]} and {name}")
            if geonames.get(z) != name:
                fail(f"{z}: {name} here, {geonames.get(z)} in GeoNames")
            main_zip[z] = name
        for z in p["other_zips"]:
            shared.setdefault(z, []).append(name)
    missing = sorted(set(geonames) - set(main_zip))
    if missing:
        fail(f"GeoNames ZIPs with no place: {missing[:10]}")
    for z, names in shared.items():
        if z not in main_zip:
            fail(f"{z}: shared by {names} but no place's main ZIP")
        if main_zip[z] in names:
            fail(f"{z}: {main_zip[z]} lists it as main and shared")
    not_census = [p["city"] for p in places
                  if "census" in p["sources"]
                  and letters(CENSUS_NAME.get(p["city"], p["city"])) not in census]
    if not_census:
        fail(f"census places not in the Gazetteer: {not_census}")

    OUT.mkdir(parents=True, exist_ok=True)
    with (OUT / "places.tsv").open("w", encoding="utf-8", newline="") as f:
        f.write("name\tcounty\tkind\tsources\tvariants\n")
        for p in sorted(places, key=lambda p: p["city"]):
            f.write("\t".join([p["city"], p["county_name"].upper(), p["kind"],
                               ";".join(p["sources"]), ";".join(sorted(p["variants"]))]) + "\n")
    with (OUT / "zips.tsv").open("w", encoding="utf-8", newline="") as f:
        f.write("zip\tplace\tshared\n")
        for z in sorted(main_zip):
            f.write(f"{z}\t{main_zip[z]}\t{';'.join(sorted(shared.get(z, [])))}\n")
    nv = sum(len(p["variants"]) for p in places)
    print(f"build_fl: {len(places)} places, {nv} variants, {len(main_zip)} ZIPs "
          f"({len(shared)} shared) -> {OUT}")
    for d in dropped:
        print(f"build_fl: dropped variant {d}: another Census place")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        fail("usage: build_fl.py <auction-tracker radar data dir>")
    main(sys.argv[1])
