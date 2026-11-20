#!/usr/bin/env python3
"""packprof.py <linked pack.json> <stderr of a NAV_PROF run of it> [top n] [--json out]: per DISTINCT definition of the pack (the
shared entries' own steps summed over its binding patterns; a made table's programs), the rules whose programs reach it, and
the steps, each counted once (the run attributes them to the rule that asked first). Names: `step n` / `union n` /
`join n side` = the compiler's merged node n; `own` = an attempt's own cut; `sub` = an unnamed repeated sub-program."""
import collections, json, os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
sys.setrecursionlimit(20000)
import nav
args = [a for a in sys.argv[1:] if not a.startswith("--")]
out_json = sys.argv[sys.argv.index("--json") + 1] if "--json" in sys.argv else None
top = int(args[2]) if len(args) > 2 else 25
pack = json.load(open(args[0]))
table, meta, made = pack["shared"], pack["shared_meta"], {m["name"]: m for m in pack["made"]}


def name(m):
    k = m[0]
    if k is None:
        return "sub"
    return " ".join("own" if isinstance(t, list) else str(t) for t in k)


reach_memo = {}


def reach_entry(i):
    """shared entries and made tables one entry reaches (itself included)"""
    if i not in reach_memo:
        reach_memo[i] = (set(), set())
        es, ms = {i}, set()
        walk(table[i], es, ms)
        reach_memo[i] = (es, ms)
    return reach_memo[i]


def names(x, es, ms):
    if isinstance(x, str):
        if x.startswith("made:") and x not in ms:
            ms.add(x)
            for q in made[x]["programs"] + made[x].get("right_programs", []):
                walk(q, es, ms)
    elif isinstance(x, list):
        for y in x:
            names(y, es, ms)


def walk(p, es, ms):
    for ins in p:
        if ins[0] == "shared":
            if ins[1] not in es:
                e2, m2 = reach_entry(ins[1])
                es |= e2
                for n in m2 - ms:
                    names(n, es, ms)
            continue
        subs = nav.SUBS_AT.get(ins[0], ())
        for k in subs:
            for q in ins[k]:
                walk(q, es, ms)
        kt = nav.table_at(ins)
        if kt is not None:
            walk(ins[kt][0][0], es, ms)
        for j, y in enumerate(ins):
            if j not in subs and j != kt:
                names(y, es, ms)


holders = collections.defaultdict(set)
per_rule_entries = {}
for r in pack["rules"]:
    es, ms = set(), set()
    for alts in r["nav"]["programs"]:
        for a in alts:
            walk(a["prog"], es, ms)
    for a in r["nav"].get("aux", []):
        for q in a["programs"]:
            walk(q, es, ms)
    per_rule_entries[r["id"]] = (len(es), len(ms))
    for i in es:
        holders[("def", name(meta[i]))].add(r["id"]) if meta[i][0] is not None else holders[("sub", i)].add(r["id"])
    for n in ms:
        holders[("made", n)].add(r["id"])
own = collections.Counter()
made_steps = collections.Counter()
total = collections.Counter()
for line in open(args[1]):
    if line.startswith('{"navprof"'):
        for i, c, incl, o in json.loads(line)["navprof"]:
            own[("def", name(meta[i])) if meta[i][0] is not None else ("sub", i)] += o
    elif line.startswith('{"navkprof"'):
        for n, b, s, k in json.loads(line)["navmade"]:
            made_steps[("made", n)] += s
    elif line.startswith('{"nav"'):
        for rid, (a, b, c) in json.loads(line)["nav"]["per_rule"].items():
            total[rid] += c
steps = own + made_steps
T = sum(total.values())
in_defs = sum(steps.values())
multi = sum(s for k, s in steps.items() if len(holders[k]) > 1)
res = dict(rules=len(pack["rules"]), steps=T, steps_top_level=T - in_defs,
           distinct_definitions=sum(1 for k in holders if k[0] == "def"), made_tables=sum(1 for k in holders if k[0] == "made"),
           unnamed_sub_programs=sum(1 for k in holders if k[0] == "sub"),
           definitions_summed_over_rules=sum(len(v) for k, v in holders.items() if k[0] != "sub"),
           held_by_more_than_one_rule=sum(1 for k, v in holders.items() if k[0] != "sub" and len(v) > 1),
           steps_in_definitions_more_than_one_rule_holds=multi, steps_in_definitions_one_rule_holds=in_defs - multi,
           shared_entries=len(table), entries_summed_over_rules=sum(v[0] for v in per_rule_entries.values()))
res["largest"] = [dict(definition=k[1] if k[0] != "sub" else f"sub {k[1]}", steps=s, rules_holding=len(holders[k]), rules=sorted(holders[k])[:8])
                  for k, s in steps.most_common(top)]
print(json.dumps({k: v for k, v in res.items() if k != "largest"}, indent=1))
print("| definition | steps (once) | rules holding it | rules |")
print("|---|---:|---:|---|")
for x in res["largest"]:
    print(f"| {x['definition']} | {x['steps']:,} | {x['rules_holding']} | {', '.join(x['rules'])}{' ..' if x['rules_holding'] > 8 else ''} |")
if out_json:
    json.dump(res, open(out_json, "w"), indent=1)
