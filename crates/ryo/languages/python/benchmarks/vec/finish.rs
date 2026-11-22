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

/// 7: a mechanism switch (`VEC_*` in the environment), read once a run at its call site, not once a step.
macro_rules! sw {
    ($n:literal) => {{
        static S: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        *S.get_or_init(|| std::env::var_os($n).is_some())
    }};
}

#[path = "finish/drive.rs"]
mod drive;
#[path = "finish/handover.rs"]
pub(super) mod handover;
#[path = "band.rs"]
pub(super) mod band;
#[path = "closure_walk.rs"]
mod closure_walk;
#[path = "walk.rs"]
mod walk;
#[path = "filter_order.rs"]
mod filter_order;
#[path = "finish/kernel_input.rs"]
mod kernel_input;
#[path = "finish/union_kernel.rs"]
mod union_kernel;
#[path = "finish/antijoin_kernel.rs"]
mod antijoin_kernel;
#[path = "finish/frame.rs"]
mod frame;
#[path = "finish/known.rs"]
mod known;
#[path = "finish/empty_why.rs"]
pub(super) mod empty_why;

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

/// 7: a table's column names and types, shared with its step's schema (never copied per table).
pub(super) type Names = Rc<Vec<String>>;
pub(super) type Tys = Rc<Vec<Ty>>;

/// A column table.
#[derive(Default, Clone, Debug)]
pub(super) struct Table {
    pub(super) names: Names,
    pub(super) tys: Tys,
    pub(super) cols: Vec<Vec<u64>>,
    pub(super) n: usize,
}

impl Table {
    fn with(names: impl Into<Names>, tys: impl Into<Tys>) -> Self {
        let (names, tys) = (names.into(), tys.into());
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
    pub(super) rels: FxHashMap<String, (languages::Relation, Names, Tys)>,
    pub(super) link_sym: [u32; 3],
    /// The project's root on disk and its search roots, for `project-namespace-membership`.
    pub(super) root_dir: String,
    pub(super) search_roots: Vec<String>,
    pub(super) classified: FxHashMap<String, Vec<&'static str>>,
    /// Functions a step asked that are not in the step set (the rule is refused).
    pub(super) missing: std::collections::BTreeSet<String>,
    /// 6h: the provider relations read in place (`Native::Fact`), with their column counts.
    pub(super) fact_rels: Vec<(String, usize)>,
}

impl Run {
    pub(super) fn links(&mut self) {
        self.link_sym = [self.sym(b"next-token"), self.sym(b"matching-delimiter"), self.sym(b"enclosing-delimiter")];
    }
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
    pub(super) fn catalog(&mut self, name: &str) -> Option<(languages::Relation, Names, Tys)> {
        if let Some(r) = self.rels.get(name) {
            return Some(r.clone());
        }
        if name == REGION_REL {
            // 6f probe: the provider's per-node evaluation region (`ColumnFile::evaluation_region`); not a catalog relation,
            // so the placeholder relation is never filled (`Provider::fill` answers it by name).
            let relation = catalog::relation_named(NameRef("binding-read"))?;
            let names: Names = Rc::new(REGION_COLS.iter().map(|c| c.0.to_owned()).collect());
            let tys: Tys = Rc::new(REGION_COLS.iter().map(|c| c.1).collect());
            self.rels.insert(name.to_owned(), (relation, names.clone(), tys.clone()));
            return Some((relation, names, tys));
        }
        let relation = catalog::relation_named(NameRef(name))?;
        let names: Names = Rc::new(catalog::columns(relation).into_iter().map(|c| catalog::column_name(c).0.to_owned()).collect());
        let tys: Tys = Rc::new(catalog::kinds(relation).iter().map(|k| ty_of(*k)).collect());
        self.rels.insert(name.to_owned(), (relation, names.clone(), tys.clone()));
        Some((relation, names, tys))
    }
}

/// 6f probe: the provider's evaluation region of every node, by name.
pub(super) const REGION_REL: &str = "evaluation-region";
pub(super) const REGION_COLS: [(&str, Ty); 6] =
    [("id", Ty::Node), ("region", Ty::Node), ("region-kind", Ty::Sym), ("annotation", Ty::Node), ("timing", Ty::Sym), ("file", Ty::File)];

/// 6f probe: a made table's rows as text lines (`label`, path, then `column=value` cells; a node as `id:line`).
pub(super) fn dump_rows(f: &File<'_>, run: &Run, t: &Table, label: &str, path: &str, out: &mut String) {
    use std::fmt::Write as _;
    for r in 0..t.n {
        let _ = write!(out, "{label}\t{path}");
        for c in 0..t.names.len() {
            let v = t.cols[c][r];
            if t.tys[c] == Ty::File {
                continue;
            }
            let _ = write!(out, "\t{}=", t.names[c]);
            if v == NULL {
                out.push_str("null");
                continue;
            }
            match t.tys[c] {
                Ty::Node => {
                    let line = if (v as usize) < f.n { f.line(f.start[v as usize]) } else { 0 };
                    let _ = write!(out, "{v}:{line}");
                }
                Ty::Sym => out.push_str(&String::from_utf8_lossy(run.text(v as u32)).replace(['\t', '\n'], " ")),
                Ty::Span => {
                    let _ = write!(out, "{}-{}", v >> 32, v & 0xffff_ffff);
                }
                _ => {
                    let _ = write!(out, "{}", v as i64);
                }
            }
        }
        out.push('\n');
    }
}

/// A plan node, as lowered.
pub(super) struct PNode {
    pub(super) op: String,
    pub(super) j: J,
    pub(super) inputs: Vec<usize>,
    /// 7: the relation arguments (`args.<name>.rel`), by name.
    pub(super) rels: Vec<(String, usize)>,
}

pub(super) struct Plan {
    pub(super) nodes: Vec<PNode>,
    pub(super) emits: Vec<(String, usize, String, Vec<String>)>,
    /// The survivor sets the plan's seed tests name, by set: the label of the prefix rule whose roots fill it.
    pub(super) seeds: Vec<String>,
    /// 6e item 2: per set, the move paths from the prefix's roots to the other variables the set also holds.
    pub(super) seed_paths: Vec<Vec<Vec<String>>>,
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
            let rels = n["args"].as_object().map(|m| m.iter().filter_map(|(k, v)| v.get("rel").and_then(J::as_u64).map(|r| (k.clone(), r as usize))).collect()).unwrap_or_default();
            PNode { op, j: n.clone(), inputs, rels }
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
    let seeds = j["seeds"].as_array().map(|a| a.iter().map(|x| x.as_str().unwrap_or("").to_owned()).collect()).unwrap_or_default();
    let seed_paths = j["seed_paths"]
        .as_array()
        .map(|a| a.iter().map(|ps| ps.as_array().map(|ps| ps.iter().map(|p| p.as_array().map(|p| p.iter().map(|m| m.as_str().unwrap_or("").to_owned()).collect()).unwrap_or_default()).collect()).unwrap_or_default()).collect())
        .unwrap_or_default();
    Plan { nodes, emits, seeds, seed_paths }
}

/// The relations of the tree read in place, by position.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Native {
    /// `syntax-tree` / `semantic-syntax`: a row an element.
    Tree,
    /// `semantic-child` / `syntax-child`: a row an element with a parent.
    Child,
    /// `source-location`: a row an element.
    Location,
    /// `token-link`: a row a (source token, link kind), from the token order; row = source * 3 + kind.
    Token,
    /// 6h: a provider relation read in place by key: the file's fill, never copied into a step; a probe on any column reads
    /// that column's keyed list (made from the fill on the first ask in a file), and `col = literal` enters by its bucket.
    /// The number is the relation's place in `Run::fact_rels`.
    Fact(u16),
}

/// 6h: the rows of the keyed lists made from provider fills (one entry a fill row a column asked), for the report.
pub(super) static FACT_IX_ROWS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

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
const TOKEN_COLS: [(&str, Ty); 5] = [("source", Ty::Node), ("target", Ty::Node), ("link-kind", Ty::Sym), ("file", Ty::File), ("span", Ty::Span)];

fn native_of(rel: &str) -> Option<Native> {
    match rel {
        "syntax-tree" | "semantic-syntax" => Some(Native::Tree),
        "semantic-child" | "syntax-child" => Some(Native::Child),
        "source-location" => Some(Native::Location),
        "token-link" => Some(Native::Token),
        _ => None,
    }
}

fn native_cols(n: Native) -> &'static [(&'static str, Ty)] {
    match n {
        Native::Tree => &TREE_COLS,
        Native::Child => &CHILD_COLS,
        Native::Location => &LOC_COLS,
        Native::Token => &TOKEN_COLS,
        Native::Fact(_) => &[],
    }
}

/// 6h: a provider relation's place among the facts read in place (`VEC_NO_FACTVIEW` turns this off): every catalog
/// relation the provider fills per file; never the file list or the two project relations made here.
fn fact_of(run: &mut Run, rel: &str) -> Option<Native> {
    if sw!("VEC_NO_FACTVIEW") || native_of(rel).is_some() || matches!(rel, "filesystem.file" | "project-namespace-membership" | "module-identity") {
        return None;
    }
    let (_, names, _) = run.catalog(rel)?;
    let at = match run.fact_rels.iter().position(|(r, _)| r == rel) {
        Some(i) => i,
        None => {
            run.fact_rels.push((rel.to_owned(), names.len()));
            run.fact_rels.len() - 1
        }
    };
    Some(Native::Fact(at as u16))
}

const NONE: u32 = u32::MAX;

/// redo-union pass 2: the step clock, read only where `VEC_STEP_TIME` asks for per-step timing; else no clock is read per
/// run (0: the stats carry calls and rows only).
#[inline]
pub(super) fn step_clock() -> f64 {
    if sw!("VEC_STEP_TIME") { super::thread_cpu() } else { 0.0 }
}

/// The token order's links, one pass a file, made the first time a step reads `token-link` in that file: next token,
/// matching delimiter (both ways) and enclosing delimiter by position, and the tokens each opener encloses.
pub(super) struct Tokens {
    next: Vec<u32>,
    prev: Vec<u32>,
    pair: Vec<u32>,
    opener: Vec<bool>,
    enc: Vec<u32>,
    enclosed: FxHashMap<u32, Vec<u32>>,
    order: Vec<u32>,
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
    text_memo: RefCell<Vec<u32>>,
    by_start: RefCell<Option<Index>>,
    by_end: RefCell<Option<Index>>,
    /// 6e: elements by start line and by end line (a keyed list made on first ask).
    by_line: [RefCell<Option<Index>>; 2],
    tokens: RefCell<Option<Rc<Tokens>>>,
    memo: Vec<Option<Res>>,
    /// The nodes made, in order: a probe's override drops what was made under it.
    log: Vec<usize>,
    vrows: FxHashMap<usize, Rc<Vec<u32>>>,
    vmat: FxHashMap<usize, Rc<Table>>,
    index: FxHashMap<usize, Vec<(Vec<usize>, Rc<Index>)>>,
    fills: FxHashMap<String, Rc<Table>>,
    lazies: FxHashMap<usize, Rc<Lazy>>,
    /// The survivor sets of this file (sorted node ids), by set: the prefix's roots for each seeded rule.
    pub(super) seeds: Vec<Vec<u32>>,
    /// A probed closure walk's adjacency by (closure, forward): the nodes asked so far and their neighbours.
    walks: FxHashMap<(usize, bool), FxHashMap<u64, Vec<u64>>>,
    /// 6h: the fills read in place, by `Native::Fact` place, and their keyed lists by (place, column).
    facts: Vec<Option<Rc<Table>>>,
    fact_ix: RefCell<FxHashMap<(u16, usize), Rc<Index>>>,
    /// redo-union pass 2: whether a step is known empty in this file from its kinds, seeds and per-file tests (`known`).
    known: Vec<u8>,
    /// The kinds the file has, one bit a kind code (made at the first ask).
    kind_bits: Vec<u64>,
}

/// How a step asks the provider for a relation's rows: the visit's facts.
pub(super) trait Provider {
    fn fill(&mut self, run: &mut Run, rel: &str) -> Result<Table, String>;
    /// A node's text in a syntax role: `Ok(None)` where the role cannot hold it.
    fn render(&mut self, node: u32, context: &str) -> Result<Option<Vec<u8>>, String>;
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
            by_line: [RefCell::new(None), RefCell::new(None)],
            tokens: RefCell::new(None),
            memo: vec![None; nodes],
            log: Vec::new(),
            vrows: FxHashMap::default(),
            vmat: FxHashMap::default(),
            index: FxHashMap::default(),
            fills: FxHashMap::default(),
            lazies: FxHashMap::default(),
            seeds: Vec::new(),
            walks: FxHashMap::default(),
            facts: Vec::new(),
            fact_ix: RefCell::new(FxHashMap::default()),
            known: vec![0; nodes],
            kind_bits: Vec::new(),
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
    fn toks(&self, run: &Run) -> Rc<Tokens> {
        if let Some(t) = &*self.tokens.borrow() {
            return t.clone();
        }
        let n = self.n;
        let mut t = Tokens { next: vec![NONE; n], prev: vec![NONE; n], pair: vec![NONE; n], opener: vec![false; n], enc: vec![NONE; n], enclosed: FxHashMap::default(), order: Vec::new() };
        let start = node_table::node_count(self.table).0 as u32;
        let end = node_table::own_end(self.table).0;
        let mut stack: Vec<(u16, u32)> = Vec::new();
        let mut previous: Option<u32> = None;
        for at in start..end {
            let k = self.kind[at as usize];
            if !(k >= run.token_first && k <= run.token_last) {
                continue;
            }
            if k == run.token_error {
                previous = None;
                stack.clear();
                continue;
            }
            t.order.push(at);
            let mut enclosing = stack.last().map(|o| o.1);
            if let Some(close) = run.pairs.iter().find(|(o, _)| *o == k).map(|(_, c)| *c) {
                stack.push((close, at));
            } else if run.pairs.iter().any(|(_, c)| *c == k) {
                match stack.pop() {
                    Some((close, open)) if close == k => {
                        t.pair[open as usize] = at;
                        t.pair[at as usize] = open;
                        t.opener[open as usize] = true;
                        enclosing = stack.last().map(|o| o.1);
                    }
                    _ => {
                        stack.clear();
                        enclosing = None;
                    }
                }
            }
            if let Some(p) = previous.replace(at) {
                t.next[p as usize] = at;
                t.prev[at as usize] = p;
            }
            if let Some(e) = enclosing {
                t.enc[at as usize] = e;
                t.enclosed.entry(e).or_default().push(at);
            }
        }
        let rc = Rc::new(t);
        *self.tokens.borrow_mut() = Some(rc.clone());
        rc
    }
    /// A native relation's cell.
    fn native_get(&self, run: &mut Run, nat: Native, row: u32, col: usize) -> u64 {
        let r = row as usize;
        match (nat, col) {
            (Native::Tree, 0) | (Native::Location, 0) => u64::from(row),
            (Native::Tree, 1) => u64::from(run.kind_sym[usize::from(self.kind[r])]),
            (Native::Tree, 2) => u64::from(self.elem_text(run, row)),
            (Native::Tree, 3) | (Native::Location, 1) | (Native::Child, 4) | (Native::Token, 3) => u64::from(self.file),
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
            (Native::Fact(i), c) => self.facts[usize::from(i)].as_ref().map_or(NULL, |t| t.cols[c][r]),
            (Native::Token, _) => {
                let (src, k) = (row / 3, row % 3);
                match col {
                    0 => u64::from(src),
                    1 => {
                        let t = self.toks(run);
                        u64::from(match k {
                            0 => t.next[src as usize],
                            1 => t.pair[src as usize],
                            _ => t.enc[src as usize],
                        })
                    }
                    2 => u64::from(run.link_sym[k as usize]),
                    4 => (u64::from(self.start[src as usize]) << 32) | u64::from(self.end[src as usize]),
                    _ => NULL,
                }
            }
            _ => NULL,
        }
    }
    fn token_row(t: &Tokens, src: u32, k: u32) -> bool {
        let s = src as usize;
        match k {
            0 => t.next[s] != NONE,
            1 => t.opener[s],
            _ => t.enc[s] != NONE,
        }
    }
    fn native_rows(&self, run: &Run, nat: Native) -> Vec<u32> {
        match nat {
            Native::Tree | Native::Location => (0..self.n as u32).collect(),
            Native::Child => (0..self.n as u32).filter(|p| self.has_parent(*p)).collect(),
            Native::Token => {
                let t = self.toks(run);
                let mut out = Vec::new();
                for &p in &t.order {
                    for k in 0..3 {
                        if Self::token_row(&t, p, k) {
                            out.push(p * 3 + k);
                        }
                    }
                }
                out
            }
            Native::Fact(i) => (0..self.facts[usize::from(i)].as_ref().map_or(0, |t| t.n as u32)).collect(),
        }
    }
    /// 6h: a fill's keyed list on one column (made on the first ask in a file).
    fn fact_list(&self, i: u16, col: usize) -> Rc<Index> {
        if let Some(ix) = self.fact_ix.borrow().get(&(i, col)) {
            return ix.clone();
        }
        let m = match &self.facts[usize::from(i)] {
            Some(t) => {
                FACT_IX_ROWS.fetch_add(t.n as u64, std::sync::atomic::Ordering::Relaxed);
                Index::build(t.n, |r| t.cols[col][r])
            }
            None => Index::build(0, |_| 0),
        };
        let rc = Rc::new(m);
        self.fact_ix.borrow_mut().insert((i, col), rc.clone());
        rc
    }
    /// Whether a native relation is probed in place on a column.
    fn probes(nat: Native, col: usize) -> bool {
        matches!((nat, col), (Native::Tree | Native::Location, 0) | (Native::Child, 0 | 1) | (Native::Location, 3..=6) | (Native::Token, 0 | 1) | (Native::Fact(_), _))
    }
    /// Rows of a native relation whose column holds `v`.
    fn native_probe(&self, run: &Run, nat: Native, col: usize, v: u64, out: &mut Vec<u32>) {
        let ok = v < self.n as u64;
        match (nat, col) {
            (Native::Tree, 0) | (Native::Location, 0) => {
                if ok {
                    out.push(v as u32);
                }
            }
            (Native::Child, 1) => {
                if ok && self.has_parent(v as u32) {
                    out.push(v as u32);
                }
            }
            (Native::Child, 0) => {
                if ok {
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
                    let src = if col == 5 { &self.start } else { &self.end };
                    Index::build(src.len(), |p| u64::from(src[p]))
                });
                if let Some(rows) = idx.get(&v) {
                    out.extend_from_slice(rows);
                }
            }
            (Native::Location, 3) | (Native::Location, 4) => {
                let mut b = self.by_line[col - 3].borrow_mut();
                let idx = b.get_or_insert_with(|| {
                    let src = if col == 3 { &self.start } else { &self.end };
                    Index::build(src.len(), |p| u64::from(self.line(src[p]) as u32))
                });
                if let Some(rows) = idx.get(&(u64::from(v as u32))) {
                    out.extend_from_slice(rows);
                }
            }
            (Native::Token, 0) => {
                if ok {
                    let t = self.toks(run);
                    for k in 0..3 {
                        if Self::token_row(&t, v as u32, k) {
                            out.push(v as u32 * 3 + k);
                        }
                    }
                }
            }
            (Native::Fact(i), c) => {
                if let Some(rows) = self.fact_list(i, c).get(&v) {
                    out.extend_from_slice(rows);
                }
            }
            (Native::Token, 1) => {
                if ok {
                    let t = self.toks(run);
                    let x = v as usize;
                    if t.prev[x] != NONE {
                        out.push(t.prev[x] * 3);
                    }
                    if t.pair[x] != NONE && !t.opener[x] {
                        out.push(t.pair[x] * 3 + 1);
                    }
                    if let Some(es) = t.enclosed.get(&(v as u32)) {
                        out.extend(es.iter().map(|e| e * 3 + 2));
                    }
                }
            }
            _ => {}
        }
    }
}

/// Counters a run keeps: per node, own time, calls, rows out and rows touched.
#[derive(Default)]
pub(super) struct Stats {
    pub(super) time: Vec<f64>,
    pub(super) rows: Vec<u64>,
    pub(super) calls: Vec<u64>,
    pub(super) touched: Vec<u64>,
    pub(super) empty: Vec<u64>,
    pub(super) probes: Vec<u64>,
    /// redo-union pass 2: runs (whole or by key) answered empty before the bracket (`known`), and the seconds of the
    /// bracketed runs that returned nothing.
    pub(super) known: Vec<u64>,
    pub(super) empty_time: Vec<f64>,
    /// pass 2: probe runs that returned nothing (bracketed).
    pub(super) probe_empty: Vec<u64>,
    /// The step that first asked a node whole in a file (diagnostic).
    pub(super) made_by: Vec<usize>,
    /// Why a node was first made whole in a file, where the asking step says (empty: read whole by `made_by`).
    pub(super) why: Vec<&'static str>,
    /// 6e item 4: rows copied into the per-key memo, rows copied out of it, keys looked up in it.
    pub(super) memo_in: u64,
    pub(super) memo_out: u64,
    pub(super) memo_keys: u64,
    /// 7: probes answered by a key list asked before, and their keys.
    pub(super) memo_reused: u64,
    pub(super) memo_reused_keys: u64,
    /// pass 2 (`VEC_STEP_TIME`): seconds of the whole runs answered known empty, from the memo miss to the answer, and
    /// how many (the clock's own pair cost is taken off in the report).
    pub(super) known_time: f64,
    pub(super) known_timed: u64,
    /// pass 2 (`VEC_STEP_TIME`): seconds deciding the per-file static part (`known::static_empty_`, outermost calls).
    pub(super) static_time: f64,
    pub(super) static_depth: u32,
    pub(super) static_visits: u64,
    pub(super) static_file_tests: u64,
    pub(super) rec_time: f64,
    pub(super) rec_busy: bool,
    /// pass 2: keys the per-key memo did not have (run below it), and probes it answered whole.
    pub(super) memo_missing: u64,
    pub(super) memo_whole_hits: u64,
}

// ------------------------------------------------------------------------------------------------ expressions

#[derive(Clone, Debug)]
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
    /// A text tested against a constant regex (from `matches` or `glob_any`), compiled once.
    Re(Box<E>, Rc<regex::bytes::Regex>),
    /// The survivor handoff: whether a node id is in survivor set `k` of this file (the prefix's roots for a rule).
    Seed(Box<E>, usize),
}

/// A node's column names and types, known from the plan before any file.
pub(super) struct Schema {
    names: Names,
    tys: Tys,
}

/// A select over one tree relation (or another such view) with only column picks and tests: never made whole; its rows are
/// read in place by position, entered by a kind bucket or a keyed probe, and tested as they are read.
pub(super) struct ViewSpec {
    base: Native,
    map: Vec<usize>,
    filter: Vec<E>,
}

/// What a node answers in a file.
#[derive(Clone)]
pub(super) enum Res {
    Tab(Rc<Table>),
    View(Rc<ViewSpec>),
}

/// A select compiled once a run.
struct SelPlan {
    srcs: Vec<usize>,
    conj: Vec<E>,
    refs: Vec<u64>,
    items: Vec<(String, E)>,
}

/// One source of a select in a file.
enum Src {
    Tab(Rc<Table>, usize),
    View(Rc<ViewSpec>, usize),
    Base(Native),
    Lazy(Rc<Lazy>),
    /// A step not made yet in this file: probed with the keys a join gives it, or made when it must start the join.
    Pending(usize),
}

/// A closure that is never made whole: it answers its sources' reach when probed by either end.
pub(super) struct Lazy {
    keys: Vec<usize>,
    adj: FxHashMap<Vec<u64>, Vec<u64>>,
    radj: FxHashMap<Vec<u64>, Vec<u64>>,
    node: usize,
    names: Names,
    tys: Tys,
    cache: RefCell<FxHashMap<Vec<u64>, Rc<Vec<u64>>>>,
    rcache: RefCell<FxHashMap<Vec<u64>, Rc<Vec<u64>>>>,
}

fn truthy(v: V) -> bool {
    v.ty() == Ty::Flag && v.1 == 1
}

// ------------------------------------------------------------------------------------------------ the executor

pub(super) struct Exec<'p> {
    pub(super) plan: &'p Plan,
    pub(super) stats: Stats,
    schemas: Vec<Option<Rc<Schema>>>,
    views: Vec<Option<Option<Rc<ViewSpec>>>>,
    sels: Vec<Option<Rc<SelPlan>>>,
    inner: f64,
    deps: FxHashMap<(usize, usize), bool>,
    stack: Vec<usize>,
    /// Steps whose inputs are all run-wide (the file list, written rows): made once a run.
    constant: Vec<Option<bool>>,
    run_memo: Vec<Option<Rc<Table>>>,
    /// redo-union pass 2: per node, what the step bracket reads on every run (`frame`).
    frames: Vec<frame::Frame>,
    ptargets: Vec<Option<Box<[(u32, u32)]>>>,
    /// each step's `args` in the plan (read once).
    args: Vec<&'p J>,
    scratch_made: Vec<usize>,
    scratch_ranges: Vec<(usize, usize)>,
    scratch_order: Vec<(u64, u32)>,
    scratch_set: rustc_hash::FxHashSet<u64>,
    /// The steps that read each node, and whether every one of them reads it only by key (never made whole then).
    consumers: Vec<Vec<usize>>,
    keyed_memo: Vec<Option<bool>>,
    emitted: rustc_hash::FxHashSet<usize>,
    /// 6e item 1: whether a step's rows are restricted by a survivor seed (static, through row-wise positions).
    bound_memo: Vec<Option<bool>>,
    /// The reason the next whole make is asked for (taken by `res`).
    reason: &'static str,
    /// `VEC_STATIC_KEYED`: the 6c static rule decides which steps are read by key (the base of 6e item 1).
    static_keyed: bool,
    /// 6e item 1: a probed input put in place for one step's run only (step, input, rows), never in the file's memo, so
    /// another input that reads the same step (a diamond) still sees all of it.
    over: Option<(usize, usize, Option<usize>, Rc<Table>)>,
    pub(super) tally: band::Tally,
    /// 7: the per-key memo, by (step, column) slot; slots used in this file are cleared at the next (buffers kept).
    pmemo: Vec<MemoSlot>,
    /// pass 2: the memo slots by node, (column, slot) (no map looked up a probe).
    pmemo_at: Vec<Vec<(u32, u32)>>,
    pmemo_dirty: Vec<usize>,
    stamp: u32,
    scratch_keys: Vec<u64>,
    /// redo-union pass 2: what each step needs to return anything (`known`), and its shared empty answer.
    needs: Vec<Option<&'static known::Need>>,
    flat: known::Flat,
    empties: Vec<Option<Rc<Table>>>,
}

/// 7: a key's rows in a memo slot: one range of the slot's table (`start` PENDING while its batch runs).
#[derive(Clone, Copy)]
struct MemoKey {
    start: u32,
    len: u32,
    stamp: u32,
}
const PENDING: u32 = u32::MAX;

/// 7: key lists this short are looked up key by key; longer ones are remembered whole.
const LIST_MIN: usize = 4;

fn hash_keys(keys: &[u64]) -> u64 {
    let mut h: u64 = keys.len() as u64;
    for k in keys {
        h = (h.rotate_left(5) ^ k).wrapping_mul(0x517c_c1b7_2722_0a95);
    }
    h
}

struct MemoSlot {
    t: Table,
    ix: FxHashMap<u64, MemoKey>,
    /// 7: answers by key list (its hash: the list, the answer), this file.
    lists: FxHashMap<u64, (Vec<u64>, Rc<Table>)>,
    dirty: bool,
    /// pass 2: over the run, keys asked and keys missing, probes, key lists answered; `direct` once the memo is seen not
    /// to answer (after `MEMO_TRIAL` probes, under 3% of keys answered and no list): the slot is probed straight.
    asked: u64,
    missed: u64,
    tried: u32,
    list_hits: u32,
    direct: bool,
}

/// pass 2: probes of a memo slot before it may be found useless.
const MEMO_TRIAL: u32 = 256;

type R<T> = Result<T, String>;

fn s(j: &J) -> R<&str> {
    j.as_str().ok_or_else(|| format!("not a text: {j}"))
}
/// A plan argument's names. 7: read once a run (the plan is not changed while it runs: a value is known by its address).
fn strs(j: &J) -> R<Rc<Vec<String>>> {
    thread_local! {
        static SEEN: RefCell<FxHashMap<usize, Rc<Vec<String>>>> = RefCell::new(FxHashMap::default());
    }
    let at = std::ptr::from_ref(j) as usize;
    if let Some(v) = SEEN.with(|m| m.borrow().get(&at).cloned()) {
        return Ok(v);
    }
    let v: Vec<String> = match j {
        J::Array(a) => a.iter().map(|x| s(x).map(str::to_owned)).collect::<R<_>>()?,
        J::String(x) => vec![x.clone()],
        J::Null => Vec::new(),
        _ => return Err(format!("not a list: {j}")),
    };
    let v = Rc::new(v);
    SEEN.with(|m| m.borrow_mut().insert(at, v.clone()));
    Ok(v)
}

fn pos(names: &[String], c: &str) -> R<usize> {
    names.iter().position(|x| x == c).ok_or_else(|| format!("no column {c} in {names:?}"))
}

impl<'p> Exec<'p> {
    pub(super) fn new(plan: &'p Plan) -> Self {
        let k = plan.nodes.len();
        Exec {
            plan,
            stats: Stats { time: vec![0.0; k], rows: vec![0; k], calls: vec![0; k], touched: vec![0; k], empty: vec![0; k], probes: vec![0; k], known: vec![0; k], empty_time: vec![0.0; k], probe_empty: vec![0; k], made_by: vec![usize::MAX; k], why: vec![""; k], memo_in: 0, memo_out: 0, memo_keys: 0, memo_reused: 0, memo_reused_keys: 0, known_time: 0.0, known_timed: 0, static_time: 0.0, static_depth: 0, static_visits: 0, static_file_tests: 0, rec_time: 0.0, rec_busy: false, memo_missing: 0, memo_whole_hits: 0 },
            schemas: vec![None; k],
            views: vec![None; k],
            sels: vec![None; k],
            inner: 0.0,
            deps: FxHashMap::default(),
            stack: Vec::new(),
            constant: vec![None; k],
            run_memo: vec![None; k],
            frames: frame::frames(plan),
            ptargets: vec![None; k],
            args: plan.nodes.iter().map(|n| &n.j["args"]).collect(),
            scratch_made: Vec::new(),
            scratch_ranges: Vec::new(),
            scratch_order: Vec::new(),
            scratch_set: rustc_hash::FxHashSet::default(),
            consumers: {
                let mut c = vec![Vec::new(); k];
                for (i, n) in plan.nodes.iter().enumerate() {
                    for x in &n.inputs {
                        if !c[*x].contains(&i) {
                            c[*x].push(i);
                        }
                    }
                }
                c
            },
            keyed_memo: vec![None; k],
            emitted: plan.emits.iter().map(|e| e.1).collect(),
            bound_memo: vec![None; k],
            reason: "",
            static_keyed: sw!("VEC_STATIC_KEYED"),
            over: None,
            tally: band::Tally::default(),
            pmemo: Vec::new(),
            pmemo_at: vec![Vec::new(); k],
            pmemo_dirty: Vec::new(),
            stamp: 0,
            scratch_keys: Vec::new(),
            needs: vec![None; k],
            flat: known::Flat::new(k),
            empties: vec![None; k],
        }
    }

    pub(super) fn reset_file(&mut self) {
        for i in std::mem::take(&mut self.pmemo_dirty) {
            let sl = &mut self.pmemo[i];
            sl.dirty = false;
            sl.t.n = 0;
            sl.t.cols.iter_mut().for_each(Vec::clear);
            sl.ix.clear();
            sl.lists.clear();
        }
    }

    /// Whether every step that reads `node` reads it only by key: a join that binds one of its columns from another
    /// source, the right side of an antijoin, or a step that passes rows through and is itself read only by key. Such a
    /// node is never made whole: it is read as a batch of keys in, rows out.
    fn keyed(&mut self, run: &mut Run, node: usize) -> bool {
        if let Some(k) = self.keyed_memo[node] {
            return k;
        }
        self.keyed_memo[node] = Some(false);
        let plan: &'p Plan = self.plan;
        let k = !self.emitted.contains(&node)
            && !plan.nodes[node].op.starts_with("closure")
            && !self.consumers[node].is_empty()
            && self.consumers[node].clone().into_iter().all(|c| {
                let op = plan.nodes[c].op.as_str();
                let arg = |name: &str| plan.nodes[c].j["args"][name]["rel"].as_u64() == Some(node as u64);
                match op {
                    "select" => {
                        let Ok(sp) = self.sel(run, c) else { return false };
                        let at: Vec<usize> = (0..sp.srcs.len()).filter(|i| sp.srcs[*i] == node).collect();
                        if at.len() != 1 {
                            return false;
                        }
                        if sp.srcs.len() == 1 {
                            return self.keyed(run, c);
                        }
                        let me = at[0];
                        sp.conj.iter().any(|e| match e {
                            E::Cmp(o, x, y) if o == "=" => matches!((&**x, &**y), (E::Col(a, _, _), E::Col(b, _, _)) if (*a == me) != (*b == me)),
                            _ => false,
                        })
                    }
                    "antijoin" if arg("right") => true,
                    "antijoin" | "union" | "extend" | "project" | "span-between" | "span-bounds" | "replace" | "capture" | "source-matches" | "fits-rendered" | "render-syntax" => self.keyed(run, c),
                    "axis" if !arg("relation") && arg("from") => self.keyed(run, c),
                    "axis" if !arg("relation") && arg("domain") => true,
                    _ => false,
                }
            });
        self.keyed_memo[node] = Some(k);
        k
    }

    /// Whether column `kc` of `node` is the tree's kind of the node in its column `ic` (through picks, equal-id joins,
    /// unions and steps that pass rows through): then a kind test on it may enter by the kind bucket.
    fn tree_kind(&mut self, run: &mut Run, node: usize, kc: usize, ic: usize) -> bool {
        if let Ok(Some(v)) = self.view(run, node) {
            return v.base == Native::Tree && v.map.get(kc) == Some(&1) && v.map.get(ic) == Some(&0);
        }
        let plan: &'p Plan = self.plan;
        let pn = &plan.nodes[node];
        let Ok(sc) = self.schema(run, node) else { return false };
        let (kn, inm) = (sc.names[kc].clone(), sc.names[ic].clone());
        let through = |me: &mut Self, run: &mut Run, arg: &str| -> bool {
            let Ok(n) = me.arg_rel(node, arg) else { return false };
            let Ok(isc) = me.schema(run, n) else { return false };
            match (isc.names.iter().position(|c| *c == kn), isc.names.iter().position(|c| *c == inm)) {
                (Some(a), Some(b)) => me.tree_kind(run, n, a, b),
                _ => false,
            }
        };
        match pn.op.as_str() {
            "select" => {
                let Ok(sp) = self.sel(run, node) else { return false };
                let (E::Col(sk, a, _), E::Col(si, b, _)) = (&sp.items[kc].1, &sp.items[ic].1) else { return false };
                let (sk, a, si, b) = (*sk, *a, *si, *b);
                if sk == si {
                    return self.tree_kind(run, sp.srcs[sk], a, b);
                }
                // the id from one source, the kind from another joined to it on the id
                let joined = sp.conj.iter().find_map(|e| match e {
                    E::Cmp(o, x, y) if o == "=" => match (&**x, &**y) {
                        (E::Col(s1, c1, _), E::Col(s2, c2, _)) if *s1 == si && *c1 == b && *s2 == sk => Some(*c2),
                        (E::Col(s2, c2, _), E::Col(s1, c1, _)) if *s1 == si && *c1 == b && *s2 == sk => Some(*c2),
                        _ => None,
                    },
                    _ => None,
                });
                joined.is_some_and(|c| self.tree_kind(run, sp.srcs[sk], a, c))
            }
            "union" => through(self, run, "left") && through(self, run, "right"),
            "antijoin" => through(self, run, "left"),
            "extend" | "span-between" | "span-bounds" | "replace" | "source-matches" | "axis" | "group-min" | "group-max" | "fits-rendered" | "render-syntax" => through(self, run, "from"),
            _ => false,
        }
    }

    /// Whether a step's rows are the same in every file: it reads only run-wide relations, through steps that read no file.
    fn is_constant(&mut self, node: usize) -> bool {
        if let Some(c) = self.constant[node] {
            return c;
        }
        let pn = &self.plan.nodes[node];
        let c = match pn.op.as_str() {
            "base" => pn.j["rel"].as_str() == Some("filesystem.file"),
            "rows" => true,
            "select" | "union" | "antijoin" | "extend" | "project" | "group-count" | "group-min" | "group-max" | "capture" | "replace" => {
                let ins = pn.inputs.clone();
                !ins.is_empty() && ins.iter().all(|i| self.is_constant(*i))
            }
            _ => false,
        };
        self.constant[node] = Some(c);
        c
    }

    /// Whether `node` reads `input`, directly or through other steps.
    fn depends(&mut self, node: usize, input: usize) -> bool {
        if node == input {
            return true;
        }
        if let Some(d) = self.deps.get(&(node, input)) {
            return *d;
        }
        let ins = self.plan.nodes[node].inputs.clone();
        let d = ins.iter().any(|i| self.depends(*i, input));
        self.deps.insert((node, input), d);
        d
    }

    fn arg_rel(&self, node: usize, name: &str) -> R<usize> {
        // 7: the step's relation arguments, read from the plan once (`PNode::rels`)
        self.plan.nodes[node].rels.iter().find(|(n, _)| n == name).map(|x| x.1).ok_or_else(|| format!("no relation argument {name}"))
    }

    // -------------------------------------------------------------------------------------------- static: schemas, views

    fn schema(&mut self, run: &mut Run, node: usize) -> R<Rc<Schema>> {
        if let Some(sc) = &self.schemas[node] {
            return Ok(sc.clone());
        }
        // 7: the plan's own JSON (never a copy: `strs` knows a plan value by its address)
        let plan: &'p Plan = self.plan;
        let pn = &plan.nodes[node];
        let j = &pn.j;
        let a = &j["args"];
        let op = pn.op.clone();
        let from = |me: &mut Self, run: &mut Run, name: &str| -> R<Rc<Schema>> {
            let n = me.arg_rel(node, name)?;
            me.schema(run, n)
        };
        let mk = |names: Vec<String>, tys: Vec<Ty>| Rc::new(Schema { names: Rc::new(names), tys: Rc::new(tys) });
        let sc = match op.as_str() {
            "base" => {
                let rel = s(&j["rel"])?;
                if let Some(nat) = native_of(rel) {
                    mk(native_cols(nat).iter().map(|c| c.0.to_owned()).collect(), native_cols(nat).iter().map(|c| c.1).collect())
                } else if rel == "filesystem.file" {
                    mk(vec!["path".into()], vec![Ty::File])
                } else {
                    let (_, names, tys) = run.catalog(rel).ok_or_else(|| format!("no relation {rel}"))?;
                    Rc::new(Schema { names, tys })
                }
            }
            "select" => {
                let sp = self.sel(run, node)?;
                mk(sp.items.iter().map(|i| i.0.clone()).collect(), sp.items.iter().map(|i| static_ty(&i.1)).collect())
            }
            "union" | "antijoin" | "group-min" | "group-max" => from(self, run, if op == "union" || op == "antijoin" { "left" } else { "from" })?,
            "extend" => {
                let f = from(self, run, "from")?;
                let (mut names, mut tys) = ((*f.names).clone(), (*f.tys).clone());
                for (k, v) in a["values"].as_object().ok_or("extend values")? {
                    names.push(k.clone());
                    tys.push(match v {
                        J::Number(_) => Ty::Num,
                        J::String(t) if t == "true" || t == "false" => Ty::Flag,
                        _ => Ty::Sym,
                    });
                }
                mk(names, tys)
            }
            "project" => {
                let f = from(self, run, "from")?;
                let cols: Vec<usize> = strs(&a["columns"])?.iter().map(|c| pos(&f.names, c)).collect::<R<_>>()?;
                mk(cols.iter().map(|c| f.names[*c].clone()).collect(), cols.iter().map(|c| f.tys[*c]).collect())
            }
            "axis" => {
                let f = from(self, run, "from")?;
                let (mut names, mut tys) = ((*f.names).clone(), (*f.tys).clone());
                names.push("related".into());
                tys.push(Ty::Node);
                mk(names, tys)
            }
            "group-count" => {
                let f = from(self, run, "from")?;
                let keys: Vec<usize> = strs(&a["keys"])?.iter().map(|c| pos(&f.names, c)).collect::<R<_>>()?;
                let mut names: Vec<String> = keys.iter().map(|c| f.names[*c].clone()).collect();
                let mut tys: Vec<Ty> = keys.iter().map(|c| f.tys[*c]).collect();
                names.push(s(&a["count"])?.to_owned());
                tys.push(Ty::Num);
                mk(names, tys)
            }
            "closure" | "follow" => {
                let e = from(self, run, "edges")?;
                let keys: Vec<usize> = strs(&a["keys"])?.iter().map(|c| pos(&e.names, c)).collect::<R<_>>()?;
                let (si, di) = (pos(&e.names, s(&a["source"])?)?, pos(&e.names, s(&a["destination"])?)?);
                let mut names: Vec<String> = keys.iter().map(|c| e.names[*c].clone()).collect();
                let mut tys: Vec<Ty> = keys.iter().map(|c| e.tys[*c]).collect();
                names.extend([e.names[si].clone(), e.names[di].clone()]);
                tys.extend([e.tys[si], e.tys[di]]);
                mk(names, tys)
            }
            "ordered-match" | "as-of" => {
                let (l, r) = (from(self, run, "left")?, from(self, run, "right")?);
                let mut names = Vec::new();
                let mut tys = Vec::new();
                for (name, v) in a["output"].as_object().ok_or("output")? {
                    let side = if s(&v[0])? == "left" { &l } else { &r };
                    names.push(name.clone());
                    tys.push(side.tys[pos(&side.names, s(&v[1])?)?]);
                }
                mk(names, tys)
            }
            "span-between" | "replace" => {
                let f = from(self, run, "from")?;
                let (mut names, mut tys) = ((*f.names).clone(), (*f.tys).clone());
                names.push(s(&a["output"])?.to_owned());
                tys.push(if op == "replace" { Ty::Sym } else { Ty::Span });
                mk(names, tys)
            }
            "span-bounds" => {
                let f = from(self, run, "from")?;
                let (mut names, mut tys) = ((*f.names).clone(), (*f.tys).clone());
                names.extend([s(&a["start-output"])?.to_owned(), s(&a["end-output"])?.to_owned()]);
                tys.extend([Ty::Num, Ty::Num]);
                mk(names, tys)
            }
            "source-matches" => {
                let f = from(self, run, "from")?;
                let (mut names, mut tys) = ((*f.names).clone(), (*f.tys).clone());
                names.push(s(&a["output"])?.to_owned());
                tys.push(Ty::Span);
                if let Some(lo) = a.get("line-output").and_then(J::as_str) {
                    names.push(lo.to_owned());
                    tys.push(Ty::Num);
                }
                if a.get("capture").and_then(J::as_str).is_some() {
                    names.push(s(&a["capture-output"])?.to_owned());
                    tys.push(Ty::Span);
                    if let Some(ct) = a.get("capture-text-output").and_then(J::as_str) {
                        names.push(ct.to_owned());
                        tys.push(Ty::Sym);
                    }
                    if let Some(cl) = a.get("capture-line-output").and_then(J::as_str) {
                        names.push(cl.to_owned());
                        tys.push(Ty::Num);
                    }
                }
                mk(names, tys)
            }
            "capture" => {
                let f = from(self, run, "from")?;
                let (mut names, mut tys) = ((*f.names).clone(), (*f.tys).clone());
                for c in strs(&a["captures"])?.iter().cloned() {
                    names.push(c);
                    tys.push(Ty::Sym);
                }
                mk(names, tys)
            }
            "rows" => {
                let names = strs(&a["columns"])?;
                let k = names.len();
                mk((*names).clone(), vec![Ty::Sym; k])
            }
            "render-syntax" => {
                let f = from(self, run, "from")?;
                let (mut names, mut tys) = ((*f.names).clone(), (*f.tys).clone());
                names.push(s(&a["output"])?.to_owned());
                tys.push(Ty::Sym);
                mk(names, tys)
            }
            "fits-rendered" => from(self, run, "from")?,
            "group-ordinal" => {
                let f = from(self, run, "from")?;
                let (mut names, mut tys) = ((*f.names).clone(), (*f.tys).clone());
                names.push(s(&a["output"])?.to_owned());
                tys.push(Ty::Num);
                mk(names, tys)
            }
            other => return Err(format!("operator {other} not in the step set")),
        };
        self.schemas[node] = Some(sc.clone());
        Ok(sc)
    }

    fn expr(&self, run: &mut Run, j: &J, names: &[(Option<String>, Rc<Schema>)]) -> R<E> {
        let a = j.as_array().ok_or("expr")?;
        let tag = s(&a[0])?;
        Ok(match tag {
            "col" => {
                let name = s(&a[1])?;
                let alias = a[2].as_str();
                let mut found = None;
                for (i, (al, sc)) in names.iter().enumerate() {
                    if alias.is_some() && al.as_deref() != alias {
                        continue;
                    }
                    if let Some(c) = sc.names.iter().position(|x| x == name) {
                        found = Some(E::Col(i, c, sc.tys[c]));
                        break;
                    }
                }
                match found {
                    Some(e) => e,
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
            "seed" => E::Seed(Box::new(self.expr(run, &a[1], names)?), a[2].as_u64().ok_or("seed set")? as usize),
            other => return Err(format!("expression {other}")),
        })
    }

    /// A select's compiled form, once a run.
    fn sel(&mut self, run: &mut Run, node: usize) -> R<Rc<SelPlan>> {
        if let Some(sp) = &self.sels[node] {
            return Ok(sp.clone());
        }
        let plan: &'p Plan = self.plan;
        let j = &plan.nodes[node].j;
        let mut srcs = Vec::new();
        let mut names = Vec::new();
        for sj in j["src"].as_array().ok_or("src")? {
            let n = sj[0].as_u64().ok_or("src node")? as usize;
            names.push((sj[1].as_str().map(str::to_owned), self.schema(run, n)?));
            srcs.push(n);
        }
        let mut conj = Vec::new();
        if !j["cond"].is_null() {
            let e = self.expr(run, &j["cond"], &names)?;
            flatten(e, &mut conj);
        }
        // a test of two file columns always holds: every relation here is the file's
        conj.retain(|c| !matches!(c, E::Cmp(o, a, b) if o == "=" && matches!(**a, E::Col(_, _, Ty::File)) && matches!(**b, E::Col(_, _, Ty::File))));
        let refs = conj.iter().map(refs_of).collect();
        let items: Vec<(String, E)> = match &j["items"] {
            J::Null => {
                let mut v: Vec<(String, E)> = Vec::new();
                for (i, (_, sc)) in names.iter().enumerate() {
                    for (c, nm) in sc.names.iter().enumerate() {
                        if !v.iter().any(|(x, _)| x == nm) {
                            v.push((nm.clone(), E::Col(i, c, sc.tys[c])));
                        }
                    }
                }
                v
            }
            J::Array(a) => a.iter().map(|it| Ok((s(&it[0])?.to_owned(), self.expr(run, &it[1], &names)?))).collect::<R<_>>()?,
            _ => return Err("items".to_owned()),
        };
        let sp = Rc::new(SelPlan { srcs, conj, refs, items });
        self.sels[node] = Some(sp.clone());
        Ok(sp)
    }

    /// The view a node is, when it is one: a tree relation, or a select of column picks and tests over one view.
    #[inline]
    fn view(&mut self, run: &mut Run, node: usize) -> R<Option<Rc<ViewSpec>>> {
        match &self.views[node] {
            Some(None) => Ok(None),
            Some(v) => Ok(v.clone()),
            None => self.view_slow(run, node),
        }
    }

    #[inline(never)]
    fn view_slow(&mut self, run: &mut Run, node: usize) -> R<Option<Rc<ViewSpec>>> {
        let pn = &self.plan.nodes[node];
        let v = match pn.op.as_str() {
            "base" => {
                let rel = pn.j["rel"].as_str().unwrap_or("").to_owned();
                match native_of(&rel) {
                    Some(nat) => Some(Rc::new(ViewSpec { base: nat, map: (0..native_cols(nat).len()).collect(), filter: Vec::new() })),
                    None => fact_of(run, &rel).map(|nat| {
                        let Native::Fact(i) = nat else { unreachable!() };
                        Rc::new(ViewSpec { base: nat, map: (0..run.fact_rels[usize::from(i)].1).collect(), filter: Vec::new() })
                    }),
                }
            }
            "select" if pn.j["src"].as_array().is_some_and(|a| a.len() == 1) => {
                let inner = pn.j["src"][0][0].as_u64().unwrap_or(0) as usize;
                match self.view(run, inner)? {
                    Some(iv) => {
                        let sp = self.sel(run, node)?;
                        // 6e: `VEC_SEED_NOT_VIEW`: a seed test is never a view's filter (measured a loss: j13 239.6 M against j12 212.0 M)
                        let plain = sp.items.iter().all(|(_, e)| matches!(e, E::Col(0, _, _))) && (!sw!("VEC_SEED_NOT_VIEW") || !sp.conj.iter().any(|e| matches!(e, E::Seed(..))));
                        if plain {
                            let map = sp.items.iter().map(|(_, e)| if let E::Col(_, c, _) = e { iv.map[*c] } else { 0 }).collect();
                            let mut filter = iv.filter.clone();
                            filter.extend(sp.conj.iter().map(|e| remap(e, &iv.map)));
                            Some(Rc::new(ViewSpec { base: iv.base, map, filter }))
                        } else {
                            None
                        }
                    }
                    None => None,
                }
            }
            _ => None,
        };
        self.views[node] = Some(v.clone());
        Ok(v)
    }

    // -------------------------------------------------------------------------------------------- per file

    /// A node's answer in this file, made once; an input is asked only when the step needs it.
    pub(super) fn res(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize) -> R<Res> {
        let why = std::mem::take(&mut self.reason);
        if let Some(r) = &f.memo[node] {
            return Ok(r.clone());
        }
        let tk = step_clock();
        if let Some(v) = self.view(run, node)? {
            self.load_fact(f, run, p, v.base)?;
            let r = Res::View(v);
            f.memo[node] = Some(r.clone());
            return Ok(r);
        }
        if let Some(t) = &self.run_memo[node] {
            let r = Res::Tab(t.clone());
            f.memo[node] = Some(r.clone());
            return Ok(r);
        }
        let decided = f.known[node] != 0;
        if !sw!("VEC_NO_KNOWN") && self.known_empty(f, run, node) {
            self.stats.known[node] += 1;
            let r = Res::Tab(self.empty_rc(run, node)?);
            f.memo[node] = Some(r.clone());
            f.log.push(node);
            if sw!("VEC_STEP_TIME") && decided {
                self.stats.known_time += step_clock() - tk;
                self.stats.known_timed += 1;
            }
            return Ok(r);
        }
        let ew = if sw!("VEC_EMPTY_WHY") { Some(empty_why::class(self, f, run, node)) } else { None };
        let t0 = step_clock();
        let saved = self.inner;
        self.inner = 0.0;
        if self.stats.made_by[node] == usize::MAX {
            self.stats.made_by[node] = self.stack.last().copied().unwrap_or(usize::MAX - 1);
            self.stats.why[node] = why;
        }
        self.stack.push(node);
        let out = self.body(f, run, p, node);
        self.stack.pop();
        let dt = step_clock() - t0;
        let own = dt - self.inner;
        self.stats.time[node] += own;
        self.inner = saved + dt;
        let out = out?;
        if out.n == 0 {
            self.stats.empty_time[node] += own;
        }
        if let Some(c) = ew {
            empty_why::tally(self, f, node, c, out.n == 0);
        }
        self.stats.calls[node] += 1;
        self.stats.rows[node] += out.n as u64;
        if out.n == 0 {
            self.stats.empty[node] += 1;
        }
        let t = out;
        if self.is_constant(node) {
            self.run_memo[node] = Some(t.clone());
        }
        let rc = Res::Tab(t);
        f.memo[node] = Some(rc.clone());
        f.log.push(node);
        Ok(rc)
    }

    /// The rows of a node whose column `col` holds one of `keys`, without making the node whole where its inputs can be
    /// probed in turn (sideways key passing through the steps a query defines). What is made under a probe is dropped.
    fn probe(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize, col: usize, keys: &[u64]) -> R<Rc<Table>> {
        if keys.is_empty() {
            return self.empty_rc(run, node);
        }
        let sc = self.schema(run, node)?;
        if let Some(o) = self.overridden(node) {
            return Ok(Rc::new(drive::subset_of(&o, col, keys)));
        }
        if let Some(Res::Tab(t)) = &f.memo[node] {
            let t = t.clone();
            return Ok(Rc::new(subset(f, node, &t, col, keys)));
        }
        if !sw!("VEC_NO_KNOWN") && self.known_empty(f, run, node) {
            self.stats.known[node] += 1;
            return self.empty_rc(run, node);
        }
        if let Some(v) = self.view(run, node)? {
            self.load_fact(f, run, p, v.base)?;
            if File::probes(v.base, v.map[col]) {
                let mut rows = Vec::new();
                for k in keys {
                    f.native_probe(run, v.base, v.map[col], *k, &mut rows);
                }
                rows.retain(|r| view_holds(f, run, &v, *r));
                let mut out = Table::with(sc.names.clone(), sc.tys.clone());
                for c in 0..v.map.len() {
                    out.cols[c] = rows.iter().map(|r| f.native_get(run, v.base, *r, v.map[c])).collect();
                }
                out.n = rows.len();
                self.stats.touched[node] += rows.len() as u64;
                return Ok(Rc::new(out));
            }
            self.reason = "view probed on a column it has no keyed read for";
            let t = self.eval(f, run, p, node)?;
            return Ok(Rc::new(subset(f, node, &t, col, keys)));
        }
        // the per-key memo (P1): a (step, column, key) is computed once a file, whichever consumer asks. 7: each key's rows
        // are one contiguous range of the slot's table (no list a key); a key asked is marked pending at once, so the
        // missing keys come out distinct without a set; a stamp a call dedupes the keys answered.
        let pm = !sw!("VEC_NO_PMEMO");
        let mut missing: Vec<u64> = std::mem::take(&mut self.scratch_keys);
        missing.clear();
        let mut list_hash = 0u64;
        let slot = if pm {
            let slot = self.memo_slot(node, col, &sc);
            if self.pmemo[slot].direct {
                self.scratch_keys = missing;
                return self.probe_direct(f, run, p, node, col, keys, &sc);
            }
            // 7: a key list asked before of this slot is answered by the table made for it then (by reference, no copy)
            if keys.len() >= LIST_MIN {
                list_hash = hash_keys(keys);
                if let Some((ks, t)) = self.pmemo[slot].lists.get(&list_hash)
                    && ks[..] == keys[..]
                {
                    let t = t.clone();
                    self.pmemo[slot].list_hits += 1;
                    self.stats.memo_reused += 1;
                    self.stats.memo_reused_keys += keys.len() as u64;
                    self.scratch_keys = missing;
                    return Ok(t);
                }
            }
            self.stats.memo_keys += keys.len() as u64;
            let ix = &mut self.pmemo[slot].ix;
            for k in keys {
                if let std::collections::hash_map::Entry::Vacant(v) = ix.entry(*k) {
                    v.insert(MemoKey { start: PENDING, len: 0, stamp: 0 });
                    missing.push(*k);
                }
            }
            self.stats.memo_missing += missing.len() as u64;
            {
                let sl = &mut self.pmemo[slot];
                sl.asked += keys.len() as u64;
                sl.missed += missing.len() as u64;
                sl.tried += 1;
                if sl.tried >= MEMO_TRIAL && sl.list_hits == 0 && sl.missed * 100 >= sl.asked * 97 && !sw!("VEC_MEMO_ALWAYS") {
                    sl.direct = true;
                }
            }
            if missing.is_empty() {
                self.stats.memo_whole_hits += 1;
                self.scratch_keys = missing;
                let t = self.memo_rows(node, slot, keys);
                self.stats.memo_out += t.n as u64;
                if keys.len() >= LIST_MIN {
                    self.pmemo[slot].lists.insert(list_hash, (keys.to_vec(), t.clone()));
                }
                return Ok(t);
            }
            Some(slot)
        } else {
            missing.extend_from_slice(keys);
            None
        };
        let t0 = step_clock();
        let saved = self.inner;
        self.inner = 0.0;
        self.stack.push(node);
        let out = self.probe_body(f, run, p, node, col, &missing, &sc);
        self.stack.pop();
        let dt = step_clock() - t0;
        let own = dt - self.inner;
        self.stats.time[node] += own;
        self.inner = saved + dt;
        let out = match out {
            Ok(o) => o,
            Err(e) => {
                if let Some(slot) = slot {
                    for k in &missing {
                        self.pmemo[slot].ix.remove(k);
                    }
                }
                self.scratch_keys = missing;
                return Err(e);
            }
        };
        static TRACE: std::sync::OnceLock<Option<usize>> = std::sync::OnceLock::new();
        if *TRACE.get_or_init(|| std::env::var("VEC_TRACE_PROBE").ok().and_then(|x| x.parse::<usize>().ok())) == Some(node) {
            eprintln!("probe {node} col {col} keys {} rows {} stack {:?}", missing.len(), out.n, &self.stack[self.stack.len().saturating_sub(6)..]);
        }
        self.count_probe(node, own, out.n);
        let Some(slot) = slot else {
            self.scratch_keys = missing;
            return Ok(out);
        };
        for k in &missing {
            if let Some(e) = self.pmemo[slot].ix.get_mut(k) {
                e.start = 0;
            }
        }
        // the batch's rows by key (in row order within a key), each key's run appended as one range
        let mut order = std::mem::take(&mut self.scratch_order);
        order.clear();
        order.extend((0..out.n as u32).map(|r| (out.cols[col][r as usize], r)));
        order.sort_unstable();
        let sl = &mut self.pmemo[slot];
        let mut i = 0;
        while i < order.len() {
            let k = order[i].0;
            let mut j = i;
            while j < order.len() && order[j].0 == k {
                j += 1;
            }
            let start = sl.t.n as u32;
            let e = sl.ix.entry(k).or_insert(MemoKey { start, len: 0, stamp: 0 });
            if e.len > 0 {
                sl.lists.clear();
                // rows for a key answered before (a step that returned rows not asked): its old range moves to the end first
                let (os, ol) = (e.start as usize, e.len as usize);
                for c in 0..sl.t.cols.len() {
                    let col_c = &mut sl.t.cols[c];
                    col_c.extend_from_within(os..os + ol);
                }
                sl.t.n += ol;
            }
            let e = sl.ix.get_mut(&k).expect("the key just filed");
            let ol = e.len;
            e.start = start;
            e.len = ol + (j - i) as u32;
            for c in 0..sl.t.cols.len() {
                let (dst, src) = (&mut sl.t.cols[c], &out.cols[c]);
                dst.extend(order[i..j].iter().map(|x| src[x.1 as usize]));
            }
            sl.t.n += j - i;
            i = j;
        }
        self.scratch_order = order;
        self.stats.memo_in += out.n as u64;
        let t = self.memo_rows(node, slot, keys);
        self.stats.memo_out += t.n as u64;
        if keys.len() >= LIST_MIN {
            self.pmemo[slot].lists.insert(list_hash, (keys.to_vec(), t.clone()));
        }
        self.scratch_keys = missing;
        Ok(t)
    }

    /// pass 2: a probe run counted: its rows, and when it returned nothing, its own seconds (timing on).
    #[inline]
    pub(super) fn count_probe(&mut self, node: usize, own: f64, n: usize) {
        self.stats.probes[node] += 1;
        self.stats.rows[node] += n as u64;
        if n == 0 {
            self.stats.probe_empty[node] += 1;
            self.stats.empty_time[node] += own;
        }
    }

    /// pass 2: a probe of a slot whose memo does not answer: the step's body by the distinct keys, timed as its probe,
    /// and its rows of keys asked only (what the memo would hand on; no filing).
    #[expect(clippy::too_many_arguments, reason = "a throwaway")]
    fn probe_direct(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize, col: usize, keys: &[u64], sc: &Schema) -> R<Rc<Table>> {
        let mut ks = kernel_input::take_keys();
        let mut set = std::mem::take(&mut self.scratch_set);
        set.clear();
        let small = keys.len() <= 16;
        if small {
            for k in keys {
                if !ks.contains(k) {
                    ks.push(*k);
                }
            }
        } else {
            if set.capacity() > 8 * keys.len() && set.capacity() > 4096 {
                set = rustc_hash::FxHashSet::default();
            }
            ks.extend(keys.iter().copied().filter(|k| set.insert(*k)));
        }
        let t0 = step_clock();
        let saved = self.inner;
        self.inner = 0.0;
        self.stack.push(node);
        let out = self.probe_body(f, run, p, node, col, &ks, sc);
        self.stack.pop();
        let dt = step_clock() - t0;
        let own = dt - self.inner;
        self.stats.time[node] += own;
        self.inner = saved + dt;
        let out = match out {
            Ok(o) => o,
            Err(e) => {
                kernel_input::give_keys(ks);
                self.scratch_set = set;
                return Err(e);
            }
        };
        self.count_probe(node, own, out.n);
        let asked = |k: &u64| if small { ks.contains(k) } else { set.contains(k) };
        let out = if out.cols[col].iter().all(asked) {
            out
        } else {
            let keep: Vec<usize> = (0..out.n).filter(|r| asked(&out.cols[col][*r])).collect();
            if keep.is_empty() {
                self.empty_rc(run, node)?
            } else {
                let cols = out.cols.iter().map(|c| keep.iter().map(|r| c[*r]).collect()).collect();
                Rc::new(Table { names: out.names.clone(), tys: out.tys.clone(), cols, n: keep.len() })
            }
        };
        kernel_input::give_keys(ks);
        self.scratch_set = set;
        Ok(out)
    }

    /// 7: the memo slot of a (step, column) for this file (its buffers kept across files).
    fn memo_slot(&mut self, node: usize, col: usize, sc: &Schema) -> usize {
        if let Some(&(_, i)) = self.pmemo_at[node].iter().find(|x| x.0 as usize == col) {
            let i = i as usize;
            if !self.pmemo[i].dirty {
                self.pmemo[i].dirty = true;
                self.pmemo_dirty.push(i);
            }
            return i;
        }
        let i = self.pmemo.len();
        self.pmemo.push(MemoSlot { t: Table::with(sc.names.clone(), sc.tys.clone()), ix: FxHashMap::default(), lists: FxHashMap::default(), dirty: true, asked: 0, missed: 0, tried: 0, list_hits: 0, direct: false });
        self.pmemo_at[node].push((col as u32, i as u32));
        self.pmemo_dirty.push(i);
        i
    }

    /// 7: the memoised rows of a probed step for these keys, in key order (each key once).
    fn memo_rows(&mut self, node: usize, slot: usize, keys: &[u64]) -> Rc<Table> {
        self.stamp = self.stamp.wrapping_add(1);
        let st = self.stamp;
        let mut ranges = std::mem::take(&mut self.scratch_ranges);
        ranges.clear();
        let sl = &mut self.pmemo[slot];
        let mut total = 0;
        for k in keys {
            if let Some(e) = sl.ix.get_mut(k)
                && e.stamp != st
            {
                e.stamp = st;
                if e.len > 0 {
                    ranges.push((e.start as usize, e.len as usize));
                    total += e.len as usize;
                }
            }
        }
        if total == 0 {
            self.scratch_ranges = ranges;
            // pass 2: no rows: the step's shared empty answer
            if let Some(t) = &self.empties[node] {
                return t.clone();
            }
            let t = Rc::new(Table::with(sl.t.names.clone(), sl.t.tys.clone()));
            self.empties[node] = Some(t.clone());
            return t;
        }
        let mut out = Table::with(sl.t.names.clone(), sl.t.tys.clone());
        for (c, col) in sl.t.cols.iter().enumerate() {
            let mut v = Vec::with_capacity(total);
            for (a, l) in &ranges {
                v.extend_from_slice(&col[*a..*a + *l]);
            }
            out.cols[c] = v;
        }
        out.n = total;
        self.scratch_ranges = ranges;
        Rc::new(out)
    }

    #[expect(clippy::too_many_arguments, reason = "a throwaway")]
    fn probe_body(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize, col: usize, keys: &[u64], sc: &Schema) -> R<Rc<Table>> {
        // pass 2: the target resolved once a run by (step, column); the substitution with reused buffers
        if self.static_keyed && !sw!("VEC_PTARGET_OFF") {
            let (input, icol) = self.ptarget(run, node, col)?;
            match input {
                frame::GENERAL => {}
                frame::WHOLE_CARRIED | frame::WHOLE_UNCARRIED => {
                    if self.frames[node].semi {
                        let _ = self.semi_drives_whole(f, run, p, node, keys.len())?;
                    }
                    self.reason = if input == frame::WHOLE_CARRIED { "probed on a column its input shares with another input" } else { "probed on a column no input carries" };
                    let t = self.eval(f, run, p, node)?;
                    return Ok(Rc::new(subset(f, node, &t, col, keys)));
                }
                _ => {
                    if self.frames[node].semi && self.semi_drives_whole(f, run, p, node, keys.len())? {
                        self.reason = "probed on a column no input carries";
                        let t = self.eval(f, run, p, node)?;
                        return Ok(Rc::new(subset(f, node, &t, col, keys)));
                    }
                    return self.substituted(f, run, p, node, input as usize, icol as usize, keys);
                }
            }
        }
        let plan: &'p Plan = self.plan;
        let op: &'p String = &plan.nodes[node].op;
        let a = &plan.nodes[node].j["args"];
        let name: &String = &sc.names[col];
        // the input a unary step reads, when the probed column is one of its input's
        let unary = |me: &mut Self, run: &mut Run, arg: &str| -> R<Option<(usize, usize)>> {
            let n = me.arg_rel(node, arg)?;
            let isc = me.schema(run, n)?;
            Ok(isc.names.iter().position(|c| c == name).map(|c| (n, c)))
        };
        if op == "closure"
            && !f.lazies.contains_key(&node)
            && !sw!("VEC_NO_WALK")
            && let Some(t) = self.walk_probed(f, run, p, node, col, keys, sc)?
        {
            return Ok(Rc::new(t));
        }
        let target = match op.as_str() {
            "select" if self.semi_drives_whole(f, run, p, node, keys.len())? => None,
            "select" => {
                let sp = self.sel(run, node)?;
                match &sp.items[col].1 {
                    E::Col(si, c, _) if sp.srcs.iter().filter(|x| **x == sp.srcs[*si]).count() == 1 => Some((sp.srcs[*si], *c)),
                    _ => None,
                }
            }
            "union" if !sw!("VEC_OLD_UNION") => return union_kernel::probe(self, f, run, p, node, col, keys),
            "antijoin" if self.static_keyed && !sw!("VEC_OLD_ANTIJOIN") => return antijoin_kernel::probe(self, f, run, p, node, col, keys),
            "union" => {
                let (ln, rn) = (self.arg_rel(node, "left")?, self.arg_rel(node, "right")?);
                let rsc = self.schema(run, rn)?;
                let rc = pos(&rsc.names, &name)?;
                let l = self.probe(f, run, p, ln, col, keys)?;
                let r = self.probe(f, run, p, rn, rc, keys)?;
                let mut out = (*l).clone();
                for (c, nm) in l.names.iter().enumerate() {
                    out.cols[c].extend_from_slice(&r.cols[pos(&r.names, nm)?]);
                }
                out.n += r.n;
                return Ok(Rc::new(distinct(out)));
            }
            "antijoin" => unary(self, run, "left")?,
            "as-of" | "ordered-match" if !self.static_keyed => {
                // as-of is row-wise on its left; ordered-match pairs within a key group, so only a left key carries
                let m = &a["output"][name.as_str()];
                match (m[0].as_str(), m[1].as_str()) {
                    (Some("left"), Some(lc)) if op == "as-of" || strs(&a["left-on"])?.iter().any(|k| k == lc) => {
                        let ln = self.arg_rel(node, "left")?;
                        let lsc = self.schema(run, ln)?;
                        Some((ln, pos(&lsc.names, lc)?))
                    }
                    _ => None,
                }
            }
            "extend" | "project" | "span-between" | "span-bounds" | "source-matches" | "replace" | "capture" => unary(self, run, "from")?,
            "axis" => {
                if name == "related" {
                    if let Some(t) = self.axis_related(f, run, p, node, keys)? {
                        return Ok(Rc::new(t));
                    }
                    None
                } else {
                    let fnode = self.arg_rel(node, "from")?;
                    let fsc = self.schema(run, fnode)?;
                    let icol = pos(&fsc.names, &name)?;
                    let sub = self.probe(f, run, p, fnode, icol, keys)?;
                    return Ok(Rc::new(self.axis_over(f, run, p, node, &a, &sub)?));
                }
            }
            "group-count" | "group-min" | "group-max" => {
                let ks = strs(&a["keys"])?;
                if ks.contains(&name) { unary(self, run, "from")? } else { None }
            }
            "closure" => {
                let l = self.lazy(f, run, p, node)?;
                let nk = l.keys.len();
                if col >= nk && nk == 0 {
                    let forward = col == 0;
                    let mut out = Table::with(sc.names.clone(), sc.tys.clone());
                    for k in keys {
                        for d in reach_dir(&l, &[*k], forward).iter() {
                            out.push_row(&if forward { [*k, *d] } else { [*d, *k] });
                        }
                    }
                    return Ok(Rc::new(out));
                }
                if col >= nk && l.tys[..nk].iter().all(|t| *t == Ty::File) {
                    let forward = col == nk;
                    let mut out = Table::with(sc.names.clone(), sc.tys.clone());
                    for k in keys {
                        let mut key: Vec<u64> = vec![u64::from(f.file); nk];
                        key.push(*k);
                        for d in reach_dir(&l, &key, forward).iter() {
                            let mut row = vec![u64::from(f.file); nk];
                            row.extend(if forward { [*k, *d] } else { [*d, *k] });
                            out.push_row(&row);
                        }
                    }
                    return Ok(Rc::new(out));
                }
                None
            }
            _ => None,
        };
        // the input may be put in place only where no other input of the step reads it (6c); with the override (6e) only
        // where the step reads it once
        let carried = target.is_some();
        if !self.static_keyed {
            // the probed source's position, where a select reads the same step at several (a self-join)
            let at = if op == "select" {
                match &self.sel(run, node)?.items[col].1 {
                    E::Col(si, c, _) => Some((*si, *c)),
                    _ => None,
                }
            } else {
                None
            };
            let at_pos = at.map(|x| x.0);
            let target = match (at, target) {
                (Some((si, c)), None) if op == "select" => Some((self.sel(run, node)?.srcs[si], c)),
                (_, t) => t,
            };
            let target = target.filter(|(input, _)| at_pos.is_some() || matches!(op.as_str(), "antijoin" | "ordered-match" | "as-of") || self.plan.nodes[node].inputs.iter().filter(|i| *i == input).count() == 1);
            if target.is_none()
                && let Some(t) = self.probe_by_group(f, run, p, node, col, keys)?
            {
                return Ok(Rc::new(t));
            }
            let Some((input, icol)) = target else {
                self.reason = if carried { "probed on a column of an input it reads twice" } else { "probed on a column no input carries" };
                let t = self.eval(f, run, p, node)?;
                return Ok(Rc::new(subset(f, node, &t, col, keys)));
            };
            let sub = self.probe(f, run, p, input, icol, keys)?;
            return self.run_over(f, run, p, node, input, at_pos, sub);
        }
        let target = target.filter(|(input, _)| {
            let others: Vec<usize> = self.plan.nodes[node].inputs.iter().copied().filter(|i| i != input).collect();
            self.plan.nodes[node].inputs.iter().filter(|i| *i == input).count() == 1 && others.iter().all(|o| !self.depends(*o, *input))
        });
        let Some((input, icol)) = target else {
            self.reason = if carried { "probed on a column its input shares with another input" } else { "probed on a column no input carries" };
            let t = self.eval(f, run, p, node)?;
            return Ok(Rc::new(subset(f, node, &t, col, keys)));
        };
        // the input probed, put in its place while the step runs once over it; everything made meanwhile is dropped
        let sub = self.probe(f, run, p, input, icol, keys)?;
        // the input has no rows for these keys: the step over it answers nothing (its body would return at once; a
        // counted group count reads its select's sources, not the input, and runs)
        if sub.n == 0 && !self.frames[node].counted {
            self.stats.known[node] += 1;
            return self.empty_rc(run, node);
        }
        let saved = f.memo[input].replace(Res::Tab(sub));
        let saved_ix = f.index.remove(&input);
        let mark = f.log.len();
        let out = self.body(f, run, p, node);
        f.memo[input] = saved;
        f.index.remove(&input);
        if let Some(ix) = saved_ix {
            f.index.insert(input, ix);
        }
        let made: Vec<usize> = f.log.drain(mark..).collect();
        let mut dropped: rustc_hash::FxHashSet<usize> = rustc_hash::FxHashSet::default();
        dropped.insert(input);
        for n in made {
            if self.depends(n, input) {
                dropped.insert(n);
            } else {
                f.log.push(n);
            }
        }
        for n in &dropped {
            if *n != input {
                f.memo[*n] = None;
            }
            f.vmat.remove(n);
            f.lazies.remove(n);
        }
        for n in &dropped {
            if *n != input {
                f.index.remove(n);
            }
        }
        // no other input of the step reads the substituted one (checked above), so no probe memo is made over it but this
        // step's own, which is filed under the keys it was asked
        out
    }

    /// pass 2: the step run once over its input probed by `keys`, put in its place; what was made meanwhile over it is
    /// dropped (the same as the general path below, with reused buffers and no set).
    #[expect(clippy::too_many_arguments, reason = "a throwaway")]
    fn substituted(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize, input: usize, icol: usize, keys: &[u64]) -> R<Rc<Table>> {
        let sub = self.probe(f, run, p, input, icol, keys)?;
        // the input has no rows for these keys: the step over it answers nothing (its body would return at once; a
        // counted group count reads its select's sources, not the input, and runs)
        if sub.n == 0 && !self.frames[node].counted {
            self.stats.known[node] += 1;
            return self.empty_rc(run, node);
        }
        let saved = f.memo[input].replace(Res::Tab(sub));
        let saved_ix = if f.index.is_empty() { None } else { f.index.remove(&input) };
        let mark = f.log.len();
        let out = self.body(f, run, p, node);
        f.memo[input] = saved;
        if let Some(ix) = saved_ix {
            f.index.insert(input, ix);
        } else if !f.index.is_empty() {
            f.index.remove(&input);
        }
        if f.log.len() > mark {
            let mut made = std::mem::take(&mut self.scratch_made);
            made.clear();
            made.extend(f.log.drain(mark..));
            for &n in &made {
                if n == input {
                    continue;
                }
                if self.depends(n, input) {
                    f.memo[n] = None;
                    f.vmat.remove(&n);
                    f.lazies.remove(&n);
                    f.index.remove(&n);
                } else {
                    f.log.push(n);
                }
            }
            self.scratch_made = made;
        }
        if !f.vmat.is_empty() {
            f.vmat.remove(&input);
        }
        if !f.lazies.is_empty() {
            f.lazies.remove(&input);
        }
        out
    }

    /// A node's rows as a table (a view is made whole here, once a file).
    pub(super) fn eval(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize) -> R<Rc<Table>> {
        let why = self.reason;
        if let Some(t) = self.overridden(node) {
            return Ok(t);
        }
        match self.res(f, run, p, node)? {
            Res::Tab(t) => Ok(t),
            Res::View(v) => {
                if let Some(t) = f.vmat.get(&node) {
                    return Ok(t.clone());
                }
                let t0 = step_clock();
                let sc = self.schema(run, node)?;
                let rows = view_rows(f, run, &v, node);
                let mut out = Table::with(sc.names.clone(), sc.tys.clone());
                for c in 0..v.map.len() {
                    out.cols[c] = rows.iter().map(|r| f.native_get(run, v.base, *r, v.map[c])).collect();
                }
                out.n = rows.len();
                let rc = Rc::new(out);
                if self.stats.made_by[node] == usize::MAX {
                    self.stats.made_by[node] = self.stack.last().copied().unwrap_or(usize::MAX - 1);
                    self.stats.why[node] = if why.is_empty() { "a view read whole" } else { why };
                }
                f.vmat.insert(node, rc.clone());
                f.log.push(node);
                let dt = step_clock() - t0;
                self.stats.time[node] += dt;
                self.inner += dt;
                self.stats.calls[node] += 1;
                self.stats.rows[node] += rc.n as u64;
                Ok(rc)
            }
        }
    }

    /// The step's shared empty answer (made once a run).
    #[inline]
    pub(super) fn empty_rc(&mut self, run: &mut Run, node: usize) -> R<Rc<Table>> {
        if let Some(t) = &self.empties[node] {
            return Ok(t.clone());
        }
        let sc = self.schema(run, node)?;
        let t = Rc::new(Table::with(sc.names.clone(), sc.tys.clone()));
        self.empties[node] = Some(t.clone());
        Ok(t)
    }

    fn empty(&mut self, run: &mut Run, node: usize) -> R<Table> {
        let sc = self.schema(run, node)?;
        Ok(Table::with(sc.names.clone(), sc.tys.clone()))
    }

    fn lazy(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize) -> R<Rc<Lazy>> {
        if let Some(l) = f.lazies.get(&node) {
            return Ok(l.clone());
        }
        self.reason = "closure edges";
        let edges = self.eval(f, run, p, self.arg_rel(node, "edges")?)?;
        let t0 = step_clock();
        let a = &self.plan.nodes[node].j["args"];
        let keys: Vec<usize> = strs(&a["keys"])?.iter().map(|k| edges.col(k)).collect::<R<_>>()?;
        let (si, di) = (edges.col(s(&a["source"])?)?, edges.col(s(&a["destination"])?)?);
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
        let sc = self.schema(run, node)?;
        let l = Rc::new(Lazy { keys, adj, radj, node, names: sc.names.clone(), tys: sc.tys.clone(), cache: RefCell::new(FxHashMap::default()), rcache: RefCell::new(FxHashMap::default()) });
        let dt = step_clock() - t0;
        self.stats.time[node] += dt;
        self.inner += dt;
        f.lazies.insert(node, l.clone());
        f.log.push(node);
        Ok(l)
    }

    /// A step's run: the kernels' dispatch, then what ends the step before anything else (the frame, prepared at plan
    /// load), then the operator's body. redo-union pass 2: the answer is shared (`Rc`): a kernel's passed-through input
    /// and the step's empty answer are handed on, not copied.
    fn body(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize) -> R<Rc<Table>> {
        let fr = self.frames[node];
        match fr.op {
            frame::Op::Union if !sw!("VEC_OLD_UNION") => return union_kernel::whole(self, f, run, p, node),
            frame::Op::Antijoin if !sw!("VEC_OLD_ANTIJOIN") => return antijoin_kernel::whole(self, f, run, p, node),
            _ => {}
        }
        if fr.counted
            && let Some(t) = self.counted(f, run, p, node)?
        {
            return Ok(Rc::new(t));
        }
        // a step over an empty input answers nothing, and asks for nothing else
        if fr.first == frame::MISSING {
            return Err(format!("no relation argument of {}", self.plan.nodes[node].op));
        }
        if fr.first != NONE {
            let t = self.eval(f, run, p, fr.first as usize)?;
            if t.n == 0 && fr.op != frame::Op::Union {
                return self.empty_rc(run, node);
            }
        }
        // an empty answer is the step's shared one (the body's own table is dropped, nothing kept a run)
        let t = self.step(f, run, p, node)?;
        if t.n == 0 { self.empty_rc(run, node) } else { Ok(Rc::new(t)) }
    }

    /// A group count over a counted select: its kept rows and their counts, grouped (None: the band does not apply).
    fn counted(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize) -> R<Option<Table>> {
        let plan: &'p Plan = self.plan;
        let a = &plan.nodes[node].j["args"];
        {
            let fnode = self.arg_rel(node, "from")?;
            let pb = plan.nodes[fnode].j["semi"].as_u64().ok_or("count without semi")? as usize;
            if let Some((from, counts)) = self.band_rows(f, run, p, fnode, pb, true)? {
                self.stats.rows[fnode] += from.n as u64;
                let keys: Vec<usize> = strs(&a["keys"])?.iter().map(|c| from.col(c)).collect::<R<_>>()?;
                let mut groups: FxHashMap<Vec<u64>, u64> = FxHashMap::default();
                let mut order: Vec<Vec<u64>> = Vec::new();
                for i in 0..from.n {
                    let k: Vec<u64> = keys.iter().map(|c| from.cols[*c][i]).collect();
                    let e = groups.entry(k.clone()).or_insert_with(|| {
                        order.push(k);
                        0
                    });
                    *e += counts[i];
                }
                let mut out = self.empty(run, node)?;
                for k in order {
                    let mut row = k.clone();
                    row.push(groups[&k]);
                    out.push_row(&row);
                }
                return Ok(Some(out));
            }
        }
        Ok(None)
    }

    /// The operator's body (its first input is not empty).
    fn step(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize) -> R<Table> {
        let plan: &'p Plan = self.plan;
        let pn = &plan.nodes[node];
        let op = pn.op.as_str();
        let j = &pn.j;
        let a = self.args[node];
        match op {
            "base" => self.base(f, run, p, s(&j["rel"])?),
            "select" => self.select(f, run, p, node),
            "union" => {
                let l = self.eval(f, run, p, self.arg_rel(node, "left")?)?;
                let r = self.eval(f, run, p, self.arg_rel(node, "right")?)?;
                let mut out = (*l).clone();
                let map: Vec<usize> = l.names.iter().map(|c| r.col(c)).collect::<R<_>>()?;
                for (c, m) in map.iter().enumerate() {
                    out.cols[c].extend_from_slice(&r.cols[*m]);
                }
                out.n += r.n;
                Ok(distinct(out))
            }
            "antijoin" => self.antijoin(f, run, p, node, a),
            "extend" => {
                let from = self.eval(f, run, p, (self.frames[node].first as usize))?;
                let mut out = (*from).clone();
                for (k, v) in a["values"].as_object().ok_or("extend values")? {
                    let (t, x) = match v {
                        J::Number(n) => (Ty::Num, n.as_i64().ok_or("extend number")? as u64),
                        J::String(t) if t == "true" || t == "false" => (Ty::Flag, u64::from(t == "true")),
                        J::String(t) => (Ty::Sym, u64::from(run.sym(t.as_bytes()))),
                        _ => return Err(format!("extend value {v}")),
                    };
                    out.cols.push(vec![x; from.n]);
                }
                let sc = self.schema(run, node)?;
                (out.names, out.tys) = (sc.names.clone(), sc.tys.clone());
                Ok(out)
            }
            "project" => {
                let from = self.eval(f, run, p, (self.frames[node].first as usize))?;
                let cols: Vec<usize> = strs(&a["columns"])?.iter().map(|c| from.col(c)).collect::<R<_>>()?;
                let osc = self.schema(run, node)?;
                let mut out = Table::with(osc.names.clone(), osc.tys.clone());
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
                let from = self.eval(f, run, p, (self.frames[node].first as usize))?;
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
                let mut out = self.empty(run, node)?;
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
                let from = self.eval(f, run, p, (self.frames[node].first as usize))?;
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
                    let reach = reach_dir(&l, &k, true);
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
                let mut out = self.empty(run, node)?;
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
                let from = self.eval(f, run, p, (self.frames[node].first as usize))?;
                let (sc, ec) = (from.col(s(&a["start"])?)?, from.col(s(&a["end"])?)?);
                let mut out = (*from).clone();
                let col: Vec<u64> = (0..from.n)
                    .map(|i| {
                        let (x, y) = (from.cols[sc][i], from.cols[ec][i]);
                        if x == NULL || y == NULL { NULL } else { (x & 0xffff_ffff_0000_0000) | (y & 0xffff_ffff) }
                    })
                    .collect();
                out.cols.push(col);
                let sc = self.schema(run, node)?;
                (out.names, out.tys) = (sc.names.clone(), sc.tys.clone());
                Ok(out)
            }
            "span-bounds" => {
                let from = self.eval(f, run, p, (self.frames[node].first as usize))?;
                let sc = from.col(s(&a["span"])?)?;
                let mut out = (*from).clone();
                out.cols.push(from.cols[sc].iter().map(|x| x >> 32).collect());
                out.cols.push(from.cols[sc].iter().map(|x| x & 0xffff_ffff).collect());
                let osc = self.schema(run, node)?;
                (out.names, out.tys) = (osc.names.clone(), osc.tys.clone());
                Ok(out)
            }
            "source-matches" => self.source_matches(f, run, p, node, a),
            "replace" => {
                let from = self.eval(f, run, p, (self.frames[node].first as usize))?;
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
                out.cols.push(col);
                let sc = self.schema(run, node)?;
                (out.names, out.tys) = (sc.names.clone(), sc.tys.clone());
                Ok(out)
            }
            "capture" => {
                let from = self.eval(f, run, p, (self.frames[node].first as usize))?;
                let pc = from.col(s(&a["path"])?)?;
                let caps = strs(&a["captures"])?;
                let re = run.re(&path_pattern(s(&a["pattern"])?))?;
                let mut out = self.empty(run, node)?;
                let mut row = Vec::new();
                for i in 0..from.n {
                    let v = from.cols[pc][i];
                    let text = if from.tys[pc] == Ty::File { run.text(run.path_sym[v as usize]).to_vec() } else { run.text(v as u32).to_vec() };
                    if let Some(m) = re.captures(&text) {
                        from.row(i, &mut row);
                        for c in caps.iter() {
                            let t = m.name(c).map_or(&b""[..], |x| x.as_bytes()).to_vec();
                            row.push(u64::from(run.sym(&t)));
                        }
                        out.push_row(&row);
                    }
                }
                Ok(out)
            }
            "group-ordinal" => {
                // the distinct rows, each with how many distinct rows of its group have a strictly smaller order
                let from = self.eval(f, run, p, (self.frames[node].first as usize))?;
                let keys: Vec<usize> = strs(&a["keys"])?.iter().map(|c| from.col(c)).collect::<R<_>>()?;
                let oc = from.col(s(&a["order"])?)?;
                let mut seen = rustc_hash::FxHashSet::default();
                let mut distinct = Vec::new();
                let mut row = Vec::new();
                for i in 0..from.n {
                    from.row(i, &mut row);
                    if seen.insert(row.clone()) {
                        distinct.push(i);
                    }
                }
                let mut groups: FxHashMap<Vec<u64>, Vec<i64>> = FxHashMap::default();
                for &i in &distinct {
                    groups.entry(keys.iter().map(|c| from.cols[*c][i]).collect()).or_default().push(from.cols[oc][i] as i64);
                }
                for g in groups.values_mut() {
                    g.sort_unstable();
                }
                let mut out = self.empty(run, node)?;
                for &i in &distinct {
                    let g = &groups[&keys.iter().map(|c| from.cols[*c][i]).collect::<Vec<u64>>()];
                    let place = g.partition_point(|x| *x < from.cols[oc][i] as i64);
                    from.row(i, &mut row);
                    row.push(place as u64);
                    out.push_row(&row);
                }
                Ok(out)
            }
            "render-syntax" => {
                let from = self.eval(f, run, p, (self.frames[node].first as usize))?;
                let nc = from.col(s(&a["node"])?)?;
                let ctx = s(&a["context"])?;
                let mut out = self.empty(run, node)?;
                let mut row = Vec::new();
                for i in 0..from.n {
                    if let Some(t) = p.render(from.cols[nc][i] as u32, ctx)? {
                        from.row(i, &mut row);
                        row.push(u64::from(run.sym(&t)));
                        out.push_row(&row);
                    }
                }
                Ok(out)
            }
            "fits-rendered" => {
                let from = self.eval(f, run, p, (self.frames[node].first as usize))?;
                let texts: Vec<usize> = strs(&a["texts"])?.iter().map(|c| from.col(c)).collect::<R<_>>()?;
                let lits = strs(&a["literals"])?;
                let sc = from.col(s(&a["span"])?)?;
                let max = a["max-line"].as_u64().ok_or("max-line")? as usize;
                let tab = a["tab-width"].as_u64().ok_or("tab-width")? as usize;
                let mut out = self.empty(run, node)?;
                let mut row = Vec::new();
                let src = std::str::from_utf8(f.src).map_err(|_| "a source that is no text")?;
                for i in 0..from.n {
                    let sp = from.cols[sc][i];
                    let start = (sp >> 32) as usize;
                    let line_start = src[..start].rfind(['\r', '\n']).map_or(0, |at| at + 1);
                    let mut cand = String::new();
                    for (l, c) in lits.iter().zip(&texts) {
                        cand.push_str(l);
                        cand.push_str(&String::from_utf8_lossy(run.text(from.cols[*c][i] as u32)));
                    }
                    if let Some(last) = lits.last() {
                        cand.push_str(last);
                    }
                    if fits(&cand, &src[line_start..start], max, tab) {
                        from.row(i, &mut row);
                        out.push_row(&row);
                    }
                }
                Ok(out)
            }
            "rows" => {
                let mut out = self.empty(run, node)?;
                for r in a["from"].as_array().ok_or("rows from")? {
                    let row: Vec<u64> = r.as_array().ok_or("row")?.iter().map(|x| u64::from(run.sym(x.as_str().unwrap_or("").as_bytes()))).collect();
                    out.push_row(&row);
                }
                Ok(out)
            }
            other => Err(format!("operator {other} not in the step set")),
        }
    }

    /// 6i: an upper bound on a step's rows from what is known without making it: a made table's rows, a view's bucket, a
    /// select bounded by its smallest bounded source (a join's rows never exceed... a heuristic: the smallest source's),
    /// a union's sum, a pass-through step's input. None: unknown.
    fn bound_size(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize, depth: u32) -> R<Option<usize>> {
        if let Some(Res::Tab(t)) = &f.memo[node] {
            return Ok(Some(t.n));
        }
        if let Some(v) = self.view(run, node)? {
            self.load_fact(f, run, p, v.base)?;
            return Ok(kind_set(run, &v.filter, 0, v.base, &[]).map(|ks| bucket_len(f, run, &ks)).or_else(|| lit_rows(f, &v.filter, 0, v.base, &[]).map(|r| r.len())));
        }
        if depth == 0 {
            return Ok(None);
        }
        let plan: &'p Plan = self.plan;
        let pn = &plan.nodes[node];
        Ok(match pn.op.as_str() {
            "select" => {
                let mut m: Option<usize> = None;
                for i in pn.inputs.clone() {
                    if let Some(b) = self.bound_size(f, run, p, i, depth - 1)? {
                        m = Some(m.map_or(b, |x| x.min(b)));
                    }
                }
                m
            }
            "union" => {
                let (l, r) = (self.arg_rel(node, "left")?, self.arg_rel(node, "right")?);
                match (self.bound_size(f, run, p, l, depth - 1)?, self.bound_size(f, run, p, r, depth - 1)?) {
                    (Some(a), Some(b)) => Some(a + b),
                    _ => None,
                }
            }
            "antijoin" => self.bound_size(f, run, p, self.arg_rel(node, "left")?, depth - 1)?,
            "extend" | "project" => self.bound_size(f, run, p, self.arg_rel(node, "from")?, depth - 1)?,
            _ => None,
        })
    }

    /// 6h: a fact view's fill, asked once a file (the same fill a whole `base` step reads).
    fn load_fact(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, nat: Native) -> R<()> {
        let Native::Fact(i) = nat else { return Ok(()) };
        let i = usize::from(i);
        if f.facts.get(i).is_some_and(Option::is_some) {
            return Ok(());
        }
        let rel = run.fact_rels[i].0.clone();
        if !f.fills.contains_key(&rel) {
            let t = p.fill(run, &rel)?;
            f.fills.insert(rel.clone(), Rc::new(t));
        }
        if f.facts.len() <= i {
            f.facts.resize(i + 1, None);
        }
        f.facts[i] = f.fills.get(&rel).cloned();
        Ok(())
    }

    fn base(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, rel: &str) -> R<Table> {
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
        if rel == "project-namespace-membership" {
            return namespace_membership(f, run);
        }
        if rel == "module-identity" {
            return module_identity(f, run);
        }
        let t = p.fill(run, rel)?;
        f.fills.insert(rel.to_owned(), Rc::new(t.clone()));
        Ok(t)
    }

    fn antijoin(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize, a: &J) -> R<Table> {
        let l = self.eval(f, run, p, self.arg_rel(node, "left")?)?;
        let rn = self.arg_rel(node, "right")?;
        let lk: Vec<usize> = strs(&a["left-on"])?.iter().map(|c| l.col(c)).collect::<R<_>>()?;
        let rsc = self.schema(run, rn)?;
        let rk: Vec<usize> = strs(&a["right-on"])?.iter().map(|c| pos(&rsc.names, c)).collect::<R<_>>()?;
        let mut out = Table::with(l.names.clone(), l.tys.clone());
        let mut row = Vec::new();
        // a right side that is a view probed in place on a key: a knock-out per left row, nothing made
        if self.view(run, rn)?.is_some()
            && let Res::View(v) = self.res(f, run, p, rn)?
            && let Some(ki) = (0..rk.len()).find(|i| File::probes(v.base, v.map[rk[*i]]))
        {
            let mut hits = Vec::new();
            for i in 0..l.n {
                hits.clear();
                f.native_probe(run, v.base, v.map[rk[ki]], l.cols[lk[ki]][i], &mut hits);
                let knocked = hits.iter().any(|h| {
                    view_holds(f, run, &v, *h) && (0..rk.len()).all(|k| f.native_get(run, v.base, *h, v.map[rk[k]]) == l.cols[lk[k]][i] || rsc.tys[rk[k]] == Ty::File)
                });
                self.stats.touched[node] += hits.len() as u64;
                if !knocked {
                    l.row(i, &mut row);
                    out.push_row(&row);
                }
            }
            return Ok(out);
        }
        let r = self.right_by_keys(f, run, p, rn, &l, &lk, &rk)?;
        if r.n == 0 {
            return Ok((*l).clone());
        }
        self.stats.touched[node] += r.n as u64;
        if rk.len() == 1 {
            let set: rustc_hash::FxHashSet<u64> = r.cols[rk[0]].iter().copied().collect();
            for i in 0..l.n {
                if !set.contains(&l.cols[lk[0]][i]) {
                    l.row(i, &mut row);
                    out.push_row(&row);
                }
            }
            return Ok(out);
        }
        let set: rustc_hash::FxHashSet<Vec<u64>> = (0..r.n).map(|i| rk.iter().map(|c| r.cols[*c][i]).collect()).collect();
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

    fn src_of(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize) -> R<Src> {
        if let Some(t) = self.overridden(node) {
            return Ok(Src::Tab(t, usize::MAX - 40));
        }
        if self.plan.nodes[node].op == "closure" {
            // a closure put in place by a probe (its rows for the keys asked) is read as that table
            if !sw!("VEC_NO_WALK")
                && let Some(Res::Tab(t)) = &f.memo[node]
            {
                return Ok(Src::Tab(t.clone(), node));
            }
            if !f.lazies.contains_key(&node) && f.memo[node].is_none() && !sw!("VEC_NO_WALK") && {
                let sc = self.schema(run, node)?;
                let nk = strs(&self.plan.nodes[node].j["args"]["keys"])?.len();
                sc.tys[..nk].iter().all(|t| *t == Ty::File)
            } {
                return Ok(Src::Pending(node));
            }
            return Ok(Src::Lazy(self.lazy(f, run, p, node)?));
        }
        if self.deferred(f, run, node)? {
            return Ok(Src::Pending(node));
        }
        Ok(match self.res(f, run, p, node)? {
            Res::Tab(t) => Src::Tab(t, node),
            Res::View(v) => Src::View(v, node),
        })
    }

    fn select(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize) -> R<Table> {
        {
            let sp = self.sel(run, node)?;
            if filter_order::file_fails(run, f, &sp.conj) {
                *self.tally.back.entry((node, "a per-file test fails")).or_default() += 1;
                return self.empty(run, node);
            }
        }
        if let Some(pb) = self.plan.nodes[node].j.get("semi").and_then(J::as_u64)
            && self.plan.nodes[node].j.get("count").is_none()
            && !sw!("VEC_NO_SEMI")
            && let Some(t) = self.semi(f, run, p, node, pb as usize)?
        {
            return Ok(t);
        }
        let sp = self.sel(run, node)?;
        let k = sp.srcs.len();
        let conj = &sp.conj;
        let refs = &sp.refs;
        // 6e: a probed source put in place at one position (the others read the step itself)
        let ov = self.take_positional(&sp.srcs);
        // the made inputs first, in order: an empty one ends the step
        let mut srcs: Vec<Src> = Vec::with_capacity(k);
        for (i, n) in sp.srcs.iter().enumerate() {
            let src = match &ov {
                Some((at, t)) if *at == i => Src::Tab(t.clone(), usize::MAX - 40),
                _ => self.src_of(f, run, p, *n)?,
            };
            if let Src::Tab(t, _) = &src
                && t.n == 0
            {
                let _ = i;
                return self.empty(run, node);
            }
            srcs.push(src);
        }
        // the survivor handoff: a source with a seed test starts the step from the rule's survivors, read by key
        let mut driver: Option<usize> = None;
        if let Some((si, c, k)) = conj.iter().find_map(|e| match e {
            E::Seed(x, k) => match **x {
                E::Col(si, c, _) => Some((si, c, *k)),
                _ => None,
            },
            _ => None,
        }) {
            let ids: Vec<u64> = f.seeds.get(k).map(|s| s.iter().map(|x| u64::from(*x)).collect()).unwrap_or_default();
            let t = self.probe(f, run, p, sp.srcs[si], c, &ids)?;
            self.stats.touched[node] += ids.len() as u64;
            if t.n == 0 {
                return self.empty(run, node);
            }
            srcs[si] = Src::Tab(t, usize::MAX - si);
            driver = Some(si);
        }
        // 6e item 1: a source the survivors bound is made first (it is small), so exact counts choose the driver
        if driver.is_none() && !self.static_keyed {
            for i in 0..k {
                if let Src::Pending(n) = srcs[i]
                    && self.bound(run, n)
                {
                    self.reason = "bound by the survivors (made, to drive)";
                    let t = self.eval(f, run, p, n)?;
                    if t.n == 0 {
                        return self.empty(run, node);
                    }
                    srcs[i] = Src::Tab(t, n);
                }
            }
        }
        // the driver: the smallest entry that is already a count, a made table's rows or a view's kind bucket
        let mut best = if driver.is_some() { 0 } else { usize::MAX };
        for (i, sr) in srcs.iter().enumerate() {
            let size = match sr {
                Src::Tab(t, _) => Some(t.n),
                Src::View(v, _) => kind_set(run, &v.filter, 0, v.base, &[])
                    .or_else(|| kind_set(run, conj, i, v.base, &v.map))
                    .map(|ks| bucket_len(f, run, &ks))
                    .or_else(|| lit_rows(f, &v.filter, 0, v.base, &[]).or_else(|| lit_rows(f, conj, i, v.base, &v.map)).map(|r| r.len())),
                _ => None,
            };
            if let Some(n) = size
                && n < best
            {
                best = n;
                driver = Some(i);
            }
        }
        // 6i: a step not made yet whose rows are bounded by a small made or bucketed input gets that bound as its size;
        // when it is smaller than every sized entry it is made and drives (`VEC_NO_PENDSIZE` off)
        if !sw!("VEC_NO_PENDSIZE") && best != 0 {
            let mut pick: Option<(usize, usize, usize)> = None;
            for i in 0..k {
                if let Src::Pending(n) = srcs[i]
                    && let Some(b) = self.bound_size(f, run, p, n, 6)?
                    && b < best
                    && pick.is_none_or(|q| b < q.2)
                {
                    pick = Some((i, n, b));
                }
            }
            if let Some((i, n, _)) = pick {
                self.reason = "join driver: a step bounded by a small input";
                let t = self.eval(f, run, p, n)?;
                if t.n == 0 {
                    return self.empty(run, node);
                }
                best = t.n;
                srcs[i] = Src::Tab(t, n);
                driver = Some(i);
            }
        }
        let own_of = |d: usize| -> Vec<usize> { (0..conj.len()).filter(|c| refs[*c] & !(1u64 << d) == 0).collect() };
        // the driver: a made table (the smallest), else a view entered by a kind bucket, else a step not made yet (made
        // now: the first in order), else a view read whole
        let driver = match driver {
            Some(d) => d,
            None => match (0..k).find(|i| match &srcs[*i] {
                Src::View(v, _) => kind_set(run, &v.filter, 0, v.base, &[]).is_some() || kind_set(run, conj, *i, v.base, &v.map).is_some() || lit_rows(f, &v.filter, 0, v.base, &[]).is_some() || lit_rows(f, conj, *i, v.base, &v.map).is_some(),
                _ => false,
            }) {
                Some(d) => d,
                None => match (0..k).find_map(|i| {
                    let Src::Pending(n) = srcs[i] else { return None };
                    let sc = self.schema(run, n).ok()?;
                    let (kc, ic) = (sc.names.iter().position(|c| c == "kind")?, sc.names.iter().position(|c| c == "id")?);
                    let ks = conj.iter().filter(|e| refs_of(e) == 1 << i).find_map(|e| kinds_required(e, i, kc))?;
                    self.tree_kind(run, n, kc, ic).then_some((i, n, ic, ks))
                }) {
                    Some((d, n, ic, ks)) => {
                        // entered by the kind bucket: the step is read only at those ids (its own tests still apply)
                        let ids: Vec<u64> = bucket(f, run, &ks).into_iter().map(u64::from).collect();
                        let t = self.probe(f, run, p, n, ic, &ids)?;
                        self.stats.touched[node] += ids.len() as u64;
                        if t.n == 0 {
                            return self.empty(run, node);
                        }
                        srcs[d] = Src::Tab(t, usize::MAX - d);
                        d
                    }
                    None => match (0..k).find(|i| matches!(srcs[*i], Src::Pending(_))) {
                    Some(d) => {
                        let Src::Pending(n) = srcs[d] else { unreachable!() };
                        self.reason = "join driver: no source bound";
                        let t = self.eval(f, run, p, n)?;
                        if t.n == 0 {
                            return self.empty(run, node);
                        }
                        srcs[d] = Src::Tab(t, n);
                        d
                    }
                    None => (0..k).find(|i| matches!(srcs[*i], Src::View(..))).ok_or("a select with no source to start from")?,
                    },
                },
            },
        };
        let mut used = vec![false; conj.len()];
        let mut bound: u64 = 1 << driver;
        let mut tuples: Vec<Vec<u32>> = vec![Vec::new(); k];
        let own = own_of(driver);
        let first: Rc<Vec<u32>> = match &srcs[driver] {
            Src::Tab(t, _) => Rc::new((0..t.n as u32).collect()),
            Src::View(v, vn) => {
                // the select's own kind tests narrow the view's entry too
                match kind_set(run, conj, driver, v.base, &v.map) {
                    Some(ks) if kind_set(run, &v.filter, 0, v.base, &[]).is_none() => Rc::new(bucket(f, run, &ks).into_iter().filter(|r| view_holds(f, run, v, *r)).collect()),
                    _ => match lit_rows(f, conj, driver, v.base, &v.map) {
                        Some(rows) if lit_rows(f, &v.filter, 0, v.base, &[]).is_none() => Rc::new(rows.into_iter().filter(|r| view_holds(f, run, v, *r)).collect()),
                        _ => view_rows(f, run, v, *vn),
                    },
                }
            }
            _ => return Err("a select starts from no table or view".to_owned()),
        };
        self.stats.touched[node] += first.len() as u64;
        let mut cur = vec![0u32; k];
        for &r in first.iter() {
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
            if count == 0 {
                return self.empty(run, node);
            }
            // the next source: one an unused equality joins to the bound ones, one probed in place first
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
                                Src::View(v, _) => File::probes(v.base, v.map[col]),
                                Src::Lazy(l) => col + 2 >= l.names.len(),
                                Src::Pending(_) => true,
                                _ => false,
                            };
                            if pick.is_none_or(|q| native && !q.3) {
                                pick = Some((si, col, ci, native));
                            }
                        }
                    }
                }
            }
            let mut next: Vec<Vec<u32>> = vec![Vec::new(); k];
            let fill_cur = |cur: &mut Vec<u32>, tuples: &Vec<Vec<u32>>, ti: usize, bound: u64| {
                for (s2, cur_s) in cur.iter_mut().enumerate() {
                    if bound & (1 << s2) != 0 {
                        *cur_s = tuples[s2][ti];
                    }
                }
            };
            match pick {
                Some((si, col, ci, native)) => {
                    used[ci] = true;
                    let other = match &conj[ci] {
                        E::Cmp(_, x, y) => {
                            if matches!(**x, E::Col(a, b, _) if a == si && b == col) { (**y).clone() } else { (**x).clone() }
                        }
                        _ => unreachable!(),
                    };
                    // a step not made yet is probed with the keys the bound rows give it
                    if let Src::Pending(pn) = srcs[si] {
                        let mut ks = rustc_hash::FxHashSet::default();
                        for ti in 0..count {
                            fill_cur(&mut cur, &tuples, ti, bound);
                            ks.insert(eval(run, f, &srcs, &cur, &other).1);
                        }
                        let ks: Vec<u64> = ks.into_iter().collect();
                        let sub = self.probe(f, run, p, pn, col, &ks)?;
                        self.stats.touched[node] += sub.n as u64;
                        srcs[si] = Src::Tab(sub, usize::MAX - si);
                    }
                    // a view probed on a column it cannot be probed on in place is made whole, then indexed
                    if let Src::View(_, vn) = &srcs[si]
                        && !native
                    {
                        let vn = *vn;
                        self.reason = "view probed on a column it has no keyed read for";
                        srcs[si] = Src::Tab(self.eval(f, run, p, vn)?, vn);
                    }
                    // a closure is made for the ends this join asks, and only those
                    if let Src::Lazy(l) = &srcs[si] {
                        let l = l.clone();
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
                        let forward = col + 2 == l.names.len();
                        for ti in 0..count {
                            fill_cur(&mut cur, &tuples, ti, bound);
                            let mut key: Vec<u64> = key_exprs.iter().map(|e| eval(run, f, &srcs, &cur, e).1).collect();
                            let at = eval(run, f, &srcs, &cur, &other).1;
                            key.push(at);
                            if asked.insert(key.clone()) {
                                for d in reach_dir(&l, &key, forward).iter() {
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
                    let nb = bound | (1 << si);
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
                    if tab_cols.len() > 1 {
                        used[ci] = false;
                    }
                    let checks: Vec<usize> = (0..conj.len()).filter(|c| !used[*c] && refs[*c] & !nb == 0).collect();
                    let index = if let Src::Tab(t, tn) = &srcs[si] {
                        Some(if *tn > usize::MAX - 64 { Rc::new(index_of(t, &tab_cols)) } else { table_index(f, *tn, t, &tab_cols) })
                    } else {
                        None
                    };
                    let mut probe_key: Vec<u64> = Vec::new();
                    let mut hits = Vec::new();
                    for ti in 0..count {
                        fill_cur(&mut cur, &tuples, ti, bound);
                        let v = eval(run, f, &srcs, &cur, &other);
                        hits.clear();
                        match &srcs[si] {
                            Src::View(vs, _) => {
                                f.native_probe(run, vs.base, vs.map[col], v.1, &mut hits);
                                hits.retain(|h| view_holds(f, run, vs, *h));
                            }
                            Src::Tab(..) => {
                                probe_key.clear();
                                probe_key.push(v.1);
                                for e in &tab_exprs[1..] {
                                    probe_key.push(eval(run, f, &srcs, &cur, e).1);
                                }
                                if let Some(rows) = index.as_ref().and_then(|ix| ix.get(&key_of(probe_key.iter().copied()))) {
                                    hits.extend_from_slice(rows);
                                }
                            }
                            _ => unreachable!(),
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
                    let si = (0..k).find(|s2| bound & (1 << s2) == 0).expect("an unbound source");
                    if let Src::Lazy(l) = &srcs[si] {
                        let ln = l.node;
                        self.reason = "closure crossed (no key joins it)";
                        srcs[si] = Src::Tab(self.eval(f, run, p, ln)?, ln);
                    }
                    if let Src::Pending(pn) = srcs[si] {
                        self.reason = "cross product: no key joins it";
                        srcs[si] = Src::Tab(self.eval(f, run, p, pn)?, pn);
                    }
                    let nb = bound | (1 << si);
                    let checks: Vec<usize> = (0..conj.len()).filter(|c| !used[*c] && refs[*c] & !nb == 0).collect();
                    let rows: Rc<Vec<u32>> = match &srcs[si] {
                        Src::Tab(t, _) => Rc::new((0..t.n as u32).collect()),
                        Src::View(v, vn) => view_rows(f, run, v, *vn),
                        _ => unreachable!(),
                    };
                    for ti in 0..count {
                        fill_cur(&mut cur, &tuples, ti, bound);
                        self.stats.touched[node] += rows.len() as u64;
                        for r in rows.iter() {
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
        let mut out = self.empty(run, node)?;
        for (c, (_, e)) in sp.items.iter().enumerate() {
            let mut col = Vec::with_capacity(count);
            for ti in 0..count {
                for (s2, cur_s) in cur.iter_mut().enumerate() {
                    *cur_s = tuples[s2][ti];
                }
                col.push(eval(run, f, &srcs, &cur, e).1);
            }
            out.cols[c] = col;
        }
        out.n = count;
        // 7: a projection of one source that drops columns is a set, as the engine's relations are (`VEC_SELECT_BAG`: carried as a bag, the interpreter before 7)
        if !sw!("VEC_SELECT_BAG")
            && k == 1
            && sp.items.iter().all(|(_, e)| matches!(e, E::Col(0, _, _)))
            && match &srcs[0] {
                Src::Tab(t, _) => sp.items.len() < t.names.len(),
                Src::View(v, _) => sp.items.len() < v.map.len(),
                _ => false,
            }
        {
            return Ok(distinct(out));
        }
        Ok(out)
    }

    fn axis(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize, a: &J) -> R<Table> {
        let from = self.eval(f, run, p, self.arg_rel(node, "from")?)?;
        self.axis_over(f, run, p, node, a, &from)
    }

    /// The hierarchy an axis walks: the tree, a view probed by its child and parent columns, or a made table indexed once.
    fn hier(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, rel: usize) -> R<Hier> {
        if self.plan.nodes[rel].op == "base" && native_of(self.plan.nodes[rel].j["rel"].as_str().unwrap_or("")) == Some(Native::Child) {
            return Ok(Hier::Tree);
        }
        let rsc = self.schema(run, rel)?;
        let (pc, cc) = (pos(&rsc.names, "parent")?, pos(&rsc.names, "child")?);
        Ok(match self.res(f, run, p, rel)? {
            Res::View(v) if File::probes(v.base, v.map[pc]) && File::probes(v.base, v.map[cc]) => Hier::View(v, pc, cc),
            _ => {
                self.reason = "axis hierarchy";
                let t = self.eval(f, run, p, rel)?;
                let mut up: FxHashMap<u64, Vec<u64>> = FxHashMap::default();
                let mut down: FxHashMap<u64, Vec<u64>> = FxHashMap::default();
                for r in 0..t.n {
                    up.entry(t.cols[cc][r]).or_default().push(t.cols[pc][r]);
                    down.entry(t.cols[pc][r]).or_default().push(t.cols[cc][r]);
                }
                Hier::Maps(up, down)
            }
        })
    }

    /// The domain an axis admits by: everything, a set, a view probed by id, or a step read by key (never made whole).
    fn dom(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize) -> R<Dom> {
        let plan: &'p Plan = self.plan;
        let Some(J::Object(_)) = plan.nodes[node].j["args"].get("domain") else { return Ok(Dom::All) };
        let dn = self.arg_rel(node, "domain")?;
        let sc = self.schema(run, dn)?;
        let c = pos(&sc.names, "id")?;
        if self.deferred(f, run, dn)? {
            return Ok(Dom::Keyed(dn, c));
        }
        self.reason = "axis domain";
        Ok(match self.res(f, run, p, dn)? {
            Res::View(v) if File::probes(v.base, v.map[c]) => Dom::View(v, c),
            _ => {
                let d = self.eval(f, run, p, dn)?;
                Dom::Set(d.cols[c].iter().copied().collect())
            }
        })
    }

    /// Which of `xs` the domain admits, asked as one batch.
    fn admit(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, dom: &Dom, xs: &[u64]) -> R<rustc_hash::FxHashSet<u64>> {
        Ok(match dom {
            Dom::All => xs.iter().copied().collect(),
            Dom::Set(s) => xs.iter().copied().filter(|x| s.contains(x)).collect(),
            Dom::View(v, c) => {
                let mut hits = Vec::new();
                let mut out = rustc_hash::FxHashSet::default();
                for x in xs {
                    hits.clear();
                    f.native_probe(run, v.base, v.map[*c], *x, &mut hits);
                    if hits.iter().any(|h| view_holds(f, run, v, *h)) {
                        out.insert(*x);
                    }
                }
                out
            }
            Dom::Keyed(dn, c) => {
                let mut ks: Vec<u64> = xs.to_vec();
                ks.sort_unstable();
                ks.dedup();
                let t = self.probe(f, run, p, *dn, *c, &ks)?;
                t.cols[*c].iter().copied().collect()
            }
        })
    }

    /// The axis over given start rows (made, or a probed subset).
    fn axis_over(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize, a: &J, from: &Table) -> R<Table> {
        let rel = self.arg_rel(node, "relation")?;
        let dir = s(&a["direction"])?.to_owned();
        let mut out = self.empty(run, node)?;
        if from.n == 0 {
            return Ok(out);
        }
        if let Some(t) = self.walk_axis(f, run, p, node, a, from)? {
            return Ok(t);
        }
        let dom = self.dom(f, run, p, node)?;
        let hier = self.hier(f, run, p, rel)?;
        let ic = from.col("id")?;
        let mut row = Vec::new();
        let mut nbrs = Vec::new();
        // nearest ancestor: walked level by level for all starts at once, the domain asked once a level
        if dir == "nearest-ancestor" {
            let mut found: Vec<u64> = vec![NULL; from.n];
            let mut frontier: Vec<(usize, u64)> = (0..from.n).map(|i| (i, from.cols[ic][i])).collect();
            for _ in 0..4096 {
                if frontier.is_empty() {
                    break;
                }
                let mut next: Vec<(usize, u64)> = Vec::new();
                for (i, x) in &frontier {
                    hier_step(f, run, &hier, *x, true, &mut nbrs);
                    for q in &nbrs {
                        next.push((*i, *q));
                    }
                }
                self.stats.touched[node] += next.len() as u64;
                let qs: Vec<u64> = next.iter().map(|x| x.1).collect();
                let ok = self.admit(f, run, p, &dom, &qs)?;
                frontier.clear();
                for (i, q) in next {
                    if found[i] != NULL {
                        continue;
                    }
                    if ok.contains(&q) {
                        found[i] = q;
                    } else {
                        frontier.push((i, q));
                    }
                }
                frontier.retain(|(i, _)| found[*i] == NULL);
            }
            for (i, q) in found.iter().enumerate() {
                if *q != NULL {
                    from.row(i, &mut row);
                    row.push(*q);
                    out.push_row(&row);
                }
            }
            return Ok(out);
        }
        let mut all: Vec<(usize, u64)> = Vec::new();
        for i in 0..from.n {
            let x = from.cols[ic][i];
            match dir.as_str() {
                "parent" | "ancestor" => {
                    let mut seen = rustc_hash::FxHashSet::default();
                    let mut frontier = vec![x];
                    while let Some(at) = frontier.pop() {
                        hier_step(f, run, &hier, at, true, &mut nbrs);
                        for q in nbrs.clone() {
                            if seen.insert(q) {
                                all.push((i, q));
                                if dir != "parent" {
                                    frontier.push(q);
                                }
                            }
                        }
                    }
                }
                "descendant" | "child" => {
                    if matches!(hier, Hier::Tree) && (x as usize) < f.n {
                        if dir == "child" {
                            for c in node_table::children_of(f.table, Position(x as u32)) {
                                all.push((i, u64::from(c.0)));
                            }
                        } else {
                            let end = node_table::subtree_end_of(f.table, Position(x as u32)).0;
                            all.extend((x as u32 + 1..end).map(|q| (i, u64::from(q))));
                        }
                    } else {
                        let mut seen = rustc_hash::FxHashSet::default();
                        let mut stack = vec![x];
                        while let Some(at) = stack.pop() {
                            hier_step(f, run, &hier, at, false, &mut nbrs);
                            for c in nbrs.clone() {
                                if seen.insert(c) {
                                    all.push((i, c));
                                    if dir != "child" {
                                        stack.push(c);
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
                            all.push((i, u64::from(nx.0)));
                        }
                    }
                }
                other => return Err(format!("axis direction {other}")),
            }
        }
        self.stats.touched[node] += all.len() as u64;
        let qs: Vec<u64> = all.iter().map(|x| x.1).collect();
        let ok = self.admit(f, run, p, &dom, &qs)?;
        for (i, q) in all {
            if ok.contains(&q) {
                from.row(i, &mut row);
                row.push(q);
                out.push_row(&row);
            }
        }
        Ok(out)
    }

    /// The axis read by its `related` end: the start rows whose nearest admitted ancestor is one of `keys`. Walked down from
    /// each key through nodes the domain does not admit (a tree or a single-parent chain), then the starts probed by id.
    fn axis_related(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize, keys: &[u64]) -> R<Option<Table>> {
        let plan: &'p Plan = self.plan;
        let a = &plan.nodes[node].j["args"];
        if s(&a["direction"])? != "nearest-ancestor" {
            return Ok(None);
        }
        let rel = self.arg_rel(node, "relation")?;
        let hier = self.hier(f, run, p, rel)?;
        if matches!(hier, Hier::Tree) {
            return Ok(None);
        }
        let dom = self.dom(f, run, p, node)?;
        let fnode = self.arg_rel(node, "from")?;
        let fsc = self.schema(run, fnode)?;
        let ic = pos(&fsc.names, "id")?;
        let mut pairs: Vec<(u64, u64)> = Vec::new();
        let admitted = self.admit(f, run, p, &dom, keys)?;
        let mut frontier: Vec<(u64, u64)> = keys.iter().filter(|k| admitted.contains(k)).map(|k| (*k, *k)).collect();
        let mut nbrs = Vec::new();
        for _ in 0..4096 {
            if frontier.is_empty() {
                break;
            }
            let mut next = Vec::new();
            for (x, at) in &frontier {
                hier_step(f, run, &hier, *at, false, &mut nbrs);
                for c in &nbrs {
                    next.push((*x, *c));
                }
            }
            self.stats.touched[node] += next.len() as u64;
            let cs: Vec<u64> = next.iter().map(|x| x.1).collect();
            let ok = self.admit(f, run, p, &dom, &cs)?;
            frontier.clear();
            for (x, c) in next {
                pairs.push((c, x));
                if !ok.contains(&c) {
                    frontier.push((x, c));
                }
            }
        }
        let mut cs: Vec<u64> = pairs.iter().map(|x| x.0).collect();
        cs.sort_unstable();
        cs.dedup();
        let starts = self.probe(f, run, p, fnode, ic, &cs)?;
        let mut by: FxHashMap<u64, Vec<u64>> = FxHashMap::default();
        for (c, x) in pairs {
            by.entry(c).or_default().push(x);
        }
        let mut out = self.empty(run, node)?;
        let mut row = Vec::new();
        for i in 0..starts.n {
            if let Some(xs) = by.get(&starts.cols[ic][i]) {
                for x in xs {
                    starts.row(i, &mut row);
                    row.push(*x);
                    out.push_row(&row);
                }
            }
        }
        Ok(Some(out))
    }

    fn ordered(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize, a: &J, as_of: bool) -> R<Table> {
        let l = self.eval(f, run, p, self.arg_rel(node, "left")?)?;
        let rn = self.arg_rel(node, "right")?;
        let lk: Vec<usize> = strs(&a["left-on"])?.iter().map(|c| l.col(c)).collect::<R<_>>()?;
        let rsc = self.schema(run, rn)?;
        let rk0: Vec<usize> = strs(&a["right-on"])?.iter().map(|c| pos(&rsc.names, c)).collect::<R<_>>()?;
        let r = self.right_by_keys(f, run, p, rn, &l, &lk, &rk0)?;
        if r.n == 0 {
            return self.empty(run, node);
        }
        let rk: Vec<usize> = strs(&a["right-on"])?.iter().map(|c| r.col(c)).collect::<R<_>>()?;
        let (lo, ro) = (l.col(s(&a["left-order"])?)?, r.col(s(&a["right-order"])?)?);
        let outm = a["output"].as_object().ok_or("output")?;
        let mut picks = Vec::new();
        for (_, v) in outm {
            let side = s(&v[0])?;
            picks.push((side == "left", if side == "left" { l.col(s(&v[1])?)? } else { r.col(s(&v[1])?)? }));
        }
        let mut out = self.empty(run, node)?;
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
        let emit = |out: &mut Table, li: u32, ri: u32| {
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
            for (k, ls) in &mut lefts {
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
        let line_out = a.get("line-output").and_then(J::as_str).is_some();
        let cap = a.get("capture").and_then(J::as_str).map(str::to_owned);
        let cap_text = a.get("capture-text-output").and_then(J::as_str).is_some();
        let cap_line = a.get("capture-line-output").and_then(J::as_str).is_some();
        let mut out = self.empty(run, node)?;
        let mut row = Vec::new();
        for i in 0..from.n {
            let sp = from.cols[sc][i];
            if sp == NULL {
                continue;
            }
            let (st, en) = ((sp >> 32) as usize, (sp & 0xffff_ffff) as usize);
            let text = &f.src[st..en];
            let line0 = base_line.map(|b| from.cols[b][i] as i64);
            // 7: the newlines before `off`, counted on from the last offset asked (matches come in order), not from the start
            let counted = std::cell::Cell::new((0usize, 0i64));
            let line_at = |off: usize| -> i64 {
                let (mut at, mut lines) = counted.get();
                if off < at {
                    (at, lines) = (0, 0);
                }
                lines += text[at..off].iter().filter(|c| **c == b'\n').count() as i64;
                counted.set((off, lines));
                line0.unwrap_or(0) + lines
            };
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
                if line_out {
                    row.push(line_at(w.start()) as u64);
                }
                if let Some(g) = g {
                    row.push((((st + g.start()) as u64) << 32) | (st + g.end()) as u64);
                    if cap_text {
                        row.push(u64::from(run.sym(g.as_bytes())));
                    }
                    if cap_line {
                        row.push(line_at(g.start()) as u64);
                    }
                }
                out.push_row(&row);
            }
        }
        Ok(out)
    }
}

/// The hierarchy an axis walks.
pub(super) enum Hier {
    Tree,
    View(Rc<ViewSpec>, usize, usize),
    Maps(FxHashMap<u64, Vec<u64>>, FxHashMap<u64, Vec<u64>>),
}

/// The domain an axis admits by.
pub(super) enum Dom {
    All,
    Set(rustc_hash::FxHashSet<u64>),
    View(Rc<ViewSpec>, usize),
    Keyed(usize, usize),
}

/// The other end of each edge from `x`: upward (its parents) or downward (its children).
fn hier_step(f: &File<'_>, run: &mut Run, h: &Hier, x: u64, up: bool, out: &mut Vec<u64>) {
    out.clear();
    match h {
        Hier::Tree => {
            if (x as usize) < f.n {
                if up {
                    if f.has_parent(x as u32) {
                        out.push(u64::from(f.parent[x as usize]));
                    }
                } else {
                    out.extend(node_table::children_of(f.table, Position(x as u32)).iter().map(|c| u64::from(c.0)));
                }
            }
        }
        Hier::View(v, pc, cc) => {
            let mut hits = Vec::new();
            let (by, to) = if up { (*cc, *pc) } else { (*pc, *cc) };
            f.native_probe(run, v.base, v.map[by], x, &mut hits);
            for h in hits {
                if view_holds(f, run, v, h) {
                    out.push(f.native_get(run, v.base, h, v.map[to]));
                }
            }
        }
        Hier::Maps(u, d) => {
            if let Some(xs) = if up { u.get(&x) } else { d.get(&x) } {
                out.extend_from_slice(xs);
            }
        }
    }
}

/// Whether a view's tests hold on one of its base rows.
fn view_holds(f: &File<'_>, run: &mut Run, v: &ViewSpec, row: u32) -> bool {
    if v.filter.is_empty() {
        return true;
    }
    let srcs = [Src::Base(v.base)];
    let cur = [row];
    v.filter.iter().all(|e| truthy(eval(run, f, &srcs, &cur, e)))
}

/// A view's rows in this file: its kind bucket when its tests name kinds, else every base row; tested; made once a file.
fn view_rows(f: &mut File<'_>, run: &mut Run, v: &ViewSpec, node: usize) -> Rc<Vec<u32>> {
    if let Some(r) = f.vrows.get(&node) {
        return r.clone();
    }
    if filter_order::file_fails(run, f, &v.filter) {
        let rc = Rc::new(Vec::new());
        f.vrows.insert(node, rc.clone());
        return rc;
    }
    let rows = match kind_set(run, &v.filter, 0, v.base, &[]) {
        Some(ks) => bucket(f, run, &ks),
        None => lit_rows(f, &v.filter, 0, v.base, &[]).unwrap_or_else(|| f.native_rows(run, v.base)),
    };
    let kept: Vec<u32> = rows.into_iter().filter(|r| view_holds(f, run, v, *r)).collect();
    let rc = Rc::new(kept);
    f.vrows.insert(node, rc.clone());
    rc
}

/// How many elements some kinds have.
fn bucket_len(f: &File<'_>, run: &Run, ks: &[u32]) -> usize {
    run.kind_sym.iter().enumerate().filter(|(_, s)| ks.contains(s)).map(|(c, _)| node_table::ids_of_kind(f.table, KindBits(c as u16)).len()).sum()
}

/// The elements of some kinds, in position order.
fn bucket(f: &File<'_>, run: &Run, ks: &[u32]) -> Vec<u32> {
    let mut rows = Vec::new();
    for (code, sym) in run.kind_sym.iter().enumerate() {
        if ks.contains(sym) {
            rows.extend(node_table::ids_of_kind(f.table, KindBits(code as u16)).iter().map(|p| p.0));
        }
    }
    rows.sort_unstable();
    rows
}

/// 6h: a fact view's rows where a test fixes one of its columns to literals (`col = "x"`, `col in [..]`, an or of such): that
/// column's keyed list read at those values, a per-file bucket. `map` takes a view's column to its base column (empty: the
/// base's own).
fn lit_rows(f: &File<'_>, conj: &[E], si: usize, base: Native, map: &[usize]) -> Option<Vec<u32>> {
    let Native::Fact(i) = base else { return None };
    fn cols(e: &E, si: usize, out: &mut Vec<usize>) {
        match e {
            E::Col(s, c, _) if *s == si => out.push(*c),
            E::Cmp(_, a, b) | E::And(a, b) | E::Or(a, b) | E::In(a, b, _) => {
                cols(a, si, out);
                cols(b, si, out);
            }
            _ => {}
        }
    }
    for e in conj {
        let mut cs = Vec::new();
        cols(e, si, &mut cs);
        for c in cs {
            if let Some(vals) = kinds_required(e, si, c) {
                let bc = if map.is_empty() { c } else { map[c] };
                // a text literal names a symbol cell only; a flag or number column is tested row by row
                if f.facts[usize::from(i)].as_ref().is_none_or(|t| t.tys[bc] != Ty::Sym) {
                    continue;
                }
                let ix = f.fact_list(i, bc);
                let mut rows: Vec<u32> = vals.iter().filter_map(|v| ix.get(&u64::from(*v))).flatten().copied().collect();
                rows.sort_unstable();
                rows.dedup();
                return Some(rows);
            }
        }
    }
    None
}

/// The kinds a conjunction requires of a tree source's `kind` column (a literal or an or of literals), if any. `map` takes a
/// view's column to its base column (empty: the columns are the base's).
fn kind_set(_run: &Run, conj: &[E], si: usize, base: Native, map: &[usize]) -> Option<Vec<u32>> {
    if base != Native::Tree {
        return None;
    }
    let is_kind = |c: usize| if map.is_empty() { c == 1 } else { map.get(c) == Some(&1) };
    fn kinds(e: &E, si: usize, is_kind: &dyn Fn(usize) -> bool, out: &mut Vec<u32>) -> bool {
        match e {
            E::Cmp(o, a, b) if o == "=" => match (&**a, &**b) {
                (E::Col(s, c, _), E::Text(t)) | (E::Text(t), E::Col(s, c, _)) if *s == si && is_kind(*c) => {
                    out.push(*t);
                    true
                }
                _ => false,
            },
            E::Or(a, b) => kinds(a, si, is_kind, out) && kinds(b, si, is_kind, out),
            E::In(a, b, false) => match (&**a, &**b) {
                (E::Col(s, c, _), E::List(items)) if *s == si && is_kind(*c) => items.iter().all(|i| {
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
    for c in conj {
        let mut ks = Vec::new();
        if kinds(c, si, &is_kind, &mut ks) {
            return Some(ks);
        }
    }
    None
}

/// The kinds a test requires of column `kc` of source `si`: a literal, a list, an `or` of such, or an `and` holding one.
fn kinds_required(e: &E, si: usize, kc: usize) -> Option<Vec<u32>> {
    match e {
        E::Cmp(o, a, b) if o == "=" => match (&**a, &**b) {
            (E::Col(s, c, _), E::Text(t)) | (E::Text(t), E::Col(s, c, _)) if *s == si && *c == kc => Some(vec![*t]),
            _ => None,
        },
        E::In(a, b, false) => match (&**a, &**b) {
            (E::Col(s, c, _), E::List(items)) if *s == si && *c == kc => items.iter().map(|i| if let E::Text(t) = i { Some(*t) } else { None }).collect(),
            _ => None,
        },
        E::Or(a, b) => {
            let mut x = kinds_required(a, si, kc)?;
            x.extend(kinds_required(b, si, kc)?);
            Some(x)
        }
        E::And(a, b) => kinds_required(a, si, kc).or_else(|| kinds_required(b, si, kc)),
        _ => None,
    }
}

/// An expression over a view's columns, rewritten over its base's.
fn remap(e: &E, map: &[usize]) -> E {
    let b = |x: &E| Box::new(remap(x, map));
    match e {
        E::Col(s, c, t) => E::Col(*s, map[*c], *t),
        E::List(v) => E::List(v.iter().map(|x| remap(x, map)).collect()),
        E::Call(n, v) => E::Call(n.clone(), v.iter().map(|x| remap(x, map)).collect()),
        E::Cmp(o, x, y) => E::Cmp(o.clone(), b(x), b(y)),
        E::In(x, y, n) => E::In(b(x), b(y), *n),
        E::And(x, y) => E::And(b(x), b(y)),
        E::Or(x, y) => E::Or(b(x), b(y)),
        E::Not(x) => E::Not(b(x)),
        E::Re(x, r) => E::Re(b(x), r.clone()),
        E::Seed(x, k) => E::Seed(b(x), *k),
        other => other.clone(),
    }
}


/// A union's rows as a set, as the language's `union` is (`set_rows`): the first of equal rows kept, in order.
fn distinct(t: Table) -> Table {
    if sw!("VEC_UNION_ALL") {
        return t;
    }
    let mut keep = Vec::with_capacity(t.n);
    // 7: one or two columns are hashed as words, not as a list a row
    match t.cols.len() {
        1 => {
            let mut seen: rustc_hash::FxHashSet<u64> = rustc_hash::FxHashSet::default();
            keep.extend((0..t.n).filter(|r| seen.insert(t.cols[0][*r])));
        }
        2 => {
            let mut seen: rustc_hash::FxHashSet<(u64, u64)> = rustc_hash::FxHashSet::default();
            keep.extend((0..t.n).filter(|r| seen.insert((t.cols[0][*r], t.cols[1][*r]))));
        }
        _ => {
            let mut seen: rustc_hash::FxHashSet<Vec<u64>> = rustc_hash::FxHashSet::default();
            for r in 0..t.n {
                if seen.insert(t.cols.iter().map(|c| c[r]).collect()) {
                    keep.push(r);
                }
            }
        }
    }
    if keep.len() == t.n {
        return t;
    }
    let mut out = Table::with(t.names.clone(), t.tys.clone());
    for (c, col) in t.cols.iter().enumerate() {
        out.cols[c] = keep.iter().map(|r| col[*r]).collect();
    }
    out.n = keep.len();
    out
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
    let rc = Rc::new(out);
    cache.borrow_mut().insert(key.to_vec(), rc.clone());
    rc
}

/// A made table's rows by the value of one column, or by a hash of several (a hash hit is tested again by the join).
/// 7: rows by key, compressed: one row array, each key's rows one range of it (ascending), found through an offset array
/// when the keys are small and dense (node ids, offsets), else through a flat hash of (start, length).
pub(super) struct Index {
    dense: bool,
    offs: Vec<u32>,
    at: FxHashMap<u64, (u32, u32)>,
    rows: Vec<u32>,
}

impl Index {
    pub(super) fn build(n: usize, key: impl Fn(usize) -> u64) -> Index {
        let keys: Vec<u64> = (0..n).map(key).collect();
        let max = keys.iter().copied().max().unwrap_or(0);
        if n > 0 && max < (4 * n as u64).max(1024) {
            let m = max as usize + 1;
            let mut offs = vec![0u32; m + 1];
            for k in &keys {
                offs[*k as usize + 1] += 1;
            }
            for i in 0..m {
                offs[i + 1] += offs[i];
            }
            let mut fill = offs[..m].to_vec();
            let mut rows = vec![0u32; n];
            for (r, k) in keys.iter().enumerate() {
                let p = &mut fill[*k as usize];
                rows[*p as usize] = r as u32;
                *p += 1;
            }
            return Index { dense: true, offs, at: FxHashMap::default(), rows };
        }
        let mut order: Vec<(u64, u32)> = keys.iter().enumerate().map(|(r, k)| (*k, r as u32)).collect();
        order.sort_unstable();
        let mut at: FxHashMap<u64, (u32, u32)> = FxHashMap::default();
        at.reserve(n.min(1 << 16));
        let mut i = 0;
        while i < order.len() {
            let mut j = i;
            while j < order.len() && order[j].0 == order[i].0 {
                j += 1;
            }
            at.insert(order[i].0, (i as u32, (j - i) as u32));
            i = j;
        }
        Index { dense: false, offs: Vec::new(), at, rows: order.into_iter().map(|x| x.1).collect() }
    }
    pub(super) fn get(&self, k: &u64) -> Option<&[u32]> {
        if self.dense {
            let k = *k;
            if k + 1 >= self.offs.len() as u64 {
                return None;
            }
            let (a, b) = (self.offs[k as usize] as usize, self.offs[k as usize + 1] as usize);
            return if a == b { None } else { Some(&self.rows[a..b]) };
        }
        self.at.get(k).map(|(a, l)| &self.rows[*a as usize..(*a + *l) as usize])
    }
}

fn key_of(vals: impl Iterator<Item = u64>) -> u64 {
    let mut h: u64 = 0;
    let mut n = 0;
    let mut first = 0;
    for v in vals {
        if n == 0 {
            first = v;
        }
        h = (h.rotate_left(5) ^ v).wrapping_mul(0x517c_c1b7_2722_0a95);
        n += 1;
    }
    if n == 1 { first } else { h }
}

fn index_of(t: &Table, cols: &[usize]) -> Index {
    if cols.len() == 1 {
        let c = &t.cols[cols[0]];
        return Index::build(t.n, |r| c[r]);
    }
    Index::build(t.n, |r| key_of(cols.iter().map(|c| t.cols[*c][r])))
}

fn table_index(f: &mut File<'_>, tn: usize, t: &Table, cols: &[usize]) -> Rc<Index> {
    if let Some(v) = f.index.get(&tn)
        && let Some((_, ix)) = v.iter().find(|(c, _)| c == cols)
    {
        return ix.clone();
    }
    let rc = Rc::new(index_of(t, cols));
    f.index.entry(tn).or_default().push((cols.to_vec(), rc.clone()));
    rc
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
        E::Not(a) | E::Re(a, _) | E::Seed(a, _) => refs_of(a),
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
        Src::View(v, _) => f.native_get(run, v.base, cur[si], v.map[col]),
        Src::Base(n) => f.native_get(run, *n, cur[si], col),
        Src::Lazy(_) | Src::Pending(_) => NULL,
    }
}

/// The rows of a made table whose column holds one of `keys`, through its index on that column (made once a file).
fn subset(f: &mut File<'_>, node: usize, t: &Table, col: usize, keys: &[u64]) -> Table {
    let ix = table_index(f, node, t, &[col]);
    let mut out = Table::with(t.names.clone(), t.tys.clone());
    let mut row = Vec::new();
    for k in keys {
        if let Some(rows) = ix.get(k) {
            for r in rows {
                t.row(*r as usize, &mut row);
                out.push_row(&row);
            }
        }
    }
    out
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
        E::Col(si, c, t) => V::new(*t, get(run, f, srcs, cur, *si, *c)),
        E::Lit(v) => *v,
        E::Text(t) => V::new(Ty::Sym, u64::from(*t)),
        E::Num(n) => V::new(Ty::Num, *n as u64),
        E::List(_) => flag(false),
        E::And(a, b) => flag(truthy(eval(run, f, srcs, cur, a)) && truthy(eval(run, f, srcs, cur, b))),
        E::Or(a, b) => flag(truthy(eval(run, f, srcs, cur, a)) || truthy(eval(run, f, srcs, cur, b))),
        E::Not(a) => flag(!truthy(eval(run, f, srcs, cur, a))),
        E::Cmp(o, a, b) if o == "=" || o == "!=" => {
            let eq = match (&**a, &**b) {
                (E::Col(s1, c1, t1), E::Col(s2, c2, t2)) if t1 == t2 => {
                    let (x, y) = (get(run, f, srcs, cur, *s1, *c1), get(run, f, srcs, cur, *s2, *c2));
                    Some(x == y && x != NULL)
                }
                (E::Col(s1, c1, Ty::Sym), E::Text(t)) | (E::Text(t), E::Col(s1, c1, Ty::Sym)) => Some(get(run, f, srcs, cur, *s1, *c1) == u64::from(*t)),
                _ => None,
            };
            match eq {
                Some(e) => flag(e == (o == "=")),
                None => {
                    let (x, y) = (eval(run, f, srcs, cur, a), eval(run, f, srcs, cur, b));
                    match compare(run, x, y) {
                        Some(ord) => flag((ord == std::cmp::Ordering::Equal) == (o == "=")),
                        None => flag(false),
                    }
                }
            }
        }
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
        E::Seed(a, k) => {
            let x = eval(run, f, srcs, cur, a);
            flag(f.seeds.get(*k).is_some_and(|s| s.binary_search(&(x.1 as u32)).is_ok()))
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
        "numeric_gte" => {
            let num = |run: &Run, v: V| -> Option<f64> {
                match v.ty() {
                    Ty::Num => Some(v.1 as i64 as f64),
                    _ => String::from_utf8_lossy(&text_of(run, v)).replace('_', "").parse::<f64>().ok(),
                }
            };
            flag(matches!((num(run, vs[0]), num(run, vs[1])), (Some(a), Some(b)) if a >= b))
        }
        _ => {
            run.missing.insert(name.to_owned());
            flag(false)
        }
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
    let mut out = Table::with(cols.iter().map(|c| c.0.to_owned()).collect::<Vec<_>>(), cols.iter().map(|c| c.1).collect::<Vec<_>>());
    for r in f.native_rows(run, nat) {
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

/// Whether every line of `text`, after `prefix` on its first line, stays within `max` display columns (the operator's rule).
fn fits(text: &str, prefix: &str, max: usize, tab: usize) -> bool {
    use unicode_width::UnicodeWidthChar;
    let adv = |col: usize, c: char| if c == '\t' { (col / tab + 1) * tab } else { col + UnicodeWidthChar::width(c).unwrap_or(0) };
    let mut col = prefix.chars().fold(0, adv);
    if col > max {
        return false;
    }
    let mut cr = false;
    for c in text.chars() {
        if c == '\r' {
            col = 0;
            cr = true;
            continue;
        }
        if c == '\n' {
            if !cr {
                col = 0;
            }
            cr = false;
            continue;
        }
        cr = false;
        col = adv(col, c);
        if col > max {
            return false;
        }
    }
    true
}

/// `project-namespace-membership` for this file: each dotted prefix of each absolute import path it writes, classified by
/// the entries under the project's search roots (module file, package directory with an initializer, namespace directory),
/// with a present child of a parent not present made unknown; as the provider's project phase states it.
fn namespace_membership(f: &File<'_>, run: &mut Run) -> R<Table> {
    let (_, names, tys) = run.catalog("project-namespace-membership").ok_or("no project-namespace-membership")?;
    let kind = |n: &str| run.kind_sym.iter().position(|s| run.text(*s) == n.as_bytes()).map(|c| c as u16);
    let (alias, from_k, dot, ell, ident, name) = (kind("alias"), kind("import-from"), kind("dot"), kind("ellipsis"), kind("identifier"), kind("name-token"));
    let names_of = |f: &File<'_>, at: u32, out: &mut Vec<String>| {
        let mut stack = vec![at];
        let mut toks = Vec::new();
        while let Some(x) = stack.pop() {
            for c in node_table::children_of(f.table, Position(x)).iter().rev() {
                stack.push(c.0);
            }
            if Some(f.kind[x as usize]) == name {
                toks.push(x);
            }
        }
        toks.sort_unstable_by_key(|t| f.start[*t as usize]);
        for t in toks {
            out.push(String::from_utf8_lossy(&f.src[f.start[t as usize] as usize..f.end[t as usize] as usize]).into_owned());
        }
    };
    let mut paths: std::collections::BTreeSet<(String, String)> = std::collections::BTreeSet::new();
    for a in 0..f.n as u32 {
        if Some(f.kind[a as usize]) != alias {
            continue;
        }
        let imp = f.parent[a as usize];
        let is_from = Some(f.kind[imp as usize]) == from_k;
        let mut level = 0;
        let mut parts: Vec<String> = Vec::new();
        if is_from {
            for c in node_table::children_of(f.table, Position(imp)) {
                let k = Some(f.kind[c.0 as usize]);
                if k == alias {
                    break;
                }
                if k == dot {
                    level += 1;
                } else if k == ell {
                    level += 3;
                } else if k == ident {
                    names_of(f, c.0, &mut parts);
                }
            }
        }
        let id = node_table::children_of(f.table, Position(a)).iter().find(|c| Some(f.kind[c.0 as usize]) == ident).map(|c| c.0);
        match id {
            Some(i) => names_of(f, i, &mut parts),
            None => {
                if !is_from {
                    continue;
                }
            }
        }
        if level != 0 || parts.is_empty() || parts[0].is_empty() {
            continue;
        }
        paths.insert((parts[0].clone(), parts.join(".")));
    }
    let mut out = Table::with(names.clone(), tys.clone());
    let col = |n: &str| names.iter().position(|c| c == n).expect("a column of the relation");
    let (fc, rc, pc, mc, kc) = (col("file"), col("root"), col("path"), col("membership"), col("kind"));
    let mut rows: std::collections::BTreeSet<(String, String)> = std::collections::BTreeSet::new();
    for (root, path) in &paths {
        let comps: Vec<&str> = path.split('.').filter(|c| !c.is_empty()).collect();
        for i in 1..=comps.len() {
            rows.insert((root.clone(), comps[..i].join(".")));
        }
    }
    for (root, path) in rows {
        let kinds = classified(run, &path);
        // a present child of a parent that is not present cannot prove a project path
        let present = |ks: &[&str]| ks.iter().any(|k| matches!(*k, "module" | "package" | "namespace"));
        let kinds = match path.rsplit_once('.') {
            Some((parent, _)) if present(&kinds) && !present(&classified(run, parent)) => vec!["unknown"],
            _ => kinds,
        };
        for k in kinds {
            let state = match k {
                "module" | "package" | "namespace" => "present",
                "absent" => "absent",
                _ => "unknown",
            };
            let mut row = vec![0u64; names.len()];
            row[fc] = u64::from(f.file);
            row[rc] = u64::from(run.sym(root.as_bytes()));
            row[pc] = u64::from(run.sym(path.as_bytes()));
            row[mc] = u64::from(run.sym(state.as_bytes()));
            row[kc] = u64::from(run.sym(k.as_bytes()));
            out.push_row(&row);
        }
    }
    Ok(out)
}

fn classified(run: &mut Run, dotted: &str) -> Vec<&'static str> {
    if let Some(k) = run.classified.get(dotted) {
        return k.clone();
    }
    let fs = dotted.replace('.', "/");
    let (mut module, mut package, mut namespace) = (false, false, false);
    for search in run.search_roots.clone() {
        let base = if search.is_empty() { format!("{}/{fs}", run.root_dir) } else { format!("{}/{search}/{fs}", run.root_dir) };
        let file = |p: &str| std::path::Path::new(p).is_file();
        module |= file(&format!("{base}.py")) || file(&format!("{base}.pyi"));
        let dir = std::path::Path::new(&base).is_dir();
        let init = file(&format!("{base}/__init__.py")) || file(&format!("{base}/__init__.pyi"));
        package |= dir && init;
        namespace |= dir && !init;
    }
    let mut kinds = Vec::new();
    for (p, k) in [(module, "module"), (package, "package"), (namespace, "namespace")] {
        if p {
            kinds.push(k);
        }
    }
    if kinds.is_empty() {
        kinds.push("absent");
    }
    run.classified.insert(dotted.to_owned(), kinds.clone());
    kinds
}

/// `module-identity` for this file: inside a package (its directory holds `__init__.py`), the dotted path from the topmost
/// such directory's parent, the module and package directory names and the topmost package's name (the provider's rule).
fn module_identity(f: &File<'_>, run: &mut Run) -> R<Table> {
    let (_, names, tys) = run.catalog("module-identity").ok_or("no module-identity")?;
    let mut out = Table::with(names.clone(), tys);
    let rel = run.paths[f.file as usize].clone();
    let file = std::path::Path::new(&rel);
    let mut dir = file.parent();
    let mut package: Option<&std::path::Path> = None;
    while let Some(d) = dir {
        let init = std::path::Path::new(&run.root_dir).join(d).join("__init__.py");
        if d.as_os_str().is_empty() || !init.is_file() {
            break;
        }
        package = Some(d);
        dir = d.parent();
    }
    let Some(package) = package else { return Ok(out) };
    let Some(stem) = file.file_name().and_then(|n| n.to_str()).and_then(|n| n.strip_suffix(".pyi").or_else(|| n.strip_suffix(".py"))) else {
        return Ok(out);
    };
    let top = package.parent().unwrap_or(std::path::Path::new(""));
    let mut parts: Vec<String> = file.strip_prefix(top).ok().and_then(|r| r.parent()).map(|r| r.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect()).unwrap_or_default();
    parts.push(stem.to_owned());
    let pkg_dir = file.parent().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let root_name = package.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let mut row = vec![0u64; names.len()];
    for (c, n) in names.iter().enumerate() {
        row[c] = match n.as_str() {
            "file" => u64::from(f.file),
            "path" => u64::from(run.sym(parts.join(".").as_bytes())),
            "module-name" => u64::from(run.sym(stem.as_bytes())),
            "package-name" => u64::from(run.sym(pkg_dir.as_bytes())),
            "package-root-name" => u64::from(run.sym(root_name.as_bytes())),
            _ => NULL,
        };
    }
    out.push_row(&row);
    Ok(out)
}
