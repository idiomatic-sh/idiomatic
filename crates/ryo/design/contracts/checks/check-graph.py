#!/usr/bin/env python3
"""check-graph.py [CONTRACTS_DIR]: reads the `IMPORTS:` line of every sheet.md; fails on a cycle, on an import against
the directions, and on an import of a crate with no sheet."""
import os, re, sys
ROOT = sys.argv[1] if len(sys.argv) > 1 else os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ORDER = {"domain": ["table", "symbols", "relations", "operators", "ryoql", "plan", "programs"],
         "application": ["screen", "execute", "project"]}
MAY = {"domain": {"domain"}, "outbound": {"domain", "outbound"}, "languages": {"domain", "outbound"},
       "application": {"domain", "outbound", "application"}, "inbound": {"domain", "outbound", "application"},
       "edges": {"domain", "outbound", "application", "inbound", "languages"}}
crates, imports, fails = {}, {}, 0
for dirpath, _, files in os.walk(ROOT):
    if "sheet.md" not in files or "/inputs" in dirpath: continue
    rel = os.path.relpath(dirpath, ROOT)
    if rel.startswith(("briefs", "benchmarks", "checks")): continue
    folder = rel.split("/")[0]; name = rel.split("/")[-1]
    crates[name] = folder
    text = open(os.path.join(dirpath, "sheet.md"), errors="replace").read()
    m = re.search(r"^\s*-?\s*\**IMPORTS:?\**\s*:?\s*(.+)$", text, re.M)
    if not m: print(f"{rel}: no IMPORTS: line"); fails += 1; imports[name] = []; continue
    body = re.sub(r"\(.*?\)", "", m.group(1)).replace("`", "")
    imports[name] = [] if re.match(r"\s*none\b", body, re.I) else [w.strip().split("/")[-1] for w in re.split(r"[,;]", body) if w.strip()]
for name, deps in imports.items():
    f = crates[name]
    for d in deps:
        d = re.sub(r"[^a-z_]", "", d.lower())
        if not d: continue
        if d not in crates: print(f"{f}/{name}: imports {d}, which has no sheet"); fails += 1; continue
        g = crates[d]
        if g not in MAY.get(f, set()): print(f"{f}/{name}: imports {g}/{d}, against the directions"); fails += 1
        elif f == g and f in ORDER and ORDER[f].index(d) >= ORDER[f].index(name):
            print(f"{f}/{name}: imports {d}, which comes after it in the {f} order"); fails += 1
seen, stack = set(), []
def visit(n):
    global fails
    if n in stack: print("cycle: " + " -> ".join(stack[stack.index(n):] + [n])); fails += 1; return
    if n in seen: return
    seen.add(n); stack.append(n)
    for d in imports.get(n, []):
        d = re.sub(r"[^a-z_]", "", d.lower())
        if d in crates: visit(d)
    stack.pop()
for n in imports: visit(n)
print(f"check-graph: {len(crates)} sheets, {fails} failures", file=sys.stderr); sys.exit(1 if fails else 0)
