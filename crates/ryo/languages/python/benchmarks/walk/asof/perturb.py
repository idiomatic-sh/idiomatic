#!/usr/bin/env python3
"""perturb.py <plans.json> <out dir> <rule..>: copies of the rules' plans that mean the same but miss the recognised shape, to
prove the fallbacks (each writes <out>/<variant>.plans.json, same program nodes):
  A  the right input of every doubling join reads its input through an identity projection (left != right): no chain is
     recognised, the step is lowered as before (definitions and calls: the slow path);
  B  the event self-pointers read the events through a select with no comparison: the chain is recognised, the as-of shape
     is not: the general `power` table;
  C  the first semi-join above each chain reads the chain through a select with no comparison: the as-of table without the
     folded semi-joins.
New steps are appended (ids past the plan's), so no existing step id moves."""
import copy, json, os, sys
plans = json.load(open(sys.argv[1]))
out = sys.argv[2]
want = set(sys.argv[3:])
os.makedirs(out, exist_ok=True)


def width(steps, v):
    st = steps[v["step"]]
    p = st["params"]
    if st["op"] == "project":
        return len(p["columns"])
    if st["op"] in ("join", "ordered_match"):
        return len(p["output"])
    if st["op"] in ("select",):
        return len(p["kept"]) if p["kept"] else width(steps, p["from"])
    if st["op"] in ("union",):
        return width(steps, p["left"])
    if st["op"] == "extend":
        return width(steps, p["from"]) + len(p["values"])
    raise SystemExit("width of " + st["op"])


def doubling(st):
    p = st["params"]
    return st["op"] == "join" and p["left"] == p["right"] and "step" in p["left"] and not p["filter"] and \
        sum(a != b for a, b in zip(p["left_on"], p["right_on"])) == 1


def add(rule, op, params):
    steps = rule["plan"]["steps"]
    steps.append(dict(step=len(steps), scope="per-file", op=op, params=params))
    rule["step_nodes"].append(None)
    return {"step": len(steps) - 1}


for variant in "ABC":
    rules = []
    counts = {}
    for r in plans["rules"]:
        if r["id"] not in want:
            continue
        r = copy.deepcopy(r)
        steps = r["plan"]["steps"]
        n = 0
        tops = set()
        for i in range(len(steps)):
            st = steps[i]
            if not doubling(st):
                continue
            src = st["params"]["left"]
            if variant == "A":
                st["params"]["right"] = add(r, "project", {"from": src, "columns": list(range(width(steps, src))), "distinct": True})
                n += 1
            if not doubling(steps[src["step"]]) and variant == "B":
                # the base J(0): under identity projections a union of the probe pointers and the event self-pointers
                v = src
                while steps[v["step"]]["op"] == "project":
                    v = steps[v["step"]]["params"]["from"]
                u = steps[v["step"]]
                if u["op"] == "union":
                    for side in ("left", "right"):
                        x = u["params"][side]
                        xp = steps[x["step"]]
                        if xp["op"] == "project" and len(set(xp["params"]["columns"])) < len(xp["params"]["columns"]):
                            xp["params"]["from"] = add(r, "select", {"from": xp["params"]["from"], "comparisons": [], "kept": []})
                            n += 1
        if variant == "C":
            dbl = {i for i in range(len(steps)) if doubling(steps[i])}
            inner = {steps[i]["params"]["left"]["step"] for i in dbl}
            for i in range(len(steps)):
                st = steps[i]
                if st["op"] == "join" and i not in dbl and st["params"]["left"].get("step") in dbl - inner:
                    st["params"]["left"] = add(r, "select", {"from": st["params"]["left"], "comparisons": [], "kept": []})
                    n += 1
        counts[r["id"]] = n
        rules.append(r)
    json.dump(dict(pack=plans.get("pack"), rules=rules, program=dict(nodes=plans["program"]["nodes"])), open(f"{out}/{variant}.plans.json", "w"))
    print(variant, counts)
