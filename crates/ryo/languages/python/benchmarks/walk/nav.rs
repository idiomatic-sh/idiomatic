//! Navigation programs (bead ryo-lxty, throwaway): each rule's core, lowered by `nav.py` from its own logical plan, runs per
//! candidate node as a flat array of steps over the node table (the walk's view): an anchor (a kind bucket, a literal bucket of
//! a relation column, a scan), field reads keyed on bound ids, tests on interned codes, absence/existence sub-programs with
//! early exit, an emit of the core's row. No step writes a row of a relation. Dispatch is nodes outer, rules inner: one pass
//! over the file's kind buckets, each bucket carrying the programs anchored on it. Relations outside the tree (semantic and
//! derived) come from the provider's own readers, run once per file on the first step that needs them.

use crate::sema::FxMap;
use crate::symbols::{NULL, Symbols};
use crate::view::View;
use regex_automata::meta::Regex;
use serde_json::Value;
use std::rc::Rc;
use std::time::Instant;

pub(crate) type R = u16;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum TRel {
    Ss,
    Sc,
    Sl,
    Id,
}

#[derive(Clone, Debug)]
pub(crate) enum Opd {
    Reg(R),
    Num(i64),
    Sym(u32),
    Syms(Box<[u32]>),
    Nums(Box<[i64]>),
    File,
}

#[derive(Clone, Debug)]
pub(crate) enum Val {
    Reg(R),
    Syms(Box<[u32]>),
    Nums(Box<[i64]>),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum T {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    In,
    NotIn,
    Matches,
    Glob,
    NotGlob,
    Starts,
    Ends,
    /// navwide: `numeric>=` (numbers and canonical numeric text) and `natural<` (natural text order)
    NumGe,
    NatLt,
    /// `integer-successor`: the right side is the left plus one (export README correction)
    Succ,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Dir {
    Parent,
    Child,
    Ancestor,
    Descendant,
}

#[derive(Debug)]
pub(crate) enum Ins {
    IterKind { kinds: Box<[u32]>, out: R },
    IterValue { rel: TRel, col: u8, val: Val, out: R },
    IterChildren { parent: R, out: R },
    IterScan { rel: TRel, out: R },
    IterFact { rel: u16, col: u8, val: Val, out: R },
    IterFactScan { rel: u16, out: R },
    Row { rel: TRel, key: R },
    Read { rel: TRel, col: u8, row: R, out: R },
    ReadFact { rel: u16, col: u8, row: R, out: R },
    Test { test: T, l: Opd, r: Opd, text: bool, num: bool, regex: Box<[usize]> },
    Axis { dir: Dir, start: R, out: R },
    /// `anc` is a proper ancestor of `desc` (both bound): one walk up the parent chain
    Within { anc: R, desc: R },
    /// a value bucket of a tree relation walked in start order between bounds (load-time fusion of IterValue, the read of the
    /// row's start and the range tests on it): binary search to the lower bound, stop at the upper
    IterRange { rel: TRel, col: u8, val: Val, out: R, start: R, lo: Option<(R, bool)>, hi: Option<(R, bool)> },
    /// the children of `parent` under one field (load-time fusion of IterChildren, the field read and its equality test)
    IterField { parent: R, field: u32, out: R, field_out: R },
    Near { start: R, out: R, subs: Box<[Prog]> },
    Sub { negate: bool, subs: Box<[Prog]> },
    /// the core row of rule `rule` (u32::MAX: the running made table's slot)
    Emit { cols: Box<[Opd]>, rule: u32 },
    /// shared-prefix fan-out: each child runs when one of its (rule, branch) slots is enabled in this file
    Fork { subs: Box<[(Box<[u32]>, Prog)]> },
    // ---- load-time specialisations (never stored in the pack): direct column reads and constant tests
    /// node column read (kind 0, text 1, start 2, stop 3 of the view; any id in range)
    ReadCol { which: u8, row: R, out: R },
    /// semantic-child read of a node with a parent: parent 0, ordinal 1, field 2
    ReadChild { which: u8, row: R, out: R },
    TestEqC { reg: R, c: u64 },
    TestNeC { reg: R, c: u64 },
    TestEqR { a: R, b: R },
    TestNeR { a: R, b: R },
    TestInC { reg: R, set: Box<[u32]>, negate: bool },
    /// a loop-invariant inner enumeration: `seg` (ending in Capture) runs once per file per value of `key` and its captured
    /// tuples are replayed (registers `caps` restored) into the rest of the program
    Hoist { id: u32, key: Box<[R]>, seg: Prog, caps: Box<[R]> },
    /// pushes the registers' values onto the running hoist's capture buffer
    Capture { regs: Box<[R]> },
    /// a Hoist whose replay keeps only the tuples whose captured register `probe.0` equals register `probe.1` (an index on it,
    /// built with the tuples once per file per key)
    HoistProbe { id: u32, key: Box<[R]>, seg: Prog, caps: Box<[R]>, probe: (u32, R) },
    /// a negated check whose failure holds for every later element of the range walk that encloses it (monotone in the walk's
    /// start: its only use of the walk is an upper bound on a walk of the same bucket): failing ends that walk
    SubFirst { subs: Box<[Prog]> },
    Yield,
    SpanBetween { a: Opd, b: Opd, out: R },
    SpanBounds { s: Opd, start: R, end: R },
    SourceMatches { span: Opd, base: Option<Opd>, regex: usize, group: Option<usize>, outs: Box<[R]>, capture_text: bool, capture_line: bool },
    /// transitive closure from the node in `src`, one or more edges, each edge a sub-program run with `xin` bound that collects
    /// the reached node (and the partition keys when they are not bound before); per (keys, node) distinct; `out` and `kout`
    /// receive each reached node and its keys
    Closure { src: R, out: R, xin: R, kout: Box<[R]>, nkeys: usize, subs: Box<[Prog]> },
    /// a closure edge sub-program's answer: the registers are pushed to the collect buffer
    Collect { regs: Box<[R]> },
    /// a union as one step: each sub-program collects the free output registers; the distinct tuples continue the program
    Union { outs: Box<[R]>, subs: Box<[Prog]>, memo: bool },
    /// a register takes a value (an equality whose one side nothing else binds)
    Let { from: Opd, out: R },
    /// runs entry `idx` of the shared sub-program table in place (a sub-program used more than once in a rule)
    Shared { idx: u32 },
    /// `concat` / `lower` of texts into a minted text
    TextFn { lower: bool, args: Box<[Opd]>, out: R },
    /// RyoQL `capture`: every split of the text by the slot pattern, the slots' texts into `outs`
    CaptureText { text: Opd, pattern: Box<str>, outs: Box<[R]> },
    /// RyoQL `replace`: the text with every regex match replaced by the literal
    Replace { text: Opd, regex: usize, replacement: Box<str>, out: R },
    /// nearest ancestor over derived edges: breadth-first up the edge sub-programs (each collects the reached node with
    /// `xin` bound) until a level holds a node the domain sub-programs admit; the first such node goes on
    NearDerived { start: R, out: R, xin: R, edges: Box<[Prog]>, domain: Box<[Prog]> },
    /// the next (or previous) sibling in the tree, when there is one
    Sibling { next: bool, start: R, out: R },
    /// an absence/existence test reading no candidate register: answered once per file
    SubMemo { negate: bool, subs: Box<[Prog]> },
    // ---- load-time memo specialisations (join; never stored in the pack)
    /// Near whose sub-programs read no register but the ancestor: the nearest proper ancestor per start node, memoised per file
    /// (every node passed on the way up gets the same answer)
    NearMemo { start: R, out: R, subs: Box<[Prog]> },
    /// an absence/existence check whose sub-programs read one outer register: answered once per file per value of `key`
    SubKey { negate: bool, key: R, subs: Box<[Prog]> },
    /// an absence/existence check "some proper ancestor `out` of `start` passes `cond`" whose condition reads only `out`:
    /// answered per node per file, every node on the way up sharing the answer
    AncExists { negate: bool, start: R, out: R, cond: Prog },
}

/// a made table answered by an ordered match of two inputs' rows (navwide)
pub(crate) struct OrderedMatch {
    pub left: Vec<Prog>,
    pub right: Vec<Prog>,
    pub lw: usize,
    pub rw: usize,
    pub left_on: Vec<usize>,
    pub right_on: Vec<usize>,
    pub lo: usize,
    pub ro: usize,
    pub number: bool,
    pub output: Vec<(bool, usize)>,
    /// navwide: an ordered depth-first walk instead (roots = left, edges = right)
    pub dfs: Option<OrderedDfs>,
}

/// the columns of an ordered depth-first walk (navwide)
pub(crate) struct OrderedDfs {
    pub root: usize,
    pub source: usize,
    pub destination: usize,
    pub order: usize,
    pub edge: usize,
    pub root_keys: Vec<usize>,
    pub edge_keys: Vec<usize>,
}

pub(crate) type Prog = Box<[Ins]>;

pub(crate) const NINS: usize = 47;
pub(crate) const INS_NAMES: [&str; NINS] = [
    "iter_kind", "iter_value", "iter_children", "iter_scan", "iter_fact", "iter_fact_scan", "row", "read", "read_fact", "test", "axis",
    "near", "sub", "emit", "yield", "span_between", "span_bounds", "source_matches", "within", "iter_range", "iter_field", "fork", "read_col", "read_child",
    "test_eq_c", "test_ne_c", "test_eq_r", "test_ne_r", "test_in_c", "hoist", "capture", "sub_first",
    "collect", "closure", "union", "let", "shared", "text_fn", "capture_text", "replace", "near_derived", "sibling", "sub_memo",
    "near_memo", "sub_key", "anc_exists", "hoist_probe",
];

impl Ins {
    #[inline]
    pub(crate) fn code(&self) -> usize {
        match self {
            Ins::IterKind { .. } => 0,
            Ins::IterValue { .. } => 1,
            Ins::IterChildren { .. } => 2,
            Ins::IterScan { .. } => 3,
            Ins::IterFact { .. } => 4,
            Ins::IterFactScan { .. } => 5,
            Ins::Row { .. } => 6,
            Ins::Read { .. } => 7,
            Ins::ReadFact { .. } => 8,
            Ins::Test { .. } => 9,
            Ins::Axis { .. } => 10,
            Ins::Near { .. } => 11,
            Ins::Sub { .. } => 12,
            Ins::Emit { .. } => 13,
            Ins::Yield => 14,
            Ins::SpanBetween { .. } => 15,
            Ins::SpanBounds { .. } => 16,
            Ins::SourceMatches { .. } => 17,
            Ins::Within { .. } => 18,
            Ins::IterRange { .. } => 19,
            Ins::IterField { .. } => 20,
            Ins::Fork { .. } => 21,
            Ins::ReadCol { .. } => 22,
            Ins::ReadChild { .. } => 23,
            Ins::TestEqC { .. } => 24,
            Ins::TestNeC { .. } => 25,
            Ins::TestEqR { .. } => 26,
            Ins::TestNeR { .. } => 27,
            Ins::TestInC { .. } => 28,
            Ins::Hoist { .. } => 29,
            Ins::Capture { .. } => 30,
            Ins::SubFirst { .. } => 31,
            Ins::Collect { .. } => 32,
            Ins::Closure { .. } => 33,
            Ins::Union { .. } => 34,
            Ins::Let { .. } => 35,
            Ins::Shared { .. } => 36,
            Ins::TextFn { .. } => 37,
            Ins::CaptureText { .. } => 38,
            Ins::Replace { .. } => 39,
            Ins::NearDerived { .. } => 40,
            Ins::Sibling { .. } => 41,
            Ins::SubMemo { .. } => 42,
            Ins::NearMemo { .. } => 43,
            Ins::SubKey { .. } => 44,
            Ins::AncExists { .. } => 45,
            Ins::HoistProbe { .. } => 46,
        }
    }
}

/// Every sub-program of an instruction (Fork children and a hoisted segment included), mutably.
pub(crate) fn subs_mut(i: &mut Ins, f: &mut dyn FnMut(&mut Prog)) {
    match i {
        Ins::Near { subs, .. }
        | Ins::Sub { subs, .. }
        | Ins::SubFirst { subs }
        | Ins::Closure { subs, .. }
        | Ins::Union { subs, .. }
        | Ins::SubMemo { subs, .. }
        | Ins::NearMemo { subs, .. }
        | Ins::SubKey { subs, .. } => subs.iter_mut().for_each(f),
        Ins::NearDerived { edges, domain, .. } => {
            edges.iter_mut().for_each(&mut *f);
            domain.iter_mut().for_each(f);
        }
        Ins::Fork { subs } => subs.iter_mut().for_each(|(_, q)| f(q)),
        Ins::Hoist { seg, .. } | Ins::HoistProbe { seg, .. } => f(seg),
        Ins::AncExists { cond, .. } => f(cond),
        _ => {}
    }
}

/// Every sub-program of an instruction, read-only.
pub(crate) fn subs_ref<'x>(i: &'x Ins, f: &mut dyn FnMut(&'x [Ins])) {
    match i {
        Ins::Near { subs, .. }
        | Ins::Sub { subs, .. }
        | Ins::SubFirst { subs }
        | Ins::Closure { subs, .. }
        | Ins::Union { subs, .. }
        | Ins::SubMemo { subs, .. }
        | Ins::NearMemo { subs, .. }
        | Ins::SubKey { subs, .. } => subs.iter().for_each(|q| f(q)),
        Ins::NearDerived { edges, domain, .. } => {
            edges.iter().for_each(|q| f(q));
            domain.iter().for_each(|q| f(q));
        }
        Ins::Fork { subs } => subs.iter().for_each(|(_, q)| f(q)),
        Ins::Hoist { seg, .. } | Ins::HoistProbe { seg, .. } => f(seg),
        Ins::AncExists { cond, .. } => f(cond),
        _ => {}
    }
}

thread_local! {
    /// registers each entry of the shared sub-program table reads (for reads_defs on Ins::Shared; set by specialize_all)
    static SHARED_READS: std::cell::RefCell<Vec<Vec<R>>> = const { std::cell::RefCell::new(Vec::new()) };
}

pub(crate) struct Alt {
    pub prog: Prog,
    /// the anchor's kinds when the program starts with a kind bucket
    pub anchor: Option<Box<[u32]>>,
    pub est: f64,
    pub per: f64,
}

pub(crate) struct NavRule {
    pub id: String,
    /// branches, each with its anchor alternatives (chosen per file by the anchor bucket's size)
    pub programs: Vec<Vec<Alt>>,
    pub xmerged: u32,
    pub kinds: Vec<u8>,
    pub nregs: usize,
    /// navwide: relations the tail reads that its interpreter does not serve: (merged id, scan programs, cell kinds)
    pub aux: Vec<(u32, Vec<Prog>, Vec<u8>)>,
}

pub(crate) struct FactRel {
    /// the model relation that serves it (index into semrows::RELATIONS); None: the provider's reader
    pub model: Option<usize>,
    pub name: String,
    pub relation: Option<ryo_relations::domain::relations::types::Relation>,
    pub kinds: Vec<ryo_relations::domain::relations::types::CellKind>,
    /// a made table (group count): its input's programs, the number of key columns, and whether it counts distinct values
    pub made: Option<(Vec<Prog>, Vec<usize>, Option<usize>, usize)>,
    /// navwide: a made table answered by an ordered match
    pub made_om: Option<OrderedMatch>,
    /// a keyed twin: its programs enumerate the input rows whose output key column `.0` equals register `.1` (set before
    /// the run); rows are filled per key on first demand
    pub keyed: Option<(u8, R)>,
    /// a keyed twin's base (unkeyed) table, u16::MAX when none: a file whose keyed builds cost more than building the base once
    /// switches to the base (ski rental)
    pub base: u16,
    /// registers its programs use (0..nregs): saved and restored around a build inside a running program
    pub nregs: usize,
}

/// Programs merged on their shared prefixes (the alpha network): one per anchor kind set (or one for every program without a kind
/// anchor), covering the (rule, branch) slots listed.
pub(crate) struct Group {
    pub kinds: Option<Box<[u32]>>,
    pub slots: Box<[u32]>,
    pub prog: Prog,
}

pub(crate) struct NavProgram {
    pub groups: Vec<Group>,
    /// (rule, branch) -> slot
    pub slot_of: Vec<Vec<u32>>,
    pub slot_rule: Vec<u32>,
    pub nslots: usize,
    pub rules: Vec<NavRule>,
    /// shared sub-programs of every rule (navwide)
    pub shared: Vec<Prog>,
    pub facts: Vec<FactRel>,
    pub regexes: Vec<Regex>,
    /// each regex's pattern (the compiled pack stores these)
    pub regex_patterns: Vec<String>,
    pub max_regs: usize,
    /// navwide: whether any relation of the program is served only by the old provider's readers
    pub needs_provider: bool,
}

fn trel(name: &str) -> Option<TRel> {
    Some(match name {
        "semantic-syntax" | "syntax-tree" => TRel::Ss,
        "semantic-child" | "syntax-child" => TRel::Sc,
        "source-location" => TRel::Sl,
        "identifier" => TRel::Id,
        _ => return None,
    })
}

pub(crate) fn cell_kind(name: &str) -> u8 {
    match name {
        "node" => 0,
        "symbol" => 1,
        "file" => 2,
        "span" => 3,
        "number" => 4,
        _ => 5,
    }
}

struct Loader<'a> {
    sym: &'a mut Symbols,
    regs: FxMap<i64, R>,
    kinds: &'a serde_json::Map<String, Value>,
    facts: &'a mut Vec<FactRel>,
    regexes: &'a mut Vec<Regex>,
    patterns: &'a mut Vec<String>,
    regex_of: &'a mut FxMap<(String, bool), usize>,
    /// a made table's name -> the fact index of the identical made table already loaded (shared across rules)
    alias: &'a mut FxMap<String, u16>,
    /// the rule whose programs are loading (u32::MAX while its made tables load)
    rule: u32,
    /// navwide's shared sub-program table of the rule loading (inlined at load: register maps differ per context; the
    /// identical sub-programs are shared again after register canonicalisation, `reshare`)
    shared_json: &'a [Value],
}

pub(crate) fn glob_to_regex(glob: &str) -> String {
    let mut out = String::from("^");
    let chars: Vec<char> = glob.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '*' if chars.get(i + 1) == Some(&'*') => {
                out.push_str(".*");
                i += 1;
                if chars.get(i + 1) == Some(&'/') {
                    out.push_str("/?");
                    i += 1;
                }
            }
            '*' => out.push_str("[^/]*"),
            '?' => out.push_str("[^/]"),
            c => {
                if "\\.+*?()|[]{}^$#&-~".contains(c) {
                    out.push('\\');
                }
                out.push(c);
            }
        }
        i += 1;
    }
    out.push('$');
    out
}

impl Loader<'_> {
    fn reg(&mut self, v: &Value) -> R {
        let k = v.as_i64().expect("var");
        let n = self.regs.len() as R;
        *self.regs.entry(k).or_insert(n)
    }

    fn var_kind(&self, v: &Value) -> &str {
        v.as_i64().and_then(|k| self.kinds.get(&k.to_string())).and_then(Value::as_str).unwrap_or("node")
    }

    fn opd(&mut self, v: &Value) -> Opd {
        if v.is_i64() || v.is_u64() {
            return Opd::Reg(self.reg(v));
        }
        let a = v.as_array().expect("const");
        let tag = a[0].as_str().unwrap();
        match tag {
            "t" => match a[1].as_str() {
                Some(s) => Opd::Sym(self.sym.base(s)),
                None => Opd::Sym(NULL),
            },
            "n" => Opd::Num(a[1].as_i64().unwrap()),
            "f" => Opd::File,
            "ts" => {
                let mut v: Vec<u32> = a[1].as_array().unwrap().iter().map(|x| x.as_str().map_or(NULL, |s| self.sym.base(s))).collect();
                v.sort_unstable();
                Opd::Syms(v.into())
            }
            "ns" => {
                let mut v: Vec<i64> = a[1].as_array().unwrap().iter().map(|x| x.as_i64().unwrap()).collect();
                v.sort_unstable();
                Opd::Nums(v.into())
            }
            t => panic!("const {t}"),
        }
    }

    fn val(&mut self, v: &Value) -> Val {
        match self.opd(v) {
            Opd::Reg(r) => Val::Reg(r),
            Opd::Sym(s) => Val::Syms(vec![s].into()),
            Opd::Syms(s) => Val::Syms(s),
            Opd::Num(n) => Val::Nums(vec![n].into()),
            Opd::Nums(n) => Val::Nums(n),
            Opd::File => Val::Nums(vec![0].into()),
        }
    }

    fn fact(&mut self, name: &str) -> u16 {
        if let Some(&i) = self.alias.get(name) {
            return i;
        }
        if let Some(i) = self.facts.iter().position(|f| f.name == name) {
            return i as u16;
        }
        use ryo_relations::domain::labels::types::NameRef;
        use ryo_relations::domain::relations::service::{kinds, relation_named};
        if name.starts_with("made:") {
            panic!("made table {name} used before its definition");
        }
        let relation = relation_named(NameRef(name)).unwrap_or_else(|| panic!("relation {name}"));
        let model = if std::env::var("NAV_PROVIDER_ONLY").is_ok() { None } else { crate::semrows::RELATIONS.iter().position(|r| *r == name) };
        self.facts.push(FactRel { model, name: name.to_owned(), relation: Some(relation), kinds: kinds(relation).to_vec(), made: None, made_om: None, keyed: None, base: u16::MAX, nregs: 0 });
        (self.facts.len() - 1) as u16
    }

    fn regex(&mut self, text: &str, glob: bool) -> usize {
        if let Some(&i) = self.regex_of.get(&(text.to_owned(), glob)) {
            return i;
        }
        let pattern = if glob { glob_to_regex(text) } else { text.to_owned() };
        self.regexes.push(Regex::new(&pattern).unwrap_or_else(|e| panic!("regex {pattern}: {e}")));
        self.patterns.push(pattern.clone());
        self.regex_of.insert((text.to_owned(), glob), self.regexes.len() - 1);
        self.regexes.len() - 1
    }

    fn prog(&mut self, p: &Value) -> Prog {
        let a = p.as_array().unwrap();
        if a.len() == 1 && a[0][0] == "shared" {
            let sj = self.shared_json;
            return self.prog(&sj[a[0][1].as_u64().unwrap() as usize]);
        }
        fuse(p.as_array().unwrap().iter().map(|i| self.ins(i)).collect::<Vec<_>>()).into()
    }

    fn ins(&mut self, i: &Value) -> Ins {
        let a = i.as_array().unwrap();
        match a[0].as_str().unwrap() {
            "iter" => {
                let rel = a[1].as_str().unwrap();
                let kind = a[2].as_str().unwrap();
                let out = self.reg(&a[4]);
                if let Some(t) = trel(rel) {
                    match kind {
                        "kind" => {
                            let Opd::Syms(k) = (match self.opd(&a[3][1]) {
                                Opd::Sym(s) => Opd::Syms(vec![s].into()),
                                o => o,
                            }) else {
                                panic!("kind literal")
                            };
                            Ins::IterKind { kinds: k, out }
                        }
                        "value" => {
                            let col = a[3][0].as_u64().unwrap() as u8;
                            let val = self.val(&a[3][1]);
                            Ins::IterValue { rel: t, col, val, out }
                        }
                        "children" => Ins::IterChildren { parent: self.reg(&a[3]), out },
                        "scan" => Ins::IterScan { rel: t, out },
                        k => panic!("iter {k}"),
                    }
                } else {
                    let f = self.fact(rel);
                    match kind {
                        "by" => {
                            let col = a[3][0].as_u64().unwrap() as u8;
                            let val = self.val(&a[3][1]);
                            Ins::IterFact { rel: f, col, val, out }
                        }
                        "scan" => Ins::IterFactScan { rel: f, out },
                        k => panic!("iter fact {k}"),
                    }
                }
            }
            "row" => Ins::Row { rel: trel(a[1].as_str().unwrap()).expect("row rel"), key: self.reg(&a[2]) },
            "read" => {
                let rel = a[1].as_str().unwrap();
                let col = a[2].as_u64().unwrap() as u8;
                let row = self.reg(&a[3]);
                let out = self.reg(&a[4]);
                match trel(rel) {
                    Some(t) => Ins::Read { rel: t, col, row, out },
                    None => {
                        let f = self.fact(rel);
                        Ins::ReadFact { rel: f, col, row, out }
                    }
                }
            }
            "test" => {
                let test = match a[1].as_str().unwrap() {
                    "=" => T::Eq,
                    "!=" => T::Ne,
                    "<" => T::Lt,
                    "<=" => T::Le,
                    ">" => T::Gt,
                    ">=" => T::Ge,
                    "in" => T::In,
                    "not-in" => T::NotIn,
                    "matches" => T::Matches,
                    "glob-any" => T::Glob,
                    "not-glob-any" => T::NotGlob,
                    "starts-with" => T::Starts,
                    "ends-with" => T::Ends,
                    "numeric>=" => T::NumGe,
                    "natural<" => T::NatLt,
                    "integer-successor" => T::Succ,
                    t => panic!("test {t}"),
                };
                let kl = if a[2].is_i64() { self.var_kind(&a[2]).to_owned() } else { String::new() };
                let kr = if a[3].is_i64() { self.var_kind(&a[3]).to_owned() } else { String::new() };
                let l = self.opd(&a[2]);
                let r = self.opd(&a[3]);
                let text = kl == "symbol" || kr == "symbol" || matches!(l, Opd::Sym(_)) || matches!(r, Opd::Sym(_));
                let num = kl == "number" || kr == "number" || kl == "flag" || kr == "flag" || matches!(l, Opd::Num(_)) || matches!(r, Opd::Num(_));
                let mut regex = Vec::new();
                if matches!(test, T::Matches | T::Glob | T::NotGlob) {
                    let glob = test != T::Matches;
                    let texts: Vec<String> = match &a[3] {
                        Value::Array(c) if c[0] == "t" => vec![c[1].as_str().unwrap_or("").to_owned()],
                        Value::Array(c) if c[0] == "ts" => c[1].as_array().unwrap().iter().map(|x| x.as_str().unwrap_or("").to_owned()).collect(),
                        _ => panic!("regex operand"),
                    };
                    for t in texts {
                        regex.push(self.regex(&t, glob));
                    }
                }
                Ins::Test { test, l, r, text, num, regex: regex.into() }
            }
            "axis" => {
                let dir = match a[1].as_str().unwrap() {
                    "parent" => Dir::Parent,
                    "child" => Dir::Child,
                    "ancestor" => Dir::Ancestor,
                    "descendant" => Dir::Descendant,
                    d => panic!("axis {d}"),
                };
                Ins::Axis { dir, start: self.reg(&a[2]), out: self.reg(&a[3]) }
            }
            "within" => Ins::Within { anc: self.reg(&a[1]), desc: self.reg(&a[2]) },
            "near" => {
                let start = self.reg(&a[1]);
                let out = self.reg(&a[2]);
                let subs = a[3].as_array().unwrap().iter().map(|p| self.prog(p)).collect::<Vec<_>>().into();
                Ins::Near { start, out, subs }
            }
            "neg" | "exists" => {
                let negate = a[0] == "neg";
                let subs = a[1].as_array().unwrap().iter().map(|p| self.prog(p)).collect::<Vec<_>>().into();
                Ins::Sub { negate, subs }
            }
            "emit" => Ins::Emit { cols: a[1].as_array().unwrap().iter().map(|c| self.opd(c)).collect::<Vec<_>>().into(), rule: self.rule },
            "yield" => Ins::Yield,
            "fn" => {
                let args: Vec<Opd> = a[2].as_array().unwrap().iter().map(|x| self.opd(x)).collect();
                let outs: Vec<R> = a[3].as_array().unwrap().iter().map(|x| self.reg(x)).collect();
                match a[1].as_str().unwrap() {
                    "span_between" => Ins::SpanBetween { a: args[0].clone(), b: args[1].clone(), out: outs[0] },
                    "span_bounds" => Ins::SpanBounds { s: args[0].clone(), start: outs[0], end: outs[1] },
                    "source_matches" => {
                        let p = &a[4];
                        let regex = self.regex(p["pattern"].as_str().unwrap(), false);
                        let group = p["capture"]
                            .as_str()
                            .and_then(|name| self.regexes[regex].group_info().to_index(regex_automata::PatternID::ZERO, name));
                        Ins::SourceMatches {
                            span: args[0].clone(),
                            base: args.get(1).cloned(),
                            regex,
                            group,
                            outs: outs.into(),
                            capture_text: p["capture_text"].as_bool().unwrap_or(false),
                            capture_line: p["capture_line"].as_bool().unwrap_or(false),
                        }
                    }
                    "text" => Ins::TextFn { lower: a[4]["function"] == "lower", args: args.into(), out: outs[0] },
                    "capture" => Ins::CaptureText { text: args[0].clone(), pattern: a[4]["pattern"].as_str().unwrap().into(), outs: outs.into() },
                    "replace" => Ins::Replace {
                        text: args[0].clone(),
                        regex: self.regex(a[4]["pattern"].as_str().unwrap(), false),
                        replacement: a[4]["replacement"].as_str().unwrap().into(),
                        out: outs[0],
                    },
                    f => panic!("fn {f}"),
                }
            }
            "clos" => self.closure_ins(a),
            "shared" => panic!("a shared sub-program reference is a whole sub-program"),
            "nearc" => Ins::NearDerived {
                start: self.reg(&a[1]),
                out: self.reg(&a[2]),
                xin: self.reg(&a[3]),
                edges: a[5].as_array().unwrap().iter().map(|p| self.prog(p)).collect::<Vec<_>>().into(),
                domain: a[6].as_array().unwrap().iter().map(|p| self.prog(p)).collect::<Vec<_>>().into(),
            },
            "neg-memo" | "exists-memo" => Ins::SubMemo {
                negate: a[0] == "neg-memo",
                subs: a[1].as_array().unwrap().iter().map(|p| self.prog(p)).collect::<Vec<_>>().into(),
            },
            "sib" => Ins::Sibling { next: a[1].as_bool().unwrap(), start: self.reg(&a[2]), out: self.reg(&a[3]) },
            "let" => Ins::Let { from: self.opd(&a[1]), out: self.reg(&a[2]) },
            "or" => Ins::Union {
                outs: a[1].as_array().unwrap().iter().map(|x| self.reg(x)).collect::<Vec<_>>().into(),
                subs: a[2].as_array().unwrap().iter().map(|p| self.prog(p)).collect::<Vec<_>>().into(),
                memo: a.len() > 3,
            },
            "collect" => Ins::Collect { regs: a[1].as_array().unwrap().iter().map(|x| self.reg(x)).collect::<Vec<_>>().into() },
            o => panic!("ins {o}"),
        }
    }

    /// `["clos", src, out, xin, yout, kcol, kout, progs]` (navwide)
    fn closure_ins(&mut self, a: &[Value]) -> Ins {
        let src = self.reg(&a[1]);
        let out = self.reg(&a[2]);
        let xin = self.reg(&a[3]);
        let _yout = self.reg(&a[4]);
        let nkeys = a[5].as_array().unwrap().len();
        let kout = a[6].as_array().unwrap().iter().map(|x| self.reg(x)).collect::<Vec<_>>().into();
        let subs = a[7].as_array().unwrap().iter().map(|p| self.prog(p)).collect::<Vec<_>>().into();
        Ins::Closure { src, out, xin, kout, nkeys, subs }
    }
}

/// Load-time specialisation (NAV_SPEC=0 turns it off; applied after the pack is read, never stored): reads of the view's
/// own columns and constant/equality tests become direct instructions.
pub(crate) fn specialize(p: Prog) -> Prog {
    if std::env::var("NAV_SPEC").is_ok_and(|v| v == "0") {
        return p;
    }
    p.into_vec()
        .into_iter()
        .map(|i| match i {
            Ins::Read { rel: TRel::Ss, col: 1, row, out } => Ins::ReadCol { which: 0, row, out },
            Ins::Read { rel: TRel::Ss, col: 2, row, out } => Ins::ReadCol { which: 1, row, out },
            Ins::Read { rel: TRel::Sl, col: 5, row, out } => Ins::ReadCol { which: 2, row, out },
            Ins::Read { rel: TRel::Sl, col: 6, row, out } => Ins::ReadCol { which: 3, row, out },
            Ins::Read { rel: TRel::Sc, col: 0, row, out } => Ins::ReadChild { which: 0, row, out },
            Ins::Read { rel: TRel::Sc, col: 2, row, out } => Ins::ReadChild { which: 1, row, out },
            Ins::Read { rel: TRel::Sc, col: 3, row, out } => Ins::ReadChild { which: 2, row, out },
            Ins::Test { test: T::Eq, l: Opd::Reg(a), r: Opd::Reg(b), .. } => Ins::TestEqR { a, b },
            Ins::Test { test: T::Ne, l: Opd::Reg(a), r: Opd::Reg(b), .. } => Ins::TestNeR { a, b },
            Ins::Test { test: T::Eq, l: Opd::Reg(reg), r: Opd::Sym(c), .. } => Ins::TestEqC { reg, c: u64::from(c) },
            Ins::Test { test: T::Ne, l: Opd::Reg(reg), r: Opd::Sym(c), .. } => Ins::TestNeC { reg, c: u64::from(c) },
            Ins::Test { test: T::Eq, l: Opd::Reg(reg), r: Opd::Num(c), .. } => Ins::TestEqC { reg, c: c as u64 },
            Ins::Test { test: T::Ne, l: Opd::Reg(reg), r: Opd::Num(c), .. } => Ins::TestNeC { reg, c: c as u64 },
            Ins::Test { test: T::In, l: Opd::Reg(reg), r: Opd::Syms(set), .. } => Ins::TestInC { reg, set, negate: false },
            Ins::Test { test: T::NotIn, l: Opd::Reg(reg), r: Opd::Syms(set), .. } => Ins::TestInC { reg, set, negate: true },
            mut other => {
                subs_mut(&mut other, &mut |q| *q = specialize(std::mem::take(q)));
                other
            }
        })
        .collect::<Vec<_>>()
        .into()
}

/// Registers an instruction reads (sub-programs: every register they read, an over-approximation) and defines (top level).
fn reads_defs(i: &Ins, reads: &mut Vec<R>, defs: &mut Vec<R>) {
    let o = |x: &Opd, r: &mut Vec<R>| {
        if let Opd::Reg(x) = x {
            r.push(*x);
        }
    };
    let v = |x: &Val, r: &mut Vec<R>| {
        if let Val::Reg(x) = x {
            r.push(*x);
        }
    };
    let sub = |ps: &[Prog], r: &mut Vec<R>| {
        for q in ps {
            for j in q.iter() {
                let mut d = Vec::new();
                reads_defs(j, r, &mut d);
            }
        }
    };
    match i {
        Ins::IterKind { out, .. } | Ins::IterScan { out, .. } | Ins::IterFactScan { out, .. } => defs.push(*out),
        Ins::IterValue { val, out, .. } | Ins::IterFact { val, out, .. } => {
            v(val, reads);
            defs.push(*out);
        }
        Ins::IterChildren { parent, out } => {
            reads.push(*parent);
            defs.push(*out);
        }
        Ins::Row { key, .. } => reads.push(*key),
        Ins::Read { row, out, .. } | Ins::ReadFact { row, out, .. } | Ins::ReadCol { row, out, .. } | Ins::ReadChild { row, out, .. } => {
            reads.push(*row);
            defs.push(*out);
        }
        Ins::Test { l, r, .. } => {
            o(l, reads);
            o(r, reads);
        }
        Ins::TestEqC { reg, .. } | Ins::TestNeC { reg, .. } | Ins::TestInC { reg, .. } => reads.push(*reg),
        Ins::TestEqR { a, b } | Ins::TestNeR { a, b } => {
            reads.push(*a);
            reads.push(*b);
        }
        Ins::Axis { start, out, .. } => {
            reads.push(*start);
            defs.push(*out);
        }
        Ins::Within { anc, desc } => {
            reads.push(*anc);
            reads.push(*desc);
        }
        Ins::IterRange { val, out, start, lo, hi, .. } => {
            v(val, reads);
            if let Some((r, _)) = lo {
                reads.push(*r);
            }
            if let Some((r, _)) = hi {
                reads.push(*r);
            }
            defs.push(*out);
            defs.push(*start);
        }
        Ins::IterField { parent, out, field_out, .. } => {
            reads.push(*parent);
            defs.push(*out);
            defs.push(*field_out);
        }
        Ins::Near { start, out, subs } => {
            reads.push(*start);
            sub(subs, reads);
            defs.push(*out);
        }
        Ins::Sub { subs, .. } => sub(subs, reads),
        Ins::Fork { subs } => {
            for (_, q) in subs.iter() {
                sub(std::slice::from_ref(q), reads);
            }
        }
        Ins::Emit { cols, .. } => cols.iter().for_each(|c| o(c, reads)),
        Ins::Yield => {}
        Ins::SpanBetween { a, b, out } => {
            o(a, reads);
            o(b, reads);
            defs.push(*out);
        }
        Ins::SpanBounds { s, start, end } => {
            o(s, reads);
            defs.push(*start);
            defs.push(*end);
        }
        Ins::SourceMatches { span, base, outs, .. } => {
            o(span, reads);
            if let Some(b) = base {
                o(b, reads);
            }
            defs.extend(outs.iter().copied());
        }
        Ins::Hoist { key, seg, caps, .. } => {
            reads.extend(key.iter().copied());
            sub(std::slice::from_ref(seg), reads);
            defs.extend(caps.iter().copied());
        }
        Ins::HoistProbe { key, seg, caps, probe, .. } => {
            reads.extend(key.iter().copied());
            reads.push(probe.1);
            sub(std::slice::from_ref(seg), reads);
            defs.extend(caps.iter().copied());
        }
        Ins::Capture { regs } => reads.extend(regs.iter().copied()),
        Ins::SubFirst { subs } => sub(subs, reads),
        Ins::Collect { regs } => reads.extend(regs.iter().copied()),
        Ins::Closure { src, out, xin, kout, subs, .. } => {
            reads.push(*src);
            sub(subs, reads);
            reads.retain(|r| r != xin);
            defs.push(*out);
            defs.extend(kout.iter().copied());
        }
        Ins::Union { outs, subs, .. } => {
            sub(subs, reads);
            defs.extend(outs.iter().copied());
        }
        Ins::Let { from, out } => {
            o(from, reads);
            defs.push(*out);
        }
        Ins::Shared { idx } => SHARED_READS.with(|t| {
            let t = t.borrow();
            assert!((*idx as usize) < t.len(), "shared reads not computed");
            reads.extend(t[*idx as usize].iter().copied());
        }),
        Ins::TextFn { args, out, .. } => {
            args.iter().for_each(|a| o(a, reads));
            defs.push(*out);
        }
        Ins::CaptureText { text, outs, .. } => {
            o(text, reads);
            defs.extend(outs.iter().copied());
        }
        Ins::Replace { text, out, .. } => {
            o(text, reads);
            defs.push(*out);
        }
        Ins::NearDerived { start, out, xin, edges, domain } => {
            reads.push(*start);
            sub(edges, reads);
            sub(domain, reads);
            reads.retain(|r| r != xin && r != out);
            defs.push(*out);
            defs.push(*xin);
        }
        Ins::Sibling { start, out, .. } => {
            reads.push(*start);
            defs.push(*out);
        }
        Ins::SubMemo { subs, .. } => sub(subs, reads),
        Ins::NearMemo { start, out, subs } => {
            reads.push(*start);
            sub(subs, reads);
            defs.push(*out);
        }
        Ins::SubKey { key, subs, .. } => {
            reads.push(*key);
            sub(subs, reads);
        }
        Ins::AncExists { start, out, cond, .. } => {
            reads.push(*start);
            sub(std::slice::from_ref(cond), reads);
            reads.retain(|r| r != out);
        }
    }
}

thread_local! {
    /// the free registers of each shared entry (read and not defined inside it; set by specialize_all)
    static SHARED_FREE: std::cell::RefCell<Vec<Vec<R>>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Registers a program reads that nothing inside it defines (every nested sub-program included; a shared entry contributes its
/// own free registers). Over-approximates what the program needs from outside.
pub(crate) fn free_regs(p: &[Ins]) -> Vec<R> {
    fn walk(p: &[Ins], reads: &mut Vec<R>, defs: &mut Vec<R>) {
        for i in p {
            if let Ins::Shared { idx } = i {
                SHARED_FREE.with(|t| reads.extend(t.borrow()[*idx as usize].iter().copied()));
                continue;
            }
            let mut r = Vec::new();
            reads_defs_top(i, &mut r, defs);
            reads.extend(r);
            subs_ref(i, &mut |q| walk(q, reads, defs));
        }
    }
    let (mut reads, mut defs) = (Vec::new(), Vec::new());
    walk(p, &mut reads, &mut defs);
    defs.sort_unstable();
    defs.dedup();
    reads.sort_unstable();
    reads.dedup();
    reads.retain(|r| defs.binary_search(r).is_err());
    reads
}

/// reads_defs without the sub-programs' reads (free_regs walks those itself); a closure's or nearest-derived walk's input
/// register counts as defined (the step sets it before its sub-programs run)
fn reads_defs_top(i: &Ins, reads: &mut Vec<R>, defs: &mut Vec<R>) {
    let mut has_subs = false;
    subs_ref(i, &mut |_| has_subs = true);
    if !has_subs {
        reads_defs(i, reads, defs);
        return;
    }
    match i {
        Ins::Near { start, out, .. } | Ins::NearMemo { start, out, .. } => {
            reads.push(*start);
            defs.push(*out);
        }
        Ins::Closure { src, out, xin, kout, .. } => {
            reads.push(*src);
            defs.push(*out);
            defs.push(*xin);
            defs.extend(kout.iter().copied());
        }
        Ins::Union { outs, .. } => defs.extend(outs.iter().copied()),
        Ins::NearDerived { start, out, xin, .. } => {
            reads.push(*start);
            defs.push(*out);
            defs.push(*xin);
        }
        Ins::SubKey { key, .. } => reads.push(*key),
        Ins::AncExists { start, out, .. } => {
            reads.push(*start);
            defs.push(*out);
        }
        Ins::Hoist { key, caps, .. } => {
            reads.extend(key.iter().copied());
            defs.extend(caps.iter().copied());
        }
        Ins::HoistProbe { key, caps, probe, .. } => {
            reads.extend(key.iter().copied());
            reads.push(probe.1);
            defs.extend(caps.iter().copied());
        }
        _ => {}
    }
}

/// Memo specialisations (NAV_MEMO=0 turns them off): a nearest-ancestor walk whose condition reads only the ancestor becomes
/// NearMemo; an absence/existence check reading no outer register becomes a per-file SubMemo, one reading exactly one becomes
/// SubKey (per file per value).
/// [[Axis Ancestor start -> out, cond...]] whose cond reads no register but `out` (and what it defines): (start, out)
fn anc_shape(subs: &[Prog]) -> Option<(R, R)> {
    let [q] = subs else { return None };
    let Some(Ins::Axis { dir: Dir::Ancestor, start, out }) = q.first() else { return None };
    let free = free_regs(&q[1..]);
    free.iter().all(|r| r == out).then_some((*start, *out))
}

/// Detached components (NAV_DETACH_LOAD=0 turns it off): in a program run for existence (the sub-programs of absence and
/// existence checks, nearest-ancestor conditions, union and closure sub-programs, whose consumers keep sets), a run of steps that
/// shares no register with the steps after it (but registers bound before it) is its own existence check: its loops no longer
/// multiply the work of what follows. The planner's order is kept.
pub(crate) fn detach(p: Prog) -> Prog {
    if std::env::var("NAV_DETACH_LOAD").is_ok_and(|v| v == "0") {
        return p;
    }
    // a program ending in Yield is run for existence wherever it is used (shared entries included)
    detach_rec(p, true)
}

fn detach_rec(p: Prog, exist: bool) -> Prog {
    let mut v: Vec<Ins> = p.into_vec();
    for i in v.iter_mut() {
        let sub_exist = matches!(
            i,
            Ins::Sub { .. } | Ins::SubMemo { .. } | Ins::Near { .. } | Ins::Union { .. } | Ins::Closure { .. } | Ins::NearDerived { .. }
        );
        subs_mut(i, &mut |q| *q = detach_rec(std::mem::take(q), sub_exist));
    }
    if !exist || v.len() < 3 || !matches!(v.last(), Some(Ins::Yield)) {
        return v.into();
    }
    let body = v.len() - 1;
    // reads[c..body] per suffix (over-approximate), defs per step
    let mut rd: Vec<Vec<R>> = Vec::with_capacity(body);
    let mut df: Vec<Vec<R>> = Vec::with_capacity(body);
    for i in &v[..body] {
        let (mut r, mut d) = (Vec::new(), Vec::new());
        reads_defs(i, &mut r, &mut d);
        rd.push(r);
        df.push(d);
    }
    let mut suffix: Vec<std::collections::HashSet<R>> = vec![Default::default(); body + 1];
    for c in (0..body).rev() {
        let mut s = suffix[c + 1].clone();
        s.extend(rd[c].iter().copied());
        suffix[c] = s;
    }
    // chunk boundaries: c where nothing at or after c reads what [start, c) defined
    let mut chunks: Vec<(usize, usize)> = Vec::new();
    let mut start = 0;
    let mut defined: Vec<R> = Vec::new();
    for c in 0..body {
        defined.extend(df[c].iter().copied());
        if !defined.iter().any(|r| suffix[c + 1].contains(r)) {
            chunks.push((start, c + 1));
            start = c + 1;
            defined.clear();
        }
    }
    if start < body {
        chunks.push((start, body));
    }
    let iterating = |a: usize, b: usize, v: &[Ins]| v[a..b].iter().any(is_iter);
    if chunks.len() < 2 || chunks.iter().filter(|&&(a, b)| iterating(a, b, &v)).count() == 0 {
        return v.into();
    }
    // every chunk with a loop becomes an existence check (a single trailing chunk that is the whole loop stays inline)
    let n_iter = chunks.iter().filter(|&&(a, b)| iterating(a, b, &v)).count();
    let mut out: Vec<Ins> = Vec::with_capacity(v.len());
    let mut it = v.into_iter();
    let mut pos = 0;
    let mut wrapped = 0;
    for (a, b) in chunks {
        let seg: Vec<Ins> = (a..b).map(|_| it.next().unwrap()).collect();
        pos += b - a;
        let loops = seg.iter().any(is_iter);
        if loops && (n_iter > 1 || wrapped == 0 && pos < body) {
            let mut q = seg;
            q.push(Ins::Yield);
            out.push(Ins::Sub { negate: false, subs: vec![q.into_boxed_slice()].into() });
            wrapped += 1;
        } else {
            out.extend(seg);
        }
    }
    out.extend(it);
    out.into()
}

/// Memoised checks sink (NAV_SINK=0 turns it off): a SubKey or AncExists moves down its straight run to just before the run's
/// end (an emit, yield, fork, collect, capture, hoist or shared step), past every step that does not redefine what it reads.
/// Filters commute; past a fan-out the memo answers the repeats (its key was bound before the fan-out), past a filter it runs
/// for fewer candidates.
pub(crate) fn sink(p: Prog) -> Prog {
    if std::env::var("NAV_SINK").is_ok_and(|v| v == "0") {
        return p;
    }
    let mut v: Vec<Ins> = p.into_vec();
    for i in v.iter_mut() {
        subs_mut(i, &mut |q| *q = sink(std::mem::take(q)));
    }
    let stop = |i: &Ins| {
        matches!(i, Ins::Emit { .. } | Ins::Yield | Ins::Fork { .. } | Ins::Collect { .. } | Ins::Capture { .. } | Ins::Hoist { .. } | Ins::HoistProbe { .. } | Ins::Shared { .. })
    };
    let mut k = v.len();
    while k > 0 {
        k -= 1;
        if !matches!(v[k], Ins::SubKey { .. } | Ins::AncExists { .. }) {
            continue;
        }
        let mut reads = Vec::new();
        reads_defs_top(&v[k], &mut reads, &mut Vec::new());
        let mut j = k + 1;
        while j < v.len() && !stop(&v[j]) {
            let mut d = Vec::new();
            reads_defs_top(&v[j], &mut Vec::new(), &mut d);
            if d.iter().any(|r| reads.contains(r)) {
                break;
            }
            j += 1;
        }
        if j < v.len() && j > k + 1 {
            let x = v.remove(k);
            v.insert(j - 1, x);
        }
    }
    v.into()
}

pub(crate) fn memoize(p: Prog) -> Prog {
    if std::env::var("NAV_MEMO").is_ok_and(|v| v == "0") {
        return p;
    }
    memo_rec(p).0
}

/// memoize, bottom-up: returns the program and its free registers (reads not defined in the same program level or inside the
/// sub-program that reads them: a superset of free_regs, so a memo it allows is sound)
fn memo_rec(p: Prog) -> (Prog, Vec<R>) {
    let memo0 = !std::env::var("NAV_MEMO0").is_ok_and(|v| v == "0");
    let mut reads: Vec<R> = Vec::new();
    let mut defs: Vec<R> = Vec::new();
    let out: Vec<Ins> = p
        .into_vec()
        .into_iter()
        .map(|mut i| {
            if let Ins::Shared { idx } = &i {
                SHARED_FREE.with(|t| reads.extend(t.borrow()[*idx as usize].iter().copied()));
                return i;
            }
            let mut child: Vec<R> = Vec::new();
            subs_mut(&mut i, &mut |q| {
                let (q2, f) = memo_rec(std::mem::take(q));
                *q = q2;
                child.extend(f);
            });
            child.sort_unstable();
            child.dedup();
            let i = match i {
                Ins::Near { start, out, subs } => {
                    if child.iter().all(|r| *r == out) { Ins::NearMemo { start, out, subs } } else { Ins::Near { start, out, subs } }
                }
                Ins::Sub { negate, subs } if anc_shape(&subs).is_some() && !std::env::var("NAV_ANC").is_ok_and(|v| v == "0") => {
                    let (start, out) = anc_shape(&subs).unwrap();
                    let mut v = subs.into_vec();
                    let q = v.pop().unwrap();
                    let cond: Prog = q.into_vec().into_iter().skip(1).collect::<Vec<_>>().into();
                    Ins::AncExists { negate, start, out, cond }
                }
                Ins::Sub { negate, subs } => match child.len() {
                    0 if memo0 => Ins::SubMemo { negate, subs },
                    1 => Ins::SubKey { negate, key: child[0], subs },
                    _ => Ins::Sub { negate, subs },
                },
                other => other,
            };
            let mut r = Vec::new();
            reads_defs_top(&i, &mut r, &mut defs);
            reads.extend(r);
            // a closure's / nearest-derived walk's input register and a Near's output are set by the step itself
            match &i {
                Ins::Closure { xin, .. } | Ins::NearDerived { xin, .. } => child.retain(|r| r != xin),
                _ => {}
            }
            if let Ins::Near { out, .. } | Ins::NearMemo { out, .. } | Ins::NearDerived { out, .. } = &i {
                child.retain(|r| r != out);
            }
            reads.extend(child);
            i
        })
        .collect();
    defs.sort_unstable();
    defs.dedup();
    reads.sort_unstable();
    reads.dedup();
    reads.retain(|r| defs.binary_search(r).is_err());
    (out.into(), reads)
}

/// Constant folding from bucket walks (NAV_FOLD=0 turns it off): a row reached through a single-value bucket of a tree column
/// has that value, so a read of that column is known and an equality or membership test on it always holds (dropped).
pub(crate) fn fold(p: Prog, known: &FxMap<R, u64>) -> Prog {
    if std::env::var("NAV_FOLD").is_ok_and(|v| v == "0") {
        return p;
    }
    let mut known = known.clone();
    // row register -> (view column: 0 kind 1 text, value)
    let mut bucket: FxMap<R, (u8, u64)> = FxMap::default();
    let one = |val: &Val| match val {
        Val::Syms(s) if s.len() == 1 => Some(u64::from(s[0])),
        _ => None,
    };
    let mut out: Vec<Ins> = Vec::new();
    for i in p.into_vec() {
        match &i {
            Ins::IterValue { rel: TRel::Ss, col, val, out: o } | Ins::IterRange { rel: TRel::Ss, col, val, out: o, .. } if *col == 1 || *col == 2 => {
                if let Some(c) = one(val) {
                    bucket.insert(*o, (*col - 1, c));
                }
            }
            Ins::IterKind { kinds, out: o } if kinds.len() == 1 => {
                bucket.insert(*o, (0, u64::from(kinds[0])));
            }
            Ins::IterField { field, field_out, .. } => {
                known.insert(*field_out, u64::from(*field));
            }
            Ins::ReadCol { which, row, out: x } if *which <= 1 => {
                if let Some(&(c, v)) = bucket.get(row) {
                    if c == *which {
                        known.insert(*x, v);
                    }
                }
            }
            Ins::TestEqC { reg, c } if known.get(reg) == Some(c) => continue,
            Ins::TestInC { reg, set, negate: false } if known.get(reg).is_some_and(|&k| k <= u64::from(u32::MAX) && set.binary_search(&(k as u32)).is_ok()) => continue,
            _ => {}
        }
        let mut i = i;
        subs_mut(&mut i, &mut |q| *q = fold(std::mem::take(q), &known));
        out.push(i);
    }
    out.into()
}

/// Marks first-element checks (NAV_FIRST=0 turns it off): after a range walk with no upper bound, a negated single
/// sub-program in the same straight run whose last step before Yield walks the same bucket below the walk's start, and which
/// reads nothing else bound by the walk or after it, becomes SubFirst.
pub(crate) fn mark_first(p: Prog) -> Prog {
    if std::env::var("NAV_FIRST").is_ok_and(|v| v == "0") {
        return p;
    }
    let mut v: Vec<Ins> = p
        .into_vec()
        .into_iter()
        .map(|mut i| {
            subs_mut(&mut i, &mut |q| *q = mark_first(std::mem::take(q)));
            i
        })
        .collect();
    for i in 0..v.len() {
        let (oval, ost, oout) = match &v[i] {
            Ins::IterRange { val, start, out, hi: None, .. } => (format!("{val:?}"), *start, *out),
            _ => continue,
        };
        // registers bound by the walk or after it, up to the check
        let mut bound_after: Vec<R> = vec![ost, oout];
        let mut j = i + 1;
        while j < v.len() {
            if is_iter(&v[j]) || matches!(v[j], Ins::Fork { .. } | Ins::Emit { .. } | Ins::Hoist { .. } | Ins::HoistProbe { .. } | Ins::Yield) {
                break;
            }
            let ok = match &v[j] {
                Ins::Sub { negate: true, subs } if subs.len() == 1 => {
                    let q = &subs[0];
                    let n = q.len();
                    n >= 2
                        && matches!(q[n - 1], Ins::Yield)
                        && matches!(&q[n - 2], Ins::IterRange { val, hi: Some((h, false)), lo, .. }
                            if format!("{val:?}") == oval && *h == ost && lo.is_none_or(|(l, _)| !bound_after.contains(&l)))
                        && q[..n - 2].iter().all(|k| {
                            let (mut r, mut d) = (Vec::new(), Vec::new());
                            reads_defs(k, &mut r, &mut d);
                            r.iter().all(|x| !bound_after.contains(x))
                        })
                }
                _ => false,
            };
            if !ok && std::env::var_os("NAV_FIRST_DEBUG").is_some() {
                if let Ins::Sub { negate: true, subs } = &v[j] {
                    let q = &subs[0];
                    let n = q.len();
                    let bad: Vec<R> = q[..n.saturating_sub(2)]
                        .iter()
                        .flat_map(|k| {
                            let (mut r, mut d) = (Vec::new(), Vec::new());
                            reads_defs(k, &mut r, &mut d);
                            r
                        })
                        .filter(|x| bound_after.contains(x))
                        .collect();
                    eprintln!("first? subs {} last {:?} bad {:?} bound {:?}", subs.len(), q.get(n.wrapping_sub(2)).map(|i| format!("{i:?}").chars().take(120).collect::<String>()), bad, bound_after);
                }
            }
            if ok {
                let Ins::Sub { subs, .. } = std::mem::replace(&mut v[j], Ins::Yield) else { unreachable!() };
                v[j] = Ins::SubFirst { subs };
                break;
            }
            let (mut r, mut d) = (Vec::new(), Vec::new());
            reads_defs(&v[j], &mut r, &mut d);
            bound_after.extend(d);
            j += 1;
        }
    }
    v.into()
}

fn is_iter(i: &Ins) -> bool {
    matches!(
        i,
        Ins::IterKind { .. }
            | Ins::IterValue { .. }
            | Ins::IterChildren { .. }
            | Ins::IterScan { .. }
            | Ins::IterFact { .. }
            | Ins::IterFactScan { .. }
            | Ins::IterRange { .. }
            | Ins::IterField { .. }
            | Ins::Axis { dir: Dir::Ancestor | Dir::Descendant | Dir::Child, .. }
            | Ins::SourceMatches { .. }
            | Ins::Closure { .. }
            | Ins::Union { .. }
            | Ins::CaptureText { .. }
    )
}

thread_local! {
    static HOIST_ANY: bool = std::env::var("NAV_HOIST_ANY").is_ok_and(|v| v == "1");
}

static HOIST_ID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// Loop-invariant inner enumerations (NAV_HOIST=0 turns it off): an iteration whose inputs were all bound before an
/// enclosing iteration of the same program, together with the steps after it that read nothing else bound outside it, runs
/// once per file per value of those inputs; its result tuples are replayed into the rest.
pub(crate) fn hoist(p: Prog) -> Prog {
    if std::env::var("NAV_HOIST").is_ok_and(|v| v == "0") {
        return p;
    }
    hoist_rec(p, false)
}

/// the registers an instruction needs from outside: its own reads plus its sub-programs' free registers (not what the step
/// itself sets for them)
fn ins_free_reads(i: &Ins) -> Vec<R> {
    let mut has = false;
    subs_ref(i, &mut |_| has = true);
    let (mut r, mut d) = (Vec::new(), Vec::new());
    if !has {
        reads_defs(i, &mut r, &mut d);
        return r;
    }
    reads_defs_top(i, &mut r, &mut d);
    let own: Vec<R> = match i {
        Ins::Closure { xin, .. } | Ins::NearDerived { xin, .. } => vec![*xin],
        Ins::Near { out, .. } | Ins::NearMemo { out, .. } | Ins::AncExists { out, .. } => vec![*out],
        _ => Vec::new(),
    };
    subs_ref(i, &mut |q| r.extend(free_regs(q).into_iter().filter(|x| !own.contains(x))));
    if let Ins::NearDerived { out, .. } = i {
        r.retain(|x| x != out);
    }
    r
}

thread_local! {
    static SKI: bool = !std::env::var("NAV_SKI").is_ok_and(|v| v == "0");
    static PROVIDER_ALL: bool = std::env::var_os("NAV_PROVIDER").is_some();
    static HOIST_CONST: bool = !std::env::var("NAV_HOIST_CONST").is_ok_and(|v| v == "0");
    static HOIST_DEFER: bool = !std::env::var("NAV_HOIST_DEFER").is_ok_and(|v| v == "0");
    static HOIST_PROBE: bool = !std::env::var("NAV_HOIST_PROBE").is_ok_and(|v| v == "0");
}

fn is_test(i: &Ins) -> bool {
    matches!(i, Ins::Test { .. } | Ins::TestEqC { .. } | Ins::TestNeC { .. } | Ins::TestEqR { .. } | Ins::TestNeR { .. } | Ins::TestInC { .. })
}

fn hoist_rec(p: Prog, nested: bool) -> Prog {
    let mut v: Vec<Ins> = p.into_vec();
    // recurse first
    for i in v.iter_mut() {
        let sub_nested = !matches!(i, Ins::Fork { .. }) || nested;
        subs_mut(i, &mut |q| *q = hoist_rec(std::mem::take(q), sub_nested));
    }
    let mut def_at: FxMap<R, usize> = FxMap::default();
    let mut s = 0;
    while s < v.len() {
        let mut reads = Vec::new();
        let mut defs = Vec::new();
        // the registers an iteration needs from outside (its sub-programs' free registers); definitions are top level
        reads_defs_top(&v[s], &mut reads, &mut defs);
        if is_iter(&v[s]) {
            reads = ins_free_reads(&v[s]);
        }
        let full = if is_iter(&v[s]) { reads.clone() } else { Vec::new() };
        // a constant iteration (no register read) inside a loop or a repeatedly run sub-program: once per file (NAV_HOIST_CONST)
        let konst = is_iter(&v[s]) && full.is_empty() && HOIST_CONST.with(|h| *h) && (nested || (0..s).any(|j| is_iter(&v[j])));
        let candidate = konst
            || is_iter(&v[s]) && !reads.is_empty() && reads.iter().all(|r| def_at.contains_key(r)) && {
                let mut k = reads.clone();
                k.sort_unstable();
                k.dedup();
                k.len() <= 2
            };
        if candidate {
            let d = reads.iter().filter_map(|r| def_at.get(r)).copied().max().unwrap_or(0);
            // inside a loop whose body re-derives the same key (an iteration after the key's binding), or (NAV_HOIST_ANY) any
            // loop at all: the runtime turns a site's memo off when its keys do not repeat
            // a hoist's replay is a loop too
            let is_loop = |i: &Ins| is_iter(i) || matches!(i, Ins::Hoist { .. } | Ins::HoistProbe { .. });
            let enclosing = konst || (d + 1..s).any(|j| is_loop(&v[j])) || (HOIST_ANY.with(|h| *h) && (0..s).any(|j| is_loop(&v[j])));
            if enclosing {
                // grow the segment while each step reads only the key or what the segment defined
                let key: Vec<R> = if konst {
                    Vec::new()
                } else {
                    let mut k = reads.clone();
                    k.sort_unstable();
                    k.dedup();
                    k
                };
                let mut local: Vec<R> = defs.clone();
                let mut t = s + 1;
                // tests reading something bound outside the segment are deferred past it (filters commute; NAV_HOIST_DEFER)
                let mut deferred: Vec<usize> = Vec::new();
                let mut inside: Vec<usize> = vec![s];
                while t < v.len() {
                    if matches!(v[t], Ins::Emit { .. } | Ins::Yield | Ins::Fork { .. } | Ins::Hoist { .. } | Ins::HoistProbe { .. } | Ins::Collect { .. } | Ins::Shared { .. }) {
                        break;
                    }
                    let r2 = ins_free_reads(&v[t]);
                    if !r2.iter().all(|r| key.contains(r) || local.contains(r)) {
                        if is_test(&v[t]) && HOIST_DEFER.with(|h| *h) {
                            deferred.push(t);
                            t += 1;
                            continue;
                        }
                        break;
                    }
                    let (mut r3, mut d2) = (Vec::new(), Vec::new());
                    reads_defs_top(&v[t], &mut r3, &mut d2);
                    local.extend(d2);
                    inside.push(t);
                    t += 1;
                }
                // deferred tests at the segment's end only count when the segment still has its own steps after the first
                if inside.len() >= 3 && t < v.len() {
                    // captured: segment definitions read later (deferred tests included)
                    let mut later = Vec::new();
                    for j in deferred.iter().map(|&j| &v[j]).chain(v[t..].iter()) {
                        let mut d3 = Vec::new();
                        reads_defs(j, &mut later, &mut d3);
                    }
                    let mut caps: Vec<R> = local.iter().copied().filter(|r| later.contains(r)).collect();
                    caps.sort_unstable();
                    caps.dedup();
                    // an equality between a captured register and one bound outside: the replay probes an index on it
                    let probe = if HOIST_PROBE.with(|h| *h) {
                        deferred.iter().find_map(|&j| match &v[j] {
                            Ins::TestEqR { a, b } | Ins::Test { test: T::Eq, l: Opd::Reg(a), r: Opd::Reg(b), .. } => {
                                match (caps.iter().position(|c| c == a), caps.iter().position(|c| c == b)) {
                                    (Some(i), None) if !local.contains(b) => Some((i as u32, *b)),
                                    (None, Some(i)) if !local.contains(a) => Some((i as u32, *a)),
                                    _ => None,
                                }
                            }
                            _ => None,
                        })
                    } else {
                        None
                    };
                    let mut taken: Vec<Option<Ins>> = v.drain(s..t).map(Some).collect();
                    let mut seg: Vec<Ins> = inside.iter().map(|&j| taken[j - s].take().unwrap()).collect();
                    let tests: Vec<Ins> = deferred.iter().map(|&j| taken[j - s].take().unwrap()).collect();
                    seg.push(Ins::Capture { regs: caps.clone().into() });
                    let id = HOIST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    for r in &caps {
                        def_at.insert(*r, s);
                    }
                    let n_tests = tests.len();
                    let h = match probe {
                        Some((pc, pr)) => Ins::HoistProbe { id, key: key.into(), seg: seg.into(), caps: caps.into(), probe: (pc, pr) },
                        None => Ins::Hoist { id, key: key.into(), seg: seg.into(), caps: caps.into() },
                    };
                    v.splice(s..s, std::iter::once(h).chain(tests));
                    s += 1 + n_tests;
                    continue;
                }
            }
        }
        for r in defs {
            def_at.entry(r).or_insert(s);
        }
        s += 1;
    }
    v.into()
}

impl NavProgram {
    /// the final load step: specialise every program the executor runs
    pub(crate) fn specialize_all(&mut self) {
        let none = FxMap::default();
        // the registers each shared entry reads (entries only reference entries before them)
        SHARED_READS.with(|t| t.borrow_mut().clear());
        for q in &self.shared {
            let mut reads = Vec::new();
            for i in q.iter() {
                let mut d = Vec::new();
                // a shared entry is a whole sub-program: its own definitions stay inside (over-approximation as for subs)
                reads_defs(i, &mut reads, &mut d);
            }
            reads.sort_unstable();
            reads.dedup();
            SHARED_READS.with(|t| t.borrow_mut().push(reads));
        }
        // the shared entries' free registers (hoisting reads them; recomputed before the memo pass)
        SHARED_FREE.with(|t| t.borrow_mut().clear());
        for q in &self.shared {
            let fr = free_regs(q);
            SHARED_FREE.with(|t| t.borrow_mut().push(fr));
        }
        // each pass over every program, timed (NAV_VERBOSE)
        let stages: [(&str, &dyn Fn(Prog) -> Prog); 7] = [
            ("specialise", &specialize),
            ("detach", &detach),
            ("fold", &|q| fold(q, &none)),
            ("hoist", &hoist),
            ("first", &mark_first),
            ("memo", &memoize),
            ("sink", &sink),
        ];
        for (name, f) in stages {
            let t = Instant::now();
            let pass = |q: &mut Prog| *q = f(std::mem::take(q));
            if name == "memo" {
                // memo specialisation reads the shared entries' free registers (entries only reference entries before them)
                SHARED_FREE.with(|t| t.borrow_mut().clear());
                for q in &self.shared {
                    let fr = free_regs(q);
                    SHARED_FREE.with(|t| t.borrow_mut().push(fr));
                }
            }
            for g in &mut self.groups {
                pass(&mut g.prog);
            }
            for q in &mut self.shared {
                pass(q);
            }
            for fr in &mut self.facts {
                if let Some((progs, ..)) = &mut fr.made {
                    progs.iter_mut().for_each(pass);
                }
                if let Some(om) = &mut fr.made_om {
                    om.left.iter_mut().for_each(pass);
                    om.right.iter_mut().for_each(pass);
                }
            }
            for r in &mut self.rules {
                for (_, ps, _) in &mut r.aux {
                    ps.iter_mut().for_each(pass);
                }
            }
            if std::env::var("NAV_VERBOSE").is_ok() {
                eprintln!("pass {name}: {:.3} s", t.elapsed().as_secs_f64());
            }
        }
    }

    /// Identical sub-programs shared again (NAV_RESHARE=0 turns it off): after registers are canonical per program, every
    /// sub-program is hash-consed (children first); one used twice or more and at least NAV_RESHARE_MIN instructions long becomes
    /// an entry of the shared table, run in place by Ins::Shared; the others are inlined back.
    fn reshare(&mut self) {
        if std::env::var("NAV_RESHARE").is_ok_and(|v| v == "0") {
            return;
        }
        let min: usize = std::env::var("NAV_RESHARE_MIN").ok().and_then(|v| v.parse().ok()).unwrap_or(24);
        let mut hc = Hc::default();
        let mut all: Vec<&mut Prog> = Vec::new();
        for g in &mut self.groups {
            all.push(&mut g.prog);
        }
        for f in &mut self.facts {
            if let Some((progs, ..)) = &mut f.made {
                all.extend(progs.iter_mut());
            }
            if let Some(om) = &mut f.made_om {
                all.extend(om.left.iter_mut());
                all.extend(om.right.iter_mut());
            }
        }
        for r in &mut self.rules {
            for (_, ps, _) in &mut r.aux {
                all.extend(ps.iter_mut());
            }
        }
        for q in all.iter_mut() {
            hc.cons(q);
        }
        let before: usize = hc.size.iter().zip(&hc.uses).map(|(s, u)| *s * *u as usize).sum();
        let mut fin: FxMap<u32, u32> = FxMap::default();
        let mut table: Vec<Prog> = Vec::new();
        for q in all.iter_mut() {
            hc.expand(q, min, &mut fin, &mut table);
        }
        if std::env::var("NAV_VERBOSE").is_ok() {
            let after: usize = table.iter().map(|q| count_ins(q)).sum::<usize>() + all.iter().map(|q| count_ins(q)).sum::<usize>();
            eprintln!("nav reshare: {} distinct sub-programs, {} shared entries; instructions (subs counted per use) {before} -> {after}", hc.progs.len(), table.len());
        }
        self.shared = table;
    }
}

/// Hash-consing of sub-programs (reshare).
#[derive(Default)]
struct Hc {
    ids: FxMap<String, u32>,
    progs: Vec<Option<Prog>>,
    uses: Vec<u32>,
    /// instructions with every child expanded
    size: Vec<usize>,
}

impl Hc {
    /// replaces every sub-program inside `p` (not Fork children: those are trie branches) by [Shared{id}] of its hash-consed entry
    fn cons(&mut self, p: &mut Prog) {
        for ins in p.iter_mut() {
            if let Ins::Fork { subs } = ins {
                for (_, q) in subs.iter_mut() {
                    self.cons(q);
                }
                continue;
            }
            subs_mut(ins, &mut |q| {
                self.cons(q);
                let key = format!("{q:?}");
                let id = match self.ids.get(&key) {
                    Some(&id) => id,
                    None => {
                        let id = self.progs.len() as u32;
                        let mut size = 0;
                        for i in q.iter() {
                            size += 1;
                            subs_ref(i, &mut |c| {
                                if let [Ins::Shared { idx }] = c {
                                    size += self.size[*idx as usize];
                                }
                            });
                        }
                        self.ids.insert(key, id);
                        self.progs.push(Some(std::mem::take(q)));
                        self.uses.push(0);
                        self.size.push(size);
                        id
                    }
                };
                self.uses[id as usize] += 1;
                *q = vec![Ins::Shared { idx: id }].into();
            });
        }
    }

    /// the program with each hash-consed reference either kept as a reference into the final table or inlined back
    fn expand(&mut self, p: &mut Prog, min: usize, fin: &mut FxMap<u32, u32>, table: &mut Vec<Prog>) {
        for ins in p.iter_mut() {
            if let Ins::Fork { subs } = ins {
                for (_, q) in subs.iter_mut() {
                    self.expand(q, min, fin, table);
                }
                continue;
            }
            subs_mut(ins, &mut |q| {
                let [Ins::Shared { idx }] = &q[..] else { unreachable!("every sub-program is consed") };
                let id = *idx as usize;
                if self.uses[id] >= 2 && self.size[id] >= min {
                    let k = match fin.get(&(id as u32)) {
                        Some(&k) => k,
                        None => {
                            let mut body: Prog = self.progs[id].as_ref().unwrap().iter().map(clone_ins).collect::<Vec<_>>().into();
                            self.expand(&mut body, min, fin, table);
                            table.push(body);
                            let k = (table.len() - 1) as u32;
                            fin.insert(id as u32, k);
                            k
                        }
                    };
                    *q = vec![Ins::Shared { idx: k }].into();
                } else {
                    let mut body: Prog = self.progs[id].as_ref().unwrap().iter().map(clone_ins).collect::<Vec<_>>().into();
                    self.expand(&mut body, min, fin, table);
                    *q = body;
                }
            });
        }
    }
}


/// Load-time fusion (NAV_FUSE=0 turns it off): a value bucket followed by the read of the row's start and range tests on it
/// becomes one range walk; children followed by the field read and an equality on it become a per-field walk.
/// Tests commute (NAV_TESTS_EARLY=0 turns this off): each test of a straight program moves up to just after the step that binds
/// its last operand (registers bound outside the program count as bound at its start).
pub(crate) fn tests_early(p: Vec<Ins>) -> Vec<Ins> {
    if std::env::var("NAV_TESTS_EARLY").is_ok_and(|v| v == "0") {
        return p;
    }
    let n = p.len();
    // target slot per instruction: tests go after their last operand's definition; everything else stays
    let mut def_at: FxMap<R, usize> = FxMap::default();
    let mut key: Vec<(usize, usize)> = Vec::with_capacity(n);
    for (i, ins) in p.iter().enumerate() {
        let (mut reads, mut defs) = (Vec::new(), Vec::new());
        reads_defs(ins, &mut reads, &mut defs);
        if matches!(ins, Ins::Test { .. }) {
            let t = reads.iter().filter_map(|r| def_at.get(r)).map(|&d| d + 1).max().unwrap_or(0);
            // (position, tie-break): placed right after the defining step, before the steps that followed it
            key.push((t.min(i), 0));
        } else {
            key.push((i, 1));
        }
        for d in defs {
            def_at.insert(d, i);
        }
    }
    let mut order: Vec<usize> = (0..n).collect();
    // a test moved to slot t sorts before the original instruction at t (tie 0 < 1); stable among equals
    order.sort_by_key(|&i| (key[i].0, key[i].1, i));
    let mut slots: Vec<Option<Ins>> = p.into_iter().map(Some).collect();
    order.into_iter().map(|i| slots[i].take().unwrap()).collect()
}

pub(crate) fn fuse(p: Vec<Ins>) -> Vec<Ins> {
    let p = tests_early(p);
    if std::env::var("NAV_FUSE").is_ok_and(|v| v == "0") {
        return p;
    }
    let mut out: Vec<Ins> = Vec::with_capacity(p.len());
    let mut it = p.into_iter().peekable();
    let mut rest: Vec<Ins> = Vec::new();
    while let Some(ins) = it.next() {
        rest.clear();
        match ins {
            Ins::IterValue { rel, col, val, out: o } if matches!(rel, TRel::Ss | TRel::Sl) => {
                // the straight run that follows: reads of the row (one of them its start) and tests, in any order
                let mut run: Vec<Ins> = Vec::new();
                while matches!(it.peek(), Some(Ins::Read { row, .. }) if *row == o) || matches!(it.peek(), Some(Ins::Test { .. })) {
                    run.push(it.next().unwrap());
                }
                let st = run.iter().find_map(|i| match i {
                    Ins::Read { rel: TRel::Sl, col: 5, row, out } if *row == o => Some(*out),
                    _ => None,
                });
                let Some(st) = st else {
                    out.push(Ins::IterValue { rel, col, val, out: o });
                    out.extend(run);
                    continue;
                };
                let (mut lo, mut hi) = (None, None);
                let mut rest: Vec<Ins> = Vec::new();
                for i in run {
                    let bound = match &i {
                        Ins::Test { test, l: Opd::Reg(a), r: Opd::Reg(b), text: false, .. } => match (*a == st, *b == st, *test) {
                            (true, false, T::Ge) => Some((true, (*b, true))),
                            (true, false, T::Gt) => Some((true, (*b, false))),
                            (true, false, T::Lt) => Some((false, (*b, false))),
                            (true, false, T::Le) => Some((false, (*b, true))),
                            (false, true, T::Le) => Some((true, (*a, true))),
                            (false, true, T::Lt) => Some((true, (*a, false))),
                            (false, true, T::Gt) => Some((false, (*a, false))),
                            (false, true, T::Ge) => Some((false, (*a, true))),
                            _ => None,
                        },
                        _ => None,
                    };
                    match bound {
                        // a bound register must be bound before the walk (not a read of this row)
                        Some((is_lo, (r, incl))) if !rest.iter().any(|x| matches!(x, Ins::Read { out, .. } if *out == r)) => {
                            let slot = if is_lo { &mut lo } else { &mut hi };
                            if slot.is_none() {
                                *slot = Some((r, incl));
                                continue;
                            }
                            rest.push(i);
                        }
                        _ => {
                            if !matches!(&i, Ins::Read { rel: TRel::Sl, col: 5, row, .. } if *row == o) {
                                rest.push(i);
                            }
                        }
                    }
                }
                out.push(Ins::IterRange { rel, col, val, out: o, start: st, lo, hi });
                out.extend(rest);
            }
            Ins::IterChildren { parent, out: o } => {
                // the straight run that follows: reads of the child (one its field) and tests (one an equality on the field)
                let mut run: Vec<Ins> = Vec::new();
                while matches!(it.peek(), Some(Ins::Read { row, .. }) if *row == o) || matches!(it.peek(), Some(Ins::Test { .. })) {
                    run.push(it.next().unwrap());
                }
                let fr = run.iter().find_map(|i| match i {
                    Ins::Read { rel: TRel::Sc, col: 3, row, out } if *row == o => Some(*out),
                    _ => None,
                });
                let ft = fr.and_then(|f| {
                    run.iter().position(|i| matches!(i, Ins::Test { test: T::Eq, l: Opd::Reg(a), r: Opd::Sym(_), .. } if *a == f))
                });
                match (fr, ft) {
                    (Some(f), Some(k)) => {
                        let Ins::Test { r: Opd::Sym(x), .. } = run.remove(k) else { unreachable!() };
                        run.retain(|i| !matches!(i, Ins::Read { rel: TRel::Sc, col: 3, row, .. } if *row == o));
                        out.push(Ins::IterField { parent, field: x, out: o, field_out: f });
                        out.extend(run);
                    }
                    _ => {
                        out.push(Ins::IterChildren { parent, out: o });
                        out.extend(run);
                    }
                }
            }
            other => out.push(other),
        }
    }
    out
}

impl NavProgram {
    pub(crate) fn load(rules: &[&Value], sym: &mut Symbols) -> Self {
        let mut facts = Vec::new();
        let mut regexes = Vec::new();
        let mut out = Vec::new();
        let mut max_regs = 0;
        let mut regex_of = FxMap::default();
        let mut patterns: Vec<String> = Vec::new();
        let mut alias: FxMap<String, u16> = FxMap::default();
        // canonical form of a made table (its programs with registers numbered from 0, keys, kinds) -> its fact index
        let mut canon: FxMap<String, u16> = FxMap::default();
        let share = !std::env::var("NAV_SHARE_MADE").is_ok_and(|v| v == "0");
        for r in rules {
            let nav = &r["nav"];
            let kinds = nav["kinds"].as_object().unwrap();
            let mut loader = Loader {
                sym,
                regs: FxMap::default(),
                kinds,
                facts: &mut facts,
                regexes: &mut regexes,
                patterns: &mut patterns,
                regex_of: &mut regex_of,
                alias: &mut alias,
                rule: u32::MAX,
                shared_json: nav["shared"].as_array().map(Vec::as_slice).unwrap_or(&[]),
            };
            for m in nav["made"].as_array().map(Vec::as_slice).unwrap_or(&[]) {
                use ryo_relations::domain::relations::types::CellKind;
                // each made table's programs number their registers from 0 (they run at top level; ensure() saves the caller's)
                loader.regs = FxMap::default();
                let progs: Vec<Prog> = m["programs"].as_array().unwrap().iter().map(|p| loader.prog(p)).collect();
                max_regs = max_regs.max(loader.regs.len());
                let kinds = m["kinds"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|k| match k.as_str().unwrap() {
                        "node" => CellKind::Node,
                        "symbol" => CellKind::Symbol,
                        "file" => CellKind::File,
                        "span" => CellKind::Span,
                        "flag" => CellKind::Flag,
                        _ => CellKind::Number,
                    })
                    .collect::<Vec<CellKind>>();
                if m["op"] == "ordered_dfs" {
                    let cols = |v: &Value| v.as_array().unwrap().iter().map(|k| k.as_u64().unwrap() as usize).collect::<Vec<_>>();
                    let right: Vec<Prog> = m["right_programs"].as_array().unwrap().iter().map(|p| loader.prog(p)).collect();
                    let n = |k: &str| m[k].as_u64().unwrap() as usize;
                    let om = OrderedMatch {
                        left: progs,
                        right,
                        lw: n("width"),
                        rw: n("right_width"),
                        left_on: Vec::new(),
                        right_on: Vec::new(),
                        lo: 0,
                        ro: 0,
                        number: m["order_number"].as_bool().unwrap_or(false),
                        output: Vec::new(),
                        dfs: Some(OrderedDfs {
                            root: n("root"),
                            source: n("source"),
                            destination: n("destination"),
                            order: n("order"),
                            edge: n("edge"),
                            root_keys: cols(&m["root_keys"]),
                            edge_keys: cols(&m["edge_keys"]),
                        }),
                    };
                    max_regs = max_regs.max(loader.regs.len());
                    loader.facts.push(FactRel { model: None, name: m["name"].as_str().unwrap().to_owned(), relation: None, kinds, made: None, made_om: Some(om), keyed: None, base: u16::MAX, nregs: loader.regs.len() });
                    continue;
                }
                if m["op"] == "ordered_match" {
                    let cols = |v: &Value| v.as_array().unwrap().iter().map(|k| k.as_u64().unwrap() as usize).collect::<Vec<_>>();
                    let right: Vec<Prog> = m["right_programs"].as_array().unwrap().iter().map(|p| loader.prog(p)).collect();
                    let om = OrderedMatch {
                        left: progs,
                        right,
                        lw: m["width"].as_u64().unwrap() as usize,
                        rw: m["right_width"].as_u64().unwrap() as usize,
                        left_on: cols(&m["left_on"]),
                        right_on: cols(&m["right_on"]),
                        lo: m["left_order"].as_u64().unwrap() as usize,
                        ro: m["right_order"].as_u64().unwrap() as usize,
                        number: m["order_number"].as_bool().unwrap_or(false),
                        output: m["output"].as_array().unwrap().iter().map(|o| (o[0].as_u64().unwrap() == 1, o[1].as_u64().unwrap() as usize)).collect(),
                        dfs: None,
                    };
                    max_regs = max_regs.max(loader.regs.len());
                    loader.facts.push(FactRel { model: None, name: m["name"].as_str().unwrap().to_owned(), relation: None, kinds, made: None, made_om: Some(om), keyed: None, base: u16::MAX, nregs: loader.regs.len() });
                    continue;
                }
                let name = m["name"].as_str().unwrap().to_owned();
                let keyed = m.get("param").map(|p| (m["keycol"].as_u64().unwrap() as u8, loader.reg(p)));
                let base = match (&keyed, name.rsplit_once('@')) {
                    (Some(_), Some((b, _))) => loader
                        .alias
                        .get(b)
                        .copied()
                        .or_else(|| loader.facts.iter().position(|f| f.name == b).map(|i| i as u16))
                        .unwrap_or(u16::MAX),
                    _ => u16::MAX,
                };
                max_regs = max_regs.max(loader.regs.len());
                let key = format!("{progs:?}|{}|{}|{}|{kinds:?}|{keyed:?}", m["keys"], m["distinct"], m["width"]);
                if share {
                    if let Some(&i) = canon.get(&key) {
                        loader.alias.insert(name, i);
                        continue;
                    }
                    canon.insert(key, loader.facts.len() as u16);
                }
                loader.facts.push(FactRel {
                    made_om: None,
                    model: None,
                    name,
                    relation: None,
                    kinds,
                    made: Some((
                        progs,
                        m["keys"].as_array().unwrap().iter().map(|k| k.as_u64().unwrap() as usize).collect(),
                        m["distinct"].as_u64().map(|d| d as usize),
                        m["width"].as_u64().unwrap() as usize,
                    )),
                    keyed,
                    base,
                    nregs: loader.regs.len(),
                });
            }
            loader.regs = FxMap::default();
            loader.rule = out.len() as u32;
            let programs: Vec<Vec<Alt>> = nav["programs"]
                .as_array()
                .unwrap()
                .iter()
                .map(|alts| {
                    alts.as_array()
                        .unwrap()
                        .iter()
                        .map(|a| {
                            let prog = loader.prog(&a["prog"]);
                            let anchor = if let Some(Ins::IterKind { kinds, .. }) = prog.first() { Some(kinds.clone()) } else { None };
                            Alt { prog, anchor, est: a["est"].as_f64().unwrap(), per: a["per"].as_f64().unwrap() }
                        })
                        .collect()
                })
                .collect();
            loader.rule = u32::MAX;
            let aux: Vec<(u32, Vec<Prog>, Vec<u8>)> = nav["aux"]
                .as_array()
                .map(Vec::as_slice)
                .unwrap_or(&[])
                .iter()
                .map(|a| {
                    let progs = a["programs"].as_array().unwrap().iter().map(|p| loader.prog(p)).collect();
                    let kinds = a["columns"].as_array().unwrap().iter().map(|c| cell_kind(c.as_str().unwrap())).collect();
                    (a["merged"].as_u64().unwrap() as u32, progs, kinds)
                })
                .collect();
            let nregs = loader.regs.len();
            max_regs = max_regs.max(nregs);
            out.push(NavRule {
                id: r["id"].as_str().unwrap().to_owned(),
                programs,
                xmerged: nav["xmerged"].as_u64().unwrap() as u32,
                kinds: nav["columns"].as_array().unwrap().iter().map(|c| cell_kind(c.as_str().unwrap())).collect(),
                nregs,
                aux,
            });
        }
        let needs_provider = std::env::var_os("NAV_PROVIDER").is_some()
            || facts.iter().any(|f| {
                f.model.is_none() && f.made.is_none() && f.made_om.is_none() && crate::derived::index_of(&f.name).is_none() && !f.name.starts_with("filesystem.")
            });
        let mut me = Self {
            groups: Vec::new(),
            slot_of: Vec::new(),
            slot_rule: Vec::new(),
            nslots: 0,
            rules: out,
            shared: Vec::new(),
            facts,
            regexes,
            regex_patterns: patterns,
            max_regs,
            needs_provider,
        };
        me.build_groups();
        me.reshare();
        me
    }

    /// Canonical registers per program (numbered by first appearance), then a trie over the programs of each anchor group.
    fn build_groups(&mut self) {
        let merge = !std::env::var("NAV_SHARE_PREFIX").is_ok_and(|v| v == "0");
        let mut slot = 0u32;
        let mut by_anchor: Vec<(Option<Box<[u32]>>, Vec<(u32, Vec<Ins>)>)> = Vec::new();
        let mut max_regs = self.max_regs;
        for (ri, r) in self.rules.iter_mut().enumerate() {
            let mut slots = Vec::new();
            for alts in &mut r.programs {
                slots.push(slot);
                self.slot_rule.push(ri as u32);
                let prog = std::mem::take(&mut alts[0].prog);
                let mut m: FxMap<R, R> = FxMap::default();
                let canon: Vec<Ins> = prog.into_vec().into_iter().map(|i| map_regs(i, &mut m)).collect();
                max_regs = max_regs.max(m.len());
                let key = alts[0].anchor.clone();
                let gi = if merge { by_anchor.iter().position(|(k, _)| *k == key) } else { None };
                match gi {
                    Some(g) => by_anchor[g].1.push((slot, canon)),
                    None => by_anchor.push((key, vec![(slot, canon)])),
                }
                slot += 1;
            }
            self.slot_of.push(slots);
        }
        self.max_regs = max_regs;
        self.nslots = slot as usize;
        let (mut before, mut after) = (0usize, 0usize);
        for (kinds, progs) in by_anchor {
            before += progs.iter().map(|(_, p)| count_ins(p)).sum::<usize>();
            let slots: Box<[u32]> = progs.iter().map(|(s, _)| *s).collect();
            let keyed: Vec<(u32, Vec<Ins>, Vec<String>)> = progs
                .into_iter()
                .map(|(s, p)| {
                    let k = p.iter().map(|i| format!("{i:?}")).collect();
                    (s, p, k)
                })
                .collect();
            let prog = trie(keyed, 0);
            after += count_ins(&prog);
            self.groups.push(Group { kinds, slots, prog: prog.into() });
        }
        if std::env::var("NAV_VERBOSE").is_ok() {
            eprintln!("nav groups: {} programs in {} groups; instructions {before} -> {after}", self.nslots, self.groups.len());
        }
    }
}

fn count_ins(p: &[Ins]) -> usize {
    p.iter()
        .map(|i| {
            let mut n = 1;
            subs_ref(i, &mut |q| n += count_ins(q));
            n
        })
        .sum()
}

/// A trie over programs from `depth`: the longest common prefix once, then a Fork over the groups that differ next.
fn trie(mut group: Vec<(u32, Vec<Ins>, Vec<String>)>, depth: usize) -> Vec<Ins> {
    if group.len() == 1 {
        let (_, p, _) = group.pop().unwrap();
        return p.into_iter().skip(depth).collect();
    }
    let min_len = group.iter().map(|g| g.1.len()).min().unwrap();
    let mut l = 0;
    while depth + l < min_len && group.iter().all(|g| g.2[depth + l] == group[0].2[depth + l]) {
        l += 1;
    }
    if depth + l >= min_len {
        // one program is a prefix of (or identical to) another: keep them apart from here
        let mut out: Vec<Ins> = Vec::new();
        let subs: Vec<(Box<[u32]>, Prog)> = group
            .into_iter()
            .map(|(s, p, _)| (vec![s].into_boxed_slice(), p.into_iter().skip(depth).collect::<Vec<_>>().into_boxed_slice()))
            .collect();
        out.push(Ins::Fork { subs: subs.into() });
        return out;
    }
    let mut out: Vec<Ins> = group[0].1[depth..depth + l].iter().map(clone_ins).collect();
    let d = depth + l;
    // partition by the next instruction, first-seen order
    let mut parts: Vec<Vec<(u32, Vec<Ins>, Vec<String>)>> = Vec::new();
    for g in group {
        match parts.iter().position(|p| p[0].2[d] == g.2[d]) {
            Some(i) => parts[i].push(g),
            None => parts.push(vec![g]),
        }
    }
    let subs: Vec<(Box<[u32]>, Prog)> = parts
        .into_iter()
        .map(|p| {
            let slots: Box<[u32]> = p.iter().map(|g| g.0).collect();
            (slots, trie(p, d).into_boxed_slice())
        })
        .collect();
    out.push(Ins::Fork { subs: subs.into() });
    out
}

fn clone_ins(i: &Ins) -> Ins {
    map_regs_ref(i, &mut |r| r)
}

/// Renumbers every register of `i` (and its sub-programs) through `m`, assigning the next number on first appearance.
fn map_regs(i: Ins, m: &mut FxMap<R, R>) -> Ins {
    map_regs_ref(&i, &mut |r| {
        let n = m.len() as R;
        *m.entry(r).or_insert(n)
    })
}

fn map_regs_ref(i: &Ins, f: &mut dyn FnMut(R) -> R) -> Ins {
    fn o(x: &Opd, f: &mut dyn FnMut(R) -> R) -> Opd {
        match x {
            Opd::Reg(r) => Opd::Reg(f(*r)),
            other => other.clone(),
        }
    }
    fn v(x: &Val, f: &mut dyn FnMut(R) -> R) -> Val {
        match x {
            Val::Reg(r) => Val::Reg(f(*r)),
            other => other.clone(),
        }
    }
    fn p(q: &[Ins], f: &mut dyn FnMut(R) -> R) -> Prog {
        q.iter().map(|i| map_regs_ref(i, f)).collect::<Vec<_>>().into()
    }
    match i {
        Ins::IterKind { kinds, out } => Ins::IterKind { kinds: kinds.clone(), out: f(*out) },
        Ins::IterValue { rel, col, val, out } => {
            let val = v(val, f);
            Ins::IterValue { rel: *rel, col: *col, val, out: f(*out) }
        }
        Ins::IterChildren { parent, out } => {
            let parent = f(*parent);
            Ins::IterChildren { parent, out: f(*out) }
        }
        Ins::IterScan { rel, out } => Ins::IterScan { rel: *rel, out: f(*out) },
        Ins::IterFact { rel, col, val, out } => {
            let val = v(val, f);
            Ins::IterFact { rel: *rel, col: *col, val, out: f(*out) }
        }
        Ins::IterFactScan { rel, out } => Ins::IterFactScan { rel: *rel, out: f(*out) },
        Ins::Row { rel, key } => Ins::Row { rel: *rel, key: f(*key) },
        Ins::Read { rel, col, row, out } => {
            let row = f(*row);
            Ins::Read { rel: *rel, col: *col, row, out: f(*out) }
        }
        Ins::ReadFact { rel, col, row, out } => {
            let row = f(*row);
            Ins::ReadFact { rel: *rel, col: *col, row, out: f(*out) }
        }
        Ins::Test { test, l, r, text, num, regex } => {
            let l = o(l, f);
            let r = o(r, f);
            Ins::Test { test: *test, l, r, text: *text, num: *num, regex: regex.clone() }
        }
        Ins::Axis { dir, start, out } => {
            let start = f(*start);
            Ins::Axis { dir: *dir, start, out: f(*out) }
        }
        Ins::Within { anc, desc } => {
            let anc = f(*anc);
            Ins::Within { anc, desc: f(*desc) }
        }
        Ins::IterRange { rel, col, val, out, start, lo, hi } => {
            let val = v(val, f);
            let lo = lo.map(|(r, b)| (f(r), b));
            let hi = hi.map(|(r, b)| (f(r), b));
            let out = f(*out);
            Ins::IterRange { rel: *rel, col: *col, val, out, start: f(*start), lo, hi }
        }
        Ins::IterField { parent, field, out, field_out } => {
            let parent = f(*parent);
            let out = f(*out);
            Ins::IterField { parent, field: *field, out, field_out: f(*field_out) }
        }
        Ins::Near { start, out, subs } => {
            let start = f(*start);
            let out = f(*out);
            Ins::Near { start, out, subs: subs.iter().map(|q| p(q, f)).collect::<Vec<_>>().into() }
        }
        Ins::Sub { negate, subs } => Ins::Sub { negate: *negate, subs: subs.iter().map(|q| p(q, f)).collect::<Vec<_>>().into() },
        Ins::Emit { cols, rule } => Ins::Emit { cols: cols.iter().map(|c| o(c, f)).collect::<Vec<_>>().into(), rule: *rule },
        Ins::Fork { subs } => Ins::Fork { subs: subs.iter().map(|(s, q)| (s.clone(), p(q, f))).collect::<Vec<_>>().into() },
        Ins::Yield => Ins::Yield,
        Ins::ReadCol { .. }
        | Ins::ReadChild { .. }
        | Ins::TestEqC { .. }
        | Ins::TestNeC { .. }
        | Ins::TestEqR { .. }
        | Ins::TestNeR { .. }
        | Ins::TestInC { .. }
        | Ins::Hoist { .. }
        | Ins::HoistProbe { .. }
        | Ins::Capture { .. }
        | Ins::NearMemo { .. }
        | Ins::SubKey { .. }
        | Ins::AncExists { .. }
        | Ins::SubFirst { .. } => unreachable!("specialisation runs after the tries are built"),
        Ins::SpanBetween { a, b, out } => {
            let a = o(a, f);
            let b = o(b, f);
            Ins::SpanBetween { a, b, out: f(*out) }
        }
        Ins::SpanBounds { s, start, end } => {
            let s = o(s, f);
            let start = f(*start);
            Ins::SpanBounds { s, start, end: f(*end) }
        }
        Ins::SourceMatches { span, base, regex, group, outs, capture_text, capture_line } => {
            let span = o(span, f);
            let base = base.as_ref().map(|b| o(b, f));
            Ins::SourceMatches {
                span,
                base,
                regex: *regex,
                group: *group,
                outs: outs.iter().map(|&r| f(r)).collect::<Vec<_>>().into(),
                capture_text: *capture_text,
                capture_line: *capture_line,
            }
        }
        Ins::Collect { regs } => Ins::Collect { regs: regs.iter().map(|&r| f(r)).collect::<Vec<_>>().into() },
        Ins::Closure { src, out, xin, kout, nkeys, subs } => {
            let src = f(*src);
            let out = f(*out);
            let xin = f(*xin);
            let kout = kout.iter().map(|&r| f(r)).collect::<Vec<_>>().into();
            Ins::Closure { src, out, xin, kout, nkeys: *nkeys, subs: subs.iter().map(|q| p(q, f)).collect::<Vec<_>>().into() }
        }
        Ins::Union { outs, subs, memo } => {
            let outs = outs.iter().map(|&r| f(r)).collect::<Vec<_>>().into();
            Ins::Union { outs, subs: subs.iter().map(|q| p(q, f)).collect::<Vec<_>>().into(), memo: *memo }
        }
        Ins::Let { from, out } => {
            let from = o(from, f);
            Ins::Let { from, out: f(*out) }
        }
        Ins::Shared { idx } => Ins::Shared { idx: *idx },
        Ins::TextFn { lower, args, out } => {
            let args = args.iter().map(|a| o(a, f)).collect::<Vec<_>>().into();
            Ins::TextFn { lower: *lower, args, out: f(*out) }
        }
        Ins::CaptureText { text, pattern, outs } => {
            let text = o(text, f);
            Ins::CaptureText { text, pattern: pattern.clone(), outs: outs.iter().map(|&r| f(r)).collect::<Vec<_>>().into() }
        }
        Ins::Replace { text, regex, replacement, out } => {
            let text = o(text, f);
            Ins::Replace { text, regex: *regex, replacement: replacement.clone(), out: f(*out) }
        }
        Ins::NearDerived { start, out, xin, edges, domain } => {
            let start = f(*start);
            let out = f(*out);
            let xin = f(*xin);
            let edges = edges.iter().map(|q| p(q, f)).collect::<Vec<_>>().into();
            Ins::NearDerived { start, out, xin, edges, domain: domain.iter().map(|q| p(q, f)).collect::<Vec<_>>().into() }
        }
        Ins::Sibling { next, start, out } => {
            let start = f(*start);
            Ins::Sibling { next: *next, start, out: f(*out) }
        }
        Ins::SubMemo { negate, subs } => Ins::SubMemo { negate: *negate, subs: subs.iter().map(|q| p(q, f)).collect::<Vec<_>>().into() },
    }
}

// ------------------------------------------------------------------ per-file state

#[derive(Default)]
pub(crate) struct Counters {
    pub steps: u64,
    pub candidates: u64,
    pub starts: u64,
    pub emitted: u64,
    pub subs: u64,
    pub made_tables: u64,
    pub made_rows: u64,
    pub fact_files: u64,
    pub fact_rows: u64,
    pub fact_ns: u64,
    pub provider_build_ns: u64,
    /// time in made-table builds (their programs' steps included, nested builds counted once) and in value-index builds
    pub made_ns: u64,
    pub index_ns: u64,
    /// index_ns split: [tree value index, constant scans, fact column index]
    pub index_split: [u64; 3],
    /// constant-list misses, bucket (CSR) builds and their time
    pub const_misses: u64,
    pub csr_builds: u64,
    pub csr_ns: u64,
    /// per made table (fact index): (builds, exclusive ns, input rows, steps)
    pub made_by: FxMap<u16, (u64, u64, u64, u64)>,
    pub kind_index_ns: u64,
    pub model_lex_ns: u64,
    pub model_flow_ns: u64,
    pub model_rows_ns: u64,
    pub model_files: u64,
    pub flow_files: u64,
    pub provider_files: u64,
    /// navwide: thread-ns and files of the derived relations served from the node table
    pub derived_ns: u64,
    pub derived_files: u64,
    pub visits: u64,
    pub rule_steps: Vec<u64>,
    pub rule_visits: Vec<u64>,
    pub rule_starts: Vec<u64>,
    /// made-table rows and the steps spent building made tables, per rule that demanded them
    pub rule_made_rows: Vec<u64>,
    pub rule_made_steps: Vec<u64>,
    /// steps per instruction kind (index: Ins::code)
    pub op_steps: Vec<u64>,
    /// (rule, branch) -> (runs, steps)
    pub branch_steps: FxMap<(u32, u32), (u64, u64)>,
    /// NAV_PROFILE: steps per instruction site (its address)
    pub prof: Option<FxMap<usize, u64>>,
    /// steps executed inside batched loops
    pub batched_steps: u64,
    /// steps spent building made tables (outermost builds), excluded from per-program accounting
    pub made_steps_total: u64,
    pub hoist_runs: u64,
    pub hoist_misses: u64,
    /// per hoist site over the run: (lookups, misses)
    pub hoist_sites: FxMap<u32, (u64, u64)>,
    /// per SubKey site over the run: (lookups, misses)
    pub key_sites: FxMap<usize, (u64, u64)>,
    /// per base made table over the run: (steps of its builds, builds) as a twin's buy price; twins switched
    pub twin_buy: FxMap<u16, (u64, u64)>,
    pub twin_switches: u64,
    /// NearMemo / AncExists memo arrays allocated (per file per site) and their cells
    pub memo_allocs: u64,
    pub memo_cells: u64,
    /// per derived relation: (files, ns, rows)
    pub derived_by: FxMap<u16, (u64, u64, u64)>,
    pub made_depth: u32,
    /// NAV_DIAG / NAV_PROFILE: per-instruction-kind and per-site counters in the hot loop
    pub diag: bool,
}

/// What a file's facts from the provider are, decoded into the view's ids and the run's text codes.
#[derive(Default)]
pub(crate) struct Facts {
    pub cols: Vec<Vec<Vec<u64>>>,
    pub loaded: Vec<bool>,
    pub provider_done: bool,
    pub idx: FxMap<(u16, u8), Rc<FIdx>>,
}

/// A tree column bucketed by base-symbol value: value -> range of `ids` (start order).
pub(crate) struct Csr {
    at: FxMap<u32, (u32, u32)>,
    ids: Vec<u32>,
}

/// A tree relation column's value index: ids sorted by (value, start, id).
pub(crate) struct TIdx {
    vals: Vec<u32>,
    ids: Vec<u32>,
}

impl TIdx {
    fn get(&self, v: u32) -> &[u32] {
        let a = self.vals.partition_point(|&x| x < v);
        let b = a + self.vals[a..].partition_point(|&x| x == v);
        &self.ids[a..b]
    }
}

/// A fact column's index: by view id (dense head/next chains) for node columns, hashed otherwise.
pub(crate) enum FIdx {
    Dense { head: Vec<u32>, next: Vec<u32> },
    Hash(FxMap<u64, Vec<u32>>),
}

pub(crate) struct Provider {
    pub engine: std::sync::Arc<dyn ryo_language::inbound::use_cases::parse::types::ParseEngine>,
    pub elements: ryo_language::domain::elements::types::Elements,
    pub symbols: ryo_language::domain::symbols::types::Symbols,
    pub columns: Vec<Vec<ryo_relations::domain::relations::types::FactColumn>>,
}

impl Provider {
    pub(crate) fn new() -> Self {
        let language = ryo_python::linked_with_settings(ryo_python::settings::PythonParserSettings::new(
            ryo_python::settings::PythonTargetVersion::Py310,
        ));
        Self {
            engine: language.parse_engine(),
            elements: Default::default(),
            symbols: Default::default(),
            columns: Vec::new(),
        }
    }
}

#[derive(Default)]
pub(crate) struct FileCache {
    /// kind code -> ids of that kind (nodes and tokens), a CSR over the view's kind column
    pub kind_off: Vec<u32>,
    pub kind_ids: Vec<u32>,
    /// identifier text code per AST node id (NULL when the node is neither an identifier nor a name)
    pub ident: Option<Vec<u32>>,
    pub tree_idx: FxMap<(u8, u8), Rc<TIdx>>,
    /// a constant value set's ids (by the instruction's value address), in start order: one linear scan per file
    pub const_idx: FxMap<usize, Rc<Vec<u32>>>,
    /// the same lists by content (relation, column, values), shared by every instruction naming that set
    pub const_by: FxMap<(u8, u8, Vec<u64>), Rc<Vec<u32>>>,
    /// per tree column: ids bucketed by base-symbol value (counting sort, each bucket in start order)
    pub csr: FxMap<(u8, u8), Rc<Csr>>,
    pub csr_count: Vec<u32>,
    /// keyed twin rows per (fact, key value): the row range in the twin's columns
    pub keyed_memo: FxMap<(u16, u64), (u32, u32)>,
    /// ski rental per keyed twin in this file: steps spent in keyed builds, and the twins switched to their base
    pub twin_rent: FxMap<u16, u64>,
    pub twin_bought: std::collections::HashSet<u16, std::hash::BuildHasherDefault<crate::sema::Fx>>,
    /// hoisted inner enumerations per (site, key values): captured tuples, flattened
    pub hoisted: FxMap<(u32, u64, u64), Rc<Vec<u64>>>,
    /// HoistProbe: per (site, key values) the captured tuples indexed by the probed register
    pub probed: FxMap<(u32, u64, u64), Rc<FxMap<u64, Vec<u64>>>>,
    pub facts: Option<Facts>,
    pub sub_memo: FxMap<usize, bool>,
    /// join: NearMemo answers per site (indexed by node id) and SubKey answers per (site, key value)
    pub near_memo: FxMap<usize, Vec<u32>>,
    pub key_memo: FxMap<(usize, u64), bool>,
    /// navwide: reusable state of the derived relations (derived.rs)
    pub dstate: crate::derived::State,
    pub model_lex: bool,
    pub model_flow: bool,
    /// per-file dispatch: kind code -> (rule, branch, alternative) anchored on it
    pub dispatch: Vec<Vec<(u32, u32, u32)>>,
    pub dispatch_used: Vec<u32>,
    pub other: Vec<(u32, u32, u32)>,
    /// per file: (rule, branch) slot -> may find rows (no kind-anchored alternative has an empty bucket)
    pub enabled: Vec<bool>,
}

pub(crate) struct File<'a> {
    pub view: &'a View,
    pub table: &'a crate::table::Table<'a>,
    pub source: &'a str,
    pub path: &'a str,
    pub file: usize,
    pub incomplete: bool,
    /// navwide: the parse, for the derived relations served from the node table (derived.rs)
    pub parsed: &'a ruff_python_parser::Parsed<ruff_python_ast::Mod>,
}

pub(crate) struct Exec<'a, 'p> {
    pub prog: &'p NavProgram,
    pub f: &'a File<'a>,
    pub sym: &'a mut Symbols,
    pub cache: &'a mut FileCache,
    /// the old provider, only when a relation of the program has no other source (navwide: None otherwise)
    pub provider: Option<&'a mut Provider>,
    pub model: &'a mut crate::model::Model,
    pub scratch: String,
    pub c: &'a mut Counters,
    pub regs: Vec<u64>,
    pub outs: Vec<Vec<u64>>,
    pub cur_rule: usize,
    /// the collect buffer of the innermost running closure edge sub-program (navwide)
    pub collect: Vec<u64>,
    /// per-file rows of unions that read no bound variable, by their sub-program list (navwide)
    pub union_memo: FxMap<usize, Rc<Vec<u64>>>,
    /// per-file answers of absence/existence tests that read no candidate register (navwide)
    pub sub_memo: FxMap<usize, bool>,
    /// the running hoist's capture buffer
    pub cap_buf: Vec<u64>,
    /// set by a failing SubFirst: the enclosing range walk stops
    pub first_fail: bool,
}

fn span(s: u32, e: u32) -> u64 {
    (u64::from(s) << 32) | u64::from(e)
}

impl<'a, 'p> Exec<'a, 'p> {
    pub(crate) fn build_kinds(&mut self) {
        let t = Instant::now();
        let v = self.f.view;
        let maxk = v.kind.iter().copied().max().unwrap_or(0) as usize + 2;
        let off = &mut self.cache.kind_off;
        off.clear();
        off.resize(maxk + 1, 0);
        for &k in &v.kind {
            off[k as usize + 1] += 1;
        }
        for i in 0..maxk {
            off[i + 1] += off[i];
        }
        let mut fill = off.clone();
        self.cache.kind_ids.clear();
        self.cache.kind_ids.resize(v.kind.len(), 0);
        for (id, &k) in v.kind.iter().enumerate() {
            let s = &mut fill[k as usize];
            self.cache.kind_ids[*s as usize] = id as u32;
            *s += 1;
        }
        self.c.kind_index_ns += t.elapsed().as_nanos() as u64;
    }

    pub(crate) fn bucket(&self, k: u32) -> &[u32] {
        let k = k as usize;
        if k + 1 >= self.cache.kind_off.len() {
            return &[];
        }
        &self.cache.kind_ids[self.cache.kind_off[k] as usize..self.cache.kind_off[k + 1] as usize]
    }

    fn ident(&mut self, id: u32) -> u32 {
        if self.cache.ident.is_none() {
            let n = self.f.view.n_nodes;
            let mut col = vec![NULL; n];
            for (i, node) in self.f.table.node.iter().enumerate() {
                let text = match node {
                    ruff_python_ast::AnyNodeRef::Identifier(x) => x.as_str(),
                    ruff_python_ast::AnyNodeRef::ExprName(x) => x.id.as_str(),
                    _ => continue,
                };
                col[i] = self.sym.text(text);
            }
            self.cache.ident = Some(col);
        }
        let col = self.cache.ident.as_ref().unwrap();
        col.get(id as usize).copied().unwrap_or(NULL)
    }

    /// A tree relation's column at a row (the row is the relation's key id); None when the row does not exist.
    fn tread(&mut self, rel: TRel, col: u8, id: u32) -> Option<u64> {
        let v = self.f.view;
        let i = id as usize;
        if i >= v.len() {
            return None;
        }
        Some(match rel {
            TRel::Ss => match col {
                0 => id as u64,
                1 => v.kind[i] as u64,
                2 => v.text[i] as u64,
                3 => 0,
                _ => span(v.start[i], v.stop[i]),
            },
            TRel::Sc => {
                let p = v.parent[i];
                if p == u32::MAX {
                    return None;
                }
                match col {
                    0 => p as u64,
                    1 => id as u64,
                    2 => v.ordinal[i] as u64,
                    3 => v.field[i] as u64,
                    _ => 0,
                }
            }
            TRel::Sl => {
                let (s, e) = (v.start[i], v.stop[i]);
                match col {
                    0 => id as u64,
                    1 => 0,
                    2 => span(s, e),
                    3 => v.line_of(s) as u64,
                    4 => v.line_of(e) as u64,
                    5 => s as u64,
                    6 => e as u64,
                    7 => v.column_of(s) as u64,
                    _ => v.column_of(e) as u64,
                }
            }
            TRel::Id => {
                let t = self.ident(id);
                if t == NULL {
                    return None;
                }
                match col {
                    0 => id as u64,
                    1 => t as u64,
                    2 => 0,
                    _ => span(v.start[i], v.stop[i]),
                }
            }
        })
    }

    fn row_exists(&mut self, rel: TRel, id: u32) -> bool {
        match rel {
            TRel::Ss | TRel::Sl => (id as usize) < self.f.view.len(),
            TRel::Sc => (id as usize) < self.f.view.len() && self.f.view.parent[id as usize] != u32::MAX,
            TRel::Id => self.tread(TRel::Id, 0, id).is_some(),
        }
    }

    /// Group count of a made table's input rows: (key values..., count) rows, flattened (keys then count).
    fn group_rows(rows: &[u64], keys: &[usize], distinct: Option<usize>, width: usize) -> Vec<u64> {
        let w = width.max(1);
        let rest: Vec<usize> = match distinct {
            Some(d) => vec![d],
            None => (0..width).filter(|c| !keys.contains(c)).collect(),
        };
        assert!(keys.len() + rest.len() <= 8, "made identity wider than 8");
        let mut ids: Vec<[u64; 8]> = rows
            .chunks(w)
            .map(|row| {
                let mut x = [0u64; 8];
                for (i, &k) in keys.iter().chain(rest.iter()).enumerate() {
                    x[i] = row[k];
                }
                x
            })
            .collect();
        ids.sort_unstable();
        ids.dedup();
        let nk = keys.len();
        let mut out = Vec::new();
        let mut i = 0;
        while i < ids.len() {
            let mut j = i + 1;
            while j < ids.len() && ids[j][..nk] == ids[i][..nk] {
                j += 1;
            }
            out.extend_from_slice(&ids[i][..nk]);
            out.push((j - i) as u64);
            i = j;
        }
        out
    }

    /// A keyed twin's rows for one key value: run its programs with the key register set, group, append; memoised per file.
    fn keyed_rows(&mut self, rel: u16, value: u64) -> (u32, u32) {
        if let Some(&r) = self.cache.keyed_memo.get(&(rel, value)) {
            return r;
        }
        let prog = self.prog;
        let f = &prog.facts[rel as usize];
        // ski rental (NAV_SKI=0 turns it off): per file, keyed builds run until their steps exceed what building the base table
        // once costs (the run's average so far); then the base is built and indexed by the key, and every later key is served
        // from it
        if f.base != u16::MAX && SKI.with(|s| *s) {
            let bought = self.cache.twin_bought.contains(&rel);
            let rent = self.cache.twin_rent.get(&rel).copied().unwrap_or(0);
            let (buy_steps, buys) = self.c.twin_buy.get(&f.base).copied().unwrap_or((0, 0));
            let budget = if buys == 0 { 20_000 } else { buy_steps / buys };
            if bought || rent > budget {
                return self.keyed_from_base(rel, value);
            }
        }
        let steps_at = self.c.steps;
        let r = self.keyed_build(rel, value);
        *self.cache.twin_rent.entry(rel).or_default() += self.c.steps - steps_at;
        r
    }

    /// a keyed twin's rows for one key value from its base table (built once per file, indexed by the key column)
    fn keyed_from_base(&mut self, rel: u16, value: u64) -> (u32, u32) {
        let prog = self.prog;
        let f = &prog.facts[rel as usize];
        let base = f.base;
        let col = f.keyed.unwrap().0;
        if self.cache.twin_bought.insert(rel) {
            let loaded = self.cache.facts.as_ref().is_some_and(|x| x.loaded[base as usize]);
            if !loaded {
                let s0 = self.c.steps;
                self.ensure(base);
                let e = self.c.twin_buy.entry(base).or_default();
                e.0 += self.c.steps - s0;
                e.1 += 1;
            }
            self.c.twin_switches += 1;
        }
        let idx = self.fact_index(base, col);
        let rows: Vec<u32> = match &*idx {
            FIdx::Dense { head, next } => {
                let mut v = Vec::new();
                if (value as usize) < head.len() - 1 {
                    let mut r = head[value as usize];
                    while r != u32::MAX {
                        v.push(r);
                        r = next[r as usize];
                    }
                }
                v
            }
            FIdx::Hash(m) => m.get(&value).cloned().unwrap_or_default(),
        };
        let facts = self.cache.facts.as_mut().unwrap();
        let width = facts.cols[rel as usize].len();
        let start = facts.cols[rel as usize][width - 1].len() as u32;
        for &r in &rows {
            for c in 0..width {
                let v = facts.cols[base as usize][c][r as usize];
                facts.cols[rel as usize][c].push(v);
            }
        }
        let end = facts.cols[rel as usize][width - 1].len() as u32;
        self.cache.keyed_memo.insert((rel, value), (start, end));
        (start, end)
    }

    fn keyed_build(&mut self, rel: u16, value: u64) -> (u32, u32) {
        let prog = self.prog;
        let f = &prog.facts[rel as usize];
        let (progs, keys, distinct, width) = f.made.as_ref().unwrap();
        let (_, param) = f.keyed.unwrap();
        let t0 = Instant::now();
        let made_before = self.c.made_ns;
        let fact_before = self.c.fact_ns;
        let steps0 = self.c.steps;
        self.c.made_depth += 1;
        self.ensure_facts();
        let slot = self.outs.len() - 1;
        let saved = (self.cur_rule, std::mem::take(&mut self.outs[slot]));
        let nr = f.nregs;
        let saved_regs: Vec<u64> = self.regs[..nr].to_vec();
        self.cur_rule = slot;
        self.regs[param as usize] = value;
        for q in progs {
            self.run_inner(q, 0);
        }
        self.regs[..nr].copy_from_slice(&saved_regs);
        let rows = std::mem::replace(&mut self.outs[slot], saved.1);
        self.cur_rule = saved.0;
        let grouped = Self::group_rows(&rows, keys, *distinct, *width);
        let nk = keys.len();
        let facts = self.cache.facts.as_mut().unwrap();
        let cols = &mut facts.cols[rel as usize];
        let start = cols[nk].len() as u32;
        for g in grouped.chunks(nk + 1) {
            for (c, v) in g.iter().enumerate() {
                cols[c].push(*v);
            }
        }
        let end = cols[nk].len() as u32;
        self.c.made_tables += 1;
        self.c.made_rows += u64::from(end - start) + (rows.len() / width.max(&1)) as u64;
        let excl = t0.elapsed().as_nanos() as u64 - (self.c.fact_ns - fact_before) - (self.c.made_ns - made_before);
        self.c.made_ns = made_before + t0.elapsed().as_nanos() as u64 - (self.c.fact_ns - fact_before);
        let e = self.c.made_by.entry(rel).or_default();
        e.0 += 1;
        e.1 += excl;
        e.2 += (rows.len() / width.max(&1)) as u64;
        e.3 += self.c.steps - steps0;
        self.c.made_depth -= 1;
        if self.c.made_depth == 0 {
            self.c.made_steps_total += self.c.steps - steps0;
        }
        self.cache.keyed_memo.insert((rel, value), (start, end));
        (start, end)
    }

    fn ensure_facts(&mut self) {
        let prog = self.prog;
        self.cache.facts.get_or_insert_with(|| Facts {
            cols: prog.facts.iter().map(|f| vec![Vec::new(); f.kinds.len()]).collect(),
            loaded: vec![false; prog.facts.len()],
            provider_done: false,
            idx: FxMap::default(),
        });
    }

    /// the ids whose tree column holds one of a constant set of values, in start order (one scan, memoised per file)
    fn const_list(&mut self, rel: TRel, col: u8, val: &Val, sorted: bool) -> Rc<Vec<u32>> {
        let key = (val as *const Val as usize) * 2 + usize::from(sorted);
        if let Some(l) = self.cache.const_idx.get(&key) {
            return l.clone();
        }
        let t0 = Instant::now();
        self.c.const_misses += 1;
        let n = if rel == TRel::Id { self.f.view.n_nodes } else { self.f.view.len() };
        let wanted: Vec<u64> = match val {
            Val::Syms(s) => s.iter().map(|&x| u64::from(x)).collect(),
            Val::Nums(v) => v.iter().map(|&x| u64::from(x as u32)).collect(),
            Val::Reg(_) => unreachable!(),
        };
        let mut ckey = (rel as u8, col, wanted.clone());
        ckey.2.push(u64::from(sorted));
        if let Some(l) = self.cache.const_by.get(&ckey) {
            let l = l.clone();
            self.cache.const_idx.insert(key, l.clone());
            return l;
        }
        let v = self.f.view;
        if rel == TRel::Id && col == 1 && self.cache.ident.is_none() {
            // identifier text against a few constants: compare the parser's own strings, no interning of the file's names
            let texts: Vec<&str> = wanted.iter().filter_map(|&w| self.sym.str(w as u32)).collect();
            let mut out: Vec<u32> = Vec::new();
            for (i, node) in self.f.table.node.iter().enumerate() {
                let t = match node {
                    ruff_python_ast::AnyNodeRef::Identifier(x) => x.as_str(),
                    ruff_python_ast::AnyNodeRef::ExprName(x) => x.id.as_str(),
                    _ => continue,
                };
                if texts.contains(&t) {
                    out.push(i as u32);
                }
            }
            out.sort_by_key(|&id| (v.start[id as usize], id));
            self.c.made_tables += 1;
            self.c.index_ns += t0.elapsed().as_nanos() as u64;
            self.c.index_split[1] += t0.elapsed().as_nanos() as u64;
            let l = Rc::new(out);
            self.cache.const_idx.insert(key, l.clone());
            self.cache.const_by.insert(ckey, l.clone());
            return l;
        }
        let csr = self.col_csr(rel, col);
        let mut out: Vec<u32> = Vec::new();
        for &w in &wanted {
            if let Some(&(a, b)) = csr.at.get(&(w as u32)) {
                out.extend_from_slice(&csr.ids[a as usize..b as usize]);
            }
        }
        let _ = n;
        if sorted {
            out.sort_by_key(|&id| (v.start[id as usize], id));
        }
        self.c.made_tables += 1;
        self.c.made_rows += out.len() as u64;
        self.c.index_ns += t0.elapsed().as_nanos() as u64;
        self.c.index_split[1] += t0.elapsed().as_nanos() as u64;
        let l = Rc::new(out);
        self.cache.const_idx.insert(key, l.clone());
        self.cache.const_by.insert(ckey, l.clone());
        l
    }

    /// a tree column's ids bucketed by value for base-symbol values (the pack's literals are base symbols): one counting sort
    /// whose cost is the file's size (a reusable count array; only the values present are touched)
    fn col_csr(&mut self, rel: TRel, col: u8) -> Rc<Csr> {
        if let Some(c) = self.cache.csr.get(&(rel as u8, col)) {
            return c.clone();
        }
        let t_csr = Instant::now();
        self.c.csr_builds += 1;
        let nb = self.sym.base_count();
        let n = if rel == TRel::Id { self.f.view.n_nodes } else { self.f.view.len() };
        let v = self.f.view;
        let mut vals: Vec<u32> = Vec::with_capacity(n);
        match (rel, col) {
            (TRel::Sc, 3) => vals.extend((0..n).map(|i| if v.parent[i] == u32::MAX { u32::MAX } else { v.field[i] })),
            (TRel::Ss, 2) => vals.extend_from_slice(&v.text[..n]),
            _ => {
                for id in 0..n as u32 {
                    vals.push(self.tread(rel, col, id).map_or(u32::MAX, |x| x as u32));
                }
            }
        }
        let count = &mut self.cache.csr_count;
        if count.len() < nb {
            count.resize(nb, 0);
        }
        let mut touched: Vec<u32> = Vec::new();
        for &x in &vals {
            if (x as usize) < nb {
                if count[x as usize] == 0 {
                    touched.push(x);
                }
                count[x as usize] += 1;
            }
        }
        let mut at: FxMap<u32, (u32, u32)> = FxMap::default();
        let mut pos = 0u32;
        let mut starts: Vec<u32> = Vec::with_capacity(touched.len());
        for &t in &touched {
            let c = count[t as usize];
            starts.push(pos);
            count[t as usize] = pos;
            pos += c;
        }
        let mut ids = vec![0u32; pos as usize];
        for (id, &x) in vals.iter().enumerate() {
            if (x as usize) < nb {
                ids[count[x as usize] as usize] = id as u32;
                count[x as usize] += 1;
            }
        }
        for (k, &t) in touched.iter().enumerate() {
            at.insert(t, (starts[k], count[t as usize]));
            count[t as usize] = 0;
        }
        // buckets stay in id order; a range walk sorts its own (small) list by start
        self.c.csr_ns += t_csr.elapsed().as_nanos() as u64;
        let c = Rc::new(Csr { at, ids });
        self.cache.csr.insert((rel as u8, col), c.clone());
        c
    }

    /// a tree relation's value index: (value, start, id) sorted once per file; a value's ids are a slice in start order
    fn tree_index(&mut self, rel: TRel, col: u8) -> Rc<TIdx> {
        let key = (rel as u8, col);
        if let Some(i) = self.cache.tree_idx.get(&key) {
            return i.clone();
        }
        let t0 = Instant::now();
        let n = if rel == TRel::Id { self.f.view.n_nodes } else { self.f.view.len() };
        let mut pairs: Vec<(u64, u32)> = Vec::with_capacity(n);
        for id in 0..n as u32 {
            if let Some(v) = self.tread(rel, col, id) {
                pairs.push(((u64::from(v as u32) << 32) | u64::from(self.f.view.start[id as usize]), id));
            }
        }
        pairs.sort_unstable();
        let idx = TIdx { vals: pairs.iter().map(|p| (p.0 >> 32) as u32).collect(), ids: pairs.iter().map(|p| p.1).collect() };
        self.c.made_tables += 1;
        self.c.made_rows += n as u64;
        self.c.index_ns += t0.elapsed().as_nanos() as u64;
        self.c.index_split[0] += t0.elapsed().as_nanos() as u64;
        let m = Rc::new(idx);
        self.cache.tree_idx.insert(key, m.clone());
        m
    }

    // ---- facts: the model's relations one at a time on first demand; the provider's readers once per file for the rest
    fn ensure(&mut self, rel: u16) {
        let prog = self.prog;
        let facts = self.cache.facts.get_or_insert_with(|| Facts {
            cols: prog.facts.iter().map(|f| vec![Vec::new(); f.kinds.len()]).collect(),
            loaded: vec![false; prog.facts.len()],
            provider_done: false,
            idx: FxMap::default(),
        });
        if facts.loaded[rel as usize] {
            return;
        }
        let t = Instant::now();
        if let Some(om) = &prog.facts[rel as usize].made_om {
            self.ordered_match(rel, om);
            self.c.fact_ns += t.elapsed().as_nanos() as u64;
            return;
        }
        if let Some((progs, keys, distinct, width)) = &prog.facts[rel as usize].made {
            let nkeys = &keys.len();
            let made_t0 = Instant::now();
            self.c.made_depth += 1;
            let made_before = self.c.made_ns;
            let fact_before = self.c.fact_ns;
            let steps0 = self.c.steps;
            let owner = self.cur_rule;
            let slot = self.outs.len() - 1;
            let saved = (self.cur_rule, std::mem::take(&mut self.outs[slot]));
            let nr = prog.facts[rel as usize].nregs;
            let saved_regs: Vec<u64> = self.regs[..nr].to_vec();
            self.cur_rule = slot;
            for q in progs {
                self.run_inner(q, 0);
            }
            self.regs[..nr].copy_from_slice(&saved_regs);
            let rows = std::mem::replace(&mut self.outs[slot], saved.1);
            let rows_len = rows.len();
            self.cur_rule = saved.0;
            // group count by sorting fixed-width identities (keys first, then the distinct column or the rest of the row):
            // equal identities collapse, runs of equal keys are the groups
            let w = (*width).max(1);
            let rest: Vec<usize> = match distinct {
                Some(d) => vec![*d],
                None => (0..*width).filter(|c| !keys.contains(c)).collect(),
            };
            let t = keys.len() + rest.len();
            assert!(t <= 8, "made identity wider than 8");
            let mut ids: Vec<[u64; 8]> = rows
                .chunks(w)
                .map(|row| {
                    let mut x = [0u64; 8];
                    for (i, &k) in keys.iter().chain(rest.iter()).enumerate() {
                        x[i] = row[k];
                    }
                    x
                })
                .collect();
            ids.sort_unstable();
            ids.dedup();
            let facts = self.cache.facts.as_mut().unwrap();
            let cols = &mut facts.cols[rel as usize];
            let nk = keys.len();
            let mut i = 0;
            while i < ids.len() {
                let mut j = i + 1;
                while j < ids.len() && ids[j][..nk] == ids[i][..nk] {
                    j += 1;
                }
                for c in 0..nk {
                    cols[c].push(ids[i][c]);
                }
                cols[*nkeys].push((j - i) as u64);
                i = j;
            }
            let _ = t;
            self.c.made_tables += 1;
            self.c.made_rows += cols[*nkeys].len() as u64;
            facts.loaded[rel as usize] = true;
            let excl = made_t0.elapsed().as_nanos() as u64 - (self.c.fact_ns - fact_before) - (self.c.made_ns - made_before);
            self.c.made_ns = made_before + made_t0.elapsed().as_nanos() as u64 - (self.c.fact_ns - fact_before);
            let e = self.c.made_by.entry(rel).or_default();
            e.0 += 1;
            e.1 += excl;
            e.2 += (rows_len / width.max(&1)) as u64;
            e.3 += self.c.steps - steps0;
            self.c.made_depth -= 1;
            if self.c.made_depth == 0 {
                self.c.made_steps_total += self.c.steps - steps0;
            }
            if owner < self.c.rule_made_rows.len() {
                self.c.rule_made_rows[owner] += cols[*nkeys].len() as u64 + (rows_len / width.max(&1)) as u64;
                self.c.rule_made_steps[owner] += self.c.steps - steps0;
            }
            return;
        }
        match prog.facts[rel as usize].model {
            Some(mi) => {
                if !self.cache.model_lex {
                    let t = Instant::now();
                    self.model.build_lexical(self.f.table, self.f.incomplete);
                    self.model.resolve_loads(self.f.table);
                    self.cache.model_lex = true;
                    self.c.model_lex_ns += t.elapsed().as_nanos() as u64;
                    self.c.model_files += 1;
                }
                if mi >= 6 && !self.cache.model_flow {
                    let t = Instant::now();
                    self.model.build_flow(self.f.table);
                    self.model.build_resolution(self.f.table);
                    self.cache.model_flow = true;
                    self.c.model_flow_ns += t.elapsed().as_nanos() as u64;
                    self.c.flow_files += 1;
                }
                let t = Instant::now();
                let facts = self.cache.facts.as_mut().unwrap();
                let mut cols = std::mem::take(&mut facts.cols[rel as usize]);
                crate::semrows::produce(self.model, self.f.table, mi, self.sym, &mut cols, &mut self.scratch);
                self.c.fact_rows += cols.first().map_or(0, Vec::len) as u64;
                let facts = self.cache.facts.as_mut().unwrap();
                facts.cols[rel as usize] = cols;
                facts.loaded[rel as usize] = true;
                self.c.model_rows_ns += t.elapsed().as_nanos() as u64;
            }
            None if !PROVIDER_ALL.with(|p| *p)
                && (crate::derived::index_of(&prog.facts[rel as usize].name).is_some() || prog.facts[rel as usize].name.starts_with("filesystem.")) =>
            {
                self.load_derived(rel);
            }
            None => {
                let loaded = self.load_facts();
                let facts = self.cache.facts.as_mut().unwrap();
                for (i, cols) in loaded.into_iter().enumerate() {
                    if prog.facts[i].model.is_none() && prog.facts[i].made.is_none() && !facts.loaded[i] {
                        facts.cols[i] = cols;
                        facts.loaded[i] = true;
                    }
                }
                facts.provider_done = true;
                self.c.fact_files += 1;
            }
        }
        self.c.fact_ns += t.elapsed().as_nanos() as u64;
    }

    fn load_facts(&mut self) -> Vec<Vec<Vec<u64>>> {
        use ryo_language::domain::elements::service::{end_at, kind_at, kind_names, start_at};
        use ryo_language::domain::symbols::service::{clear_symbols, text_of};
        use ryo_language::domain::symbols::types::ByteSlice;
        use ryo_language::inbound::use_cases::parse::types::{FieldRequest, NoteRequest, ParseRequest};
        use ryo_relations::domain::relations::types::{CellKind, FactColumn, FileId, Position};
        use ryo_relations::domain::terms::types::Symbol;
        let prog = self.prog;
        let wanted: Vec<_> = prog.facts.iter().filter(|f| f.model.is_none()).filter_map(|f| f.relation).collect();
        self.c.provider_files += 1;
        let p = self.provider.as_deref_mut().expect("a relation only the provider serves");
        if p.columns.len() != prog.facts.len() {
            p.columns = prog.facts.iter().map(|f| (0..f.kinds.len()).map(|_| FactColumn(Vec::new())).collect()).collect();
        }
        for rel in &mut p.columns {
            for c in rel.iter_mut() {
                c.0.clear();
            }
        }
        let bytes = self.f.source.as_bytes();
        let t = Instant::now();
        clear_symbols(&mut p.symbols);
        let request = ParseRequest {
            path: ryo_language::domain::files::types::SourcePath(self.f.path),
            file: FileId(self.f.file as u32),
            bytes: ByteSlice(bytes),
            notes: NoteRequest::Unwanted,
            fields: FieldRequest::Wanted,
        };
        let mut out: Vec<Vec<Vec<u64>>> = Vec::new();
        if p.engine.build_file(request, &mut p.elements, &mut p.symbols, &wanted).is_err() {
            return prog.facts.iter().map(|f| vec![Vec::new(); f.kinds.len()]).collect();
        }
        self.c.provider_build_ns += t.elapsed().as_nanos() as u64;
        {
            let mut cols: ryo_execution_runtime::outbound::facts::types::ColumnsMut<'_> = prog
                .facts
                .iter()
                .zip(p.columns.iter_mut())
                .filter(|(f, _)| f.model.is_none() && f.relation.is_some())
                .map(|(f, c)| (f.relation.unwrap(), c.as_mut_slice()))
                .collect();
            p.engine.walk_file(&mut p.elements, &mut p.symbols, ByteSlice(bytes), FileId(self.f.file as u32), &wanted, &mut cols);
        }
        // decode: provider positions -> view ids by (start, end, kind name); provider symbols -> the run's codes
        let v = self.f.view;
        let mut at: FxMap<(u32, u32, u32), u32> = FxMap::default();
        for id in 0..v.len() {
            at.insert((v.start[id], v.stop[id], v.kind[id]), id as u32);
        }
        // a node and its first descendant can share span and kind only through tokens: keep the AST node (lower id)
        for id in (0..v.n_nodes).rev() {
            at.insert((v.start[id], v.stop[id], v.kind[id]), id as u32);
        }
        let names: Vec<u32> = kind_names(&p.elements)
            .iter()
            .map(|s| text_of(&p.symbols, ByteSlice(bytes), *s).and_then(|t| self.sym.get_base(t.0)).unwrap_or(NULL))
            .collect();
        let mut pos_memo: FxMap<u32, u32> = FxMap::default();
        let mut sym_memo: FxMap<u32, u32> = FxMap::default();
        for (fi, f) in prog.facts.iter().enumerate() {
            let mut rel_cols = Vec::with_capacity(f.kinds.len());
            for (ci, kind) in f.kinds.iter().enumerate() {
                let src = &p.columns[fi][ci].0;
                let col: Vec<u64> = match kind {
                    CellKind::Node => src
                        .iter()
                        .map(|&w| {
                            let pos = (w & 0xffff_ffff) as u32;
                            *pos_memo.entry(pos).or_insert_with(|| {
                                let position = Position(pos);
                                if (pos as usize) >= ryo_language::domain::elements::service::element_count(&p.elements).0 {
                                    return u32::MAX - 1;
                                }
                                let k = kind_at(&p.elements, position).0 as usize;
                                let name = names.get(k).copied().unwrap_or(NULL);
                                at.get(&(start_at(&p.elements, position).0, end_at(&p.elements, position).0, name)).copied().unwrap_or(u32::MAX - 1)
                            }) as u64
                        })
                        .collect(),
                    CellKind::Symbol => src
                        .iter()
                        .map(|&w| {
                            let s = w as u32;
                            *sym_memo.entry(s).or_insert_with(|| match text_of(&p.symbols, ByteSlice(bytes), Symbol(s)) {
                                Some(t) => self.sym.text(t.0),
                                None => NULL,
                            }) as u64
                        })
                        .collect(),
                    CellKind::File => vec![0; src.len()],
                    _ => src.clone(),
                };
                self.c.fact_rows += if ci == 0 { col.len() as u64 } else { 0 };
                rel_cols.push(col);
            }
            if f.name.starts_with("filesystem.") {
                rel_cols = host_filesystem(&f.name, f.kinds.len(), self.f.path);
            }
            out.push(rel_cols);
        }
        out
    }


    fn fact_index(&mut self, rel: u16, col: u8) -> Rc<FIdx> {
        self.ensure(rel);
        let n_view = self.f.view.len();
        let dense = matches!(self.prog.facts[rel as usize].kinds[col as usize], ryo_relations::domain::relations::types::CellKind::Node);
        let facts = self.cache.facts.as_mut().unwrap();
        if let Some(i) = facts.idx.get(&(rel, col)) {
            return i.clone();
        }
        let t0 = Instant::now();
        let c = &facts.cols[rel as usize][col as usize];
        let idx = if dense {
            let mut head = vec![u32::MAX; n_view + 1];
            let mut next = vec![u32::MAX; c.len()];
            for (row, &v) in c.iter().enumerate().rev() {
                let k = (v as usize).min(n_view);
                next[row] = head[k];
                head[k] = row as u32;
            }
            FIdx::Dense { head, next }
        } else {
            let mut m: FxMap<u64, Vec<u32>> = FxMap::default();
            for (row, &v) in c.iter().enumerate() {
                m.entry(v).or_default().push(row as u32);
            }
            FIdx::Hash(m)
        };
        self.c.made_tables += 1;
        self.c.made_rows += c.len() as u64;
        self.c.index_ns += t0.elapsed().as_nanos() as u64;
        self.c.index_split[2] += t0.elapsed().as_nanos() as u64;
        let m = Rc::new(idx);
        facts.idx.insert((rel, col), m.clone());
        m
    }

    // ---- tests
    fn opd(&self, o: &Opd) -> u64 {
        match o {
            Opd::Reg(r) => self.regs[*r as usize],
            Opd::Num(n) => *n as u64,
            Opd::Sym(s) => *s as u64,
            Opd::File => 0,
            _ => 0,
        }
    }

    fn text_of(&self, o: &Opd, v: u64) -> Option<&str> {
        if matches!(o, Opd::File) {
            return Some(self.f.path);
        }
        self.sym.str(v as u32)
    }

    fn test(&self, test: T, l: &Opd, r: &Opd, text: bool, num: bool, regex: &[usize]) -> bool {
        let lv = self.opd(l);
        match test {
            T::Eq | T::Ne | T::Lt | T::Le | T::Gt | T::Ge => {
                let rv = self.opd(r);
                if test == T::Eq {
                    return lv == rv;
                }
                if test == T::Ne {
                    return lv != rv;
                }
                let ord = if text {
                    if lv == rv {
                        std::cmp::Ordering::Equal
                    } else {
                        match (self.sym.str(lv as u32), self.sym.str(rv as u32)) {
                            (Some(a), Some(b)) => a.cmp(b),
                            _ => lv.cmp(&rv),
                        }
                    }
                } else if num {
                    (lv as i64).cmp(&(rv as i64))
                } else {
                    lv.cmp(&rv)
                };
                match test {
                    T::Lt => ord.is_lt(),
                    T::Le => ord.is_le(),
                    T::Gt => ord.is_gt(),
                    _ => ord.is_ge(),
                }
            }
            T::In | T::NotIn => {
                let inside = match r {
                    Opd::Syms(s) => s.binary_search(&(lv as u32)).is_ok(),
                    Opd::Nums(n) => n.binary_search(&(lv as i64)).is_ok(),
                    _ => lv == self.opd(r),
                };
                inside == (test == T::In)
            }
            T::Matches | T::Glob | T::NotGlob => {
                let hit = self.text_of(l, lv).is_some_and(|t| regex.iter().any(|&i| self.prog.regexes[i].is_match(t)));
                if test == T::NotGlob { !hit } else { hit }
            }
            T::NumGe | T::NatLt | T::Succ => self.test_numeric(test, l, r, lv, text, num),
            T::Starts | T::Ends => {
                let rv = self.opd(r);
                match (self.text_of(l, lv), self.text_of(r, rv)) {
                    (Some(a), Some(b)) => {
                        if test == T::Starts { a.starts_with(b) } else { a.ends_with(b) }
                    }
                    _ => false,
                }
            }
        }
    }

    fn iter_fact_list(&mut self, p: &'p [Ins], pc: usize, idx: &FIdx, keys: &[u64], out: R) -> bool {
        for &k in keys {
            match idx {
                FIdx::Dense { head, next } => {
                    if k as usize >= head.len() - 1 {
                        continue;
                    }
                    let mut row = head[k as usize];
                    while row != u32::MAX {
                        self.regs[out as usize] = row as u64;
                        self.c.visits += 1;
                        if self.run_inner(p, pc + 1) {
                            return true;
                        }
                        row = next[row as usize];
                    }
                }
                FIdx::Hash(m) => {
                    if let Some(list) = m.get(&k) {
                        for &row in list {
                            self.regs[out as usize] = row as u64;
                            self.c.visits += 1;
                            if self.run_inner(p, pc + 1) {
                                return true;
                            }
                        }
                    }
                }
            }
        }
        false
    }

    /// Runs `p` from `pc`; returns true when a sub-program reached Yield (the caller stops enumerating).
    pub(crate) fn run(&mut self, p: &'p [Ins], pc: usize) -> bool {
        let mut pc = pc;
        let steps0 = self.c.steps;
        let visits0 = self.c.visits;
        let flow = self.run_inner(p, pc);
        let r = self.cur_rule;
        self.c.rule_steps[r] = self.c.rule_steps[r].wrapping_add(self.c.steps - steps0);
        self.c.rule_visits[r] = self.c.rule_visits[r].wrapping_add(self.c.visits - visits0);
        let _ = &mut pc;
        flow
    }

    /// Batched anchor loop (NAV_BATCH=0 turns it off): `p` runs over every candidate of a bucket at once; its straight-line
    /// reads and tests are loops over a selection vector, a Fork hands the survivors to each enabled child, and the first
    /// instruction that is neither runs per survivor with the batch's registers restored.
    pub(crate) fn run_batched(&mut self, p: &'p [Ins], anchor: R, ids: &[u32]) {
        let mut cols: Vec<(R, Vec<u64>)> = vec![(anchor, ids.iter().map(|&x| u64::from(x)).collect())];
        let sel: Vec<u32> = (0..ids.len() as u32).collect();
        self.batch(p, 0, &mut cols, sel);
    }

    fn batch(&mut self, p: &'p [Ins], pc0: usize, cols: &mut Vec<(R, Vec<u64>)>, mut sel: Vec<u32>) {
        let base = cols.len();
        let col = |cols: &[(R, Vec<u64>)], r: R| cols.iter().rposition(|c| c.0 == r);
        let mut pc = pc0;
        let v = self.f.view;
        loop {
            if sel.is_empty() {
                break;
            }
            match &p[pc] {
                Ins::ReadCol { which, row, out } if col(cols, *row).is_some() => {
                    let ci = col(cols, *row).unwrap();
                    self.c.steps += sel.len() as u64;
                    self.c.batched_steps += sel.len() as u64;
                    let src = &cols[ci].1;
                    let mut o = vec![0u64; src.len()];
                    let a: &[u32] = match which {
                        0 => &v.kind,
                        1 => &v.text,
                        2 => &v.start,
                        _ => &v.stop,
                    };
                    sel.retain(|&k| {
                        let i = src[k as usize] as usize;
                        if i < a.len() {
                            o[k as usize] = u64::from(a[i]);
                            true
                        } else {
                            false
                        }
                    });
                    cols.push((*out, o));
                }
                Ins::ReadChild { which, row, out } if col(cols, *row).is_some() => {
                    let ci = col(cols, *row).unwrap();
                    self.c.steps += sel.len() as u64;
                    self.c.batched_steps += sel.len() as u64;
                    let src = &cols[ci].1;
                    let mut o = vec![0u64; src.len()];
                    let a: &[u32] = match which {
                        0 => &v.parent,
                        1 => &v.ordinal,
                        _ => &v.field,
                    };
                    sel.retain(|&k| {
                        let i = src[k as usize] as usize;
                        if i < v.parent.len() && v.parent[i] != u32::MAX {
                            o[k as usize] = u64::from(a[i]);
                            true
                        } else {
                            false
                        }
                    });
                    cols.push((*out, o));
                }
                Ins::Row { rel, key } if col(cols, *key).is_some() && *rel != TRel::Id => {
                    let ci = col(cols, *key).unwrap();
                    self.c.steps += sel.len() as u64;
                    self.c.batched_steps += sel.len() as u64;
                    let src = &cols[ci].1;
                    let rel = *rel;
                    sel.retain(|&k| {
                        let i = src[k as usize] as usize;
                        i < v.parent.len() && (rel != TRel::Sc || v.parent[i] != u32::MAX)
                    });
                }
                Ins::TestEqC { reg, c } if col(cols, *reg).is_some() => {
                    let src = &cols[col(cols, *reg).unwrap()].1;
                    self.c.steps += sel.len() as u64;
                    self.c.batched_steps += sel.len() as u64;
                    sel.retain(|&k| src[k as usize] == *c);
                }
                Ins::TestNeC { reg, c } if col(cols, *reg).is_some() => {
                    let src = &cols[col(cols, *reg).unwrap()].1;
                    self.c.steps += sel.len() as u64;
                    self.c.batched_steps += sel.len() as u64;
                    sel.retain(|&k| src[k as usize] != *c);
                }
                Ins::TestInC { reg, set, negate } if col(cols, *reg).is_some() => {
                    let src = &cols[col(cols, *reg).unwrap()].1;
                    self.c.steps += sel.len() as u64;
                    self.c.batched_steps += sel.len() as u64;
                    sel.retain(|&k| {
                        let x = src[k as usize];
                        (x <= u64::from(u32::MAX) && set.binary_search(&(x as u32)).is_ok()) != *negate
                    });
                }
                Ins::TestEqR { a, b } if col(cols, *a).is_some() && col(cols, *b).is_some() => {
                    let (x, y) = (&cols[col(cols, *a).unwrap()].1, &cols[col(cols, *b).unwrap()].1);
                    self.c.steps += sel.len() as u64;
                    self.c.batched_steps += sel.len() as u64;
                    sel.retain(|&k| x[k as usize] == y[k as usize]);
                }
                Ins::TestNeR { a, b } if col(cols, *a).is_some() && col(cols, *b).is_some() => {
                    let (x, y) = (&cols[col(cols, *a).unwrap()].1, &cols[col(cols, *b).unwrap()].1);
                    self.c.steps += sel.len() as u64;
                    self.c.batched_steps += sel.len() as u64;
                    sel.retain(|&k| x[k as usize] != y[k as usize]);
                }
                Ins::Fork { subs } => {
                    self.c.steps += sel.len() as u64;
                    self.c.batched_steps += sel.len() as u64;
                    for (slots, q) in subs.iter() {
                        if !slots.iter().any(|&s| self.cache.enabled[s as usize]) {
                            continue;
                        }
                        self.batch(q, 0, cols, sel.clone());
                    }
                    break;
                }
                _ => {
                    for &k in &sel {
                        for (r, vals) in cols.iter() {
                            self.regs[*r as usize] = vals[k as usize];
                        }
                        self.run_inner(p, pc);
                    }
                    break;
                }
            }
            pc += 1;
        }
        cols.truncate(base);
    }

    fn run_inner(&mut self, p: &'p [Ins], pc: usize) -> bool {
        let mut pc = pc;
        loop {
            self.c.steps += 1;
            if self.c.diag {
                if self.c.op_steps.len() < NINS {
                    self.c.op_steps.resize(NINS, 0);
                }
                self.c.op_steps[p[pc].code()] += 1;
                if let Some(prof) = self.c.prof.as_mut() {
                    *prof.entry(&p[pc] as *const Ins as usize).or_default() += 1;
                }
            }
            match &p[pc] {
                Ins::ReadCol { which, row, out } => {
                    let i = self.regs[*row as usize] as usize;
                    let v = self.f.view;
                    if i >= v.kind.len() {
                        return false;
                    }
                    self.regs[*out as usize] = u64::from(match which {
                        0 => v.kind[i],
                        1 => v.text[i],
                        2 => v.start[i],
                        _ => v.stop[i],
                    });
                }
                Ins::ReadChild { which, row, out } => {
                    let i = self.regs[*row as usize] as usize;
                    let v = self.f.view;
                    if i >= v.parent.len() || v.parent[i] == u32::MAX {
                        return false;
                    }
                    self.regs[*out as usize] = u64::from(match which {
                        0 => v.parent[i],
                        1 => v.ordinal[i],
                        _ => v.field[i],
                    });
                }
                Ins::TestEqC { reg, c } => {
                    if self.regs[*reg as usize] != *c {
                        return false;
                    }
                }
                Ins::TestNeC { reg, c } => {
                    if self.regs[*reg as usize] == *c {
                        return false;
                    }
                }
                Ins::TestEqR { a, b } => {
                    if self.regs[*a as usize] != self.regs[*b as usize] {
                        return false;
                    }
                }
                Ins::TestNeR { a, b } => {
                    if self.regs[*a as usize] == self.regs[*b as usize] {
                        return false;
                    }
                }
                Ins::TestInC { reg, set, negate } => {
                    let x = self.regs[*reg as usize];
                    let inside = x <= u64::from(u32::MAX) && set.binary_search(&(x as u32)).is_ok();
                    if inside == *negate {
                        return false;
                    }
                }
                Ins::Read { rel, col, row, out } => {
                    let id = self.regs[*row as usize] as u32;
                    match self.tread(*rel, *col, id) {
                        Some(v) => self.regs[*out as usize] = v,
                        None => return false,
                    }
                }
                Ins::Test { test, l, r, text, num, regex } => {
                    if !self.test(*test, l, r, *text, *num, regex) {
                        return false;
                    }
                }
                Ins::Row { rel, key } => {
                    let id = self.regs[*key as usize] as u32;
                    if !self.row_exists(*rel, id) {
                        return false;
                    }
                }
                Ins::ReadFact { rel, col, row, out } => {
                    let r = self.regs[*row as usize] as usize;
                    let v = self.cache.facts.as_ref().unwrap().cols[*rel as usize][*col as usize][r];
                    self.regs[*out as usize] = v;
                }
                Ins::IterChildren { parent, out } => {
                    let par = self.regs[*parent as usize] as u32;
                    let v = self.f.view;
                    if par as usize >= v.len() {
                        return false;
                    }
                    let (s, e) = (v.child_off[par as usize] as usize, v.child_off[par as usize + 1] as usize);
                    for k in s..e {
                        self.regs[*out as usize] = v.children[k] as u64;
                        self.c.visits += 1;
                        if self.run_inner(p, pc + 1) {
                            return true;
                        }
                    }
                    return false;
                }
                Ins::IterKind { kinds, out } => {
                    for &k in kinds.iter() {
                        let (s, e) = {
                            let kk = k as usize;
                            if kk + 1 >= self.cache.kind_off.len() {
                                continue;
                            }
                            (self.cache.kind_off[kk] as usize, self.cache.kind_off[kk + 1] as usize)
                        };
                        for i in s..e {
                            self.regs[*out as usize] = self.cache.kind_ids[i] as u64;
                            self.c.visits += 1;
                            if self.run_inner(p, pc + 1) {
                                return true;
                            }
                        }
                    }
                    return false;
                }
                Ins::IterValue { rel, col, val, out } if !matches!(val, Val::Reg(_)) => {
                    let list = self.const_list(*rel, *col, val, false);
                    for &id in list.iter() {
                        self.regs[*out as usize] = id as u64;
                        self.c.visits += 1;
                        if self.run_inner(p, pc + 1) {
                            return true;
                        }
                    }
                    return false;
                }
                Ins::IterValue { rel, col, val, out } => {
                    let idx = self.tree_index(*rel, *col);
                    let one;
                    let keys: &[u32] = match val {
                        Val::Reg(r) => {
                            one = [self.regs[*r as usize] as u32];
                            &one
                        }
                        Val::Syms(s) => s,
                        Val::Nums(_) => unreachable!("a constant value walk takes the constant list"),
                    };
                    for &k in keys {
                        for &id in idx.get(k) {
                            self.regs[*out as usize] = id as u64;
                            self.c.visits += 1;
                            if self.run_inner(p, pc + 1) {
                                return true;
                            }
                        }
                    }
                    return false;
                }
                Ins::IterScan { rel, out } => {
                    let n = if *rel == TRel::Id { self.f.view.n_nodes } else { self.f.view.len() };
                    for id in 0..n {
                        if !self.row_exists(*rel, id as u32) {
                            continue;
                        }
                        self.regs[*out as usize] = id as u64;
                                self.c.visits += 1;
                        if self.run_inner(p, pc + 1) {
                            return true;
                        }
                    }
                    return false;
                }
                Ins::IterFact { rel, col, val: Val::Reg(r), out } if self.prog.facts[*rel as usize].keyed.is_some_and(|k| k.0 == *col) => {
                    let (a, b) = self.keyed_rows(*rel, self.regs[*r as usize]);
                    for row in a..b {
                        self.regs[*out as usize] = u64::from(row);
                        self.c.visits += 1;
                        if self.run_inner(p, pc + 1) {
                            return true;
                        }
                    }
                    return false;
                }
                Ins::IterFact { rel, col, val, out } => {
                    let idx = self.fact_index(*rel, *col);
                    let one;
                    let keys: &[u64] = match val {
                        Val::Reg(r) => {
                            one = [self.regs[*r as usize]];
                            &one
                        }
                        Val::Syms(s) if s.len() == 1 => {
                            one = [u64::from(s[0])];
                            &one
                        }
                        Val::Syms(s) => {
                            one = [0];
                            let _ = one;
                            return self.iter_fact_list(p, pc, &idx, &s.iter().map(|&x| x as u64).collect::<Vec<_>>(), *out);
                        }
                        Val::Nums(n) => {
                            one = [0];
                            let _ = one;
                            return self.iter_fact_list(p, pc, &idx, &n.iter().map(|&x| x as u64).collect::<Vec<_>>(), *out);
                        }
                    };
                    return self.iter_fact_list(p, pc, &idx, keys, *out);
                }
                Ins::IterFactScan { rel, out } => {
                    self.ensure(*rel);
                    let n = self.cache.facts.as_ref().unwrap().cols[*rel as usize].first().map_or(0, Vec::len);
                    for row in 0..n {
                        self.regs[*out as usize] = row as u64;
                                self.c.visits += 1;
                        if self.run_inner(p, pc + 1) {
                            return true;
                        }
                    }
                    return false;
                }
                Ins::Axis { dir, start, out } => {
                    let s = self.regs[*start as usize] as u32;
                    let v = self.f.view;
                    if s as usize >= v.len() {
                        return false;
                    }
                    match dir {
                        Dir::Parent => {
                            let par = v.parent[s as usize];
                            if par == u32::MAX {
                                return false;
                            }
                            self.regs[*out as usize] = par as u64;
                        }
                        Dir::Ancestor => {
                            let mut a = v.parent[s as usize];
                            while a != u32::MAX {
                                self.regs[*out as usize] = a as u64;
                                self.c.visits += 1;
                                if self.run_inner(p, pc + 1) {
                                    return true;
                                }
                                a = v.parent[a as usize];
                            }
                            return false;
                        }
                        Dir::Child => {
                            let (b, e) = (v.child_off[s as usize] as usize, v.child_off[s as usize + 1] as usize);
                            for k in b..e {
                                self.regs[*out as usize] = v.children[k] as u64;
                        self.c.visits += 1;
                                if self.run_inner(p, pc + 1) {
                                    return true;
                                }
                            }
                            return false;
                        }
                        Dir::Descendant => {
                            let mut stack: Vec<u32> = Vec::new();
                            let (b, e) = (v.child_off[s as usize] as usize, v.child_off[s as usize + 1] as usize);
                            stack.extend(v.children[b..e].iter().rev());
                            while let Some(d) = stack.pop() {
                                self.regs[*out as usize] = d as u64;
                                self.c.visits += 1;
                                if self.run_inner(p, pc + 1) {
                                    return true;
                                }
                                let (b, e) = (v.child_off[d as usize] as usize, v.child_off[d as usize + 1] as usize);
                                stack.extend(v.children[b..e].iter().rev());
                            }
                            return false;
                        }
                    }
                }
                Ins::IterRange { rel, col, val, out, start, lo, hi } => {
                    let v = self.f.view;
                    let lo = lo.map(|(r, incl)| (self.regs[r as usize] as i64, incl));
                    let hi = hi.map(|(r, incl)| (self.regs[r as usize] as i64, incl));
                    let (tidx, clist) = match val {
                        Val::Reg(_) => (Some(self.tree_index(*rel, *col)), None),
                        _ => (None, Some(self.const_list(*rel, *col, val, true))),
                    };
                    let k1 = match val {
                        Val::Reg(r) => self.regs[*r as usize] as u32,
                        _ => 0,
                    };
                    {
                        let k = k1;
                        let list: &[u32] = match (&tidx, &clist) {
                            (Some(t), _) => t.get(k),
                            (_, Some(c)) => c,
                            _ => &[],
                        };
                        let first = match lo {
                            Some((x, true)) => list.partition_point(|&id| (v.start[id as usize] as i64) < x),
                            Some((x, false)) => list.partition_point(|&id| (v.start[id as usize] as i64) <= x),
                            None => 0,
                        };
                        for &id in &list[first..] {
                            let st = v.start[id as usize] as i64;
                            match hi {
                                Some((x, true)) if st > x => break,
                                Some((x, false)) if st >= x => break,
                                _ => {}
                            }
                            self.regs[*out as usize] = id as u64;
                            self.regs[*start as usize] = st as u64;
                            self.c.visits += 1;
                            if self.run_inner(p, pc + 1) {
                                return true;
                            }
                            if self.first_fail {
                                self.first_fail = false;
                                break;
                            }
                        }
                    }
                    return false;
                }
                Ins::IterField { parent, field, out, field_out } => {
                    let par = self.regs[*parent as usize] as u32;
                    let v = self.f.view;
                    if par as usize >= v.len() {
                        return false;
                    }
                    let (s, e) = (v.child_off[par as usize] as usize, v.child_off[par as usize + 1] as usize);
                    for k in s..e {
                        let ch = v.children[k];
                        if v.field[ch as usize] != *field {
                            continue;
                        }
                        self.regs[*out as usize] = ch as u64;
                        self.regs[*field_out as usize] = *field as u64;
                        self.c.visits += 1;
                        if self.run_inner(p, pc + 1) {
                            return true;
                        }
                    }
                    return false;
                }
                Ins::Hoist { id, key, seg, caps } => {
                    let hk = (*id, key.first().map_or(0, |&r| self.regs[r as usize]), key.get(1).map_or(0, |&r| self.regs[r as usize]));
                    // per site over the run: (lookups, misses); a site whose keys do not repeat stops memoising
                    let site = self.c.hoist_sites.entry(*id).or_default();
                    site.0 += 1;
                    let off = site.0 >= 256 && site.1 * 10 >= site.0 * 9;
                    let cached = if off { None } else { self.cache.hoisted.get(&hk).cloned() };
                    let tuples = match cached {
                        Some(t) => t,
                        None => {
                            let saved = std::mem::take(&mut self.cap_buf);
                            self.run_inner(seg, 0);
                            let t = Rc::new(std::mem::replace(&mut self.cap_buf, saved));
                            self.c.hoist_misses += 1;
                            self.c.hoist_sites.get_mut(id).unwrap().1 += 1;
                            if !off {
                                self.cache.hoisted.insert(hk, t.clone());
                            }
                            t
                        }
                    };
                    self.c.hoist_runs += 1;
                    let w = caps.len().max(1);
                    for tup in tuples.chunks(w) {
                        for (k, &r) in caps.iter().enumerate() {
                            self.regs[r as usize] = tup[k];
                        }
                        if caps.is_empty() && tup.is_empty() {
                            continue;
                        }
                        if self.run_inner(p, pc + 1) {
                            return true;
                        }
                    }
                    return false;
                }
                Ins::HoistProbe { id, key, seg, caps, probe } => {
                    let hk = (*id, key.first().map_or(0, |&r| self.regs[r as usize]), key.get(1).map_or(0, |&r| self.regs[r as usize]));
                    let idx = match self.cache.probed.get(&hk) {
                        Some(x) => x.clone(),
                        None => {
                            let saved = std::mem::take(&mut self.cap_buf);
                            self.run_inner(seg, 0);
                            let t = std::mem::replace(&mut self.cap_buf, saved);
                            self.c.hoist_misses += 1;
                            let w = caps.len().max(1);
                            let mut m: FxMap<u64, Vec<u64>> = FxMap::default();
                            for tup in t.chunks(w) {
                                m.entry(tup[probe.0 as usize]).or_default().extend_from_slice(tup);
                            }
                            let x = Rc::new(m);
                            self.cache.probed.insert(hk, x.clone());
                            x
                        }
                    };
                    self.c.hoist_runs += 1;
                    let want = self.regs[probe.1 as usize];
                    let Some(tuples) = idx.get(&want) else { return false };
                    let w = caps.len().max(1);
                    for tup in tuples.chunks(w) {
                        for (k, &r) in caps.iter().enumerate() {
                            self.regs[r as usize] = tup[k];
                        }
                        if self.run_inner(p, pc + 1) {
                            return true;
                        }
                    }
                    return false;
                }
                Ins::Capture { regs } => {
                    if regs.is_empty() {
                        self.cap_buf.push(0);
                    }
                    for &r in regs.iter() {
                        let x = self.regs[r as usize];
                        self.cap_buf.push(x);
                    }
                    return false;
                }
                Ins::Within { anc, desc } => {
                    let (a, d) = (self.regs[*anc as usize] as u32, self.regs[*desc as usize] as u32);
                    let v = self.f.view;
                    if d as usize >= v.len() || a as usize >= v.len() {
                        return false;
                    }
                    let mut x = v.parent[d as usize];
                    while x != u32::MAX && x != a {
                        x = v.parent[x as usize];
                    }
                    if x != a {
                        return false;
                    }
                }
                Ins::Near { start, out, subs } => {
                    let s = self.regs[*start as usize] as u32;
                    let v = self.f.view;
                    if s as usize >= v.len() {
                        return false;
                    }
                    let mut a = v.parent[s as usize];
                    let mut found = false;
                    while a != u32::MAX {
                        self.regs[*out as usize] = a as u64;
                        self.c.visits += 1;
                        if subs.iter().any(|q| {
                            self.c.subs += 1;
                            self.run_inner(q, 0)
                        }) {
                            found = true;
                            break;
                        }
                        a = v.parent[a as usize];
                    }
                    if !found {
                        return false;
                    }
                    self.regs[*out as usize] = a as u64;
                }
                Ins::Sub { negate, subs } => {
                    let any = subs.iter().any(|q| {
                        self.c.subs += 1;
                        self.run_inner(q, 0)
                    });
                    if any == *negate {
                        return false;
                    }
                }
                Ins::NearMemo { start, out, subs } => {
                    let s = self.regs[*start as usize] as u32;
                    match self.near_memo(s, *out, subs) {
                        Some(a) => self.regs[*out as usize] = u64::from(a),
                        None => return false,
                    }
                }
                Ins::SubKey { negate, key, subs } => {
                    let site = subs.as_ptr() as usize;
                    let k = self.regs[*key as usize];
                    let st = self.c.key_sites.entry(site).or_default();
                    st.0 += 1;
                    let off = st.0 >= 256 && st.1 * 10 >= st.0 * 9;
                    let hit = if off { None } else { self.cache.key_memo.get(&(site, k)).copied() };
                    let any = match hit {
                        Some(v) => v,
                        None => {
                            self.c.key_sites.get_mut(&site).unwrap().1 += 1;
                            let v = subs.iter().any(|q| {
                                self.c.subs += 1;
                                self.run_inner(q, 0)
                            });
                            if !off {
                                self.cache.key_memo.insert((site, k), v);
                            }
                            v
                        }
                    };
                    if any == *negate {
                        return false;
                    }
                }
                Ins::AncExists { negate, start, out, cond } => {
                    let s = self.regs[*start as usize] as u32;
                    if self.anc_exists(s, *out, cond) == *negate {
                        return false;
                    }
                }
                Ins::SubFirst { subs } => {
                    self.c.subs += 1;
                    if self.run_inner(&subs[0], 0) {
                        self.first_fail = true;
                        return false;
                    }
                }
                Ins::Emit { cols, rule } => {
                    let slot = if *rule == u32::MAX { self.cur_rule } else { *rule as usize };
                    for c in cols.iter() {
                        let v = self.opd(c);
                        self.outs[slot].push(v);
                    }
                    self.c.emitted += 1;
                    return false;
                }
                Ins::Yield => return true,
                Ins::Collect { regs } => {
                    for r in regs.iter() {
                        let v = self.regs[*r as usize];
                        self.collect.push(v);
                    }
                    return false;
                }
                Ins::Closure { src, out, xin, kout, nkeys, subs } => return self.closure(p, pc, *src, *out, *xin, kout, *nkeys, subs),
                Ins::Union { outs, subs, memo } => return self.union(p, pc, outs, subs, *memo),
                Ins::Let { from, out } => self.regs[*out as usize] = self.opd(from),
                Ins::SubMemo { negate, subs } => {
                    let key = subs.as_ptr() as usize;
                    let any = match self.sub_memo.get(&key) {
                        Some(&v) => v,
                        None => {
                            let v = subs.iter().any(|q| {
                                self.c.subs += 1;
                                self.run_inner(q, 0)
                            });
                            self.sub_memo.insert(key, v);
                            v
                        }
                    };
                    if any == *negate {
                        return false;
                    }
                }
                Ins::Sibling { next, start, out } => {
                    let s = self.regs[*start as usize] as u32;
                    let v = self.f.view;
                    if s as usize >= v.len() || v.parent[s as usize] == u32::MAX {
                        return false;
                    }
                    let p = v.parent[s as usize] as usize;
                    let kids = &v.children[v.child_off[p] as usize..v.child_off[p + 1] as usize];
                    let Some(i) = kids.iter().position(|&k| k == s) else { return false };
                    let j = if *next { i + 1 } else { i.wrapping_sub(1) };
                    match kids.get(j) {
                        Some(&k) => self.regs[*out as usize] = u64::from(k),
                        None => return false,
                    }
                }
                Ins::TextFn { lower, args, out } => {
                    let mut t = String::new();
                    for a in args.iter() {
                        let v = self.opd(a);
                        match self.text_of(a, v) {
                            Some(x) => t.push_str(x),
                            None => return false,
                        }
                    }
                    if *lower {
                        t = t.to_lowercase();
                    }
                    self.regs[*out as usize] = self.sym.minted(t) as u64;
                }
                Ins::CaptureText { text, pattern, outs } => return self.capture(p, pc, text, pattern, outs),
                Ins::NearDerived { start, out, xin, edges, domain } => {
                    match self.near_derived(*start, *out, *xin, edges, domain) {
                        Some(n) => self.regs[*out as usize] = n,
                        None => return false,
                    }
                }
                Ins::Replace { text, regex, replacement, out } => {
                    let v = self.opd(text);
                    let r = match self.text_of(text, v).map(str::to_owned) {
                        None => NULL,
                        Some(t) => {
                            let mut result = String::new();
                            let mut last = 0;
                            for m in self.prog.regexes[*regex].find_iter(t.as_bytes()) {
                                result.push_str(&t[last..m.start()]);
                                result.push_str(replacement);
                                last = m.end();
                            }
                            result.push_str(&t[last..]);
                            self.sym.minted(result)
                        }
                    };
                    self.regs[*out as usize] = r as u64;
                }
                Ins::Shared { idx } => {
                    // an indirection, not a step of the plan
                    self.c.steps -= 1;
                    let q: &'p [Ins] = &self.prog.shared[*idx as usize];
                    return self.run_inner(q, 0);
                }
                Ins::Fork { subs } => {
                    for (slots, q) in subs.iter() {
                        if !slots.iter().any(|&s| self.cache.enabled[s as usize]) {
                            continue;
                        }
                        if self.c.diag {
                            // NAV_DIAG: a child owned by one rule charges its steps to that rule (run() charges the rest)
                            let r0 = self.prog.slot_rule[slots[0] as usize] as usize;
                            if self.cur_rule != r0 && slots.iter().all(|&s| self.prog.slot_rule[s as usize] as usize == r0) {
                                let (s0, v0, saved) = (self.c.steps, self.c.visits, self.cur_rule);
                                self.cur_rule = r0;
                                let y = self.run_inner(q, 0);
                                self.cur_rule = saved;
                                let (ds, dv) = (self.c.steps - s0, self.c.visits - v0);
                                self.c.rule_steps[r0] += ds;
                                self.c.rule_visits[r0] += dv;
                                self.c.rule_steps[saved] = self.c.rule_steps[saved].wrapping_sub(ds);
                                self.c.rule_visits[saved] = self.c.rule_visits[saved].wrapping_sub(dv);
                                if y {
                                    return true;
                                }
                                continue;
                            }
                        }
                        if self.run_inner(q, 0) {
                            return true;
                        }
                    }
                    return false;
                }
                Ins::SpanBetween { a, b, out } => {
                    let (x, y) = (self.opd(a), self.opd(b));
                    self.regs[*out as usize] = (x & !0xffff_ffff) | (y & 0xffff_ffff);
                }
                Ins::SpanBounds { s, start, end } => {
                    let x = self.opd(s);
                    self.regs[*start as usize] = x >> 32;
                    self.regs[*end as usize] = x & 0xffff_ffff;
                }
                Ins::SourceMatches { span: sarg, base, regex, group, outs, capture_text, capture_line } => {
                    let sp = self.opd(sarg);
                    let base_line = base.as_ref().map(|b| self.opd(b));
                    let bytes = self.f.source.as_bytes();
                    let (s, e) = ((sp >> 32) as usize, (sp & 0xffff_ffff) as usize);
                    if e > bytes.len() || s > e {
                        return false;
                    }
                    let re = &self.prog.regexes[*regex];
                    let mut caps = re.create_captures();
                    let mut at = 0usize;
                    let hay = &bytes[s..e];
                    let line_at = |off: usize| -> u64 { base_line.unwrap_or(0) + bytes[s..off].iter().filter(|&&b| b == b'\n').count() as u64 };
                    while at <= hay.len() {
                        re.search_captures(&regex_automata::Input::new(hay).range(at..), &mut caps);
                        let Some(m) = caps.get_match() else { break };
                        let (ms, me) = (s + m.start(), s + m.end());
                        at = if m.end() > m.start() { m.end() } else { m.end() + 1 };
                        let mut k = 0;
                        self.regs[outs[k] as usize] = span(ms as u32, me as u32);
                        k += 1;
                        if base_line.is_some() {
                            self.regs[outs[k] as usize] = line_at(ms);
                            k += 1;
                        }
                        if let Some(g) = group {
                            let Some(cg) = caps.get_group(*g) else { continue };
                            let (cs, ce) = (s + cg.start, s + cg.end);
                            self.regs[outs[k] as usize] = span(cs as u32, ce as u32);
                            k += 1;
                            if *capture_text {
                                let t = String::from_utf8_lossy(&bytes[cs..ce]).to_string();
                                self.regs[outs[k] as usize] = self.sym.minted(t) as u64;
                                k += 1;
                            }
                            if *capture_line {
                                self.regs[outs[k] as usize] = line_at(cs);
                            }
                        }
                        self.c.visits += 1;
                        if self.run_inner(p, pc + 1) {
                            return true;
                        }
                    }
                    return false;
                }
            }
            pc += 1;
        }
    }

    /// Ins::Closure (navwide): breadth-first from the start node over the edge sub-programs, per key partition, each reached
    /// (keys, node) once; then the rest of the program per reached pair
    #[allow(clippy::too_many_arguments)]
    fn closure(&mut self, p: &'p [Ins], pc: usize, src: R, out: R, xin: R, kout: &[R], nkeys: usize, subs: &'p [Prog]) -> bool {
        let w = 1 + nkeys;
        let start = self.regs[src as usize];
        let mut seen: std::collections::HashSet<Vec<u64>, std::hash::BuildHasherDefault<crate::sema::Fx>> = Default::default();
        let mut found: Vec<u64> = Vec::new();
        let mut stack: Vec<(u64, Vec<u64>)> = Vec::new();
        let saved = std::mem::take(&mut self.collect);
        let saved_x = self.regs[xin as usize];
        let mut first = true;
        stack.push((start, Vec::new()));
        while let Some((n, keys)) = stack.pop() {
            self.regs[xin as usize] = n;
            self.collect.clear();
            for q in subs {
                self.c.subs += 1;
                self.run_inner(q, 0);
            }
            let rows = std::mem::take(&mut self.collect);
            for row in rows.chunks(w) {
                if !first && row[1..] != keys[..] {
                    continue;
                }
                let mut k = row[1..].to_vec();
                k.push(row[0]);
                if seen.insert(k) {
                    self.c.visits += 1;
                    found.extend_from_slice(row);
                    stack.push((row[0], row[1..].to_vec()));
                }
            }
            self.collect = rows;
            first = false;
        }
        self.collect = saved;
        self.regs[xin as usize] = saved_x;
        for row in found.chunks(w) {
            self.regs[out as usize] = row[0];
            for (i, r) in kout.iter().enumerate() {
                self.regs[*r as usize] = row[1 + i];
            }
            if self.run_inner(p, pc + 1) {
                return true;
            }
        }
        false
    }

    /// Ins::NearMemo: the nearest proper ancestor of `s` whose condition holds, memoised per file and site; every node passed
    /// on the way up shares the answer
    fn near_memo(&mut self, s: u32, out: R, subs: &'p [Prog]) -> Option<u32> {
        const UNKNOWN: u32 = u32::MAX - 1;
        const NONE: u32 = u32::MAX;
        let v = self.f.view;
        if s as usize >= v.len() {
            return None;
        }
        let site = subs.as_ptr() as usize;
        let mut memo = self.cache.near_memo.remove(&site).unwrap_or_else(|| {
            self.c.memo_allocs += 1;
            self.c.memo_cells += v.len() as u64;
            vec![UNKNOWN; v.len()]
        });
        let mut stack: Vec<u32> = Vec::new();
        let mut x = s;
        let res = loop {
            let m = memo[x as usize];
            if m != UNKNOWN {
                break m;
            }
            let p = v.parent[x as usize];
            if p == u32::MAX {
                memo[x as usize] = NONE;
                break NONE;
            }
            self.regs[out as usize] = u64::from(p);
            self.c.visits += 1;
            if subs.iter().any(|q| {
                self.c.subs += 1;
                self.run_inner(q, 0)
            }) {
                memo[x as usize] = p;
                break p;
            }
            stack.push(x);
            x = p;
        };
        for y in stack {
            memo[y as usize] = res;
        }
        self.cache.near_memo.insert(site, memo);
        (res != NONE).then_some(res)
    }

    /// Ins::AncExists: whether some proper ancestor of `s` passes `cond`, memoised per file and site by node
    fn anc_exists(&mut self, s: u32, out: R, cond: &'p [Ins]) -> bool {
        const UNKNOWN: u32 = u32::MAX - 1;
        let v = self.f.view;
        if s as usize >= v.len() {
            return false;
        }
        let site = cond.as_ptr() as usize;
        let mut memo = self.cache.near_memo.remove(&site).unwrap_or_else(|| {
            self.c.memo_allocs += 1;
            self.c.memo_cells += v.len() as u64;
            vec![UNKNOWN; v.len()]
        });
        let mut stack: Vec<u32> = Vec::new();
        let mut x = s;
        let res = loop {
            let m = memo[x as usize];
            if m != UNKNOWN {
                break m;
            }
            let p = v.parent[x as usize];
            if p == u32::MAX {
                memo[x as usize] = 0;
                break 0;
            }
            self.regs[out as usize] = u64::from(p);
            self.c.visits += 1;
            self.c.subs += 1;
            if self.run_inner(cond, 0) {
                memo[x as usize] = 1;
                break 1;
            }
            stack.push(x);
            x = p;
        };
        for y in stack {
            memo[y as usize] = res;
        }
        self.cache.near_memo.insert(site, memo);
        res == 1
    }

    /// Ins::Union (navwide): every sub-program's collected tuples, distinct, each continuing the program
    fn union(&mut self, p: &'p [Ins], pc: usize, outs: &[R], subs: &'p [Prog], memo: bool) -> bool {
        let w = outs.len();
        let key = subs.as_ptr() as usize;
        let rows: Rc<Vec<u64>> = match self.union_memo.get(&key) {
            Some(r) if memo => r.clone(),
            _ => {
                let saved = std::mem::take(&mut self.collect);
                for q in subs {
                    self.c.subs += 1;
                    self.run_inner(q, 0);
                }
                let rows = std::mem::replace(&mut self.collect, saved);
                let mut seen: std::collections::HashSet<&[u64], std::hash::BuildHasherDefault<crate::sema::Fx>> = Default::default();
                let mut distinct = Vec::with_capacity(rows.len());
                for row in rows.chunks(w.max(1)) {
                    if seen.insert(row) {
                        distinct.extend_from_slice(row);
                    }
                }
                let r = Rc::new(distinct);
                if memo {
                    self.union_memo.insert(key, r.clone());
                }
                r
            }
        };
        for row in rows.chunks(w.max(1)) {
            for (i, r) in outs.iter().enumerate() {
                self.regs[*r as usize] = row[i];
            }
            self.c.visits += 1;
            if self.run_inner(p, pc + 1) {
                return true;
            }
        }
        false
    }

    /// navwide: `numeric>=` and `natural<`; a number register is a number, a text register is read as canonical numeric text
    fn test_numeric(&self, test: T, l: &Opd, r: &Opd, lv: u64, text: bool, num: bool) -> bool {
        let rv = self.opd(r);
        let kind = |o: &Opd| -> bool { matches!(o, Opd::Num(_)) || (matches!(o, Opd::Reg(_)) && num && !text) };
        if test == T::NatLt {
            return match (self.text_of(l, lv), self.text_of(r, rv)) {
                (Some(a), Some(b)) => natural_compare(a, b).is_lt(),
                _ => false,
            };
        }
        let read = |o: &Opd, v: u64| -> Option<Numeric> {
            if kind(o) {
                return Some(Numeric::Int(i128::from(v as i64)));
            }
            read_numeric(self.text_of(o, v)?)
        };
        match (read(l, lv), read(r, rv)) {
            (Some(Numeric::Int(a)), Some(Numeric::Int(b))) if test == T::Succ => b == a + 1,
            (Some(_), Some(_)) if test == T::Succ => false,
            (Some(a), Some(b)) => numeric_cmp(a, b).is_some_and(|o| o.is_ge()),
            _ => false,
        }
    }

    /// navwide: the rows of a rule's auxiliary relations (scan programs), for its tail
    pub(crate) fn run_aux(&mut self, rule: usize) -> Vec<(u32, Vec<u8>, Vec<u64>)> {
        let prog = self.prog;
        let slot = self.outs.len() - 1;
        let mut out = Vec::new();
        for (merged, progs, kinds) in &prog.rules[rule].aux {
            let saved = (self.cur_rule, std::mem::take(&mut self.outs[slot]));
            self.cur_rule = slot;
            for q in progs {
                self.run_inner(q, 0);
            }
            let rows = std::mem::replace(&mut self.outs[slot], saved.1);
            self.cur_rule = saved.0;
            out.push((*merged, kinds.clone(), rows));
        }
        out
    }

    /// navwide: one derived relation (and the relation computed in the same pass) from derived.rs's rows form over the node
    /// table, decoded into the view's ids and the run's text codes
    fn load_derived(&mut self, rel: u16) {
        let t = Instant::now();
        let rows0 = self.c.fact_rows;
        self.load_derived_inner(rel);
        let e = self.c.derived_by.entry(rel).or_default();
        e.0 += 1;
        e.1 += t.elapsed().as_nanos() as u64;
        e.2 += self.c.fact_rows - rows0;
    }

    fn load_derived_inner(&mut self, rel: u16) {
        let t = Instant::now();
        if !self.cache.model_lex {
            let t = Instant::now();
            self.model.build_lexical(self.f.table, self.f.incomplete);
            self.model.resolve_loads(self.f.table);
            self.cache.model_lex = true;
            self.c.model_lex_ns += t.elapsed().as_nanos() as u64;
            self.c.model_files += 1;
        }
        let prog = self.prog;
        let name = prog.facts[rel as usize].name.as_str();
        if name.starts_with("filesystem.") {
            // the host's relations of the file (derived.rs has file and executable; a read failure never reaches a rule here)
            let cols = host_filesystem(name, prog.facts[rel as usize].kinds.len(), self.f.path);
            let facts = self.cache.facts.as_mut().unwrap();
            facts.cols[rel as usize] = cols;
            facts.loaded[rel as usize] = true;
            self.c.derived_ns += t.elapsed().as_nanos() as u64;
            return;
        }
        if name == "comment" || name == "indent-region" {
            // served from the run's own view (derived.rs reads the same view type, built a second time)
            let cols = self.view_rows(name, prog.facts[rel as usize].kinds.len());
            let facts = self.cache.facts.as_mut().unwrap();
            self.c.fact_rows += cols.first().map_or(0, Vec::len) as u64;
            facts.cols[rel as usize] = cols;
            facts.loaded[rel as usize] = true;
            self.c.derived_files += 1;
            self.c.derived_ns += t.elapsed().as_nanos() as u64;
            return;
        }
        let di = crate::derived::index_of(name).unwrap();
        let mut targets: Vec<(usize, u16)> = vec![(di, rel)];
        if let Some(p) = crate::derived::partner(di) {
            if let Some(pi) = prog.facts.iter().position(|f| f.name == crate::derived::RELATIONS[p] && f.made.is_none() && f.model.is_none()) {
                targets.push((p, pi as u16));
            }
        }
        let wanted = targets.iter().fold(0u64, |m, (d, _)| m | (1 << d));
        let root = std::env::args().nth(3).unwrap_or_default();
        let full = std::path::Path::new(&root).join(self.f.path);
        let ctx = crate::derived::Ctx {
            parsed: self.f.parsed,
            path: &full,
            table: self.f.table,
            source: self.f.source,
            model: self.model,
            incomplete: self.f.incomplete,
        };
        let mut sink = DSink {
            targets: targets.iter().map(|(d, r)| (*d, prog.facts[*r as usize].kinds.len())).collect(),
            out: targets.iter().map(|(_, r)| vec![Vec::new(); prog.facts[*r as usize].kinds.len()]).collect(),
            sym: self.sym,
            view: self.f.view,
            at: None,
        };
        let mut st = std::mem::take(&mut self.cache.dstate);
        st.view_file = None;
        let mut buf = std::mem::take(&mut self.scratch);
        crate::derived::produce(&ctx, wanted, &mut sink, &mut buf, &mut st);
        self.scratch = buf;
        self.cache.dstate = st;
        let out = sink.out;
        let facts = self.cache.facts.as_mut().unwrap();
        for ((_, r), cols) in targets.iter().zip(out) {
            self.c.fact_rows += cols.first().map_or(0, Vec::len) as u64;
            facts.cols[*r as usize] = cols;
            facts.loaded[*r as usize] = true;
        }
        self.c.derived_files += 1;
        self.c.derived_ns += t.elapsed().as_nanos() as u64;
    }

    /// navwide: `comment` and `indent-region` rows from the run's view, as derived.rs's rows_view writes them
    fn view_rows(&mut self, name: &str, width: usize) -> Vec<Vec<u64>> {
        let v = self.f.view;
        let mut cols = vec![Vec::new(); width];
        let mut push = |cols: &mut Vec<Vec<u64>>, row: &[u64]| {
            for (c, x) in row.iter().enumerate().take(width) {
                cols[c].push(*x);
            }
        };
        if name == "comment" {
            let k = self.sym.get_base("comment");
            for id in v.n_nodes..v.len() {
                if Some(v.kind[id]) == k {
                    let text = &self.f.source[v.start[id] as usize + 1..v.stop[id] as usize];
                    let t = u64::from(self.sym.text(text));
                    push(&mut cols, &[id as u64, t, 0, span(v.start[id], v.stop[id]), 0]);
                }
            }
        } else {
            for r in &v.regions {
                let [rid, parent, ind, s, e] = *r;
                push(&mut cols, &[u64::from(rid), u64::from(parent), u64::from(ind), u64::from(s), u64::from(e), 0, span(s, e)]);
            }
        }
        cols
    }

    /// Ins::NearDerived (navwide)
    fn near_derived(&mut self, start: R, out: R, xin: R, edges: &'p [Prog], domain: &'p [Prog]) -> Option<u64> {
        let mut seen: std::collections::HashSet<u64, std::hash::BuildHasherDefault<crate::sema::Fx>> = Default::default();
        let mut level = vec![self.regs[start as usize]];
        let saved = std::mem::take(&mut self.collect);
        let mut answer = None;
        'up: while !level.is_empty() {
            let mut next = Vec::new();
            for &n in &level {
                self.regs[xin as usize] = n;
                self.collect.clear();
                for q in edges {
                    self.c.subs += 1;
                    self.run_inner(q, 0);
                }
                for &y in &self.collect {
                    if seen.insert(y) {
                        next.push(y);
                    }
                }
            }
            for &y in &next {
                self.regs[out as usize] = y;
                self.c.visits += 1;
                if domain.iter().any(|q| {
                    self.c.subs += 1;
                    self.run_inner(q, 0)
                }) {
                    answer = Some(y);
                    break 'up;
                }
            }
            level = next;
        }
        self.collect = saved;
        answer
    }

    /// Ins::Capture (navwide)
    fn capture(&mut self, p: &'p [Ins], pc: usize, text: &Opd, pattern: &str, outs: &[R]) -> bool {
        let v = self.opd(text);
        let Some(t) = self.text_of(text, v).map(str::to_owned) else { return false };
        let mut found: Vec<Vec<std::ops::Range<usize>>> = Vec::new();
        capture_matches(pattern, &t, &t, &mut Vec::new(), &mut found);
        for spans in found {
            for (i, r) in spans.into_iter().enumerate() {
                let id = self.sym.minted(t[r].to_owned());
                self.regs[outs[i] as usize] = id as u64;
            }
            self.c.visits += 1;
            if self.run_inner(p, pc + 1) {
                return true;
            }
        }
        false
    }

    /// a made table by ordered match (navwide): per equal-key group, each left row in ascending order consumes the earliest
    /// unconsumed right row with a strictly greater order
    fn ordered_match(&mut self, rel: u16, om: &'p OrderedMatch) {
        let slot = self.outs.len() - 1;
        let nr = self.prog.facts[rel as usize].nregs.min(self.regs.len());
        let saved_regs: Vec<u64> = self.regs[..nr].to_vec();
        let run = |ex: &mut Self, progs: &'p [Prog]| -> Vec<u64> {
            let saved = (ex.cur_rule, std::mem::take(&mut ex.outs[slot]));
            ex.cur_rule = slot;
            for q in progs {
                ex.run_inner(q, 0);
            }
            let rows = std::mem::replace(&mut ex.outs[slot], saved.1);
            ex.cur_rule = saved.0;
            rows
        };
        let lrows = run(self, &om.left);
        let rrows = run(self, &om.right);
        self.regs[..nr].copy_from_slice(&saved_regs);
        let distinct = |rows: &[u64], w: usize| -> Vec<Vec<u64>> {
            let mut seen: std::collections::HashSet<&[u64], std::hash::BuildHasherDefault<crate::sema::Fx>> = Default::default();
            rows.chunks(w.max(1)).filter(|r| seen.insert(r)).map(<[u64]>::to_vec).collect()
        };
        let ls = distinct(&lrows, om.lw);
        let rs = distinct(&rrows, om.rw);
        let key = |o: u64| -> i128 { if om.number { i128::from(o as i64) } else { i128::from(o) } };
        if let Some(d) = &om.dfs {
            let cols = ordered_dfs(d, &ls, &rs, key);
            self.c.made_tables += 1;
            self.c.made_rows += cols.first().map_or(0, Vec::len) as u64;
            let facts = self.cache.facts.as_mut().unwrap();
            facts.cols[rel as usize] = cols;
            facts.loaded[rel as usize] = true;
            return;
        }
        let mut groups: FxMap<Vec<u64>, (Vec<usize>, Vec<usize>)> = FxMap::default();
        let mut order: Vec<Vec<u64>> = Vec::new();
        for (i, r) in ls.iter().enumerate() {
            let k: Vec<u64> = om.left_on.iter().map(|&c| r[c]).collect();
            let e = groups.entry(k.clone()).or_insert_with(|| {
                order.push(k);
                (Vec::new(), Vec::new())
            });
            e.0.push(i);
        }
        for (i, r) in rs.iter().enumerate() {
            let k: Vec<u64> = om.right_on.iter().map(|&c| r[c]).collect();
            if let Some(e) = groups.get_mut(&k) {
                e.1.push(i);
            }
        }
        let mut cols: Vec<Vec<u64>> = vec![Vec::new(); om.output.len()];
        for k in order {
            let (mut li, mut ri) = groups.remove(&k).unwrap();
            li.sort_by_key(|&i| key(ls[i][om.lo]));
            ri.sort_by_key(|&i| key(rs[i][om.ro]));
            let mut j = 0usize;
            for &l in &li {
                let lo = key(ls[l][om.lo]);
                while j < ri.len() && key(rs[ri[j]][om.ro]) <= lo {
                    j += 1;
                }
                if j >= ri.len() {
                    break;
                }
                let r = ri[j];
                j += 1;
                for (c, &(right, col)) in om.output.iter().enumerate() {
                    cols[c].push(if right { rs[r][col] } else { ls[l][col] });
                }
            }
        }
        self.c.made_tables += 1;
        self.c.made_rows += cols.first().map_or(0, Vec::len) as u64;
        let facts = self.cache.facts.as_mut().unwrap();
        facts.cols[rel as usize] = cols;
        facts.loaded[rel as usize] = true;
    }
}

/// RyoQL `ordered_dfs` (navwide): per distinct (root keys, root), depth-first, out-edges in ascending order, the root seen first;
/// every edge visit is a row (root keys.., root, source, destination, edge, ordinal from 0); unseen destinations are entered at once
fn ordered_dfs(d: &OrderedDfs, roots: &[Vec<u64>], edges: &[Vec<u64>], key: impl Fn(u64) -> i128) -> Vec<Vec<u64>> {
    let mut adj: FxMap<(Vec<u64>, u64), Vec<usize>> = FxMap::default();
    for (i, e) in edges.iter().enumerate() {
        let k: Vec<u64> = d.edge_keys.iter().map(|&c| e[c]).collect();
        adj.entry((k, e[d.source])).or_default().push(i);
    }
    for list in adj.values_mut() {
        list.sort_by_key(|&i| key(edges[i][d.order]));
    }
    let mut cols: Vec<Vec<u64>> = vec![Vec::new(); d.root_keys.len() + 5];
    let mut done: std::collections::HashSet<Vec<u64>, std::hash::BuildHasherDefault<crate::sema::Fx>> = Default::default();
    for r in roots {
        let rk: Vec<u64> = d.root_keys.iter().map(|&c| r[c]).collect();
        let root = r[d.root];
        let mut id = rk.clone();
        id.push(root);
        if !done.insert(id) {
            continue;
        }
        let mut seen: std::collections::HashSet<u64, std::hash::BuildHasherDefault<crate::sema::Fx>> = Default::default();
        seen.insert(root);
        let mut ordinal = 0u64;
        let mut stack: Vec<(u64, usize)> = vec![(root, 0)];
        while let Some(top) = stack.last_mut() {
            let (node, at) = *top;
            let list = adj.get(&(rk.clone(), node));
            let Some(&e) = list.and_then(|l| l.get(at)) else {
                stack.pop();
                continue;
            };
            top.1 += 1;
            let row = &edges[e];
            for (c, v) in rk.iter().enumerate() {
                cols[c].push(*v);
            }
            let n = rk.len();
            cols[n].push(root);
            cols[n + 1].push(row[d.source]);
            cols[n + 2].push(row[d.destination]);
            cols[n + 3].push(row[d.edge]);
            cols[n + 4].push(ordinal);
            ordinal += 1;
            let dst = row[d.destination];
            if seen.insert(dst) {
                stack.push((dst, 0));
            }
        }
    }
    cols
}

/// RyoQL `capture` (navwide copy of the walk interpreter's): every way the text matches the slot pattern
fn capture_matches(pattern: &str, whole: &str, tail: &str, slots: &mut Vec<std::ops::Range<usize>>, out: &mut Vec<Vec<std::ops::Range<usize>>>) {
    let Some(open) = pattern.find('{') else {
        if pattern == tail {
            out.push(slots.clone());
        }
        return;
    };
    let (literal, after_open) = pattern.split_at(open);
    let crosses = after_open.starts_with("{*");
    let Some(from) = tail.strip_prefix(literal) else { return };
    let rest = after_open.split_once('}').map(|(_, a)| a).unwrap_or_default();
    let start = whole.len() - from.len();
    if !rest.contains('{') {
        let Some(end) = from.len().checked_sub(rest.len()) else { return };
        if end >= 1 {
            if let (Some(slot), Some(after)) = (from.get(..end), from.get(end..)) {
                if after == rest && (crosses || !slot.contains('/')) {
                    slots.push(start..start + end);
                    out.push(slots.clone());
                    slots.pop();
                }
            }
        }
        return;
    }
    for end in 1..=from.len() {
        let (Some(slot), Some(after)) = (from.get(..end), from.get(end..)) else { continue };
        if !crosses && slot.contains('/') {
            continue;
        }
        slots.push(start..start + end);
        capture_matches(rest, whole, after, slots, out);
        slots.pop();
    }
}

#[derive(Clone, Copy)]
enum Numeric {
    Int(i128),
    Float(f64),
}

/// canonical signed decimal integers, marked decimal floats, `inf`, `-inf`, `NaN` (no radix, underscore or whitespace)
fn read_numeric(t: &str) -> Option<Numeric> {
    match t {
        "inf" => return Some(Numeric::Float(f64::INFINITY)),
        "-inf" => return Some(Numeric::Float(f64::NEG_INFINITY)),
        "NaN" => return Some(Numeric::Float(f64::NAN)),
        _ => {}
    }
    let digits = t.strip_prefix('-').unwrap_or(t);
    if t.contains(['.', 'e', 'E']) {
        let b = digits.as_bytes();
        if b.is_empty() || !b[0].is_ascii_digit() || (b.len() > 1 && b[0] == b'0' && b[1].is_ascii_digit()) {
            return None;
        }
        return t.parse::<f64>().ok().map(Numeric::Float);
    }
    if digits.is_empty() || !digits.bytes().all(|c| c.is_ascii_digit()) || (digits.len() > 1 && digits.starts_with('0')) {
        return None;
    }
    t.parse::<i128>().ok().map(Numeric::Int)
}

fn numeric_cmp(a: Numeric, b: Numeric) -> Option<std::cmp::Ordering> {
    match (a, b) {
        (Numeric::Int(x), Numeric::Int(y)) => Some(x.cmp(&y)),
        (Numeric::Int(x), Numeric::Float(y)) => (x as f64).partial_cmp(&y),
        (Numeric::Float(x), Numeric::Int(y)) => x.partial_cmp(&(y as f64)),
        (Numeric::Float(x), Numeric::Float(y)) => x.partial_cmp(&y),
    }
}

/// natural text order (ryoql.md): ASCII digit runs by length then digits unless zero-led (then lexically); Unicode
/// whitespace separates runs without weight; other scalars compare case-sensitively
fn natural_compare(left: &str, right: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    let (mut left, mut right) = (left, right);
    loop {
        left = left.trim_start_matches(char::is_whitespace);
        right = right.trim_start_matches(char::is_whitespace);
        let (Some(a), Some(b)) = (left.chars().next(), right.chars().next()) else {
            return left.is_empty().cmp(&right.is_empty()).reverse();
        };
        if a.is_ascii_digit() && b.is_ascii_digit() {
            let a_len = left.bytes().take_while(u8::is_ascii_digit).count();
            let b_len = right.bytes().take_while(u8::is_ascii_digit).count();
            let (a_run, a_rest) = left.split_at(a_len);
            let (b_run, b_rest) = right.split_at(b_len);
            let order = if a == '0' || b == '0' { a_run.cmp(b_run) } else { a_len.cmp(&b_len).then_with(|| a_run.cmp(b_run)) };
            if order != Ordering::Equal {
                return order;
            }
            left = a_rest;
            right = b_rest;
        } else {
            let order = a.cmp(&b);
            if order != Ordering::Equal {
                return order;
            }
            left = &left[a.len_utf8()..];
            right = &right[b.len_utf8()..];
        }
    }
}

/// navwide: the host's filesystem relations for the current file (its path, its executable bit; no read failures, since
/// the file was read)
fn host_filesystem(name: &str, width: usize, file: &str) -> Vec<Vec<u64>> {
    use std::os::unix::fs::PermissionsExt;
    let mut cols = vec![Vec::new(); width];
    match name {
        "filesystem.file" => cols[0].push(0),
        "filesystem.executable" => {
            let root = std::env::args().nth(3).unwrap_or_default();
            let path = std::path::Path::new(&root).join(file);
            let exec = std::fs::metadata(&path).map(|m| m.permissions().mode() & 0o111 != 0).unwrap_or(false);
            cols[0].push(0);
            cols[1].push(u64::from(exec));
        }
        _ => {}
    }
    cols
}

/// navwide: derived.rs rows into nav's columns (node ids are the view's: AST nodes share the table's ids; keyed elements by span
/// and kind name; texts as the run's codes)
struct DSink<'s> {
    targets: Vec<(usize, usize)>,
    out: Vec<Vec<Vec<u64>>>,
    sym: &'s mut Symbols,
    view: &'s View,
    at: Option<FxMap<(u32, u32, u32), u32>>,
}

impl crate::derived::Sink for DSink<'_> {
    fn row(&mut self, rel: usize, cells: &[crate::derived::Cell<'_>]) {
        use crate::derived::Cell;
        let Some(slot) = self.targets.iter().position(|(d, _)| *d == rel) else { return };
        let width = self.targets[slot].1;
        for (c, cell) in cells.iter().enumerate().take(width) {
            let v = match *cell {
                Cell::Node(n) => u64::from(n),
                Cell::Key(k, a, b) => {
                    let v = self.view;
                    let at = self.at.get_or_insert_with(|| {
                        let mut m: FxMap<(u32, u32, u32), u32> = FxMap::default();
                        for id in (0..v.len()).rev() {
                            m.insert((v.start[id], v.stop[id], v.kind[id]), id as u32);
                        }
                        m
                    });
                    let kc = self.sym.get_base(k).unwrap_or(NULL);
                    u64::from(at.get(&(a, b, kc)).copied().unwrap_or(u32::MAX - 1))
                }
                Cell::Text(t) => u64::from(self.sym.text(t)),
                Cell::Null => u64::from(NULL),
                Cell::Num(n) => n as u64,
                Cell::Flag(f) => u64::from(f),
                Cell::Span(a, b) => span(a, b),
                Cell::File => 0,
            };
            self.out[slot][c].push(v);
        }
    }
}

/// NAV_PROFILE report: the hottest instruction sites, each with its rule, branch, path and instruction.
pub(crate) fn profile_report(prog: &NavProgram, prof: &FxMap<usize, u64>) -> String {
    let mut sites: FxMap<usize, String> = FxMap::default();
    let owner_of = |slots: &[u32]| -> String {
        let mut ids: Vec<&str> = slots.iter().map(|&s| prog.rules[prog.slot_rule[s as usize] as usize].id.as_str()).collect();
        ids.dedup();
        if ids.len() > 3 { format!("{}+{}more", ids[0], ids.len() - 1) } else { ids.join("+") }
    };
    fn walk(p: &[Ins], path: String, owner: &str, sites: &mut FxMap<usize, String>, owner_of: &dyn Fn(&[u32]) -> String) {
        for (pc, ins) in p.iter().enumerate() {
            let here = format!("{path}.{pc}");
            let text = format!("{ins:?}");
            let text: String = text.chars().take(160).collect();
            sites.insert(ins as *const Ins as usize, format!("{owner} {here} {text}"));
            match ins {
                Ins::Fork { subs } => {
                    for (k, (slots, q)) in subs.iter().enumerate() {
                        let o = owner_of(slots);
                        walk(q, format!("{here}/{k}"), &o, sites, owner_of);
                    }
                }
                _ => {
                    let mut k = 0;
                    subs_ref(ins, &mut |q| {
                        walk(q, format!("{here}/{k}"), owner, sites, owner_of);
                        k += 1;
                    });
                }
            }
        }
    }
    for (g, grp) in prog.groups.iter().enumerate() {
        let o = owner_of(&grp.slots);
        walk(&grp.prog, format!("g{g}"), &o, &mut sites, &owner_of);
    }
    for f in &prog.facts {
        if let Some((progs, ..)) = &f.made {
            for (k, q) in progs.iter().enumerate() {
                walk(q, format!("p{k}"), &f.name, &mut sites, &owner_of);
            }
        }
        if let Some(om) = &f.made_om {
            for (k, q) in om.left.iter().chain(om.right.iter()).enumerate() {
                walk(q, format!("o{k}"), &f.name, &mut sites, &owner_of);
            }
        }
    }
    for (k, q) in prog.shared.iter().enumerate() {
        walk(q, format!("s{k}"), "shared", &mut sites, &owner_of);
    }
    for r in &prog.rules {
        for (k, (_, ps, _)) in r.aux.iter().enumerate() {
            for q in ps {
                walk(q, format!("a{k}"), &r.id, &mut sites, &owner_of);
            }
        }
    }
    let mut v: Vec<(usize, u64)> = prof.iter().map(|(&k, &n)| (k, n)).collect();
    v.sort_by(|a, b| b.1.cmp(&a.1));
    v.iter().take(1_000_000).map(|(k, n)| format!("{n:>12} {}", sites.get(k).map_or("?", String::as_str))).collect::<Vec<_>>().join("\n")
}
