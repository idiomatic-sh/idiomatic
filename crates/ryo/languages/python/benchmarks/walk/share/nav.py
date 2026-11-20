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
# the probe's build is weighed against a tenth of the candidates the plan expects: estimates of candidates err upward
# (most files hold none), a keyed call costs nothing without one, the build is paid whole once one arrives
PROBE_BIAS = float(os.environ.get("NAV_PROBE_BIAS", "0.1"))
# whether a probe may be PREFERRED to a keyed call on the cost estimate (off: a probe is taken only when the definition has
# no binding pattern left, where last round scanned its memoised rows per candidate). Measured: preferring it gained nothing on
# the 341 (0.697G without, 0.706G with) and its build is unbounded when the estimate of candidates errs: UP034 built an
# all-pairs closure of the token chain once in one file, 4 x 289M steps, for a call that a keyed walk answers in a few.
PROBE_PREFER = os.environ.get("NAV_PROBE_PREFER", "1") == "1"
# ... so a probe is preferred only when the definition's uncorrelated sub-program walks no closure and no nearest-over-edges
# (PROBE_SAFE): then its rows are bounded by the buckets it iterates, which the estimate does see.
PROBE_SAFE = os.environ.get("NAV_PROBE_SAFE", "1") == "1"
# a bucket iterated inside a candidate keeps its penalty (BUCKET_X) as its RANK only; the plan's cost and rows use its size, so
# plans from different anchors compare on what they would run (last round the penalty also multiplied the rows after it)
RANK_SPLIT = os.environ.get("NAV_RANK_SPLIT", "1") == "1"
# a literal of a symbol column with no value counts (more than 256 distinct values on the corpus): at most the column's
# rows over 256 on average (it was 1% of the rows), and no 10x inside a candidate (a specific name, not a class of rows)
RARE_LITERAL = os.environ.get("NAV_RARE_LITERAL", "1") == "1"
# the item bound cuts one BRANCH of a joined pair (a definition of one branch), not a whole side: every joined branch keeps its
# own anchor choice (a side cut whole hides the selective anchors of some of its branches behind the wide ones of others)
PAIR_CUT = os.environ.get("NAV_PAIR_CUT", "0") == "1"
# a step within the branch bound whose flattening has branches over the item bound: only those branches become uses of
# one-branch definitions; the others stay inline with their own anchors (last round the whole step became one use)
BRANCH_CUT = os.environ.get("NAV_BRANCH_CUT", "1") == "1"
# a test that costs this many steps or more waits for a keyed fact lookup (at most about one row: rank up to TEST_WAIT_RANK)
# that may drop the candidate first; a cheaper test runs as soon as its args are bound (as before)
TEST_WAIT_STEPS = float(os.environ.get("NAV_TEST_WAIT_STEPS", "10"))
TEST_WAIT_RANK = float(os.environ.get("NAV_TEST_WAIT_RANK", "1.35"))
# a test of TEST_WAIT_STEPS2 steps or more also waits for an item ranked up to TEST_WAIT_RANK2 (a regex over the source, a
# children iteration with a field literal): 0 = off
TEST_WAIT_STEPS2 = float(os.environ.get("NAV_TEST_WAIT_STEPS2", "20"))
TEST_WAIT_RANK2 = float(os.environ.get("NAV_TEST_WAIT_RANK2", "2.05"))
# a closure walked along tree child edges is a descendant walk (30, as the axis); walked the other way an ancestor walk (8);
# last round every closure cost 8, so a test that walks all descendants looked cheaper than it runs
CLOS_COST = os.environ.get("NAV_CLOS_COST", "1") == "1"
# a nearest-over-derived-edges walk whose reached end is bound and whose start is not runs from the reached end: back over the
# edges through nodes the domain does not admit (the only starts whose nearest admitted node can be the reached one), each
# such start then checked by the forward walk. Before, the start had to come from another atom (every token of the file).
NEARC_INV = os.environ.get("NAV_NEARC_INV", "1") == "1"
# an equality between two variables of which one is bound binds the other at once (a `let`), before any atom is reached through
# a scan and tested after (an identity edge walked from its destination scanned every node per closure step)
EAGER_EQ = os.environ.get("NAV_EAGER_EQ", "1") == "1"
# a closure whose two ends are both bound walks from the cheaper end (up the tree rather than over all descendants)
CLOS_BOTH = os.environ.get("NAV_CLOS_BOTH", "1") == "1"
CLOS_LIT = os.environ.get("NAV_CLOS_LIT", "1") == "1"
# a fact relation (a made table included) reached with two or more of its columns bound is looked up through a composite
# index over all of them (executor: IterFactMulti), not through one column's index with the others tested per row
MULTI_KEY = os.environ.get("NAV_MULTI_KEY", "1") == "1"
# a made table (a group count) holds one row per value of its key columns: reached with every key column bound it is one row
# (1.0); with only some of them it is several rows of unknown number (MADE_PARTIAL, dearer than a walk up the tree). Before,
# any bound node column cost 1.3, so "every descendant of this root" looked like one row.
MADE_PARTIAL = float(os.environ.get("NAV_MADE_PARTIAL", "10"))
# the chance that a candidate reaches a made table's first read (and builds it) is (candidates expected there) ** MADE_P_EXP,
# at most 1: 1 takes the planner's count at face value
MADE_P_EXP = float(os.environ.get("NAV_MADE_P_EXP", "1"))
# a top-level plan is tried from the ANCHOR_TOP anchors with the smallest start (bucket size, or a definition's cheapest
# anchor), not from every atom: planning is (anchors tried) x (items squared) accesses, the dominant lowering cost
ANCHOR_TOP = int(os.environ.get("NAV_ANCHOR_TOP", "4"))
# the first read of a made table in a plan builds it for the file: it ranks after the ready tests (MADE_FIRST_RANK, above
# TEST_WAIT_RANK and below a children iteration), so a test that drops the candidate spares the build
MADE_FIRST_RANK = float(os.environ.get("NAV_MADE_FIRST_RANK", "0"))  # tried at 1.4: C411 -2%, ISC004 x15.8, ASYNC100 x13.6; off
# the branches of one definition (and the programs of one made table) that start with the same instructions run that
# prefix once and fork after it (factor_prefix): a union of variants of one shape no longer walks its anchor once per variant
PREFIX = os.environ.get("NAV_PREFIX", "1") == "1"
TOP_PREFIX = os.environ.get("NAV_TOP_PREFIX", "1") == "1"   # the same for the core's own branches (merge_top)
COST_CAP, ROWS_CAP = 1e30, 1e15   # the planner's estimates never leave the finite range
# bind: the cell kinds of a fact relation's column a bound value may be looked up by. `span` was missing: a join on a span
# column (a definition's name span against an identifier's) had no keyed access and was answered by enumerating the other side
KEY_KINDS = ("node", "symbol", "number", "span") if os.environ.get("NAV_SPAN_KEY", "1") == "1" else ("node", "symbol", "number")
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
    return type(t) is int


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
        return ["or", list(s[1]), list(s[2])] + list(s[3:])
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
        self.tableable_memo = {}  # bind: definition -> whether it is held as one table per file
        self.way_memo = {}        # bind: (definition, bound columns, collected columns) -> how the use is evaluated
        self.flag_scan = False    # bind: the program being planned iterates a bucket, a scan or a table inside a bound context
        self.flag_pairs = False   # bind: the program being planned walks a closure with neither end bound (all pairs)
        self.body_scans, self.body_pairs = {}, {}
        self.table_memo = {}  # asof: definition -> the shared entry of its whole-table program (False: not plannable)
        self.fired, self.keyed = [], {}  # asof: recognitions that fired; made tables with keyed access (name -> key columns)
        self.shared_meta = []  # asof (diagnostic): per shared entry, the definition key, the bound columns, the collected columns
        self.walk_memo = {}
        self.all_cols_memo = {}
        # cut: per definition program (key, mask, free): (steps per call, rows per call) from its plan, and its branch entries
        self.body_cost, self.body_branches = {}, {}
        self.mult = 1.0          # calls per file of the sub-program being planned (amortises one-off costs)
        self.last_card = 1.0     # rows the last planned program ends with
        self.top_cost = self.top_card = 0.0
        self.made_cost = {}      # made table -> steps to build it once per file
        self.made_keys = {}      # made table (group count) -> its key columns
        self.anchor_memo = {}
        self.require_memo = {}
        # share: ONE lowerer for the pack. What a definition is does not depend on the rule that reaches it, so everything
        # keyed by a definition (its branches, formals, programs per binding pattern, costs, whether it is a table) and
        # every made table is kept across rules; what is a rule's own is set by `begin`
        self.made_all, self.made_by_node, self.nmade, self.serial = {}, {}, 0, 0
        self.body_serial, self.reused, self.planned, self.body_depth, self.body_cpu = {}, set(), 0, 0, 0.0

    def begin(self, rid, steps, step_nodes):
        """share: the state of one attempt at one rule"""
        self.rid, self.steps, self.step_nodes = rid, steps, step_nodes
        self.memo, self.plan_memo = {}, {}
        self.uses_of = collections.Counter(v["step"] for st in steps for _, v in inputs(st["params"]) if "step" in v)
        self.made = []
        self.outer_vars = set()
        self.masks, self.way_memo = {}, {}
        self.n_bodies = 0
        self.mult, self.last_card, self.top_cost, self.top_card = 1.0, 1.0, 0.0, 0.0
        self.flag_scan = self.flag_pairs = False
        self.fired = []
        self.serial += 1
        self.new_from = len(self.shared)
        self.reused, self.planned, self.body_depth, self.body_cpu = set(), 0, 0, 0.0

    def ck(self, kind, s, *extra):
        """share: the pack-wide name of a definition taken from plan step `s`: the compiler's own merged-program node of the
        step (one node is one sub-plan: operation, constants, inputs' nodes) with what the lowering took of it (its rows,
        its union, one side of its join). Two rules that reach the same node name the same definition, so it is lowered
        once and the executor holds one table of it per file. A name that is not a function of the node alone (branch i of
        a flattening, whose shape depends on the rule's other uses and the caps) stays this attempt's own."""
        if all(isinstance(e, str) for e in extra):
            return (kind, self.step_nodes[s]) + extra
        return (kind, ("own", self.serial, s)) + extra

    def forget(self):
        """share: every definition is dropped (an attempt was cut off part-way, or is retried under smaller bounds); the
        programs already written stay valid, later rules lower what they reach again"""
        for m in (self.defs, self.formal, self.body_memo, self.tableable_memo, self.body_scans, self.body_pairs, self.table_memo,
                  self.all_cols_memo, self.body_cost, self.body_branches, self.anchor_memo, self.require_memo, self.made_by_node,
                  self.heavy_memo, self.walk_memo, self.body_serial):
            m.clear()

    def made_hit(self, op, s):
        """share: the made table of the node of step `s` when a rule before made it: one atom over it"""
        name = self.made_by_node.get((op, self.step_nodes[s]))
        if name is None:
            return None
        ts = [("f", 0) if c["kind"] == "file" else self.fresh(c["kind"]) for c in self.schema[name]["columns"]]
        return [Br([[name, list(ts)]], out=list(ts))]

    def made_name(self):
        self.nmade += 1
        return f"made:{self.nmade}"

    def made_done(self, op, s, name):
        self.made_all[name] = self.made[-1]
        self.made_by_node[(op, self.step_nodes[s])] = name

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
                sp.append(["or", [f(t) for t in s[1]], [self.subst_sub(b, m) for b in s[2]]] + list(s[3:]))
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
                return [self.cut(bs, self.ck("union", s))]
            return bs
        if op == "closure":
            return self.closure_br(self.input(p["edges"]), p["keys"], p["source"], p["destination"])
        if op == "join":
            hit = self.asof_try(s) if ASOF else None  # asof: a doubling chain / an as-of lookup as one keyed made table
            if hit is not None:
                return hit
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
                    R_ = [self.cut(R_, self.ck("join", s, "right"))]
                else:
                    L = [self.cut(L, self.ck("join", s, "left"))]
            if JOIN_ITEM_CUT and not PAIR_CUT and L and R_ and max(x.size() for x in L) + max(x.size() for x in R_) > ITEM_CAP:
                # the joined branch would exceed the item bound: one side becomes a definition's use (cut: the side
                # without the cheapest anchor, see cut_right)
                if self.cut_right(L, R_, lambda bs: max(x.size() for x in bs)):
                    R_ = [self.cut(R_, self.ck("join", s, "right"))]
                else:
                    L = [self.cut(L, self.ck("join", s, "left"))]
            out = []
            cut_l, cut_r = {}, {}
            for li, bl in enumerate(L):
                for ri, br0 in enumerate(R_):
                    if JOIN_ITEM_CUT and PAIR_CUT and bl.size() + br0.size() > ITEM_CAP:
                        # cut: this joined pair would exceed the item bound: one of its two branches becomes the use of a
                        # one-branch definition (the one without the cheapest anchor), stored once per branch
                        if self.cut_right([bl], [br0], lambda bs: max(x.size() for x in bs)):
                            if ri not in cut_r:
                                cut_r[ri] = self.cut([br0], self.ck("join", s, "right", ri))
                            br0 = cut_r[ri]
                        else:
                            if li not in cut_l:
                                cut_l[li] = self.cut([bl], self.ck("join", s, "left", li))
                            bl = cut_l[li]
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
            hit = self.made_hit("group_count", s)
            if hit is not None:
                return hit
            # navwide: a bag input counts as its distinct rows (a bag projection keeps a column that identifies the row)
            sub = self.input(p["from"])
            name = self.made_name()
            progs, mcost, mrows = self.made_plan(sub)
            kinds = []
            for t in sub[0].out if sub else []:
                pass
            kk = [self.kind.get(sub[0].out[k], "symbol") if is_var(sub[0].out[k]) else {"t": "symbol", "n": "number", "f": "file"}.get(sub[0].out[k][0], "symbol") for k in p["keys"]] if sub else []
            cols = [dict(name=f"k{i}", kind=k, key=False) for i, k in enumerate(kk)] + [dict(name="count", kind="number", key=False)]
            self.schema[name] = dict(columns=cols, rows=2873 * 5)
            self.made_size(name, mcost, mrows)
            self.made_keys[name] = set(i for i, k in enumerate(kk) if k != "file")
            self.made.append(dict(name=name, programs=progs, keys=list(p["keys"]), distinct=p["distinct"], width=len(sub[0].out) if sub else 0,
                                  kinds=[c["kind"] for c in cols]))
            self.made_done("group_count", s, name)
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
            hit = self.made_hit("ordered_match", s)
            if hit is not None:
                return hit
            L = self.input(p["left"])
            R = self.input(p["right"])
            if not L or not R:
                return []
            name = self.made_name()
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
            self.made_done("ordered_match", s, name)
            ts = [("f", 0) if k == "file" else self.fresh(k) for k in kinds]
            return [Br([[name, list(ts)]], out=list(ts))]
        if op == "ordered_dfs":
            # a made table: per file, the roots' and the edges' rows (programs of their own), walked depth-first in order
            hit = self.made_hit("ordered_dfs", s)
            if hit is not None:
                return hit
            L = self.input(p["from"])
            E = self.input(p["edges"])
            if not L or not E:
                return []
            name = self.made_name()
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
            self.made_done("ordered_dfs", s, name)
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
            progs.append(self.bulk_prog(b))
            cost += self.top_cost
            rows += self.top_card
        if PREFIX and len(progs) > 1:
            fp, n_shared = factor_prefix(progs, False)
            if n_shared:
                progs = [fp]
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
            k = self.ck("step", v["step"])
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
            if self.body_serial.get(k, self.serial) != self.serial:
                self.reused.add(k)  # share: a program a rule before planned
            return hit
        self.body_memo[k] = []  # a definition never reaches itself (the plan is acyclic)
        if BODY_UNDO:
            # asof: a plan that fails part-way (an anchor the planner tries and gives up: `plan_top` catches Blocked and goes
            # on) must not leave this empty list as the definition's program: every later use would call a definition with
            # no branches (an absence test that always holds). A hazard closed, not an observed fault: F821's extra
            # findings of the last round are NOT this (bind: they were the executor's u16 register wrap, see nav.rs `R`).
            try:
                return self.body_planned(key, mask, free, k, calls)
            except BaseException:
                self.body_memo.pop(k, None)
                raise
        return self.body_planned(key, mask, free, k, calls)

    def body_planned(self, key, mask, free, k, calls):
        saved_mult, self.mult = self.mult, (max(calls * 1.0, 1e-3) if mask else 1.0)
        saved_flags, self.flag_scan, self.flag_pairs = (self.flag_scan, self.flag_pairs), False, False
        t0 = time.process_time() if self.body_depth == 0 else 0.0
        self.body_depth += 1
        try:
            return self.body_planned2(key, mask, free, k, saved_mult)
        finally:
            self.body_depth -= 1
            if self.body_depth == 0:
                self.body_cpu += time.process_time() - t0  # share (diagnostic): CPU planning definition programs
            self.body_scans[k], self.body_pairs[k] = self.flag_scan, self.flag_pairs
            self.flag_scan, self.flag_pairs = saved_flags

    def body_planned2(self, key, mask, free, k, saved_mult):
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
            entry = clean_sub(prog, set(F[c] for c in mask))
            if NEED:
                entry = self.with_need(B, entry)  # bind
            self.shared.append(entry)
            self.shared_meta.append([list(key), list(mask), None if free is None else list(free), "branch"])
            idxs.append(len(self.shared) - 1)
        self.mult = saved_mult
        tc = tc if tc < COST_CAP else COST_CAP
        tn = tn if tn < ROWS_CAP else ROWS_CAP
        self.body_cost[k] = (tc, tn if free is not None else min(tn, 1.0))
        self.body_branches[k] = list(idxs)
        if len(idxs) > 1:
            # one entry runs every branch (each pushes its rows, or the first that holds answers the test): every call
            # site names the definition once, not once per branch
            group = None
            if PREFIX:
                # cut: branches that start alike share that start (the fork runs each rest in turn)
                fp, n_shared = factor_prefix([self.shared[i] for i in idxs], free is None)
                if n_shared:
                    group = fp
            if group is None:
                group = [["exists", [[["shared", i]] for i in idxs]]] + ([["yield"]] if free is None else [])
            self.shared.append(group)
            self.shared_meta.append([list(key), list(mask), None if free is None else list(free), "group"])
            idxs = [len(self.shared) - 1]
        self.body_memo[k] = idxs
        self.body_serial[k] = self.serial
        self.planned += 1
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
                    return [self.cut(T, self.ck("step", s))]
                bs = [self.rename(b) for b in T]
            if (JOIN_ITEM_CUT or len(bs) > UNION_CAP) and self.too_big(bs):
                if BRANCH_CUT and 1 < len(bs) <= UNION_CAP and self.uses_of.get(s, 0) <= 1:
                    return [self.cut([b], self.ck("step", s, i)) if b.size() > ITEM_CAP else b for i, b in enumerate(bs) if b is not None]
                return [self.cut(bs, self.ck("step", s))]
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
        if RARE_LITERAL:
            return rows / 256.0 * len(vals)
        return rows * 0.01 * len(vals)

    def rare_literal(self, rel, col, val):
        """a text literal of a fact column the corpus statistics hold no value counts for (a high-cardinality column)"""
        return RARE_LITERAL and rel not in TREE and val[0] in ("t", "ts") and \
            self.freq.get("facts", {}).get(rel, {}).get(str(col)) is None

    def plan(self, b, bound, outvars, sub):
        if sub:
            # a sub-program depends only on the branch and which of its variables are bound: planned once per rule
            key = (id(b), frozenset(v for v in b.deep() if v in bound), json.dumps(sub) if isinstance(sub, list) else sub)
            hit = self.plan_memo.get(key)
            if hit is None or hit[0] is not b:
                saved, self.flag_scan, self.flag_pairs = (self.flag_scan, self.flag_pairs), False, False
                prog = self.plan1(b, bound, outvars, sub, None)[0] if bound else self.plan_top(b, bound, outvars, sub)
                hit = (b, prog, self.flag_scan, self.flag_pairs)
                self.flag_scan, self.flag_pairs = saved
                self.plan_memo[key] = hit
            # bind: what the sub-plan does inside a bound context counts for the program it is part of
            self.flag_scan = self.flag_scan or (hit[2] and bool(bound))
            self.flag_pairs = self.flag_pairs or hit[3]
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
                self.last_anchor = None
                prog, cost = self.plan1(b, bound, outvars, sub, first)
            except Blocked:
                continue
            # bind: equal costs (estimates past the cap are all the cap) are decided by the smaller start
            rank = (cost, self.last_anchor or 0.0) if SMALL_FIRST else (cost, 0.0)
            if best is None or rank < best[3]:
                best = (prog, cost, self.last_card, rank)
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
        alts.sort(key=lambda a: (a[0], a[2] or 0.0) if SMALL_FIRST else a[0])
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
            est = est if est < ROWS_CAP else ROWS_CAP
            per = cost / est
            out.append(dict(prog=prog, est=est, per=per if per < COST_CAP else COST_CAP))
            if len(out) >= 4:
                break
        return out

    def anchor_choices(self, b):
        """what a top-level plan may start from: each atom, and (cut) each use of a definition, entered from the
        definition's side (its rows drive, the rest of the branch becomes the test)"""
        out = [("a", i) for i in range(len(b.atoms))]
        if CALL_COST:
            out += [("s", j) for j, sp in enumerate(b.specials) if sp[0] == "call"]
        if ANCHOR_TOP and len(out) > ANCHOR_TOP:
            one = Br()
            rank = []
            for ch in out:
                if ch[0] == "a":
                    one.atoms, one.filt, one.specials = [b.atoms[ch[1]]], b.filt, []
                    rank.append((self.branch_anchor(one), len(rank), ch))
                else:
                    rank.append((self.def_anchor(b.specials[ch[1]][1]), len(rank), ch))
            rank.sort()
            out = [ch for _, _, ch in rank[:ANCHOR_TOP]]
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
        # cut: an atom's possible accesses change only when one of its variables gets bound: kept per atom, dropped then
        # (they were recomputed for every atom at every placement: items squared, the dominant lowering cost)
        acc_cache, var_atoms, seen_bound = {}, {}, set(bound)
        for i0, (_, ts0) in enumerate(b.atoms):
            for t0 in ts0:
                if isinstance(t0, int):
                    var_atoms.setdefault(t0, []).append(i0)
        atoms = list(range(len(b.atoms)))
        filt = list(b.filt)
        negs = list(b.negs)
        specials = list(b.specials)

        def tbound(t):
            return type(t) is not int or t in bound

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
                    if kind == "byk" and c in arg[0]:  # asof: the keyed access compared this column already
                        continue
                    if kind == "bym" and c in arg[0] and arg[1][arg[0].index(c)] == t and tbound(t):
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
            if kind == "bym":
                # cut: the composite key's literal columns are implied by the access
                for c0, v0 in zip(arg[0], arg[1]):
                    if not is_var(v0) and is_var(ts[c0]):
                        lit_var = ts[c0]
                        filt[:] = [f for f in filt if not (f[0] == "=" and f[1] == lit_var and f[2] == v0)]
                        prog[:] = [x for x in prog if not (x[0] == "test" and x[1] == "=" and x[2] == lit_var and x[3] == v0)]
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
                    elif is_var(t) and t in bound and self.schema[rel]["columns"][c]["kind"] in (("symbol", "node", "span") if "span" in KEY_KINDS else ("symbol", "node")):
                        # (bind: a span too; the executor's index keeps the low half of the cell, and place_atom
                        # compares the whole column after the read, so the bucket may hold other spans)
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
                    if tbound(t) and cols[c]["kind"] in KEY_KINDS:
                        if is_var(t) or t[0] in ("t", "n"):
                            cost = 1.0 if cols[c]["key"] else (1.3 if cols[c]["kind"] in ("node", "span") else 6.0)
                            if MADE_PARTIAL and rel in self.made_keys and c in self.made_keys[rel] and len(self.made_keys[rel]) > 1:
                                cost = MADE_PARTIAL
                            out.append((cost, "by", [c, t]))
                    elif is_var(t):
                        lit = lit_on(t)
                        if lit is not None:
                            out.append((self.est_bucket(rel, c, lit), "by", [c, lit]))
                out.extend(self.keyed_accesses(rel, ts, tbound))  # asof: every bound column of a keyed made table at once
                if MULTI_KEY:
                    # cut: every bound column (and every column fixed to one literal) as one composite key
                    mc, mv, single, nvar = [], [], [], 0
                    for c, t in enumerate(ts):
                        if cols[c]["kind"] not in KEY_KINDS:
                            continue
                        if tbound(t) and (is_var(t) or t[0] in ("t", "n")):
                            mc.append(c); mv.append(t); nvar += 1 if is_var(t) else 0
                            single.append(1.0 if cols[c]["key"] else (1.3 if cols[c]["kind"] in ("node", "span") else 6.0))
                        elif is_var(t):
                            lit = lit_on(t)
                            if lit is not None and lit[0] in ("t", "n"):
                                mc.append(c); mv.append(lit)
                    if len(mc) >= 2 and nvar >= 1:
                        # ranked just before the best single column of the same atom, never before another atom's keyed read
                        mcost = min(single) * 0.99
                        if MADE_PARTIAL and rel in self.made_keys and len(self.made_keys[rel]) > 1:
                            mcost = 1.29 if self.made_keys[rel] <= set(mc) else MADE_PARTIAL * 0.99
                        out.append((mcost, "bym", [mc, mv]))
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
                return tbound(s[1]) or (NEARC_INV and tbound(s[2]))
            return tbound(s[2]) or tbound(s[3])

        flush_filters()
        while atoms or specials or negs:
            if EAGER_EQ:
                eq = next((f for f in filt if f[0] == "=" and is_var(f[1]) and is_var(f[2]) and (f[1] in bound) != (f[2] in bound)), None)
                if eq is not None:
                    src, dst = (eq[1], eq[2]) if eq[1] in bound else (eq[2], eq[1])
                    prog.append(["let", src, dst])
                    bound.add(dst)
                    filt.remove(eq)
                    flush_filters()
                    continue
            # cut: estimates stay finite (nested definitions multiply into inf, inf * 0 into nan, and a program's est/per
            # must be numbers the executor's JSON reader accepts)
            if not cost[0] < COST_CAP:
                cost[0] = COST_CAP
            if not cost[1] < ROWS_CAP:
                cost[1] = ROWS_CAP
            best = None
            best_fan = 0.0
            if len(bound) != len(seen_bound):
                for v0 in bound - seen_bound:
                    for i0 in var_atoms.get(v0, ()):
                        acc_cache.pop(i0, None)
                seen_bound = set(bound)
            for i in atoms:
                if first is not None and not bound and first != ("a", i):
                    continue
                acc = acc_cache.get(i)
                if acc is None:
                    acc = acc_cache[i] = accesses(i)
                for c, kind, arg in acc:
                    if not bound and kind in ("key", "children"):
                        continue
                    fan = c
                    if bound and (kind in ("kind", "value", "scan") and not (kind == "value" and is_var(arg[1])) or
                                  kind == "by" and not is_var(arg[1]) and not self.schema[b.atoms[i][0]]["columns"][arg[0]]["key"]):
                        if kind == "by" and self.rare_literal(b.atoms[i][0], arg[0], arg[1]):
                            c = c + 10.0
                        else:
                            c = c + 10.0 if not BUCKET_X else c * BUCKET_X + 10.0  # a bucket inside a candidate is a cross product
                    if not RANK_SPLIT:
                        fan = c
                    if MADE_FIRST_RANK and bound and c < MADE_FIRST_RANK and b.atoms[i][0] in self.made_cost and b.atoms[i][0] not in built:
                        c = MADE_FIRST_RANK
                    if best is None or c < best[0]:
                        best = (c, "atom", i, (kind, arg))
                        best_fan = fan
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
                        c = self.clos_cost(s, tbound(s[1])) if CLOS_COST else 8.0
                    elif s[0] == "nearc":
                        c = 10.0 if tbound(s[1]) else 12.0
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
            if ready_negs and CALL_COST and TEST_WAIT_STEPS > 0:
                # cut: the cheapest ready test first; a dear one lets a keyed fact lookup go before it
                eff = cost[1] * self.mult
                tc = [(self.test_cost(negs[k][1], negs[k][2], tbound, eff), k) for k in ready_negs]
                per0, k0 = min(tc)
                ready_negs = [k0]
                wait = TEST_WAIT_RANK if per0 >= TEST_WAIT_STEPS else 1.0
                if TEST_WAIT_STEPS2 and per0 >= TEST_WAIT_STEPS2:
                    wait = TEST_WAIT_RANK2
            else:
                wait = 1.0
            if ready_negs and (best is None or best[0] > wait):
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
                    self.flag_pairs = True  # bind: every pair of the walk
                    if bound:
                        self.flag_scan = True
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
                    cost[0] += self.made_cost[rel0] * min(1.0, (cost[1] * self.mult) ** MADE_P_EXP)
            if not bound and cost[0] == 0.0:
                self.last_anchor = best[0]
            step = best_fan if best[1] == "atom" else best[0]
            cost[0] += cost[1] * step
            if not (best[1] == "atom" and best[3][0] == "key") and not (best[1] == "special" and best[0] <= 1.0) and \
                    not (best[1] == "special" and specials[best[2]][0] == "fn"):
                cost[1] *= max(step, 0.05)
            if best[1] == "atom":
                kind0, arg0 = best[3]
                if bound and (kind0 in ("kind", "scan") or (kind0 in ("value", "by") and not is_var(arg0[1])) or
                              (kind0 == "bym" and not any(is_var(v) for v in arg0[1]))):
                    self.flag_scan = True  # bind: a bucket or a scan per candidate: not navigation
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
                elif s[0] == "nearc" and not tbound(s[1]):
                    self.place_nearc_inverse(prog, s, bound)
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
                    outs, D = s[1], s[2]
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
                    if tbound(a) and not (CLOS_BOTH and tbound(bb) and self.clos_cost(s, False) < self.clos_cost(s, True)):
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
        self.last_card = cost[1] if cost[1] < ROWS_CAP else ROWS_CAP
        return prog, (cost[0] if cost[0] < COST_CAP else COST_CAP)

    def mask_of(self, key, args, tbound):
        """the columns a use binds; past MASK_CAP distinct patterns per definition the use binds none (the definition's
        uncorrelated program, memoised per file, with the args tested after): the bound on programs per definition"""
        mask = tuple(c for c, t in enumerate(args) if t is not None and tbound(t))
        seen = self.masks.setdefault(key, set())
        if mask not in seen and len(seen) >= MASK_CAP:
            DEGRADE_SEEN[0] += 1  # asof (diagnostic): NAV_DEGRADE_MAX bisects which degraded use changes an answer
            if DEGRADE_MAX < 0 or DEGRADE_SEEN[0] <= DEGRADE_MAX:
                if os.environ.get("NAV_DEGRADE_LOG"):
                    print("DEGRADE", DEGRADE_SEEN[0], key, mask, [None if t is None else (t if is_var(t) else list(t)) for t in args], file=sys.stderr)
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
            tu = self.table_use(key, mask, ())  # asof: or answered from the definition's own table of the file
            prog.append([m + "-keyed", subs, [F[c] for c in mask], []] + ([tu] if tu else []))
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
                tu = self.table_use(key, mask, ())  # asof
                prog.append(["exists-keyed", subs, [F[c] for c in mask], []] + ([tu] if tu else []))
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
            ins = ["or-keyed", outs, ins[2], [F[c] for c in mask], []]
            tu = self.table_use(key, mask, free)  # asof: or answered from the definition's own table of the file
            if tu:
                ins.append(tu)
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
        hit = self.all_cols_memo.get(key)
        if hit is None:
            body = self.defs[key]
            hit = self.all_cols_memo[key] = tuple(c for c, t in enumerate(body[0].out) if is_var(t) or t[0] != "f") if body else ()
        return hit

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
        if ONE:
            way = self.way(key, cols, mask, free if free else None, eff)
            if way == "probe":
                # the definition's table of the file: its rows (nothing bound), or the rows under the bound columns
                c0, n0 = self.def_cost(key, (), self.all_cols(key))
                once = c0 * min(1.0, eff)
                if not mask:
                    if not any_bound:
                        return ("probe", c0, 0.0, c0, n0)
                    # (bind) a table's rows per candidate are a cross product: the last resort, after anything a key reaches
                    return ("probe", max(n0 * BUCKET_X + 10.0, CROSS_RANK), max(n0, 1.0), once, n0)
                fan = n0 * 0.1 ** len(mask)
                if not free:
                    fan = min(fan, 1.0)
                per = 1.0 + fan
                return ("probe", max(fan, (once / max(eff, 1e-6) + per) / KEYED_STEPS, 1.05), per, once, fan)
            if way in ("keyed", "budget"):
                cm, nm = self.def_cost(key, mask, free if free else None, eff)
                return ("keyed", max(nm, cm / KEYED_STEPS, 1.05), cm + 1.0, 0.0, nm)
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
            if best is None or (self.probe_ok(key, allc) and once + eff * PROBE_BIAS * per < eff * PROBE_BIAS * best[2]):
                best = ("probe", max(fan, (once / max(eff, 1e-6) + per) / KEYED_STEPS, 1.05), per, once, fan)
        if best is None:
            # no pattern left and no probe: last round's degraded use (uncorrelated rows, args tested after)
            c0, n0 = self.def_cost(key, (), cols)
            best = ("degraded", n0 * BUCKET_X + 10.0, max(n0, 1.0), c0 * min(1.0, eff), n0 * 0.1 ** len(mask))
        return best

    def walks(self, idxs):
        """whether sub-programs (and the shared entries they run) walk a closure or a nearest-over-edges: their rows with
        nothing bound can be every pair of a chain, which no bucket statistic shows"""
        memo = self.walk_memo

        def h(p):
            for ins in p:
                if ins[0] == "shared":
                    j = ins[1]
                    if j not in memo:
                        memo[j] = False
                        memo[j] = h(self.shared[j])
                    if memo[j]:
                        return True
                elif ins[0] in ("clos", "nearc"):
                    return True
                else:
                    for k in SUBS_AT.get(ins[0], ()):
                        if any(h(q) for q in ins[k]):
                            return True
            return False
        return any(h([["shared", i]]) for i in idxs)

    def probe_ok(self, key, allc):
        """whether a probe of the definition may be preferred to a keyed call on the cost estimate"""
        if not PROBE_PREFER:
            return False
        return not PROBE_SAFE or not self.walks(self.body(key, (), allc))

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

    def place_nearc_inverse(self, prog, s, bound):
        """a nearest-over-derived-edges walk entered from its reached end (bound): a closure back over the edges from the
        reached node, going on only through nodes the domain does not admit, yields every start whose shortest way to an
        admitted node can end at the reached one (a superset); the forward walk from each start then decides."""
        _, start, reached, x, y, EB, D = s
        back = [self.plan(z, bound | {y}, None, [x]) for z in EB]
        dom_y = [self.plan(z, bound | {y}, None, True) for z in self.subst_all(D, reached, y)]
        subs = [[["test", "=", y, reached]] + q for q in back] + [[["neg", dom_y]] + q for q in back]
        prog.append(["clos", reached, start, y, x, [], [], subs])
        bound.add(start)
        tmp = self.fresh("node")
        eprogs = [self.plan(z, bound | {x}, None, [y]) for z in EB]
        dprogs = [self.plan(z, bound | {tmp}, None, True) for z in self.subst_all(D, reached, tmp)]
        prog.append(["nearc", start, tmp, x, y, eprogs, dprogs])
        prog.append(["test", "=", tmp, reached])

    def clos_cost(self, s, from_source):
        """rows a closure reaches per start: 30 when it walks the tree's parent-to-child edges away from the start (all
        descendants), 8 otherwise (towards the root, or over another relation)"""
        _, a, bb, kv, x, y, kx, D = s
        down = up = False
        for z in D:
            for rel, ts in z.atoms:
                if rel in PARENT_COL:
                    if ts[PARENT_COL[rel]] == x and ts[TREE[rel]] == y:
                        down = True
                    if ts[PARENT_COL[rel]] == y and ts[TREE[rel]] == x:
                        up = True
        if not (down if from_source else up):
            return 8.0
        if not CLOS_LIT:
            return 30.0
        # cut: a walk down the tree only over edges with literal tests (a field, a kind) reaches few nodes: 0.3 per test of
        # the least tested edge, never under 2 (the nested elements of a target, not every descendant)
        k = min(sum(1 for f in z.filt if f[0] in ("=", "in") and (not is_var(f[1]) or not is_var(f[2]))) for z in D)
        return max(2.0, 30.0 * 0.3 ** k)

    def test_cost(self, key, args, tbound, eff):
        """steps per candidate of a use of a definition as a test here (the cheaper of its keyed sub-program and a probe
        of its uncorrelated rows; place_test takes the same choice)"""
        if not self.defs[key]:
            return 1.0
        cols = tuple(c for c, t in enumerate(args) if t is not None)
        mask = tuple(c for c in cols if tbound(args[c]))
        if ONE:
            way = self.way(key, mask, mask, None, eff)
            if way == "probe":
                return 1.0 + self.def_cost(key, (), self.all_cols(key))[0] * min(1.0, eff) / max(eff, 1e-6)
            if way in ("keyed", "budget"):
                return self.def_cost(key, mask, None, eff)[0] + 1.0
        if not mask:
            return 1.0
        cm = None
        if self.mask_room(key, mask):
            cm = self.def_cost(key, mask, None, eff)[0] + 1.0
        allc = self.all_cols(key)
        if PROBE and all(c in allc for c in mask):
            once = self.def_cost(key, (), allc)[0] * min(1.0, eff)
            if cm is None or (self.probe_ok(key, allc) and once + eff * PROBE_BIAS < eff * PROBE_BIAS * cm):
                return 1.0 + once / max(eff, 1e-6)
        return cm if cm is not None else 5.0

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
        if ONE:
            way = self.way(key, mask, mask, None, eff)
            if way == "probe":
                allc = self.all_cols(key)
                idxs = self.body(key, (), allc)
                prog.append([m + "-probe", [[["shared", i]] for i in idxs], [allc.index(c) for c in mask], [args[c] for c in mask], len(allc)])
                return 1.0, self.body_cost[(key, (), allc)][0] * min(1.0, eff)
            if way in ("keyed", "budget"):
                idxs = self.body(key, mask, None, eff)
                cm = self.body_cost[(key, mask, None)][0]
                br = self.body_branches[(key, mask, None)]
                if INLINE_LIGHT and self.light(br):
                    sub = self.inline_args(prog, key, mask, args)
                    prog.append([m, [substitute(self.shared[i], sub) for i in br]])
                    return cm, 0.0
                subs = [[["shared", i]] for i in idxs]
                if KEYED_MEMO and self.heavy(idxs):
                    tu = self.table_use(key, mask, ())
                    prog.append([m + "-keyed", subs, [F[c] for c in mask], [args[c] for c in mask]] + ([tu] if tu else []))
                else:
                    for c in mask:
                        prog.append(["let", args[c], F[c]])
                    prog.append([m, subs])
                return cm + 1.0, 0.0
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
            if cm is None or (self.probe_ok(key, allc) and once + eff * PROBE_BIAS < eff * PROBE_BIAS * (cm + 1.0)):
                idxs = self.body(key, (), allc)
                prog.append([m + "-probe", [[["shared", i]] for i in idxs], [allc.index(c) for c in mask], [args[c] for c in mask], len(allc)])
                return 1.0, once
        if cm is None:
            self.place_test_old(prog, m, key, args, tbound)
            return 5.0, 0.0
        subs = [[["shared", i]] for i in idxs]
        if KEYED_MEMO and self.heavy(idxs):
            tu = self.table_use(key, mask, ())  # bind: asof's definition table on cut's keyed test
            prog.append([m + "-keyed", subs, [F[c] for c in mask], [args[c] for c in mask]] + ([tu] if tu else []))
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
            if not mask and bound:
                self.flag_scan = True  # bind: a table's rows per candidate: not navigation
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
                tu = self.table_use(key, mask, ())  # bind
                prog.append(["exists-keyed", subs, [F[c] for c in mask], [args[c] for c in mask]] + ([tu] if tu else []))
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
            tu = self.table_use(key, mask, free)  # bind: asof's definition table on cut's keyed call
            prog.append(["or-keyed", outs, [[["shared", i]] for i in idxs], [F[c] for c in mask], [args[c] for c in mask]] + ([tu] if tu else []))
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
        low = pack_lowerer(schema, freq)  # share: its variables are the pack's (they were a fixed range every rule reused)
        low.outer_vars = set()
        cols = schema[rel]["columns"]
        ts = [("f", 0) if c["kind"] == "file" else low.fresh(c["kind"]) for c in cols]
        prog = low.plan_alts(Br([[rel, list(ts)]], out=list(ts)))[0]["prog"]
        kinds = [c["kind"] for c in cols]
        for st in tail:
            if st.get("step") == a["step"]:
                st["kinds"] = kinds
        out.append(dict(merged=a["merged"], programs=[prog], columns=kinds))
    return out


PACK = [None]


def pack_lowerer(schema, freq):
    """share: the one lowerer of the pack (its relation table grows by the made tables)"""
    if PACK[0] is None:
        PACK[0] = Lowerer(dict(schema), freq)
    return PACK[0]


def lower_core(rid, steps, X, schema, freq, step_nodes):
    low = pack_lowerer(schema, freq)
    low.begin(rid, steps, step_nodes)
    branches = low.template(X)
    if TOP_INLINE:
        branches = low.expand_top(branches)
    if low.too_big(branches) and not (TOP_INLINE and len(branches) <= BRANCH_CAP):
        branches = [low.cut(branches, low.ck("step", X))]
    progs = []
    for b in branches:
        low.outer_vars = collect_vars(b, set())
        progs.append(low.plan_alts(b))
        if REQUIRE:
            low.require_alts(b, progs[-1])
    if not progs:
        raise Blocked("no-branch")
    if PREFIX and TOP_PREFIX:
        progs = merge_top(progs)
    LAST_STATS.update(programs_planned=low.planned, programs_reused=len(low.reused), shared_new=len(low.shared) - low.new_from,
                      made_new=len(low.made), template_steps=len(low.memo), body_cpu_s=round(low.body_cpu, 3), asof=list(low.fired))
    return progs, low.made


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
                progs, made = lower_core(rid, steps, X, schema, freq, rule["step_nodes"])
                low = PACK[0]
                navd = dict(x=X, xmerged=xnode, columns=xcols, programs=progs)
                if aux:
                    navd["aux"] = lower_aux(aux, tail, schema, freq)
                share_rule(navd, low.shared, low.new_from, made)
                low.shared_meta.extend([None, [], None, "sub"] for _ in range(len(low.shared) - len(low.shared_meta)))
                # share: the bound is on what this attempt added to the pack (its own programs, the definition programs and
                # made tables no rule before had)
                if len(json.dumps(navd)) + len(json.dumps(low.shared[low.new_from:])) + len(json.dumps(made)) > size_cap:
                    raise Blocked("program-size")
                done = (X, tail, navd)
            except Blocked as x:
                first_reason = first_reason or str(x)
                if str(x) == "program-size":
                    PACK[0].forget()  # share: the retry under smaller bounds cuts its definitions anew
                    continue
            except RecursionError:
                first_reason = first_reason or "recursion"
                PACK[0].forget()
            except TimeoutError:
                first_reason = first_reason or "lowering-time"
                PACK[0].forget()  # share: cut off part-way: nothing half-planned may be reused
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
    t = time.time()
    c0 = time.process_time()
    LAST_STATS.clear()
    r, why = lower_rule(_G["rules"][rid], _G["nodes"], _G["schema"], _G["freq"])
    STATS[rid] = dict(LAST_STATS, cpu_s=round(time.process_time() - c0, 3))
    if os.environ.get("NAV_STATS"):
        print(rid, json.dumps(STATS[rid]), why, file=sys.stderr)
    return r, why, time.time() - t


def main():
    """share: the rules are lowered one after the other by ONE lowerer (a definition a rule before lowered is not lowered
    again), then linked: the pack's shared entries, made tables and register kinds, and each rule's own programs"""
    wanted = [l.strip() for l in open(sys.argv[4]) if l.strip() and not l.startswith("#")]
    _init(sys.argv[1], sys.argv[2], sys.argv[3])
    out_rules, blocked, seconds = [], {}, {}
    c0 = time.process_time()
    for rid in wanted:
        if rid not in _G["rules"]:
            continue
        r, why, secs = _one(rid)
        seconds[rid] = round(secs, 2)
        if r:
            out_rules.append(r)
        else:
            blocked[rid] = why
        print(rid, round(secs, 1), why or "", file=sys.stderr, flush=True)
    low = PACK[0]
    lowering = time.process_time() - c0
    pack = dict(kinds={str(k): v for k, v in low.kind.items()}, made=list(low.made_all.values()), shared=low.shared,
                shared_meta=low.shared_meta, rules=out_rules) if low else dict(kinds={}, made=[], shared=[], shared_meta=[], rules=out_rules)
    before = (len(pack["shared"]), len(pack["made"]))
    pack = link(pack)
    stats = collections.Counter(blocked.values())
    json.dump(pack, open(sys.argv[5], "w"))
    totals = dict(lowering_cpu_s=round(lowering, 1), link_cpu_s=round(time.process_time() - c0 - lowering, 1),
                  definitions=len(low.defs) if low else 0, definition_programs=len(low.body_memo) if low else 0,
                  shared_entries_planned=before[0], shared_entries_reached=len(pack["shared"]), made_planned=before[1],
                  made_reached=len(pack["made"]), registers=len(pack["kinds"]), variables_minted=low.nvar if low else 0,
                  programs_planned=sum(v.get("programs_planned", 0) for v in STATS.values()),
                  programs_reused=sum(v.get("programs_reused", 0) for v in STATS.values()),
                  body_cpu_s=round(sum(v.get("body_cpu_s", 0) for v in STATS.values()), 1))
    json.dump(dict(blocked=blocked, counts=stats.most_common(), seconds=seconds, stats=STATS, pack=totals),
              open(sys.argv[6], "w"), indent=1)
    print(f"lowered {len(out_rules)} of {len(wanted)}; blocked {len(blocked)}: {stats.most_common()}; {json.dumps(totals)}", file=sys.stderr)


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
        if ins[2] == "byk":
            return [t for t in arg[1] if is_var(t)]
        if ins[2] == "bym":
            return [t for t in arg[1] if is_var(t)]
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
            elif ins[2] == "byk":
                arg = [arg[0], [f(t) for t in arg[1]]]
            elif ins[2] == "bym":
                arg = [arg[0], [f(t) for t in arg[1]]]
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
            # [op, subs, formals | key columns, args | key values, (table | width)]: the terms of the fourth are renamed
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


def clean_sub(p, formals):
    return dce(cse(p, {}), set())


# ------------------------------------------------------------------ shared sub-programs (navwide)
SUBS_AT = {"neg": (1,), "exists": (1,), "neg-memo": (1,), "exists-memo": (1,), "neg-keyed": (1,), "exists-keyed": (1,), "or": (2,), "or-keyed": (2,), "clos": (7,), "near": (3,), "nearc": (5, 6),
           "or-probe": (2,), "neg-probe": (1,), "exists-probe": (1,), "need": (3,)}


# ------------------------------------------------------------------ cut: common prefixes of a union's programs
def rn_prog(p, m):
    return [rn_ins(i, m) for i in p]


def rn_ins(ins, m):
    """the instruction with every register (used, defined, or internal to a sub-program) renamed by m"""
    def f(t):
        return m.get(t, t) if type(t) is int else t

    def fl(ts):
        return [f(t) for t in ts]
    op = ins[0]
    if op == "iter":
        a = ins[3]
        if ins[2] == "children":
            a = f(a)
        elif ins[2] == "bym":
            a = [a[0], fl(a[1])]
        elif isinstance(a, list) and len(a) == 2:
            a = [a[0], f(a[1])]
        return [op, ins[1], ins[2], a, f(ins[4])]
    if op == "row":
        return [op, ins[1], f(ins[2])]
    if op == "read":
        return [op, ins[1], ins[2], f(ins[3]), f(ins[4])]
    if op == "test":
        return [op, ins[1], f(ins[2]), f(ins[3])]
    if op in ("axis", "sib"):
        return [op, ins[1], f(ins[2]), f(ins[3])]
    if op == "let":
        return [op, f(ins[1]), f(ins[2])]
    if op == "fn":
        return [op, ins[1], fl(ins[2]), fl(ins[3]), ins[4]]
    if op in ("emit", "collect"):
        return [op, fl(ins[1])]
    if op in ("yield", "shared", "need"):
        return ins  # (bind: a need names no register of the program it guards)
    if op in ("neg", "exists", "neg-memo", "exists-memo"):
        return [op, [rn_prog(q, m) for q in ins[1]]] + ins[2:]
    if op in ("neg-keyed", "exists-keyed"):
        return [op, [rn_prog(q, m) for q in ins[1]], fl(ins[2])] + ([fl(ins[3])] + ins[4:] if len(ins) > 3 else [])
    if op in ("neg-probe", "exists-probe"):
        return [op, [rn_prog(q, m) for q in ins[1]], ins[2], fl(ins[3]), ins[4]]
    if op == "or":
        return [op, fl(ins[1]), [rn_prog(q, m) for q in ins[2]]] + ins[3:]
    if op == "or-keyed":
        return [op, fl(ins[1]), [rn_prog(q, m) for q in ins[2]], fl(ins[3])] + ([fl(ins[4])] + ins[5:] if len(ins) > 4 else [])
    if op == "or-probe":
        return [op, fl(ins[1]), [rn_prog(q, m) for q in ins[2]], ins[3], fl(ins[4]), ins[5], ins[6]]
    if op == "near":
        return [op, f(ins[1]), f(ins[2]), [rn_prog(q, m) for q in ins[3]]]
    if op == "clos":
        return [op, f(ins[1]), f(ins[2]), f(ins[3]), f(ins[4]), fl(ins[5]), fl(ins[6]), [rn_prog(q, m) for q in ins[7]]]
    if op == "nearc":
        return [op, f(ins[1]), f(ins[2]), f(ins[3]), f(ins[4]), [rn_prog(q, m) for q in ins[5]], [rn_prog(q, m) for q in ins[6]]]
    raise Blocked("rename-" + op)


def ins_defs(ins):
    """the registers an instruction writes at its own level (a closure's and a nearest walk's scratch registers included)"""
    op = ins[0]
    if op in ("iter", "read"):
        return [ins[4]]
    if op in ("axis", "sib"):
        return [ins[3]]
    if op == "let":
        return [ins[2]]
    if op == "fn":
        return list(ins[3])
    if op == "near":
        return [ins[2]]
    if op == "clos":
        return [ins[2], ins[3], ins[4]] + list(ins[5]) + list(ins[6])
    if op == "nearc":
        return [ins[2], ins[3], ins[4]]
    if op in ("or", "or-keyed", "or-probe"):
        return list(ins[1])
    return []


class _Seen(dict):
    """a rename map that records every register asked for (rn_ins as a register collector)"""
    def get(self, k, d=None):
        self[k] = True
        return d


def prog_regs(p):
    seen = _Seen()
    rn_prog(p, seen)
    return set(seen)


def reg_seq(ins):
    """every register occurrence of the instruction (sub-programs included), in a fixed order"""
    seq = []

    class Rec(dict):
        def get(self, k, d=None):
            seq.append(k)
            return d
    rn_ins(ins, Rec())
    return seq


def all_defs(ins, acc):
    """the registers the instruction writes, at its own level and inside its sub-programs"""
    acc.update(t for t in ins_defs(ins) if type(t) is int)
    for k in SUBS_AT.get(ins[0], ()):
        for q in ins[k]:
            for x in q:
                all_defs(x, acc)
    return acc


def match_ins(a, b, rest_regs):
    """the renaming of the registers b writes (its own and those inside its sub-programs) that makes it the instruction
    a, or None; {} when they are equal. `rest_regs` is b's remaining program. Only registers b itself writes are renamed (every earlier one already has a's
    name, the prefix being shared), and a register of a that b's remaining program names refuses the match."""
    if a == b:
        return {}
    if a[0] != b[0]:
        return None
    try:
        sa, sb = reg_seq(a), reg_seq(b)
    except Blocked:
        return None
    if len(sa) != len(sb):
        return None
    own = all_defs(b, set())
    m = {}
    rest = None
    for x, y in zip(sa, sb):
        if x != y:
            if y in m:
                if m[y] != x:
                    return None
            else:
                if rest is None:
                    rest = prog_regs(rest_regs)  # the registers b's remaining program names (asked for once, when needed)
                if y not in own or x in rest:
                    return None
                m[y] = x
    if not m or len(set(m.values())) != len(m):
        return None
    return m if rn_ins(b, m) == a else None


def factor_prefix(progs, test, top=False):
    """programs of one union (every one runs: each collects or emits its rows, or the first that holds yields) as ONE
    program: those that start with the same instruction (up to the names of the registers it writes) share it and fork
    after, recursively. Returns the program, and how many instructions were shared.
    top: the programs grouped by their first instruction only, as [(program, [indexes of the programs it holds])] (the
    core's branches: each group stays a top program with its anchor first)."""
    saved = [0]

    def group(ps):
        groups = []
        for k, p in enumerate(ps):
            # bind: a leading need is set aside (it names no register): programs that differ only in what they need still
            # share their start; each need is put back at the head of its own rest, or before the shared start when the
            # whole group needs the same
            nd = None
            if len(p) > 1 and p[0][0] == "need":
                nd, p = p[0], p[1:]
            for g in groups:
                m = match_ins(g[0], p[0], p[1:])
                if m is not None:
                    g[1].append(rn_prog(p[1:], m) if m else p[1:])
                    g[2].append(k)
                    g[3].append(nd)
                    saved[0] += 1
                    break
            else:
                groups.append((p[0], [p[1:]], [k], [nd]))
        subs = []
        for head, rests, ks, nds in groups:
            if len(rests) == 1 or not all(rests):
                subs.append((([nds[0]] if nds[0] else []) + [head] + rests[0], ks))
            elif all(n == nds[0] for n in nds):
                subs.append((([nds[0]] if nds[0] else []) + [head] + fork(rests), ks))
            else:
                subs.append(([head] + fork([([n] if n else []) + r for n, r in zip(nds, rests)]), ks))
        return subs

    def fork(ps):
        subs = [q for q, _ in group([p for p in ps if p])]
        if len(subs) == 1:
            return subs[0]
        return [["exists", subs]] + ([["yield"]] if test else [])
    if top:
        return group([list(p) for p in progs]), saved[0]
    return fork([list(p) for p in progs]), saved[0]


def merge_top(progs):
    """cut: the core's branches whose chosen programs start with the same instruction become one top program (the shared
    start once, the rests forked). A merged branch keeps only the kind alternatives every one of its branches had (an
    empty bucket must prove all of them empty)."""
    if len(progs) < 2:
        return progs
    groups, n_shared = factor_prefix([alts[0]["prog"] for alts in progs], False, top=True)
    if not n_shared:
        return progs
    out = []
    for prog, ks in groups:
        if len(ks) == 1:
            out.append(progs[ks[0]])
            continue
        first = progs[ks[0]]
        alts = [dict(prog=prog, est=first[0]["est"], per=min(sum(progs[k][0]["per"] for k in ks), COST_CAP))]
        kinds = lambda a: json.dumps(a["prog"][0][3]) if a["prog"] and a["prog"][0][0] == "iter" and a["prog"][0][2] == "kind" else None
        common = None
        for k in ks:
            have = {kinds(a) for a in progs[k][1:]} - {None}
            common = have if common is None else common & have
        seen = set()
        for a in first[1:]:
            kk = kinds(a)
            if kk in common and kk not in seen:
                seen.add(kk)
                alts.append(dict(prog=[a["prog"][0], ["emit", []]], est=1.0, per=1.0))
        out.append(alts)
    return out


def table_at(ins):
    """bind: where a keyed call or test carries its definition's table ([[[shared entry]], width, key places, collected
    places]), or None"""
    k = 4 if ins[0] in ("neg-keyed", "exists-keyed") else 5 if ins[0] == "or-keyed" else None
    return k if k is not None and len(ins) > k and ins[k] else None


def share_rule(nav, table, base_from, made, min_bytes=300):
    """sub-programs that occur more than once in what one rule added to the pack (its programs, its new made tables, the
    shared entries from `base_from` on) become entries of the pack's shared table; each use is the one-step program
    [["shared", k]] (the executor runs the entry in place)"""
    counts = collections.Counter()

    def count(p):
        for ins in p:
            for k in SUBS_AT.get(ins[0], ()):
                for q in ins[k]:
                    counts[json.dumps(q)] += 1
                    count(q)

    n0 = len(table)
    progs = [a["prog"] for alts in nav["programs"] for a in alts] + [q for m in made for q in m["programs"]] + \
        [q for m in made for q in m.get("right_programs", [])] + table[base_from:]
    for p in progs:
        count(p)
    index = {}

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
    for m in made:
        m["programs"] = [tr(q) for q in m["programs"]]
        if "right_programs" in m:
            m["right_programs"] = [tr(q) for q in m["right_programs"]]
    for i in range(base_from, n0):
        table[i] = tr(table[i])


def link(pack, ids=None):
    """share: the pack restricted to the rules named (all of them when None): the shared entries and made tables their
    programs reach (plans tried and dropped, and patterns costed but not used, leave entries behind), renumbered, and the
    kinds of the registers those name. Order: kinds, made tables, shared entries, rules (the loader reads it as a stream)."""
    rules = [r for r in pack["rules"] if ids is None or r["id"] in ids]
    table, made = pack["shared"], {m["name"]: m for m in pack["made"]}
    keep, keep_made = set(), set()

    def names(x):
        if isinstance(x, str):
            if x.startswith("made:") and x not in keep_made:
                keep_made.add(x)
                for q in made[x]["programs"] + made[x].get("right_programs", []):
                    walk(q)
        elif isinstance(x, list):
            for y in x:
                names(y)

    def walk(p):
        for ins in p:
            if ins[0] == "shared":
                if ins[1] not in keep:
                    keep.add(ins[1])
                    walk(table[ins[1]])
                continue
            subs = SUBS_AT.get(ins[0], ())
            for k in subs:
                for q in ins[k]:
                    walk(q)
            kt = table_at(ins)
            if kt is not None:
                walk(ins[kt][0][0])
            for j, y in enumerate(ins):
                if j not in subs and j != kt:
                    names(y)
    for r in rules:
        for alts in r["nav"]["programs"]:
            for a in alts:
                walk(a["prog"])
        for a in r["nav"].get("aux", []):
            for q in a["programs"]:
                walk(q)
    new = {old: i for i, old in enumerate(sorted(keep))}
    used = set()

    def tr(p):
        out = []
        for ins in p:
            if ins[0] == "shared":
                ins = ["shared", new[ins[1]]]
            else:
                ks = SUBS_AT.get(ins[0], ())
                kt = table_at(ins)
                if ks or kt is not None:
                    ins = list(ins)
                    for k in ks:
                        ins[k] = [tr(q) for q in ins[k]]
                    if kt is not None:
                        ins[kt] = [[tr(q) for q in ins[kt][0]]] + list(ins[kt][1:])
                ints(ins, ks)
            out.append(ins)
        return out

    def ints(x, skip=()):
        for j, y in enumerate(x):
            if j in skip:
                continue
            if isinstance(y, list):
                ints(y)
            elif isinstance(y, int) and not isinstance(y, bool):
                used.add(str(y))
    out_rules = []
    for r in rules:
        nav = dict(r["nav"])
        nav["programs"] = [[dict(a, prog=tr(a["prog"])) for a in alts] for alts in nav["programs"]]
        if "aux" in nav:
            nav["aux"] = [dict(a, programs=[tr(q) for q in a["programs"]]) for a in nav["aux"]]
        out_rules.append(dict(r, nav=nav))
    out_made = []
    for m in pack["made"]:
        if m["name"] in keep_made:
            m = dict(m, programs=[tr(q) for q in m["programs"]])
            if "right_programs" in m:
                m["right_programs"] = [tr(q) for q in m["right_programs"]]
            out_made.append(m)
    meta = pack.get("shared_meta") or []
    return dict(kinds={k: v for k, v in pack["kinds"].items() if k in used}, made=out_made,
                shared=[tr(table[old]) for old in sorted(keep)],
                shared_meta=[meta[old] if old < len(meta) else [None, [], None, "sub"] for old in sorted(keep)], rules=out_rules)


# ====================================================================== asof (ryo-8hfg) =====================================
# Everything below is this round's; the hooks into the shared lowering are single lines marked `# asof`.
#
# 1. STRUCTURAL RECOGNITION of pointer doubling (no rule, fragment or relation is named):
#      J(k+1) = join(J(k), J(k)) on equal group keys and left.target = right.node, output (keys, left.node, right.target)
#    is relation composition per key group, so the top of a chain of k such steps is J(0)^(2^k). It is lowered to ONE made
#    table of the file (built on demand, keyed access by every bound column), instead of k definitions calling each other:
#      - `power`: any J(0) (its rows from J(0)'s own programs), squared k times by the executor (stops at a fixed point);
#      - `asof`: when J(0) is exactly  { probe -> the element before it } U { event -> itself }  taken from an ordered match
#        of (events U probes) with itself per key group, the table is "for each probe, the latest event at or before it in
#        its group": events and probes sorted once per file by (keys, order), one sweep; the joins of the result back to the
#        probes and to the events (semi-joins the table satisfies by construction) are folded into the table.
#    When a shape is not met exactly the step is lowered as before (definitions and calls).
# 2. KEYED ACCESS to such a table: `["iter", table, "byk", [[columns], [terms]], row]`, a hash of every bound column.
# (3. bind: the factoring of uncorrelated parts of a branch is deleted: the eight binding rules cost 42M steps less without it.)
ASOF = os.environ.get("NAV_ASOF", "1") == "1"
ASOF_SHAPE = os.environ.get("NAV_ASOF_SHAPE", "1") == "1"   # 0: only the general `power` table (to measure the two apart)
TABLE = os.environ.get("NAV_TABLE", "1") == "1"
BODY_UNDO = os.environ.get("NAV_BODY_UNDO", "1") == "1"  # 0: the last round's behaviour
DEGRADE_MAX = int(os.environ.get("NAV_DEGRADE_MAX", "-1"))
DEGRADE_SEEN = [0]


def _step_of(v):
    return v["step"] if isinstance(v, dict) and "step" in v else None


def _width(low, v):
    """the number of columns of an input (None: not derivable from the plan's shape)"""
    if "relation" in v:
        rel = low.schema.get(v["relation"])
        return len(rel["columns"]) if rel else None
    st = low.steps[v["step"]]
    op, p = st["op"], st["params"]
    if op == "project":
        return len(p["columns"])
    if op in ("join", "ordered_match"):
        return len(p["output"])
    if op == "select":
        return len(p["kept"]) if p["kept"] else _width(low, p["from"])
    if op == "extend":
        w = _width(low, p["from"])
        return None if w is None else w + len(p["values"])
    if op in ("union", "antijoin", "difference", "intersection"):
        return _width(low, p["left"])
    return None


def _strip(low, v):
    """the input under identity projections (the lowering is set-valued: `distinct` changes nothing)"""
    while _step_of(v) is not None:
        st = low.steps[v["step"]]
        if st["op"] != "project" or st.get("scope") == "cross-file":
            break
        w = _width(low, st["params"]["from"])
        if w is None or st["params"]["columns"] != list(range(w)):
            break
        v = st["params"]["from"]
    return v


def _doubling(low, s):
    """(input step, key columns, node column, target column, width) when step s is J join J on equal keys and
    left.target = right.node with output (left keys and node in place, right target in place); else None"""
    st = low.steps[s]
    if st["op"] != "join" or st.get("scope") == "cross-file":
        return None
    p = st["params"]
    l, r = p["left"], p["right"]
    if _step_of(l) is None or l != r or p["filter"] or p.get("view") or p.get("reach"):
        return None
    lo, ro = p["left_on"], p["right_on"]
    if len(lo) != len(ro) or len(set(lo)) != len(lo) or len(set(ro)) != len(ro):
        return None
    diff = [(a, b) for a, b in zip(lo, ro) if a != b]
    if len(diff) != 1:
        return None
    tcol, ncol = diff[0]
    keys = tuple(sorted(a for a, b in zip(lo, ro) if a == b))
    w = len(p["output"])
    if sorted(keys + (ncol, tcol)) != list(range(w)) or _width(low, l) != w:
        return None
    for i, o in enumerate(p["output"]):
        if o["column"] != i or o["side"] != ("right" if i == tcol else "left"):
            return None
    return l["step"], keys, ncol, tcol, w


def _chain(low, s):
    """(base input, levels, keys, node, target, width) of the doubling chain whose top is step s"""
    sig = _doubling(low, s)
    if sig is None:
        return None
    k, base = 0, s
    while True:
        g = _doubling(low, base)
        if g is None or g[1:] != sig[1:]:
            break
        k, base = k + 1, g[0]
    return ({"step": base}, k) + sig[1:]


def _asof_shape(low, base, keys, ncol, tcol, w):
    """J(0) as { probe -> previous element } U { event -> itself } over an ordered match of (events U probes) with itself:
    dict(events, probes, ekeys (the events' column of each key, in `keys` order), order, events_first), or None"""
    v = _strip(low, base)
    if _step_of(v) is None:
        return None
    u = low.steps[v["step"]]
    if u["op"] != "union":
        return None
    for a, b in ((u["params"]["left"], u["params"]["right"]), (u["params"]["right"], u["params"]["left"])):
        got = _asof_sides(low, _strip(low, a), _strip(low, b), keys, ncol, tcol, w)
        if got:
            return got
    return None


def _asof_sides(low, a, b, keys, ncol, tcol, w):
    """a = the probe pointers (node = the matched right element, a probe; target = the left element), b = the event
    self-pointers (node = target = the event's order)"""
    if _step_of(a) is None or _step_of(b) is None:
        return None
    pb, pa = low.steps[b["step"]], low.steps[a["step"]]
    if pb["op"] != "project" or pa["op"] != "project":
        return None
    cb, ca = pb["params"]["columns"], pa["params"]["columns"]
    if len(cb) != w or len(ca) != w or cb[ncol] != cb[tcol]:
        return None
    events = pb["params"]["from"]
    we = _width(low, events)
    ekeys, order = [cb[i] for i in keys], cb[ncol]
    if we is None or we != len(keys) + 1 or sorted(set(ekeys) | {order}) != list(range(we)) or len(set(ekeys)) != len(ekeys):
        return None
    # the probe pointers: project(select(ordered_match(U, U), tag of the right = the probes' tag))
    sel = pa["params"]["from"]
    if _step_of(sel) is None or low.steps[sel["step"]]["op"] != "select":
        return None
    sp = low.steps[sel["step"]]["params"]
    om = sp["from"]
    if _step_of(om) is None or low.steps[om["step"]]["op"] != "ordered_match" or len(sp["comparisons"]) != 1:
        return None
    mp = low.steps[om["step"]]["params"]
    out = mp["output"]
    kept = sp["kept"] or list(range(len(out)))
    if mp["left"] != mp["right"] or _step_of(mp["left"]) is None or mp["left_on"] != mp["right_on"] or \
            mp["left_order"] != mp["right_order"] or mp["left_order"] != order or sorted(mp["left_on"]) != sorted(ekeys):
        return None
    cmp_ = sp["comparisons"][0]
    if cmp_["test"] != "=" or "column" not in cmp_["left"] or "text" not in cmp_["right"]:
        return None
    tag_at = out[cmp_["left"]["column"]]
    if tag_at["side"] != "right" or tag_at["column"] != we:
        return None
    probe_tag = cmp_["right"]["text"]
    for i, key in enumerate(keys):
        o = out[kept[ca[key]]]
        if o["column"] != ekeys[i]:
            return None
    n_at, t_at = out[kept[ca[ncol]]], out[kept[ca[tcol]]]
    if (n_at["side"], n_at["column"]) != ("right", order) or (t_at["side"], t_at["column"]) != ("left", order):
        return None
    # U = union(extend(events, tag), extend(probes, tag')) in either order
    un = low.steps[mp["left"]["step"]]
    if un["op"] != "union":
        return None
    sides = []
    for x in (un["params"]["left"], un["params"]["right"]):
        if _step_of(x) is None or low.steps[x["step"]]["op"] != "extend":
            return None
        xp = low.steps[x["step"]]["params"]
        if len(xp["values"]) != 1 or "text" not in xp["values"][0] or _width(low, xp["from"]) != we:
            return None
        sides.append((xp["from"], xp["values"][0]["text"]))
    (x0, t0), (x1, t1) = sides
    if t0 == t1:
        return None
    if x0 == events and t1 == probe_tag:
        return dict(events=events, probes=x1, ekeys=ekeys, order=order, events_first=True)
    if x1 == events and t0 == probe_tag:
        return dict(events=events, probes=x0, ekeys=ekeys, order=order, events_first=False)
    return None


def _semi(low, s, shape, keys, ncol, tcol, w):
    """step s as a semi-join of the chain's rows with the probes or the events: join(X, probes|events) on every key and
    node|target = order, output = X's row. Returns (X input, restriction index) or None; restrictions: 0 node is a probe,
    1 target is an event, 2 node is an event, 3 target is a probe"""
    st = low.steps[s]
    if st["op"] != "join" or st.get("scope") == "cross-file":
        return None
    p = st["params"]
    if p["filter"] or p.get("view") or p.get("reach") or len(p["output"]) != w:
        return None
    if any(o["side"] != "left" or o["column"] != i for i, o in enumerate(p["output"])):
        return None
    if _width(low, p["left"]) != w:
        return None
    pairs = dict(zip(p["left_on"], p["right_on"]))
    if len(pairs) != len(p["left_on"]) or len(pairs) != len(keys) + 1:
        return None
    if any(pairs.get(key) != shape["ekeys"][i] for i, key in enumerate(keys)):
        return None
    which = None
    if pairs.get(ncol) == shape["order"]:
        which = "node"
    elif pairs.get(tcol) == shape["order"]:
        which = "target"
    if which is None:
        return None
    if p["right"] == shape["probes"]:
        return p["left"], (0 if which == "node" else 3)
    if p["right"] == shape["events"]:
        return p["left"], (2 if which == "node" else 1)
    return None


def asof_try(self, s):
    """the branches of step s when it is the top of a doubling chain (or semi-joins of an as-of chain with its own probes
    and events): one atom over a keyed made table. None: not that shape (the caller lowers the step as before)."""
    hit = self.made_hit("chain", s)  # share: the node's table when a rule before made it
    if hit is not None:
        return hit
    # peel semi-joins down to a chain top; they only count when the chain under them has the as-of shape
    restrict, v = [], {"step": s}
    chain = _chain(self, s)
    shape = None
    if chain is None and ASOF_SHAPE:
        t, peeled = {"step": s}, []
        while chain is None and _step_of(t) is not None and self.steps[t["step"]]["op"] == "join":
            p = self.steps[t["step"]]["params"]
            c2 = _chain(self, p["left"]["step"]) if _step_of(p["left"]) is not None else None
            peeled.append(t["step"])
            if c2 is not None:
                chain, v = c2, p["left"]
            else:
                t = p["left"]
        if chain is None:
            return None
        base, k, keys, ncol, tcol, w = chain
        shape = _asof_shape(self, base, keys, ncol, tcol, w)
        if shape is None:
            return None
        for js in peeled:
            got = _semi(self, js, shape, keys, ncol, tcol, w)
            if got is None:
                return None
            restrict.append(got[1])
    else:
        if chain is None:
            return None
        base, k, keys, ncol, tcol, w = chain
        shape = _asof_shape(self, base, keys, ncol, tcol, w) if ASOF_SHAPE else None
    if shape is not None:
        E = self.input(shape["events"])
        P = self.input(shape["probes"])
        if not E and not P:
            return []
        some = (E or P)[0]
        we = len(some.out)
        ekinds = [self.tkind(some.out[c]) for c in range(we)]
        kinds = [None] * w
        for i, key in enumerate(keys):
            kinds[key] = ekinds[shape["ekeys"][i]]
        kinds[ncol] = kinds[tcol] = ekinds[shape["order"]]
        name = self.made_name()
        self.made.append(dict(name=name, op="asof", programs=[self.bulk_prog(b) for b in E],
                              right_programs=[self.bulk_prog(b) for b in P], width=we, right_width=we,
                              ekeys=list(shape["ekeys"]), order=shape["order"], order_number=ekinds[shape["order"]] == "number",
                              events_first=shape["events_first"], levels=k, jkeys=list(keys), node=ncol, target=tcol,
                              restrict=[i in restrict for i in range(4)], keys=[], distinct=None, kinds=kinds))
        self.fired.append(["asof", s, k, sorted(restrict)])
    else:
        B = self.input(base)
        if not B:
            return []
        kinds = [self.tkind(t) for t in B[0].out]
        name = self.made_name()
        self.made.append(dict(name=name, op="power", programs=[self.bulk_prog(b) for b in B], width=w,
                              levels=k, jkeys=list(keys), node=ncol, target=tcol, keys=[], distinct=None, kinds=kinds))
        self.fired.append(["power", s, k, []])
    cols = [dict(name=f"c{i}", kind=kd, key=False) for i, kd in enumerate(kinds)]
    self.schema[name] = dict(columns=cols, rows=2873 * 5)
    self.made_done("chain", s, name)
    self.keyed[name] = sorted(keys + (ncol,))
    ts = [("f", 0) if kd == "file" else self.fresh(kd) for kd in kinds]
    return [Br([[name, list(ts)]], out=list(ts))]


def keyed_accesses(self, rel, ts, tbound):
    """a keyed made table read by every bound column at once (cost 1 when they cover its key: at most one row)"""
    key = self.keyed.get(rel)
    if key is None:
        return []
    cols = self.schema[rel]["columns"]
    bc = [c for c, t in enumerate(ts) if cols[c]["kind"] != "file" and tbound(t) and (is_var(t) or t[0] in ("t", "n"))]
    if not bc:
        return []
    covered = all(c in bc or cols[c]["kind"] == "file" for c in key)
    return [(1.0 if covered else 1.2, "byk", [bc, [ts[c] for c in bc]])]


# ---- bind: what a branch cannot hold without (emptiness known before anything is enumerated) ------------------------------
NEED = os.environ.get("NAV_NEED", "1") == "1"
LIGHT_OPS = ("read", "row", "test", "let", "fn", "yield", "collect", "emit")


def branch_needs(self, b):
    """bind: what must exist in the file for the branch to hold a row, read off the branch itself: a kind of node for each
    kind-literal atom, a node carrying each text or field literal, a row (or a row with the literal) of each fact relation
    it reads. None of it depends on a binding, so the executor decides each need once per file; a need that fails ends
    the branch before it enumerates anything. This is the old engine's propagation of empty inputs, stated per branch
    (the tables the planned program probes for rows are added by `with_need`)."""
    kinds, facts = [], []

    def lit_of(t):
        if not is_var(t):
            return t
        return next((r for x, l, r in b.filt if x in ("=", "in") and l == t and not is_var(r)), None)
    for rel, ts in b.atoms:
        if rel in KIND_COL:
            lit = lit_of(ts[KIND_COL[rel]])
            if lit is not None and lit[0] in ("t", "ts"):
                ks = list(lit[1]) if lit[0] == "ts" else [lit[1]]
                if ks and None not in ks and sorted(ks) not in kinds:
                    kinds.append(sorted(ks))
            continue
        if rel in TREE:
            # a text or field literal of a tree relation: some node of the file must carry it
            for c, t in enumerate(ts):
                lit = lit_of(t)
                if lit is not None and lit[0] in ("t", "ts") and (c in (TEXT_COL.get(rel), FIELD_COL.get(rel)) or (rel == "identifier" and c == 1)):
                    vals = [lit[1]] if lit[0] == "t" else list(lit[1])
                    if None not in vals and [rel, c, list(lit)] not in facts:
                        facts.append([rel, c, list(lit)])
            continue
        if rel not in self.schema or rel.startswith("made:"):
            continue
        cols = self.schema[rel]["columns"]
        got = False
        for c, t in enumerate(ts):
            lit = lit_of(t)
            if lit is None or cols[c]["kind"] not in ("symbol", "number"):
                continue
            ok = (lit[0] == "t" and cols[c]["kind"] == "symbol" and lit[1] is not None) or (lit[0] == "n" and cols[c]["kind"] == "number") or \
                 (lit[0] == "ts" and cols[c]["kind"] == "symbol" and None not in lit[1]) or (lit[0] == "ns" and cols[c]["kind"] == "number")
            if ok:
                f = [rel, c, list(lit)]
                if f not in facts:
                    facts.append(f)
                got = True
        if not got and [rel, None, None] not in facts:
            facts.append([rel, None, None])
    return kinds, facts


def with_need(self, b, prog):
    """bind: the program with the branch's needs in front, when it does more than read and test"""
    if all(ins[0] in LIGHT_OPS for ins in prog):
        return prog
    kinds, facts = self.branch_needs(b)
    subs = []
    # the tables the program probes for rows (not for absence): each is one program [["shared", entry]]
    for ins in prog:
        q = ins[2] if ins[0] == "or-probe" else ins[1] if ins[0] == "exists-probe" else None
        if q and len(q) == 1 and q[0] not in subs:
            subs.append(q[0])
        # a keyed call for rows whose definition has a table under the bound (`way`: 'budget'): the same question,
        # asked of that table when it can be built within the bound (written as the table's program and its width)
        k = table_at(ins) if ins[0] in ("or-keyed", "exists-keyed") else None
        if k is not None:
            q = [ins[k][0][0][0], ["test", "=", ["n", ins[k][1]], ["n", ins[k][1]]]]
            if q not in subs:
                subs.append(q)

    def build_cost(q):
        key = tuple(self.shared_meta[q[0][1]][0])
        return self.body_cost.get((key, (), self.all_cols(key)), (COST_CAP, 0.0))[0]
    subs.sort(key=build_cost)  # the cheapest table first: an empty one spares the others
    if not kinds and not facts and not subs:
        return prog
    return [["need", [["ts", ks] for ks in kinds], facts, subs]] + list(prog)


def bulk_prog(self, b):
    """bind: the program of one branch of a made table's input, with its needs in front"""
    prog = self.plan_alts(b.copy())[0]["prog"]
    return self.with_need(b, prog) if NEED else prog


def snapshot(self):
    return (len(self.shared), set(self.body_memo), {k: set(v) for k, v in self.masks.items()}, self.n_bodies,
            set(self.table_memo), set(self.heavy_memo), set(self.plan_memo), set(self.walk_memo), set(self.body_cost),
            set(self.body_branches), set(), set(self.tableable_memo), set(self.body_scans), set(self.body_pairs), set(self.way_memo))


def rollback(self, snap):
    """everything a planning attempt added is taken back (it failed: not plannable without bound args)"""
    del self.shared[snap[0]:]
    del self.shared_meta[snap[0]:]
    self.masks = snap[2]
    self.n_bodies = snap[3]
    for memo, was in ((self.body_memo, snap[1]), (self.table_memo, snap[4]), (self.heavy_memo, snap[5]), (self.plan_memo, snap[6]),
                      (self.walk_memo, snap[7]), (self.body_cost, snap[8]), (self.body_branches, snap[9]),
                      (self.tableable_memo, snap[11]), (self.body_scans, snap[12]), (self.body_pairs, snap[13]), (self.way_memo, snap[14])):
        for k in [k for k in memo if k not in was]:
            del memo[k]


# ---- bind: ONE way to hold a file-level sub-plan ----------------------------------------------------------------------------
# A definition (a sub-plan) is evaluated in one of two forms, chosen by what its plan IS, never by an estimate or at run time:
#   table       its rows once per file (its program with nothing bound, emptiness of its inputs known first), indexed by
#               the columns each use binds: every use is a probe (`or-probe`, `exists-probe`, `neg-probe`; no key = the rows);
#   navigation  per candidate, when the program for the bound columns reaches everything through a key (no bucket, no scan,
#               no table's rows inside the bound context): `or-keyed` / `neg-keyed` / `exists-keyed`, or inlined when it only
#               reads and tests.
# A definition whose rows with nothing bound are every pair of a walk (a closure entered at neither end), or that cannot be
# planned with nothing bound, has no table and stays navigation.
ONE = os.environ.get("NAV_ONE", "1") == "1"
EITHER = os.environ.get("NAV_EITHER", "1") == "1"
BUDGET = os.environ.get("NAV_BUDGET", "1") == "1"  # 0: a use neither form covers is a probe of the table
SMALL_FIRST = os.environ.get("NAV_SMALL_FIRST", "1") == "1"   # bind: plans of equal estimated cost: the smaller start wins
CROSS_RANK = float(os.environ.get("NAV_CROSS_RANK", "1e4"))


def tableable(self, key):
    hit = self.tableable_memo.get(key)
    if hit is None:
        self.tableable_memo[key] = False  # while it is planned (the plan is acyclic)
        hit = False
        allc = self.all_cols(key)
        if self.defs[key] and allc:  # (a definition with no column but the file has no row to hold: a test, as before)
            snap = self.snapshot()
            try:
                self.body(key, (), allc)
                hit = not self.body_pairs.get((key, (), allc), False)  # (every pair of a walk is no table)
            except Blocked:
                self.rollback(snap)
        self.tableable_memo[key] = hit
    return hit


def way(self, key, cols, mask, free, eff):
    """how a use of a definition is evaluated, by what its plans ARE:
    'keyed'   navigation: the program for the bound columns reaches everything through a key;
    'probe'   the table: the program with nothing bound is keyed joins from one start, every use probes its rows;
    'budget'  neither is true (each form holds a cross product: a bucket or a table's rows iterated inside a loop). Which
              is cheaper depends on how many keys the file asks against the size of the product, which no plan shows: the
              keyed call runs, and the table replaces it once the calls have cost what the table would (asof's budget);
    None      no table can be planned and nothing is bound: the older placement."""
    k = (key, mask, free)
    hit = self.way_memo.get(k)
    if hit is None:
        hit = self.way_memo[k] = self.way1(key, cols, mask, free, eff) or "none"
    return None if hit == "none" else hit


def way1(self, key, cols, mask, free, eff):
    allc = self.all_cols(key)
    can_table = all(c in allc for c in cols) and self.tableable(key)
    strict = can_table and not self.body_scans.get((key, (), allc), False)
    if mask and self.mask_room(key, mask):
        try:
            self.body(key, mask, free, eff)
        except Blocked:
            if not can_table:
                raise
            return "probe"
        if not self.body_scans.get((key, mask, free), False):
            # navigation; when the table is clean too, the estimate picks between the two clean forms (cut's costing)
            return None if strict and EITHER else "keyed"
        if not can_table:
            return "keyed"
        return "probe" if strict or not BUDGET else "budget"
    if mask:
        return "probe" if can_table else None  # no program left for this pattern: the table, however it is built
    return "probe" if strict else None


def table_use(self, key, mask, free):
    """the definition's own table for a keyed call: [shared entry of its uncorrelated program collecting every column, row
    width, the key columns' places in the row, the collected columns' places] (bind: the entry is written as the one-program
    list [[["shared", entry]]], which the passes that renumber shared entries rewrite). The executor answers the call from that
    table (one hash lookup) once the per-call evaluation of the definition has cost the file more than the table would
    (it builds the table under a step budget, see nav.rs `table_spend`). None when the definition cannot be planned unbound."""
    if not TABLE or (ONE and self.way_memo.get((key, mask, free if free else None)) != "budget"):
        return None  # (bind: only for the uses neither form covers, see `way`)
    F = self.formals(key)
    allc = tuple(c for c in range(len(F)) if self.kind.get(F[c]) != "file")
    if not allc or any(c not in allc for c in mask) or any(c not in allc for c in free):
        return None
    hit = self.table_memo.get(key)
    if hit is None:
        snap = self.snapshot()
        self.table_memo[key] = False  # while it is planned (the plan is acyclic; a nested use never asks for it)
        try:
            idxs = self.body(key, (), allc)
            hit = idxs[0] if len(idxs) == 1 else False
        except Blocked:
            # not plannable without bound args: everything the attempt added is taken back
            self.rollback(snap)
            hit = False
        self.table_memo[key] = hit
    if hit is False:
        return None
    return [[[["shared", hit]]], len(allc), [allc.index(c) for c in mask], [allc.index(c) for c in free]]


Lowerer.table_use = table_use
Lowerer.tableable = tableable
Lowerer.way = way
Lowerer.way1 = way1
Lowerer.branch_needs = branch_needs
Lowerer.with_need = with_need
Lowerer.bulk_prog = bulk_prog
Lowerer.snapshot = snapshot
Lowerer.rollback = rollback
Lowerer.asof_try = asof_try
Lowerer.keyed_accesses = keyed_accesses
# ====================================================================== end asof ============================================


if __name__ == "__main__":
    sys.setrecursionlimit(20000)
    main()


