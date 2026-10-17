"""Label the blind Florida sample (docs/design.md item 36) one row at a time.

    python3 tools/calibrate/label.py [--file PATH] [--review] [--show-draft] [--start ID]

Reads `local-data/florida-sample/sample.jsonl` (drafted by `sample_fl.py`),
shows each unlabeled row (note ending in `TODO`), and writes the corrected
record back after every accepted row, so it can be stopped and resumed. The
rules it applies are the corpus conventions in `tests/corpus/README.md`:

- Values are picked from the input's own words or, for suffixes,
  directionals, unit types, states and highway prefixes, from the standard
  forms in `data/*.toml`; typed text is uppercased with periods dropped.
- The warnings that follow from the fields (`missing_city`,
  `missing_state`, `missing_zip`, `unparsed_remainder`,
  `missing_street_number`) are computed; the rest are toggled.
- A row is saved only if the corpus runner accepts it (`addr corpus test`
  on that row alone).
- When the label differs from the parser's answer, a line goes to
  `NOTES.md` beside the sample (`fs-042: <reason>`); a row that matches the
  parser again loses its line.

The draft is hidden until you ask for it, so you decide the answer from the
input first (`--show-draft` turns that off). `--review` walks the rows
already labeled instead. Nothing here prints anything but the sample's own
rows, which never leave `local-data/`.
"""
import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(Path(__file__).resolve().parent))
import sample_fl  # noqa: E402

DATA = ROOT / "data"

# Corpus `expect` keys in the runner's order (crates/addr-core/src/corpus.rs).
FIELDS = [
    "street_number", "street_dir_prefix", "street_name", "street_suffix",
    "street_dir_suffix", "highway", "unit_type", "unit_number",
    "second_unit_type", "second_unit_number", "po_box", "rural_route",
    "general_delivery", "military", "military_post_office", "attention",
    "building", "city", "state", "zip5", "zip4", "unparsed",
]
# Shown on every row; the rest only when set or after `a`.
COMMON = [
    "street_number", "street_dir_prefix", "street_name", "street_suffix",
    "street_dir_suffix", "unit_type", "unit_number", "city", "state", "zip5",
    "zip4", "unparsed",
]
# Warnings computed from the fields (corpus README, "Labeling conventions").
AUTO = ["missing_street_number", "unparsed_remainder", "missing_city", "missing_state", "missing_zip"]
# Warnings set by hand, in the order offered.
MANUAL = [
    "default_state_applied", "zip_city_mismatch", "non_standard_abbreviation",
    "possibly_truncated", "invalid_zip", "unknown_state", "placeholder",
    "zero_street_number", "missing_street_name", "missing_unit_number",
    "dual_address", "multiple_addresses", "ambiguous_street_number",
    "ambiguous_token", "country_ignored", "intersection_detected",
]
AMBIGUOUS = [
    "number glued to an ordinal",
    "city or street name unclear",
    "unit or part of the name unclear",
    "address cut off",
]


def load(name, key):
    return tomllib.loads((DATA / name).read_text()).get(key, [])


def spellings(entries, std, extra=("forms", "observed")):
    """Spelling -> standard form, for picking from the input's words."""
    out = {}
    for e in entries:
        for k in extra:
            for f in e.get(k, []):
                out[f] = e[std]
        out[e[std]] = e[std]
    return out


SUFFIX = spellings(load("suffixes.toml", "suffix"), "abbr")
UNIT = spellings(load("units.toml", "unit"), "abbr")
DIRS = [d["abbr"] for d in load("directionals.toml", "directional")]
DIR = spellings(load("directionals.toml", "directional"), "abbr")
STATES = {s["code"] for s in load("states.toml", "state")}
STATE = spellings(load("states.toml", "state"), "code", ("aliases",))
for s in load("states.toml", "state"):
    STATE[s["name"]] = s["code"]
PREFIXES = sorted(sample_fl.PREFIXES)
# Standard values the runner checks, per field.
STANDARD = {
    "street_suffix": set(SUFFIX.values()),
    "street_dir_prefix": set(DIRS),
    "street_dir_suffix": set(DIRS),
    "unit_type": set(UNIT.values()),
    "second_unit_type": set(UNIT.values()),
    "state": STATES,
}
PICK = {
    "street_suffix": SUFFIX,
    "street_dir_prefix": DIR,
    "street_dir_suffix": DIR,
    "unit_type": UNIT,
    "second_unit_type": UNIT,
    "state": STATE,
}


def clean(s):
    """A typed or picked value as the corpus stores it."""
    return re.sub(r"\s+", " ", s.replace(".", "").upper()).strip()


def words_of(record):
    if "input" in record:
        text = record["input"]
    else:
        f = record["fields"]
        text = " ".join(f.get(k, "") for k in ("line1", "line2", "city", "state", "zip"))
    out = []
    for w in re.split(r"[\s,;]+", text):
        w = clean(w).strip("-")
        if w:
            out.append(w)
    return out


def input_text(record):
    if "input" in record:
        return record["input"]
    f = record["fields"]
    return "  |  ".join(f"{k}: {f[k]}" for k in ("line1", "line2", "city", "state", "zip") if k in f)


def show(v):
    if v is None:
        return "-"
    if isinstance(v, dict):
        return " ".join(f"{k}={v[k]}" for k in ("state", "prefix", "kind", "route", "number", "box") if k in v)
    if v is True:
        return "yes"
    return v


def auto_warnings(e, manual):
    w = set()
    if any(e.get(k) for k in ("street_name", "street_suffix", "highway")) and not e.get(
        "street_number"
    ) and "zero_street_number" not in manual:
        w.add("missing_street_number")
    if e.get("unparsed"):
        w.add("unparsed_remainder")
    if not e.get("city") and not e.get("military_post_office"):
        w.add("missing_city")
    if not e.get("state") and "unknown_state" not in manual:
        w.add("missing_state")
    if not e.get("zip5") and "invalid_zip" not in manual:
        w.add("missing_zip")
    return w


def ordered(expect):
    return {k: expect[k] for k in FIELDS if k in expect}


def warnings_list(manual, expect):
    ws = auto_warnings(expect, manual) | set(manual)
    return [w for w in AUTO + MANUAL if w in ws]


def ask(prompt):
    try:
        return input(prompt)
    except EOFError:
        raise SystemExit("\n(stopped; saved rows are kept)")


class Session:
    def __init__(self, path, review, blind):
        self.path = path
        self.notes = path.parent / "NOTES.md"
        self.rows = [json.loads(l) for l in path.read_text().splitlines() if l.strip()]
        self.review = review
        self.blind = blind
        self.exe = sample_fl.addr_bin()

    # ---- files ---------------------------------------------------------------

    def save(self):
        tmp = self.path.with_suffix(".jsonl.tmp")
        tmp.write_text("".join(json.dumps(r) + "\n" for r in self.rows))
        os.replace(tmp, self.path)

    def read_notes(self):
        if not self.notes.exists():
            return {}
        out = {}
        for line in self.notes.read_text().splitlines():
            m = re.match(r"(fs-\d+): (.*)", line)
            if m:
                out[m.group(1)] = m.group(2)
        return out

    def write_note(self, rid, text):
        notes = self.read_notes()
        if text:
            notes[rid] = text
        else:
            notes.pop(rid, None)
        body = "".join(f"{k}: {v}\n" for k, v in sorted(notes.items()))
        self.notes.write_text("# Parser errors in the Florida sample\n\n" + body)

    def valid(self, record):
        """Whether the corpus runner accepts this record (format, warning
        codes, standard values). Returns the runner's complaint or None."""
        d = Path(tempfile.mkdtemp(prefix="fs-check-"))
        try:
            (d / "one.jsonl").write_text(json.dumps(record) + "\n")
            r = subprocess.run(
                [str(self.exe), "corpus", "test", "--dir", str(d)],
                capture_output=True, text=True,
            )
            if r.returncode == 2:
                return (r.stdout + r.stderr).strip()
            return None
        finally:
            shutil.rmtree(d, ignore_errors=True)

    # ---- one row -------------------------------------------------------------

    def label(self, i):
        """Label row i. Returns the next index, "back", or None to quit."""
        rec = self.rows[i]
        rid = rec["id"]
        words = words_of(rec)
        parsed, parsed_w = sample_fl.parser_answer(self.exe, rec)
        expect = dict(rec["expect"])
        manual = {w for w in rec["warnings"] if w not in AUTO}
        note = rec.get("note", "")
        meta = [p for p in note.removesuffix("TODO").strip().rstrip(";").split("; ") if p]
        all_fields = False
        todo = sum(r.get("note", "").endswith("TODO") for r in self.rows)

        print("\n" + "=" * 72)
        print(f"{rid}  [{i + 1} of {len(self.rows)}, {todo} to label]  {'; '.join(meta)}")
        print(f"INPUT:  {input_text(rec)}")
        if "default_state" in rec:
            print(f"        (parsed with default_state {rec['default_state']})")
        print("WORDS:  " + "  ".join(f"{n}:{w}" for n, w in enumerate(words, 1)))
        if self.blind and note.endswith("TODO"):
            ask("Decide the parse, then press Enter to see the draft... ")

        while True:
            shown = [f for f in FIELDS if all_fields or f in COMMON or f in expect]
            print()
            for n, f in enumerate(shown, 1):
                mark = "" if expect.get(f) == parsed.get(f) else "   (parser: %s)" % show(parsed.get(f))
                print(f" {n:2} {f:20} {show(expect.get(f))}{mark}")
            ws = warnings_list(manual, expect)
            print(" warnings: " + (", ".join(ws) or "(none)"))
            if set(ws) != set(parsed_w):
                print("   (parser: %s)" % (", ".join(parsed_w) or "none"))
            amb = [m for m in meta if m.startswith("ambiguous:")]
            if amb:
                print(" " + amb[0])
            cmd = ask(
                "\n[Enter] correct  [1-%d] fix field  [w] warnings  [?] ambiguous  "
                "[a] all fields  [r] reset  [s] skip  [b] back  [q] quit > " % len(shown)
            ).strip()
            if cmd == "":
                if self.accept(i, expect, manual, meta, parsed, parsed_w):
                    return i + 1
            elif cmd.isdigit() and 1 <= int(cmd) <= len(shown):
                self.edit(shown[int(cmd) - 1], expect, words)
            elif cmd == "w":
                self.toggle(manual)
            elif cmd == "?":
                meta = self.ambiguous(meta)
            elif cmd == "a":
                all_fields = not all_fields
            elif cmd == "r":
                expect = dict(parsed)
                manual = {w for w in parsed_w if w not in AUTO}
            elif cmd == "s":
                return i + 1
            elif cmd == "b":
                return "back"
            elif cmd == "q":
                return None
            else:
                print("?")

    def accept(self, i, expect, manual, meta, parsed, parsed_w):
        rec = dict(self.rows[i])
        rec["expect"] = ordered({k: v for k, v in expect.items() if v not in (None, "", False)})
        rec["warnings"] = warnings_list(manual, rec["expect"])
        rec["note"] = "; ".join(meta)
        problem = self.valid(rec)
        if problem:
            print("\nNot saved; the corpus runner rejects it:\n" + problem)
            return False
        same = rec["expect"] == ordered(parsed) and set(rec["warnings"]) == set(parsed_w)
        if same:
            self.write_note(rec["id"], None)
        else:
            diff = self.diff(rec["expect"], rec["warnings"], parsed, parsed_w)
            old = self.read_notes().get(rec["id"])
            print(f"\nDiffers from the parser: {diff}")
            if old:
                print(f"NOTES.md now says: {old}")
            text = ask("Reason for NOTES.md ([Enter] keep %s) > " % ("it" if old else "the diff")).strip()
            self.write_note(rec["id"], text or old or diff)
        self.rows[i] = rec
        self.save()
        return True

    @staticmethod
    def diff(e, w, parsed, parsed_w):
        parts = []
        for f in FIELDS:
            if e.get(f) != parsed.get(f):
                parts.append(f"{f}: {show(parsed.get(f))} -> {show(e.get(f))}")
        if set(w) != set(parsed_w):
            gone = sorted(set(parsed_w) - set(w))
            new = sorted(set(w) - set(parsed_w))
            parts.append("warnings" + "".join(f" -{x}" for x in gone) + "".join(f" +{x}" for x in new))
        return "; ".join(parts)

    def ambiguous(self, meta):
        meta = [m for m in meta if not m.startswith("ambiguous:")]
        for n, a in enumerate(AMBIGUOUS, 1):
            print(f" {n} {a}")
        c = ask("Reason (number, text, or Enter to clear) > ").strip()
        if not c:
            return meta
        reason = AMBIGUOUS[int(c) - 1] if c.isdigit() and 1 <= int(c) <= len(AMBIGUOUS) else c
        return meta + [f"ambiguous: {reason}"]

    def toggle(self, manual):
        while True:
            for n, w in enumerate(MANUAL, 1):
                print(f" {n:2} [{'x' if w in manual else ' '}] {w}")
            print("    (missing_*, unparsed_remainder and missing_street_number follow the fields)")
            c = ask("Toggle (numbers, Enter when done) > ").strip()
            if not c:
                return
            for t in c.replace(",", " ").split():
                if t.isdigit() and 1 <= int(t) <= len(MANUAL):
                    manual ^= {MANUAL[int(t) - 1]}

    # ---- one field -----------------------------------------------------------

    def edit(self, field, expect, words):
        cur = expect.get(field)
        print(f"\n{field}: {show(cur)}")
        if field in ("highway", "rural_route", "military"):
            v = self.structured(field, cur)
        elif field == "general_delivery":
            v = None if cur else True
        elif field in PICK:
            v = self.pick(field, words)
        else:
            v = self.span(field, words)
        if v == "KEEP":
            return
        if v in (None, ""):
            expect.pop(field, None)
        else:
            expect[field] = v

    def pick(self, field, words):
        table = PICK[field]
        opts = []
        for w in words + [" ".join(p) for p in zip(words, words[1:])]:
            if w in table and table[w] not in opts:
                opts.append(table[w])
        if field.endswith("dir_prefix") or field.endswith("dir_suffix"):
            opts += [d for d in DIRS if d not in opts]
        for n, o in enumerate(opts, 1):
            print(f" {n} {o}")
        while True:
            c = ask("Pick a number, type a value, '-' to clear, Enter to keep > ").strip()
            if not c:
                return "KEEP"
            if c == "-":
                return None
            if c.isdigit() and 1 <= int(c) <= len(opts):
                return opts[int(c) - 1]
            v = table.get(clean(c), clean(c))
            if v in STANDARD[field]:
                return v
            print(f"'{v}' is not a standard {field} value")

    def span(self, field, words):
        print("  " + "  ".join(f"{n}:{w}" for n, w in enumerate(words, 1)))
        c = ask("Words (e.g. 3 or 3-4), '=TEXT' to type, '-' to clear, Enter to keep > ").strip()
        if not c:
            return "KEEP"
        if c == "-":
            return None
        if c.startswith("="):
            return clean(c[1:])
        m = re.fullmatch(r"(\d+)(?:-(\d+))?", c)
        if m:
            a, b = int(m.group(1)), int(m.group(2) or m.group(1))
            if 1 <= a <= b <= len(words):
                return " ".join(words[a - 1 : b])
        print("not understood; kept")
        return "KEEP"

    def structured(self, field, cur):
        c = ask("[e] edit  [-] clear  [Enter] keep > ").strip()
        if not c:
            return "KEEP"
        if c == "-":
            return None
        cur = dict(cur or {})
        if field == "highway":
            for n, p in enumerate(PREFIXES, 1):
                print(f" {n:2} {p}")
            p = ask(f"prefix ({cur.get('prefix', '-')}) > ").strip()
            if p.isdigit() and 1 <= int(p) <= len(PREFIXES):
                cur["prefix"] = PREFIXES[int(p) - 1]
            cur["route"] = clean(ask(f"route ({cur.get('route', '-')}) > ") or cur.get("route", ""))
            st = clean(ask(f"state in the name, '-' for none ({cur.get('state', '-')}) > "))
            if st == "-":
                cur.pop("state", None)
            elif st:
                cur["state"] = STATE.get(st, st)
            return {k: cur[k] for k in ("state", "prefix", "route") if k in cur}
        kinds = ["RR", "HC"] if field == "rural_route" else ["PSC", "UNIT", "CMR", "OPC", "UPR"]
        k = clean(ask(f"kind {'/'.join(kinds)} ({cur.get('kind', '-')}) > ")) or cur.get("kind", "")
        cur["kind"] = k
        key = "route" if field == "rural_route" else "number"
        cur[key] = clean(ask(f"{key} ({cur.get(key, '-')}) > ")) or cur.get(key, "")
        b = clean(ask(f"box, '-' for none ({cur.get('box', '-')}) > "))
        if b == "-":
            cur.pop("box", None)
        elif b:
            cur["box"] = b
        return {x: cur[x] for x in ("kind", key, "box") if x in cur}

    # ---- the loop ------------------------------------------------------------

    def wanted(self, r):
        todo = r.get("note", "").endswith("TODO")
        return not todo if self.review else todo

    def run(self, start):
        ids = [r["id"] for r in self.rows]
        i = ids.index(start) if start in ids else 0
        force = start in ids
        shown = []
        while 0 <= i < len(self.rows):
            if not force and not self.wanted(self.rows[i]):
                i += 1
                continue
            force = False
            nxt = self.label(i)
            if nxt is None:
                break
            if nxt == "back":
                # The row shown before this one, labeled or not.
                if shown:
                    i = shown.pop()
                force = True
                continue
            shown.append(i)
            i = nxt
        left = sum(r.get("note", "").endswith("TODO") for r in self.rows)
        print(f"\n{len(self.rows) - left} of {len(self.rows)} labeled; {left} left.")


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--file", type=Path, default=ROOT / "local-data" / "florida-sample" / "sample.jsonl")
    ap.add_argument("--review", action="store_true", help="walk the rows already labeled")
    ap.add_argument("--show-draft", action="store_true", help="show the draft at once")
    ap.add_argument("--start", help="row id to start at (e.g. fs-042)")
    a = ap.parse_args()
    if not a.file.exists():
        sys.exit(f"{a.file} not found; draw it with tools/calibrate/sample_fl.py")
    Session(a.file, a.review, not a.show_draft).run(a.start)


if __name__ == "__main__":
    main()
