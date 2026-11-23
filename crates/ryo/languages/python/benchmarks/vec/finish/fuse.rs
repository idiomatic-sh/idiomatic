//! THROWAWAY (ryo-fmx3.1, redo-fuse): a rule's chain of small steps run as one fused pass, the way a database engine runs
//! a query plan: pipelines and pipeline breakers.
//!
//! THE CUT (once a run, when the plan is loaded; by plan structure alone): a step is INSIDE a group when it has exactly one
//! consumer, is read by it once and as a stream, is no emit root, is no view and is not made once a run. Everything else is
//! a BOUNDARY: its result is a table for the file, made whole once on first need (two or more consumers, an emit root, or
//! an input its consumer needs whole: a join's second source, an antijoin's right side, a group's or sort's input, a
//! closure's edges). That is the "strict" cut; `VEC_FUSE_CUT=<path>` writes it, node by node.
//!
//! THE FUSED PASS: a group whose steps are all of the kinds handled here (`handled`) runs as one pass: at its first run in a
//! process it is compiled into PATHS, one per DRIVER (a leaf input of the group: a boundary table, or a leaf step that
//! enters a view by its kind bucket, run by its own body), each a flat list of typed operators over the path's SOURCES
//! (the driver, one row of the constants the group's extends add, and each boundary a join or antijoin reads). A path's
//! rows are row numbers into its sources (one vector a source, the group's own, reused across files); a select's filter,
//! an extend's constants, a projection and a union's column order are positions fixed at compile time; a join adds a
//! source (the boundary's rows found through its index, made once a file), an antijoin drops rows, a set projection or a
//! union keeps first occurrences. Only the SINK makes a table: the root's columns gathered once from the sources. Between
//! operators there is no table, memo entry, bracket or emptiness decision. A path whose per-file tests fail, whose driver
//! is empty, or whose rows run out stops at that point.
//!
//! FALLBACK: a strict group is split at any step of a kind not handled, or of a form not handled (`form_ok`); those steps
//! run through the step-by-step executor unchanged, and so does a group whose compile refuses it. `VEC_NO_FUSE` turns
//! fusion off; `VEC_FUSE_KINDS=a,b` limits the kinds handled.
//!
//! Every answer is the same set of rows (and, for a bag, the same rows with their counts) as the step-by-step run's;
//! only the order of a join's rows can differ, where the executor would drive the join from its smaller side.

use super::select_kernel::{P, compile, filter, holds};
use super::{E, Exec, File, Native, Op, Provider, R, Res, Run, Src, Table, Ty, emptyprop, key_of, recycle, refs_of, step_clock, strs, table_index, view_cols, view_holds, view_rows};
use rustc_hash::FxHashSet;
use serde_json::Value as J;
use std::cell::RefCell;
use std::rc::Rc;

const NONE: u32 = u32::MAX;
/// Most sources a path may read (the driver, the constants and the boundaries its joins add).
const MAX_SRC: usize = 16;
/// The kinds fused by default (cycle 1). The runner also has union and antijoin (`VEC_FUSE_KINDS=select,extend,union,
/// antijoin`), not gated yet: cycle 1's constants came out far from the assumption, so coverage waits on the why.
const KINDS: [&str; 2] = ["select", "extend"];

/// A path's cell: (source, column).
type Slot = (u8, u32);

/// The cut and the compiled groups.
#[derive(Default)]
pub(crate) struct Fuse {
    /// Per node: whether it is the root of a group run as one pass.
    root: Vec<bool>,
    /// Per node: the root of the fused group it is inside (its own number for a root), NONE where it is in none.
    pub(super) group_of: Vec<u32>,
    groups: Vec<Option<Rc<Group>>>,
    /// The build sides made in this file, by select: (file, rows of the view, index of their positions).
    builds: rustc_hash::FxHashMap<usize, (u32, Rc<Vec<u32>>, Rc<super::Index>)>,
    pub(crate) c: Counts,
}

/// Deterministic counters of the fused runs (printed at the end: `FUSE` lines).
#[derive(Default)]
pub(crate) struct Counts {
    pub(crate) group_runs: u64,
    pub(crate) empty_runs: u64,
    pub(crate) paths: u64,
    pub(crate) paths_stopped: u64,
    pub(crate) driver_rows: u64,
    pub(crate) op_rows: u64,
    pub(crate) out_rows: u64,
    pub(crate) body_drivers: u64,
    pub(crate) passed: u64,
    pub(crate) refused: u64,
    pub(crate) time: f64,
    /// `VEC_FUSE_PROF`: paths by where they ended (a per-file test, the leaf known empty, the driver empty from the
    /// executor, the leaf's body empty, the rows ran out, rows made), and their seconds (monotonic, nested runs out).
    pub(crate) prof: [(u64, f64); 6],
    /// `VEC_FUSE_PROF`: seconds in the drivers (to the first row), the operators, the sinks (nested runs out).
    pub(crate) sect: [f64; 3],
    /// `VEC_FUSE_PROF`: by root: seconds in its drivers, operators, sinks; paths; rows into its operators.
    pub(crate) by_root: rustc_hash::FxHashMap<usize, [f64; 5]>,
}

struct Group {
    names: super::Names,
    tys: super::Tys,
    paths: Vec<Path>,
    sets: usize,
}

enum Driver {
    /// A boundary step (or a view made whole), asked through the executor.
    Eval(usize),
    /// A leaf step of the group run by its own body (a select that enters a view).
    Body(usize),
}

struct Path {
    driver: Driver,
    nsrc: usize,
    /// The constants the path's extends add: one row.
    consts: Option<Rc<Table>>,
    /// The selects' per-file tests on this path (their conjunct lists, as the kernels pass them).
    file_tests: Vec<Rc<super::SelPlan>>,
    ops: Vec<FOp>,
    out: Box<[Slot]>,
    /// The sink may hand on the driver's table itself when every row survives (same columns, same names).
    ident: bool,
}

enum FOp {
    Filter(P, bool),
    Join(Join),
    Dedup(u16, Box<[Slot]>),
    Anti(Anti),
}

struct Join {
    si: u8,
    node: usize,
    how: How,
    checks: Vec<P>,
}

enum How {
    /// The boundary as a table, by its index on `cols` (made once a file), keyed by the driver's `drv` cells.
    Tab(Box<[usize]>, Box<[Slot]>),
    /// The boundary is a view probed in place on its column `col` by the driver's cell, or through its build side where
    /// its entry is smaller than the driver's rows (empty: nothing joins).
    View(usize, Slot, Box<Build>),
    /// No key joins it: every pair.
    Cross,
    /// The boundary is a view with no keyed read on the join column: its BUILD SIDE is made once a file (the entry the
    /// kernels would drive from: the select's kind bucket or literal rows of the view, its own tests, then the select's
    /// tests on this source alone), indexed on `cols`, keyed by the driver's `drv` cells.
    Build(Box<Build>),
}

struct Build {
    sel: usize,
    sp: Rc<super::SelPlan>,
    cols: Box<[usize]>,
    drv: Box<[Slot]>,
    /// The select's tests on this source alone (its own coordinates: source 1).
    own: Vec<P>,
    kinds: (Option<Vec<u32>>, Option<Vec<u32>>),
}

struct Anti {
    node: usize,
    left: Box<[Slot]>,
    rk: Box<[usize]>,
    rfile: Box<[bool]>,
    view_key: Option<usize>,
}

/// A path under compile: its driver, sources so far, the cell of each column of the step reached.
struct Pb {
    driver: Driver,
    srcs: Vec<Option<usize>>,
    consts: Vec<(Ty, u64)>,
    file_tests: Vec<Rc<super::SelPlan>>,
    ops: Vec<FOp>,
    map: Vec<Slot>,
    /// Rows are only filtered (never joined, deduplicated by a union, re-mapped): the driver's columns as they were.
    ident: bool,
}

#[derive(Default)]
struct Work {
    rows: Vec<Vec<u32>>,
    next: Vec<Vec<u32>>,
    hits: Vec<u32>,
    key: Vec<u64>,
    s1: Vec<FxHashSet<u64>>,
    s2: Vec<FxHashSet<(u64, u64)>>,
    sk: Vec<FxHashSet<super::inline_key::Key>>,
    keep: Vec<bool>,
    srcs: Vec<Src>,
}

thread_local! {
    /// The groups' scratch: a stack, since a boundary a path asks may run another group.
    static WORK: RefCell<Vec<Work>> = const { RefCell::new(Vec::new()) };
}

fn kinds() -> Vec<String> {
    match std::env::var("VEC_FUSE_KINDS") {
        Ok(v) => v.split(',').map(str::to_owned).collect(),
        Err(_) => KINDS.iter().map(|k| (*k).to_owned()).collect(),
    }
}

impl Exec<'_> {
    /// The cut, once a run when the plan is loaded (redo-fuse).
    pub(crate) fn fuse_load(&mut self, run: &mut Run) {
        let k = self.plan.nodes.len();
        self.fuse = Fuse { root: vec![false; k], group_of: vec![NONE; k], groups: vec![None; k], builds: rustc_hash::FxHashMap::default(), c: Counts::default() };
        let (strict, why) = self.strict_cut(run);
        // the strict cut's counts (the fusion diagnosis's figures are 10,545 groups, 17,623 inner steps)
        let running: Vec<bool> = (0..k).map(|i| self.view(run, i).ok().flatten().is_none()).collect();
        let inner = strict.iter().filter(|p| **p != NONE).count();
        let roots = (0..k).filter(|i| running[*i] && strict[*i] == NONE).count();
        let mut by_kind: std::collections::BTreeMap<&str, (u32, u32)> = std::collections::BTreeMap::new();
        for i in 0..k {
            if running[i] {
                let e = by_kind.entry(self.plan.nodes[i].op.as_str()).or_default();
                e.0 += 1;
                if strict[i] != NONE {
                    e.1 += 1;
                }
            }
        }
        println!("FUSE cut strict: steps that run {} groups {roots} inner {inner}", running.iter().filter(|x| **x).count());
        println!("FUSE cut strict by kind (steps inner): {}", by_kind.iter().map(|(k, (a, b))| format!("{k} {a} {b}")).collect::<Vec<_>>().join(", "));
        // the fused cut: the strict cut split at what is not handled
        let ks = kinds();
        let ok: Vec<bool> = (0..k).map(|i| running[i] && ks.iter().any(|x| *x == self.plan.nodes[i].op) && self.form_ok(run, i)).collect();
        let mut parent = vec![NONE; k];
        for i in 0..k {
            let q = strict[i];
            if q != NONE && ok[i] && ok[q as usize] {
                parent[i] = q;
            }
        }
        for i in 0..k {
            if parent[i] != NONE {
                let mut r = i;
                while parent[r] != NONE {
                    r = parent[r] as usize;
                }
                self.fuse.group_of[i] = r as u32;
                self.fuse.group_of[r] = r as u32;
                self.fuse.root[r] = true;
            }
        }
        let fin = parent.iter().filter(|p| **p != NONE).count();
        let groups = self.fuse.root.iter().filter(|x| **x).count();
        let mut fk: std::collections::BTreeMap<&str, u32> = std::collections::BTreeMap::new();
        for i in 0..k {
            if parent[i] != NONE {
                *fk.entry(self.plan.nodes[i].op.as_str()).or_default() += 1;
            }
        }
        println!("FUSE cut fused (kinds {}): groups {groups} inner {fin}; inner by kind: {}", ks.join(","), fk.iter().map(|(k, n)| format!("{k} {n}")).collect::<Vec<_>>().join(", "));
        if let Some(path) = std::env::var_os("VEC_FUSE_CUT") {
            let mut s = String::from("node\top\tstrict_parent\twhy\tfused_group\n");
            for i in 0..k {
                use std::fmt::Write as _;
                let _ = writeln!(s, "{i}\t{}\t{}\t{}\t{}", self.plan.nodes[i].op, if strict[i] == NONE { -1 } else { i64::from(strict[i]) }, why[i], if self.fuse.group_of[i] == NONE { -1 } else { i64::from(self.fuse.group_of[i]) });
            }
            let _ = std::fs::write(path, s);
        }
        if sw!("VEC_NO_FUSE") {
            self.fuse.root.iter_mut().for_each(|x| *x = false);
        }
    }

    /// The strict cut: per node, the consumer it streams into, or NONE (a boundary, a view, or nothing reads it); and why.
    fn strict_cut(&mut self, run: &mut Run) -> (Vec<u32>, Vec<&'static str>) {
        let plan = self.plan;
        let k = plan.nodes.len();
        let mut parent = vec![NONE; k];
        let mut why = vec![""; k];
        for i in 0..k {
            if self.view(run, i).ok().flatten().is_some() {
                why[i] = "view";
                continue;
            }
            if self.emitted.contains(&i) {
                why[i] = "emit root";
                continue;
            }
            let cons = &self.consumers[i];
            if cons.len() != 1 {
                why[i] = if cons.is_empty() { "no consumer" } else { "two or more consumers" };
                continue;
            }
            let c = cons[0];
            if plan.nodes[c].inputs.iter().filter(|x| **x == i).count() != 1 {
                why[i] = "read twice by its consumer";
                continue;
            }
            if self.is_constant(i) {
                why[i] = "made once a run";
                continue;
            }
            if !streams(plan, c, i) {
                why[i] = "its consumer needs it whole";
                continue;
            }
            parent[i] = c as u32;
        }
        (parent, why)
    }

    /// Whether the fused runner handles this step's form (its kind is handled).
    fn form_ok(&mut self, run: &mut Run, node: usize) -> bool {
        if self.is_constant(node) {
            return false;
        }
        let plan = self.plan;
        let pn = &plan.nodes[node];
        match pn.op.as_str() {
            "select" => {
                if pn.j.get("count").is_some_and(|c| !c.is_null()) {
                    return false;
                }
                let Ok(sp) = self.sel(run, node) else { return false };
                let k = sp.srcs.len();
                if k == 0 || k > 2 || sp.srcs.iter().any(|n| plan.nodes[*n].op == "closure") || sp.items.iter().any(|(_, e)| !matches!(e, E::Col(..))) {
                    return false;
                }
                if k == 2 && sp.srcs[0] == sp.srcs[1] {
                    return false;
                }
                // a join key that is not a column (the kernels refuse it too)
                if k == 2 {
                    for e in &sp.conj {
                        if let E::Cmp(Op::Eq, a, b) = e
                            && refs_of(e) == 3
                            && (matches!(**a, E::Col(..)) || matches!(**b, E::Col(..)))
                            && !(matches!(**a, E::Col(..)) && matches!(**b, E::Col(..)))
                        {
                            return false;
                        }
                    }
                }
                true
            }
            "extend" => pn.j["args"]["values"].as_object().is_some_and(|m| m.values().all(|v| matches!(v, J::Number(n) if n.as_i64().is_some()) || v.is_string())),
            "union" => true,
            "antijoin" => {
                let (Ok(l), Ok(r)) = (self.arg_rel(node, "left"), self.arg_rel(node, "right")) else { return false };
                l != r && match (self.schema(run, node), self.schema(run, l)) {
                    (Ok(a), Ok(b)) => a.names == b.names,
                    _ => false,
                }
            }
            _ => false,
        }
    }

    #[inline]
    pub(super) fn fused_root(&self, node: usize) -> bool {
        self.fuse.root.get(node).copied().unwrap_or(false)
    }

    /// The hook's body: the group's root asked in this file: the group run as one pass, inside the step bracket the
    /// executor's `res` gives every run (its time, counts, memo entry).
    pub(super) fn fused_res(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize, why: &'static str) -> R<Option<Res>> {
        let g = match &self.fuse.groups[node] {
            Some(g) => g.clone(),
            None => match self.fuse_compile(run, node) {
                Ok(g) => {
                    let g = Rc::new(g);
                    self.fuse.groups[node] = Some(g.clone());
                    g
                }
                Err(e) => {
                    if sw!("VEC_FUSE_LOG") {
                        eprintln!("FUSE refused {node}: {e}");
                    }
                    self.fuse.c.refused += 1;
                    self.fuse.root[node] = false;
                    return Ok(None);
                }
            },
        };
        let t0 = step_clock();
        let saved = self.inner;
        self.inner = 0.0;
        if self.stats.made_by[node] == usize::MAX {
            self.stats.made_by[node] = self.stack.last().copied().unwrap_or(usize::MAX - 1);
            self.stats.why[node] = why;
        }
        self.stack.push(node);
        let mut w = WORK.with(|w| w.borrow_mut().pop()).unwrap_or_default();
        let out = self.fused_run(f, run, p, node, &g, &mut w);
        WORK.with(|s| s.borrow_mut().push(w));
        self.stack.pop();
        let dt = step_clock() - t0;
        let own = dt - self.inner;
        self.stats.time[node] += own;
        self.fuse.c.time += own;
        self.inner = saved + dt;
        let out = out?;
        if out.n == 0 {
            self.stats.empty_time[node] += own;
            emptyprop::empty_run(node, false, own);
            self.stats.empty[node] += 1;
            self.fuse.c.empty_runs += 1;
        }
        emptyprop::ran();
        self.stats.calls[node] += 1;
        self.stats.rows[node] += out.n as u64;
        self.fuse.c.group_runs += 1;
        self.fuse.c.out_rows += out.n as u64;
        let rc = Res::Tab(out);
        f.touched.push(node as u32);
        f.memo[node] = Some(rc.clone());
        f.log.push(node);
        Ok(Some(rc))
    }

    // -------------------------------------------------------------------------------------------- compile

    fn fuse_compile(&mut self, run: &mut Run, root: usize) -> R<Group> {
        let mut sets = 0usize;
        let paths = self.fc_node(run, root, root, &mut sets)?;
        let sc = self.schema(run, root)?;
        let mut out = Vec::with_capacity(paths.len());
        for pb in paths {
            if pb.srcs.len() > MAX_SRC {
                return Err("too many sources".into());
            }
            let dn = match pb.driver {
                Driver::Eval(n) | Driver::Body(n) => n,
            };
            let dsc = self.schema(run, dn)?;
            let ident = pb.ident && pb.map.len() == dsc.names.len() && pb.map.iter().enumerate().all(|(i, s)| *s == (0, i as u32)) && dsc.names == sc.names;
            let consts = if pb.consts.is_empty() {
                None
            } else {
                let names: super::Names = Rc::new((0..pb.consts.len()).map(|i| format!("c{i}")).collect());
                let tys: super::Tys = Rc::new(pb.consts.iter().map(|c| c.0).collect());
                let mut t = Table::with(names, tys);
                let row: Vec<u64> = pb.consts.iter().map(|c| c.1).collect();
                t.push_row(&row);
                Some(Rc::new(t))
            };
            out.push(Path { driver: pb.driver, nsrc: pb.srcs.len(), consts, file_tests: pb.file_tests, ops: pb.ops, out: pb.map.into_boxed_slice(), ident });
        }
        Ok(Group { names: sc.names.clone(), tys: sc.tys.clone(), paths: out, sets })
    }

    /// The paths of `node`'s rows inside the group of `root` (each ends at `node`'s columns).
    fn fc_node(&mut self, run: &mut Run, root: usize, node: usize, sets: &mut usize) -> R<Vec<Pb>> {
        let plan = self.plan;
        let op = plan.nodes[node].op.clone();
        let inside = |me: &Self, x: usize| me.fuse.group_of[x] == root as u32 && x != root || (x == root && false);
        let _ = inside;
        let member = |me: &Self, x: usize| x != node && me.fuse.group_of[x] == root as u32 && x != root;
        // a stream input: its paths (inside the group), or a path driven by it (a boundary or a view)
        let stream = |me: &mut Self, run: &mut Run, x: usize, sets: &mut usize| -> R<Vec<Pb>> {
            if member(me, x) {
                me.fc_node(run, root, x, sets)
            } else {
                let sc = me.schema(run, x)?;
                Ok(vec![Pb { driver: Driver::Eval(x), srcs: vec![Some(x), None], consts: Vec::new(), file_tests: Vec::new(), ops: Vec::new(), map: (0..sc.names.len() as u32).map(|c| (0, c)).collect(), ident: true }])
            }
        };
        match op.as_str() {
            "select" => {
                let sp = self.sel(run, node)?;
                let s0 = sp.srcs[0];
                if !member(self, s0) && self.view(run, s0)?.is_some() {
                    // a leaf that enters a view: run by its own body, the path's driver
                    let sc = self.schema(run, node)?;
                    return Ok(vec![Pb { driver: Driver::Body(node), srcs: vec![Some(node), None], consts: Vec::new(), file_tests: Vec::new(), ops: Vec::new(), map: (0..sc.names.len() as u32).map(|c| (0, c)).collect(), ident: true }]);
                }
                let mut paths = stream(self, run, s0, sets)?;
                let width = self.schema(run, s0)?.names.len();
                let set = sp.srcs.len() == 1 && sp.items.iter().all(|(_, e)| matches!(e, E::Col(0, _, _))) && !sw!("VEC_SELECT_BAG") && sp.items.len() < width;
                let file_const = sp.conj.iter().any(|c| super::filter_order::file_const(c) && !matches!(c, E::Lit(_)));
                for pb in &mut paths {
                    if file_const {
                        pb.file_tests.push(sp.clone());
                    }
                    self.fc_select(run, node, &sp, pb, set, sets)?;
                }
                Ok(paths)
            }
            "extend" => {
                let from = self.frames[node].first as usize;
                let mut paths = stream(self, run, from, sets)?;
                let vals: Vec<(Ty, u64)> = plan.nodes[node].j["args"]["values"]
                    .as_object()
                    .ok_or("extend values")?
                    .values()
                    .map(|v| match v {
                        J::Number(n) => Ok((Ty::Num, n.as_i64().ok_or("extend number")? as u64)),
                        J::String(t) if t == "true" || t == "false" => Ok((Ty::Flag, u64::from(t == "true"))),
                        J::String(t) => Ok((Ty::Sym, u64::from(run.sym(t.as_bytes())))),
                        _ => Err(format!("extend value {v}")),
                    })
                    .collect::<R<_>>()?;
                for pb in &mut paths {
                    for v in &vals {
                        pb.map.push((1, pb.consts.len() as u32));
                        pb.consts.push(*v);
                    }
                    pb.ident = false;
                }
                Ok(paths)
            }
            "union" => {
                let sc = self.schema(run, node)?;
                // a union read by a union of the group: the outer one keeps first occurrences of the same columns
                let outer_union = self.fuse.group_of[node] == root as u32 && node != root && {
                    let c = self.consumers[node][0];
                    plan.nodes[c].op == "union"
                };
                let set = *sets;
                if !outer_union {
                    *sets += 1;
                }
                let mut all = Vec::new();
                for side in ["left", "right"] {
                    let x = self.arg_rel(node, side)?;
                    let xsc = self.schema(run, x)?;
                    let at: Vec<usize> = sc.names.iter().map(|c| super::pos(&xsc.names, c)).collect::<R<_>>()?;
                    for mut pb in stream(self, run, x, sets)? {
                        pb.map = at.iter().map(|c| pb.map[*c]).collect();
                        if !outer_union {
                            pb.ops.push(FOp::Dedup(set as u16, pb.map.clone().into_boxed_slice()));
                        }
                        pb.ident = false;
                        all.push(pb);
                    }
                }
                Ok(all)
            }
            "antijoin" => {
                let a = &plan.nodes[node].j["args"];
                let (l, r) = (self.arg_rel(node, "left")?, self.arg_rel(node, "right")?);
                let (lsc, rsc) = (self.schema(run, l)?, self.schema(run, r)?);
                let lk: Vec<usize> = strs(&a["left-on"])?.iter().map(|c| super::pos(&lsc.names, c)).collect::<R<_>>()?;
                let rk: Box<[usize]> = strs(&a["right-on"])?.iter().map(|c| super::pos(&rsc.names, c)).collect::<R<_>>()?;
                let rfile: Box<[bool]> = rk.iter().map(|c| rsc.tys[*c] == Ty::File).collect();
                let view_key = match self.view(run, r)? {
                    Some(v) => (0..rk.len()).find(|i| File::probes(v.base, v.map[rk[*i]])),
                    None => None,
                };
                let mut paths = stream(self, run, l, sets)?;
                for pb in &mut paths {
                    let left = lk.iter().map(|c| pb.map[*c]).collect();
                    pb.ops.push(FOp::Anti(Anti { node: r, left, rk: rk.clone(), rfile: rfile.clone(), view_key }));
                }
                Ok(paths)
            }
            other => Err(format!("kind {other} not handled")),
        }
    }

    /// A select applied to a path whose rows are its first source: the kernels' plan with the stream as the driver.
    fn fc_select(&mut self, run: &mut Run, node: usize, sp: &super::SelPlan, pb: &mut Pb, set: bool, sets: &mut usize) -> R<()> {
        let conj = &sp.conj;
        let k = sp.srcs.len();
        let osi = pb.srcs.len() as u8;
        // a select's own (source, column) to the path's cell
        let to = |pb: &Pb, s: usize, c: usize| -> Slot { if s == 0 { pb.map[c] } else { (osi, c as u32) } };
        let rm = |pb: &Pb, e: &E| -> E { remap_e(e, &|s, c| to(pb, s, c)) };
        let mut used: u64 = 0;
        let refs = &sp.refs;
        for (ci, e) in conj.iter().enumerate() {
            if refs[ci] & !1 == 0 {
                used |= 1 << ci;
                let re = rm(pb, e);
                let single = refs_of(&re) & !0b11 == 0;
                pb.ops.push(FOp::Filter(compile(&re), single));
                pb.ident &= true;
            }
        }
        if k == 2 {
            let other = sp.srcs[1];
            let view = self.view(run, other)?;
            // the join column: an unused equality of the other side's column with a column of the driver, one probed in place first
            let mut pick: Option<(usize, usize, bool)> = None;
            for (ci, c) in conj.iter().enumerate() {
                if used & (1 << ci) != 0 {
                    continue;
                }
                if let E::Cmp(Op::Eq, x, y) = c {
                    for (a, bb) in [(x, y), (y, x)] {
                        if let E::Col(si, col, _) = **a
                            && si == 1
                            && refs_of(bb) == 1
                        {
                            let native = match &view {
                                Some(v) => File::probes(v.base, v.map[col]),
                                None => false,
                            };
                            if pick.is_none_or(|q| native && !q.2) {
                                pick = Some((col, ci, native));
                            }
                        }
                    }
                }
            }
            pb.srcs.push(Some(other));
            let how = match pick {
                Some((col, ci, native)) => {
                    used |= 1 << ci;
                    let dc = match &conj[ci] {
                        E::Cmp(_, x, y) => match (&**x, &**y) {
                            (E::Col(a, c1, _), E::Col(_, c2, _)) if *a == 1 && *c1 == col => *c2,
                            (E::Col(_, c1, _), _) => *c1,
                            _ => return Err("a join key that is not a column".to_owned()),
                        },
                        _ => unreachable!(),
                    };
                    // the build side's key: every equality of a column of this source with a column of the driver
                    let mut tab_cols = vec![col];
                    let mut drv = vec![to(pb, 0, dc)];
                    for (cj, c) in conj.iter().enumerate() {
                        if used & (1 << cj) != 0 {
                            continue;
                        }
                        if let E::Cmp(Op::Eq, x, y) = c {
                            for (a2, b2) in [(x, y), (y, x)] {
                                if let E::Col(s3, c3, _) = **a2
                                    && s3 == 1
                                    && refs_of(b2) == 1
                                    && !tab_cols.contains(&c3)
                                {
                                    let E::Col(_, c4, _) = **b2 else { return Err("a join key that is not a column".to_owned()) };
                                    tab_cols.push(c3);
                                    drv.push(to(pb, 0, c4));
                                    break;
                                }
                            }
                        }
                    }
                    let build = |me: &mut Self, run: &mut Run, used: &mut u64, mark: bool| -> R<Box<Build>> {
                        let v = view.as_ref().expect("a view");
                        let mut own = Vec::new();
                        for (cj, e) in conj.iter().enumerate() {
                            if *used & (1 << cj) == 0 && refs[cj] == 2 {
                                own.push(compile(e));
                                if mark {
                                    *used |= 1 << cj;
                                }
                            }
                        }
                        let kinds = (super::kind_set(run, &v.filter, 0, v.base, &[]), super::kind_set(run, &sp.conj, 1, v.base, &v.map));
                        let sp = me.sel(run, node)?;
                        Ok(Box::new(Build { sel: node, sp, cols: tab_cols.clone().into(), drv: drv.clone().into(), own, kinds }))
                    };
                    if view.is_some() && native {
                        // probed in place by each driver row, or (where the view's entry is the smaller, as the kernels
                        // choose their driver) joined through its build side
                        let b = build(self, run, &mut used, false)?;
                        How::View(col, to(pb, 0, dc), b)
                    } else {
                        if tab_cols.len() > 1 {
                            used &= !(1 << ci);
                        }
                        match &view {
                            Some(_) => How::Build(build(self, run, &mut used, true)?),
                            None => How::Tab(tab_cols.into(), drv.into()),
                        }
                    }
                }
                None => How::Cross,
            };
            let mut checks = Vec::new();
            for (ci, e) in conj.iter().enumerate() {
                if used & (1 << ci) == 0 && refs[ci] & !3 == 0 {
                    checks.push(compile(&rm(pb, e)));
                    used |= 1 << ci;
                }
            }
            pb.ops.push(FOp::Join(Join { si: osi, node: other, how, checks }));
            pb.ident = false;
        }
        let all: u64 = if conj.len() >= 64 { u64::MAX } else { (1u64 << conj.len()) - 1 };
        if used != all {
            return Err("a test no source binds".to_owned());
        }
        let map: Vec<Slot> = sp.items.iter().map(|(_, e)| match e {
            E::Col(s, c, _) => to(pb, *s, *c),
            _ => (0, 0),
        }).collect();
        if map != pb.map {
            pb.ident = false;
        }
        pb.map = map;
        if set {
            pb.ops.push(FOp::Dedup(*sets as u16, pb.map.clone().into_boxed_slice()));
            *sets += 1;
            pb.ident = false;
        }
        let _ = node;
        Ok(())
    }

    // -------------------------------------------------------------------------------------------- run

    fn fused_run(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, root: usize, g: &Group, w: &mut Work) -> R<Rc<Table>> {
        // only the sets this group uses, and only where a run left rows in them (a clear costs a set's room)
        for i in 0..g.sets.min(w.s1.len()) {
            if !w.s1[i].is_empty() {
                w.s1[i].clear();
            }
            if !w.s2[i].is_empty() {
                w.s2[i].clear();
            }
            if !w.sk[i].is_empty() {
                w.sk[i].clear();
            }
        }
        if w.s1.len() < g.sets {
            w.s1.resize_with(g.sets, FxHashSet::default);
            w.s2.resize_with(g.sets, FxHashSet::default);
            w.sk.resize_with(g.sets, FxHashSet::default);
        }
        let mut out: Option<Table> = None;
        let prof = sw!("VEC_FUSE_PROF");
        for path in &g.paths {
            self.fuse.c.paths += 1;
            let pt = if prof { (super::select_kernel::ticks(), self.inner) } else { (0.0, 0.0) };
            macro_rules! stop {
                ($k:expr) => {
                    if prof {
                        let e = &mut self.fuse.c.prof[$k];
                        e.0 += 1;
                        e.1 += super::select_kernel::ticks() - pt.0 - (self.inner - pt.1);
                    }
                };
            }
            if path.file_tests.iter().any(|sp| super::filter_order::file_fails(run, f, &sp.conj)) {
                self.fuse.c.paths_stopped += 1;
                stop!(0);
                continue;
            }
            let (dt, dn) = match path.driver {
                Driver::Eval(n) => (self.eval(f, run, p, n)?, n),
                Driver::Body(n) => {
                    self.fuse.c.body_drivers += 1;
                    if self.known_empty(f, run, p, n) {
                        self.fuse.c.paths_stopped += 1;
                        stop!(1);
                        continue;
                    }
                    (self.body(f, run, p, n)?, usize::MAX - 41)
                }
            };
            if dt.n == 0 {
                self.fuse.c.paths_stopped += 1;
                stop!(if matches!(path.driver, Driver::Body(_)) { 3 } else { 2 });
                continue;
            }
            self.fuse.c.driver_rows += dt.n as u64;
            let lap = |me: &mut Self, k: usize, at: &mut (f64, f64)| {
                if prof {
                    let t = super::select_kernel::ticks();
                    let d = t - at.0 - (me.inner - at.1);
                    me.fuse.c.sect[k] += d;
                    me.fuse.c.by_root.entry(root).or_default()[k] += d;
                    *at = (t, me.inner);
                }
            };
            let mut at = pt;
            lap(self, 0, &mut at);
            // a sole path that keeps every row of its driver hands the driver's table on
            w.srcs.clear();
            w.srcs.push(Src::Tab(dt.clone(), dn));
            w.srcs.push(match &path.consts {
                Some(c) => Src::Tab(c.clone(), usize::MAX - 42),
                None => Src::Pending(usize::MAX),
            });
            for _ in 2..path.nsrc {
                w.srcs.push(Src::Pending(usize::MAX));
            }
            if w.rows.len() < path.nsrc {
                w.rows.resize_with(path.nsrc, Vec::new);
                w.next.resize_with(path.nsrc, Vec::new);
            }
            w.rows[0].clear();
            w.rows[0].extend(0..dt.n as u32);
            let mut live: u64 = 1;
            let mut alive = true;
            for op in &path.ops {
                let n = w.rows[0].len();
                if n == 0 {
                    alive = false;
                    break;
                }
                self.fuse.c.op_rows += n as u64;
                if prof {
                    self.fuse.c.by_root.entry(root).or_default()[4] += n as f64;
                }
                match op {
                    FOp::Filter(pr, single) => {
                        if *single && live == 1 {
                            let mut sel = std::mem::take(&mut w.rows[0]);
                            filter(run, f, &w.srcs, 0, pr, &mut sel);
                            w.rows[0] = sel;
                        } else {
                            let mut cur = [0u32; MAX_SRC];
                            w.keep.clear();
                            for i in 0..n {
                                load(&w.rows, live, i, &mut cur);
                                w.keep.push(holds(run, f, &w.srcs, &cur[..path.nsrc], pr));
                            }
                            compact(&mut w.rows, live, &w.keep);
                        }
                    }
                    FOp::Dedup(set, slots) => {
                        let mut cur = [0u32; MAX_SRC];
                        w.keep.clear();
                        let set = *set as usize;
                        match slots.len() {
                            1 => {
                                let s = &mut w.s1[set];
                                for i in 0..n {
                                    load(&w.rows, live, i, &mut cur);
                                    let v = super::get(run, f, &w.srcs, &cur, slots[0].0 as usize, slots[0].1 as usize);
                                    w.keep.push(s.insert(v));
                                }
                            }
                            2 => {
                                let s = &mut w.s2[set];
                                for i in 0..n {
                                    load(&w.rows, live, i, &mut cur);
                                    let a = super::get(run, f, &w.srcs, &cur, slots[0].0 as usize, slots[0].1 as usize);
                                    let b = super::get(run, f, &w.srcs, &cur, slots[1].0 as usize, slots[1].1 as usize);
                                    w.keep.push(s.insert((a, b)));
                                }
                            }
                            _ => {
                                for i in 0..n {
                                    load(&w.rows, live, i, &mut cur);
                                    w.key.clear();
                                    for (s, c) in slots.iter() {
                                        w.key.push(super::get(run, f, &w.srcs, &cur, *s as usize, *c as usize));
                                    }
                                    let s = &mut w.sk[set];
                                    let fresh = !s.contains(&w.key[..]);
                                    if fresh {
                                        s.insert(super::inline_key::Key::of(&w.key));
                                    }
                                    w.keep.push(fresh);
                                }
                            }
                        }
                        compact(&mut w.rows, live, &w.keep);
                    }
                    FOp::Join(j) => {
                        let si = j.si as usize;
                        let mut cur = [0u32; MAX_SRC];
                        for s in 0..path.nsrc {
                            if live & (1 << s) != 0 || s == si {
                                w.next[s].clear();
                            }
                        }
                        match &j.how {
                            How::Tab(cols, drv) => {
                                let t = self.eval(f, run, p, j.node)?;
                                if t.n == 0 {
                                    alive = false;
                                    break;
                                }
                                let ix = table_index(f, j.node, &t, cols);
                                w.srcs[si] = Src::Tab(t, j.node);
                                for i in 0..n {
                                    load(&w.rows, live, i, &mut cur);
                                    let key = if drv.len() == 1 {
                                        super::get(run, f, &w.srcs, &cur, drv[0].0 as usize, drv[0].1 as usize)
                                    } else {
                                        w.key.clear();
                                        for (s, c) in drv.iter() {
                                            w.key.push(super::get(run, f, &w.srcs, &cur, *s as usize, *c as usize));
                                        }
                                        key_of(w.key.iter().copied())
                                    };
                                    let Some(hs) = ix.get(&key) else { continue };
                                    for h in hs {
                                        cur[si] = *h;
                                        if j.checks.iter().all(|c| holds(run, f, &w.srcs, &cur[..path.nsrc], c)) {
                                            push(&mut w.next, live, &cur, si);
                                        }
                                    }
                                }
                            }
                            How::View(col, d, b) => {
                                let Res::View(v) = self.res(f, run, p, j.node)? else { return Err("a view joined is no view".to_owned()) };
                                let size = self.fuse_entry_len(f, run, j.node, &v, b);
                                if size == Some(0) {
                                    alive = false;
                                    break;
                                }
                                if size.is_some_and(|z| z < n) {
                                    let (rows, ix) = self.fuse_build(f, run, j.node, &v, b);
                                    w.srcs[si] = Src::View(v.clone(), j.node);
                                    probe_build(run, f, &mut w.srcs, &w.rows, &mut w.next, &mut w.key, live, n, si, path.nsrc, &b.drv, &rows, &ix, &j.checks);
                                    live |= 1 << si;
                                    std::mem::swap(&mut w.rows, &mut w.next);
                                    continue;
                                }
                                let (base, bcol) = (v.base, v.map[*col]);
                                w.srcs[si] = Src::View(v.clone(), j.node);
                                for i in 0..n {
                                    load(&w.rows, live, i, &mut cur);
                                    let x = super::get(run, f, &w.srcs, &cur, d.0 as usize, d.1 as usize);
                                    w.hits.clear();
                                    f.native_probe(run, base, bcol, x, &mut w.hits);
                                    view_cols::retain(f, run, &v, &mut w.hits);
                                    for h in &w.hits {
                                        cur[si] = *h;
                                        if j.checks.iter().all(|c| holds(run, f, &w.srcs, &cur[..path.nsrc], c)) {
                                            push(&mut w.next, live, &cur, si);
                                        }
                                    }
                                }
                            }
                            How::Build(b) => {
                                let Res::View(v) = self.res(f, run, p, j.node)? else { return Err("a view joined is no view".to_owned()) };
                                let (rows, ix) = self.fuse_build(f, run, j.node, &v, b);
                                w.srcs[si] = Src::View(v.clone(), j.node);
                                probe_build(run, f, &mut w.srcs, &w.rows, &mut w.next, &mut w.key, live, n, si, path.nsrc, &b.drv, &rows, &ix, &j.checks);
                            }
                            How::Cross => {
                                let others: Rc<Vec<u32>> = match self.res(f, run, p, j.node)? {
                                    Res::Tab(t) => {
                                        let r = Rc::new((0..t.n as u32).collect());
                                        w.srcs[si] = Src::Tab(t, j.node);
                                        r
                                    }
                                    Res::View(v) => {
                                        let r = view_rows(f, run, &v, j.node);
                                        w.srcs[si] = Src::View(v, j.node);
                                        r
                                    }
                                };
                                for i in 0..n {
                                    load(&w.rows, live, i, &mut cur);
                                    for h in others.iter() {
                                        cur[si] = *h;
                                        if j.checks.iter().all(|c| holds(run, f, &w.srcs, &cur[..path.nsrc], c)) {
                                            push(&mut w.next, live, &cur, si);
                                        }
                                    }
                                }
                            }
                        }
                        live |= 1 << si;
                        std::mem::swap(&mut w.rows, &mut w.next);
                    }
                    FOp::Anti(a) => {
                        let mut cur = [0u32; MAX_SRC];
                        w.keep.clear();
                        if let Some(ki) = a.view_key
                            && let Res::View(v) = self.res(f, run, p, a.node)?
                        {
                            let probe_col = v.map[a.rk[ki]];
                            for i in 0..n {
                                load(&w.rows, live, i, &mut cur);
                                w.key.clear();
                                for (s, c) in a.left.iter() {
                                    w.key.push(super::get(run, f, &w.srcs, &cur, *s as usize, *c as usize));
                                }
                                w.hits.clear();
                                f.native_probe(run, v.base, probe_col, w.key[ki], &mut w.hits);
                                let key = &w.key;
                                let knocked = w.hits.iter().any(|h| view_holds(f, run, &v, *h) && (0..a.rk.len()).all(|k| a.rfile[k] || view_cols::cell(f, run, v.base, *h, v.map[a.rk[k]]) == key[k]));
                                w.keep.push(!knocked);
                            }
                        } else {
                            let r = self.eval(f, run, p, a.node)?;
                            if r.n == 0 {
                                continue;
                            }
                            let ix = table_index(f, a.node, &r, &a.rk);
                            for i in 0..n {
                                load(&w.rows, live, i, &mut cur);
                                w.key.clear();
                                for (s, c) in a.left.iter() {
                                    w.key.push(super::get(run, f, &w.srcs, &cur, *s as usize, *c as usize));
                                }
                                let hit = match ix.get(&key_of(w.key.iter().copied())) {
                                    None => false,
                                    Some(hs) if a.rk.len() == 1 => !hs.is_empty(),
                                    Some(hs) => hs.iter().any(|h| a.rk.iter().zip(&w.key).all(|(c, v)| r.cols[*c][*h as usize] == *v)),
                                };
                                w.keep.push(!hit);
                            }
                        }
                        compact(&mut w.rows, live, &w.keep);
                    }
                }
            }
            if !alive || w.rows[0].is_empty() {
                self.fuse.c.paths_stopped += 1;
                stop!(4);
                continue;
            }
            lap(self, 1, &mut at);
            let n = w.rows[0].len();
            if path.ident && g.paths.len() == 1 && n == dt.n {
                self.fuse.c.passed += 1;
                return Ok(dt);
            }
            let o = out.get_or_insert_with(|| Table::with_rows(g.names.clone(), g.tys.clone(), n));
            for (c, (s, sc)) in path.out.iter().enumerate() {
                let (s, sc) = (*s as usize, *sc as usize);
                let col = &mut o.cols[c];
                match &w.srcs[s] {
                    Src::Tab(t, _) => {
                        let src = &t.cols[sc][..];
                        if s == 1 {
                            col.extend(std::iter::repeat_n(src[0], n));
                        } else {
                            col.extend(w.rows[s].iter().map(|r| src[*r as usize]));
                        }
                    }
                    Src::View(v, _) => view_cols::gather(f, run, v.base, v.map[sc], &w.rows[s], col),
                    _ => return Err("a sink reads no table or view".to_owned()),
                }
            }
            o.n += n;
            lap(self, 2, &mut at);
            if prof {
                self.fuse.c.by_root.entry(root).or_default()[3] += 1.0;
            }
            stop!(5);
            for s in w.srcs.drain(..) {
                if let Src::Tab(t, _) = s {
                    drop(t);
                }
            }
        }
        w.srcs.clear();
        match out {
            Some(t) if t.n > 0 => Ok(recycle::rc(t)),
            _ => self.empty_rc(run, root),
        }
    }
}

/// The driver's rows joined through a build side (its positions in `rows`, by `ix`), the pairs the checks hold appended
/// to `next`.
#[expect(clippy::too_many_arguments, reason = "a throwaway")]
fn probe_build(run: &mut Run, f: &File<'_>, srcs: &mut [Src], cur_rows: &[Vec<u32>], next: &mut [Vec<u32>], key: &mut Vec<u64>, live: u64, n: usize, si: usize, nsrc: usize, drv: &[Slot], rows: &[u32], ix: &super::Index, checks: &[P]) {
    let mut cur = [0u32; MAX_SRC];
    for i in 0..n {
        load(cur_rows, live, i, &mut cur);
        let k = if drv.len() == 1 {
            super::get(run, f, srcs, &cur, drv[0].0 as usize, drv[0].1 as usize)
        } else {
            key.clear();
            for (s, c) in drv {
                key.push(super::get(run, f, srcs, &cur, *s as usize, *c as usize));
            }
            key_of(key.iter().copied())
        };
        let Some(hs) = ix.get(&k) else { continue };
        for h in hs {
            cur[si] = rows[*h as usize];
            if checks.iter().all(|c| holds(run, f, srcs, &cur[..nsrc], c)) {
                push(next, live, &cur, si);
            }
        }
    }
}

impl Exec<'_> {
    /// The size of a view's entry as the kernels' front measures it (None: no entry known without reading it all).
    fn fuse_entry_len(&mut self, f: &mut File<'_>, run: &mut Run, vn: usize, v: &Rc<super::ViewSpec>, b: &Build) -> Option<usize> {
        if let Some((file, rows, _)) = self.fuse.builds.get(&b.sel)
            && *file == f.file
        {
            return Some(rows.len());
        }
        use super::select_kernel::{LIT_CONJ, LIT_FILTER, lit_file};
        match &b.kinds {
            (Some(ks), _) | (None, Some(ks)) => Some(super::bucket_len(f, run, ks)),
            (None, None) => lit_file(f, LIT_FILTER | vn, &v.filter, 0, v.base, &[]).or_else(|| lit_file(f, LIT_CONJ | (b.sel << 1) | 1, &b.sp.conj, 1, v.base, &v.map)).map(|r| r.len()),
        }
    }

    /// A view's build side for a join (once a file a select): the kernels' entry rows, the select's own tests on it, and
    /// an index of their positions by the join's key cells.
    fn fuse_build(&mut self, f: &mut File<'_>, run: &mut Run, vn: usize, v: &Rc<super::ViewSpec>, b: &Build) -> (Rc<Vec<u32>>, Rc<super::Index>) {
        if let Some((file, rows, ix)) = self.fuse.builds.get(&b.sel)
            && *file == f.file
        {
            return (rows.clone(), ix.clone());
        }
        use super::select_kernel::{LIT_CONJ, LIT_FILTER, lit_file};
        let mut rows: Vec<u32> = match &b.kinds {
            (None, Some(ks)) => {
                let mut r = super::bucket(f, run, ks);
                view_cols::retain(f, run, v, &mut r);
                r
            }
            _ => match lit_file(f, LIT_CONJ | (b.sel << 1) | 1, &b.sp.conj, 1, v.base, &v.map) {
                Some(r) if lit_file(f, LIT_FILTER | vn, &v.filter, 0, v.base, &[]).is_none() => {
                    let mut kept = r.to_vec();
                    view_cols::retain(f, run, v, &mut kept);
                    kept
                }
                _ => view_rows(f, run, v, vn).to_vec(),
            },
        };
        if !b.own.is_empty() {
            let local = [Src::Pending(usize::MAX), Src::View(v.clone(), vn)];
            for pr in &b.own {
                filter(run, f, &local, 1, pr, &mut rows);
            }
        }
        let keys: Vec<u64> = rows.iter().map(|r| key_of(b.cols.iter().map(|c| view_cols::cell(f, run, v.base, *r, v.map[*c])))).collect();
        let ix = Rc::new(super::Index::build(keys.len(), |i| keys[i]));
        let rows = Rc::new(rows);
        self.fuse.builds.insert(b.sel, (f.file, rows.clone(), ix.clone()));
        (rows, ix)
    }
}

/// Whether consumer `c` reads its input `i` as a stream (rows one at a time, in any batch), by plan structure alone.
fn streams(plan: &super::Plan, c: usize, i: usize) -> bool {
    let pn = &plan.nodes[c];
    let role_is = |name: &str| pn.j["args"][name]["rel"].as_u64() == Some(i as u64);
    match pn.op.as_str() {
        "select" => pn.j["src"][0][0].as_u64() == Some(i as u64),
        "antijoin" => role_is("left"),
        "axis" => role_is("from"),
        "group-count" | "group-min" | "group-max" | "group-ordinal" | "ordered-match" | "ordered-dfs" | "as-of" | "closure" | "follow" | "subtree-class" => false,
        _ => true,
    }
}

/// An expression of a select's own (source, column) as one over the path's cells.
fn remap_e(e: &E, to: &dyn Fn(usize, usize) -> Slot) -> E {
    let b = |x: &E| Box::new(remap_e(x, to));
    match e {
        E::Col(s, c, t) => {
            let (s2, c2) = to(*s, *c);
            E::Col(s2 as usize, c2 as usize, *t)
        }
        E::List(v) => E::List(v.iter().map(|x| remap_e(x, to)).collect()),
        E::Call(n, v) => E::Call(n.clone(), v.iter().map(|x| remap_e(x, to)).collect()),
        E::Cmp(o, x, y) => E::Cmp(*o, b(x), b(y)),
        E::In(x, y, n) => E::In(b(x), b(y), *n),
        E::And(x, y) => E::And(b(x), b(y)),
        E::Or(x, y) => E::Or(b(x), b(y)),
        E::Not(x) => E::Not(b(x)),
        E::Null(x, n) => E::Null(b(x), *n),
        E::Re(x, r) => E::Re(b(x), r.clone()),
        E::Seed(x, k) => E::Seed(b(x), *k),
        other => other.clone(),
    }
}

#[inline]
fn load(rows: &[Vec<u32>], live: u64, i: usize, cur: &mut [u32; MAX_SRC]) {
    let mut m = live;
    while m != 0 {
        let s = m.trailing_zeros() as usize;
        cur[s] = rows[s][i];
        m &= m - 1;
    }
}

#[inline]
fn push(next: &mut [Vec<u32>], live: u64, cur: &[u32; MAX_SRC], si: usize) {
    let mut m = live | (1 << si);
    while m != 0 {
        let s = m.trailing_zeros() as usize;
        next[s].push(cur[s]);
        m &= m - 1;
    }
}

fn compact(rows: &mut [Vec<u32>], live: u64, keep: &[bool]) {
    let mut m = live;
    while m != 0 {
        let s = m.trailing_zeros() as usize;
        let v = &mut rows[s];
        let mut k = 0;
        for i in 0..v.len() {
            if keep[i] {
                v[k] = v[i];
                k += 1;
            }
        }
        v.truncate(k);
        m &= m - 1;
    }
}

#[allow(dead_code, reason = "a throwaway: the driver kinds named for the notes")]
fn _names(_: Native) {}
