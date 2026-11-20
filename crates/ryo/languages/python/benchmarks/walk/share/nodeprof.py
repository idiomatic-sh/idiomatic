#!/usr/bin/env python3
"""nodeprof.py <linked pack.json> <stderr of a NAV_INS_PROF=1 NAV_PROF=1 run of it> [top n] [--json out]

Every executed step charged to the compiler's MERGED PLAN NODE it works for, whatever definition, branch or inline copy the
instruction sits in. The executor counts steps per instruction (numbered in load order; this tool replays the numbering and
checks the loader's count and checksum of operation names). An instruction's node is, in this order:
  call     a use of a definition (keyed call, probe, test, `shared`): the node the definition was taken from;
  own      the node of the variable it binds (iter, read, axis, walk, let, fn: the output; test: its right operand, else its
           left; row: its key): the lowering records, per variable, the plan step being flattened when the variable was minted
           (a renamed copy keeps its original's node), i.e. the innermost step that read the relation or made the value;
  context  none of its variables has a node (yield, collect, emit, need, a planner's temporary): the node of the instruction
           before it in the program, else of the instruction or piece that holds the program;
  none     no node at all (the head of a rule's own program before any variable with a node): reported as not charged.
A comparison is charged to the node that produced its operand, not to the select or join that states it (filters carry no
step of their own in the lowering): steps of a rule's own select over a shared node's rows count as the shared node's. That
errs toward MORE shared steps, so the bound below is an upper bound in that respect too.
A node is SHARED when more than one rule of the pack file's rule set owns it in the compiler's program (export `owners`)."""
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
origin = {int(k): v for k, v in pack["origin"].items()}
meta = pack["shared_meta"]
OUT = {"iter": 4, "read": 4, "axis": 3, "near": 2, "sib": 3, "let": 2, "clos": 2, "nearc": 2, "row": 2}
CALLS = ("or", "or-keyed", "or-probe", "neg", "exists", "neg-memo", "exists-memo", "neg-keyed", "exists-keyed", "neg-probe", "exists-probe")
seq = [0]
checksum = [0xcbf29ce484222325]
tags = []          # per instruction number: (node, how, piece)
ops = []


def var_node(t):
    return origin.get(t) if isinstance(t, int) and not isinstance(t, bool) else None


def tag(ins, ctx):
    op = ins[0]
    if op == "shared":
        m = meta[ins[1]]
        n = m[4] if len(m) > 4 else None
        return (n, "call") if n is not None else (ctx, "context")
    if op in CALLS:
        subs = ins[nav.SUBS_AT[op][0]]
        if subs and subs[0] and subs[0][0][0] == "shared":
            m = meta[subs[0][0][1]]
            if len(m) > 4 and m[4] is not None:
                return m[4], "call"
        if op.startswith("or"):
            for t in ins[1]:
                if var_node(t) is not None:
                    return var_node(t), "own"
        return ctx, "context"
    if op in OUT:
        n = var_node(ins[OUT[op]]) if len(ins) > OUT[op] else None
        if n is None and op in ("iter", "read"):
            n = var_node(ins[3]) if op == "read" else None
        return (n, "own") if n is not None else (ctx, "context")
    if op == "test":
        for t in (ins[3], ins[2]):
            if var_node(t) is not None:
                return var_node(t), "own"
        return ctx, "context"
    if op == "fn":
        for t in list(ins[3]) + list(ins[2]):
            if var_node(t) is not None:
                return var_node(t), "own"
    return ctx, "context"


def walk(p, ctx, piece):
    for ins in p:
        i = seq[0]
        seq[0] += 1
        for b in (ins[0] + "\n").encode():
            checksum[0] = ((checksum[0] ^ b) * 0x100000001b3) & 0xFFFFFFFFFFFFFFFF
        n, how = tag(ins, ctx)
        tags.append((n, how if n is not None else "none", piece))
        ops.append(ins[0])
        if how != "context" and n is not None:
            ctx = n
        for k in nav.SUBS_AT.get(ins[0], ()):
            for q in ins[k]:
                walk(q, n, piece)


for m in pack["made"]:
    for q in m["programs"] + m.get("right_programs", []):
        walk(q, m.get("node"), ("made", m["name"]))
for e, q in enumerate(pack["shared"]):
    walk(q, meta[e][4] if len(meta[e]) > 4 else None, ("entry", e))
for r in pack["rules"]:
    for alts in r["nav"]["programs"]:
        for a in alts:
            walk(a["prog"], None, ("rule", r["id"]))
    for a in r["nav"].get("aux", []):
        for q in a["programs"]:
            walk(q, None, ("rule", r["id"]))
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
owners = lambda n: sum(1 for o in nodes[n]["owners"] if ordn.get(o) in idset)
by_node = collections.Counter()
node_files = collections.Counter()
copies = collections.Counter()
node_pieces = collections.defaultdict(set)
how_steps = collections.Counter()
for i, s in steps.items():
    n, how, piece = tags[i]
    how_steps[how] += s
    if n is None:
        continue
    by_node[n] += s
    node_files[n] = max(node_files[n], files[i])
    copies[n] += 1
    node_pieces[n].add(piece if piece[0] == "rule" else piece[0])
T = sum(steps.values())
charged = sum(by_node.values())
shared_nodes = {n for n in by_node if owners(n) > 1}
S_shared = sum(by_node[n] for n in shared_nodes)
S_single = charged - S_shared
# keyed calls and distinct keys of the definitions taken from a shared node
calls = collections.defaultdict(lambda: [0, 0, 0])
for e, v in kp.items():
    n = meta[e][4] if len(meta[e]) > 4 else None
    if n is not None:
        calls[n][0] += v[0]; calls[n][1] += v[1]; calls[n][2] += v[3] + v[4]
old_rows = lambda n: old.get(n, {}).get("rows", 0)
old_runs = lambda n: old.get(n, {}).get("runs", 0)
floor_shared = sum(min(old_rows(n), by_node[n]) for n in shared_nodes)
res = dict(
    rules=len(ids), steps_counted=T, steps_of_the_run=total_run, charged_by=dict(how_steps), not_charged=how_steps.get("none", 0),
    not_charged_share=round(how_steps.get("none", 0) / max(T, 1), 4),
    nodes_with_steps=len(by_node), shared_nodes_with_steps=len(shared_nodes),
    steps_on_shared_nodes=S_shared, steps_on_single_rule_nodes=S_single,
    share_of_steps_on_shared_nodes=round(S_shared / max(T, 1), 4),
    executed_instructions_on_shared_nodes=sum(copies[n] for n in shared_nodes),
    todays_engine_rows_of_those_shared_nodes=sum(old_rows(n) for n in shared_nodes),
    todays_engine_runs_of_those_shared_nodes=sum(old_runs(n) for n in shared_nodes),
    steps_per_todays_row_on_shared_nodes=round(S_shared / max(sum(old_rows(n) for n in shared_nodes), 1), 1),
    keyed_calls_into_definitions_of_shared_nodes=sum(v[0] for n, v in calls.items() if n in shared_nodes),
    distinct_keys_evaluated_there=sum(v[1] for n, v in calls.items() if n in shared_nodes),
    table_attempts_there=sum(v[2] for n, v in calls.items() if n in shared_nodes),
    bound_steps_if_every_shared_node_were_one_table_per_file=T - S_shared + floor_shared,
    bound_saving=S_shared - floor_shared, bound_saving_share=round((S_shared - floor_shared) / max(T, 1), 4),
)


def reads(n, depth=3):
    out = set()
    stack = [(n, 0)]
    while stack:
        x, d = stack.pop()
        for i in nodes[x]["inputs"]:
            if "relation" in i:
                out.add(i["relation"])
            elif "node" in i and d < depth:
                stack.append((i["node"], d + 1))
    return sorted(out)


res["largest_shared_nodes"] = []
for n in sorted(shared_nodes, key=lambda n: -by_node[n])[:top]:
    own = sorted(ordn[o] for o in nodes[n]["owners"] if ordn.get(o) in idset)
    res["largest_shared_nodes"].append(dict(
        node=n, op=nodes[n]["op"], steps=by_node[n], rules_owning=len(own), rules=own[:8], executed_copies=copies[n],
        in_rule_programs=sorted(p[1] for p in node_pieces[n] if isinstance(p, tuple))[:8], in_shared_entries="entry" in node_pieces[n],
        in_made_tables="made" in node_pieces[n], files_at_least=node_files[n], todays_rows=old_rows(n), todays_runs=old_runs(n),
        keyed_calls=calls[n][0], distinct_keys=calls[n][1], reads_within_3_inputs=reads(n)))
res["largest_single_rule_nodes"] = [dict(node=n, op=nodes[n]["op"], steps=by_node[n], rule=[ordn[o] for o in nodes[n]["owners"] if ordn.get(o) in idset][:1],
                                         todays_rows=old_rows(n)) for n in sorted(set(by_node) - shared_nodes, key=lambda n: -by_node[n])[:top]]
print(json.dumps({k: v for k, v in res.items() if not k.startswith("largest")}, indent=1))
print("| node | op | steps | rules owning it | executed copies | keyed calls / distinct keys | today's engine: runs, rows | reads (within 3 inputs) | rules |")
print("|---|---|---:|---:|---:|---:|---|---|---|")
for x in res["largest_shared_nodes"]:
    print(f"| {x['node']} | {x['op']} | {x['steps']:,} | {x['rules_owning']} | {x['executed_copies']} | {x['keyed_calls']:,} / {x['distinct_keys']:,} | {x['todays_runs']:,}, {x['todays_rows']:,} | {', '.join(x['reads_within_3_inputs'][:5])} | {', '.join(x['rules'])}{' ..' if x['rules_owning'] > 8 else ''} |")
print("\nlargest nodes ONE rule owns:")
for x in res["largest_single_rule_nodes"]:
    print(f"  {x['node']} {x['op']} {x['steps']:,} {x['rule']} today's rows {x['todays_rows']:,}")
if out_json:
    json.dump(res, open(out_json, "w"), indent=1)
