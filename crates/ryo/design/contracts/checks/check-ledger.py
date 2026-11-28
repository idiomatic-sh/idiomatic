#!/usr/bin/env python3
"""check-ledger.py [CONTRACTS_DIR]: counts trait declarations at main and the perf tip (PLAN.md section 5, both classes)
and compares with ledger.tsv: same rows, each with a sheet and a fate."""
import os, re, subprocess, sys, collections
ROOT = sys.argv[1] if len(sys.argv) > 1 else os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
REPO = "/workspaces/idiomatic-public-benchmark"
PATHS = ["crates/ryo/crates/*.rs", "crates/ryo/cli/*.rs", "crates/ryo/languages/*.rs"]
CLASSES = {"pub": r"^\s*pub trait ", "restricted": r"^\s*pub\((crate|super|in [a-z_:]+)\) trait "}
want = set()
for rev in ("f4ae3bd802", "2a207464c"):
    for cls, pat in CLASSES.items():
        out = subprocess.run(["git", "-C", REPO, "grep", "-n", "-E", pat, rev, "--", *PATHS], capture_output=True, text=True).stdout
        for l in out.splitlines():
            _, path, line, text = l.split(":", 3)
            name = re.search(r"trait\s+([A-Za-z0-9_]+)", text).group(1)
            want.add((rev, path, line, name))
counts = collections.Counter((r, ) for r, *_ in want)
print("expected: " + ", ".join(f"{r} {c}" for (r,), c in sorted(counts.items())), file=sys.stderr)
p = os.path.join(ROOT, "ledger.tsv")
if not os.path.exists(p): print("ledger.tsv does not exist yet"); sys.exit(1)
have, fails = set(), 0
for i, row in enumerate(open(p), 1):
    if i == 1 or not row.strip(): continue
    c = row.rstrip("\n").split("\t")
    if len(c) < 10: print(f"ledger.tsv:{i}: {len(c)} columns, want 10"); fails += 1; continue
    have.add((c[0][:10], c[2], c[3], c[4]))
    if not c[8].strip() or not c[9].strip(): print(f"ledger.tsv:{i}: {c[4]} has no sheet or no fate"); fails += 1
want10 = {(r[:10], p_, l, n) for r, p_, l, n in want}
for k in sorted(want10 - have): print(f"missing from ledger: {k[0]} {k[1]}:{k[2]} {k[3]}"); fails += 1
for k in sorted(have - want10): print(f"in ledger but not in code: {k[0]} {k[1]}:{k[2]} {k[3]}"); fails += 1
print(f"check-ledger: {len(want10)} declarations, {fails} failures", file=sys.stderr); sys.exit(1 if fails else 0)
