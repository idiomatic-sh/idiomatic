#!/usr/bin/env python3
"""defprof.py <program.json (lowered with NAV_META=1)> <stderr of a NAV_PROF run> [top n] [--json out]: per rule, the definitions
(sub-plans, named by plan step) by navigation steps: own steps of their programs (every binding pattern), keyed calls,
distinct keys evaluated (memo misses), rows those evaluations produced, table builds / abandoned / rows / steps, and beside
each the old engine's rows for the same plan node over the same corpus (../../engine/s17.stats.jsonl through the export's
step_nodes). ratio = (own steps + table build steps) / max(old rows, 1)."""
import collections, json, os, sys
H = os.path.dirname(os.path.abspath(__file__))
args = [a for a in sys.argv[1:] if not a.startswith("--")]
prog, err = args[0], args[1]
top = int(args[2]) if len(args) > 2 else 25
out_json = sys.argv[sys.argv.index("--json") + 1] if "--json" in sys.argv else None
plans = {r["id"]: r for r in json.load(open(H + "/../export/plans/python.json"))["rules"]}
old = {}
for line in open(H + "/../../engine/s17.stats.jsonl"):
    x = json.loads(line)
    old[x["node"]] = x
rules = json.load(open(prog))["rules"]
base, owner = [], []
for r in rules:
    for i, m in enumerate(r["nav"].get("shared_meta") or [[None, [], None, "?"]] * len(r["nav"]["shared"])):
        owner.append((r["id"], m))
ent = collections.defaultdict(lambda: [0] * 11)  # calls, incl, self, kcalls, misses, rows, tbuilds, tfails, trows, tsteps, probe rows
made = collections.defaultdict(lambda: [0, 0, 0])
total = collections.Counter()
for line in open(err):
    if line.startswith('{"navprof"'):
        for i, c, incl, own in json.loads(line)["navprof"]:
            e = ent[i]; e[0] += c; e[1] += incl; e[2] += own
    elif line.startswith('{"navkprof"'):
        d = json.loads(line)
        for row in d["navkprof"]:
            e = ent[row[0]]
            for k in range(8):
                e[3 + k] += row[1 + k]
        for name, b, s, n in d["navmade"]:
            m = made[name]; m[0] += b; m[1] += s; m[2] += n
    elif line.startswith('{"nav"'):
        for rid, (a, b, c) in json.loads(line)["nav"]["per_rule"].items():
            total[rid] += c
def node_of(rid, key):
    p = plans[rid]; sn = p["step_nodes"]; steps = p["plan"]["steps"]
    if key is None or len(key) < 2:
        return None
    if key[0] in ("step", "union"):
        return sn[key[1]]
    if key[0] == "join":
        v = steps[key[1]]["params"].get(key[2])
        if isinstance(v, dict) and "step" in v:
            return sn[v["step"]]
        return None
    return None
per = collections.defaultdict(lambda: collections.defaultdict(lambda: dict(steps=0, calls=0, kcalls=0, keys=0, rows=0, tbuilds=0, tfails=0, trows=0, tsteps=0, probe_rows=0, patterns=set())))
for i, e in ent.items():
    if i >= len(owner):
        continue
    rid, m = owner[i]
    key = json.dumps(m[0] if m[0] is not None else ["(repeated sub-program)"])
    d = per[rid][key]
    d["steps"] += e[2]
    d["patterns"].add(json.dumps([m[1], m[2]]))
    if m[3] != "branch" or e[3]:
        pass
    d["kcalls"] += e[3]; d["keys"] += e[4]; d["rows"] += e[5]; d["tbuilds"] += e[6]; d["tfails"] += e[7]; d["trows"] += e[8]; d["tsteps"] += e[9]; d["probe_rows"] += e[10]
result = {}
for r in rules:
    rid = r["id"]
    if rid not in per and not total[rid]:
        continue
    rows = []
    for key, d in per[rid].items():
        k = json.loads(key)
        node = node_of(rid, k)
        o = old.get(node)
        rows.append(dict(definition=k, node=node, steps=d["steps"], calls=d["kcalls"], distinct_keys=d["keys"], rows=d["rows"] + d["trows"] + d["probe_rows"],
                         table_builds=d["tbuilds"], table_abandoned=d["tfails"], table_rows=d["trows"], build_steps=d["tsteps"], patterns=len(d["patterns"]),
                         old_rows=None if o is None else o["rows"], old_runs=None if o is None else o["runs"], old_op=None if o is None else o["desc"].split(" {")[0]))
    rows.sort(key=lambda x: -x["steps"])
    in_defs = sum(x["steps"] for x in rows)
    mk = sorted(((n, v) for n, v in made.items() if n.startswith("made:" + rid + ":")), key=lambda x: -x[1][1])
    made_steps = sum(v[1] for _, v in mk)
    result[rid] = dict(steps=total[rid], in_definitions=in_defs, made_table_programs=made_steps, top_level=total[rid] - in_defs - made_steps, definitions=rows,
                       made=[dict(name=n, builds=v[0], steps=v[1], rows=v[2]) for n, v in mk])
    print(f"\n### {rid}: {total[rid]:,} steps; {in_defs:,} inside {len(rows)} definitions, {made_steps:,} in made-table programs, {total[rid] - in_defs - made_steps:,} at top level")
    print("| definition (plan step) | steps | keyed calls | distinct keys | rows produced | table builds / abandoned | build steps | old engine: op, runs, rows | steps / old rows |")
    print("|---|---:|---:|---:|---:|---:|---:|---|---:|")
    for x in rows[:top]:
        o = "-" if x["old_rows"] is None else f"{x['old_op']}, {x['old_runs']:,}, {x['old_rows']:,}"
        ratio = "-" if x["old_rows"] is None else f"{x['steps'] / max(x['old_rows'], 1):.1f}"
        print(f"| {' '.join(str(t) for t in x['definition'])} | {x['steps']:,} | {x['calls']:,} | {x['distinct_keys']:,} | {x['rows']:,} | {x['table_builds']:,} / {x['table_abandoned']:,} | {x['build_steps']:,} | {o} | {ratio} |")
    for m in result[rid]["made"][:6]:
        print(f"| {m['name']} (made table) | {m['steps']:,} | {m['builds']:,} builds | | {m['rows']:,} | | | | |")
if out_json:
    json.dump(result, open(out_json, "w"))
