"""Draw the blind Florida sample for calibrated confidence (docs/design.md
Milestone 7, item 36) and write it as draft corpus records to label by hand.

    uv run --with pyarrow tools/calibrate/sample_fl.py [--seed N] [--out DIR]
    python3 tools/calibrate/sample_fl.py --status [--out DIR]

Sources (all in `local-data/florida/`, never committed):

- `raw_addresses.csv`, the auction listings: column `raw_address`, parsed as
  one line with `default_state` FL (how that data is parsed in use);
- the DBPR licensee exports (`tools/packs/dbpr.py`): the real estate parquet
  and the construction, cosmetology and `lic03bb` board files, Florida rows
  only, parsed as fields. Only address columns are read; licensee names
  never leave the source files.

The draw is uniform over distinct addresses with a fixed seed, so it can be
repeated. Inputs already in `tests/corpus/` are skipped. A top-up stratum
(`stratum=flagged` in the note) is drawn only from rows whose parse raised a
warning other than the missing_* ones, or left text unparsed, so the hard
cases have enough examples; item 38 weights it back down.

Each record's `expect` and `warnings` are pre-filled from the parser's
current output and its note ends in `TODO`. Labeling means correcting them
to what is right and deleting the `TODO` (`local-data/florida-sample/
README.md`). `--status` counts what is left. The script refuses to overwrite
an existing sample.
"""
import argparse
import csv
import json
import random
import re
import subprocess
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools" / "packs"))
import dbpr  # noqa: E402

FL = ROOT / "local-data" / "florida"
DBPR = [
    ("dbpr-re", 50, ["dbpr_20251010_113032.parquet"]),
    ("dbpr-construction", 17, ["CONSTRUCTIONLICENSE_1.csv"]),
    ("dbpr-cosmetology", 17, ["COSMETOLOGYLICENSE_1.csv"]),
    ("dbpr-lic03bb", 16, ["lic03bb.csv"]),
]
AUCTION = ("auction", 50)
FLAGGED = 40
# Standard highway prefixes (`data/special.toml` `[[highway]]`).
PREFIXES = {
    h["standard"]
    for h in tomllib.loads((ROOT / "data" / "special.toml").read_text()).get("highway", [])
}
# Warnings that say only that a part is absent; any other one flags a row.
PLAIN = {"missing_city", "missing_state", "missing_zip", "default_state_applied"}


def norm(s):
    return re.sub(r"\s+", " ", (s or "").strip().upper())


def corpus_inputs():
    """Every input already in the golden corpus, normalized."""
    seen = set()
    for p in (ROOT / "tests" / "corpus").glob("*.jsonl"):
        for line in p.read_text().splitlines():
            if not line.strip():
                continue
            c = json.loads(line)
            if "input" in c:
                seen.add(norm(c["input"]))
            for l in c.get("lines", []):
                seen.add(norm(l))
            if "fields" in c:
                seen.add(norm(c["fields"].get("line1")))
    return seen


def auction_pool():
    with (FL / "raw_addresses.csv").open(encoding="utf-8", newline="") as f:
        return sorted({r["raw_address"].strip() for r in csv.DictReader(f) if r["raw_address"].strip()})


def dbpr_pool(files):
    rows = set()
    for name in files:
        for l1, l2, city, z in dbpr.florida_rows(FL / name):
            if l1.strip():
                rows.add((l1.strip(), l2.strip(), city.strip(), z.strip()))
    return sorted(rows)


def addr_bin():
    exe = ROOT / "target" / "release" / "addr"
    subprocess.run(
        ["cargo", "build", "-q", "--release", "-p", "addr-cli"], cwd=ROOT, check=True
    )
    return exe


def parse(exe, record):
    if "input" in record:
        args = [record["input"], "--default-state", record["default_state"]]
    else:
        args = []
        for k, v in record["fields"].items():
            args += [f"--{k}", v]
    out = subprocess.run(
        [str(exe), "parse", *args], capture_output=True, text=True, check=True
    )
    return json.loads(out.stdout)


def structured(comp):
    """The CLI writes highway, rural route and military as strings; the
    corpus wants objects (tests/corpus/README.md)."""
    c = dict(comp)
    if "highway" in c:
        # The longest standard prefix ending before the route; words before
        # it are a state (`KY STATE HIGHWAY 1`), and with no prefix the
        # words before the route are the state (`KY 1207`).
        words = c["highway"].split()
        head, route = words[:-1], words[-1]
        h = {"state": " ".join(head), "prefix": "", "route": route}
        for k in range(len(head)):
            if " ".join(head[k:]) in PREFIXES:
                h = {"prefix": " ".join(head[k:]), "route": route}
                if k:
                    h["state"] = " ".join(head[:k])
                break
        c["highway"] = h
    if "rural_route" in c:
        w = c["rural_route"].split()
        r = {"kind": w[0], "route": w[1]}
        if len(w) >= 4:
            r["box"] = w[-1]
        c["rural_route"] = r
    if "military" in c:
        w = c["military"].split()
        c["military"] = {"kind": w[0], "number": w[1], "box": w[-1]}
    if "general_delivery" in c:
        c["general_delivery"] = True
    return c


def draft(exe, record, src, stratum):
    d = parse(exe, record)
    expect = structured(d["components"])
    if d["unparsed"]:
        expect["unparsed"] = d["unparsed"]
    record["expect"] = expect
    record["warnings"] = d["warnings"]
    record["note"] = f"src={src}; stratum={stratum}; TODO"
    flagged = bool(d["unparsed"]) or any(w not in PLAIN for w in d["warnings"])
    return record, flagged


def make_record(src, row):
    if src == "auction":
        return {"input": row, "default_state": "FL"}
    l1, l2, city, z = row
    fields = {"line1": l1}
    if l2:
        fields["line2"] = l2
    if city:
        fields["city"] = city
    fields["state"] = "FL"
    if z:
        fields["zip"] = z
    return {"fields": fields}


def key(src, row):
    return norm(row if src == "auction" else row[0])


def draw(seed, out):
    rng = random.Random(seed)
    exe = addr_bin()
    skip = corpus_inputs()
    sources = [(AUCTION[0], AUCTION[1], auction_pool())]
    for src, n, files in DBPR:
        sources.append((src, n, dbpr_pool(files)))
    random_rows, flagged_rows = [], []
    used = set()
    for src, n, pool in sources:
        order = list(range(len(pool)))
        rng.shuffle(order)
        got = 0
        for i in order:
            if got == n:
                break
            row = pool[i]
            k = key(src, row)
            if k in skip or k in used:
                continue
            rec, _ = draft(exe, make_record(src, row), src, "random")
            random_rows.append(rec)
            used.add(k)
            got += 1
        print(f"{src}: {len(pool)} distinct addresses, {got} drawn", file=sys.stderr)
    # The flagged top-up: from each source in proportion to its share, from
    # rows not drawn above, in a second seeded order.
    total = sum(n for _, n, _ in sources)
    for src, n, pool in sources:
        want = round(FLAGGED * n / total)
        order = list(range(len(pool)))
        rng.shuffle(order)
        got = 0
        for i in order:
            if got == want:
                break
            row = pool[i]
            k = key(src, row)
            if k in skip or k in used:
                continue
            rec, flagged = draft(exe, make_record(src, row), src, "flagged")
            used.add(k)
            if flagged:
                flagged_rows.append(rec)
                got += 1
        print(f"{src}: {got} flagged", file=sys.stderr)
    rows = random_rows + flagged_rows
    out.mkdir(parents=True, exist_ok=True)
    with (out / "sample.jsonl").open("w", encoding="utf-8") as f:
        for n, rec in enumerate(rows, 1):
            ordered = {"id": f"fs-{n:03d}"}
            ordered.update(rec)
            f.write(json.dumps(ordered) + "\n")
    (out / "SEED").write_text(f"{seed}\n")
    print(f"wrote {len(rows)} records to {out / 'sample.jsonl'} (seed {seed})", file=sys.stderr)


def status(out):
    path = out / "sample.jsonl"
    rows = [json.loads(l) for l in path.read_text().splitlines() if l.strip()]
    todo = [r["id"] for r in rows if r.get("note", "").endswith("TODO")]
    print(f"{len(rows) - len(todo)} of {len(rows)} labeled; {len(todo)} left")
    if todo:
        print("next: " + ", ".join(todo[:10]))


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--seed", type=int, default=20261017)
    ap.add_argument("--out", type=Path, default=ROOT / "local-data" / "florida-sample")
    ap.add_argument("--status", action="store_true", help="count labeled rows and stop")
    ap.add_argument("--force", action="store_true", help="overwrite an existing sample")
    a = ap.parse_args()
    if a.status:
        return status(a.out)
    if (a.out / "sample.jsonl").exists() and not a.force:
        sys.exit(f"{a.out / 'sample.jsonl'} exists; it may hold labels (use --force to replace)")
    draw(a.seed, a.out)


if __name__ == "__main__":
    main()
