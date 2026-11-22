//! THROWAWAY (ryo-fmx3.1, step 6b): the finish stage. Each rule's WHOLE query, lowered by `scripts6b/lower.py` to one shared
//! plan of generic steps, run per file over the node table's columns and the provider's fills. No rule id is read here: the
//! plan names the emits, and the steps are the language's operators.
//!
//! Relations are column tables (one u64 a cell, typed per column). Base relations of the tree (`syntax-tree`,
//! `semantic-syntax`, `semantic-child`, `syntax-child`, `source-location`) are not tables: they are read in place from the
//! node table, by position (a join on their key is a position read; a scan with a kind test reads the kind buckets).
//! `token-link` comes from the token order. Every other relation is the provider's fill, read once a file when a step asks.

use ryo::languages::{self, CellKind, KindBits, NodeTable, Position, node_table};
use ryo_relations::domain::labels::types::NameRef;
use ryo_relations::domain::relations::service as catalog;
use rustc_hash::FxHashMap;
use serde_json::Value as J;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Ty {
    Node,
    Sym,
    File,
    Span,
    Num,
    Flag,
}

fn ty_of(k: CellKind) -> Ty {
    match k {
        CellKind::Node => Ty::Node,
        CellKind::Symbol => Ty::Sym,
        CellKind::File => Ty::File,
        CellKind::Span => Ty::Span,
        CellKind::Number => Ty::Num,
        CellKind::Flag => Ty::Flag,
    }
}

/// A value: a typed cell.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub(super) struct V(Ty2, u64);
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
enum Ty2 {
    T(u8),
}
impl V {
    fn new(t: Ty, v: u64) -> Self {
        V(Ty2::T(t as u8), v)
    }
    fn ty(self) -> Ty {
        let Ty2::T(t) = self.0;
        [Ty::Node, Ty::Sym, Ty::File, Ty::Span, Ty::Num, Ty::Flag][t as usize]
    }
}

pub(super) const NULL: u64 = u64::MAX;

/// A column table.
#[derive(Default, Clone, Debug)]
pub(super) struct Table {
    pub(super) names: Vec<String>,
    pub(super) tys: Vec<Ty>,
    pub(super) cols: Vec<Vec<u64>>,
    pub(super) n: usize,
}

impl Table {
    fn with(names: Vec<String>, tys: Vec<Ty>) -> Self {
        let k = names.len();
        Table { names, tys, cols: vec![Vec::new(); k], n: 0 }
    }
    pub(super) fn col(&self, name: &str) -> Result<usize, String> {
        self.names.iter().position(|c| c == name).ok_or_else(|| format!("no column {name} in {:?}", self.names))
    }
    fn push_row(&mut self, row: &[u64]) {
        if self.cols.len() < self.names.len() {
            self.cols.resize(self.names.len(), Vec::new());
        }
        for (c, v) in row.iter().enumerate() {
            self.cols[c].push(*v);
        }
        self.n += 1;
    }
    fn row(&self, r: usize, out: &mut Vec<u64>) {
        out.clear();
        out.extend(self.cols.iter().map(|c| c[r]));
    }
}

/// The run: texts, regexes, kind and field names, file paths.
#[derive(Default)]
pub(super) struct Run {
    pub(super) texts: Vec<Box<[u8]>>,
    pub(super) ids: FxHashMap<Box<[u8]>, u32>,
    pub(super) regexes: FxHashMap<String, Rc<regex::bytes::Regex>>,
    pub(super) kind_sym: Vec<u32>,
    pub(super) field_sym: Vec<u32>,
    pub(super) paths: Vec<String>,
    pub(super) path_sym: Vec<u32>,
    pub(super) empty: u32,
    pub(super) token_first: u16,
    pub(super) token_last: u16,
    pub(super) token_error: u16,
    pub(super) pairs: [(u16, u16); 3],
    pub(super) rels: FxHashMap<String, (languages::Relation, Vec<String>, Vec<Ty>)>,
}

impl Run {
    pub(super) fn sym(&mut self, t: &[u8]) -> u32 {
        if let Some(i) = self.ids.get(t) {
            return *i;
        }
        let i = self.texts.len() as u32;
        self.texts.push(t.into());
        self.ids.insert(t.into(), i);
        i
    }
    fn text(&self, s: u32) -> &[u8] {
        &self.texts[s as usize]
    }
    fn re(&mut self, p: &str) -> Result<Rc<regex::bytes::Regex>, String> {
        if let Some(r) = self.regexes.get(p) {
            return Ok(r.clone());
        }
        let r = Rc::new(regex::bytes::Regex::new(p).map_err(|e| format!("regex {p}: {e}"))?);
        self.regexes.insert(p.to_owned(), r.clone());
        Ok(r)
    }
    pub(super) fn catalog(&mut self, name: &str) -> Option<(languages::Relation, Vec<String>, Vec<Ty>)> {
        if let Some(r) = self.rels.get(name) {
            return Some(r.clone());
        }
        let relation = catalog::relation_named(NameRef(name))?;
        let names: Vec<String> = catalog::columns(relation).into_iter().map(|c| catalog::column_name(c).0.to_owned()).collect();
        let tys: Vec<Ty> = catalog::kinds(relation).iter().map(|k| ty_of(*k)).collect();
        self.rels.insert(name.to_owned(), (relation, names.clone(), tys.clone()));
        Some((relation, names, tys))
    }
}

/// A plan node, as lowered.
pub(super) struct PNode {
    pub(super) op: String,
    pub(super) j: J,
    pub(super) inputs: Vec<usize>,
}

pub(super) struct Plan {
    pub(super) nodes: Vec<PNode>,
    pub(super) emits: Vec<(String, usize, String, Vec<String>)>,
}

fn inputs_of(j: &J, out: &mut Vec<usize>) {
    match j {
        J::Object(m) => {
            if let Some(J::Number(n)) = m.get("rel")
                && m.len() == 1
            {
                out.push(n.as_u64().expect("rel") as usize);
                return;
            }
            for v in m.values() {
                inputs_of(v, out);
            }
        }
        J::Array(a) => a.iter().for_each(|v| inputs_of(v, out)),
        _ => {}
    }
}

pub(super) fn load(path: &str) -> Plan {
    let j: J = serde_json::from_str(&std::fs::read_to_string(path).expect("plan")).expect("plan json");
    let nodes = j["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .map(|n| {
            let op = n["op"].as_str().expect("op").to_owned();
            let mut inputs = Vec::new();
            if op == "select" {
                for s in n["src"].as_array().expect("src") {
                    inputs.push(s[0].as_u64().expect("src node") as usize);
                }
            } else if op != "base" {
                inputs_of(&n["args"], &mut inputs);
            }
            PNode { op, j: n.clone(), inputs }
        })
        .collect();
    let mut emits = Vec::new();
    for (rule, e) in j["emits"].as_object().expect("emits") {
        emits.push((
            rule.clone(),
            e["node"].as_u64().expect("emit node") as usize,
            e["subject"].as_str().unwrap_or("file").to_owned(),
            e["key"].as_array().map(|a| a.iter().map(|x| x.as_str().unwrap_or("").to_owned()).collect()).unwrap_or_default(),
        ));
    }
    Plan { nodes, emits }
}

/// The relations of the tree read in place.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Native {
    /// `syntax-tree` / `semantic-syntax`: a row an element.
    Tree,
    /// `semantic-child` / `syntax-child`: a row an element with a parent.
    Child,
    /// `source-location`: a row an element.
    Location,
}

const TREE_COLS: [(&str, Ty); 5] = [("id", Ty::Node), ("kind", Ty::Sym), ("text", Ty::Sym), ("file", Ty::File), ("span", Ty::Span)];
const CHILD_COLS: [(&str, Ty); 5] = [("parent", Ty::Node), ("child", Ty::Node), ("ordinal", Ty::Num), ("field", Ty::Sym), ("file", Ty::File)];
const LOC_COLS: [(&str, Ty); 9] = [
    ("id", Ty::Node),
    ("file", Ty::File),
    ("span", Ty::Span),
    ("start-line", Ty::Num),
    ("end-line", Ty::Num),
    ("start-byte", Ty::Num),
    ("end-byte", Ty::Num),
    ("start-column", Ty::Num),
    ("end-column", Ty::Num),
];

fn native_of(rel: &str) -> Option<Native> {
    match rel {
        "syntax-tree" | "semantic-syntax" => Some(Native::Tree),
        "semantic-child" | "syntax-child" => Some(Native::Child),
        "source-location" => Some(Native::Location),
        _ => None,
    }
}

fn native_cols(n: Native) -> &'static [(&'static str, Ty)] {
    match n {
        Native::Tree => &TREE_COLS,
        Native::Child => &CHILD_COLS,
        Native::Location => &LOC_COLS,
    }
}

/// One file: the node table's columns, the source, the fills and the made tables.
pub(super) struct File<'a> {
    pub(super) table: &'a NodeTable,
    pub(super) src: &'a [u8],
    pub(super) file: u32,
    pub(super) n: usize,
    pub(super) kind: Vec<u16>,
    pub(super) start: Vec<u32>,
    pub(super) end: Vec<u32>,
    pub(super) parent: Vec<u32>,
    pub(super) line_starts: Vec<u32>,
    pub(super) text_memo: RefCell<Vec<u32>>,
    pub(super) by_start: RefCell<Option<FxHashMap<u32, Vec<u32>>>>,
    pub(super) by_end: RefCell<Option<FxHashMap<u32, Vec<u32>>>>,
    pub(super) memo: Vec<Option<Rc<Table>>>,
    pub(super) index: FxHashMap<(usize, Vec<usize>), Rc<FxHashMap<Vec<u64>, Vec<u32>>>>,
    pub(super) fills: FxHashMap<String, Rc<Table>>,
}

/// How a step asks the provider for a relation's rows: the visit's facts.
pub(super) trait Provider {
    fn fill(&mut self, run: &mut Run, rel: &str) -> Result<Table, String>;
}

impl File<'_> {
    pub(super) fn new<'a>(table: &'a NodeTable, src: &'a [u8], file: u32, nodes: usize) -> File<'a> {
        let n = node_table::element_count(table).0;
        let mut f = File {
            table,
            src,
            file,
            n,
            kind: Vec::with_capacity(n),
            start: Vec::with_capacity(n),
            end: Vec::with_capacity(n),
            parent: Vec::with_capacity(n),
            line_starts: vec![0],
            text_memo: RefCell::new(vec![u32::MAX; n]),
            by_start: RefCell::new(None),
            by_end: RefCell::new(None),
            memo: vec![None; nodes],
            index: FxHashMap::default(),
            fills: FxHashMap::default(),
        };
        for i in 0..n as u32 {
            let p = Position(i);
            f.kind.push(node_table::kind_of(table, p).0);
            let s = node_table::span_of(table, p);
            f.start.push(s.start.0);
            f.end.push(s.end.0);
            f.parent.push(node_table::parent_of(table, p).0);
        }
        let mut i = 0;
        while i < src.len() {
            match src[i] {
                b'\n' => f.line_starts.push(i as u32 + 1),
                b'\r' => {
                    if i + 1 < src.len() && src[i + 1] == b'\n' {
                        i += 1;
                    }
                    f.line_starts.push(i as u32 + 1);
                }
                _ => {}
            }
            i += 1;
        }
        f
    }
    fn line(&self, off: u32) -> i64 {
        self.line_starts.partition_point(|s| *s <= off) as i64
    }
    fn column(&self, off: u32) -> i64 {
        let l = self.line_starts.partition_point(|s| *s <= off) - 1;
        i64::from(off - self.line_starts[l])
    }
    fn is_token(&self, run: &Run, k: u16) -> bool {
        k >= run.token_first && k <= run.token_last
    }
    fn elem_text(&self, run: &mut Run, p: u32) -> u32 {
        let m = self.text_memo.borrow()[p as usize];
        if m != u32::MAX {
            return m;
        }
        let k = self.kind[p as usize];
        let s = if self.is_token(run, k) || k == run.token_error {
            match node_table::decoded_of(self.table, Position(p)) {
                Some(t) => run.sym(t.0.as_bytes()),
                None => run.sym(&self.src[self.start[p as usize] as usize..self.end[p as usize] as usize]),
            }
        } else {
            run.empty
        };
        self.text_memo.borrow_mut()[p as usize] = s;
        s
    }
    fn has_parent(&self, p: u32) -> bool {
        p != 0 && self.parent[p as usize] != p
    }
    /// A native relation's cell.
    fn native_get(&self, run: &mut Run, nat: Native, row: u32, col: usize) -> u64 {
        let r = row as usize;
        match (nat, col) {
            (Native::Tree, 0) | (Native::Location, 0) => u64::from(row),
            (Native::Tree, 1) => u64::from(run.kind_sym[usize::from(self.kind[r])]),
            (Native::Tree, 2) => u64::from(self.elem_text(run, row)),
            (Native::Tree, 3) | (Native::Location, 1) | (Native::Child, 4) => u64::from(self.file),
            (Native::Tree, 4) | (Native::Location, 2) => (u64::from(self.start[r]) << 32) | u64::from(self.end[r]),
            (Native::Child, 0) => u64::from(self.parent[r]),
            (Native::Child, 1) => u64::from(row),
            (Native::Child, 2) => u64::from(node_table::ordinal_of(self.table, Position(row)).0),
            (Native::Child, 3) => {
                let f = usize::from(node_table::field_of(self.table, Position(row)).0);
                u64::from(run.field_sym.get(f).copied().unwrap_or(run.empty))
            }
            (Native::Location, 3) => self.line(self.start[r]) as u64,
            (Native::Location, 4) => self.line(self.end[r]) as u64,
            (Native::Location, 5) => u64::from(self.start[r]),
            (Native::Location, 6) => u64::from(self.end[r]),
            (Native::Location, 7) => self.column(self.start[r]) as u64,
            (Native::Location, 8) => self.column(self.end[r]) as u64,
            _ => NULL,
        }
    }
    fn native_rows(&self, nat: Native) -> Vec<u32> {
        match nat {
            Native::Tree | Native::Location => (0..self.n as u32).collect(),
            Native::Child => (0..self.n as u32).filter(|p| self.has_parent(*p)).collect(),
        }
    }
    /// Rows of a native relation whose column holds `v`.
    fn native_probe(&self, nat: Native, col: usize, v: u64, out: &mut Vec<u32>) -> bool {
        let ok = |p: u64| p < self.n as u64;
        match (nat, col) {
            (Native::Tree, 0) | (Native::Location, 0) => {
                if ok(v) {
                    out.push(v as u32);
                }
            }
            (Native::Child, 1) => {
                if ok(v) && self.has_parent(v as u32) {
                    out.push(v as u32);
                }
            }
            (Native::Child, 0) => {
                if ok(v) {
                    for c in node_table::children_of(self.table, Position(v as u32)) {
                        if self.parent[c.0 as usize] == v as u32 && c.0 != 0 {
                            out.push(c.0);
                        }
                    }
                }
            }
            (Native::Location, 5) | (Native::Location, 6) => {
                let cell = if col == 5 { &self.by_start } else { &self.by_end };
                let mut b = cell.borrow_mut();
                let idx = b.get_or_insert_with(|| {
                    let mut m: FxHashMap<u32, Vec<u32>> = FxHashMap::default();
                    let src = if col == 5 { &self.start } else { &self.end };
                    for (p, s) in src.iter().enumerate() {
                        m.entry(*s).or_default().push(p as u32);
                    }
                    m
                });
                if let Some(rows) = idx.get(&(v as u32)) {
                    out.extend_from_slice(rows);
                }
            }
            _ => return false,
        }
        true
    }
}

/// Counters a run keeps: per node, time and rows; per op kind.
#[derive(Default)]
pub(super) struct Stats {
    pub(super) time: Vec<f64>,
    pub(super) rows: Vec<u64>,
    pub(super) calls: Vec<u64>,
    pub(super) touched: Vec<u64>,
}

// ------------------------------------------------------------------------------------------------ expressions

#[derive(Clone, Debug)]
#[expect(dead_code, reason = "a throwaway")]
enum E {
    Col(usize, usize, Ty),
    Lit(V),
    Text(u32),
    Num(i64),
    List(Vec<E>),
    Call(String, Vec<E>),
    Cmp(String, Box<E>, Box<E>),
    In(Box<E>, Box<E>, bool),
    And(Box<E>, Box<E>),
    Or(Box<E>, Box<E>),
    Not(Box<E>),
    True,
    /// A text tested against a constant regex (from `matches` or `glob_any`), compiled once.
    Re(Box<E>, Rc<regex::bytes::Regex>),
}

/// One source of a select: a table or a native relation of the tree, or a lazy closure.
enum Src {
    Tab(Rc<Table>, usize),
    Nat(Native),
    Lazy(Rc<Lazy>),
}

/// A closure that is never made whole: it answers its sources' reach when probed by source.
pub(super) struct Lazy {
    edges: Rc<Table>,
    s: usize,
    d: usize,
    keys: Vec<usize>,
    adj: FxHashMap<Vec<u64>, Vec<u64>>,
    radj: FxHashMap<Vec<u64>, Vec<u64>>,
    node: usize,
    names: Vec<String>,
    tys: Vec<Ty>,
    cache: RefCell<FxHashMap<Vec<u64>, Rc<Vec<u64>>>>,
    rcache: RefCell<FxHashMap<Vec<u64>, Rc<Vec<u64>>>>,
}

impl Src {
    fn names(&self) -> Vec<String> {
        match self {
            Src::Tab(t, _) => t.names.clone(),
            Src::Nat(n) => native_cols(*n).iter().map(|c| c.0.to_owned()).collect(),
            Src::Lazy(l) => l.names.clone(),
        }
    }
    fn tys(&self) -> Vec<Ty> {
        match self {
            Src::Tab(t, _) => t.tys.clone(),
            Src::Nat(n) => native_cols(*n).iter().map(|c| c.1).collect(),
            Src::Lazy(l) => l.tys.clone(),
        }
    }
}

fn truthy(v: V) -> bool {
    v.ty() == Ty::Flag && v.1 == 1
}

// ------------------------------------------------------------------------------------------------ the executor

pub(super) struct Exec<'p> {
    pub(super) plan: &'p Plan,
    pub(super) stats: Stats,
    pub(super) lazies: FxHashMap<usize, Rc<Lazy>>,
}

type R<T> = Result<T, String>;

fn s(j: &J) -> R<&str> {
    j.as_str().ok_or_else(|| format!("not a text: {j}"))
}
fn strs(j: &J) -> R<Vec<String>> {
    match j {
        J::Array(a) => a.iter().map(|x| s(x).map(str::to_owned)).collect(),
        J::String(x) => Ok(vec![x.clone()]),
        J::Null => Ok(Vec::new()),
        _ => Err(format!("not a list: {j}")),
    }
}

impl<'p> Exec<'p> {
    pub(super) fn new(plan: &'p Plan) -> Self {
        let k = plan.nodes.len();
        Exec { plan, stats: Stats { time: vec![0.0; k], rows: vec![0; k], calls: vec![0; k], touched: vec![0; k] }, lazies: FxHashMap::default() }
    }

    fn arg_rel(&self, node: usize, name: &str) -> R<usize> {
        self.plan.nodes[node].j["args"][name]["rel"].as_u64().map(|x| x as usize).ok_or_else(|| format!("no relation argument {name}"))
    }

    /// A node's rows, made once a file.
    pub(super) fn eval(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize) -> R<Rc<Table>> {
        if let Some(t) = &f.memo[node] {
            return Ok(t.clone());
        }
        let pn = &self.plan.nodes[node];
        // inputs first (a lazy closure's input too), so the time below is this step's own
        for i in pn.inputs.clone() {
            if self.plan.nodes[i].op == "base" && native_of(self.plan.nodes[i].j["rel"].as_str().unwrap_or("")).is_some() {
                continue;
            }
            if self.plan.nodes[i].op == "closure" && pn.op == "select" {
                self.lazy(f, run, p, i)?;
                continue;
            }
            self.eval(f, run, p, i)?;
        }
        let t0 = super::thread_cpu();
        let out = self.body(f, run, p, node)?;
        self.stats.time[node] += super::thread_cpu() - t0;
        self.stats.calls[node] += 1;
        self.stats.rows[node] += out.n as u64;
        let rc = Rc::new(out);
        f.memo[node] = Some(rc.clone());
        Ok(rc)
    }

    fn lazy(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize) -> R<Rc<Lazy>> {
        if let Some(l) = self.lazies.get(&node) {
            return Ok(l.clone());
        }
        let edges_node = self.arg_rel(node, "edges")?;
        let edges = self.eval(f, run, p, edges_node)?;
        let t0 = super::thread_cpu();
        let a = &self.plan.nodes[node].j["args"];
        let (sn, dn) = (s(&a["source"])?.to_owned(), s(&a["destination"])?.to_owned());
        let keys: Vec<usize> = strs(&a["keys"])?.iter().map(|k| edges.col(k)).collect::<R<_>>()?;
        let (si, di) = (edges.col(&sn)?, edges.col(&dn)?);
        let mut adj: FxHashMap<Vec<u64>, Vec<u64>> = FxHashMap::default();
        let mut radj: FxHashMap<Vec<u64>, Vec<u64>> = FxHashMap::default();
        for r in 0..edges.n {
            let mut k: Vec<u64> = keys.iter().map(|c| edges.cols[*c][r]).collect();
            let mut k2 = k.clone();
            k.push(edges.cols[si][r]);
            adj.entry(k).or_default().push(edges.cols[di][r]);
            k2.push(edges.cols[di][r]);
            radj.entry(k2).or_default().push(edges.cols[si][r]);
        }
        let mut names: Vec<String> = keys.iter().map(|c| edges.names[*c].clone()).collect();
        let mut tys: Vec<Ty> = keys.iter().map(|c| edges.tys[*c]).collect();
        names.push(sn);
        names.push(dn);
        tys.push(edges.tys[si]);
        tys.push(edges.tys[di]);
        let l = Rc::new(Lazy { edges: edges.clone(), s: si, d: di, keys, adj, radj, node, names, tys, cache: RefCell::new(FxHashMap::default()), rcache: RefCell::new(FxHashMap::default()) });
        self.stats.time[node] += super::thread_cpu() - t0;
        self.stats.calls[node] += 1;
        self.lazies.insert(node, l.clone());
        Ok(l)
    }

    pub(super) fn reset_file(&mut self) {
        self.lazies.clear();
    }

    fn src_of(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize) -> R<Src> {
        let pn = &self.plan.nodes[node];
        if pn.op == "base"
            && let Some(nat) = native_of(pn.j["rel"].as_str().unwrap_or(""))
        {
            return Ok(Src::Nat(nat));
        }
        if pn.op == "closure" {
            return Ok(Src::Lazy(self.lazy(f, run, p, node)?));
        }
        Ok(Src::Tab(self.eval(f, run, p, node)?, node))
    }

    fn body(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize) -> R<Table> {
        let pn = &self.plan.nodes[node];
        let op = pn.op.clone();
        let j = pn.j.clone();
        let a = &j["args"];
        match op.as_str() {
            "base" => self.base(f, run, p, s(&j["rel"])?),
            "select" => self.select(f, run, p, node, &j),
            "union" => {
                let l = self.eval(f, run, p, self.arg_rel(node, "left")?)?;
                let r = self.eval(f, run, p, self.arg_rel(node, "right")?)?;
                let mut out = (*l).clone();
                let map: Vec<usize> = l.names.iter().map(|c| r.col(c)).collect::<R<_>>()?;
                for (c, m) in map.iter().enumerate() {
                    out.cols[c].extend_from_slice(&r.cols[*m]);
                }
                out.n += r.n;
                Ok(out)
            }
            "antijoin" => {
                let l = self.eval(f, run, p, self.arg_rel(node, "left")?)?;
                let r = self.eval(f, run, p, self.arg_rel(node, "right")?)?;
                let lk: Vec<usize> = strs(&a["left-on"])?.iter().map(|c| l.col(c)).collect::<R<_>>()?;
                let rk: Vec<usize> = strs(&a["right-on"])?.iter().map(|c| r.col(c)).collect::<R<_>>()?;
                let set: rustc_hash::FxHashSet<Vec<u64>> = (0..r.n).map(|i| rk.iter().map(|c| r.cols[*c][i]).collect()).collect();
                let mut out = Table::with(l.names.clone(), l.tys.clone());
                let mut row = Vec::new();
                let mut key = Vec::new();
                for i in 0..l.n {
                    key.clear();
                    key.extend(lk.iter().map(|c| l.cols[*c][i]));
                    if !set.contains(&key) {
                        l.row(i, &mut row);
                        out.push_row(&row);
                    }
                }
                Ok(out)
            }
            "extend" => {
                let from = self.eval(f, run, p, self.arg_rel(node, "from")?)?;
                let mut out = (*from).clone();
                for (k, v) in a["values"].as_object().ok_or("extend values")? {
                    let (t, x) = match v {
                        J::Number(n) => (Ty::Num, n.as_i64().ok_or("extend number")? as u64),
                        J::String(t) if t == "true" || t == "false" => (Ty::Flag, u64::from(t == "true")),
                        J::String(t) => (Ty::Sym, u64::from(run.sym(t.as_bytes()))),
                        _ => return Err(format!("extend value {v}")),
                    };
                    out.names.push(k.clone());
                    out.tys.push(t);
                    out.cols.push(vec![x; from.n]);
                }
                Ok(out)
            }
            "project" => {
                let from = self.eval(f, run, p, self.arg_rel(node, "from")?)?;
                let cols: Vec<usize> = strs(&a["columns"])?.iter().map(|c| from.col(c)).collect::<R<_>>()?;
                let mut out = Table::with(cols.iter().map(|c| from.names[*c].clone()).collect(), cols.iter().map(|c| from.tys[*c]).collect());
                let mut seen = rustc_hash::FxHashSet::default();
                for i in 0..from.n {
                    let row: Vec<u64> = cols.iter().map(|c| from.cols[*c][i]).collect();
                    if seen.insert(row.clone()) {
                        out.push_row(&row);
                    }
                }
                Ok(out)
            }
            "axis" => self.axis(f, run, p, node, a),
            "group-count" => {
                let from = self.eval(f, run, p, self.arg_rel(node, "from")?)?;
                let keys: Vec<usize> = strs(&a["keys"])?.iter().map(|c| from.col(c)).collect::<R<_>>()?;
                let distinct = match &a["distinct"] {
                    J::String(d) => Some(from.col(d)?),
                    _ => None,
                };
                let mut groups: FxHashMap<Vec<u64>, (u64, rustc_hash::FxHashSet<u64>)> = FxHashMap::default();
                let mut order: Vec<Vec<u64>> = Vec::new();
                for i in 0..from.n {
                    let k: Vec<u64> = keys.iter().map(|c| from.cols[*c][i]).collect();
                    let e = groups.entry(k.clone()).or_insert_with(|| {
                        order.push(k);
                        (0, rustc_hash::FxHashSet::default())
                    });
                    match distinct {
                        Some(d) => {
                            e.1.insert(from.cols[d][i]);
                        }
                        None => e.0 += 1,
                    }
                }
                let mut names: Vec<String> = keys.iter().map(|c| from.names[*c].clone()).collect();
                let mut tys: Vec<Ty> = keys.iter().map(|c| from.tys[*c]).collect();
                names.push(s(&a["count"])?.to_owned());
                tys.push(Ty::Num);
                let mut out = Table::with(names, tys);
                for k in order {
                    let g = &groups[&k];
                    let c = if distinct.is_some() { g.1.len() as u64 } else { g.0 };
                    let mut row = k.clone();
                    row.push(c);
                    out.push_row(&row);
                }
                Ok(out)
            }
            "group-min" | "group-max" => {
                let from = self.eval(f, run, p, self.arg_rel(node, "from")?)?;
                let keys: Vec<usize> = strs(&a["keys"])?.iter().map(|c| from.col(c)).collect::<R<_>>()?;
                let oc = from.col(s(&a["order"])?)?;
                let max = op == "group-max";
                let mut best: FxHashMap<Vec<u64>, i64> = FxHashMap::default();
                for i in 0..from.n {
                    let k: Vec<u64> = keys.iter().map(|c| from.cols[*c][i]).collect();
                    let v = from.cols[oc][i] as i64;
                    let e = best.entry(k).or_insert(v);
                    if (max && v > *e) || (!max && v < *e) {
                        *e = v;
                    }
                }
                let mut out = Table::with(from.names.clone(), from.tys.clone());
                let mut row = Vec::new();
                for i in 0..from.n {
                    let k: Vec<u64> = keys.iter().map(|c| from.cols[*c][i]).collect();
                    if best[&k] == from.cols[oc][i] as i64 {
                        from.row(i, &mut row);
                        out.push_row(&row);
                    }
                }
                Ok(out)
            }
            "closure" => {
                let l = self.lazy(f, run, p, node)?;
                let mut out = Table::with(l.names.clone(), l.tys.clone());
                let starts: Vec<Vec<u64>> = l.adj.keys().cloned().collect();
                for k in starts {
                    let reach = reach_of(&l, &k);
                    for d in reach.iter() {
                        let mut row = k.clone();
                        row.push(*d);
                        out.push_row(&row);
                    }
                }
                Ok(out)
            }
            "follow" => {
                let edges = self.eval(f, run, p, self.arg_rel(node, "edges")?)?;
                let (sn, dn) = (s(&a["source"])?, s(&a["destination"])?);
                let keys: Vec<usize> = strs(&a["keys"])?.iter().map(|k| edges.col(k)).collect::<R<_>>()?;
                let (si, di) = (edges.col(sn)?, edges.col(dn)?);
                let mut adj: FxHashMap<Vec<u64>, Vec<u64>> = FxHashMap::default();
                let mut fixed: rustc_hash::FxHashSet<Vec<u64>> = rustc_hash::FxHashSet::default();
                for r in 0..edges.n {
                    let mut k: Vec<u64> = keys.iter().map(|c| edges.cols[*c][r]).collect();
                    k.push(edges.cols[si][r]);
                    if edges.cols[si][r] == edges.cols[di][r] {
                        fixed.insert(k.clone());
                    }
                    adj.entry(k).or_default().push(edges.cols[di][r]);
                }
                let mut names: Vec<String> = keys.iter().map(|c| edges.names[*c].clone()).collect();
                let mut tys: Vec<Ty> = keys.iter().map(|c| edges.tys[*c]).collect();
                names.extend([sn.to_owned(), dn.to_owned()]);
                tys.extend([edges.tys[si], edges.tys[di]]);
                let mut out = Table::with(names, tys);
                let starts: Vec<Vec<u64>> = adj.keys().cloned().collect();
                for k in starts {
                    let kp = &k[..k.len() - 1];
                    let mut seen = rustc_hash::FxHashSet::default();
                    let mut stack = vec![k[k.len() - 1]];
                    let mut found = std::collections::BTreeSet::new();
                    seen.insert(k[k.len() - 1]);
                    while let Some(x) = stack.pop() {
                        let mut kx = kp.to_vec();
                        kx.push(x);
                        if fixed.contains(&kx) {
                            found.insert(x);
                        }
                        if let Some(ns) = adj.get(&kx) {
                            for y in ns {
                                if seen.insert(*y) {
                                    stack.push(*y);
                                }
                            }
                        }
                    }
                    for x in found {
                        let mut row = k.clone();
                        row.push(x);
                        out.push_row(&row);
                    }
                }
                Ok(out)
            }
            "ordered-match" | "as-of" => self.ordered(f, run, p, node, a, op == "as-of"),
            "span-between" => {
                let from = self.eval(f, run, p, self.arg_rel(node, "from")?)?;
                let (sc, ec) = (from.col(s(&a["start"])?)?, from.col(s(&a["end"])?)?);
                let mut out = (*from).clone();
                let col: Vec<u64> = (0..from.n)
                    .map(|i| {
                        let (x, y) = (from.cols[sc][i], from.cols[ec][i]);
                        if x == NULL || y == NULL { NULL } else { (x & 0xffff_ffff_0000_0000) | (y & 0xffff_ffff) }
                    })
                    .collect();
                out.names.push(s(&a["output"])?.to_owned());
                out.tys.push(Ty::Span);
                out.cols.push(col);
                Ok(out)
            }
            "span-bounds" => {
                let from = self.eval(f, run, p, self.arg_rel(node, "from")?)?;
                let sc = from.col(s(&a["span"])?)?;
                let mut out = (*from).clone();
                out.names.push(s(&a["start-output"])?.to_owned());
                out.names.push(s(&a["end-output"])?.to_owned());
                out.tys.extend([Ty::Num, Ty::Num]);
                out.cols.push(from.cols[sc].iter().map(|x| x >> 32).collect());
                out.cols.push(from.cols[sc].iter().map(|x| x & 0xffff_ffff).collect());
                Ok(out)
            }
            "source-matches" => self.source_matches(f, run, p, node, a),
            "replace" => {
                let from = self.eval(f, run, p, self.arg_rel(node, "from")?)?;
                let tc = from.col(s(&a["text"])?)?;
                let re = run.re(s(&a["pattern"])?)?;
                let rep = s(&a["replacement"])?.as_bytes().to_vec();
                let mut out = (*from).clone();
                let col: Vec<u64> = (0..from.n)
                    .map(|i| {
                        let t = run.text(from.cols[tc][i] as u32).to_vec();
                        let r = re.replace_all(&t, regex::bytes::NoExpand(&rep)).into_owned();
                        u64::from(run.sym(&r))
                    })
                    .collect();
                out.names.push(s(&a["output"])?.to_owned());
                out.tys.push(Ty::Sym);
                out.cols.push(col);
                Ok(out)
            }
            "capture" => {
                let from = self.eval(f, run, p, self.arg_rel(node, "from")?)?;
                let pc = from.col(s(&a["path"])?)?;
                let caps = strs(&a["captures"])?;
                let re = run.re(&path_pattern(s(&a["pattern"])?))?;
                let mut out = Table::with(from.names.clone(), from.tys.clone());
                for c in &caps {
                    out.names.push(c.clone());
                    out.tys.push(Ty::Sym);
                }
                let mut row = Vec::new();
                for i in 0..from.n {
                    let v = from.cols[pc][i];
                    let text = if from.tys[pc] == Ty::File { run.text(run.path_sym[v as usize]).to_vec() } else { run.text(v as u32).to_vec() };
                    if let Some(m) = re.captures(&text) {
                        from.row(i, &mut row);
                        for c in &caps {
                            let t = m.name(c).map_or(&b""[..], |x| x.as_bytes()).to_vec();
                            row.push(u64::from(run.sym(&t)));
                        }
                        out.push_row(&row);
                    }
                }
                Ok(out)
            }
            "rows" => {
                let names = strs(&a["columns"])?;
                let mut out = Table::with(names.clone(), vec![Ty::Sym; names.len()]);
                for r in a["from"].as_array().ok_or("rows from")? {
                    let row: Vec<u64> = r.as_array().ok_or("row")?.iter().map(|x| u64::from(run.sym(x.as_str().unwrap_or("").as_bytes()))).collect();
                    out.push_row(&row);
                }
                Ok(out)
            }
            other => Err(format!("operator {other} not in the step set")),
        }
    }

    fn base(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, rel: &str) -> R<Table> {
        if let Some(nat) = native_of(rel) {
            let cols = native_cols(nat);
            let mut out = Table::with(cols.iter().map(|c| c.0.to_owned()).collect(), cols.iter().map(|c| c.1).collect());
            for r in f.native_rows(nat) {
                for c in 0..cols.len() {
                    let v = f.native_get(run, nat, r, c);
                    out.cols[c].push(v);
                }
                out.n += 1;
            }
            return Ok(out);
        }
        if rel == "token-link" {
            return Ok(token_links(f, run));
        }
        if rel == "filesystem.file" {
            let mut out = Table::with(vec!["path".to_owned()], vec![Ty::File]);
            for i in 0..run.paths.len() {
                out.push_row(&[i as u64]);
            }
            return Ok(out);
        }
        if let Some(t) = f.fills.get(rel) {
            return Ok((**t).clone());
        }
        let t = p.fill(run, rel)?;
        f.fills.insert(rel.to_owned(), Rc::new(t.clone()));
        Ok(t)
    }

    fn expr(&self, run: &mut Run, j: &J, names: &[(Option<String>, Vec<String>, Vec<Ty>)]) -> R<E> {
        let a = j.as_array().ok_or("expr")?;
        let tag = s(&a[0])?;
        Ok(match tag {
            "col" => {
                let name = s(&a[1])?;
                let alias = a[2].as_str();
                let mut found = None;
                for (i, (al, ns, ts)) in names.iter().enumerate() {
                    if alias.is_some() && al.as_deref() != alias {
                        continue;
                    }
                    if let Some(c) = ns.iter().position(|x| x == name) {
                        found = Some(E::Col(i, c, ts[c]));
                        break;
                    }
                }
                match found {
                    Some(e) => e,
                    // a bare word naming no column: a word, as `true`, or an identifier argument
                    None if alias.is_none() => E::Text(run.sym(name.as_bytes())),
                    None => return Err(format!("no column {alias:?}.{name}")),
                }
            }
            "lit" => E::Text(run.sym(s(&a[1])?.as_bytes())),
            "num" => E::Num(a[1].as_i64().ok_or("num")?),
            "list" => E::List(a[1].as_array().ok_or("list")?.iter().map(|x| self.expr(run, x, names)).collect::<R<_>>()?),
            "call" => {
                let name = s(&a[1])?.to_owned();
                let args: Vec<E> = a[2].as_array().ok_or("args")?.iter().map(|x| self.expr(run, x, names)).collect::<R<_>>()?;
                match (name.as_str(), args.get(1)) {
                    ("matches", Some(E::Text(t))) if args.len() == 2 => {
                        let re = run.re(&String::from_utf8_lossy(run.text(*t)).into_owned())?;
                        E::Re(Box::new(args[0].clone()), re)
                    }
                    ("glob_any", Some(E::Text(t))) if args.len() == 2 => {
                        let re = run.re(&glob_pattern(&String::from_utf8_lossy(run.text(*t))))?;
                        E::Re(Box::new(args[0].clone()), re)
                    }
                    _ => E::Call(name, args),
                }
            }
            "cmp" => E::Cmp(s(&a[1])?.to_owned(), Box::new(self.expr(run, &a[2], names)?), Box::new(self.expr(run, &a[3], names)?)),
            "in" | "notin" => E::In(Box::new(self.expr(run, &a[1], names)?), Box::new(self.expr(run, &a[2], names)?), tag == "notin"),
            "and" => E::And(Box::new(self.expr(run, &a[1], names)?), Box::new(self.expr(run, &a[2], names)?)),
            "or" => E::Or(Box::new(self.expr(run, &a[1], names)?), Box::new(self.expr(run, &a[2], names)?)),
            "not" => E::Not(Box::new(self.expr(run, &a[1], names)?)),
            other => return Err(format!("expression {other}")),
        })
    }

    #[expect(clippy::needless_pass_by_ref_mut, reason = "a throwaway")]
    fn select(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize, j: &J) -> R<Table> {
        let src_j = j["src"].as_array().ok_or("src")?;
        let mut srcs = Vec::new();
        let mut names = Vec::new();
        for sj in src_j {
            let n = sj[0].as_u64().ok_or("src node")? as usize;
            let alias = sj[1].as_str().map(str::to_owned);
            let src = self.src_of(f, run, p, n)?;
            names.push((alias, src.names(), src.tys()));
            srcs.push(src);
        }
        let mut conj = Vec::new();
        if !j["cond"].is_null() {
            let e = self.expr(run, &j["cond"], &names)?;
            flatten(e, &mut conj);
        }
        // a test of two file columns always holds: every relation here is the file's
        conj.retain(|c| !matches!(c, E::Cmp(o, a, b) if o == "=" && matches!(**a, E::Col(_, _, Ty::File)) && matches!(**b, E::Col(_, _, Ty::File))));
        let k = srcs.len();
        let refs: Vec<u64> = conj.iter().map(refs_of).collect();
        // the driver: the smallest made table, else a native relation (by its kind test, when it has one)
        let mut driver = None;
        for (i, sr) in srcs.iter().enumerate() {
            if let Src::Tab(t, _) = sr
                && driver.is_none_or(|d: usize| matches!(&srcs[d], Src::Tab(u, _) if u.n > t.n))
            {
                driver = Some(i);
            }
        }
        let driver = driver.unwrap_or_else(|| srcs.iter().position(|s| matches!(s, Src::Nat(_))).unwrap_or(0));
        let mut used = vec![false; conj.len()];
        let mut bound: u64 = 1 << driver;
        let mut tuples: Vec<Vec<u32>> = vec![Vec::new(); k];
        // driver rows, with its own tests
        let own: Vec<usize> = (0..conj.len()).filter(|c| refs[*c] & !(1u64 << driver) == 0).collect();
        let first: Vec<u32> = match &srcs[driver] {
            Src::Tab(t, _) => (0..t.n as u32).collect(),
            Src::Nat(nat) => match kind_bucket(run, f, *nat, driver, &conj, &own) {
                Some(rows) => rows,
                None => f.native_rows(*nat),
            },
            Src::Lazy(_) => return Err("a closure as a select's first source".to_owned()),
        };
        self.stats.touched[node] += first.len() as u64;
        let mut cur = vec![0u32; k];
        for r in first {
            cur[driver] = r;
            if own.iter().all(|c| truthy(eval(run, f, &srcs, &cur, &conj[*c]))) {
                tuples[driver].push(r);
            }
        }
        for c in own {
            used[c] = true;
        }
        let mut count = tuples[driver].len();
        while bound.count_ones() as usize != k {
            // the next source: one an unused equality joins to the bound ones, a native probe first
            let mut pick: Option<(usize, usize, usize, bool)> = None;
            for (ci, c) in conj.iter().enumerate() {
                if used[ci] {
                    continue;
                }
                if let E::Cmp(o, x, y) = c
                    && o == "="
                {
                    for (a, b) in [(x, y), (y, x)] {
                        if let E::Col(si, col, _) = **a
                            && bound & (1 << si) == 0
                            && refs_of(b) & !bound == 0
                            && refs_of(b) != 0
                        {
                            let native = match &srcs[si] {
                                Src::Nat(n) => f.native_probe(*n, col, NULL, &mut Vec::new()),
                                Src::Lazy(l) => col + 2 >= l.names.len(),
                                Src::Tab(..) => false,
                            };
                            if pick.is_none_or(|q| native && !q.3) {
                                pick = Some((si, col, ci, native));
                            }
                        }
                    }
                }
            }
            let mut next: Vec<Vec<u32>> = vec![Vec::new(); k];
            let si;
            match pick {
                Some((s_i, col, ci, _)) => {
                    si = s_i;
                    used[ci] = true;
                    let other = match &conj[ci] {
                        E::Cmp(_, x, y) => {
                            if matches!(**x, E::Col(a, b, _) if a == s_i && b == col) { (**y).clone() } else { (**x).clone() }
                        }
                        _ => unreachable!(),
                    };
                    let nb = bound | (1 << si);
                    let checks: Vec<usize> = (0..conj.len()).filter(|c| !used[*c] && refs[*c] & !nb == 0).collect();
                    let mut hits = Vec::new();
                    // a closure is made for the sources this join asks, and only those (its rows are then a table)
                    if let Src::Lazy(l) = &srcs[si] {
                        // the key columns' values come from the other equalities on them
                        let nk = l.keys.len();
                        let mut key_exprs: Vec<E> = Vec::new();
                        for kc in 0..nk {
                            let found = conj.iter().enumerate().find_map(|(cj, c)| match c {
                                E::Cmp(o, x, y) if o == "=" && !used[cj] => {
                                    let hit = |a: &E, b: &E| matches!(a, E::Col(s3, c3, _) if *s3 == si && *c3 == kc) && refs_of(b) & !bound == 0;
                                    if hit(x, y) {
                                        Some((**y).clone())
                                    } else if hit(y, x) {
                                        Some((**x).clone())
                                    } else {
                                        None
                                    }
                                }
                                _ => None,
                            });
                            match found {
                                Some(e) => key_exprs.push(e),
                                None if l.tys[kc] == Ty::File => key_exprs.push(E::Lit(V::new(Ty::File, u64::from(f.file)))),
                                None => return Err("a keyed closure probed without its keys".to_owned()),
                            }
                        }
                        let mut t = Table::with(l.names.clone(), l.tys.clone());
                        let mut asked = rustc_hash::FxHashSet::default();
                        for ti in 0..count {
                            for (s2, cur_s) in cur.iter_mut().enumerate() {
                                if bound & (1 << s2) != 0 {
                                    *cur_s = tuples[s2][ti];
                                }
                            }
                            let mut key: Vec<u64> = key_exprs.iter().map(|e| eval(run, f, &srcs, &cur, e).1).collect();
                            let forward = col + 2 == l.names.len();
                            let at = eval(run, f, &srcs, &cur, &other).1;
                            key.push(at);
                            if asked.insert(key.clone()) {
                                for d in reach_dir(l, &key, forward).iter() {
                                    let mut row = key[..nk].to_vec();
                                    if forward {
                                        row.extend([at, *d]);
                                    } else {
                                        row.extend([*d, at]);
                                    }
                                    t.push_row(&row);
                                }
                            }
                        }
                        self.stats.touched[node] += t.n as u64;
                        srcs[si] = Src::Tab(Rc::new(t), usize::MAX - si);
                    }
                    // a made table's index on the probed column, made once a file
                    // a made table is probed on every column an equality binds, at once
                    let mut tab_cols = vec![col];
                    let mut tab_exprs = vec![other.clone()];
                    if let Src::Tab(..) = &srcs[si] {
                        for (cj, c) in conj.iter().enumerate() {
                            if used[cj] {
                                continue;
                            }
                            if let E::Cmp(o, x, y) = c
                                && o == "="
                            {
                                for (a2, b2) in [(x, y), (y, x)] {
                                    if let E::Col(s3, c3, _) = **a2
                                        && s3 == si
                                        && refs_of(b2) & !bound == 0
                                        && refs_of(b2) != 0
                                        && !tab_cols.contains(&c3)
                                    {
                                        tab_cols.push(c3);
                                        tab_exprs.push((**b2).clone());
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    let index = if let Src::Tab(t, tn) = &srcs[si] {
                        Some(if *tn > usize::MAX - 64 { Rc::new(index_of(t, &tab_cols)) } else { table_index(f, *tn, t, &tab_cols) })
                    } else {
                        None
                    };
                    let mut probe_key: Vec<u64> = Vec::new();
                    for ti in 0..count {
                        for (s2, cur_s) in cur.iter_mut().enumerate() {
                            if bound & (1 << s2) != 0 {
                                *cur_s = tuples[s2][ti];
                            }
                        }
                        let v = eval(run, f, &srcs, &cur, &other);
                        hits.clear();
                        match &srcs[si] {
                            Src::Nat(n) => {
                                let _ = f.native_probe(*n, col, v.1, &mut hits);
                            }
                            Src::Tab(..) => {
                                probe_key.clear();
                                probe_key.push(v.1);
                                for e in &tab_exprs[1..] {
                                    probe_key.push(eval(run, f, &srcs, &cur, e).1);
                                }
                                if let Some(rows) = index.as_ref().and_then(|ix| ix.get(&probe_key)) {
                                    hits.extend_from_slice(rows);
                                }
                            }
                            Src::Lazy(_) => unreachable!(),
                        }
                        self.stats.touched[node] += hits.len() as u64;
                        for h in &hits {
                            cur[si] = *h;
                            if checks.iter().all(|c| truthy(eval(run, f, &srcs, &cur, &conj[*c]))) {
                                for s2 in 0..k {
                                    if nb & (1 << s2) != 0 {
                                        next[s2].push(cur[s2]);
                                    }
                                }
                            }
                        }
                    }
                    for c in checks {
                        used[c] = true;
                    }
                    bound = nb;
                }
                None => {
                    // no key joins the next source: every pair
                    si = (0..k).find(|s2| bound & (1 << s2) == 0).expect("an unbound source");
                    let nb = bound | (1 << si);
                    let checks: Vec<usize> = (0..conj.len()).filter(|c| !used[*c] && refs[*c] & !nb == 0).collect();
                    if let Src::Lazy(l) = &srcs[si] {
                        let node = l.node;
                        srcs[si] = Src::Tab(self.eval(f, run, p, node)?, node);
                    }
                    let rows: Vec<u32> = match &srcs[si] {
                        Src::Tab(t, _) => (0..t.n as u32).collect(),
                        Src::Nat(n) => f.native_rows(*n),
                        Src::Lazy(_) => unreachable!(),
                    };
                    for ti in 0..count {
                        for (s2, cur_s) in cur.iter_mut().enumerate() {
                            if bound & (1 << s2) != 0 {
                                *cur_s = tuples[s2][ti];
                            }
                        }
                        self.stats.touched[node] += rows.len() as u64;
                        for r in &rows {
                            cur[si] = *r;
                            if checks.iter().all(|c| truthy(eval(run, f, &srcs, &cur, &conj[*c]))) {
                                for s2 in 0..k {
                                    if nb & (1 << s2) != 0 {
                                        next[s2].push(cur[s2]);
                                    }
                                }
                            }
                        }
                    }
                    for c in checks {
                        used[c] = true;
                    }
                    bound = nb;
                }
            }
            count = next[driver].len();
            tuples = next;
        }
        if used.iter().any(|u| !u) {
            return Err("a test no source binds".to_owned());
        }
        // the output
        let items: Vec<(String, E)> = match &j["items"] {
            J::Null => {
                let mut v = Vec::new();
                for (i, (_, ns, ts)) in names.iter().enumerate() {
                    for (c, nm) in ns.iter().enumerate() {
                        if !v.iter().any(|(x, _): &(String, E)| x == nm) {
                            v.push((nm.clone(), E::Col(i, c, ts[c])));
                        }
                    }
                }
                v
            }
            J::Array(a) => a.iter().map(|it| Ok((s(&it[0])?.to_owned(), self.expr(run, &it[1], &names)?))).collect::<R<_>>()?,
            _ => return Err("items".to_owned()),
        };
        let mut out = Table::with(items.iter().map(|i| i.0.clone()).collect(), items.iter().map(|i| static_ty(&i.1)).collect());
        for ti in 0..count {
            for (s2, cur_s) in cur.iter_mut().enumerate() {
                *cur_s = tuples[s2][ti];
            }
            for (c, (_, e)) in items.iter().enumerate() {
                let v = eval(run, f, &srcs, &cur, e).1;
                out.cols[c].push(v);
            }
            out.n += 1;
        }
        Ok(out)
    }

    fn axis(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize, a: &J) -> R<Table> {
        let from = self.eval(f, run, p, self.arg_rel(node, "from")?)?;
        let rel = self.arg_rel(node, "relation")?;
        let dir = s(&a["direction"])?.to_owned();
        let domain = match a.get("domain") {
            Some(J::Object(_)) => {
                let d = self.eval(f, run, p, self.arg_rel(node, "domain")?)?;
                let c = d.col("id")?;
                Some(d.cols[c].iter().copied().collect::<rustc_hash::FxHashSet<u64>>())
            }
            _ => None,
        };
        let admitted = |x: u64| domain.as_ref().is_none_or(|d| d.contains(&x));
        let ic = from.col("id")?;
        let mut out = Table::with(from.names.clone(), from.tys.clone());
        out.names.push("related".to_owned());
        out.tys.push(Ty::Node);
        let mut row = Vec::new();
        let tree = self.plan.nodes[rel].op == "base" && native_of(self.plan.nodes[rel].j["rel"].as_str().unwrap_or("")) == Some(Native::Child);
        // a made hierarchy: its parent and child columns, keyed by the start side
        let (up_map, down_map) = if tree {
            (None, None)
        } else {
            let t = self.eval(f, run, p, rel)?;
            let (pc, cc) = (t.col("parent")?, t.col("child")?);
            let mut up: FxHashMap<u64, Vec<u64>> = FxHashMap::default();
            let mut down: FxHashMap<u64, Vec<u64>> = FxHashMap::default();
            for r in 0..t.n {
                up.entry(t.cols[cc][r]).or_default().push(t.cols[pc][r]);
                down.entry(t.cols[pc][r]).or_default().push(t.cols[cc][r]);
            }
            (Some(up), Some(down))
        };
        let mut found: Vec<u64> = Vec::new();
        for i in 0..from.n {
            let x = from.cols[ic][i];
            found.clear();
            match dir.as_str() {
                "parent" | "ancestor" | "nearest-ancestor" => {
                    if tree {
                        let mut at = x as u32;
                        while (at as usize) < f.n && f.has_parent(at) {
                            at = f.parent[at as usize];
                            found.push(u64::from(at));
                            if dir == "parent" || (dir == "nearest-ancestor" && admitted(u64::from(at))) {
                                break;
                            }
                        }
                    } else {
                        let up = up_map.as_ref().expect("up");
                        let mut seen = rustc_hash::FxHashSet::default();
                        let mut frontier = vec![x];
                        'walk: while let Some(at) = frontier.pop() {
                            if let Some(ps) = up.get(&at) {
                                for q in ps {
                                    if seen.insert(*q) {
                                        found.push(*q);
                                        if dir == "nearest-ancestor" && admitted(*q) {
                                            break 'walk;
                                        }
                                        if dir != "parent" {
                                            frontier.push(*q);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                "descendant" | "child" => {
                    if tree {
                        if (x as usize) < f.n {
                            if dir == "child" {
                                for c in node_table::children_of(f.table, Position(x as u32)) {
                                    found.push(u64::from(c.0));
                                }
                            } else {
                                let end = node_table::subtree_end_of(f.table, Position(x as u32)).0;
                                found.extend((x as u32 + 1..end).map(u64::from));
                            }
                        }
                    } else {
                        let down = down_map.as_ref().expect("down");
                        let mut seen = rustc_hash::FxHashSet::default();
                        let mut stack = vec![x];
                        while let Some(at) = stack.pop() {
                            if let Some(cs) = down.get(&at) {
                                for c in cs {
                                    if seen.insert(*c) {
                                        found.push(*c);
                                        if dir != "child" {
                                            stack.push(*c);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                "next-sibling" => {
                    if (x as usize) < f.n && f.has_parent(x as u32) {
                        let sibs = node_table::children_of(f.table, Position(f.parent[x as usize]));
                        if let Some(at) = sibs.iter().position(|c| u64::from(c.0) == x)
                            && let Some(nx) = sibs.get(at + 1)
                        {
                            found.push(u64::from(nx.0));
                        }
                    }
                }
                other => return Err(format!("axis direction {other}")),
            }
            for q in &found {
                if admitted(*q) {
                    from.row(i, &mut row);
                    row.push(*q);
                    out.push_row(&row);
                    if dir == "nearest-ancestor" {
                        break;
                    }
                }
            }
        }
        Ok(out)
    }

    fn ordered(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize, a: &J, as_of: bool) -> R<Table> {
        let l = self.eval(f, run, p, self.arg_rel(node, "left")?)?;
        let r = self.eval(f, run, p, self.arg_rel(node, "right")?)?;
        let lk: Vec<usize> = strs(&a["left-on"])?.iter().map(|c| l.col(c)).collect::<R<_>>()?;
        let rk: Vec<usize> = strs(&a["right-on"])?.iter().map(|c| r.col(c)).collect::<R<_>>()?;
        let (lo, ro) = (l.col(s(&a["left-order"])?)?, r.col(s(&a["right-order"])?)?);
        let outm = a["output"].as_object().ok_or("output")?;
        let mut names = Vec::new();
        let mut tys = Vec::new();
        let mut picks = Vec::new();
        for (name, v) in outm {
            let side = s(&v[0])?;
            let (t, c) = if side == "left" { (&l, l.col(s(&v[1])?)?) } else { (&r, r.col(s(&v[1])?)?) };
            names.push(name.clone());
            tys.push(t.tys[c]);
            picks.push((side == "left", c));
        }
        let mut out = Table::with(names, tys);
        // right rows by key, ordered
        let mut groups: FxHashMap<Vec<u64>, Vec<(i64, u32)>> = FxHashMap::default();
        for i in 0..r.n {
            let k: Vec<u64> = rk.iter().map(|c| r.cols[*c][i]).collect();
            groups.entry(k).or_default().push((r.cols[ro][i] as i64, i as u32));
        }
        for g in groups.values_mut() {
            g.sort_unstable();
        }
        let mut lefts: FxHashMap<Vec<u64>, Vec<(i64, u32)>> = FxHashMap::default();
        for i in 0..l.n {
            let k: Vec<u64> = lk.iter().map(|c| l.cols[*c][i]).collect();
            lefts.entry(k).or_default().push((l.cols[lo][i] as i64, i as u32));
        }
        let mut emit = |out: &mut Table, li: u32, ri: u32| {
            let row: Vec<u64> = picks.iter().map(|(left, c)| if *left { l.cols[*c][li as usize] } else { r.cols[*c][ri as usize] }).collect();
            out.push_row(&row);
        };
        if as_of {
            let bound = s(&a["bound"])?;
            for (k, ls) in &lefts {
                let Some(g) = groups.get(k) else { continue };
                for (o, li) in ls {
                    let (lo_i, hi_i) = (g.partition_point(|x| x.0 < *o), g.partition_point(|x| x.0 <= *o));
                    let target = match bound {
                        "before" => (lo_i > 0).then(|| g[lo_i - 1].0),
                        "at-or-before" => (hi_i > 0).then(|| g[hi_i - 1].0),
                        "after" => g.get(hi_i).map(|x| x.0),
                        "at-or-after" => g.get(lo_i).map(|x| x.0),
                        other => return Err(format!("as-of bound {other}")),
                    };
                    if let Some(t) = target {
                        let a0 = g.partition_point(|x| x.0 < t);
                        for x in &g[a0..] {
                            if x.0 != t {
                                break;
                            }
                            emit(&mut out, *li, x.1);
                        }
                    }
                }
            }
        } else {
            for (k, ls) in lefts.iter_mut() {
                let Some(g) = groups.get(k) else { continue };
                ls.sort_unstable();
                let mut at = 0;
                for (o, li) in ls.iter() {
                    while at < g.len() && g[at].0 <= *o {
                        at += 1;
                    }
                    if at < g.len() {
                        emit(&mut out, *li, g[at].1);
                        at += 1;
                    }
                }
            }
        }
        Ok(out)
    }

    fn source_matches(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize, a: &J) -> R<Table> {
        let from = self.eval(f, run, p, self.arg_rel(node, "from")?)?;
        let sc = from.col(s(&a["span"])?)?;
        let re = run.re(s(&a["pattern"])?)?;
        let base_line = match a.get("base-line") {
            Some(J::String(b)) => Some(from.col(b)?),
            _ => None,
        };
        let line_out = a.get("line-output").and_then(J::as_str).map(str::to_owned);
        let cap = a.get("capture").and_then(J::as_str).map(str::to_owned);
        let cap_out = a.get("capture-output").and_then(J::as_str).map(str::to_owned);
        let cap_text = a.get("capture-text-output").and_then(J::as_str).map(str::to_owned);
        let cap_line = a.get("capture-line-output").and_then(J::as_str).map(str::to_owned);
        let mut out = Table::with(from.names.clone(), from.tys.clone());
        out.names.push(s(&a["output"])?.to_owned());
        out.tys.push(Ty::Span);
        if let Some(lo) = &line_out {
            out.names.push(lo.clone());
            out.tys.push(Ty::Num);
        }
        if cap.is_some() {
            out.names.push(cap_out.clone().ok_or("capture-output")?);
            out.tys.push(Ty::Span);
            if let Some(ct) = &cap_text {
                out.names.push(ct.clone());
                out.tys.push(Ty::Sym);
            }
            if let Some(cl) = &cap_line {
                out.names.push(cl.clone());
                out.tys.push(Ty::Num);
            }
        }
        let mut row = Vec::new();
        for i in 0..from.n {
            let sp = from.cols[sc][i];
            if sp == NULL {
                continue;
            }
            let (st, en) = ((sp >> 32) as usize, (sp & 0xffff_ffff) as usize);
            let text = &f.src[st..en];
            let line0 = base_line.map(|b| from.cols[b][i] as i64);
            let line_at = |off: usize| -> i64 { line0.unwrap_or(0) + text[..off].iter().filter(|c| **c == b'\n').count() as i64 };
            for m in re.captures_iter(text) {
                let w = m.get(0).expect("whole");
                let g = match &cap {
                    Some(c) => match m.name(c) {
                        Some(g) => Some(g),
                        None => continue,
                    },
                    None => None,
                };
                from.row(i, &mut row);
                row.push((((st + w.start()) as u64) << 32) | (st + w.end()) as u64);
                if line_out.is_some() {
                    row.push(line_at(w.start()) as u64);
                }
                if let Some(g) = g {
                    row.push((((st + g.start()) as u64) << 32) | (st + g.end()) as u64);
                    if cap_text.is_some() {
                        row.push(u64::from(run.sym(g.as_bytes())));
                    }
                    if cap_line.is_some() {
                        row.push(line_at(g.start()) as u64);
                    }
                }
                out.push_row(&row);
            }
        }
        Ok(out)
    }
}

fn reach_of(l: &Lazy, key: &[u64]) -> Rc<Vec<u64>> {
    reach_dir(l, key, true)
}

fn reach_dir(l: &Lazy, key: &[u64], forward: bool) -> Rc<Vec<u64>> {
    let (cache, adj) = if forward { (&l.cache, &l.adj) } else { (&l.rcache, &l.radj) };
    if let Some(r) = cache.borrow().get(key) {
        return r.clone();
    }
    let kp = &key[..key.len() - 1];
    let mut seen = rustc_hash::FxHashSet::default();
    let mut out = Vec::new();
    let mut stack = vec![key[key.len() - 1]];
    while let Some(x) = stack.pop() {
        let mut kx = kp.to_vec();
        kx.push(x);
        if let Some(ns) = adj.get(&kx) {
            for y in ns {
                if seen.insert(*y) {
                    out.push(*y);
                    stack.push(*y);
                }
            }
        }
    }
    let _ = (&l.edges, l.s, l.d);
    let rc = Rc::new(out);
    cache.borrow_mut().insert(key.to_vec(), rc.clone());
    rc
}

fn index_of(t: &Table, cols: &[usize]) -> FxHashMap<Vec<u64>, Vec<u32>> {
    let mut m: FxHashMap<Vec<u64>, Vec<u32>> = FxHashMap::default();
    for r in 0..t.n {
        m.entry(cols.iter().map(|c| t.cols[*c][r]).collect()).or_default().push(r as u32);
    }
    m
}

fn table_index(f: &mut File<'_>, tn: usize, t: &Table, cols: &[usize]) -> Rc<FxHashMap<Vec<u64>, Vec<u32>>> {
    let key = (tn, cols.to_vec());
    if let Some(ix) = f.index.get(&key) {
        return ix.clone();
    }
    let rc = Rc::new(index_of(t, cols));
    f.index.insert(key, rc.clone());
    rc
}

/// A native tree source's first rows from the kind buckets, when its own tests require a kind among literal words.
fn kind_bucket(run: &mut Run, f: &File<'_>, nat: Native, si: usize, conj: &[E], own: &[usize]) -> Option<Vec<u32>> {
    if nat != Native::Tree {
        return None;
    }
    fn kinds(e: &E, si: usize, out: &mut Vec<u32>) -> bool {
        match e {
            E::Cmp(o, a, b) if o == "=" => match (&**a, &**b) {
                (E::Col(s, 1, _), E::Text(t)) | (E::Text(t), E::Col(s, 1, _)) if *s == si => {
                    out.push(*t);
                    true
                }
                _ => false,
            },
            E::Or(a, b) => kinds(a, si, out) && kinds(b, si, out),
            E::In(a, b, false) => match (&**a, &**b) {
                (E::Col(s, 1, _), E::List(items)) if *s == si => items.iter().all(|i| {
                    if let E::Text(t) = i {
                        out.push(*t);
                        true
                    } else {
                        false
                    }
                }),
                _ => false,
            },
            _ => false,
        }
    }
    for c in own {
        let mut ks = Vec::new();
        if kinds(&conj[*c], si, &mut ks) {
            let mut rows = Vec::new();
            for (code, sym) in run.kind_sym.iter().enumerate() {
                if ks.contains(sym) {
                    rows.extend(node_table::ids_of_kind(f.table, KindBits(code as u16)).iter().map(|p| p.0));
                }
            }
            rows.sort_unstable();
            return Some(rows);
        }
    }
    None
}

fn flatten(e: E, out: &mut Vec<E>) {
    match e {
        E::And(a, b) => {
            flatten(*a, out);
            flatten(*b, out);
        }
        e => out.push(e),
    }
}

fn refs_of(e: &E) -> u64 {
    match e {
        E::Col(s, _, _) => 1 << s,
        E::List(v) | E::Call(_, v) => v.iter().map(refs_of).fold(0, |a, b| a | b),
        E::Cmp(_, a, b) | E::In(a, b, _) | E::And(a, b) | E::Or(a, b) => refs_of(a) | refs_of(b),
        E::Not(a) | E::Re(a, _) => refs_of(a),
        _ => 0,
    }
}

fn static_ty(e: &E) -> Ty {
    match e {
        E::Col(_, _, t) => *t,
        E::Num(_) => Ty::Num,
        E::Call(n, _) if n == "integer_successor" => Ty::Num,
        E::Text(_) | E::Call(..) => Ty::Sym,
        _ => Ty::Flag,
    }
}

fn get(run: &mut Run, f: &File<'_>, srcs: &[Src], cur: &[u32], si: usize, col: usize) -> u64 {
    match &srcs[si] {
        Src::Tab(t, _) => t.cols[col][cur[si] as usize],
        Src::Nat(n) => f.native_get(run, *n, cur[si], col),
        Src::Lazy(_) => NULL,
    }
}

fn text_of(run: &Run, v: V) -> Vec<u8> {
    match v.ty() {
        Ty::Sym => run.text(v.1 as u32).to_vec(),
        Ty::File => run.text(run.path_sym[v.1 as usize]).to_vec(),
        Ty::Num => (v.1 as i64).to_string().into_bytes(),
        Ty::Flag => if v.1 == 1 { b"true".to_vec() } else { b"false".to_vec() },
        _ => Vec::new(),
    }
}

fn flag(b: bool) -> V {
    V::new(Ty::Flag, u64::from(b))
}

/// The ordering of two values, with a text compared to a number or a flag read as one.
fn compare(run: &Run, a: V, b: V) -> Option<std::cmp::Ordering> {
    use Ty::*;
    if a.1 == NULL || b.1 == NULL {
        return None;
    }
    match (a.ty(), b.ty()) {
        (Num, Num) => Some((a.1 as i64).cmp(&(b.1 as i64))),
        (Sym, Sym) => {
            if a.1 == b.1 {
                Some(std::cmp::Ordering::Equal)
            } else {
                Some(run.text(a.1 as u32).cmp(run.text(b.1 as u32)))
            }
        }
        (Num, Sym) | (Sym, Num) | (Flag, Sym) | (Sym, Flag) | (File, Sym) | (Sym, File) => Some(text_of(run, a).cmp(&text_of(run, b))),
        _ => Some(a.1.cmp(&b.1)),
    }
}

fn eval(run: &mut Run, f: &File<'_>, srcs: &[Src], cur: &[u32], e: &E) -> V {
    match e {
        E::True => flag(true),
        E::Col(si, c, t) => V::new(*t, get(run, f, srcs, cur, *si, *c)),
        E::Lit(v) => *v,
        E::Text(t) => V::new(Ty::Sym, u64::from(*t)),
        E::Num(n) => V::new(Ty::Num, *n as u64),
        E::List(_) => flag(false),
        E::And(a, b) => flag(truthy(eval(run, f, srcs, cur, a)) && truthy(eval(run, f, srcs, cur, b))),
        E::Or(a, b) => flag(truthy(eval(run, f, srcs, cur, a)) || truthy(eval(run, f, srcs, cur, b))),
        E::Not(a) => flag(!truthy(eval(run, f, srcs, cur, a))),
        E::Cmp(o, a, b) => {
            let (x, y) = (eval(run, f, srcs, cur, a), eval(run, f, srcs, cur, b));
            let Some(ord) = compare(run, x, y) else { return flag(false) };
            use std::cmp::Ordering::*;
            flag(match o.as_str() {
                "=" => ord == Equal,
                "!=" => ord != Equal,
                "<" => ord == Less,
                "<=" => ord != Greater,
                ">" => ord == Greater,
                ">=" => ord != Less,
                _ => false,
            })
        }
        E::In(a, b, neg) => {
            let x = eval(run, f, srcs, cur, a);
            let hit = match &**b {
                E::List(items) => items.iter().any(|i| {
                    let y = eval(run, f, srcs, cur, i);
                    compare(run, x, y) == Some(std::cmp::Ordering::Equal)
                }),
                _ => false,
            };
            flag(hit != *neg)
        }
        E::Re(a, re) => {
            let x = eval(run, f, srcs, cur, a);
            flag(match x.ty() {
                Ty::Sym => re.is_match(run.text(x.1 as u32)),
                _ => re.is_match(&text_of(run, x)),
            })
        }
        E::Call(name, args) => {
            let vs: Vec<V> = args.iter().map(|x| eval(run, f, srcs, cur, x)).collect();
            call(run, name, &vs)
        }
    }
}

fn call(run: &mut Run, name: &str, vs: &[V]) -> V {
    let t = |run: &Run, i: usize| text_of(run, vs[i]);
    match name {
        "matches" => {
            let p = String::from_utf8(t(run, 1)).unwrap_or_default();
            let re = run.re(&p).expect("a regex");
            flag(re.is_match(&t(run, 0)))
        }
        "starts_with" => flag(t(run, 0).starts_with(&t(run, 1))),
        "ends_with" => flag(t(run, 0).ends_with(&t(run, 1))),
        "lower" => {
            let s = String::from_utf8_lossy(&t(run, 0)).to_lowercase();
            V::new(Ty::Sym, u64::from(run.sym(s.as_bytes())))
        }
        "concat" => {
            let mut s = Vec::new();
            for i in 0..vs.len() {
                s.extend(t(run, i));
            }
            V::new(Ty::Sym, u64::from(run.sym(&s)))
        }
        "glob_any" => {
            let g = String::from_utf8(t(run, 1)).unwrap_or_default();
            let re = run.re(&glob_pattern(&g)).expect("a glob");
            flag(re.is_match(&t(run, 0)))
        }
        "natural_lt" => flag(natural_lt(&String::from_utf8_lossy(&t(run, 0)), &String::from_utf8_lossy(&t(run, 1)))),
        "integer_successor" => V::new(Ty::Num, (vs[0].1 as i64 + 1) as u64),
        _ => panic!("function {name} not in the step set"),
    }
}

fn glob_pattern(g: &str) -> String {
    let mut s = String::from("^");
    for c in g.chars() {
        match c {
            '*' => s.push_str(".*"),
            '?' => s.push('.'),
            '{' => s.push_str("(?:"),
            '}' => s.push(')'),
            ',' => s.push('|'),
            c => s.push_str(&regex::escape(&c.to_string())),
        }
    }
    s.push('$');
    s
}

fn path_pattern(p: &str) -> String {
    let mut s = String::from("^");
    let mut rest = p;
    while let Some(o) = rest.find('{') {
        s.push_str(&regex::escape(&rest[..o]));
        let c = rest[o..].find('}').expect("a closed capture") + o;
        let name = &rest[o + 1..c];
        if let Some(n) = name.strip_prefix('*') {
            s.push_str(&format!("(?P<{n}>.*)"));
        } else {
            s.push_str(&format!("(?P<{name}>[^/]*)"));
        }
        rest = &rest[c + 1..];
    }
    s.push_str(&regex::escape(rest));
    s.push('$');
    s
}

/// Natural order, as the language reference states it.
fn natural_lt(a: &str, b: &str) -> bool {
    fn units(s: &str) -> Vec<(bool, String)> {
        let mut out: Vec<(bool, String)> = Vec::new();
        let mut cur = String::new();
        let mut digits = false;
        for ch in s.chars() {
            if ch.is_whitespace() {
                if !cur.is_empty() {
                    out.push((digits, std::mem::take(&mut cur)));
                }
                continue;
            }
            let d = ch.is_ascii_digit();
            if d {
                if !digits && !cur.is_empty() {
                    out.push((false, std::mem::take(&mut cur)));
                }
                digits = true;
                cur.push(ch);
            } else {
                if digits && !cur.is_empty() {
                    out.push((true, std::mem::take(&mut cur)));
                }
                digits = false;
                out.push((false, ch.to_string()));
            }
        }
        if !cur.is_empty() {
            out.push((digits, cur));
        }
        out
    }
    let (x, y) = (units(a), units(b));
    for (p, q) in x.iter().zip(y.iter()) {
        let ord = match (p.0, q.0) {
            (true, true) => {
                if p.1.starts_with('0') || q.1.starts_with('0') {
                    p.1.cmp(&q.1)
                } else {
                    p.1.len().cmp(&q.1.len()).then_with(|| p.1.cmp(&q.1))
                }
            }
            _ => p.1.cmp(&q.1),
        };
        if ord != std::cmp::Ordering::Equal {
            return ord == std::cmp::Ordering::Less;
        }
    }
    x.len() < y.len()
}

/// `token-link` from the token order (step 5's pass): next token, matching and enclosing delimiter.
fn token_links(f: &File<'_>, run: &mut Run) -> Table {
    let (nt, md, ed) = (run.sym(b"next-token"), run.sym(b"matching-delimiter"), run.sym(b"enclosing-delimiter"));
    let mut out = Table::with(
        vec!["source".into(), "target".into(), "link-kind".into(), "file".into(), "span".into()],
        vec![Ty::Node, Ty::Node, Ty::Sym, Ty::File, Ty::Span],
    );
    let start = node_table::node_count(f.table).0 as u32;
    let end = node_table::own_end(f.table).0;
    let mut stack: Vec<(u16, u32)> = Vec::new();
    let mut previous: Option<u32> = None;
    let closer = |k: u16| run.pairs.iter().find(|(o, _)| *o == k).map(|(_, c)| *c);
    let is_closer = |k: u16| run.pairs.iter().any(|(_, c)| *c == k);
    let mut rows: Vec<[u64; 5]> = Vec::new();
    let sp = |p: u32| (u64::from(f.start[p as usize]) << 32) | u64::from(f.end[p as usize]);
    for at in start..end {
        let k = f.kind[at as usize];
        if !(k >= run.token_first && k <= run.token_last) {
            continue;
        }
        if k == run.token_error {
            previous = None;
            stack.clear();
            continue;
        }
        let mut paired = None;
        let mut enclosing = stack.last().map(|o| o.1);
        if let Some(close) = closer(k) {
            stack.push((close, at));
        } else if is_closer(k) {
            match stack.pop() {
                Some((close, open)) if close == k => {
                    paired = Some((open, at));
                    enclosing = stack.last().map(|o| o.1);
                }
                _ => {
                    stack.clear();
                    enclosing = None;
                }
            }
        }
        let fl = u64::from(f.file);
        if let Some(p) = previous.replace(at) {
            rows.push([u64::from(p), u64::from(at), u64::from(nt), fl, sp(p)]);
        }
        if let Some((o, c)) = paired {
            rows.push([u64::from(o), u64::from(c), u64::from(md), fl, sp(o)]);
        }
        if let Some(t) = enclosing {
            rows.push([u64::from(at), u64::from(t), u64::from(ed), fl, sp(at)]);
        }
    }
    for r in rows {
        out.push_row(&r);
    }
    out
}

/// The provider's rows of a relation as a table: texts interned run-wide, nodes as positions.
pub(super) fn filled_table(run: &mut Run, rel: &str, filled: &languages::FilledColumns, words: &languages::FileWords) -> R<Table> {
    let (_, names, tys) = run.catalog(rel).ok_or_else(|| format!("no relation {rel}"))?;
    let n = filled.0.first().map_or(0, |c| c.0.len());
    let mut out = Table::with(names, tys.clone());
    for (c, ty) in tys.iter().enumerate() {
        let Some(column) = filled.0.get(c) else {
            out.cols[c] = vec![NULL; n];
            continue;
        };
        out.cols[c] = match ty {
            Ty::Sym => column.0.iter().map(|w| u64::from(run.sym(languages::facts::text(words, *w).0.as_bytes()))).collect(),
            Ty::Node => column.0.iter().map(|w| w.0 & 0xffff_ffff).collect(),
            _ => column.0.iter().map(|w| w.0).collect(),
        };
    }
    out.n = n;
    Ok(out)
}

/// A tree relation as this file reads it in place, made whole (for `--check-bases`).
pub(super) fn native_table(f: &mut File<'_>, run: &mut Run, rel: &str) -> Table {
    if rel == "token-link" {
        return token_links(f, run);
    }
    let nat = native_of(rel).expect("a native relation");
    let cols = native_cols(nat);
    let mut out = Table::with(cols.iter().map(|c| c.0.to_owned()).collect(), cols.iter().map(|c| c.1).collect());
    for r in f.native_rows(nat) {
        for c in 0..cols.len() {
            let v = f.native_get(run, nat, r, c);
            out.cols[c].push(v);
        }
        out.n += 1;
    }
    out
}

/// Two tables' rows on their common columns, as multisets: counts and the first differences.
pub(super) fn diff(run: &Run, a: &Table, b: &Table) -> String {
    let common: Vec<(usize, usize)> = a.names.iter().enumerate().filter_map(|(i, n)| b.names.iter().position(|m| m == n).map(|j| (i, j))).collect();
    let only_a: Vec<&String> = a.names.iter().filter(|n| !b.names.contains(n)).collect();
    let only_b: Vec<&String> = b.names.iter().filter(|n| !a.names.contains(n)).collect();
    let rows = |t: &Table, side: usize| {
        let mut m: std::collections::BTreeMap<Vec<u64>, i64> = std::collections::BTreeMap::new();
        for r in 0..t.n {
            let k: Vec<u64> = common.iter().map(|(i, j)| t.cols[if side == 0 { *i } else { *j }][r]).collect();
            *m.entry(k).or_default() += 1;
        }
        m
    };
    let (ra, rb) = (rows(a, 0), rows(b, 1));
    let mut extra = Vec::new();
    let mut missing = Vec::new();
    for (k, c) in &ra {
        let d = c - rb.get(k).copied().unwrap_or(0);
        if d > 0 {
            extra.push(k.clone());
        }
    }
    for (k, c) in &rb {
        let d = c - ra.get(k).copied().unwrap_or(0);
        if d > 0 {
            missing.push(k.clone());
        }
    }
    let show = |k: &Vec<u64>| {
        k.iter()
            .zip(&common)
            .map(|(v, (i, _))| match a.tys[*i] {
                Ty::Sym => format!("{}={:?}", a.names[*i], String::from_utf8_lossy(run.text(*v as u32))),
                _ => format!("{}={v}", a.names[*i]),
            })
            .collect::<Vec<_>>()
            .join(",")
    };
    format!(
        "mine {} theirs {} extra {} missing {} only-mine-cols {:?} only-theirs-cols {:?}{}{}",
        a.n,
        b.n,
        extra.len(),
        missing.len(),
        only_a,
        only_b,
        extra.first().map(|k| format!(" first-extra [{}]", show(k))).unwrap_or_default(),
        missing.first().map(|k| format!(" first-missing [{}]", show(k))).unwrap_or_default()
    )
}
