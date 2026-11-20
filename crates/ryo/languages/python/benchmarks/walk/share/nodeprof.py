#!/usr/bin/env python3
"""nodeprof.py <linked pack.json> <stderr of a NAV_INS_PROF=1 NAV_PROF=1 run of it> [top n] [--json out]

Every executed step charged to the compiler's MERGED PLAN NODE it works for, whatever definition, branch or inline copy the
instruction sits in. The executor counts steps per instruction (numbered in load order; this tool replays the numbering and
checks the loader's count and checksum of operation names). An instruction's INNERMOST node is, in this order:
  call     a use of a definition (keyed call, probe, test, `shared`): the node the definition was taken from;
  own      the node of the variable it binds (iter, read, axis, walk, let, fn: the output; test: its right operand, else its
           left; row: its key): the lowering records, per variable, the chain of plan steps being flattened when the variable
           was minted (a renamed copy keeps its original's chain); the innermost is the step that read the relation or made
           the value;
  context  none of its variables has a node (yield, collect, emit, need, a planner's temporary): that of the instruction
           before it in the program, else of the instruction or piece that holds the program;
  none     no node at all: reported as not charged.
Joins, selects, projects and unions mint no variable, so no instruction has one of them as its innermost node: the steps of
an inlined join are on the nodes that read its relations. What the question needs is therefore the second attribution:
  SUB-PLAN  the LARGEST node around the instruction that more than one rule of the set owns: going up the variable's chain
            from the innermost step, the last node still shared (owners only shrink going up). It is the biggest table that
            could hold the instruction's work for several rules. A sub-plan that only filters and projects ONE provider
            relation (selects, projects, extends down to the relation) is BASE: its rows are a subset of facts the file
            already holds once, a step there is the consumer's probe or its walk over them. Any other (a join, union,
            antijoin, closure, axis, count, match) is DERIVED: a step there re-derives shared rows.
A RUN is a stretch of consecutive instructions of one program in the same sub-plan; it is ENTERED each time its first
instruction runs. With the sub-plan as one table per file, a run costs one probe per entry.
The inside of a tree walk (the programs nested in a nearest-ancestor or a closure: per node visited, its reads and tests)
belongs to the walk's sub-plan: a walk's inner variables are one set for every copy of the walk, so their own chains say
nothing about the copy.
Limits, both ways: a comparison is charged to the node (and sub-plan) of its operand, not to the select or join that states
it (toward MORE shared steps). A copy of a step's flattening is re-rooted under the use it is copied for. A definition's
program runs for every caller, so its instructions' sub-plans stop at the definition's own node, and its formals and outputs
know only that node (a definition used under a shared parent counts as a sub-plan of its own: toward FEWER shared steps and
more entries). A run inside a walk is entered once per node the walk visits, where a table of the walk would be probed once
per walk (toward FEWER savings)."""
import collections, json, os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
sys.setrecursionlimit(20000)
import nav
H = os.path.dirname(os.path.abspath(__file__))
args = [a for a in sys.argv[1:] if not a.startswith("--")]
out_json = sys.argv[sys.argv.index("--json") + 1] if "--json" in sys.argv else None
top = int(args[2]) if len(args) > 2 else 10
pack = json.load(open(args[0]))
P = json.load(open(H + "/../export/plans/python.json"))
nodes = P["program"]["nodes"]
ordn = {r["ordinal"]: r["id"] for r in P["rules"]}
ids = [r["id"] for r in pack["rules"]]
idset = set(ids)
old = {}
for line in open(H + "/../../engine/s17.stats.jsonl"):
    x = json.loads(line)
    old[x["node"]] = x
old_rows = lambda n: old.get(n, {}).get("rows", 0)
old_runs = lambda n: old.get(n, {}).get("runs", 0)
origin = {int(k): v for k, v in pack["origin"].items()}
paths = pack["paths"]
meta = pack["shared_meta"]
OUT = {"iter": 4, "read": 4, "axis": 3, "near": 2, "sib": 3, "let": 2, "clos": 2, "nearc": 2, "row": 2}
CALLS = ("or", "or-keyed", "or-probe", "neg", "exists", "neg-memo", "exists-memo", "neg-keyed", "exists-keyed", "neg-probe", "exists-probe")
_own, _top = {}, {}


def owners(n):
    if n not in _own:
        _own[n] = sum(1 for o in nodes[n]["owners"] if ordn.get(o) in idset)
    return _own[n]


def is_shared(n):
    return n is not None and owners(n) > 1


_base = {}


def is_base(n):
    """a node that only filters and projects ONE provider relation (select, project or extend, down to the relation): its
    rows are a subset of facts the file already holds once"""
    if n not in _base:
        x = nodes[n]
        _base[n] = x["op"] in ("select", "project", "extend") and len(x["inputs"]) == 1 and \
            ("relation" in x["inputs"][0] or ("node" in x["inputs"][0] and is_base(x["inputs"][0]["node"])))
    return _base[n]


STOP = [None]


def top_shared(pid):
    """the largest shared node around a variable: up its chain from the innermost step while the node is shared, and not
    past the node of the definition or made table whose program the instruction is in (`STOP`: that program runs for
    every caller, so what lies above it in the chain is one caller's)"""
    k = (pid, STOP[0])
    if k not in _top:
        t, p = None, pid
        while p != -1:
            parent, n = paths[p]
            if not is_shared(n):
                break
            t, p = n, parent
            if n == STOP[0]:
                break
        _top[k] = t
    return _top[k]


def var_path(t):
    return origin.get(t) if isinstance(t, int) and not isinstance(t, bool) else None


def tag(ins, ctx):
    """(innermost node, how, sub-plan)"""
    def own(t):
        p = var_path(t)
        return None if p is None else (paths[p][1], "own", top_shared(p))
    op = ins[0]
    if op == "shared" or op in CALLS:
        e = None
        if op == "shared":
            e = ins[1]
        else:
            subs = ins[nav.SUBS_AT[op][0]]
            if subs and subs[0] and subs[0][0][0] == "shared":
                e = subs[0][0][1]
        if e is not None:
            m = meta[e]
            n = m[4] if len(m) > 4 else None
            if n is not None:
                return n, "call", (n if is_shared(n) else None)
        if op.startswith("or"):
            for t in ins[1]:
                if own(t):
                    return own(t)
        return ctx[0], "context", ctx[1]
    if op in OUT:
        got = own(ins[OUT[op]]) if len(ins) > OUT[op] else None
        if got is None and op == "read":
            got = own(ins[3])
        return got or (ctx[0], "context", ctx[1])
    if op == "test":
        return own(ins[3]) or own(ins[2]) or (ctx[0], "context", ctx[1])
    if op == "fn":
        for t in list(ins[3]) + list(ins[2]):
            if own(t):
                return own(t)
    return ctx[0], "context", ctx[1]


SHOW = os.environ.get("NODEPROF_SUB")   # (diagnostic) a sub-plan whose largest instructions are printed to stderr
shown = {}
held_by = None
runs = []          # [head instruction, sub-plan, instructions]
seq = [0]
checksum = [0xcbf29ce484222325]
tags = []          # per instruction number: (innermost node, how, piece, sub-plan, run)


def walk(p, ctx, piece, run=None, forced=None):
    """`ctx`: (node, sub-plan) the program is entered with; `run`: the run it is entered in"""
    global held_by
    for ins in p:
        i = seq[0]
        seq[0] += 1
        for b in (ins[0] + "\n").encode():
            checksum[0] = ((checksum[0] ^ b) * 0x100000001b3) & 0xFFFFFFFFFFFFFFFF
        n, how, sub = tag(ins, ctx)
        if forced is not None:
            sub = forced[0]  # inside a walk: the walk's sub-plan (its inner variables are one set for every copy of it)
        if sub is None:
            run = None
        else:
            if run is None or runs[run][1] != sub:
                run = len(runs)
                runs.append([i, sub, []])
            runs[run][2].append(i)
        tags.append((n, how if n is not None else "none", piece, sub, run))
        if SHOW:
            shown[i] = (ins[0], json.dumps([x for k, x in enumerate(ins) if k not in nav.SUBS_AT.get(ins[0], ())])[:150], held_by)
        if how != "context" and n is not None:
            ctx = (n, sub)
        for k in nav.SUBS_AT.get(ins[0], ()):
            for q in ins[k]:
                saved, held_by = held_by, ins[0]
                walk(q, (n, sub), piece, run, (sub,) if ins[0] in ("near", "nearc", "clos") else forced)
                held_by = saved


for m in pack["made"]:
    n = STOP[0] = m.get("plan_node")
    for q in m["programs"] + m.get("right_programs", []):
        walk(q, (n, n if is_shared(n) else None), ("made", m["name"]))
for e, q in enumerate(pack["shared"]):
    n = STOP[0] = meta[e][4] if len(meta[e]) > 4 else None
    # (a shared entry is entered by its caller: a run that reaches its end in the caller does not continue into it; the
    # entry's first instruction in a sub-plan starts a run, entered once per evaluation of the entry)
    walk(q, (n, n if is_shared(n) else None), ("entry", e))
STOP[0] = None
for r in pack["rules"]:
    for alts in r["nav"]["programs"]:
        for a in alts:
            walk(a["prog"], (None, None), ("rule", r["id"]))
    for a in r["nav"].get("aux", []):
        for q in a["programs"]:
            walk(q, (None, None), ("rule", r["id"]))
steps = collections.Counter()
files = collections.Counter()
kp = collections.defaultdict(lambda: [0] * 8)
loaded = None
total_run = 0
for line in open(args[1]):
    if line.startswith('{"navload"'):
        loaded = json.loads(line)["navload"]
    elif line.startswith('{"navins"'):
        for i, s, f in json.loads(line)["navins"]:
            steps[i] += s
            files[i] += f
    elif line.startswith('{"navkprof"'):
        for row in json.loads(line)["navkprof"]:
            for k in range(8):
                kp[row[0]][k] += row[1 + k]
    elif line.startswith('{"nav"'):
        total_run += json.loads(line)["nav"]["steps"]
assert loaded and loaded["instructions"] == seq[0] and loaded["checksum"] == checksum[0], (loaded, seq[0], checksum[0])
T = sum(steps.values())
if SHOW:
    for i in sorted((i for i in steps if tags[i][3] == int(SHOW)), key=lambda i: -steps[i])[:14]:
        print(f"{steps[i]:,}", "innermost", tags[i][0], tags[i][1], tags[i][2], "inside a", shown[i][2], shown[i][1], file=sys.stderr)
# ---- 1. by innermost node
by_node = collections.Counter()
copies = collections.Counter()
how_steps = collections.Counter()
for i, s in steps.items():
    n, how, piece, sub, run = tags[i]
    how_steps[how] += s
    if n is not None:
        by_node[n] += s
        copies[n] += 1
shared_nodes = {n for n in by_node if is_shared(n)}
S_shared = sum(by_node[n] for n in shared_nodes)
S_single = sum(by_node.values()) - S_shared
# ---- 2. by sub-plan (the largest shared node around the instruction)
by_sub = collections.Counter()
sub_entries = collections.Counter()
sub_runs = collections.Counter()
sub_files = collections.Counter()
sub_rules = collections.defaultdict(set)
for head, sub, ins in runs:
    s = sum(steps.get(i, 0) for i in ins)
    if s:
        by_sub[sub] += s
        sub_entries[sub] += steps.get(head, 0)
        sub_runs[sub] += 1
        sub_files[sub] = max(sub_files[sub], max(files.get(i, 0) for i in ins))
        piece = tags[head][2]
        sub_rules[sub].add(piece[1] if piece[0] == "rule" else "(a shared entry or made table)")
base_subs = {t for t in by_sub if is_base(t)}
derived_subs = set(by_sub) - base_subs
S_base = sum(by_sub[t] for t in base_subs)
S_derived = sum(by_sub[t] for t in derived_subs)
E_derived = sum(sub_entries[t] for t in derived_subs)
S_own = T - S_base - S_derived


def below(roots):
    """every node of the sub-plans (each once)"""
    seen, stack = set(), list(roots)
    while stack:
        x = stack.pop()
        if x in seen:
            continue
        seen.add(x)
        stack.extend(i["node"] for i in nodes[x]["inputs"] if "node" in i)
    return seen


under = below(derived_subs)
floor = sum(old_rows(n) for n in under)
calls = collections.defaultdict(lambda: [0, 0, 0])
for e, v in kp.items():
    n = meta[e][4] if len(meta[e]) > 4 else None
    if n is not None:
        calls[n][0] += v[0]; calls[n][1] += v[1]; calls[n][2] += v[3] + v[4]
res = dict(
    rules=len(ids), steps_counted=T, steps_of_the_run=total_run, charged_by=dict(how_steps),
    not_charged=how_steps.get("none", 0), not_charged_share=round(how_steps.get("none", 0) / max(T, 1), 4),
    by_innermost_node=dict(
        nodes_with_steps=len(by_node), shared_nodes_with_steps=len(shared_nodes), steps_on_shared_nodes=S_shared,
        steps_on_single_rule_nodes=S_single, share_on_shared_nodes=round(S_shared / max(T, 1), 4),
        of_which_on_base_nodes=sum(by_node[n] for n in shared_nodes if is_base(n)),
        todays_engine_rows_of_those_shared_nodes=sum(old_rows(n) for n in shared_nodes)),
    by_sub_plan=dict(
        steps_in_no_shared_sub_plan=S_own, steps_in_base_sub_plans=S_base, steps_in_derived_sub_plans=S_derived,
        share_in_derived_sub_plans=round(S_derived / max(T, 1), 4), share_in_base_sub_plans=round(S_base / max(T, 1), 4),
        derived_sub_plans_with_steps=len(derived_subs), runs_in_them=sum(sub_runs[t] for t in derived_subs),
        times_those_runs_are_entered=E_derived,
        todays_engine_runs_of_those_sub_plans=sum(old_runs(t) for t in derived_subs),
        entries_per_todays_run=round(E_derived / max(sum(old_runs(t) for t in derived_subs), 1), 1),
        nodes_under_them=len(under), todays_engine_rows_of_those_nodes=floor,
        keyed_calls_into_definitions_of_shared_nodes=sum(v[0] for n, v in calls.items() if is_shared(n)),
        distinct_keys_evaluated_there=sum(v[1] for n, v in calls.items() if is_shared(n)),
        table_attempts_there=sum(v[2] for n, v in calls.items() if is_shared(n))),
    bound=dict(
        steps_now=T, steps_if_every_derived_shared_sub_plan_were_one_table_per_file=T - S_derived + E_derived + floor,
        saving=S_derived - E_derived - floor, saving_share=round((S_derived - E_derived - floor) / max(T, 1), 4),
        ratio=round(T / max(T - S_derived + E_derived + floor, 1), 2),
        the_same_with_probes_free=T - S_derived + floor, with_probes_free_saving_share=round((S_derived - floor) / max(T, 1), 4),
        the_same_with_base_reads_free_too=T - S_derived - S_base + floor,
        with_base_reads_free_too_saving_share=round((S_derived + S_base - floor) / max(T, 1), 4)),
)


def reads(n, depth=3):
    out, stack = set(), [(n, 0)]
    while stack:
        x, d = stack.pop()
        for i in nodes[x]["inputs"]:
            if "relation" in i:
                out.add(i["relation"])
            elif "node" in i and d < depth:
                stack.append((i["node"], d + 1))
    return sorted(out)


def own_ids(n):
    return sorted(ordn[o] for o in nodes[n]["owners"] if ordn.get(o) in idset)


res["largest_derived_sub_plans"] = [dict(
    node=t, op=nodes[t]["op"], steps=by_sub[t], entered=sub_entries[t], runs=sub_runs[t], rules_owning=len(own_ids(t)), rules=own_ids(t)[:8],
    held_in=sorted(sub_rules[t])[:8], files_at_least=sub_files[t], todays_runs=old_runs(t), todays_rows=old_rows(t),
    nodes_under=len(below([t])), todays_rows_under=sum(old_rows(n) for n in below([t])),
    keyed_calls=calls[t][0], distinct_keys=calls[t][1], reads_within_3_inputs=reads(t)) for t in sorted(derived_subs, key=lambda t: -by_sub[t])[:top]]
res["largest_base_sub_plans"] = [dict(node=t, op=nodes[t]["op"], steps=by_sub[t], entered=sub_entries[t], rules_owning=len(own_ids(t)),
                                      todays_rows=old_rows(t), reads=reads(t, 0)) for t in sorted(base_subs, key=lambda t: -by_sub[t])[:top]]
res["largest_shared_innermost_nodes"] = [dict(node=n, op=nodes[n]["op"], base=is_base(n), steps=by_node[n], rules_owning=len(own_ids(n)),
                                              executed_copies=copies[n], todays_runs=old_runs(n), todays_rows=old_rows(n), reads=reads(n, 0))
                                         for n in sorted(shared_nodes, key=lambda n: -by_node[n])[:top]]
res["largest_single_rule_innermost_nodes"] = [dict(node=n, op=nodes[n]["op"], steps=by_node[n], rule=own_ids(n)[:1], todays_rows=old_rows(n))
                                              for n in sorted(set(by_node) - shared_nodes, key=lambda n: -by_node[n])[:top]]
print(json.dumps({k: v for k, v in res.items() if not k.startswith("largest")}, indent=1))
print("\n| derived shared sub-plan (node) | op | steps | times entered | runs (copies) | keyed calls / distinct keys | rules owning it | today's engine: runs, rows of the node, rows of all "
      "nodes under it | reads (within 3 inputs) | held in |")
print("|---|---|---:|---:|---:|---:|---:|---|---|---|")
for x in res["largest_derived_sub_plans"]:
    print(f"| {x['node']} | {x['op']} | {x['steps']:,} | {x['entered']:,} | {x['runs']} | {x['keyed_calls']:,} / {x['distinct_keys']:,} | {x['rules_owning']}: {', '.join(x['rules'])}"
          f"{' ..' if x['rules_owning'] > 8 else ''} | {x['todays_runs']:,}, {x['todays_rows']:,}, {x['todays_rows_under']:,} ({x['nodes_under']} nodes) | "
          f"{', '.join(x['reads_within_3_inputs'][:5])} | {', '.join(x['held_in'][:5])} |")
print("\n| base shared sub-plan (node) | op | steps | times entered | rules owning it | today's rows | relation |")
print("|---|---|---:|---:|---:|---:|---|")
for x in res["largest_base_sub_plans"]:
    print(f"| {x['node']} | {x['op']} | {x['steps']:,} | {x['entered']:,} | {x['rules_owning']} | {x['todays_rows']:,} | {', '.join(x['reads'])} |")
print("\n| shared innermost node | op | steps | rules owning it | executed copies | today's engine: runs, rows | relation |")
print("|---|---|---:|---:|---:|---|---|")
for x in res["largest_shared_innermost_nodes"]:
    print(f"| {x['node']} | {x['op']}{' (base)' if x['base'] else ''} | {x['steps']:,} | {x['rules_owning']} | {x['executed_copies']} | {x['todays_runs']:,}, {x['todays_rows']:,} | {', '.join(x['reads'])} |")
print("\nlargest innermost nodes ONE rule owns: " + "; ".join(f"{x['node']} {x['op']} {x['steps']:,} {x['rule'][0] if x['rule'] else '?'} (today's rows {x['todays_rows']:,})" for x in res["largest_single_rule_innermost_nodes"]))
if out_json:
    json.dump(res, open(out_json, "w"), indent=1)
