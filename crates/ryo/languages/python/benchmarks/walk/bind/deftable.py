#!/usr/bin/env python3
"""deftable.py <dir with <rule>.defs.json> <rule..> [--top n]: the definitions table of STATUS.md: per rule the definitions by
steps with the old engine's rows beside them, a first mechanical reading of the class of each one whose steps exceed ten
times the old engine's rows, and the count of such definitions."""
import json, sys
d = sys.argv[1]; top = int(sys.argv[sys.argv.index("--top") + 1]) if "--top" in sys.argv else 10
skip = {sys.argv[i + 1] for i, a in enumerate(sys.argv) if a in ("--top", "--json")}
rules = [a for a in sys.argv[2:] if not a.startswith("--") and a not in skip]
def cls(x):
    """the class, read off the counters (see STATUS for what each means)"""
    if x["definition"][0].startswith("("):
        return "(sub-programs shared by several definitions: counted with their callers)"
    o_rows, o_runs = x["old_rows"], x["old_runs"]
    if o_rows is None:
        return "no old node of its own (a cut side or a branch): not compared"
    if x["steps"] <= 10 * max(o_rows, 1) and not (o_runs == 0 and x["steps"] > 1000):
        return "within 10x"
    if o_runs == 0 or o_rows == 0:
        if x["distinct_keys"] == 0:
            return "E: dead in the old engine (an input is empty in every file); here its whole table is enumerated once per file"
        return "E: dead in the old engine (an input is empty in every file); here evaluated per key, every answer empty or unused"
    if x["distinct_keys"] == 0 and x["calls"] > 0:
        return "B: computed once per file, but by scanning one side per row of the other"
    if x["table_abandoned"] > x["table_builds"]:
        return "D: its table is tried whole per file and abandoned; then per key"
    if x["distinct_keys"] > 20 * max(x["rows"], 1):
        return "A/C: asked per candidate, nearly every answer empty (the same absence proved per key)"
    if x["distinct_keys"] > 0:
        return "A: evaluated per key by navigation where the old engine holds it once per file"
    return "B: computed once per file, but by scanning one side per row of the other"
out = {}
for rid in rules:
    r = json.load(open(f"{d}/{rid}.defs.json"))[rid]
    defs = r["definitions"]
    for x in defs:
        x["class"] = cls(x)
    over = [x for x in defs if x["class"][0] in "ABCDE" and x["class"][1] in ":/"]
    counts = {}
    for x in over:
        counts[x["class"].split(":")[0]] = counts.get(x["class"].split(":")[0], 0) + 1
    print(f"\n**{rid}**: {r['steps']:,} steps ({r['in_definitions']:,} inside {len(defs)} definitions, {r['made_table_programs']:,} in made-table programs, {r['top_level']:,} at top level). "
          f"Definitions over 10x the old engine's rows: {len(over)} ({', '.join(f'{k}: {v}' for k, v in sorted(counts.items()))}); their steps: {sum(x['steps'] for x in over):,}.\n")
    print("| definition (plan step) | steps | keyed calls | distinct keys | rows produced | table builds / abandoned | old engine: op, runs, rows | class |")
    print("|---|---:|---:|---:|---:|---:|---|---|")
    for x in defs[:top]:
        o = "-" if x["old_rows"] is None else f"{x['old_op']}, {x['old_runs']:,}, {x['old_rows']:,}"
        print(f"| {' '.join(str(t) for t in x['definition'])} | {x['steps']:,} | {x['calls']:,} | {x['distinct_keys']:,} | {x['rows']:,} | {x['table_builds']:,} / {x['table_abandoned']:,} | {o} | {x['class']} |")
    for m in r["made"][:2]:
        if m["steps"] * 20 > r["steps"]:
            print(f"| {m['name']} (made table: its input programs) | {m['steps']:,} | {m['builds']:,} builds | | {m['rows']:,} | | | |")
    out[rid] = dict(steps=r["steps"], over_10x=len(over), over_10x_steps=sum(x["steps"] for x in over), classes=counts)
if "--json" in sys.argv:
    json.dump(out, open(sys.argv[sys.argv.index("--json") + 1], "w"), indent=1)
