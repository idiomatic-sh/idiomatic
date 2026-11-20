#!/usr/bin/env python3
"""prof.py <program.json (one rule, lowered with NAV_META=1)> <stderr of a NAV_PROF=1 run> [plans.json]: where the navigation
steps of the rule went, by class of shared entry: the doubling chain (definitions of the self-join steps J1..Jk), J0 and the made
tables under it, literal-bucket scans in a bound context, the same uncorrelated, everything else. One JSON object on stdout."""
import collections, json, pickle, sys, os
prog = json.load(open(sys.argv[1]))["rules"][0]
rid = prog["id"]
nav = prog["nav"]
PK = "/tmp/claude-1000/-workspaces-idiomatic/3d52d4bd-d543-48d6-87bb-d5a9b9b3e779/scratchpad/rules.pkl"
rules = pickle.load(open(PK, "rb")) if os.path.exists(PK) else {r["id"]: r for r in json.load(open(sys.argv[3]))["rules"]}
steps = rules[rid]["plan"]["steps"]
chain = set()
for i, s in enumerate(steps):
    p = s["params"]
    if s["op"] == "join" and p["left"] == p["right"] and "step" in p["left"] and len(p["left_on"]) >= 1 and \
            p["left_on"][:-1] == p["right_on"][:-1] and p["left_on"][-1] != p["right_on"][-1]:
        chain.add(i)
shared, meta = nav["shared"], nav.get("shared_meta", [])
LIT = ("kind", "scan")


def own_literal_iter(p):
    for ins in p:
        if ins[0] == "iter":
            if ins[2] in LIT:
                return True
            if ins[2] in ("value", "by") and not isinstance(ins[3][1], int):
                return True
    return False


def own_iters(p):
    out = []
    for ins in p:
        if ins[0] == "iter":
            lit = ins[2] in LIT or (ins[2] in ("value", "by") and not isinstance(ins[3][1], int))
            out.append(f"{ins[1]}:{ins[2]}" + ("" if ins[2] == "scan" else (":lit" if lit else ":bound")))
        elif ins[0] in ("clos", "near", "nearc", "axis"):
            out.append(ins[0] if ins[0] != "axis" else "axis:" + ins[1])
    return out


def calls(p):
    return [ins for ins in p if ins[0] in ("or-keyed", "neg-keyed", "exists-keyed", "or", "neg", "exists", "neg-memo", "exists-memo")]


def cls(i):
    if i == len(shared):
        return "top-level programs"
    if i == len(shared) + 1:
        return "made-table builds (ordered-match inputs)"
    p = shared[i]
    m = meta[i] if i < len(meta) else None
    if m and m[0][0] in ("step", "join", "union") and m[0][1] in chain:
        return "doubling chain (J1..Jk bodies)"
    lit = own_literal_iter(p)
    if m and m[3] == "group":
        return "group entries (run every branch)"
    if lit and m and m[1]:
        return "literal-bucket scan, bound context"
    if lit and m:
        return "literal-bucket scan, uncorrelated (once per file)"
    if lit:
        return "literal-bucket scan, extracted sub-program"
    return "other (keyed reads, tests, calls)"


tot = collections.defaultdict(lambda: [0, 0, 0])
per = collections.defaultdict(lambda: [0, 0, 0])
keyed = [0, 0]
for line in open(sys.argv[2]):
    if line.startswith('{"prof"'):
        d = json.loads(line)
        keyed[0] += d["keyed_calls"]; keyed[1] += d["keyed_hits"]
        for i, runs, st, vis in d["prof"]:
            c = cls(i)
            for k, v in enumerate((runs, st, vis)):
                tot[c][k] += v
                per[i][k] += v
allsteps = sum(v[1] for v in tot.values())
out = dict(rule=rid, steps=allsteps, keyed_calls=keyed[0], keyed_hits=keyed[1], chain_steps_in_plan=len(chain),
           classes={c: dict(runs=v[0], steps=v[1], visits=v[2], share=round(v[1] / max(allsteps, 1), 4)) for c, v in sorted(tot.items(), key=lambda x: -x[1][1])})
top = sorted(per.items(), key=lambda x: -x[1][1])[:12]
out["top_entries"] = [dict(entry=i, cls=cls(i), meta=(meta[i][:3] if i < len(meta) else None), runs=v[0], steps=v[1], visits=v[2],
                           iters=own_iters(shared[i]) if i < len(shared) else None) for i, v in top]
print(json.dumps(out, indent=1))
