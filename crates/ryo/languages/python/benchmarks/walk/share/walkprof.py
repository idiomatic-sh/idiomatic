#!/usr/bin/env python3
"""walkprof.py <linked pack.json> <NAV_INS_PROF stderr>: the steps INSIDE nearest-ancestor walks and closures (the programs nested
in `near`, `nearc`, `clos`), by who holds the walk: a rule's own program, a definition or made table one rule of the set owns,
or one several own. nodeprof.py cannot place these by sub-plan (a walk's inner variables are one set for every copy), so
this places them by the piece that runs them. Also the times the walk instruction itself ran (= probes, were it a table)."""
import collections, json, os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
sys.setrecursionlimit(20000)
import nav
H = os.path.dirname(os.path.abspath(__file__))
pack = json.load(open(sys.argv[1]))
P = json.load(open(H + "/../export/plans/python.json"))
nodes = P["program"]["nodes"]; ordn = {r["ordinal"]: r["id"] for r in P["rules"]}
idset = {r["id"] for r in pack["rules"]}
meta = pack["shared_meta"]
owners = lambda n: sum(1 for o in nodes[n]["owners"] if ordn.get(o) in idset)
seq = [0]
info = []   # per instruction: (holder class, inside a walk, is a walk instruction)


def walk(p, holder, inside):
    for ins in p:
        info.append((holder, inside, ins[0] in ("near", "nearc", "clos")))
        seq[0] += 1
        for k in nav.SUBS_AT.get(ins[0], ()):
            for q in ins[k]:
                walk(q, holder, inside or ins[0] in ("near", "nearc", "clos"))


def cls(n):
    return "unnamed" if n is None else ("shared" if owners(n) > 1 else "one rule's")


for m in pack["made"]:
    for q in m["programs"] + m.get("right_programs", []):
        walk(q, "made table: " + cls(m.get("plan_node")), False)
for e, q in enumerate(pack["shared"]):
    walk(q, "definition: " + cls(meta[e][4] if len(meta[e]) > 4 else None), False)
for r in pack["rules"]:
    for alts in r["nav"]["programs"]:
        for a in alts:
            walk(a["prog"], "a rule's own program", False)
    for a in r["nav"].get("aux", []):
        for q in a["programs"]:
            walk(q, "a rule's own program", False)
inside = collections.Counter(); walks = collections.Counter(); total = 0
for line in open(sys.argv[2]):
    if line.startswith('{"navins"'):
        for i, s, f in json.loads(line)["navins"]:
            total += s
            h, ins_, w = info[i]
            if ins_:
                inside[h] += s
            if w:
                walks[h] += s
print(json.dumps(dict(steps=total, steps_inside_walks=sum(inside.values()), share=round(sum(inside.values()) / total, 4), inside_by_holder=dict(inside.most_common()),
                      times_a_walk_ran_by_holder=dict(walks.most_common())), indent=1))
