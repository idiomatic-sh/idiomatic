#!/usr/bin/env python3
"""factfreq_big.py: last round's diag/factfreq.py skipped fact files over 100 MB, so their columns had no value counts and a
literal bucket of one (a link kind that holds most rows) was sized like a rare name. This adds the same counts (symbol, number
and flag columns with at most 256 distinct values) for every relation the first pass skipped for size, tree relations aside
(their kind, text and field counts are the freq file's own keys). Writes freq3.json = freq2.json + those relations."""
import collections, json, os
F = "/workspaces/ryo-poc-evidence/next/swarm/export/facts/django/"
H = "/workspaces/ryo-poc-evidence/next/swarm/cut/"
TREE = {"semantic-syntax", "syntax-tree", "semantic-child", "syntax-child", "source-location", "identifier"}
schema = json.load(open(F + "schema.json"))["relations"]
fr = json.load(open(H + "freq2.json"))
added = {}
for rel, s in schema.items():
    p = F + rel + ".tsv"
    if rel in fr["facts"] or rel in TREE or not os.path.exists(p) or os.path.getsize(p) <= 100_000_000:
        continue
    cols = [i for i, c in enumerate(s["columns"]) if c["kind"] in ("symbol", "number", "flag")]
    cnt = {i: collections.Counter() for i in cols}
    dead = set()
    with open(p) as f:
        for line in f:
            cells = line.rstrip("\n").split("\t")
            for i in cols:
                if i in dead:
                    continue
                c = cnt[i]
                c[cells[i + 1]] += 1
                if len(c) > 256:
                    dead.add(i)
    rc = {str(i): dict(cnt[i]) for i in cols if i not in dead}
    if rc:
        fr["facts"][rel] = rc
        added[rel] = {k: dict(sorted(v.items(), key=lambda x: -x[1])[:6]) for k, v in rc.items()}
json.dump(fr, open(H + "freq3.json", "w"))
print(json.dumps(added, indent=1))
