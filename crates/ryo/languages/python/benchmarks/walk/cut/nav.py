#!/usr/bin/env python3
"""nav.py <plans.json> <schema.json> <freq.json> <rules file> <out program.json> <out blocked.json>

General lowering (nothing keyed on a rule id, a rule's text or a relation's name beyond the provider contract's classes) of each
rule's logical plan into per-candidate NAVIGATION programs over the node table:

  core  = the left input of the emit antijoin (the rule's candidates); its plan (select/project/join/union/extend/antijoin/
          difference/intersection/axis) is flattened into a union of conjunctive branches (atoms over relations, filters,
          negated or existential sub-branches, axis walks). Equalities unify variables; a union of branches that differ in one
          constant becomes one branch with an in-set test (the alpha-network literal set).
  plan  = each branch becomes one straight-line program: an anchor (a kind bucket, a literal bucket of a relation column, or a
          scan), then field reads keyed on bound ids (parent, child by field, kind, span, text code), tests as soon as their
          operands are bound, sub-programs for absence/existence with early exit; no step writes a row; Emit writes the core's row.
  tail  = the emit step and everything it reads except the core (the pack's suppression fragment), in the walk interpreter's
          format, with the core's root step replaced by `injected` (its rows are the navigation output).
"""
import collections, itertools, json, os, signal, sys, time


def _alarm(*_):
    raise TimeoutError()


signal.signal(signal.SIGALRM, _alarm)

TREE = {
    # relation: (key column = the row's id, columns)
    "semantic-syntax": 0, "syntax-tree": 0, "semantic-child": 1, "syntax-child": 1, "source-location": 0, "identifier": 0,
}
# tree relations whose rows exist for every node id (functional read always succeeds for a valid id)
TOTAL = {"semantic-syntax", "syntax-tree", "source-location"}
KIND_COL = {"semantic-syntax": 1, "syntax-tree": 1}
TEXT_COL = {"semantic-syntax": 2, "syntax-tree": 2}
FIELD_COL = {"semantic-child": 3, "syntax-child": 3}
PARENT_COL = {"semantic-child": 0, "syntax-child": 0}
TAIL_RELATIONS = {"semantic-syntax", "semantic-child", "source-location", "identifier", "embedded-expression", "syntax-tree",
                  "comment", "indent-region"}
NOT_LOWERED = set()  # navwide: comment, indent-region and embedded-expression come through the provider readers


BRANCH_CAP = int(os.environ.get("NAV_BRANCH_CAP", "256"))
UNION_CAP = int(os.environ.get("NAV_UNION_CAP", "32"))
# lower: the bounds the lowering guarantees by construction (see body/cut/too_big/mask_of)
ITEM_CAP = int(os.environ.get("NAV_ITEM_CAP", "40"))      # items per branch before a side becomes a definition's use
MASK_CAP = int(os.environ.get("NAV_MASK_CAP", "4"))       # binding patterns planned per definition (then uncorrelated)
BODY_CAP = int(os.environ.get("NAV_BODY_CAP", "20000"))   # definition programs per rule
KEYED_MEMO = os.environ.get("NAV_KEYED_MEMO", "1") == "1"  # calls and tests memoised per file by their bound args (executor arms)
# lower: the item bound on every input and joined branch (on, as measured). Off (bound on copies only) was tried: the selective
# anchor stays in the branch, but some single-read regions then plan into runaway programs (PLR0124 et al. did not finish)
JOIN_ITEM_CUT = os.environ.get("NAV_JOIN_ITEM_CUT", "1") == "1"
BUCKET_X = float(os.environ.get("NAV_BUCKET_X", "10"))  # a literal bucket iterated inside a bound context costs 10x its size (+10)
# cut: each mechanism of this round has a switch, so its effect can be measured alone
CALL_COST = os.environ.get("NAV_CALL_COST", "1") == "1"        # a use of a definition is costed from the definition's own plan, and may anchor
PROBE = os.environ.get("NAV_PROBE", "1") == "1"                # a use may enter from the definition's side (its rows once per file, indexed)
INLINE_LIGHT = os.environ.get("NAV_INLINE_LIGHT", "1") == "1"  # a definition whose sub-program only reads and tests is inlined at the use
ANCHOR_CUT = os.environ.get("NAV_ANCHOR_CUT", "1") == "1"      # a forced cut takes the side WITHOUT the cheapest anchor
MADE_COST = os.environ.get("NAV_MADE_COST", "1") == "1"        # a made table is sized from its input's plan and costs its build once
TOP_INLINE = os.environ.get("NAV_TOP_INLINE", "1") == "1"      # the core's branches are top programs up to the product bound (no top cut)
REQUIRE = os.environ.get("NAV_REQUIRE", "1") == "1"            # kind buckets a branch needs through its definitions prove it empty per file
KEYED_STEPS = float(os.environ.get("NAV_KEYED_STEPS", "20"))   # steps of a well-keyed call: a call ranks as its rows, or its steps over this


class Blocked(Exception):
    pass


def inputs(p):
    for key in ("from", "left", "right", "edges", "domain"):
        v = p.get(key)
        if isinstance(v, dict):
            yield key, v


def bind(value, params):
    if isinstance(value, dict):
        if set(value) == {"parameter"}:
            return params[value["parameter"]]
        return {k: bind(v, params) for k, v in value.items()}
    if isinstance(value, list):
        return [bind(v, params) for v in value]
    return value


# ------------------------------------------------------------------ terms
# a term is an int (variable) or a constant tuple: ("t", text|None) ("n", int) ("ts", (texts,)) ("ns", (ints,))

def is_var(t):
    return isinstance(t, int)


SUB_LISTS = {"near": (3,), "clos": (7,), "or": (2,), "nearc": (5, 6)}


class Br:
    __slots__ = ("atoms", "filt", "negs", "specials", "out", "fv")

    def __init__(self, atoms=None, filt=None, negs=None, specials=None, out=None):
        self.atoms = atoms or []      # [rel, [terms]]
        self.filt = filt or []        # [test, l, r]
        # lower: [mode 'neg'|'exists', def key, args]: a test against a DEFINITION (a sub-plan lowered once per rule); args
        # holds one term per column of the definition (None = existential), the correlation of this use
        self.negs = negs or []
        self.specials = specials or []  # ['axis', dir, start, reached] | ['near', start, reached, [Br]] | ['call', key, args]
        self.out = out or []
        self.fv = None

    def copy(self):
        """a copy of the top level; sub-branches (inside specials) are shared: they are never changed in place once stored
        (a substitution that reaches one copies it); definitions are never copied (uses carry their args)"""
        return Br([[r, list(ts)] for r, ts in self.atoms], [list(f) for f in self.filt],
                  [[m, k, list(a)] for m, k, a in self.negs],
                  [copy_special(s) for s in self.specials], list(self.out))

    def size(self):
        """items the planner places: top-level atoms, filters, tests and specials, plus the sub-branches a special plans
        inline (a definition's use counts one)"""
        n = len(self.atoms) + len(self.filt) + len(self.negs) + len(self.specials)
        for s in self.specials:
            for k in SUB_LISTS.get(s[0], ()):
                n += sum(x.size() for x in s[k])
        return n


    def deep(self):
        if self.fv is None:
            self.fv = frozenset(deep_vars(self, set()))
        return self.fv


def copy_special(s):
    if s[0] == "fn":
        return ["fn", s[1], list(s[2]), list(s[3]), s[4]]
    if s[0] == "near":
        return ["near", s[1], s[2], list(s[3])]
    if s[0] == "clos":
        return ["clos", s[1], s[2], list(s[3]), s[4], s[5], list(s[6]), list(s[7])]
    if s[0] == "or":
        return ["or", list(s[1]), list(s[2])]
    if s[0] == "nearc":
        return ["nearc", s[1], s[2], s[3], s[4], list(s[5]), list(s[6])]
    if s[0] == "call":
        return ["call", s[1], list(s[2])]
    return list(s)


class Lowerer:
    def __init__(self, schema, freq):
        self.schema = schema
        self.freq = freq
        self.nvar = 0
        self.kind = {}
        self.defs, self.formal, self.body_memo, self.masks, self.shared, self.n_bodies = {}, {}, {}, {}, [], 0
        self.heavy_memo = {}
        # cut: per definition program (key, mask, free): (steps per call, rows per call) from its plan, and its branch entries
        self.body_cost, self.body_branches = {}, {}
        self.mult = 1.0          # calls per file of the sub-program being planned (amortises one-off costs)
        self.last_card = 1.0     # rows the last planned program ends with
        self.top_cost = self.top_card = 0.0
        self.made_cost = {}      # made table -> steps to build it once per file
        self.anchor_memo = {}
        self.require_memo = {}

    def fresh(self, kind):
        v = self.nvar
        self.nvar += 1
        self.kind[v] = kind
        return v

    # ---- substitution
    def subst(self, br, m):
        if not m:
            return br
        def f(t):
            while is_var(t) and t in m:
                t = m[t]
            return t
        br.atoms = [[r, [f(t) for t in ts]] for r, ts in br.atoms]
        br.filt = [[x, f(l), f(r)] for x, l, r in br.filt]
        br.out = [f(t) for t in br.out]
        br.fv = None
        br.negs = [[mode, k, [None if t is None else f(t) for t in a]] for mode, k, a in br.negs]
        sp = []
        for s in br.specials:
            if s[0] == "call":
                sp.append(["call", s[1], [None if t is None else f(t) for t in s[2]]])
            elif s[0] == "near":
                sp.append(["near", f(s[1]), f(s[2]), [self.subst_sub(b, m) for b in s[3]]])
            elif s[0] == "fn":
                sp.append(["fn", s[1], [f(t) for t in s[2]], [f(t) for t in s[3]], s[4]])
            elif s[0] == "clos":
                sp.append(["clos", f(s[1]), f(s[2]), [f(t) for t in s[3]], f(s[4]), f(s[5]), [f(t) for t in s[6]],
                           [self.subst_sub(b, m) for b in s[7]]])
            elif s[0] == "or":
                sp.append(["or", [f(t) for t in s[1]], [self.subst_sub(b, m) for b in s[2]]])
            elif s[0] == "nearc":
                sp.append(["nearc", f(s[1]), f(s[2]), f(s[3]), f(s[4]), [self.subst_sub(b, m) for b in s[5]],
                           [self.subst_sub(b, m) for b in s[6]]])
            else:
                sp.append([s[0], s[1], f(s[2]), f(s[3])])
        br.specials = sp
        return br

    def subst_sub(self, b, m):
        """a stored sub-branch under a substitution: itself when no substituted variable occurs in it, else a copy"""
        if not any(v in m for v in b.deep()) if len(m) > 8 else not any(k in b.deep() for k in m):
            return b
        return self.subst(b.copy(), m)

    def flagged(self, a, b):
        """a flag cell compared with the text "true" or "false" compares with the flag's number (navwide)"""
        if is_var(a) and not is_var(b) and b[0] == "t" and b[1] in ("true", "false") and self.kind.get(a) == "flag":
            return ("n", 1 if b[1] == "true" else 0)
        return b

    def unify(self, br, a, b):
        """a = b inside br; returns br or None when the branch can never hold."""
        b = self.flagged(a, b)
        a = self.flagged(b, a)
        if a == b:
            return br
        if is_var(a) and is_var(b):
            return self.subst(br, {b: a})
        if is_var(b):
            a, b = b, a
        if is_var(a):
            if b[0] in ("t", "n"):
                br.filt.append(["=", a, b])
                return br
            raise Blocked("unify-set")
        return br if a == b else None

    # ---- flatten
    def flat(self, s):
        st = self.steps[s]
        op, p = st["op"], st["params"]
        if st.get("scope") == "cross-file":
            raise Blocked("cross-file")
        if op == "select":
            out = []
            for b in self.input(p["from"]):
                b = self.select(b, p["comparisons"])
                if b is None:
                    continue
                if p["kept"]:
                    b.out = [b.out[c] for c in p["kept"]]
                out.append(b)
            return out
        if op == "project":
            out = []
            for b in self.input(p["from"]):
                b.out = [b.out[c] for c in p["columns"]]
                out.append(b)
            return out
        if op == "extend":
            out = []
            for b in self.input(p["from"]):
                for v in p["values"]:
                    b.out.append(self.const(v))
                out.append(b)
            return out
        if op == "union":
            bs = self.merge(self.input(p["left"]) + self.input(p["right"]))
            if len(bs) > UNION_CAP:
                # a wide union stays one use of a definition enumerating its rows (lowered once, never copied)
                return [self.cut(bs, ("union", s))]
            return bs
        if op == "closure":
            return self.closure_br(self.input(p["edges"]), p["keys"], p["source"], p["destination"])
        if op == "join":
            right = p["right"]
            view = p.get("view") or []
            rv = (lambda k: view[k]) if view and right.get("relation") in ("syntax-tree", "syntax-child") else (lambda k: k)
            L = self.input(p["left"])
            if p.get("reach"):
                # a join against the transitive closure of the right edge relation: the virtual right row is the closure's
                # (keys.., source, destination), re-ordered by the reach's columns
                rc = p["reach"]
                if view:
                    raise Blocked("join-reach-view")
                R_ = []
                for c in self.closure_br(self.input(right), rc["keys"], rc["source"], rc["destination"]):
                    c.out = [c.out[k] for k in rc["columns"]]
                    R_.append(c)
            else:
                R_ = self.input(right)
            # lower: the product of branches is never expanded past the branch bound: the larger side becomes one use of a
            # definition (its rows enumerated by a sub-program lowered once), until the product fits
            while len(L) * len(R_) > BRANCH_CAP:
                if self.cut_right(L, R_, len):
                    R_ = [self.cut(R_, ("join", s, "right"))]
                else:
                    L = [self.cut(L, ("join", s, "left"))]
            if JOIN_ITEM_CUT and L and R_ and max(x.size() for x in L) + max(x.size() for x in R_) > ITEM_CAP:
                # the joined branch would exceed the item bound: one side becomes a definition's use (cut: the side
                # without the cheapest anchor, see cut_right)
                if self.cut_right(L, R_, lambda bs: max(x.size() for x in bs)):
                    R_ = [self.cut(R_, ("join", s, "right"))]
                else:
                    L = [self.cut(L, ("join", s, "left"))]
            out = []
            for bl in L:
                for br0 in R_:
                    br = br0.copy() if len(L) > 1 else br0
                    br = self.select(br, p["filter"]) if p["filter"] else br
                    if br is None:
                        continue
                    b = Br(bl.atoms + br.atoms, bl.filt + br.filt, bl.negs + br.negs, bl.specials + br.specials, [])
                    lo, ro = list(bl.out), list(br.out)
                    # lower: every key equality goes into one union-find over the joined branch's terms; one rewrite after
                    b.out = lo + ro
                    b = self.unify_all(b, [(lo[lk], ro[rv(rk)]) for lk, rk in zip(p["left_on"], p["right_on"])])
                    if b is None:
                        continue
                    n = len(lo)
                    lo, ro = b.out[:n], b.out[n:]
                    b.out = [lo[c] if o["side"] == "left" else ro[rv(o["column"])] for o in p["output"] for c in [o["column"]]]
                    out.append(b)
            # rename apart when the left had several branches sharing the right's variables
            return [self.rename(b) for b in out] if len(L) > 1 else out
        if op in ("antijoin", "difference", "intersection"):
            # lower: the right side is a DEFINITION lowered once per rule; each left branch carries one use of it whose args
            # are the left's key terms (no copy of the right side, no substitution into it)
            L = self.input(p["left"])
            key = self.def_of(p["right"], ("right", s))
            body = self.defs[key]
            width = len(body[0].out) if body else 0
            out = []
            for bl in L:
                if op == "antijoin":
                    keys = list(zip(p["left_on"], p["right_on"]))
                else:
                    keys = [(i, i) for i in range(len(bl.out))]
                args = [None] * width
                for lk, rk in (keys if body else []):
                    t = bl.out[lk]
                    if not is_var(t) and t[0] == "f":
                        continue
                    if args[rk] is not None and args[rk] != t:
                        raise Blocked("antijoin-repeated-key")
                    args[rk] = t
                live = body and any(self.may_match(x, args) for x in body)
                if op == "intersection":
                    if not live:
                        continue
                    bl.negs.append(["exists", key, args])
                elif live:
                    bl.negs.append(["neg", key, args])
                out.append(bl)
            return out
        if op == "axis":
            if p["parent"] != 0 or p["child"] != 1 or p["output"]:
                raise Blocked("axis-shape")
            derived = p["edges"].get("relation") not in ("semantic-child", "syntax-child")
            d = p["direction"]
            if derived:
                return self.axis_derived(p, d)
            if d not in ("parent", "child", "ancestor", "descendant", "nearest-ancestor", "next-sibling", "previous-sibling", "deep"):
                raise Blocked("axis-" + d)
            dom = p["domain"]
            dview = p.get("domain_view") or []
            dv = (lambda k: dview[k]) if dview and dom.get("relation") in ("syntax-tree",) else (lambda k: k)
            out = []
            for bf in self.input(p["from"]):
                start = bf.out[p["id"]]
                reached = self.fresh("node")
                D = []
                for bd in self.input(dom):
                    if p["domain_filter"]:
                        bd = self.select(bd, p["domain_filter"])
                        if bd is None:
                            continue
                    bd = self.unify_outer(bd, bd.out[dv(p["domain_id"])], reached)
                    if bd is not None:
                        D.append(bd)
                if not D:
                    continue
                if d == "nearest-ancestor":
                    b = bf.copy()
                    b.specials.append(["near", start, reached, D])
                    b.out = bf.out + [reached]
                    out.append(b)
                else:
                    for bd in D:
                        b = Br(bf.atoms + bd.atoms, bf.filt + bd.filt, bf.negs + bd.negs,
                               bf.specials + bd.specials + [["axis", d, start, reached]], bf.out + [reached])
                        out.append(b)
            return out
        if op == "group_count":
            # a made table: per file, the distinct rows of its input (planned as programs of their own), grouped by the keys
            src = self.steps[p["from"]["step"]] if "step" in p["from"] else None
            # navwide: a bag input counts as its distinct rows (a bag projection keeps a column that identifies the row)
            sub = self.input(p["from"])
            name = f"made:{self.rid}:{len(self.made)}"
            progs, mcost, mrows = self.made_plan(sub)
            kinds = []
            for t in sub[0].out if sub else []:
                pass
            kk = [self.kind.get(sub[0].out[k], "symbol") if is_var(sub[0].out[k]) else {"t": "symbol", "n": "number", "f": "file"}.get(sub[0].out[k][0], "symbol") for k in p["keys"]] if sub else []
            cols = [dict(name=f"k{i}", kind=k, key=False) for i, k in enumerate(kk)] + [dict(name="count", kind="number", key=False)]
            self.schema[name] = dict(columns=cols, rows=2873 * 5)
            self.made_size(name, mcost, mrows)
            self.made.append(dict(name=name, programs=progs, keys=list(p["keys"]), distinct=p["distinct"], width=len(sub[0].out) if sub else 0,
                                  kinds=[c["kind"] for c in cols]))
            ts = [("f", 0) if c["kind"] == "file" else self.fresh(c["kind"]) for c in cols]
            return [Br([[name, list(ts)]], out=list(ts))]
        if op == "capture":
            out = []
            for b in self.input(p["from"]):
                o = [self.fresh("symbol") for _ in range(p["pattern"].count("{"))]
                b.specials.append(["fn", "capture", [b.out[p["path"]]], o, {"pattern": p["pattern"]}])
                b.out = b.out + o
                out.append(b)
            return out
        if op == "replace":
            out = []
            for b in self.input(p["from"]):
                o = [self.fresh("symbol")]
                b.specials.append(["fn", "replace", [b.out[p["text"]]], o, {"pattern": p["pattern"], "replacement": p["replacement"]}])
                b.out = b.out + o
                out.append(b)
            return out
        if op == "ordered_match":
            # a made table: per file, the left and right inputs' rows (programs of their own), matched in order per key group
            L = self.input(p["left"])
            R = self.input(p["right"])
            if not L or not R:
                return []
            name = f"made:{self.rid}:{len(self.made)}"
            lprogs, lc, ln = self.made_plan(L)
            rprogs, rc, rn = self.made_plan(R)
            kinds = [self.tkind((L if o["side"] == "left" else R)[0].out[o["column"]]) for o in p["output"]]
            cols = [dict(name=f"c{i}", kind=k, key=False) for i, k in enumerate(kinds)]
            self.schema[name] = dict(columns=cols, rows=2873 * 5)
            self.made_size(name, lc + rc, ln + rn)
            self.made.append(dict(name=name, op="ordered_match", programs=lprogs, right_programs=rprogs,
                                  width=len(L[0].out), right_width=len(R[0].out), left_on=p["left_on"], right_on=p["right_on"],
                                  left_order=p["left_order"], right_order=p["right_order"],
                                  order_number=self.tkind(L[0].out[p["left_order"]]) == "number",
                                  output=[[0 if o["side"] == "left" else 1, o["column"]] for o in p["output"]],
                                  keys=[], distinct=None, kinds=kinds))
            ts = [("f", 0) if k == "file" else self.fresh(k) for k in kinds]
            return [Br([[name, list(ts)]], out=list(ts))]
        if op == "ordered_dfs":
            # a made table: per file, the roots' and the edges' rows (programs of their own), walked depth-first in order
            L = self.input(p["from"])
            E = self.input(p["edges"])
            if not L or not E:
                return []
            name = f"made:{self.rid}:{len(self.made)}"
            lprogs, lc, ln = self.made_plan(L)
            eprogs, ec, en = self.made_plan(E)
            kinds = [self.tkind(L[0].out[k]) for k in p["root_keys"]] + [self.tkind(L[0].out[p["root"]]),
                     self.tkind(E[0].out[p["source"]]), self.tkind(E[0].out[p["destination"]]), self.tkind(E[0].out[p["edge"]]), "number"]
            cols = [dict(name=f"c{i}", kind=k, key=False) for i, k in enumerate(kinds)]
            self.schema[name] = dict(columns=cols, rows=2873 * 5)
            self.made_size(name, lc + ec, ln + en)
            self.made.append(dict(name=name, op="ordered_dfs", programs=lprogs, right_programs=eprogs,
                                  width=len(L[0].out), right_width=len(E[0].out), root=p["root"], source=p["source"],
                                  destination=p["destination"], order=p["order"], edge=p["edge"], root_keys=p["root_keys"],
                                  edge_keys=p["edge_keys"], order_number=self.tkind(E[0].out[p["order"]]) == "number",
                                  keys=[], distinct=None, kinds=kinds))
            ts = [("f", 0) if k == "file" else self.fresh(k) for k in kinds]
            return [Br([[name, list(ts)]], out=list(ts))]
        if op in ("span_between", "span_bounds", "source_matches"):
            out = []
            for b in self.input(p["from"]):
                if op == "span_between":
                    o = [self.fresh("span")]
                    b.specials.append(["fn", op, [b.out[p["start"]], b.out[p["end"]]], o, {}])
                elif op == "span_bounds":
                    o = [self.fresh("number"), self.fresh("number")]
                    b.specials.append(["fn", op, [b.out[p["span"]]], o, {}])
                else:
                    o = [self.fresh("span")]
                    args = [b.out[p["span"]]]
                    if p["base_line"] is not None:
                        o.append(self.fresh("number"))
                        args.append(b.out[p["base_line"]])
                    if p["capture"] is not None:
                        o.append(self.fresh("span"))
                        if p["capture_text"]:
                            o.append(self.fresh("symbol"))
                        if p["capture_line"]:
                            o.append(self.fresh("number"))
                    b.specials.append(["fn", op, args, o, dict(pattern=p["pattern"], capture=p["capture"],
                                                              capture_text=p["capture_text"], capture_line=p["capture_line"],
                                                              base_line=p["base_line"] is not None)])
                b.out = b.out + o
                out.append(b)
            return out
        raise Blocked("op-" + op)

    def or_br(self, bs):
        """a union of branches as one branch: its output columns are fresh variables, enumerated (as a set) by the
        branches' sub-programs"""
        bs = [b for b in bs if b is not None]
        if len(bs) == 1:
            return bs[0]
        outs = []
        for t in bs[0].out:
            outs.append(t if not is_var(t) and t[0] == "f" else self.fresh(self.kind.get(t, "symbol") if is_var(t) else
                                                                           {"t": "symbol", "n": "number"}.get(t[0], "symbol")))
        D = []
        for b in bs:
            b = b.copy()
            ok = True
            for i, outer in enumerate(outs):
                inner = b.out[i]
                if is_var(inner) and inner in outs[:i] and is_var(outer):
                    b.filt.append(["=", outer, inner])
                    continue
                b = self.unify_outer(b, inner, outer)
                if b is None:
                    ok = False
                    break
            if ok:
                D.append(b)
        return Br(out=list(outs), specials=[["or", list(outs), D]])

    def closure_br(self, E, keys, src, dst):
        """closure over edge branches E: one branch whose output is (keys.., a, b) with a `clos` special; edges that are
        exactly the tree's parent-child relation become a descendant axis"""
        if not E:
            return []
        k0 = E[0].out
        x = self.fresh(self.kind.get(k0[src], "node") if is_var(k0[src]) else "node")
        y = self.fresh(self.kind.get(k0[dst], "node") if is_var(k0[dst]) else "node")
        kx = []
        for k in keys:
            t = k0[k]
            kx.append(t if not is_var(t) and t[0] == "f" else self.fresh(self.kind.get(t, "symbol") if is_var(t) else "symbol"))
        D = []
        for be in E:
            be = be.copy()
            ok = True
            done = []
            for col, outer in [(src, x), (dst, y)] + list(zip(keys, kx)):
                inner = be.out[col]
                if is_var(inner) and is_var(outer) and inner in done:
                    # the same variable in two edge columns (an edge from a node to itself): an equality in the branch
                    be.filt.append(["=", outer, inner])
                    continue
                be = self.unify_outer(be, inner, outer)
                if be is not None:
                    done.append(outer)
                if be is None:
                    ok = False
                    break
            if ok:
                D.append(be)
        if not D:
            return []
        a, b = self.fresh(self.kind[x]), self.fresh(self.kind[y])
        kv = [t if not is_var(t) else self.fresh(self.kind[t]) for t in kx]
        out = Br(out=kv + [a, b])
        if len(D) == 1 and not D[0].filt and not D[0].negs and not D[0].specials and len(D[0].atoms) == 1 and \
                all(not is_var(t) for t in kx):
            rel, ts = D[0].atoms[0]
            if rel in PARENT_COL and ts[PARENT_COL[rel]] == x and ts[TREE[rel]] == y and \
                    all((is_var(t) and t not in (x, y)) or t == ("f", 0) for c, t in enumerate(ts) if c not in (PARENT_COL[rel], TREE[rel])):
                out.specials.append(["axis", "descendant", a, b])
                return [out]
        out.specials.append(["clos", a, b, kv, x, y, kx, D])
        return [out]

    def axis_derived(self, p, d):
        """axis over a derived edge relation (navwide): parent/child are one edge, ancestor/descendant its closure,
        nearest-ancestor the first ancestor (breadth-first) the domain admits"""
        if d not in ("parent", "child", "ancestor", "descendant", "nearest-ancestor"):
            raise Blocked("axis-derived-" + d)
        dom = p["domain"]
        dview = p.get("domain_view") or []
        dv = (lambda k: dview[k]) if dview and dom.get("relation") in ("syntax-tree",) else (lambda k: k)
        up = d in ("parent", "ancestor", "nearest-ancestor")
        src, dst = (p["child"], p["parent"]) if up else (p["parent"], p["child"])
        out = []
        for bf in self.input(p["from"]):
            start = bf.out[p["id"]]
            reached = self.fresh("node")
            D = []
            for bd in self.input(dom):
                if p["domain_filter"]:
                    bd = self.select(bd, p["domain_filter"])
                    if bd is None:
                        continue
                bd = self.unify_outer(bd, bd.out[dv(p["domain_id"])], reached)
                if bd is not None:
                    D.append(bd)
            if not D:
                continue
            if d == "nearest-ancestor":
                x, y = self.fresh("node"), self.fresh("node")
                EB = []
                for be in self.input(p["edges"]):
                    be = be.copy()
                    be = self.unify_outer(be, be.out[src], x)
                    if be is not None:
                        be = self.unify_outer(be, be.out[dst], y)
                    if be is not None:
                        EB.append(be)
                if not EB:
                    continue
                b = bf.copy()
                b.specials.append(["nearc", start, reached, x, y, EB, D])
                b.out = bf.out + [reached]
                out.append(b)
                continue
            if d in ("parent", "child"):
                EB = []
                for be in self.input(p["edges"]):
                    be = be.copy()
                    be = self.unify_outer(be, be.out[src], start)
                    if be is not None:
                        be = self.unify_outer(be, be.out[dst], reached)
                    if be is not None:
                        EB.append(be)
            else:
                EB = []
                for c in self.closure_br(self.input(p["edges"]), [], src, dst):
                    c = self.unify_outer(c, c.out[0], start)
                    if c is not None:
                        c = self.unify_outer(c, c.out[1], reached)
                    if c is not None:
                        EB.append(c)
            for be in EB:
                for bd in D:
                    out.append(Br(bf.atoms + be.atoms + bd.atoms, bf.filt + be.filt + bd.filt, bf.negs + be.negs + bd.negs,
                                  bf.specials + be.specials + bd.specials, bf.out + [reached]))
        return out

    def tkind(self, t):
        if is_var(t):
            return self.kind.get(t, "symbol")
        return {"t": "symbol", "n": "number", "f": "file", "ts": "symbol", "ns": "number"}.get(t[0], "symbol")

    def unify_outer(self, br, inner, outer):
        """inner term (of a sub-branch) equals outer term (bound in the enclosing branch)."""
        inner = self.flagged(outer, inner)
        outer = self.flagged(inner, outer)
        if is_var(inner):
            return self.subst(br, {inner: outer}) if inner != outer else br
        if is_var(outer):
            if inner[0] in ("t", "n"):
                br.filt.append(["=", outer, inner])
                return br
            raise Blocked("unify-set")
        return br if inner == outer else None

    # ---- lower: union-find over a branch's terms, definitions and their uses
    def unify_all(self, br, pairs):
        """all equalities `a = b` of `pairs` inside br: variables joined in a union-find (congruence classes), a class
        holding a constant keeps its variable plus one equality test; one rewrite of the branch's top level after (uses of
        definitions carry their args, so nothing below the top level is copied). None when two constants differ."""
        parent, const = {}, {}

        def find(t):
            while is_var(t) and t in parent:
                t = parent[t]
            return t
        for a, b in pairs:
            b = self.flagged(a, b)
            a = self.flagged(b, a)
            a, b = find(a), find(b)
            if a == b:
                continue
            if is_var(a) and is_var(b):
                parent[b] = a
                if b in const:
                    cb = const.pop(b)
                    if a in const and const[a] != cb:
                        return None
                    const[a] = cb
            elif is_var(a) or is_var(b):
                v, c = (a, b) if is_var(a) else (b, a)
                if c[0] not in ("t", "n"):
                    raise Blocked("unify-set")
                if v in const and const[v] != c:
                    return None
                const[v] = c
            else:
                return None
        if parent:
            br = self.subst(br, {v: find(v) for v in parent})
        for v, c in const.items():
            br.filt.append(["=", v, c])
        return br

    def cut(self, bs, key):
        """a list of branches as ONE use of a definition: the branches are stored once under `key` (never changed after)
        and the use is a branch whose output columns are fresh variables bound by a call of the definition's sub-program"""
        bs = [b for b in bs if b is not None]
        if key not in self.defs:
            self.defs[key] = bs
        body = self.defs[key]
        outs = []
        for t in (body[0].out if body else []):
            outs.append(t if not is_var(t) and t[0] == "f" else self.fresh(self.tkind(t)))
        return Br(out=list(outs), specials=[["call", key, [None if (not is_var(t) and t[0] == "f") else t for t in outs]]])

    # ---- cut: made tables sized and costed from their inputs' plans
    def made_plan(self, bs):
        """the programs of a made table's input, with their planned steps and rows per file"""
        progs, cost, rows = [], 0.0, 0.0
        for b in bs:
            progs.append(self.plan_alts(b.copy())[0]["prog"])
            cost += self.top_cost
            rows += self.top_card
        return progs, cost, rows

    def made_size(self, name, cost, rows):
        """a made table's rows per file (its input's planned rows, between 0.5 and 2000) and what building it costs once
        per file; last round assumed 5 rows and a free build, so a made table looked like the cheapest anchor"""
        if MADE_COST:
            self.schema[name]["rows"] = 2873 * min(max(rows, 0.5), 2000.0)
            self.made_cost[name] = cost

    # ---- cut: the core as top programs, and what a branch needs through its definitions
    def expand_top(self, bs):
        """the core as a bare use of one definition (a union past the branch bound, a step over the item bound): the
        definition's branches are the top programs themselves, up to the product bound (a definition used once at the
        top is inlined: each branch is within the item bound by construction)"""
        while len(bs) == 1:
            b = bs[0]
            if b.atoms or b.filt or b.negs or len(b.specials) != 1 or b.specials[0][0] != "call":
                break
            key, args = b.specials[0][1], b.specials[0][2]
            body = self.defs[key]
            vs = [t for t in args if t is not None]
            if not body or len(body) > BRANCH_CAP or len(set(vs)) != len(vs) or any(not is_var(t) for t in vs):
                break
            col = {t: c for c, t in enumerate(args) if t is not None}
            if any(is_var(t) and t not in col for t in b.out):
                break
            out = []
            for B in body:
                nb = B.copy()
                nb.out = [B.out[col[t]] if is_var(t) else t for t in b.out]
                out.append(nb)
            bs = out
        return bs

    def requires(self, b):
        """kind buckets the branch cannot hold without (each a set of kinds, one of which must occur in the file): its own
        kind-literal atoms, and what the definitions it joins (uses that bind rows, existence tests) need in every
        branch. An empty bucket proves the branch empty in a file without running it."""
        out = []
        for rel, ts in b.atoms:
            if rel in KIND_COL:
                t = ts[KIND_COL[rel]]
                lit = t if not is_var(t) else next((r for x, l, r in b.filt if x in ("=", "in") and l == t and not is_var(r)), None)
                if lit is not None and lit[0] in ("t", "ts"):
                    ks = frozenset(lit[1] if lit[0] == "ts" else [lit[1]])
                    if None not in ks:
                        out.append(ks)
        for sp in b.specials:
            if sp[0] == "call":
                out += self.def_requires(sp[1])
        for m, key, _ in b.negs:
            if m == "exists":
                out += self.def_requires(key)
        return out

    def def_requires(self, key):
        if key not in self.require_memo:
            self.require_memo[key] = []
            per = [self.requires(B) for B in self.defs[key]]
            if per and all(per):
                if len(per) == 1:
                    self.require_memo[key] = per[0][:6]
                else:
                    # one need for the whole union: each branch's rarest bucket, joined
                    pick = [min(r, key=lambda ks: sum(self.freq["kind"].get(k, 0) for k in ks)) for r in per]
                    u = frozenset().union(*pick)
                    if len(u) <= 24:
                        self.require_memo[key] = [u]
        return self.require_memo[key]

    def require_alts(self, b, alts):
        """the branch's needs as extra alternatives of its top program: the executor runs alternative 0 and skips the
        branch in a file where any alternative's kind bucket is empty; these alternatives are never run"""
        have = {json.dumps(a["prog"][0][3][1]) for a in alts if a["prog"] and a["prog"][0][0] == "iter" and a["prog"][0][2] == "kind"}
        n = 0
        for ks in self.requires(b):
            lit = ["ts", sorted(ks)] if len(ks) > 1 else ["t", next(iter(ks))]
            if json.dumps(lit) in have or n >= 6:
                continue
            have.add(json.dumps(lit))
            n += 1
            alts.append(dict(prog=[["iter", "semantic-syntax", "kind", [1, lit], self.fresh("node")], ["emit", []]], est=1.0, per=1.0))

    # ---- cut: which side a forced cut takes (costed with the planner's statistics BEFORE the cut)
    def cut_right(self, L, R, measure):
        """whether the RIGHT side of a join becomes the definition's use when a bound forces a cut. A side that is already
        one item cannot shrink. Otherwise the side WITHOUT the cheapest anchor is cut (its cheapest anchor is dearer by
        more than half), so the selective anchor stays in the joined branch; near-equal anchors: the larger side (as
        last round)."""
        ml, mr = measure(L), measure(R)
        if ml <= 1 or mr <= 1:
            return mr >= ml
        if ANCHOR_CUT:
            al, ar = self.anchor_est(L), self.anchor_est(R)
            if ar > al * 1.5:
                return True
            if al > ar * 1.5:
                return False
        return mr >= ml

    def anchor_est(self, bs):
        """rows per file the cheapest anchors of a list of branches start from (every branch is enumerated: the sum)"""
        return sum(self.branch_anchor(b) for b in bs if b is not None)

    def branch_anchor(self, b):
        """the cheapest way into one branch with nothing bound: a literal bucket of one of its atoms (est_bucket), a scan
        of its smallest relation, or the cheapest anchor of a definition it uses"""
        best = 1e6
        for rel, ts in b.atoms:
            if rel not in self.schema:
                continue
            best = min(best, self.schema[rel]["rows"] / 2873.0 + (100 if rel in TREE else 50) + self.made_cost.get(rel, 0.0))
            for c, t in enumerate(ts):
                lit = t if not is_var(t) else next((r for x, l, r in b.filt if x in ("=", "in") and l == t and not is_var(r)), None)
                if lit is None or lit[0] not in ("t", "ts", "n", "ns"):
                    continue
                if rel in TREE:
                    if lit[0] not in ("t", "ts") or not ((rel in KIND_COL and c == KIND_COL[rel]) or c in (TEXT_COL.get(rel), FIELD_COL.get(rel))
                                                         or (rel == "identifier" and c == 1)):
                        continue
                elif self.schema[rel]["columns"][c]["kind"] == "file":
                    continue
                best = min(best, self.est_bucket(rel, c, lit) + self.made_cost.get(rel, 0.0))
        for sp in b.specials:
            if sp[0] == "call":
                best = min(best, self.def_anchor(sp[1]))
        return best

    def def_anchor(self, key):
        if key not in self.anchor_memo:
            self.anchor_memo[key] = 1e6
            self.anchor_memo[key] = self.anchor_est(self.defs[key])
        return self.anchor_memo[key]

    def def_of(self, v, key):
        """the definition an input reads as a sub-plan (antijoin/difference/intersection right side): a step's own
        flattening (once per rule, shared by every use), or one atom of a relation"""
        if "step" in v:
            k = ("step", v["step"])
            if k not in self.defs:
                self.defs[k] = self.template(v["step"])
            return k
        k = ("rel", v.get("relation"))
        if k not in self.defs:
            self.defs[k] = self.input(v)
        return k

    def may_match(self, b, args):
        for t, a in zip(b.out, args):
            if a is None or is_var(a) or is_var(t):
                continue
            a = self.flagged(t, a)
            if t[0] in ("t", "n") and a[0] in ("t", "n") and t != a:
                return False
        return True

    def formals(self, key):
        """one fixed variable per column of a definition: a call writes its bound args into them (`let`), every branch of
        the definition's sub-program reads them"""
        if key not in self.formal:
            body = self.defs[key]
            self.formal[key] = [self.fresh(self.tkind(t)) for t in (body[0].out if body else [])]
        return self.formal[key]

    def body(self, key, mask, free, calls=1.0):
        """the definition's sub-program for one binding pattern: `mask` = the columns the call binds, `free` = the
        columns it collects (None: a test, the program yields). Planned once per (key, mask, free); each branch becomes
        one entry of the rule's shared table. Returns the entries' indexes.
        cut: records the plan's (steps per call, rows per call) in body_cost and the branch entries in body_branches;
        `calls` = how often per file the first requester expects to call it (amortises one-off costs inside)."""
        k = (key, mask, free)
        hit = self.body_memo.get(k)
        if hit is not None:
            return hit
        self.body_memo[k] = []  # a definition never reaches itself (the plan is acyclic)
        saved_mult, self.mult = self.mult, (max(calls * 1.0, 1e-3) if mask else 1.0)
        tc = tn = 0.0
        F = self.formals(key)
        idxs = []
        for B in self.defs[key]:
            b = B.copy()
            m, extra = {}, []
            for c in mask:
                t = b.out[c]
                if is_var(t) and t not in m:
                    m[t] = F[c]
                elif is_var(t):
                    extra.append(["=", F[c], m[t]])
                elif t[0] in ("t", "n"):
                    extra.append(["=", F[c], self.flagged(F[c], t)])
            b = self.subst(b, m) if m else b
            b.filt.extend(extra)
            sub = True
            if free is not None:
                sub = []
                for c in free:
                    t = b.out[c]
                    if not is_var(t):
                        tmp = self.fresh(self.tkind(t))
                        b.filt.append(["=", tmp, t])
                        t = tmp
                    sub.append(t)
            bound = set(F[c] for c in mask)
            try:
                if bound:
                    prog, c1 = self.plan1(b, bound, None, sub, None)
                    n1 = self.last_card
                else:
                    prog = self.plan_top(b, bound, None, sub)
                    c1, n1 = self.top_cost, self.top_card
            except Blocked as x:
                self.mult = saved_mult
                raise
            tc += c1 + 1.0
            tn += n1
            self.shared.append(clean_sub(prog, set(F[c] for c in mask)))
            idxs.append(len(self.shared) - 1)
        self.mult = saved_mult
        self.body_cost[k] = (tc, tn if free is not None else min(tn, 1.0))
        self.body_branches[k] = list(idxs)
        if len(idxs) > 1:
            # one entry runs every branch (each pushes its rows, or the first that holds answers the test): every call
            # site names the definition once, not once per branch
            self.shared.append([["exists", [[["shared", i]] for i in idxs]]] + ([["yield"]] if free is None else []))
            idxs = [len(self.shared) - 1]
        self.body_memo[k] = idxs
        self.n_bodies += 1
        if self.n_bodies > BODY_CAP:
            raise Blocked("program-size")
        return idxs

    def rename(self, b):
        """fresh variables for the branch's own (top-level) variables; variables internal to a sub-branch keep their ids
        (a sub-program runs to completion before its enclosing program goes on, so two copies never hold them at once)"""
        vs = set()
        collect_vars(b, vs)
        for s in b.specials:
            if s[0] == "clos":
                vs.update(t for t in [s[4], s[5]] + list(s[6]) if is_var(t))
            if s[0] == "nearc":
                vs.update(t for t in [s[3], s[4]] if is_var(t))
        m = {v: self.fresh(self.kind[v]) for v in vs}
        return self.subst(b.copy(), m)

    def rename_deep(self, b):
        vs = set()
        def coll(br):
            for _, ts in br.atoms:
                vs.update(t for t in ts if is_var(t))
            for _, l, r in br.filt:
                vs.update(t for t in (l, r) if is_var(t))
            vs.update(t for t in br.out if is_var(t))
            for _, bs in br.negs:
                for x in bs:
                    coll(x)
            for s in br.specials:
                vs.update(t for t in special_vars(s) if is_var(t))
                if s[0] == "near":
                    for x in s[3]:
                        coll(x)
                if s[0] == "clos":
                    vs.update(t for t in [s[4], s[5]] + list(s[6]) if is_var(t))
                    for x in s[7]:
                        coll(x)
                if s[0] == "or":
                    for x in s[2]:
                        coll(x)
                if s[0] == "nearc":
                    vs.update(t for t in [s[3], s[4]] if is_var(t))
                    for x in s[5] + s[6]:
                        coll(x)
        coll(b)
        m = {v: self.fresh(self.kind[v]) for v in vs}
        return self.subst(b.copy(), m)

    def input(self, v):
        if "relation" in v:
            rel = v["relation"]
            if rel in NOT_LOWERED:
                raise Blocked("relation-" + rel)
            if rel not in self.schema:
                raise Blocked("relation-" + rel)
            cols = self.schema[rel]["columns"]
            ts = [("f", 0) if c["kind"] == "file" else self.fresh(c["kind"]) for c in cols]
            return [Br([[rel, list(ts)]], out=list(ts))]
        if "step" in v:
            # every use gets fresh variables; a step read once is used as flattened (no copy)
            s = v["step"]
            if self.uses_of.get(s, 0) <= 1:
                bs = self.flat(s)
            else:
                T = self.template(s)
                if self.too_big(T):
                    # lower: a step read several times whose flattening exceeds the bounds is ONE definition; every use is
                    # a call of it (never a copy)
                    return [self.cut(T, ("step", s))]
                bs = [self.rename(b) for b in T]
            if (JOIN_ITEM_CUT or len(bs) > UNION_CAP) and self.too_big(bs):
                return [self.cut(bs, ("step", s))]
            return bs
        raise Blocked("input")

    def too_big(self, bs):
        return len(bs) > UNION_CAP or any(b.size() > ITEM_CAP for b in bs if b is not None)

    def template(self, s):
        if s not in self.memo:
            self.memo[s] = self.flat(s)
        return self.memo[s]

    def const(self, v):
        if "number" in v:
            return ("n", v["number"])
        if "flag" in v:
            return ("n", int(v["flag"]))
        if "text" in v:
            return ("t", v["text"])
        if "texts" in v:
            return ("ts", tuple(v["texts"]))
        if "numbers" in v:
            return ("ns", tuple(v["numbers"]))
        raise Blocked("operand")

    def operand(self, b, o):
        if "column" in o:
            return b.out[o["column"]]
        if "function" in o:
            f = o["function"]
            if f not in ("concat", "lower"):
                raise Blocked("function-" + f)
            args = [self.operand(b, a) for a in o["arguments"]]
            if all(not is_var(a) and a[0] == "t" and a[1] is not None for a in args):
                return ("t", "".join(a[1] for a in args) if f == "concat" else args[0][1].lower())
            out = self.fresh("symbol")
            b.specials.append(["fn", "text", args, [out], {"function": f}])
            return out
        return self.const(o)

    def select(self, b, comps):
        b = b.copy()
        for c in comps:
            l = self.operand(b, c["left"])
            r = self.operand(b, c["right"])
            r = self.flagged(l, r)
            l = self.flagged(r, l)
            t = c["test"]
            if t == "=":
                b = self.unify(b, l, r) if (is_var(l) or is_var(r)) else (b if l == r else None)
                if b is None:
                    return None
            elif t == "in" and is_var(l) and r[0] in ("ts", "ns") and len(r[1]) == 1:
                b.filt.append(["=", l, (r[0][0], r[1][0])])
            else:
                if not is_var(l) and not is_var(r) and "f" not in (l[0], r[0]):
                    # constant test: decided here
                    if t == "!=":
                        if l == r:
                            return None
                        continue
                    if t in ("<", "<=", ">", ">=") and l[0] == r[0] and l[0] in ("n", "t") and l[1] is not None and r[1] is not None:
                        # both constants (a bound parameter and a literal): decided here (numbers numerically, texts bytewise)
                        a, c = (l[1], r[1]) if l[0] == "n" else (l[1].encode(), r[1].encode())
                        if not {"<": a < c, "<=": a <= c, ">": a > c, ">=": a >= c}[t]:
                            return None
                        continue
                    if t == "integer-successor":
                        # lower: decided here with the executor's reading (a number is itself; a text is read as canonical
                        # integer text; anything else is no integer): the right is the left plus one, else the branch
                        # never holds (RUF007's ("t","0") vs ("t","0") is false under any reading)
                        a, c = const_int(l), const_int(r)
                        if a is None or c is None or c != a + 1:
                            return None
                        continue
                    raise Blocked("const-test-" + t)
                b.filt.append([t, l, r])
        return b

    # ---- union merging: branches equal up to one constant of one equality/in test become one in-set test
    def skeleton(self, b):
        names = {}
        def n(t):
            if is_var(t):
                if t not in names:
                    names[t] = len(names)
                return ("v", names[t])
            return t
        consts = []
        def ft(f):
            x, l, r = f
            if x in ("=", "in") and is_var(l) and not is_var(r) and r[0] in ("t", "ts", "n", "ns"):
                consts.append(r)
                return ("lit", n(l))
            return (x, n(l), n(r))
        atoms = tuple((r, tuple(n(t) for t in ts)) for r, ts in b.atoms)
        filt = tuple(ft(f) for f in b.filt)
        out = tuple(n(t) for t in b.out)
        if b.negs or b.specials:
            return None, None
        return (atoms, filt, out), consts

    def merge(self, bs):
        groups = collections.OrderedDict()
        rest = []
        for b in bs:
            sk, consts = self.skeleton(b)
            if sk is None:
                rest.append(b)
                continue
            groups.setdefault(sk, []).append((b, consts))
        out = []
        for sk, members in groups.items():
            if len(members) == 1:
                out.append(members[0][0])
                continue
            n = len(members[0][1])
            diff = [i for i in range(n) if len({json.dumps(m[1][i]) for m in members}) > 1]
            if not diff:
                out.append(members[0][0])
                continue
            if len(diff) > 1:
                out.extend(m[0] for m in members)
                continue
            i = diff[0]
            vals = []
            kind = None
            for _, consts in members:
                c = consts[i]
                kind = "t" if c[0] in ("t", "ts") else "n"
                vals.extend(c[1] if c[0] in ("ts", "ns") else [c[1]])
            b = members[0][0].copy()
            # rewrite the i-th literal test
            k = -1
            for f in b.filt:
                x, l, r = f
                if x in ("=", "in") and is_var(l) and not is_var(r) and r[0] in ("t", "ts", "n", "ns"):
                    k += 1
                    if k == i:
                        f[0] = "in"
                        f[2] = (kind + "s", tuple(sorted(set(vals), key=lambda x: (x is None, str(x)))))
            out.append(b)
        return out + rest

    # ---- planning
    def est_bucket(self, rel, col, val):
        files = 2873.0
        vals = val[1] if val[0] in ("ts", "ns") else [val[1]]
        if rel in KIND_COL and col == KIND_COL[rel]:
            return sum(self.freq["kind"].get(v, 0) for v in vals) / files
        if rel in TEXT_COL and col == TEXT_COL[rel]:
            return sum(self.freq["text"].get(v, 10) for v in vals) / files
        if rel in FIELD_COL and col == FIELD_COL[rel]:
            return sum(self.freq["field"].get(v, 0) for v in vals) / files
        rows = self.schema[rel]["rows"] / files
        st = self.freq.get("facts", {}).get(rel, {}).get(str(col))
        if st is not None:
            # lower: the column's value counts on the corpus (diag/factfreq.py), when it has few distinct values
            return sum(st.get(str(v) if not isinstance(v, bool) else str(int(v)), 0) for v in vals) / files
        if val[0] in ("n", "ns"):
            return rows * 0.3 * len(vals)
        return rows * 0.01 * len(vals)

    def plan(self, b, bound, outvars, sub):
        if sub:
            # a sub-program depends only on the branch and which of its variables are bound: planned once per rule
            key = (id(b), frozenset(v for v in b.deep() if v in bound), json.dumps(sub) if isinstance(sub, list) else sub)
            hit = self.plan_memo.get(key)
            if hit is None or hit[0] is not b:
                hit = (b, self.plan1(b, bound, outvars, sub, None)[0] if bound else self.plan_top(b, bound, outvars, sub))
                self.plan_memo[key] = hit
            return hit[1]
        if bound:
            return self.plan1(b, bound, outvars, sub, None)[0]
        return self.plan_top(b, bound, outvars, sub)

    def plan_top(self, b, bound, outvars, sub):
        b = self.detach(b, bound, sub)
        # top level: try every anchor, keep the cheapest whole plan
        best = None
        for first in self.anchor_choices(b):
            try:
                prog, cost = self.plan1(b, bound, outvars, sub, first)
            except Blocked:
                continue
            if best is None or cost < best[1]:
                best = (prog, cost, self.last_card)
        if best is None:
            prog, cost = self.plan1(b, bound, outvars, sub, None)
            best = (prog, cost, self.last_card)
        self.top_cost, self.top_card = best[1], best[2]
        return best[0]

    def plan_alts(self, b):
        """top level: every anchor's whole plan; the cheapest, plus kind-bucket anchored alternatives the executor may pick per
        file by the bucket's real size"""
        b = self.detach(b, set(), False)
        alts = []
        for first in self.anchor_choices(b):
            try:
                self.last_anchor = None
                prog, cost = self.plan1(b, set(), b.out, False, first)
            except Blocked:
                continue
            alts.append((cost, prog, self.last_anchor, self.last_card))
        if not alts:
            prog, cost = self.plan1(b, set(), b.out, False, None)
            alts = [(cost, prog, self.last_anchor, self.last_card)]
        alts.sort(key=lambda a: a[0])
        best = alts[0][0]
        self.top_cost, self.top_card = alts[0][0], alts[0][3]
        out, seen = [], set()
        for cost, prog, anchor, _ in alts:
            prog = clean(prog)
            key = json.dumps(prog[0])
            if key in seen:
                continue
            if out and not (prog[0][0] == "iter" and prog[0][2] == "kind" and cost <= 50 * best + 50):
                continue
            seen.add(key)
            est = max(anchor or 1.0, 0.01)
            out.append(dict(prog=prog, est=est, per=cost / est))
            if len(out) >= 4:
                break
        return out

    def anchor_choices(self, b):
        """what a top-level plan may start from: each atom, and (cut) each use of a definition, entered from the
        definition's side (its rows drive, the rest of the branch becomes the test)"""
        out = [("a", i) for i in range(len(b.atoms))]
        if CALL_COST:
            out += [("s", j) for j, sp in enumerate(b.specials) if sp[0] == "call"]
        return out or [None]

    def detach(self, b, bound, sub):
        """navwide: parts of the branch sharing no unbound variable with the answer (its output or collected variables) are
        existence tests: one sub-program that stops at the first binding, not a cross product"""
        if os.environ.get("NAV_DETACH", "0") != "1" or (not bound and os.environ.get("NAV_DETACH_TOP", "0") != "1"):
            return b
        need = set(t for t in (b.out if not sub else (sub if isinstance(sub, list) else [])) if is_var(t))
        items = [("a", i, set(t for t in ts if is_var(t))) for i, (_, ts) in enumerate(b.atoms)]
        items += [("f", i, set(t for t in f[1:] if is_var(t))) for i, f in enumerate(b.filt)]
        items += [("s", i, set(t for t in special_vars(sp) if is_var(t))) for i, sp in enumerate(b.specials)]
        top = collect_vars(b, set())
        items += [("n", i, set(v for x in bs for v in x.deep() if v in top)) for i, (_, bs) in enumerate(b.negs)]
        if len(items) < 2:
            return b
        parent = list(range(len(items)))

        def find(i):
            while parent[i] != i:
                parent[i] = parent[parent[i]]
                i = parent[i]
            return i
        owner = {}
        for k, (_, _, vs) in enumerate(items):
            for v in vs:
                if v in bound:
                    continue
                if v in owner:
                    parent[find(k)] = find(owner[v])
                else:
                    owner[v] = k
        groups = collections.defaultdict(list)
        for k in range(len(items)):
            groups[find(k)].append(k)
        if len(groups) < 2:
            return b
        detached = [g for g in groups.values() if not any(v in need for k in g for v in items[k][2] if v not in bound)
                    and any(items[k][0] == "a" for k in g)]
        if not detached or len(detached) == len(groups):
            return b
        drop = set(k for g in detached for k in g)
        nb = Br([a for i, a in enumerate(b.atoms) if ("a", i) not in {(items[k][0], items[k][1]) for k in drop}],
                [f for i, f in enumerate(b.filt) if ("f", i) not in {(items[k][0], items[k][1]) for k in drop}],
                [n for i, n in enumerate(b.negs) if ("n", i) not in {(items[k][0], items[k][1]) for k in drop}],
                [x for i, x in enumerate(b.specials) if ("s", i) not in {(items[k][0], items[k][1]) for k in drop}],
                list(b.out))
        for g in detached:
            sel = {(items[k][0], items[k][1]) for k in g}
            sb = Br([a for i, a in enumerate(b.atoms) if ("a", i) in sel], [f for i, f in enumerate(b.filt) if ("f", i) in sel],
                    [n for i, n in enumerate(b.negs) if ("n", i) in sel], [x for i, x in enumerate(b.specials) if ("s", i) in sel], [])
            nb.negs.append(["exists-late", [sb]])
        return nb

    def plan1(self, b, bound, outvars, sub, first):
        """One branch -> a list of instructions. bound: variables bound by the enclosing program."""
        if first is None:
            b = self.detach(b, bound, sub)
        bound = set(bound)
        cost = [0.0, 1.0]
        E = collect_vars(b, set(bound))
        prog = []
        built = set()  # cut: made tables this plan already reads (their build is costed once)
        atoms = list(range(len(b.atoms)))
        filt = list(b.filt)
        negs = list(b.negs)
        specials = list(b.specials)

        def tbound(t):
            return (not is_var(t)) or t in bound

        def lit_on(var):
            for x, l, r in filt:
                if x in ("=", "in") and l == var and not is_var(r):
                    return r
            return None

        def flush_filters():
            nonlocal filt
            keep = []
            for f in filt:
                x, l, r = f
                if tbound(l) and tbound(r):
                    prog.append(["test", x, l, r])
                    if os.environ.get("NAV_SEL", "1") == "1":
                        cost[1] *= (0.1 if not is_var(r) else 0.3) if x in ("=", "in") else (0.95 if x == "!=" else 0.7)
                    else:
                        cost[1] *= 0.3 if x in ("=", "in") else 0.7
                else:
                    keep.append(f)
            filt = keep

        def place_atom(i, access):
            rel, ts = b.atoms[i]
            kind, arg = access
            row = None
            if rel in TREE:
                key = ts[TREE[rel]]
                if kind == "key":
                    prog.append(["row", rel, key])
                    row = key
                else:
                    row = key if is_var(key) and key not in bound else self.fresh("node")
                    prog.append(["iter", rel, kind, arg, row])
                    if key != row:
                        prog.append(["test", "=", row, key])
                    bound.add(row)
                for c, t in enumerate(ts):
                    if c == TREE[rel] or (not is_var(t) and t[0] == "f"):
                        continue
                    if kind == "children" and c == PARENT_COL.get(rel):
                        continue
                    if is_var(t) and t not in bound:
                        prog.append(["read", rel, c, row, t])
                        bound.add(t)
                    else:
                        # compare in place
                        tmp = self.fresh(self.schema[rel]["columns"][c]["kind"])
                        prog.append(["read", rel, c, row, tmp])
                        prog.append(["test", "=", tmp, t])
            else:
                row = self.fresh("row")
                prog.append(["iter", rel, kind, arg, row])
                for c, t in enumerate(ts):
                    if not is_var(t) and t[0] == "f":
                        continue
                    if kind == "by" and c == arg[0] and (not is_var(arg[1]) or arg[1] == t) and (is_var(t) and t in bound or not is_var(t)):
                        continue
                    if is_var(t) and t not in bound:
                        prog.append(["read", rel, c, row, t])
                        bound.add(t)
                    else:
                        tmp = self.fresh(self.schema[rel]["columns"][c]["kind"])
                        prog.append(["read", rel, c, row, tmp])
                        prog.append(["test", "=", tmp, t])
            if kind in ("kind", "value", "by") and arg is not None and not is_var(arg[1]):
                # the bucket's literal test is implied by the access
                lit_var = ts[arg[0]]
                filt[:] = [f for f in filt if not (f[0] in ("=", "in") and f[1] == lit_var and f[2] == arg[1])]
                prog[:] = [x for x in prog if not (x[0] == "test" and x[1] in ("=", "in") and x[2] == lit_var and x[3] == arg[1])]
            atoms.remove(i)
            flush_filters()

        def lead(var):
            """an iteration whose new id leads to literal tests through keyed reads is preferred (selective first)"""
            if not is_var(var) or var in bound:
                return 1.0
            f = 1.0
            for j in atoms:
                rel2, ts2 = b.atoms[j]
                if rel2 in TREE and ts2[TREE[rel2]] == var:
                    for c, t in enumerate(ts2):
                        if c != TREE[rel2] and ((not is_var(t) and t[0] in ("t", "ts")) or (is_var(t) and lit_on(t) is not None)):
                            f *= 0.3
            return f

        def accesses(i):
            """possible accesses of atom i now: (cost, kind, arg)"""
            rel, ts = b.atoms[i]
            out = []
            if rel in TREE:
                key = ts[TREE[rel]]
                if tbound(key):
                    out.append((0.9 if rel in TOTAL else 1.0, "key", None))
                if rel in PARENT_COL and tbound(ts[PARENT_COL[rel]]):
                    lit = lit_on(ts[FIELD_COL[rel]]) if is_var(ts[FIELD_COL[rel]]) else ts[FIELD_COL[rel]]
                    if lit and lit[0] == "t":
                        fan = self.freq["fanout"].get(lit[1], 1.5)
                    elif lit and lit[0] == "ts":
                        fan = sum(self.freq["fanout"].get(x, 1.5) for x in lit[1])
                    else:
                        fan = 6.0
                    if os.environ.get("NAV_FAN", "0") != "1":
                        fan = 1.5 if lit else 4.0
                    out.append((fan * (lead(key) if os.environ.get("NAV_LEAD", "0") == "1" else 1.0), "children", ts[PARENT_COL[rel]]))
                for c, t in enumerate(ts):
                    if c == TREE[rel]:
                        continue
                    lit = (t if not is_var(t) else lit_on(t))
                    if lit is not None and lit[0] in ("t", "ts") and (rel in KIND_COL and c == KIND_COL[rel]):
                        out.append((self.est_bucket(rel, c, lit), "kind", [c, lit]))
                    elif lit is not None and lit[0] in ("t", "ts") and c in (TEXT_COL.get(rel), FIELD_COL.get(rel)) or \
                            (lit is not None and lit[0] in ("t", "ts") and rel == "identifier" and c == 1):
                        out.append((self.est_bucket(rel, c, lit) + 2, "value", [c, lit]))
                    elif is_var(t) and t in bound and self.schema[rel]["columns"][c]["kind"] in ("symbol", "node"):
                        out.append((8.0, "value", [c, t]))
                    elif is_var(t) and t in bound and self.schema[rel]["columns"][c]["kind"] == "number" and \
                            os.environ.get("NAV_NUM_INDEX", "1") == "1":
                        # navwide: a number column (a line, an offset) equal to a bound value: the column's per-file index
                        out.append((8.0, "value", [c, t]))
                out.append((self.schema[rel]["rows"] / 2873.0 + 100, "scan", None))
            else:
                cols = self.schema[rel]["columns"]
                for c, t in enumerate(ts):
                    if cols[c]["kind"] in ("file",):
                        continue
                    if tbound(t) and cols[c]["kind"] in ("node", "symbol", "number"):
                        if is_var(t) or t[0] in ("t", "n"):
                            cost = 1.0 if cols[c]["key"] else (1.3 if cols[c]["kind"] == "node" else 6.0)
                            out.append((cost, "by", [c, t]))
                    elif is_var(t):
                        lit = lit_on(t)
                        if lit is not None:
                            out.append((self.est_bucket(rel, c, lit), "by", [c, lit]))
                out.append((self.schema[rel]["rows"] / 2873.0 + 50, "scan", None))
            return out

        def special_ready(s):
            if s[0] == "fn":
                return all(tbound(t) for t in s[2])
            if s[0] == "near":
                return tbound(s[1]) or tbound(s[2])
            if s[0] == "clos":
                return tbound(s[1]) or tbound(s[2])
            if s[0] in ("or", "call"):
                return True
            if s[0] == "nearc":
                return tbound(s[1])
            return tbound(s[2]) or tbound(s[3])

        flush_filters()
        while atoms or specials or negs:
            best = None
            for i in atoms:
                if first is not None and not bound and first != ("a", i):
                    continue
                for c, kind, arg in accesses(i):
                    if not bound and kind in ("key", "children"):
                        continue
                    if bound and (kind in ("kind", "value", "scan") and not (kind == "value" and is_var(arg[1])) or
                                  kind == "by" and not is_var(arg[1]) and not self.schema[b.atoms[i][0]]["columns"][arg[0]]["key"]):
                        c = c + 10.0 if not BUCKET_X else c * BUCKET_X + 10.0  # a bucket inside a candidate is a cross product
                    if best is None or c < best[0]:
                        best = (c, "atom", i, (kind, arg))
            for j, s in enumerate(specials):
                if first is not None and first[0] == "s" and not bound and first != ("s", j):
                    continue
                if special_ready(s):
                    if s[0] == "call" and CALL_COST:
                        # cut: a use of a definition is costed from the definition's own plan for this binding pattern
                        # (keyed call, probe of its rows, or its rows driving), see call_plan
                        if first is not None and not bound and first != ("s", j):
                            continue
                        opt = self.call_plan(s[1], s[2], tbound, bool(bound), cost[1] * self.mult)
                        if best is None or opt[1] < best[0]:
                            best = (opt[1], "special", j, opt)
                        continue
                    if s[0] == "fn":
                        c = 2.0 if s[1] == "source_matches" else 0.5
                    elif s[0] == "near" and not tbound(s[1]):
                        c = 30.0
                    elif s[0] == "clos":
                        c = 8.0
                    elif s[0] == "nearc":
                        c = 10.0
                    elif s[0] == "or":
                        c = (3.0 * len(s[2])) if any(tbound(t) for t in s[1]) else (200.0 * len(s[2]) if bound else 1e6)
                    elif s[0] == "call":
                        nb = max(len(self.defs[s[1]]), 1)
                        c = (3.0 * nb) if any(t is not None and tbound(t) for t in s[2]) else (200.0 * nb if bound else 1e6)
                    else:
                        d0 = s[1] if s[0] == "axis" else "near"
                        if s[0] == "axis" and not tbound(s[2]) and tbound(s[3]) and (
                                os.environ.get("NAV_INV_COST", "0") == "1" or s[1] in ("next-sibling", "previous-sibling", "deep")):
                            # walked from the reached end (navwide): the inverse direction's cost
                            d0 = {"parent": "child", "child": "parent", "ancestor": "descendant", "descendant": "ancestor",
                                  "next-sibling": "previous-sibling", "previous-sibling": "next-sibling", "deep": "ancestor"}[d0]
                        c = {"parent": 1.0, "near": 1.0, "child": 4.0, "ancestor": 8.0, "descendant": 30.0, "next-sibling": 1.0,
                             "previous-sibling": 1.0, "deep": 30.0}[d0]
                    if best is None or c < best[0]:
                        best = (c, "special", j, None)
            ready_negs = [k for k, (m, key, args) in enumerate(negs)
                          if all(t in bound for t in args if t is not None and is_var(t) and t in E)]
            if ready_negs and (best is None or best[0] > 1.0):
                k = ready_negs[0]
                m, key, args = negs.pop(k)
                per, once = self.place_test(prog, m, key, args, tbound, cost[1] * self.mult)
                cost[0] += cost[1] * per + once
                continue
            if best is None:
                if negs:
                    # a test whose args are not all bound: the unbound columns are existential inside the definition
                    m, key, args = negs.pop(0)
                    per, once = self.place_test(prog, m, key, args, tbound, cost[1] * self.mult)
                    cost[0] += cost[1] * per + once
                    continue
                k = next((j for j, x in enumerate(specials) if x[0] == "clos"), None)
                if k is not None:
                    # lower: a closure with neither end bound starts from every edge source (with its keys): an `or`
                    # collecting the edge sub-branches' source column, distinct, binds the start; then the closure runs
                    _, a, bb, kv, x, y, kx, D = specials[k]
                    col = [x] + [t for t in kx if is_var(t)]
                    want = [a] + [v for t, v in zip(kx, kv) if is_var(t)]
                    outs, post = [], []
                    for t in want:
                        if tbound(t) or t in outs:
                            tmp = self.fresh(self.tkind(t))
                            outs.append(tmp)
                            post.append(["test", "=", tmp, t])
                        else:
                            outs.append(t)
                    prog.append(["or", outs, [self.plan(z, set(bound), None, col) for z in D]] + ([1] if not bound else []))
                    bound.update(outs)
                    prog.extend(post)
                    cost[0] += cost[1] * 200.0
                    continue
                if os.environ.get("NAV_DEBUG"):
                    print("UNPLANNABLE bound", sorted(bound), "atoms", [b.atoms[i] for i in atoms], "specials", [x[:3] if x[0] != "call" else x for x in specials], "sub", sub, file=sys.stderr)
                raise Blocked("unplannable")
            if first is not None and not bound and best[1] == "atom" and best[3][0] == "scan":
                raise Blocked("scan-anchor")
            if best[1] == "special" and best[3] is not None:
                # cut: a costed use of a definition
                how, _, per, once, fan = best[3]
                if not bound and cost[0] == 0.0:
                    self.last_anchor = max(fan, 0.01)
                eff = cost[1] * self.mult
                cost[0] += once + cost[1] * per
                cost[1] *= max(fan, 0.05)
                sp = specials.pop(best[2])
                self.place_call(prog, sp[1], sp[2], tbound, bound, how, eff)
                flush_filters()
                continue
            if best[1] == "atom" and MADE_COST:
                rel0 = b.atoms[best[2]][0]
                if rel0 in self.made_cost and rel0 not in built:
                    # cut: the first read of a made table builds it (once per file, when a candidate gets here)
                    built.add(rel0)
                    cost[0] += self.made_cost[rel0] * min(1.0, cost[1] * self.mult)
            if not bound and cost[0] == 0.0:
                self.last_anchor = best[0]
            cost[0] += cost[1] * best[0]
            if not (best[1] == "atom" and best[3][0] == "key") and not (best[1] == "special" and best[0] <= 1.0) and \
                    not (best[1] == "special" and specials[best[2]][0] == "fn"):
                cost[1] *= max(best[0], 0.05)
            if best[1] == "atom":
                place_atom(best[2], best[3])
            else:
                s = specials.pop(best[2])
                if s[0] == "fn":
                    outs = []
                    post = []
                    for t in s[3]:
                        if tbound(t):
                            tmp = self.fresh(self.kind.get(t, "number") if is_var(t) else "number")
                            outs.append(tmp)
                            post.append(["test", "=", tmp, t])
                        else:
                            outs.append(t)
                            bound.add(t)
                    prog.append(["fn", s[1], s[2], outs, s[4]])
                    prog.extend(post)
                elif s[0] == "nearc":
                    _, start, reached, x, y, EB, D = s
                    eprogs = [self.plan(z, bound | {x}, None, [y]) for z in EB]
                    if tbound(reached):
                        tmp = self.fresh("node")
                        dprogs = [self.plan(z, bound | {tmp}, None, True) for z in self.subst_all(D, reached, tmp)]
                        prog.append(["nearc", start, tmp, x, y, eprogs, dprogs])
                        prog.append(["test", "=", tmp, reached])
                    else:
                        dprogs = [self.plan(z, bound | {reached}, None, True) for z in D]
                        prog.append(["nearc", start, reached, x, y, eprogs, dprogs])
                        bound.add(reached)
                elif s[0] == "call":
                    self.place_call(prog, s[1], s[2], tbound, bound, None, cost[1] * self.mult)
                elif s[0] == "or":
                    _, outs, D = s
                    free = [t for t in outs if is_var(t) and not tbound(t)]
                    free = list(dict.fromkeys(free))
                    if free:
                        # a union reading no bound variable answers the same rows for every candidate: memoised per file
                        uncorrelated = not any(v in bound for z in D for v in z.deep())
                        prog.append(["or", free, [self.plan(z, set(bound), None, free) for z in D]] + ([1] if uncorrelated else []))
                    else:
                        prog.append(["exists", [self.plan(z, set(bound), None, True) for z in D]])
                    bound.update(free)
                elif s[0] == "clos":
                    _, a, bb, kv, x, y, kx, D = s
                    if tbound(a):
                        src, dst, xin, yout = a, bb, x, y
                    else:
                        src, dst, xin, yout = bb, a, y, x
                    kb = all(tbound(k) for k in kv)
                    if kb:
                        m = {k: v for k, v in zip(kx, kv) if is_var(k)}
                        Dp = [self.subst(z.copy(), m) for z in D]
                        kcol, kout = [], []
                    else:
                        Dp = D
                        kcol = [k for k in kx if is_var(k)]
                        kout = [v for k, v in zip(kx, kv) if is_var(k)]
                        if any(tbound(v) for v in kout):
                            raise Blocked("closure-partial-keys")
                    progs = [self.plan(z, bound | {xin}, None, [yout] + kcol) for z in Dp]
                    if tbound(dst):
                        tmp = self.fresh(self.kind.get(dst, "node"))
                        prog.append(["clos", src, tmp, xin, yout, kcol, kout, progs])
                        prog.append(["test", "=", tmp, dst])
                    else:
                        prog.append(["clos", src, dst, xin, yout, kcol, kout, progs])
                        bound.add(dst)
                    bound.update(kout)
                elif s[0] == "near":
                    _, start, reached, D = s
                    if not tbound(start):
                        # reached is bound: the start is one of its descendants whose nearest admitted ancestor it is
                        prog.append(["axis", "descendant", reached, start])
                        bound.add(start)
                    if reached in bound:
                        tmp = self.fresh("node")
                        prog.append(["near", start, tmp, [self.plan(x, bound | {tmp}, None, True) for x in self.subst_all(D, reached, tmp)]])
                        prog.append(["test", "=", tmp, reached])
                    else:
                        prog.append(["near", start, reached, [self.plan(x, bound | {reached}, None, True) for x in D]])
                        bound.add(reached)
                elif s[1] in ("next-sibling", "previous-sibling", "deep"):
                    # navwide: sibling walks and deep descendants (descendants that are not children)
                    _, d, start, reached = s
                    fwd = tbound(start)
                    a, z = (start, reached) if fwd else (reached, start)
                    tmp = self.fresh("node") if tbound(z) else z
                    if d == "deep":
                        if fwd:
                            prog.append(["axis", "descendant", a, tmp])
                            par = self.fresh("node")
                            prog.append(["axis", "parent", tmp, par])
                            prog.append(["test", "!=", par, a])
                        else:
                            par = self.fresh("node")
                            prog.append(["axis", "parent", a, par])
                            prog.append(["axis", "ancestor", a, tmp])
                            prog.append(["test", "!=", tmp, par])
                    else:
                        nxt = (d == "next-sibling") == fwd
                        prog.append(["sib", nxt, a, tmp])
                    if tmp != z:
                        prog.append(["test", "=", tmp, z])
                    else:
                        bound.add(z)
                else:
                    _, d, start, reached = s
                    if tbound(start) and not tbound(reached):
                        prog.append(["axis", d, start, reached])
                        bound.add(reached)
                    elif tbound(start) and tbound(reached):
                        tmp = self.fresh("node")
                        prog.append(["axis", d, start, tmp])
                        prog.append(["test", "=", tmp, reached])
                    else:
                        inv = {"parent": "child", "child": "parent", "ancestor": "descendant", "descendant": "ancestor"}[d]
                        prog.append(["axis", inv, reached, start])
                        bound.add(start)
                flush_filters()
        while filt:
            # an equality left with one side unbound once nothing else binds it: the variable takes the other side's value
            # (a union branch whose column is a constant or another column)
            lets = [f for f in filt if f[0] == "=" and (
                (is_var(f[1]) and f[1] not in bound and tbound(f[2])) or (is_var(f[2]) and f[2] not in bound and tbound(f[1])))]
            if not lets:
                break
            x, l, r = lets[0]
            if is_var(l) and l not in bound:
                prog.append(["let", r, l])
                bound.add(l)
            else:
                prog.append(["let", l, r])
                bound.add(r)
            flush_filters()
        if filt:
            if os.environ.get("NAV_DEBUG"):
                print("UNBOUND", filt, "bound", sorted(bound), "atoms", [b.atoms[i] for i in atoms], "specials", specials, file=sys.stderr)
            raise Blocked("unbound-filter")
        if not sub:
            for t in b.out:
                if is_var(t) and t not in bound:
                    raise Blocked("unbound-output")
            prog.append(["emit", list(b.out)])
        elif isinstance(sub, list):
            for t in sub:
                if is_var(t) and t not in bound:
                    if os.environ.get("NAV_DEBUG"):
                        print("UNCOLLECT", t, sub, "bound", sorted(bound), "prog", prog, "branch", b.atoms, b.filt, b.specials, file=sys.stderr)
                    raise Blocked("unbound-collect")
            prog.append(["collect", list(sub)])
        else:
            prog.append(["yield"])
        self.last_card = cost[1]
        return prog, cost[0]

    def mask_of(self, key, args, tbound):
        """the columns a use binds; past MASK_CAP distinct patterns per definition the use binds none (the definition's
        uncorrelated program, memoised per file, with the args tested after): the bound on programs per definition"""
        mask = tuple(c for c, t in enumerate(args) if t is not None and tbound(t))
        seen = self.masks.setdefault(key, set())
        if mask not in seen and len(seen) >= MASK_CAP:
            mask = ()
        seen.add(mask)
        return mask

    def place_test_old(self, prog, m, key, args, tbound):
        """a use of a definition as an absence or existence test: lets of the bound args into its formals, then the
        sub-program (memoised per file when it binds nothing)"""
        if not self.defs[key]:
            if m == "exists":
                prog.append(["exists", []])
            return
        mask = self.mask_of(key, args, tbound)
        F = self.formals(key)
        idxs = self.body(key, mask, None)
        for c in mask:
            prog.append(["let", args[c], F[c]])
        subs = [[["shared", i]] for i in idxs]
        if not mask:
            prog.append([m + "-memo", subs])
            # a column whose arg is bound but not passed: the definition is uncorrelated, nothing to test (an
            # uncorrelated test only happens for args that are all existential)
            if any(t is not None and tbound(t) for t in args):
                prog.pop()
                # degraded pattern: collect the bound columns and compare (never an over-approximation)
                cols = tuple(c for c, t in enumerate(args) if t is not None and tbound(t))
                idxs = self.body(key, (), cols)
                outs = [self.fresh(self.tkind(args[c])) for c in cols]
                sub = [["or", outs, [[["shared", i]] for i in idxs], 1]] + \
                      [["test", "=", o, args[c]] for o, c in zip(outs, cols)] + [["yield"]]
                prog.append([m, [sub]])
            return
        if KEYED_MEMO and self.heavy(idxs):
            # a test whose sub-program does more than reads and tests: memoised per file by its bound args
            prog.append([m + "-keyed", subs, [F[c] for c in mask]])
            return
        prog.append([m, subs])

    def heavy(self, idxs):
        """a sub-program that iterates or calls further sub-programs (worth a memo lookup per call)"""
        memo = self.heavy_memo

        def h(p):
            for ins in p:
                if ins[0] == "shared":
                    j = ins[1]
                    if j not in memo:
                        memo[j] = False
                        memo[j] = h(self.shared[j])
                    if memo[j]:
                        return True
                elif ins[0] in ("iter", "neg", "exists", "or", "or-keyed", "clos", "near", "nearc", "neg-keyed", "exists-keyed",
                                "neg-memo", "exists-memo", "axis", "or-probe"):
                    return True
            return False
        return any(h([["shared", i]]) for i in idxs)

    def place_call_old(self, prog, key, args, tbound, bound):
        """a use of a definition that binds rows: lets of the bound args into its formals, then `or` over the
        sub-program's collected rows (distinct), each continuing the program; the free args are bound positionally"""
        if not self.defs[key]:
            prog.append(["exists", []])
            return
        mask = self.mask_of(key, args, tbound)
        F = self.formals(key)
        free = tuple(c for c, t in enumerate(args) if t is not None and c not in mask)
        for c in mask:
            prog.append(["let", args[c], F[c]])
        if not free:
            idxs = self.body(key, mask, None)
            subs = [[["shared", i]] for i in idxs]
            if not mask:
                prog.append(["exists-memo", subs])
            elif KEYED_MEMO and self.heavy(idxs):
                prog.append(["exists-keyed", subs, [F[c] for c in mask]])
            else:
                prog.append(["exists", subs])
            return
        idxs = self.body(key, mask, free)
        outs, post = [], []
        for c in free:
            t = args[c]
            if tbound(t) or t in outs:
                tmp = self.fresh(self.tkind(t))
                outs.append(tmp)
                post.append(["test", "=", tmp, t])
            else:
                outs.append(t)
        ins = ["or", outs, [[["shared", i]] for i in idxs]]
        if not mask:
            ins.append(1)
        elif KEYED_MEMO:
            ins = ["or-keyed", outs, ins[2], [F[c] for c in mask]]
        prog.append(ins)
        bound.update(outs)
        prog.extend(post)

    # ---- cut: uses of definitions, costed and entered from either side ----------------------------------------------
    def mask_room(self, key, mask):
        """whether the definition may be planned for this binding pattern (at most MASK_CAP patterns per definition); a
        pattern asked for is counted"""
        seen = self.masks.setdefault(key, set())
        if mask in seen:
            return True
        if len(seen) >= MASK_CAP:
            return False
        seen.add(mask)
        return True

    def all_cols(self, key):
        """the columns of a definition its uncorrelated rows hold (every column but a file constant)"""
        body = self.defs[key]
        return tuple(c for c, t in enumerate(body[0].out) if is_var(t) or t[0] != "f") if body else ()

    def def_cost(self, key, mask, free, calls=1.0):
        """(steps per call, rows per call) of the definition's sub-program for a binding pattern, from its plan"""
        self.body(key, mask, free, calls)
        return self.body_cost.get((key, mask, free), (1e6, 1.0))

    def call_plan(self, key, args, tbound, any_bound, eff):
        """how a use of a definition that binds rows runs at this point of a plan:
          (how, rank, steps per candidate, one-off steps, rows per candidate)
        drive  no arg is bound: the definition's uncorrelated rows (once per file) continue the program;
        keyed  the bound args are passed, the sub-program planned for that pattern runs (memoised by its args);
        probe  the definition's uncorrelated rows are collected once per file and indexed by the bound columns: a hash
               join whose build side is the definition. Taken when building once is cheaper than `eff` keyed calls
               (eff = candidates per file expected here), or when the definition has no pattern left.
        The rank orders the use among the branch's other items: its rows per candidate, or its steps per candidate over
        KEYED_STEPS when it is dearer than a well-keyed call."""
        if not self.defs[key]:
            return ("empty", 0.0, 0.0, 0.0, 0.0)
        cols = tuple(c for c, t in enumerate(args) if t is not None)
        mask = tuple(c for c in cols if tbound(args[c]))
        free = tuple(c for c in cols if c not in mask)
        if not mask:
            c0, n0 = self.def_cost(key, (), free if free else None)
            if not any_bound:
                return ("drive", c0, 0.0, c0, n0)
            # a cross product with the candidates so far: the memoised rows per candidate
            return ("drive", n0 * BUCKET_X + 10.0, max(n0, 1.0), c0 * min(1.0, eff), n0)
        best = None
        if self.mask_room(key, mask):
            cm, nm = self.def_cost(key, mask, free if free else None, eff)
            best = ("keyed", max(nm, cm / KEYED_STEPS, 1.05), cm + 1.0, 0.0, nm)
        allc = self.all_cols(key)
        if PROBE and all(c in allc for c in cols):
            c0, n0 = self.def_cost(key, (), allc)
            fan = min(best[4], n0) if best else n0 * 0.1 ** len(mask)
            if not free:
                fan = min(fan, 1.0)
            once = c0 * min(1.0, eff)
            per = 1.0 + fan
            if best is None or once + eff * per < eff * best[2]:
                best = ("probe", max(fan, (once / max(eff, 1e-6) + per) / KEYED_STEPS, 1.05), per, once, fan)
        if best is None:
            # no pattern left and no probe: last round's degraded use (uncorrelated rows, args tested after)
            c0, n0 = self.def_cost(key, (), cols)
            best = ("degraded", n0 * BUCKET_X + 10.0, max(n0, 1.0), c0 * min(1.0, eff), n0 * 0.1 ** len(mask))
        return best

    def light(self, idxs):
        """sub-programs that only read, test and bind (no iteration, no call): cheaper run in place than called"""
        for i in idxs:
            for ins in self.shared[i]:
                if ins[0] not in ("read", "row", "test", "let", "fn", "yield", "collect") or (ins[0] == "fn" and ins[1] in ("source_matches", "capture")):
                    return False
        return True

    def inline_args(self, prog, key, mask, args):
        """the substitution of a definition's formals by a use's bound args (constants keep a `let` into the formal)"""
        F = self.formals(key)
        m = {}
        for c in mask:
            if is_var(args[c]):
                m[F[c]] = args[c]
            else:
                prog.append(["let", args[c], F[c]])
        return m

    def place_test(self, prog, m, key, args, tbound, eff=1.0):
        """a use of a definition as an absence or existence test. Returns (steps per candidate, one-off steps).
        cut: a sub-program that only reads and tests is inlined (the args substituted for the formals, no `let`); the
        keyed test passes its args in the instruction; the test probes the definition's uncorrelated rows (indexed by
        the bound columns, built once per file) when that is cheaper than `eff` keyed runs or no pattern is left."""
        if not CALL_COST:
            self.place_test_old(prog, m, key, args, tbound)
            return 5.0, 0.0
        if not self.defs[key]:
            if m == "exists":
                prog.append(["exists", []])
            return 1.0, 0.0
        cols = tuple(c for c, t in enumerate(args) if t is not None)
        mask = tuple(c for c in cols if tbound(args[c]))
        F = self.formals(key)
        if not mask:
            idxs = self.body(key, (), None)
            prog.append([m + "-memo", [[["shared", i]] for i in idxs]])
            return 1.0, self.body_cost[(key, (), None)][0] * min(1.0, eff)
        cm = None
        if self.mask_room(key, mask):
            idxs = self.body(key, mask, None, eff)
            cm = self.body_cost[(key, mask, None)][0]
            br = self.body_branches[(key, mask, None)]
            if INLINE_LIGHT and self.light(br):
                sub = self.inline_args(prog, key, mask, args)
                prog.append([m, [substitute(self.shared[i], sub) for i in br]])
                return cm, 0.0
        allc = self.all_cols(key)
        if PROBE and all(c in allc for c in mask):
            c0 = self.def_cost(key, (), allc)[0]
            once = c0 * min(1.0, eff)
            if cm is None or once + eff * 1.0 < eff * (cm + 1.0):
                idxs = self.body(key, (), allc)
                prog.append([m + "-probe", [[["shared", i]] for i in idxs], [allc.index(c) for c in mask], [args[c] for c in mask], len(allc)])
                return 1.0, once
        if cm is None:
            self.place_test_old(prog, m, key, args, tbound)
            return 5.0, 0.0
        subs = [[["shared", i]] for i in idxs]
        if KEYED_MEMO and self.heavy(idxs):
            prog.append([m + "-keyed", subs, [F[c] for c in mask], [args[c] for c in mask]])
        else:
            for c in mask:
                prog.append(["let", args[c], F[c]])
            prog.append([m, subs])
        return cm + 1.0, 0.0

    def place_call(self, prog, key, args, tbound, bound, how=None, eff=1.0):
        """a use of a definition that binds rows, placed the way call_plan chose (`how`): its rows drive (`or`, memoised
        per file), a keyed call (`or-keyed` carrying its args; a single-branch sub-program that only reads is inlined),
        or a probe of its uncorrelated rows (`or-probe`)."""
        if how is None or how == "degraded" or not CALL_COST:
            return self.place_call_old(prog, key, args, tbound, bound)
        if not self.defs[key]:
            prog.append(["exists", []])
            return
        cols = tuple(c for c, t in enumerate(args) if t is not None)
        mask = tuple(c for c in cols if tbound(args[c]))
        free = tuple(c for c in cols if c not in mask)
        F = self.formals(key)
        if how == "probe":
            allc = self.all_cols(key)
            subs = [[["shared", i]] for i in self.body(key, (), allc)]
            if not free:
                prog.append(["exists-probe", subs, [allc.index(c) for c in mask], [args[c] for c in mask], len(allc)])
                return
            outs, post = self.call_outs(args, free, tbound)
            prog.append(["or-probe", outs, subs, [allc.index(c) for c in mask], [args[c] for c in mask], [allc.index(c) for c in free], len(allc)])
            bound.update(outs)
            prog.extend(post)
            return
        if how == "drive":
            mask = ()
        if not free:
            idxs = self.body(key, mask, None, eff)
            subs = [[["shared", i]] for i in idxs]
            if not mask:
                prog.append(["exists-memo", subs])
            elif INLINE_LIGHT and self.light(self.body_branches[(key, mask, None)]):
                sub = self.inline_args(prog, key, mask, args)
                prog.append(["exists", [substitute(self.shared[i], sub) for i in self.body_branches[(key, mask, None)]]])
            elif KEYED_MEMO and self.heavy(idxs):
                prog.append(["exists-keyed", subs, [F[c] for c in mask], [args[c] for c in mask]])
            else:
                for c in mask:
                    prog.append(["let", args[c], F[c]])
                prog.append(["exists", subs])
            return
        idxs = self.body(key, mask, free, eff)
        outs, post = self.call_outs(args, free, tbound)
        br = self.body_branches[(key, mask, free)]
        if mask and INLINE_LIGHT and len(br) == 1 and self.light(br):
            # one straight-line sub-program: run in place; its collected values become the outs
            sub = self.inline_args(prog, key, mask, args)
            body = substitute(self.shared[br[0]], sub)
            vals = body[-1][1]
            prog.extend(body[:-1])
            for o, v in zip(outs, vals):
                prog.append(["let", v, o])
        elif not mask:
            prog.append(["or", outs, [[["shared", i]] for i in idxs], 1])
        elif KEYED_MEMO:
            prog.append(["or-keyed", outs, [[["shared", i]] for i in idxs], [F[c] for c in mask], [args[c] for c in mask]])
        else:
            for c in mask:
                prog.append(["let", args[c], F[c]])
            prog.append(["or", outs, [[["shared", i]] for i in idxs]])
        bound.update(outs)
        prog.extend(post)

    def call_outs(self, args, free, tbound):
        """the registers a call's free columns bind (a column whose arg is already bound, or repeats, is tested after)"""
        outs, post = [], []
        for c in free:
            t = args[c]
            if tbound(t) or t in outs:
                tmp = self.fresh(self.tkind(t))
                outs.append(tmp)
                post.append(["test", "=", tmp, t])
            else:
                outs.append(t)
        return outs, post

    def subst_all(self, D, a, b):
        return [self.subst(x.copy(), {a: b}) for x in D]

    def outer_ready(self, bs, bound, E):
        """a sub-branch may run when every variable it shares with the enclosing branch is bound"""
        for x in bs:
            for t in deep_vars(x, set()):
                if t in E and t not in bound:
                    return False
        return True


def const_int(t):
    """a constant's integer value as the executor reads it (read_numeric: canonical integer text), or None"""
    if t[0] == "n" and isinstance(t[1], int):
        return t[1]
    if t[0] == "t" and isinstance(t[1], str):
        d = t[1][1:] if t[1].startswith("-") else t[1]
        if d and d.isascii() and d.isdigit() and not (len(d) > 1 and d[0] == "0"):
            return int(t[1])
    return None


def deep_vars(b, acc):
    """every variable of the branch and its sub-branches (stored sub-branches answer from their cache)"""
    collect_vars(b, acc)
    for s in b.specials:
        if s[0] == "near":
            for x in s[3]:
                acc.update(x.deep())
        if s[0] == "clos":
            for x in s[7]:
                acc.update(x.deep())
        if s[0] == "or":
            for x in s[2]:
                acc.update(x.deep())
        if s[0] == "nearc":
            acc.update(t for t in [s[3], s[4]] if is_var(t))
            for x in s[5] + s[6]:
                acc.update(x.deep())
    return acc


def special_vars(s):
    if s[0] == "fn":
        return list(s[2]) + list(s[3])
    if s[0] == "clos":
        return [s[1], s[2]] + list(s[3])
    if s[0] == "or":
        return list(s[1])
    if s[0] == "nearc":
        return [s[1], s[2]]
    if s[0] == "call":
        return [t for t in s[2] if t is not None]
    return list(s[1:4])


def collect_vars(b, acc):
    for _, ts in b.atoms:
        acc.update(t for t in ts if is_var(t))
    for _, l, r in b.filt:
        acc.update(t for t in (l, r) if is_var(t))
    acc.update(t for t in b.out if is_var(t))
    for s in b.specials:
        acc.update(t for t in special_vars(s) if is_var(t))
    for _, _, args in b.negs:
        acc.update(t for t in args if t is not None and is_var(t))
    return acc


EMPTY_PROPAGATES = {"antijoin": ("left",), "join": ("left", "right"), "project": ("from",), "select": ("from",),
                    "extend": ("from",), "intersection": ("left", "right"), "difference": ("left",),
                    "span_between": ("from",), "span_bounds": ("from",), "source_matches": ("from",), "group_count": ("from",),
                    "capture": ("from",), "replace": ("from",), "fits_rendered": ("from",), "words": ("from",),
                    "rank": ("from",), "axis": ("from",)}


def core_candidates(steps, emit_step):
    """steps whose emptiness makes the answer empty (so the tail may run only in files where they have rows), in the order
    tried: the usual core (the project under the emit antijoin's left) first, then breadth-first from the emit"""
    out = []
    e = steps[emit_step]
    if e["op"] == "antijoin":
        lp = steps[e["params"]["left"]["step"]]
        if lp["op"] == "project" and "step" in lp["params"]["from"]:
            out.append(lp["params"]["from"]["step"])
    queue, seen = [emit_step], set()
    while queue:
        s = queue.pop(0)
        if s in seen:
            continue
        seen.add(s)
        st = steps[s]
        if s != emit_step and s not in out and st.get("scope") != "cross-file":
            out.append(s)
        elif s == emit_step and st.get("scope") != "cross-file" and all("relation" in v for _, v in inputs(st["params"])):
            # an emit reading relations only: the whole rule is its core (navwide)
            out.append(s)
        for k in EMPTY_PROPAGATES.get(st["op"], ()):
            v = st["params"].get(k)
            if isinstance(v, dict) and "step" in v:
                queue.append(v["step"])
    return out


def tail_of(rule, steps, emit_step, X, xcols):
    need, stack = set(), [emit_step]
    while stack:
        s = stack.pop()
        if s in need:
            continue
        need.add(s)
        if s == X:
            continue
        for _, v in inputs(steps[s]["params"]):
            if "step" in v:
                stack.append(v["step"])
    tail = []
    bad = None
    aux = {}
    extra = []
    for s in sorted(need):
        st = steps[s]
        node = rule["step_nodes"][s]
        merged = node if node is not None else 10_000_000 + rule["ordinal"] * 10_000 + s
        if s == X:
            tail.append(dict(step=s, op="injected", merged=merged, kinds=xcols))
            continue
        params = dict(st["params"])
        if st["op"] == "join" and params.get("reach"):
            # a join against a closure: the closure as a step of its own (the interpreter's whole-graph closure), the join plain
            rc = params["reach"]
            cid = 200_000 + s
            extra.append(dict(step=cid, op="closure", merged=30_000_000 + rule["ordinal"] * 10_000 + s, edges=params["right"],
                              keys=rc["keys"], source=rc["source"], destination=rc["destination"]))
            right = {"step": cid}
            if list(rc["columns"]) != list(range(len(rc["columns"]))):
                pid = 300_000 + s
                extra.append(dict(step=pid, op="project", merged=40_000_000 + rule["ordinal"] * 10_000 + s, **{"from": right},
                                  columns=rc["columns"], distinct=True))
                right = {"step": pid}
            params["right"] = right
            params["reach"] = None
            ed = extra[-1 if len(extra) and extra[-1]["op"] == "closure" else -2]["edges"]
            if "relation" in ed and ed["relation"] not in TAIL_RELATIONS:
                rel = ed["relation"]
                if rel not in aux:
                    aux[rel] = dict(relation=rel, step=100_000 + len(aux), merged=20_000_000 + rule["ordinal"] * 100 + len(aux))
                for x in extra:
                    if x["op"] == "closure" and x["edges"] == ed:
                        x["edges"] = {"step": aux[rel]["step"]}
            st = dict(st, params=params)
        why = tail_unsupported(st)
        if why:
            bad = bad or why
        for k, v in inputs(st["params"]):
            if "relation" in v and v["relation"] not in TAIL_RELATIONS:
                # a relation the tail's interpreter does not serve: its rows come from a navigation program of their own
                # (a scan of the relation), injected like the core (navwide)
                rel = v["relation"]
                if rel not in aux:
                    aux[rel] = dict(relation=rel, step=100_000 + len(aux), merged=20_000_000 + rule["ordinal"] * 100 + len(aux))
                params[k] = {"step": aux[rel]["step"]}
        tail.append(dict(step=s, op=st["op"], merged=merged, **params))
    tail.extend(extra)
    for a in aux.values():
        tail.append(dict(step=a["step"], op="injected", merged=a["merged"], kinds=[]))
    return tail, (list(aux.values()) if aux else None), bad


TAIL_OPS = {"select", "project", "union", "intersection", "difference", "join", "antijoin", "extend", "axis", "source_matches",
            "span_bounds", "span_between", "group_count", "closure", "capture", "replace", "ordered_match"}
TAIL_TESTS = {"=", "!=", "<", "<=", ">", ">=", "in", "not-in", "glob-any", "not-glob-any", "matches", "starts-with", "ends-with"}


def tail_unsupported(st):
    """what of a tail step the walk interpreter cannot run (it would drop the rule), or None (navwide)"""
    op, p = st["op"], st["params"]
    if op not in TAIL_OPS:
        return "tail-op-" + op
    if op == "axis" and p["direction"] not in ("parent", "child", "ancestor", "descendant", "nearest-ancestor"):
        return "tail-axis-" + p["direction"]
    if op == "join" and p.get("reach"):
        return "tail-join-reach"
    comps = p.get("comparisons") or p.get("filter") or p.get("domain_filter") or []
    for c in comps:
        if c["test"] not in TAIL_TESTS:
            return "tail-test-" + c["test"]
        for side in ("left", "right"):
            if "function" in c[side]:
                return "tail-function-operand"
    return None


def lower_aux(aux, tail, schema, freq):
    """each relation the tail reads but its interpreter does not serve: a program scanning it, emitting every column"""
    out = []
    for a in aux:
        rel = a["relation"]
        if rel not in schema:
            raise Blocked("relation-" + rel)
        low = Lowerer(schema, freq)
        low.memo, low.plan_memo, low.uses_of, low.rid, low.made, low.schema = {}, {}, {}, "aux", [], dict(schema)
        low.nvar = 10_000_000 + 10_000 * len(out)
        cols = schema[rel]["columns"]
        ts = [("f", 0) if c["kind"] == "file" else low.fresh(c["kind"]) for c in cols]
        prog = low.plan_alts(Br([[rel, list(ts)]], out=list(ts)))[0]["prog"]
        kinds = [c["kind"] for c in cols]
        for st in tail:
            if st.get("step") == a["step"]:
                st["kinds"] = kinds
        out.append(dict(merged=a["merged"], programs=[prog], columns=kinds, kinds={str(k): v for k, v in low.kind.items()}))
    return out


def lower_core(rid, steps, X, schema, freq):
    low = Lowerer(schema, freq)
    low.steps = steps
    low.memo = {}
    low.plan_memo = {}
    low.uses_of = collections.Counter(v["step"] for st in steps for _, v in inputs(st["params"]) if "step" in v)
    low.rid = rid
    low.made = []
    low.schema = dict(schema)
    low.outer_vars = set()
    branches = low.template(X)
    if TOP_INLINE:
        branches = low.expand_top(branches)
    if low.too_big(branches) and not (TOP_INLINE and len(branches) <= BRANCH_CAP):
        branches = [low.cut(branches, ("step", X))]
    progs = []
    for b in branches:
        low.outer_vars = collect_vars(b, set())
        progs.append(low.plan_alts(b))
        if REQUIRE:
            low.require_alts(b, progs[-1])
    if not progs:
        raise Blocked("no-branch")
    LAST_STATS.update(defs=len(low.defs), bodies=low.n_bodies, shared=len(low.shared), vars=low.nvar,
                      template_steps=len(low.memo))
    return progs, low.made, dict(low.kind), low.shared


def lower_rule(rule, nodes, schema, freq):
    """one rule: (lowered rule or None, blocked reason or None)"""
    global UNION_CAP, BRANCH_CAP
    rid = rule["id"]
    plan = rule["plan"]
    params = {p["id"]: p.get("bound") for p in plan.get("parameters", [])}
    steps = [dict(s, params=bind(s["params"], params)) for s in plan["steps"]]
    emit_step = plan["emit"]["step"]
    if steps[emit_step].get("scope") == "cross-file":
        return None, "cross-file"
    size_cap = int(os.environ.get("NAV_SIZE_CAP", "3000000"))
    caps0 = (UNION_CAP, BRANCH_CAP)
    first_reason = None
    done = None
    deadline = time.time() + int(os.environ.get("NAV_RULE_SECONDS", "120"))
    cands = core_candidates(steps, emit_step)[:int(os.environ.get("NAV_CORES", "6"))]
    # cores whose tail the interpreter serves alone first, then those needing auxiliary scans (navwide)
    def needs_aux(X):
        xnode = rule["step_nodes"][X]
        return xnode is not None and bool(tail_of(rule, steps, emit_step, X, nodes[xnode]["columns"])[1])
    cands = [X for X in cands if not needs_aux(X)] + [X for X in cands if needs_aux(X)]
    for X in cands:
        xnode = rule["step_nodes"][X]
        if xnode is None:
            first_reason = first_reason or "core-node"
            continue
        xcols = nodes[xnode]["columns"]
        tail, aux, bad = tail_of(rule, steps, emit_step, X, xcols)
        if bad:
            first_reason = first_reason or bad
            continue
        for caps in (caps0, (8, 32)):
            UNION_CAP, BRANCH_CAP = caps
            left = deadline - time.time()
            if left < 1:
                first_reason = first_reason or "lowering-time"
                break
            signal.alarm(max(1, int(min(left, 60))))
            try:
                progs, made, kinds, shared = lower_core(rid, steps, X, schema, freq)
                navd = dict(x=X, xmerged=xnode, columns=xcols, programs=progs, made=made,
                            kinds={str(k): v for k, v in kinds.items()}, shared=shared)
                if aux:
                    navd["aux"] = lower_aux(aux, tail, schema, freq)
                if os.environ.get("NAV_SHARE", "1") == "1": share_rule(navd)
                prune_shared(navd)
                prune_kinds(navd)
                if len(json.dumps(navd)) > size_cap:
                    raise Blocked("program-size")
                done = (X, tail, navd)
            except Blocked as x:
                first_reason = first_reason or str(x)
                if str(x) == "program-size":
                    continue
            except RecursionError:
                first_reason = first_reason or "recursion"
            except TimeoutError:
                first_reason = first_reason or "lowering-time"
                continue
            finally:
                signal.alarm(0)
            break
        if done:
            break
    UNION_CAP, BRANCH_CAP = caps0
    if not done:
        return None, first_reason or "no-core"
    X, tail, navd = done
    std = core_candidates(steps, emit_step)[:1]
    navd["core_is_usual"] = bool(std) and std[0] == X
    return dict(id=rid, root=emit_step, emit_key=plan["emit"]["key"], steps=tail, nav=navd), None


_G = {}
LAST_STATS = {}
STATS = {}


def _init(plans_path, schema_path, freq_path):
    plans = json.load(open(plans_path))
    _G["rules"] = {r["id"]: r for r in plans["rules"]}
    _G["nodes"] = plans["program"]["nodes"]
    _G["schema"] = json.load(open(schema_path))["relations"]
    _G["freq"] = json.load(open(freq_path))


def _one(rid):
    sys.setrecursionlimit(20000)
    t = time.time()
    c0 = time.process_time()
    LAST_STATS.clear()
    r, why = lower_rule(_G["rules"][rid], _G["nodes"], _G["schema"], _G["freq"])
    js = json.dumps(r) if r else None
    STATS[rid] = dict(LAST_STATS, cpu_s=round(time.process_time() - c0, 3), program_bytes=len(js) if js else 0)
    if os.environ.get("NAV_STATS"):
        print(rid, json.dumps(STATS[rid]), why, file=sys.stderr)
    return rid, js, why, time.time() - t


def _one_pool(rid):
    rid, js, why, secs = _one(rid)
    return rid, js, why, secs, STATS.get(rid, {})


def main():
    wanted = [l.strip() for l in open(sys.argv[4]) if l.strip() and not l.startswith("#")]
    jobs = int(os.environ.get("NAV_JOBS", "1"))
    results = {}
    if jobs > 1:
        import multiprocessing as mp
        _init(sys.argv[1], sys.argv[2], sys.argv[3])
        wanted = [r for r in wanted if r in _G["rules"]]
        with mp.get_context("fork").Pool(jobs) as pool:
            for rid, r, why, secs, st in pool.imap_unordered(_one_pool, wanted):
                results[rid] = (r, why, secs)
                STATS[rid] = st
                print(rid, round(secs, 1), why or "", file=sys.stderr, flush=True)
    else:
        _init(sys.argv[1], sys.argv[2], sys.argv[3])
        for rid in wanted:
            if rid in _G["rules"]:
                _, r, why, secs = _one(rid)
                results[rid] = (r, why, secs)
    out_rules, blocked = [], {}
    for rid in wanted:
        if rid not in results:
            continue
        r, why, secs = results[rid]
        if r:
            out_rules.append(json.loads(r))
        else:
            blocked[rid] = why
    stats = collections.Counter(blocked.values())
    json.dump(dict(rules=out_rules), open(sys.argv[5], "w"))
    json.dump(dict(blocked=blocked, counts=stats.most_common(), seconds={k: round(v[2], 2) for k, v in results.items()}, stats=STATS),
              open(sys.argv[6], "w"), indent=1)
    print(f"lowered {len(out_rules)} of {len(wanted)}; blocked {len(blocked)}: {stats.most_common()}", file=sys.stderr)


# ------------------------------------------------------------------ program clean-up: common reads, dead reads
def uses(ins):
    op = ins[0]
    if op == "test":
        return [t for t in ins[2:4] if is_var(t)]
    if op == "read":
        return [ins[3]]
    if op == "row":
        return [ins[2]]
    if op == "iter":
        arg = ins[3]
        if ins[2] == "children":
            return [arg] if is_var(arg) else []
        if isinstance(arg, list) and len(arg) == 2 and is_var(arg[1]):
            return [arg[1]]
        return []
    if op == "axis":
        return [ins[2]]
    if op == "near":
        u = [ins[1]]
        for p in ins[3]:
            u += prog_uses(p)
        return u
    if op in ("neg", "exists", "neg-memo", "exists-memo", "neg-keyed", "exists-keyed", "neg-probe", "exists-probe"):
        u = []
        for p in ins[1]:
            u += prog_uses(p)
        if op in ("neg-keyed", "exists-keyed", "neg-probe", "exists-probe") and len(ins) > 3:
            u += [t for t in ins[3] if is_var(t)]  # cut: the args a keyed test passes, the keys a probe looks up
        return u
    if op == "emit":
        return [t for t in ins[1] if is_var(t)]
    if op == "fn":
        return [t for t in ins[2] if is_var(t)]
    if op == "collect":
        return [t for t in ins[1] if is_var(t)]
    if op == "let":
        return [ins[1]] if is_var(ins[1]) else []
    if op == "sib":
        return [ins[2]]
    if op == "clos":
        u = [ins[1]]
        for p in ins[7]:
            u += prog_uses(p)
        return u
    if op in ("or", "or-keyed", "or-probe"):
        u = []
        for p in ins[2]:
            u += prog_uses(p)
        if op in ("or-keyed", "or-probe") and len(ins) > 4:
            u += [t for t in ins[4] if is_var(t)]  # cut: the args a keyed call passes, the keys a probe looks up
        return u
    if op == "nearc":
        u = [ins[1]]
        for p in ins[5] + ins[6]:
            u += prog_uses(p)
        return u
    return []


def prog_uses(p):
    u = []
    for ins in p:
        u += uses(ins)
    return u


def substitute(p, m):
    def f(t):
        while is_var(t) and t in m:
            t = m[t]
        return t
    out = []
    for ins in p:
        op = ins[0]
        if op == "test":
            ins = ["test", ins[1], f(ins[2]), f(ins[3])]
        elif op == "read":
            ins = ["read", ins[1], ins[2], f(ins[3]), ins[4]]
        elif op == "row":
            ins = ["row", ins[1], f(ins[2])]
        elif op == "iter":
            arg = ins[3]
            if ins[2] == "children":
                arg = f(arg)
            elif isinstance(arg, list) and len(arg) == 2 and is_var(arg[1]):
                arg = [arg[0], f(arg[1])]
            ins = ["iter", ins[1], ins[2], arg, ins[4]]
        elif op == "axis":
            ins = ["axis", ins[1], f(ins[2]), ins[3]]
        elif op == "near":
            ins = ["near", f(ins[1]), ins[2], [substitute(q, m) for q in ins[3]]]
        elif op in ("neg", "exists", "neg-memo", "exists-memo"):
            ins = [op, [substitute(q, m) for q in ins[1]]]
        elif op in ("neg-keyed", "exists-keyed", "neg-probe", "exists-probe"):
            ins = [op, [substitute(q, m) for q in ins[1]], ins[2]] + ([[f(t) for t in ins[3]]] + ins[4:] if len(ins) > 3 else [])
        elif op == "emit":
            ins = ["emit", [f(t) for t in ins[1]]]
        elif op == "fn":
            ins = ["fn", ins[1], [f(t) for t in ins[2]], ins[3], ins[4]]
        elif op == "collect":
            ins = ["collect", [f(t) for t in ins[1]]]
        elif op == "let":
            ins = ["let", f(ins[1]), ins[2]]
        elif op == "sib":
            ins = ["sib", ins[1], f(ins[2]), ins[3]]
        elif op == "clos":
            ins = ["clos", f(ins[1]), ins[2], ins[3], ins[4], ins[5], ins[6], [substitute(q, m) for q in ins[7]]]
        elif op in ("or", "or-keyed", "or-probe"):
            ins = [op, ins[1], [substitute(q, m) for q in ins[2]]] + ins[3:]
            if op in ("or-keyed", "or-probe") and len(ins) > 4:
                ins[4] = [f(t) for t in ins[4]]
        elif op == "nearc":
            ins = ["nearc", f(ins[1]), ins[2], ins[3], ins[4], [substitute(q, m) for q in ins[5]], [substitute(q, m) for q in ins[6]]]
        out.append(ins)
    return out


def cse(p, seen):
    seen = dict(seen)
    out = []
    m = {}
    i = 0
    p = list(p)
    while i < len(p):
        ins = p[i]
        if m:
            ins = substitute([ins], m)[0]
        op = ins[0]
        if op == "read":
            key = (ins[1].replace("syntax-tree", "semantic-syntax").replace("syntax-child", "semantic-child"), ins[2], ins[3])
            if key in seen:
                m[ins[4]] = seen[key]
                i += 1
                continue
            seen[key] = ins[4]
        elif op == "row":
            key = ("row", ins[1].replace("syntax-tree", "semantic-syntax").replace("syntax-child", "semantic-child"), ins[2])
            if ins[1] in TOTAL or key in seen:
                i += 1
                continue
            seen[key] = True
        elif op == "test" and ins[1] == "=" and ins[2] == ins[3]:
            i += 1
            continue
        elif op == "test":
            key = ("test", ins[1], json.dumps(ins[2]), json.dumps(ins[3]))
            if key in seen:
                i += 1
                continue
            seen[key] = True
        elif op == "near":
            ins = ["near", ins[1], ins[2], [cse(q, seen) for q in ins[3]]]
        elif op in ("neg", "exists"):
            ins = [op, [cse(q, seen) for q in ins[1]]]
        elif op in ("neg-memo", "exists-memo"):
            ins = [op, [cse(q, {}) for q in ins[1]]]
        elif op in ("neg-keyed", "exists-keyed", "neg-probe", "exists-probe"):
            ins = [op, [cse(q, {}) for q in ins[1]]] + ins[2:]
        elif op == "clos":
            ins = ins[:7] + [[cse(q, seen) for q in ins[7]]]
        elif op in ("or", "or-keyed", "or-probe"):
            ins = [op, ins[1], [cse(q, seen) for q in ins[2]]] + ins[3:]
        elif op == "nearc":
            ins = ins[:5] + [[cse(q, seen) for q in ins[5]], [cse(q, seen) for q in ins[6]]]
        out.append(ins)
        i += 1
    return out


def dce(p, live_after):
    live = set(live_after)
    out = []
    for ins in reversed(p):
        op = ins[0]
        if op == "read" and ins[4] not in live:
            continue
        if op == "fn" and ins[1] not in ("source_matches", "capture") and not any(t in live for t in ins[3]):
            continue
        if op == "near":
            ins = ["near", ins[1], ins[2], [dce(q, live | {ins[2]}) for q in ins[3]]]
        elif op in ("neg", "exists", "neg-memo", "exists-memo"):
            ins = [op, [dce(q, live) for q in ins[1]]]
        elif op in ("neg-keyed", "exists-keyed", "neg-probe", "exists-probe"):
            ins = [op, [dce(q, live) for q in ins[1]]] + ins[2:]
        elif op == "clos":
            ins = ins[:7] + [[dce(q, live) for q in ins[7]]]
        elif op in ("or", "or-keyed", "or-probe"):
            ins = [op, ins[1], [dce(q, live) for q in ins[2]]] + ins[3:]
        elif op == "nearc":
            ins = ins[:5] + [[dce(q, live) for q in ins[5]], [dce(q, live | {ins[2]}) for q in ins[6]]]
        live.update(uses(ins))
        out.append(ins)
    return list(reversed(out))


def clean(p):
    return dce(cse(p, {}), set())


def prune_kinds(nav):
    """only the kinds of variables the programs name (lower: definitions mint many variables planning never keeps)"""
    used = set()

    def walk(x):
        if isinstance(x, list):
            for y in x:
                walk(y)
        elif isinstance(x, dict):
            for y in x.values():
                walk(y)
        elif isinstance(x, int) and not isinstance(x, bool):
            used.add(str(x))
    walk(nav["programs"])
    walk(nav["made"])
    walk(nav.get("shared", []))
    walk(nav.get("aux", []))
    nav["kinds"] = {k: v for k, v in nav["kinds"].items() if k in used}


def clean_sub(p, formals):
    return dce(cse(p, {}), set())


# ------------------------------------------------------------------ shared sub-programs (navwide)
SUBS_AT = {"neg": (1,), "exists": (1,), "neg-memo": (1,), "exists-memo": (1,), "neg-keyed": (1,), "exists-keyed": (1,), "or": (2,), "or-keyed": (2,), "clos": (7,), "near": (3,), "nearc": (5, 6),
           "or-probe": (2,), "neg-probe": (1,), "exists-probe": (1,)}


def prune_shared(nav):
    """cut: only the shared entries a program reaches are kept (plans tried and dropped, and patterns costed but not
    used, leave entries behind), renumbered"""
    table = nav.get("shared") or []
    keep, order = set(), []

    def walk(p):
        for ins in p:
            if ins[0] == "shared":
                if ins[1] not in keep:
                    keep.add(ins[1])
                    order.append(ins[1])
                    walk(table[ins[1]])
            for k in SUBS_AT.get(ins[0], ()):
                for q in ins[k]:
                    walk(q)
    roots = [a["prog"] for alts in nav["programs"] for a in alts] + [q for m in nav["made"] for q in m["programs"]] + \
        [q for m in nav["made"] for q in m.get("right_programs", [])] + [q for a in nav.get("aux", []) for q in a["programs"]]
    for p in roots:
        walk(p)
    new = {old: i for i, old in enumerate(sorted(keep))}

    def tr(p):
        out = []
        for ins in p:
            if ins[0] == "shared":
                ins = ["shared", new[ins[1]]]
            else:
                ks = SUBS_AT.get(ins[0], ())
                if ks:
                    ins = list(ins)
                    for k in ks:
                        ins[k] = [tr(q) for q in ins[k]]
            out.append(ins)
        return out
    for alts in nav["programs"]:
        for a in alts:
            a["prog"] = tr(a["prog"])
    for m in nav["made"]:
        m["programs"] = [tr(q) for q in m["programs"]]
        if "right_programs" in m:
            m["right_programs"] = [tr(q) for q in m["right_programs"]]
    for a in nav.get("aux", []):
        a["programs"] = [tr(q) for q in a["programs"]]
    nav["shared"] = [tr(table[old]) for old in sorted(keep)]


def share_rule(nav, min_bytes=300):
    """sub-programs that occur more than once in a rule's programs become entries of the rule's shared table; each use is
    the one-step program [["shared", k]] (the executor runs the entry in place)"""
    counts = collections.Counter()

    def count(p):
        for ins in p:
            for k in SUBS_AT.get(ins[0], ()):
                for q in ins[k]:
                    counts[json.dumps(q)] += 1
                    count(q)

    base = nav.get("shared") or []
    progs = [a["prog"] for alts in nav["programs"] for a in alts] + [q for m in nav["made"] for q in m["programs"]] + \
        [q for m in nav["made"] for q in m.get("right_programs", [])] + list(base)
    for p in progs:
        count(p)
    table, index = list(base), {}

    def tr(p):
        out = []
        for ins in p:
            for k in SUBS_AT.get(ins[0], ()):
                subs = []
                for q in ins[k]:
                    key = json.dumps(q)
                    t = tr(q)
                    if counts[key] >= 2 and len(key) >= min_bytes:
                        if key not in index:
                            index[key] = len(table)
                            table.append(t)
                        subs.append([["shared", index[key]]])
                    else:
                        subs.append(t)
                ins = list(ins)
                ins[k] = subs
            out.append(ins)
        return out

    for alts in nav["programs"]:
        for a in alts:
            a["prog"] = tr(a["prog"])
    for m in nav["made"]:
        m["programs"] = [tr(q) for q in m["programs"]]
        if "right_programs" in m:
            m["right_programs"] = [tr(q) for q in m["right_programs"]]
    for i in range(len(base)):
        table[i] = tr(base[i])
    nav["shared"] = table


if __name__ == "__main__":
    sys.setrecursionlimit(20000)
    main()


