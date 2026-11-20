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
    Yield,
    SpanBetween { a: Opd, b: Opd, out: R },
    SpanBounds { s: Opd, start: R, end: R },
    SourceMatches { span: Opd, base: Option<Opd>, regex: usize, group: Option<usize>, outs: Box<[R]>, capture_text: bool, capture_line: bool },
}

pub(crate) type Prog = Box<[Ins]>;

pub(crate) const INS_NAMES: [&str; 31] = [
    "iter_kind", "iter_value", "iter_children", "iter_scan", "iter_fact", "iter_fact_scan", "row", "read", "read_fact", "test", "axis",
    "near", "sub", "emit", "yield", "span_between", "span_bounds", "source_matches", "within", "iter_range", "iter_field", "fork", "read_col", "read_child",
    "test_eq_c", "test_ne_c", "test_eq_r", "test_ne_r", "test_in_c", "hoist", "capture",
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
        }
    }
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
}

pub(crate) struct FactRel {
    /// the model relation that serves it (index into semrows::RELATIONS); None: the provider's reader
    pub model: Option<usize>,
    pub name: String,
    pub relation: Option<ryo_relations::domain::relations::types::Relation>,
    pub kinds: Vec<ryo_relations::domain::relations::types::CellKind>,
    /// a made table (group count): its input's programs, the number of key columns, and whether it counts distinct values
    pub made: Option<(Vec<Prog>, Vec<usize>, Option<usize>, usize)>,
    /// a keyed twin: its programs enumerate the input rows whose output key column `.0` equals register `.1` (set before
    /// the run); rows are filled per key on first demand
    pub keyed: Option<(u8, R)>,
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
    pub facts: Vec<FactRel>,
    pub regexes: Vec<Regex>,
    /// each regex's pattern (the compiled pack stores these)
    pub regex_patterns: Vec<String>,
    pub max_regs: usize,
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
        self.facts.push(FactRel { model, name: name.to_owned(), relation: Some(relation), kinds: kinds(relation).to_vec(), made: None, keyed: None, nregs: 0 });
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
                    f => panic!("fn {f}"),
                }
            }
            o => panic!("ins {o}"),
        }
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
            Ins::Sub { negate, subs } => Ins::Sub { negate, subs: subs.into_vec().into_iter().map(specialize).collect::<Vec<_>>().into() },
            Ins::Near { start, out, subs } => {
                Ins::Near { start, out, subs: subs.into_vec().into_iter().map(specialize).collect::<Vec<_>>().into() }
            }
            Ins::Fork { subs } => Ins::Fork { subs: subs.into_vec().into_iter().map(|(sl, q)| (sl, specialize(q))).collect::<Vec<_>>().into() },
            other => other,
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
        Ins::Capture { regs } => reads.extend(regs.iter().copied()),
    }
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
    )
}

static HOIST_ID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// Loop-invariant inner enumerations (NAV_HOIST=0 turns it off): an iteration whose inputs were all bound before an
/// enclosing iteration of the same program, together with the steps after it that read nothing else bound outside it, runs
/// once per file per value of those inputs; its result tuples are replayed into the rest.
pub(crate) fn hoist(p: Prog) -> Prog {
    if std::env::var("NAV_HOIST").is_ok_and(|v| v == "0") {
        return p;
    }
    let mut v: Vec<Ins> = p.into_vec();
    // recurse first
    for i in v.iter_mut() {
        match i {
            Ins::Sub { subs, .. } | Ins::Near { subs, .. } => {
                let taken = std::mem::take(subs);
                *subs = taken.into_vec().into_iter().map(hoist).collect::<Vec<_>>().into();
            }
            Ins::Fork { subs } => {
                let taken = std::mem::take(subs);
                *subs = taken.into_vec().into_iter().map(|(sl, q)| (sl, hoist(q))).collect::<Vec<_>>().into();
            }
            _ => {}
        }
    }
    let mut def_at: FxMap<R, usize> = FxMap::default();
    let mut s = 0;
    while s < v.len() {
        let mut reads = Vec::new();
        let mut defs = Vec::new();
        reads_defs(&v[s], &mut reads, &mut defs);
        let candidate = is_iter(&v[s]) && !reads.is_empty() && reads.iter().all(|r| def_at.contains_key(r));
        if candidate {
            let d = reads.iter().map(|r| def_at[r]).max().unwrap();
            let enclosing = (d + 1..s).any(|j| is_iter(&v[j]));
            if enclosing {
                // grow the segment while each step reads only the key or what the segment defined
                let key: Vec<R> = {
                    let mut k = reads.clone();
                    k.sort_unstable();
                    k.dedup();
                    k
                };
                let mut local: Vec<R> = defs.clone();
                let mut t = s + 1;
                while t < v.len() {
                    if matches!(v[t], Ins::Emit { .. } | Ins::Yield | Ins::Fork { .. } | Ins::Hoist { .. }) {
                        break;
                    }
                    let (mut r2, mut d2) = (Vec::new(), Vec::new());
                    reads_defs(&v[t], &mut r2, &mut d2);
                    if !r2.iter().all(|r| key.contains(r) || local.contains(r)) {
                        break;
                    }
                    local.extend(d2);
                    t += 1;
                }
                if t - s >= 3 && t < v.len() {
                    // captured: segment definitions read later
                    let mut later = Vec::new();
                    for j in &v[t..] {
                        let mut d3 = Vec::new();
                        reads_defs(j, &mut later, &mut d3);
                    }
                    let mut caps: Vec<R> = local.iter().copied().filter(|r| later.contains(r)).collect();
                    caps.sort_unstable();
                    caps.dedup();
                    let mut seg: Vec<Ins> = v.drain(s..t).collect();
                    seg.push(Ins::Capture { regs: caps.clone().into() });
                    let id = HOIST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    for r in &caps {
                        def_at.insert(*r, s);
                    }
                    v.insert(s, Ins::Hoist { id, key: key.into(), seg: seg.into(), caps: caps.into() });
                    s += 1;
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
        for g in &mut self.groups {
            g.prog = hoist(specialize(std::mem::take(&mut g.prog)));
        }
        for f in &mut self.facts {
            if let Some((progs, ..)) = &mut f.made {
                for q in progs.iter_mut() {
                    *q = hoist(specialize(std::mem::take(q)));
                }
            }
        }
    }
}

/// Load-time fusion (NAV_FUSE=0 turns it off): a value bucket followed by the read of the row's start and range tests on it
/// becomes one range walk; children followed by the field read and an equality on it become a per-field walk.
pub(crate) fn fuse(p: Vec<Ins>) -> Vec<Ins> {
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
                // the reads of the row that follow (any order), one of them its start, then range tests on the start
                let mut kept: Vec<Ins> = Vec::new();
                let mut st: Option<R> = None;
                while let Some(Ins::Read { row, .. }) = it.peek() {
                    if *row != o {
                        break;
                    }
                    let r = it.next().unwrap();
                    if let Ins::Read { rel: TRel::Sl, col: 5, out, .. } = &r {
                        st = Some(*out);
                    }
                    kept.push(r);
                }
                let Some(st) = st else {
                    out.push(Ins::IterValue { rel, col, val, out: o });
                    out.extend(kept);
                    continue;
                };
                let (mut lo, mut hi) = (None, None);
                while let Some(Ins::Test { test, l: Opd::Reg(a), r: Opd::Reg(b), text: false, .. }) = it.peek() {
                    let (test, a, b) = (*test, *a, *b);
                    let bound = match (a == st, b == st, test) {
                        (true, false, T::Ge) => (true, (b, true)),
                        (true, false, T::Gt) => (true, (b, false)),
                        (true, false, T::Lt) => (false, (b, false)),
                        (true, false, T::Le) => (false, (b, true)),
                        (false, true, T::Le) => (true, (a, true)),
                        (false, true, T::Lt) => (true, (a, false)),
                        (false, true, T::Gt) => (false, (a, false)),
                        (false, true, T::Ge) => (false, (a, true)),
                        _ => break,
                    };
                    let slot = if bound.0 { &mut lo } else { &mut hi };
                    if slot.is_some() {
                        break;
                    }
                    *slot = Some(bound.1);
                    it.next();
                }
                // the start read is answered by the range walk itself
                kept.retain(|r| !matches!(r, Ins::Read { rel: TRel::Sl, col: 5, .. }));
                out.push(Ins::IterRange { rel, col, val, out: o, start: st, lo, hi });
                out.extend(kept);
            }
            Ins::IterChildren { parent, out: o } => {
                let fused = match it.peek() {
                    Some(Ins::Read { rel: TRel::Sc, col: 3, row, out: f }) if *row == o => Some(*f),
                    _ => None,
                };
                let Some(f) = fused else {
                    out.push(Ins::IterChildren { parent, out: o });
                    continue;
                };
                let read = it.next().unwrap();
                match it.peek() {
                    Some(Ins::Test { test: T::Eq, l: Opd::Reg(a), r: Opd::Sym(x), .. }) if *a == f => {
                        let x = *x;
                        it.next();
                        out.push(Ins::IterField { parent, field: x, out: o, field_out: f });
                    }
                    _ => {
                        out.push(Ins::IterChildren { parent, out: o });
                        out.push(read);
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
                let name = m["name"].as_str().unwrap().to_owned();
                let keyed = m.get("param").map(|p| (m["keycol"].as_u64().unwrap() as u8, loader.reg(p)));
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
            let nregs = loader.regs.len();
            max_regs = max_regs.max(nregs);
            out.push(NavRule {
                id: r["id"].as_str().unwrap().to_owned(),
                programs,
                xmerged: nav["xmerged"].as_u64().unwrap() as u32,
                kinds: nav["columns"].as_array().unwrap().iter().map(|c| cell_kind(c.as_str().unwrap())).collect(),
                nregs,
            });
        }
        let mut me = Self { groups: Vec::new(), slot_of: Vec::new(), slot_rule: Vec::new(), nslots: 0, rules: out, facts, regexes, regex_patterns: patterns, max_regs };
        me.build_groups();
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
        .map(|i| match i {
            Ins::Sub { subs, .. } | Ins::Near { subs, .. } => 1 + subs.iter().map(|q| count_ins(q)).sum::<usize>(),
            Ins::Fork { subs } => 1 + subs.iter().map(|(_, q)| count_ins(q)).sum::<usize>(),
            _ => 1,
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
        | Ins::Capture { .. } => unreachable!("specialisation runs after the tries are built"),
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
    /// per made table (fact index): (builds, exclusive ns, input rows, steps)
    pub made_by: FxMap<u16, (u64, u64, u64, u64)>,
    pub kind_index_ns: u64,
    pub model_lex_ns: u64,
    pub model_flow_ns: u64,
    pub model_rows_ns: u64,
    pub model_files: u64,
    pub flow_files: u64,
    pub provider_files: u64,
    pub visits: u64,
    pub rule_steps: Vec<u64>,
    pub rule_visits: Vec<u64>,
    pub rule_starts: Vec<u64>,
    /// made-table rows and the steps spent building made tables, per rule that demanded them
    pub rule_made_rows: Vec<u64>,
    pub rule_made_steps: Vec<u64>,
    /// steps per instruction kind (index: Ins::code)
    pub op_steps: [u64; 32],
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
    /// hoisted inner enumerations per (site, key values): captured tuples, flattened
    pub hoisted: FxMap<(u32, Vec<u64>), Rc<Vec<u64>>>,
    pub facts: Option<Facts>,
    pub sub_memo: FxMap<usize, bool>,
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
}

pub(crate) struct Exec<'a, 'p> {
    pub prog: &'p NavProgram,
    pub f: &'a File<'a>,
    pub sym: &'a mut Symbols,
    pub cache: &'a mut FileCache,
    pub provider: &'a mut Provider,
    pub model: &'a mut crate::model::Model,
    pub scratch: String,
    pub c: &'a mut Counters,
    pub regs: Vec<u64>,
    pub outs: Vec<Vec<u64>>,
    pub cur_rule: usize,
    /// the running hoist's capture buffer
    pub cap_buf: Vec<u64>,
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
    fn const_list(&mut self, rel: TRel, col: u8, val: &Val) -> Rc<Vec<u32>> {
        let key = val as *const Val as usize;
        if let Some(l) = self.cache.const_idx.get(&key) {
            return l.clone();
        }
        let t0 = Instant::now();
        let n = if rel == TRel::Id { self.f.view.n_nodes } else { self.f.view.len() };
        let wanted: Vec<u64> = match val {
            Val::Syms(s) => s.iter().map(|&x| u64::from(x)).collect(),
            Val::Nums(v) => v.iter().map(|&x| u64::from(x as u32)).collect(),
            Val::Reg(_) => unreachable!(),
        };
        let ckey = (rel as u8, col, wanted.clone());
        if let Some(l) = self.cache.const_by.get(&ckey) {
            let l = l.clone();
            self.cache.const_idx.insert(key, l.clone());
            return l;
        }
        let v = self.f.view;
        let csr = self.col_csr(rel, col);
        let mut out: Vec<u32> = Vec::new();
        for &w in &wanted {
            if let Some(&(a, b)) = csr.at.get(&(w as u32)) {
                out.extend_from_slice(&csr.ids[a as usize..b as usize]);
            }
        }
        let _ = n;
        if wanted.len() > 1 {
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
        // buckets in start order (ids are pre-order nodes, then tokens)
        for &(s0, e0) in at.values() {
            let sl = &mut ids[s0 as usize..e0 as usize];
            if sl.windows(2).any(|w| v.start[w[0] as usize] > v.start[w[1] as usize]) {
                sl.sort_by_key(|&id| (v.start[id as usize], id));
            }
        }
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
            None => {
                let loaded = self.load_facts();
                let facts = self.cache.facts.as_mut().unwrap();
                for (i, cols) in loaded.into_iter().enumerate() {
                    if prog.facts[i].model.is_none() && prog.facts[i].made.is_none() {
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
        let p = &mut *self.provider;
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
        self.c.rule_steps[r] += self.c.steps - steps0;
        self.c.rule_visits[r] += self.c.visits - visits0;
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
                    let list = self.const_list(*rel, *col, val);
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
                    let keys: Vec<u32> = match val {
                        Val::Reg(r) => vec![self.regs[*r as usize] as u32],
                        Val::Syms(s) => s.to_vec(),
                        Val::Nums(n) => n.iter().map(|&x| x as u32).collect(),
                    };
                    for k in keys {
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
                    let keys = keys.to_vec();
                    return self.iter_fact_list(p, pc, &idx, &keys, *out);
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
                        _ => (None, Some(self.const_list(*rel, *col, val))),
                    };
                    let keys: Vec<u32> = match val {
                        Val::Reg(r) => vec![self.regs[*r as usize] as u32],
                        _ => vec![0],
                    };
                    for k in keys {
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
                    let kv: Vec<u64> = key.iter().map(|&r| self.regs[r as usize]).collect();
                    let hk = (*id, kv);
                    let tuples = match self.cache.hoisted.get(&hk) {
                        Some(t) => t.clone(),
                        None => {
                            let saved = std::mem::take(&mut self.cap_buf);
                            self.run_inner(seg, 0);
                            let t = Rc::new(std::mem::replace(&mut self.cap_buf, saved));
                            self.c.hoist_misses += 1;
                            self.cache.hoisted.insert(hk, t.clone());
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
                Ins::Fork { subs } => {
                    for (slots, q) in subs.iter() {
                        if !slots.iter().any(|&s| self.cache.enabled[s as usize]) {
                            continue;
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
                Ins::Sub { subs, .. } | Ins::Near { subs, .. } => {
                    for (k, q) in subs.iter().enumerate() {
                        walk(q, format!("{here}/{k}"), owner, sites, owner_of);
                    }
                }
                Ins::Fork { subs } => {
                    for (k, (slots, q)) in subs.iter().enumerate() {
                        let o = owner_of(slots);
                        walk(q, format!("{here}/{k}"), &o, sites, owner_of);
                    }
                }
                _ => {}
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
    }
    let mut v: Vec<(usize, u64)> = prof.iter().map(|(&k, &n)| (k, n)).collect();
    v.sort_by(|a, b| b.1.cmp(&a.1));
    v.iter().take(1_000_000).map(|(k, n)| format!("{n:>12} {}", sites.get(k).map_or("?", String::as_str))).collect::<Vec<_>>().join("\n")
}
