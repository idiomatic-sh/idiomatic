#!/usr/bin/env python3
"""forms.py <program.json> [rule..]: how the lowered programs evaluate definitions: counts of each instruction form (and of
made tables by kind), over the whole program or the named rules; and the rules whose programs name more than 65,536 variables."""
import collections, json, sys
d = json.load(open(sys.argv[1])); only = set(sys.argv[2:])
SUBS_AT = {"neg": (1,), "exists": (1,), "neg-memo": (1,), "exists-memo": (1,), "neg-keyed": (1,), "exists-keyed": (1,), "or": (2,), "or-keyed": (2,), "clos": (7,), "near": (3,), "nearc": (5, 6), "or-probe": (2,), "neg-probe": (1,), "exists-probe": (1,), "need": (3,)}
c = collections.Counter(); rules_with = collections.defaultdict(set); big = []
def walk(p, rid):
    for ins in p:
        op = ins[0]
        name = None
        if op == "or-keyed":
            name = "keyed call" + (" + bounded table" if len(ins) > 5 and ins[5] else "")
        elif op in ("neg-keyed", "exists-keyed"):
            name = "keyed test" + (" + bounded table" if len(ins) > 4 and ins[4] else "")
        elif op == "or-probe":
            name = "probe of a table" + (" (no key: its rows)" if not ins[3] else "")
        elif op in ("neg-probe", "exists-probe"):
            name = "probe test of a table" + (" (no key)" if not ins[2] else "")
        elif op == "or":
            name = "uncorrelated rows, memoised (older placement)" if len(ins) > 3 else "union in place"
        elif op in ("neg-memo", "exists-memo"):
            name = "uncorrelated test, memoised"
        elif op == "need":
            name = "need" + (" with tables" if ins[3] else "")
        elif op == "iter" and ins[2] in ("byk", "bym"):
            name = "composite-key read (" + ins[2] + ")"
        elif op in ("clos", "nearc"):
            name = op
        if name:
            c[name] += 1; rules_with[name].add(rid)
        for k in SUBS_AT.get(op, ()):
            for q in ins[k]:
                walk(q, rid)
for r in d["rules"]:
    if only and r["id"] not in only:
        continue
    nav = r["nav"]
    for p in nav["shared"]:
        walk(p, r["id"])
    for alts in nav["programs"]:
        for a in alts:
            walk(a["prog"], r["id"])
    for m in nav["made"]:
        k = "made table: " + m.get("op", "group count"); c[k] += 1; rules_with[k].add(r["id"])
        for q in m.get("programs", []) + m.get("right_programs", []):
            walk(q, r["id"])
    if len(nav["kinds"]) > 65536:
        big.append((r["id"], len(nav["kinds"])))
out = {k: dict(instructions=v, rules=len(rules_with[k])) for k, v in sorted(c.items(), key=lambda x: -x[1])}
print(json.dumps(out, indent=1)); print("rules over 65,536 variables:", sorted(big, key=lambda x: -x[1]))
