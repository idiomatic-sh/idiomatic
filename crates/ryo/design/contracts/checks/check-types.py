#!/usr/bin/env python3
"""check-types.py [CONTRACTS_DIR]: every type defined (pub struct/enum/trait/type) in an api.txt is defined in exactly
one api.txt. Prints each type defined by two sheets."""
import os, re, sys, collections
ROOT = sys.argv[1] if len(sys.argv) > 1 else os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DEF = re.compile(r"^\s*pub (?:struct|enum|trait|type|union) ((?:[A-Za-z0-9_]+::)*)([A-Z][A-Za-z0-9_]*)")
owners = collections.defaultdict(set)
for dirpath, _, files in os.walk(ROOT):
    if "api.txt" not in files or "/inputs" in dirpath: continue
    rel = os.path.relpath(dirpath, ROOT)
    for line in open(os.path.join(dirpath, "api.txt"), errors="replace"):
        if line.lstrip().startswith("#"): continue
        m = DEF.match(line)
        if m and m.group(1).count("::") <= 2: owners[m.group(2)].add(rel)
fails = 0
for t, rels in sorted(owners.items()):
    if len(rels) > 1: print(f"{t}: defined in {', '.join(sorted(rels))}"); fails += 1
print(f"check-types: {len(owners)} types, {fails} defined twice", file=sys.stderr); sys.exit(1 if fails else 0)
