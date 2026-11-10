"""Render the concept inventory tables from the raw RyoQL answers in raw/.

Usage: python3 render.py <inventory-dir>
Reads raw/{files,items,dependencies,crate-imports}.json, writes the Markdown tables.
Joins RyoQL cannot express on this binary (see README) are done here: concept
ancestor attachment, cross-file name counts, crate-name mapping.
"""
import json, sys, collections as C, os

OUT = sys.argv[1]
MARKERS = {"service", "types", "interface", "adapter", "use_case"}
LIBS = {"ryo_" + c: c for c in "benchmark benchmark_contracts core derived execution execution_domain execution_runtime execution_store language matching operators query_language relations values".split()}


def load(name):
    d = json.load(open(f"{OUT}/raw/{name}.json"))
    return [dict(zip(d["columns"], r)) for r in d["rows"]]


files, items, deps, imps, allfns = (load(n) for n in ("files", "items", "dependencies", "crate-imports", "functions"))
LAYERS = ("domain", "application", "inbound", "outbound")

concepts = {}  # (crate, layer, dir) -> dict
for f in files:
    if f["name"] in MARKERS and f["layer"] in LAYERS:
        concepts.setdefault((f["crate"], f["layer"], f["dir"]), {"lines": 0, "files": 0, "markers": set()})["markers"].add(f["name"])


def concept_of(crate, layer, dirpath):
    parts = dirpath.split("/") if dirpath else []
    for n in range(len(parts), 0, -1):
        k = (crate, layer, "/".join(parts[:n]))
        if k in concepts:
            return k
    return None


def file_concept(path):
    p = path.split("/")
    if len(p) < 6 or p[2] != "src" or p[3] not in LAYERS:
        return None
    return concept_of(p[1], p[3], "/".join(p[4:-1]))


unattached = C.Counter()
for f in files:
    k = concept_of(f["crate"], f["layer"], f["dir"])
    if k:
        concepts[k]["lines"] += f["lines"]
        concepts[k]["files"] += 1
    else:
        unattached[(f["crate"], f["layer"])] += f["lines"]


def cname(k):
    return f"{k[0]}:{k[1]}/{k[2]}"


def table(head, rows):
    out = ["| " + " | ".join(head) + " |", "|" + "---|" * len(head)]
    out += ["| " + " | ".join(str(c) for c in r) + " |" for r in rows]
    return "\n".join(out) + "\n"


# imports
out_edges = C.defaultdict(C.Counter)  # concept -> concept (same crate)
cross = C.defaultdict(C.Counter)      # concept -> (crate, concept or '(crate root)')
for e in deps:
    a, b = file_concept(e["source"]), file_concept(e["target"])
    if a and b and a != b:
        out_edges[a][b] += 1
    elif a and not b:
        pass
for e in imps:
    a = file_concept(e["file"])
    segs = e["target"].split("::")
    crate = LIBS.get(segs[0])
    if not a or not crate or crate == a[0]:
        continue
    tgt = None
    if len(segs) > 2 and segs[1] in LAYERS:
        for n in range(len(segs) - 1, 1, -1):
            k = (crate, segs[1], "/".join(segs[2:n + 1]))
            if k in concepts:
                tgt = k
                break
    cross[a][tgt if tgt else (crate, "(root re-export)", "")] += 1

imported_by = C.defaultdict(C.Counter)
for a, m in out_edges.items():
    for b, n in m.items():
        imported_by[b][a] += n
for a, m in cross.items():
    for b, n in m.items():
        if b in concepts:
            imported_by[b][a] += n

# 1 concepts
keys = sorted(concepts)
CRATES = sorted({k[0] for k in keys})


def per_crate(folder, title, intro, make):
    os.makedirs(f"{OUT}/{folder}", exist_ok=True)
    for c in CRATES:
        open(f"{OUT}/{folder}/{c}.md", "w").write(f"# {title}: {c}\n\n{intro}\n\n" + make(c))


per_crate("concepts", "Concepts", "One row per concept directory (a directory holding service.rs, types.rs, interface.rs, adapter.rs or use_case.rs). Lines are production lines of every non-test file in the directory and its subdirectories that no deeper concept owns.",
          lambda c: table(["layer", "concept", "production lines", "files", "marker files"],
                          [(k[1], k[2], concepts[k]["lines"], concepts[k]["files"], ", ".join(sorted(concepts[k]["markers"]))) for k in keys if k[0] == c])
          + "\nProduction lines in directories with no concept ancestor:\n\n"
          + table(["layer", "production lines"], [(l, n) for (cc, l), n in sorted(unattached.items()) if cc == c]))

# 2 items
by_concept = C.defaultdict(list)
for i in items:
    k = concept_of(i["crate"], i["layer"], i["dir"])
    by_concept[k].append(i)
def item_rows(c):
    rows = []
    for k in [k for k in keys if k[0] == c]:
        lst = by_concept.get(k, [])
        if lst:
            cell = "; ".join(f"{i['visibility']} {i['kind']} {i['name']}" for i in sorted(lst, key=lambda i: (i["kind"], i["name"])))
            rows.append((f"{k[1]}/{k[2]}", len(lst), cell))
    return table(["concept", "items", "items defined"], rows)


per_crate("items", "Items", "One row per concept: every struct, enum, trait, function and type alias written with a visibility other than private, as `visibility kind name`.", item_rows)

# 3 imports
def import_rows(c):
    rows = []
    for k in [k for k in keys if k[0] == c]:
        rows.append(import_row(k))
    return table(["concept", "imports (same crate)", "imports (other crates)", "imported by"], rows)


def import_row(k):
    o = [f"{cname(b)} ({n})" for b, n in sorted(out_edges[k].items())]
    x = [f"{cname(b) if b[1] != '(root re-export)' else b[0] + ' root'} ({n})" for b, n in sorted(cross[k].items())]
    ib = [f"{cname(a)} ({n})" for a, n in sorted(imported_by[k].items())]
    return (f"{k[1]}/{k[2]}", "; ".join(o) or "-", "; ".join(x) or "-", "; ".join(ib) or "-")


per_crate("imports", "Imports", "Per concept: the concepts of its own crate it imports from (source-dependency edges, with the count of edges), the concepts and crate roots of other crates it imports from (static-import of a `ryo_*` path), and the concepts that import it.", import_rows)

# 4 duplicates
types = C.defaultdict(list)
fns = C.defaultdict(list)
for i in items:
    if i["visibility"] != "pub":
        continue
    k = concept_of(i["crate"], i["layer"], i["dir"])
    where = (i["crate"], cname(k) if k else f"{i['crate']}:{i['layer']}/{i['dir']}", i["file"])
    if i["kind"] in ("struct", "enum", "trait", "type-alias"):
        types[i["name"]].append((i["kind"], where))
    elif i["kind"] == "function":
        fns[(i["layer"], i["name"])].append(where)
duptypes = sorted(((n, v) for n, v in types.items() if len({w[0] for _, w in v}) > 1), key=lambda t: (-len({w[0] for _, w in t[1]}), t[0]))
dupfns = sorted(((k, v) for k, v in fns.items() if len({w[1] for w in v}) > 1), key=lambda t: (-len({w[1] for w in t[1]}), t[0]))
trows = [(n, ", ".join(sorted({w[0] for _, w in v})), "/".join(sorted({kd for kd, _ in v})), "; ".join(sorted(w[1] for _, w in v))) for n, v in duptypes]
frows = [(k[1], k[0], len({w[1] for w in v}), "; ".join(sorted({w[1] for w in v}))) for k, v in dupfns]
open(f"{OUT}/duplicates.md", "w").write(
    "# Duplicates\n\n## Public types and traits defined in more than one crate\n\nA name written `pub struct`, `pub enum`, `pub trait` or `pub type` in two or more crates, with every concept that defines it.\n\n"
    + table(["name", "crates", "kind", "locations"], trows)
    + "\n## Public functions defined in more than one concept of the same layer\n\nA name written `pub fn` in two or more concepts of one layer, across all crates.\n\n"
    + table(["name", "layer", "concepts", "locations"], frows))

# any-visibility function names across crates
anyfn = C.defaultdict(list)
for i in allfns:
    k = concept_of(i["crate"], i["layer"], i["dir"])
    anyfn[i["name"]].append((i["crate"], cname(k) if k else f"{i['crate']}:{i['layer']}/{i['dir']}"))
common = sorted(((n, v) for n, v in anyfn.items() if len({c for c, _ in v}) > 1), key=lambda t: (-len({c for c, _ in t[1]}), t[0]))
arows = [(n, ", ".join(sorted({c for c, _ in v})), "; ".join(sorted({w for _, w in v}))) for n, v in common]
oe = [(n, v) for n, v in common if {"operators", "execution"} <= {c for c, _ in v}]
open(f"{OUT}/duplicates.md", "a").write(
    "\n## Function names declared in more than one crate, any visibility\n\nA free function name declared in two or more crates whatever its visibility, so a private copy of another crate's function shows. Methods are not listed.\n\n"
    + table(["name", "crates", "locations"], arows))

# 5 hollow
hollow = [k for k in keys if concepts[k]["lines"] == 0]
nodecl = [k for k in keys if not by_concept.get(k) and concepts[k]["lines"] > 0]
open(f"{OUT}/hollow.md", "w").write(
    "# Hollow concepts\n\n## Zero production lines\n\n"
    + (table(["crate", "layer", "concept"], [k for k in hollow]) if hollow else "None.\n")
    + "\n## Production lines but no item with a visibility\n\nThe directory holds code, but no struct, enum, trait, function or type alias is written with a visibility other than private: what it holds is re-exports, module declarations or private helpers.\n\n"
    + (table(["crate", "layer", "concept", "production lines"], [(*k, concepts[k]["lines"]) for k in nodecl]) if nodecl else "None.\n"))

# execution internals
EXE = "execution"
ex = C.defaultdict(lambda: C.Counter())
for a, m in cross.items():
    for b, n in m.items():
        if b[0] == EXE and a[0] != EXE:
            ex[a[0]][b] += n
exrows = []
for crate in sorted(ex):
    internal = {b: n for b, n in ex[crate].items() if b[1] not in ("inbound",)}
    if internal:
        exrows.append((crate, sum(internal.values()), "; ".join(f"{cname(b) if b[1] != '(root re-export)' else 'execution root'} ({n})" for b, n in sorted(internal.items()))))
open(f"{OUT}/execution-internals.md", "w").write(
    "# Imports of execution internals\n\nCrates other than execution that import a path in execution's domain, application or outbound layers (inbound is the public use-case surface), with the concept imported and the import count.\n\n"
    + (table(["importing crate", "imports", "execution concepts imported"], exrows) if exrows else "None.\n"))

inb = [(a[0], cname(b), n) for a, m in cross.items() for b, n in m.items() if b[0] == EXE and a[0] != EXE and b[1] == "inbound"]
agg = C.Counter()
for c, b, n in inb:
    agg[(c, b)] += n
open(f"{OUT}/execution-internals.md", "a").write(
    "\n## Imports of execution's inbound layer\n\nThe public surface, for comparison.\n\n"
    + table(["importing crate", "execution concept", "imports"], [(c, b, n) for (c, b), n in sorted(agg.items())]))

# summary
top = duptypes[:15]
summ = [f"{n} ({', '.join(sorted({w[0] for _, w in v}))})" for n, v in top]
open(f"{OUT}/summary.json", "w").write(json.dumps({
    "concepts": len(concepts), "dup_types": len(duptypes), "dup_fns": len(dupfns), "hollow": [cname(k) for k in hollow],
    "nodecl": [cname(k) for k in nodecl], "top": summ, "exec_crates": [r[0] for r in exrows], "opexec": [n for n, _ in oe], "anyfn": len(common)}, indent=1))
print(json.dumps(json.load(open(f"{OUT}/summary.json")), indent=1))
