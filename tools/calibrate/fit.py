#!/usr/bin/env python3
"""Fit the confidence score (docs/design.md item 38, decision 12).

The label is "the whole parse is right". The features are what the parser
knew while parsing: the uncertain decisions its tagger made (item 37) and the
warnings it raised. Both come from the binary (`doorplate corpus features`,
`doorplate parse --features`), so this script never re-implements a rule.

Fit data (licensed or our own; the geocode benchmark is never read here, it
only checks the result, `check_benchmark.py`):

  corpus    tests/corpus (`doorplate corpus features`): pass is the corpus
            runner's whole-case result.
  usaddress tests/external/usaddress/*.xml (MIT): pass is the coarse
            whole-address score of docs/usaddress-comparison.md; addresses
            with out-of-scope labels are skipped, and so are addresses the
            corpus already holds.
  florida   local-data/florida-sample (private, hand-labeled): scored with a
            parser built *before* item 36's fixes (`--sample-addr`), so the
            fixes, which were made from this sample's errors, do not make it
            look better than it was.

    cargo build --release -p doorplate-cli
    python3 tools/calibrate/fit.py --sample-addr PATH/TO/PRE-FIX/addr \
        [--out crates/doorplate/data/confidence.toml] [--report local-data/reports/fit.md]

Model: an L2 logistic regression on the decision and warning indicators,
with every coefficient constrained to be at most zero (a warning or an
uncertain decision never raises confidence; a positive coefficient only
stands for the source it came from). It is compared with a constant by
cross-validated log loss. The Florida sample, the data nearest to what the
library is for, carries half the total weight (`--florida-share`). Only
numbers are written; no input text, no licensee data.
"""
import argparse
import collections
import glob
import json
import math
import os
import random
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
sys.path.insert(0, os.path.join(ROOT, "tools", "compare"))
import compare_usaddress as cu  # noqa: E402 (reuses the gold scoring)

DECISIONS = [
    "city_by_comma", "city_by_suffix", "city_by_place", "state_without_zip",
    "bare_unit", "number_letter", "default_state",
]


# --------------------------------------------------------------------- data

def run_lines(binary, args, text=None):
    out = subprocess.run(
        [binary, *args], input=text.encode() if text is not None else None,
        capture_output=True, check=False,
    )
    if out.returncode not in (0, 2):
        sys.exit(f"{binary} {' '.join(args)}: exit {out.returncode}\n{out.stderr.decode()}")
    return [json.loads(l) for l in out.stdout.decode().splitlines()]


def norm(s):
    return " ".join(s.lower().split())


def jsonl_inputs(directory):
    """{id: normalized input} for the corpus-format files of a directory."""
    got = {}
    for path in glob.glob(os.path.join(directory, "*.jsonl")):
        with open(path, encoding="utf-8") as fh:
            for line in fh:
                if not line.strip():
                    continue
                r = json.loads(line)
                if "input" in r:
                    s = r["input"]
                elif "lines" in r:
                    s = ", ".join(r["lines"])
                else:
                    s = " ".join(v for v in r.get("fields", {}).values() if v)
                got[r["id"]] = norm(s)
    return got


def corpus_examples(binary, directory, source, extra_fields=None):
    rows = run_lines(binary, ["corpus", "features", "--dir", directory])
    inputs = jsonl_inputs(directory)
    notes = {}
    for path in glob.glob(os.path.join(directory, "*.jsonl")):
        with open(path, encoding="utf-8") as fh:
            for line in fh:
                if line.strip():
                    r = json.loads(line)
                    notes[r["id"]] = r.get("note", "")
    out = []
    for r in rows:
        out.append({
            "source": source, "id": r["id"], "group": r["file"],
            "text": inputs.get(r["id"], ""), "y": int(r["pass"]),
            "d": r["decisions"], "w": r["warnings"],
            "stratum": ("flagged" if "stratum=flagged" in notes.get(r["id"], "") else "random")
            if source == "florida" else None,
        })
    return out


def gold_examples(binary, directory, skip_inputs):
    out = []
    for path in sorted(glob.glob(os.path.join(directory, "*.xml"))):
        gold = cu.load_gold(path)
        strings = [s.replace("\n", " ") for s, _ in gold]
        tags = cu.ours(strings, binary)
        feats = run_lines(binary, ["parse", "--features"], "\n".join(strings) + "\n")
        if len(feats) != len(strings):
            sys.exit("--features returned a different number of records")
        for n, ((s, toks), mine, f) in enumerate(zip(gold, tags, feats)):
            if any(l in cu.OUT_OF_SCOPE for _, l in toks):
                continue
            if norm(s) in skip_inputs:
                continue
            spans = cu.byte_spans(s, [t for t, _ in toks])
            count, ok, _ = cu.score(toks, spans, mine, cu.coarse)
            out.append({
                "source": "usaddress", "id": f"{os.path.basename(path)}#{n}",
                "group": os.path.basename(path), "text": norm(s),
                "y": int(ok == count), "d": f["decisions"], "w": f["warnings"],
                "stratum": None,
            })
    return out


# ------------------------------------------------------------------- models

def sigmoid(z):
    if z >= 0:
        return 1 / (1 + math.exp(-z))
    e = math.exp(z)
    return e / (1 + e)


def solve(a, b):
    """Solve a x = b by Gaussian elimination with partial pivoting."""
    n = len(b)
    m = [row[:] + [b[i]] for i, row in enumerate(a)]
    for c in range(n):
        p = max(range(c, n), key=lambda r: abs(m[r][c]))
        m[c], m[p] = m[p], m[c]
        piv = m[c][c]
        if abs(piv) < 1e-12:
            continue
        for r in range(c + 1, n):
            f = m[r][c] / piv
            if f:
                for k in range(c, n + 1):
                    m[r][k] -= f * m[c][k]
    x = [0.0] * n
    for r in range(n - 1, -1, -1):
        s = m[r][n] - sum(m[r][k] * x[k] for k in range(r + 1, n))
        x[r] = s / m[r][r] if abs(m[r][r]) > 1e-12 else 0.0
    return x


class Features:
    """Indicator features: every decision, and each warning seen at least
    `min_support` times in the fit data (the rest share `w:other`)."""

    def __init__(self, rows, min_support):
        count = collections.Counter(w for r in rows for w in set(r["w"]))
        self.warnings = sorted(w for w, n in count.items() if n >= min_support)
        self.names = ["d:" + d for d in DECISIONS] + ["w:" + w for w in self.warnings] + ["w:other"]
        self.index = {n: i for i, n in enumerate(self.names)}
        self.dropped = set()

    def vector(self, r):
        v = [0.0] * len(self.names)
        for d in r["d"]:
            v[self.index["d:" + d]] = 1.0
        for w in r["w"]:
            v[self.index.get("w:" + w, self.index["w:other"])] = 1.0
        for i in self.dropped:
            v[i] = 0.0
        return v


def fit_logistic(xs, ys, ws, l2, iters=60, pinned=()):
    """Newton's method for the L2-penalized weighted logistic regression
    (intercept unpenalized); `pinned` features keep coefficient 0. Returns
    [intercept, *coefficients]."""
    if pinned:
        xs = [[0.0 if i in pinned else v for i, v in enumerate(x)] for x in xs]
    k = len(xs[0]) + 1
    beta = [0.0] * k
    for _ in range(iters):
        grad = [0.0] * k
        hess = [[0.0] * k for _ in range(k)]
        for x, y, w in zip(xs, ys, ws):
            z = beta[0] + sum(b * v for b, v in zip(beta[1:], x) if v)
            p = sigmoid(z)
            g, h = w * (p - y), w * p * (1 - p)
            nz = [0] + [i + 1 for i, v in enumerate(x) if v]
            for i in nz:
                vi = 1.0 if i == 0 else x[i - 1]
                grad[i] += g * vi
                for j in nz:
                    vj = 1.0 if j == 0 else x[j - 1]
                    hess[i][j] += h * vi * vj
        for i in range(1, k):
            grad[i] += l2 * beta[i]
            hess[i][i] += l2
        step = solve(hess, grad)
        beta = [b - s for b, s in zip(beta, step)]
        if max(abs(s) for s in step) < 1e-8:
            break
    return beta


def predict_logistic(beta, x):
    return sigmoid(beta[0] + sum(b * v for b, v in zip(beta[1:], x) if v))


def clip(p, lo=1e-4):
    return min(max(p, lo), 1 - lo)


class Model:
    """kind: constant | logistic."""

    def __init__(self, kind, rows, weights, l2=4.0, min_support=8, monotone=True):
        self.kind = kind
        total = sum(weights)
        self.base = sum(w * r["y"] for r, w in zip(rows, weights)) / total
        if kind == "constant":
            return
        self.feat = Features(rows, min_support)
        # Rows with the same features and outcome fit as one weighted row.
        merged = collections.defaultdict(float)
        for r, w in zip(rows, weights):
            merged[(tuple(self.feat.vector(r)), r["y"])] += w
        xs = [list(k[0]) for k in merged]
        ys = [k[1] for k in merged]
        ws = list(merged.values())
        self.feat.dropped = set()
        self.beta = fit_logistic(xs, ys, ws, l2)
        if not monotone:
            return
        # A warning or an uncertain decision never raises confidence (a
        # positive coefficient only stands for the source it came from).
        # Active set: pin the most positive coefficient at zero and refit;
        # when the fit settles, free a pinned feature again if the loss
        # would fall by making it negative. Stop when nothing changes.
        for _ in range(4 * len(xs[0])):
            worst = max(range(len(xs[0])), key=lambda i: self.beta[i + 1])
            if self.beta[worst + 1] > 1e-9:
                self.feat.dropped.add(worst)
            else:
                grad = self.gradient(xs, ys, ws)
                back = [i for i in self.feat.dropped if grad[i] > 1e-6]
                if not back:
                    break
                self.feat.dropped.discard(max(back, key=lambda i: grad[i]))
            self.beta = fit_logistic(xs, ys, ws, l2, pinned=self.feat.dropped)

    def gradient(self, xs, ys, ws):
        """d loss / d coefficient at the current fit, per feature."""
        g = [0.0] * len(xs[0])
        for x, y, w in zip(xs, ys, ws):
            p = predict_logistic(self.beta, x)
            for i, v in enumerate(x):
                if v:
                    g[i] += w * (p - y) * v
        return g

    def prob(self, r):
        if self.kind == "constant":
            return clip(self.base)
        return clip(predict_logistic(self.beta, self.feat.vector(r)))


# ------------------------------------------------------------------ scoring

def log_loss(ps, ys):
    return -sum(y * math.log(p) + (1 - y) * math.log(1 - p) for p, y in zip(ps, ys)) / len(ys)


def brier(ps, ys):
    return sum((p - y) ** 2 for p, y in zip(ps, ys)) / len(ys)


def bins(ps, ys, edges=(0.0, 0.5, 0.7, 0.8, 0.9, 0.95, 0.98, 0.995, 1.0001)):
    out = []
    for lo, hi in zip(edges, edges[1:]):
        sel = [(p, y) for p, y in zip(ps, ys) if lo <= p < hi]
        if sel:
            out.append((lo, hi, len(sel), sum(p for p, _ in sel) / len(sel),
                        sum(y for _, y in sel) / len(sel)))
    return out


def ece(ps, ys):
    return sum(n * abs(mp - acc) for _, _, n, mp, acc in bins(ps, ys)) / len(ys)


def weights_for(rows, share):
    """Florida examples carry `share` of the total weight, the others split
    the rest evenly (the target data is small; the synthetic gold files are
    many)."""
    n_fl = sum(r["source"] == "florida" for r in rows)
    n_other = len(rows) - n_fl
    if not n_fl or not n_other:
        return [1.0] * len(rows)
    w_fl, w_other = share / n_fl, (1 - share) / n_other
    scale = len(rows)
    return [scale * (w_fl if r["source"] == "florida" else w_other) for r in rows]


def folds(rows, k, seed):
    """Stratified by (source, y) so every fold holds a few failures."""
    rng = random.Random(seed)
    buckets = collections.defaultdict(list)
    for i, r in enumerate(rows):
        buckets[(r["source"], r["y"])].append(i)
    assign = [0] * len(rows)
    for idx in buckets.values():
        rng.shuffle(idx)
        for n, i in enumerate(idx):
            assign[i] = n % k
    return assign


def cross_validate(rows, kind, args, seed):
    ps = [0.0] * len(rows)
    assign = folds(rows, args.folds, seed)
    for f in range(args.folds):
        train = [r for r, a in zip(rows, assign) if a != f]
        model = Model(kind, train, weights_for(train, args.florida_share), args.l2,
                      args.min_support, not args.free)
        for i, (r, a) in enumerate(zip(rows, assign)):
            if a == f:
                ps[i] = model.prob(r)
    return ps


def wilson(k, n, z=1.645):
    """90% Wilson interval for k right of n."""
    if not n:
        return (0.0, 1.0)
    p = k / n
    d = 1 + z * z / n
    c = (p + z * z / (2 * n)) / d
    h = z * math.sqrt(p * (1 - p) / n + z * z / (4 * n * n)) / d
    return (c - h, c + h)


def pct(n, d):
    return f"{100 * n / d:.1f}%" if d else "-"


# --------------------------------------------------------------------- main

def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--addr", default=os.path.join(ROOT, "target/release/doorplate"))
    ap.add_argument("--sample-addr", help="addr built before item 36's fixes, "
                    "for the Florida sample (default: --addr, optimistic)")
    ap.add_argument("--corpus", default=os.path.join(ROOT, "tests/corpus"))
    ap.add_argument("--gold", default=os.path.join(ROOT, "tests/external/usaddress"))
    ap.add_argument("--sample", default=os.path.join(ROOT, "local-data/florida-sample"))
    ap.add_argument("--folds", type=int, default=5)
    ap.add_argument("--repeats", type=int, default=5, help="cross-validation repeats")
    ap.add_argument("--l2", type=float, default=4.0)
    ap.add_argument("--min-support", type=int, default=8,
                    help="warnings seen fewer times share one 'other' feature")
    ap.add_argument("--florida-share", type=float, default=0.5,
                    help="share of the total weight carried by the Florida sample")
    ap.add_argument("--free", action="store_true",
                    help="allow warnings to raise confidence (default: they cannot)")
    ap.add_argument("--no-florida", action="store_true")
    ap.add_argument("--dump", help="write the fit rows (numbers only) as JSONL")
    ap.add_argument("--out", help="write the fitted model as TOML")
    ap.add_argument("--report", help="write the Markdown report here")
    args = ap.parse_args()

    corpus = corpus_examples(args.addr, args.corpus, "corpus")
    seen = {r["text"] for r in corpus if r["text"]}
    gold = gold_examples(args.addr, args.gold, seen)
    rows = corpus + gold
    if not args.no_florida and os.path.isdir(args.sample):
        fl = corpus_examples(args.sample_addr or args.addr, args.sample, "florida")
        rows += fl
    print(f"{len(rows)} examples:", dict(collections.Counter(r["source"] for r in rows)),
          file=sys.stderr)

    if args.dump:
        with open(args.dump, "w", encoding="utf-8") as fh:
            for r in rows:
                fh.write(json.dumps({k: r[k] for k in ("source", "group", "stratum", "y", "d", "w")}) + "\n")

    ys = [r["y"] for r in rows]
    results = {}
    for kind in ("constant", "logistic"):
        ll, br, ec = [], [], []
        pooled = [[] for _ in rows]
        for rep in range(args.repeats):
            ps = cross_validate(rows, kind, args, seed=rep)
            ll.append(log_loss(ps, ys))
            br.append(brier(ps, ys))
            ec.append(ece(ps, ys))
            for i, p in enumerate(ps):
                pooled[i].append(p)
        mean = [sum(p) / len(p) for p in pooled]
        results[kind] = {
            "log_loss": sum(ll) / len(ll), "brier": sum(br) / len(br),
            "ece": sum(ec) / len(ec), "cv_probs": mean,
        }
    best = min(results, key=lambda k: results[k]["log_loss"])

    model = Model(best, rows, weights_for(rows, args.florida_share), args.l2, args.min_support,
                  not args.free)
    report = render(args, rows, ys, results, best, model)
    print(report)
    if args.report:
        with open(args.report, "w", encoding="utf-8") as fh:
            fh.write(report)
    if args.out:
        with open(args.out, "w", encoding="utf-8") as fh:
            fh.write(toml(args, rows, best, model))
        print(f"wrote {args.out}", file=sys.stderr)


def render(args, rows, ys, results, best, model):
    L = ["# Confidence fit", ""]
    by = collections.defaultdict(lambda: [0, 0])
    for r in rows:
        by[r["source"]][0] += 1
        by[r["source"]][1] += r["y"]
    L += ["| Source | Examples | Whole parse right |", "|---|---|---|"]
    for s, (n, ok) in sorted(by.items()):
        lo, hi = wilson(ok, n)
        L.append(f"| {s} | {n} | {ok} ({pct(ok, n)}; 90% interval {100 * lo:.1f}% to {100 * hi:.1f}%) |")
    L.append(f"| **All** | {len(rows)} | {sum(ys)} ({pct(sum(ys), len(ys))}) |")
    fl = [r for r in rows if r["source"] == "florida"]
    for st in ("random", "flagged"):
        sel = [r for r in fl if r["stratum"] == st]
        if sel:
            k = sum(r["y"] for r in sel)
            lo, hi = wilson(k, len(sel))
            L.append(f"\nFlorida {st} stratum: {k}/{len(sel)} right "
                     f"(90% interval {100 * lo:.1f}% to {100 * hi:.1f}%).")
    L += ["", f"## Models ({args.repeats}x{args.folds}-fold cross-validation)", "",
          "| Model | Log loss | Brier | ECE |", "|---|---|---|---|"]
    for k, v in results.items():
        mark = " (chosen)" if k == best else ""
        L.append(f"| {k}{mark} | {v['log_loss']:.4f} | {v['brier']:.4f} | {v['ece']:.4f} |")
    L += ["", f"## Reliability of the chosen model ({best}), cross-validated", "",
          "| Predicted | n | Mean predicted | Right | 90% interval |", "|---|---|---|---|---|"]
    ps = results[best]["cv_probs"]
    for lo, hi, n, mp, acc in bins(ps, ys):
        a, b = wilson(round(acc * n), n)
        L.append(f"| {lo:.3f} to {min(hi, 1):.3f} | {n} | {mp:.3f} | {acc:.3f} | {a:.3f} to {b:.3f} |")
    L += ["", "By source (cross-validated; predicted mean against right):", "",
          "| Source | n | Mean predicted | Right | Log loss |", "|---|---|---|---|---|"]
    for s in sorted(by):
        idx = [i for i, r in enumerate(rows) if r["source"] == s]
        sp = [ps[i] for i in idx]
        sy = [ys[i] for i in idx]
        L.append(f"| {s} | {len(idx)} | {sum(sp) / len(sp):.3f} | {sum(sy) / len(sy):.3f} "
                 f"| {log_loss(sp, sy):.4f} |")
    fi = [i for i, r in enumerate(rows) if r["source"] == "florida"]
    if fi:
        fp, fy = [ps[i] for i in fi], [ys[i] for i in fi]
        L += ["", "Florida sample alone (cross-validated; the data nearest the target, "
              "190 rows and few failures, so wide intervals):", "",
              "| Predicted | n | Mean predicted | Right | 90% interval |", "|---|---|---|---|---|"]
        for lo, hi, n, mp, acc in bins(fp, fy, edges=(0.0, 0.8, 0.95, 1.0001)):
            a, b = wilson(round(acc * n), n)
            L.append(f"| {lo:.2f} to {min(hi, 1):.2f} | {n} | {mp:.3f} | {acc:.3f} | {a:.3f} to {b:.3f} |")
    if hasattr(model, "beta"):
        L += ["", "## Coefficients (log odds that the parse is right)", "",
              f"Intercept {model.beta[0]:+.3f}", "", "| Feature | n | Right | Coefficient |",
              "|---|---|---|---|"]
        for name, b in sorted(zip(model.feat.names, model.beta[1:]), key=lambda t: t[1]):
            if model.feat.index[name] in model.feat.dropped:
                continue
            idx = [i for i, r in enumerate(rows) if model.feat.vector(r)[model.feat.index[name]]]
            ok = sum(rows[i]["y"] for i in idx)
            L.append(f"| {name} | {len(idx)} | {ok} | {b:+.3f} |")
        gone = sorted(model.feat.names[i] for i in model.feat.dropped)
        if gone:
            L += ["", "Dropped (they would have raised confidence; they join the baseline): "
                  + ", ".join(gone) + "."]
    return "\n".join(L) + "\n"


def toml(args, rows, best, model):
    srcs = collections.Counter(r["source"] for r in rows)
    L = [
        "# Fitted by tools/calibrate/fit.py. Numbers only.",
        f"# Fit data: {dict(srcs)}",
        f"model = {best!r}".replace("'", '"'),
        f"intercept = {model.beta[0]:.6f}" if hasattr(model, "beta") else f"base = {model.base:.6f}",
    ]
    if hasattr(model, "beta"):
        L.append("")
        L.append("[coefficients]")
        for i, (n, b) in enumerate(zip(model.feat.names, model.beta[1:])):
            if i not in model.feat.dropped:
                L.append(f'"{n}" = {b:.6f}')
    return "\n".join(L) + "\n"


if __name__ == "__main__":
    main()
