"""Write the "Implementations per kind" section of inventory.md.

Usage: python3 registries.py <inventory-dir> <ryo-crates-dir>
Kinds come from RyoQL rows in raw/ (enum members, relation member directories)
except the two macro-declared kind lists, read by grep and marked "grep".
An implementation of a kind is a concept directory, in the crates searched for
that registry, that declares a free function or a trait impl whose name carries
the kind's name. That is a name match, not a call graph: it finds where the code
for a kind lives, and the counts are candidates for the review to read.
A kind is behind a port when its implementing files hold `impl <Port> for` for a
trait the workspace declares and that has two or more impls; otherwise reached directly.
"""
import json, re, sys, collections as C, subprocess

OUT, CRATES = sys.argv[1], sys.argv[2]


def load(n):
    d = json.load(open(f"{OUT}/raw/{n}.json"))
    return [dict(zip(d["columns"], r)) for r in d["rows"]]


files, fns, enums, impls, traits = (load(n) for n in ("files", "functions", "enum-members", "implementations", "traits"))
MARK = {"service", "types", "interface", "adapter", "use_case"}
concepts = {(f["crate"], f["layer"], f["dir"]) for f in files if f["name"] in MARK}


def concept(crate, layer, d):
    p = d.split("/") if d else []
    for n in range(len(p), 0, -1):
        k = (crate, layer, "/".join(p[:n]))
        if k in concepts:
            return f"{crate}:{layer}/{k[2]}"
    return f"{crate}:{layer}/{d}"


def snake(s):
    return re.sub(r"(?<!^)(?=[A-Z])", "_", s).lower()


# ports: workspace traits with two or more impls
tnames = {t["name"] for t in traits}
timpl = C.defaultdict(list)
for i in impls:
    base = i["target"].split("::")[-1]
    if base in tnames:
        timpl[base].append(i["file"])
ports = {t: v for t, v in timpl.items() if len(v) > 1}


def enum_variants(path, enum):
    rows = sorted((e for e in enums if e["file"] == path), key=lambda e: e["span"]["start"])
    out, cur = [], None
    for e in rows:
        if e["kind"] == "enum":
            cur = e["name"]
        elif cur == enum:
            out.append(e["name"])
    return out


def grep_variants(path, start_pat, stop_pat):
    # grep: macro-declared kind lists are invisible to `defines`
    text = open(f"{CRATES}/{path}").read()
    block = text[re.search(start_pat, text).end():]
    block = block[: re.search(stop_pat, block).start()]
    return re.findall(r"^\s*([A-Z][A-Za-z]*),?\s*$", block, re.M)


def candidates(kind, crates, layers=None, exact=False):
    key = snake(kind)
    hits = C.defaultdict(set)
    for f in fns:
        if f["crate"] not in crates or (layers and f["layer"] not in layers):
            continue
        n = f["name"]
        if n == key or (not exact and (n.startswith(key + "_") or n.endswith("_" + key) or ("_" + key + "_") in n)):
            hits[concept(f["crate"], f["layer"], f["dir"])].add(f["file"])
    return hits


def port_of(filesets):
    allf = set().union(*filesets) if filesets else set()
    found = sorted({t for t, v in ports.items() if allf & set(v)})
    return ", ".join(found)


REG = []


def registry(title, source, kinds, crates, layers=None, exact=False):
    REG.append((title, source, kinds, crates, layers, exact))


op_kinds = grep_variants("values/src/domain/query_operators/types.rs", r"operation_kinds! \{", r"\n\}")
registry("Operator kinds (OperationKind, grep: macro list)", "values/query_operators", op_kinds, {"operators", "execution", "execution_domain", "execution_store"}, {"domain", "application"})
dk = grep_variants("derived/src/domain/request/types.rs", r"kinds!\(", r"\);")
registry("Derived kinds (DerivedKind::ALL, grep: macro list)", "derived/contract", dk, {"derived", "execution", "execution_domain", "execution_store"})
members = sorted({f["dir"].split("/")[3] for f in files if f["crate"] == "relations" and f["dir"].startswith("relations/collections/relations/") and len(f["dir"].split("/")) > 3})
registry("Relation members (relations catalog, RyoQL: member directories)", "relations/collections/relations", [m.title().replace("_", "") for m in members], {"language", "execution", "execution_domain", "execution_store", "core"}, None)
walk = []
for en, path in (("WalkRoute", "execution_domain/src/domain/shared_walk/types/walk_route/value.rs"), ("WalkBarrier", "execution_domain/src/domain/shared_walk/types/walk_route/value.rs"),
                 ("Route", "execution_domain/src/domain/executable_program/types.rs"), ("BuildRoute", "execution_store/src/application/file_dag/route_counts/types.rs")):
    walk += [v for v in enum_variants("crates/" + path.split("/", 1)[0] + "/" + path.split("/", 1)[1], en)]
registry("Walk and build routes (WalkRoute, WalkBarrier, Route, BuildRoute; RyoQL enum members)", "execution", walk, {"execution", "execution_domain", "execution_store"})

lines = ["## Implementations per kind", "",
         "Kinds come from RyoQL rows; the two macro-declared lists are read by grep. An implementation is a concept directory declaring a free function whose name carries the kind's name (a name match, not a call graph). A kind is behind a port when its implementing files hold `impl <Trait> for` for a workspace trait with two or more impls.", ""]
flag = []
for title, source, kinds, crates, layers, exact in REG:
    rows = []
    for k in sorted(set(kinds)):
        h = candidates(k, crates, layers, exact)
        n = len(h)
        port = port_of(list(h.values()))
        if n > 1 and not port:
            flag.append((title.split(" (")[0], k, n))
        rows.append((k, n, "; ".join(sorted(h)) or "-", port or "direct"))
    rows.sort(key=lambda r: (-r[1], r[0]))
    lines += [f"### {title}", "", f"{len(set(kinds))} kinds, {sum(1 for r in rows if r[1] > 1)} with more than one implementation.", "",
              "| kind | implementations | locations | port |", "|---|---|---|---|"]
    lines += [f"| {k} | {n} | {loc} | {p} |" for k, n, loc, p in rows if n or title.startswith("Relation") is False]
    lines.append("")
# 5 decoders, finishers, renderers, gates: every free fn by role word, one row each
roles = ("decode", "finish", "render", "fits", "gate", "escape", "carry")
rows = []
for f in fns:
    if f["crate"] in ("execution", "execution_domain", "execution_store", "query_language", "core") and any(f["name"].startswith(r) or f["name"].startswith("call_" + r) for r in roles):
        rows.append((f["name"], concept(f["crate"], f["layer"], f["dir"]), f["file"].split("/")[-1]))
lines += ["### Frontier decoders, finishers, renderers and text gates (functions by role word)", "",
          "Free functions whose name starts with decode, finish, render, fits, gate, escape or carry, in execution, query_language and core. They are listed by name; whether each is behind a port is not decided by a name match.", "",
          "| function | concept | file |", "|---|---|---|"] + [f"| {a} | {b} | {c} |" for a, b, c in sorted(rows)] + [""]
lines += ["### Kinds with more than one implementation and no port", ""]
lines += [f"- {r}: `{k}` ({n} implementations)" for r, k, n in flag] or ["None."]
lines += ["", "Ports found (workspace traits with two or more impls): " + (", ".join(f"`{t}` ({len(v)})" for t, v in sorted(ports.items())) or "none") + "."]
sec = "\n".join(lines) + "\n\n"
p = f"{OUT}/inventory.md"
s = open(p).read()
if "<!-- implementations:begin -->" in s:
    s = re.sub(r"<!-- implementations:begin -->.*<!-- implementations:end -->", lambda m: "<!-- implementations:begin -->\n" + sec + "<!-- implementations:end -->", s, flags=re.S)
else:
    s += "\n<!-- implementations:begin -->\n" + sec + "<!-- implementations:end -->\n"
open(p, "w").write(s)
print(sec[:6000])
