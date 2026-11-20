#!/usr/bin/env python3
"""subset.py <pack.json> <rules file> <out.json> [plain]: the lowered pack restricted to the rules named: their own programs and the
shared entries, made tables and register kinds those reach (nav.link). A set run as a subset of one pack lowering reads the
same definition programs as the whole pack does."""
import json, os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
sys.setrecursionlimit(20000)
import nav
pack = json.load(open(sys.argv[1]))
ids = set(open(sys.argv[2]).read().split())
out = nav.link(pack, ids)
if len(sys.argv) > 4 and sys.argv[4] == "plain":
    # without what only the diagnostics read (each register's chain of plan steps)
    out.pop("origin", None)
    out.pop("paths", None)
json.dump(out, open(sys.argv[3], "w"))
print(len(out["rules"]), "rules;", len(out["shared"]), "of", len(pack["shared"]), "shared entries;", len(out["made"]), "of", len(pack["made"]), "made tables;",
      len(out["kinds"]), "registers")
