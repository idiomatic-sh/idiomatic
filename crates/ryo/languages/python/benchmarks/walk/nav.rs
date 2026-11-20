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
    /// cut: the rows of a fact relation (a made table included) whose `cols` cells equal the values: a composite index
    /// over those columns, built once per file per column set
    IterFactMulti { rel: u16, cols: Box<[u8]>, vals: Box<[Opd]>, out: R },
    Row { rel: TRel, key: R },
    Read { rel: TRel, col: u8, row: R, out: R },
    ReadFact { rel: u16, col: u8, row: R, out: R },
    Test { test: T, l: Opd, r: Opd, text: bool, num: bool, regex: Box<[usize]> },
    Axis { dir: Dir, start: R, out: R },
    Near { start: R, out: R, subs: Box<[Prog]> },
    Sub { negate: bool, subs: Box<[Prog]> },
    Emit { cols: Box<[Opd]> },
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
    Capture { text: Opd, pattern: Box<str>, outs: Box<[R]> },
    /// RyoQL `replace`: the text with every regex match replaced by the literal
    Replace { text: Opd, regex: usize, replacement: Box<str>, out: R },
    /// nearest ancestor over derived edges: breadth-first up the edge sub-programs (each collects the reached node with
    /// `xin` bound) until a level holds a node the domain sub-programs admit; the first such node goes on
    NearDerived { start: R, out: R, xin: R, edges: Box<[Prog]>, domain: Box<[Prog]> },
    /// the next (or previous) sibling in the tree, when there is one
    Sibling { next: bool, start: R, out: R },
    /// an absence/existence test reading no candidate register: answered once per file
    SubMemo { negate: bool, subs: Box<[Prog]> },
    /// lower: a call of a definition's sub-program that binds rows (as `Union`), its distinct rows memoised per file by the
    /// values of the `keys` registers (the formals the call wrote): the same call with the same args runs once per file
    /// cut: `args`, when given, are written into the `keys` registers by the instruction itself (the call passes its
    /// arguments in one step instead of one `let` each)
    UnionKeyed { outs: Box<[R]>, subs: Box<[Prog]>, keys: Box<[R]>, args: Box<[Opd]> },
    /// lower: an absence/existence test of a definition, its answer memoised per file by the values of the `keys` registers
    /// (the formals the call wrote): nested correlated tests asked again with the same args answer from the memo
    SubKeyed { negate: bool, subs: Box<[Prog]>, keys: Box<[R]>, args: Box<[Opd]> },
    /// cut: a call of a definition entered from the definition's side: its uncorrelated sub-program's rows (`width` cells
    /// each) are collected once per file and indexed by the `kcols` cells; the call looks up the values of `keys` and
    /// continues with each distinct tuple of the `ocols` cells in `outs` (a hash join whose build side is the definition)
    Probe { outs: Box<[R]>, subs: Box<[Prog]>, kcols: Box<[u8]>, keys: Box<[Opd]>, ocols: Box<[u8]>, width: u8 },
    /// cut: an absence/existence test of a definition answered from the same per-file index of its uncorrelated rows
    ProbeTest { negate: bool, subs: Box<[Prog]>, kcols: Box<[u8]>, keys: Box<[Opd]>, width: u8 },
}

/// cut: a definition's uncorrelated rows indexed by some of their cells: key cells -> the distinct tuples of the wanted cells
pub(crate) struct ProbeIndex {
    pub map: FxMap<Box<[u64]>, Vec<u64>>,
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
}

pub(crate) struct NavProgram {
    pub rules: Vec<NavRule>,
    /// shared sub-programs of every rule (navwide)
    pub shared: Vec<Prog>,
    pub facts: Vec<FactRel>,
    pub regexes: Vec<Regex>,
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
    regex_of: FxMap<(String, bool), usize>,
    /// the rule's first entry in the shared sub-program table (navwide)
    shared_base: usize,
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
        self.facts.push(FactRel { model, name: name.to_owned(), relation: Some(relation), kinds: kinds(relation).to_vec(), made: None, made_om: None });
        (self.facts.len() - 1) as u16
    }

    fn regex(&mut self, text: &str, glob: bool) -> usize {
        if let Some(&i) = self.regex_of.get(&(text.to_owned(), glob)) {
            return i;
        }
        let pattern = if glob { glob_to_regex(text) } else { text.to_owned() };
        self.regexes.push(Regex::new(&pattern).unwrap_or_else(|e| panic!("regex {pattern}: {e}")));
        self.regex_of.insert((text.to_owned(), glob), self.regexes.len() - 1);
        self.regexes.len() - 1
    }

    fn prog(&mut self, p: &Value) -> Prog {
        p.as_array().unwrap().iter().map(|i| self.ins(i)).collect::<Vec<_>>().into()
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
                        "bym" => Ins::IterFactMulti {
                            rel: f,
                            cols: a[3][0].as_array().unwrap().iter().map(|x| x.as_u64().unwrap() as u8).collect::<Vec<_>>().into(),
                            vals: a[3][1].as_array().unwrap().iter().map(|x| self.opd(x)).collect::<Vec<_>>().into(),
                            out,
                        },
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
            "emit" => Ins::Emit { cols: a[1].as_array().unwrap().iter().map(|c| self.opd(c)).collect::<Vec<_>>().into() },
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
                    "capture" => Ins::Capture { text: args[0].clone(), pattern: a[4]["pattern"].as_str().unwrap().into(), outs: outs.into() },
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
            "shared" => Ins::Shared { idx: (self.shared_base + a[1].as_u64().unwrap() as usize) as u32 },
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
            "neg-keyed" | "exists-keyed" => Ins::SubKeyed {
                negate: a[0] == "neg-keyed",
                subs: a[1].as_array().unwrap().iter().map(|p| self.prog(p)).collect::<Vec<_>>().into(),
                keys: a[2].as_array().unwrap().iter().map(|x| self.reg(x)).collect::<Vec<_>>().into(),
                args: a.get(3).and_then(Value::as_array).map(|v| v.iter().map(|x| self.opd(x)).collect::<Vec<_>>()).unwrap_or_default().into(),
            },
            "or-keyed" => Ins::UnionKeyed {
                outs: a[1].as_array().unwrap().iter().map(|x| self.reg(x)).collect::<Vec<_>>().into(),
                subs: a[2].as_array().unwrap().iter().map(|p| self.prog(p)).collect::<Vec<_>>().into(),
                keys: a[3].as_array().unwrap().iter().map(|x| self.reg(x)).collect::<Vec<_>>().into(),
                args: a.get(4).and_then(Value::as_array).map(|v| v.iter().map(|x| self.opd(x)).collect::<Vec<_>>()).unwrap_or_default().into(),
            },
            // cut: ["or-probe", outs, subs, key columns, key values, out columns, row width]
            "or-probe" => Ins::Probe {
                outs: a[1].as_array().unwrap().iter().map(|x| self.reg(x)).collect::<Vec<_>>().into(),
                subs: a[2].as_array().unwrap().iter().map(|p| self.prog(p)).collect::<Vec<_>>().into(),
                kcols: a[3].as_array().unwrap().iter().map(|x| x.as_u64().unwrap() as u8).collect::<Vec<_>>().into(),
                keys: a[4].as_array().unwrap().iter().map(|x| self.opd(x)).collect::<Vec<_>>().into(),
                ocols: a[5].as_array().unwrap().iter().map(|x| x.as_u64().unwrap() as u8).collect::<Vec<_>>().into(),
                width: a[6].as_u64().unwrap() as u8,
            },
            // cut: ["neg-probe" | "exists-probe", subs, key columns, key values, row width]
            "neg-probe" | "exists-probe" => Ins::ProbeTest {
                negate: a[0] == "neg-probe",
                subs: a[1].as_array().unwrap().iter().map(|p| self.prog(p)).collect::<Vec<_>>().into(),
                kcols: a[2].as_array().unwrap().iter().map(|x| x.as_u64().unwrap() as u8).collect::<Vec<_>>().into(),
                keys: a[3].as_array().unwrap().iter().map(|x| self.opd(x)).collect::<Vec<_>>().into(),
                width: a[4].as_u64().unwrap() as u8,
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

impl NavProgram {
    pub(crate) fn load(rules: &[&Value], sym: &mut Symbols) -> Self {
        let mut facts = Vec::new();
        let mut regexes = Vec::new();
        let mut shared: Vec<Prog> = Vec::new();
        let mut out = Vec::new();
        let mut max_regs = 0;
        for r in rules {
            let nav = &r["nav"];
            let kinds = nav["kinds"].as_object().unwrap();
            let mut loader = Loader {
                sym,
                regs: FxMap::default(),
                kinds,
                facts: &mut facts,
                regexes: &mut regexes,
                regex_of: FxMap::default(),
                shared_base: shared.len(),
            };
            for m in nav["made"].as_array().map(Vec::as_slice).unwrap_or(&[]) {
                use ryo_relations::domain::relations::types::CellKind;
                let progs: Vec<Prog> = m["programs"].as_array().unwrap().iter().map(|p| loader.prog(p)).collect();
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
                    .collect();
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
                    loader.facts.push(FactRel { model: None, name: m["name"].as_str().unwrap().to_owned(), relation: None, kinds, made: None, made_om: Some(om) });
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
                    loader.facts.push(FactRel { model: None, name: m["name"].as_str().unwrap().to_owned(), relation: None, kinds, made: None, made_om: Some(om) });
                    continue;
                }
                loader.facts.push(FactRel {
                    made_om: None,
                    model: None,
                    name: m["name"].as_str().unwrap().to_owned(),
                    relation: None,
                    kinds,
                    made: Some((
                        progs,
                        m["keys"].as_array().unwrap().iter().map(|k| k.as_u64().unwrap() as usize).collect(),
                        m["distinct"].as_u64().map(|d| d as usize),
                        m["width"].as_u64().unwrap() as usize,
                    )),
                });
            }
            for q in nav["shared"].as_array().map(Vec::as_slice).unwrap_or(&[]) {
                let p = loader.prog(q);
                shared.push(p);
            }
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
        Self { rules: out, shared, facts, regexes, max_regs, needs_provider }
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
    /// cut (diagnostic, NAV_PROF): per shared entry, calls, inclusive steps and own steps (inclusive minus the shared
    /// entries it ran); `prof_child` is the inclusive steps of the entries run under the current one
    pub prof: bool,
    pub prof_child: u64,
    /// cut: thread-ns grouping made tables (after their programs ran) and building fact, composite and probe indexes;
    /// keyed memo lookups and misses
    pub made_group_ns: u64,
    pub index_ns: u64,
    pub index_builds: u64,
    pub keyed_calls: u64,
    pub keyed_misses: u64,
    pub shared_calls: Vec<u64>,
    pub shared_incl: Vec<u64>,
    pub shared_self: Vec<u64>,
}

/// What a file's facts from the provider are, decoded into the view's ids and the run's text codes.
#[derive(Default)]
pub(crate) struct Facts {
    pub cols: Vec<Vec<Vec<u64>>>,
    pub loaded: Vec<bool>,
    pub provider_done: bool,
    pub idx: FxMap<(u16, u8), Rc<FIdx>>,
    /// cut: composite indexes, by relation and column set: hash of the cells -> rows
    pub midx: FxMap<(u16, Box<[u8]>), Rc<FxMap<u64, Vec<u32>>>>,
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
    pub tree_idx: FxMap<(u8, u8), Rc<FxMap<u32, Vec<u32>>>>,
    pub facts: Option<Facts>,
    pub sub_memo: FxMap<usize, bool>,
    /// navwide: reusable state of the derived relations (derived.rs)
    pub dstate: crate::derived::State,
    pub model_lex: bool,
    pub model_flow: bool,
    /// per-file dispatch: kind code -> (rule, branch, alternative) anchored on it
    pub dispatch: Vec<Vec<(u32, u32, u32)>>,
    pub dispatch_used: Vec<u32>,
    pub other: Vec<(u32, u32, u32)>,
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
    /// cut: per-file uncorrelated rows of definitions (by shared entry, or by sub-program list) and the indexes over them
    /// (by the probing instruction)
    pub def_rows: FxMap<usize, Rc<Vec<u64>>>,
    pub probe_idx: FxMap<usize, Rc<ProbeIndex>>,
    pub kbuf: Vec<u64>,
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

    fn tree_index(&mut self, rel: TRel, col: u8) -> Rc<FxMap<u32, Vec<u32>>> {
        let key = (rel as u8, col);
        if let Some(i) = self.cache.tree_idx.get(&key) {
            return i.clone();
        }
        let mut m: FxMap<u32, Vec<u32>> = FxMap::default();
        let n = if rel == TRel::Id { self.f.view.n_nodes } else { self.f.view.len() };
        for id in 0..n as u32 {
            if let Some(v) = self.tread(rel, col, id) {
                m.entry(v as u32).or_default().push(id);
            }
        }
        self.c.made_tables += 1;
        self.c.made_rows += n as u64;
        let m = Rc::new(m);
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
            midx: FxMap::default(),
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
            let slot = self.outs.len() - 1;
            let saved = (self.cur_rule, std::mem::take(&mut self.outs[slot]));
            self.cur_rule = slot;
            for q in progs {
                self.run_inner(q, 0);
            }
            let rows = std::mem::replace(&mut self.outs[slot], saved.1);
            self.cur_rule = saved.0;
            let tg = Instant::now();
            // cut: one scratch key per row, an owned copy only for a new group or a new distinct value (it was four
            // allocations per row)
            let mut seen: std::collections::HashSet<Vec<u64>, std::hash::BuildHasherDefault<crate::sema::Fx>> = Default::default();
            let mut seen_rows: std::collections::HashSet<&[u64], std::hash::BuildHasherDefault<crate::sema::Fx>> = Default::default();
            let mut groups: FxMap<Vec<u64>, u64> = FxMap::default();
            let mut order: Vec<Vec<u64>> = Vec::new();
            let mut key: Vec<u64> = Vec::with_capacity(keys.len() + 1);
            for row in rows.chunks((*width).max(1)) {
                key.clear();
                key.extend(keys.iter().map(|&k| row[k]));
                match distinct {
                    Some(d) => {
                        key.push(row[*d]);
                        let fresh = !seen.contains(&key[..]);
                        if fresh {
                            seen.insert(key.clone());
                        }
                        key.pop();
                        if !fresh {
                            continue;
                        }
                    }
                    None => {
                        if !seen_rows.insert(row) {
                            continue;
                        }
                    }
                }
                match groups.get_mut(&key[..]) {
                    Some(e) => *e += 1,
                    None => {
                        groups.insert(key.clone(), 1);
                        order.push(key.clone());
                    }
                }
            }
            let facts = self.cache.facts.as_mut().unwrap();
            let cols = &mut facts.cols[rel as usize];
            for key in order {
                let n = groups[&key];
                for (c, v) in key.iter().enumerate() {
                    cols[c].push(*v);
                }
                cols[*nkeys].push(n);
            }
            self.c.made_tables += 1;
            self.c.made_rows += cols[*nkeys].len() as u64;
            facts.loaded[rel as usize] = true;
            self.c.made_group_ns += tg.elapsed().as_nanos() as u64;
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
            None if std::env::var_os("NAV_PROVIDER").is_none()
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
        let ti = Instant::now();
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
        self.c.index_builds += 1;
        self.c.index_ns += ti.elapsed().as_nanos() as u64;
        let m = Rc::new(idx);
        facts.idx.insert((rel, col), m.clone());
        m
    }

    /// cut: the composite index of a fact relation over a column set (hash of the cells -> rows; a hit checks the cells)
    fn fact_multi_index(&mut self, rel: u16, cols: &[u8]) -> Rc<FxMap<u64, Vec<u32>>> {
        use std::hash::Hasher;
        self.ensure(rel);
        let facts = self.cache.facts.as_mut().unwrap();
        if let Some(i) = facts.midx.get(&(rel, cols.into())) {
            return i.clone();
        }
        let ti = Instant::now();
        let f = &facts.cols[rel as usize];
        let n = f.first().map_or(0, Vec::len);
        let mut m: FxMap<u64, Vec<u32>> = FxMap::default();
        for row in 0..n {
            let mut h = crate::sema::Fx::default();
            for &c in cols {
                h.write_u64(f[c as usize][row]);
            }
            m.entry(h.finish()).or_default().push(row as u32);
        }
        self.c.made_tables += 1;
        self.c.made_rows += n as u64;
        self.c.index_builds += 1;
        self.c.index_ns += ti.elapsed().as_nanos() as u64;
        let m = Rc::new(m);
        facts.midx.insert((rel, cols.into()), m.clone());
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
        self.c.rule_steps[r] += self.c.steps - steps0;
        self.c.rule_visits[r] += self.c.visits - visits0;
        let _ = &mut pc;
        flow
    }

    fn run_inner(&mut self, p: &'p [Ins], pc: usize) -> bool {
        let mut pc = pc;
        loop {
            self.c.steps += 1;
            match &p[pc] {
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
                Ins::IterValue { rel, col, val, out } => {
                    let idx = self.tree_index(*rel, *col);
                    let keys: Vec<u32> = match val {
                        Val::Reg(r) => vec![self.regs[*r as usize] as u32],
                        Val::Syms(s) => s.to_vec(),
                        Val::Nums(n) => n.iter().map(|&x| x as u32).collect(),
                    };
                    for k in keys {
                        if let Some(list) = idx.get(&k) {
                            for &id in list {
                                self.regs[*out as usize] = id as u64;
                                self.c.visits += 1;
                                if self.run_inner(p, pc + 1) {
                                    return true;
                                }
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
                Ins::IterFactMulti { rel, cols, vals, out } => {
                    use std::hash::Hasher;
                    let idx = self.fact_multi_index(*rel, cols);
                    let mut key = std::mem::take(&mut self.kbuf);
                    key.clear();
                    let mut h = crate::sema::Fx::default();
                    for v in vals.iter() {
                        let x = self.opd(v);
                        key.push(x);
                        h.write_u64(x);
                    }
                    let mut flow = false;
                    if let Some(list) = idx.get(&h.finish()) {
                        for &row in list {
                            let same = {
                                let f = &self.cache.facts.as_ref().unwrap().cols[*rel as usize];
                                cols.iter().zip(key.iter()).all(|(&c, &k)| f[c as usize][row as usize] == k)
                            };
                            if !same {
                                continue;
                            }
                            self.regs[*out as usize] = u64::from(row);
                            self.c.visits += 1;
                            if self.run_inner(p, pc + 1) {
                                flow = true;
                                break;
                            }
                        }
                    }
                    self.kbuf = key;
                    return flow;
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
                Ins::Emit { cols } => {
                    for c in cols.iter() {
                        let v = self.opd(c);
                        self.outs[self.cur_rule].push(v);
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
                Ins::UnionKeyed { outs, subs, keys, args } => {
                    for (k, a) in keys.iter().zip(args.iter()) {
                        self.regs[*k as usize] = self.opd(a);
                    }
                    return self.union_keyed(p, pc, outs, subs, keys);
                }
                Ins::SubKeyed { negate, subs, keys, args } => {
                    for (k, a) in keys.iter().zip(args.iter()) {
                        self.regs[*k as usize] = self.opd(a);
                    }
                    if self.sub_keyed(subs, keys) == *negate {
                        return false;
                    }
                }
                Ins::Probe { outs, subs, kcols, keys, ocols, width } => {
                    let idx = self.probe_index(subs, kcols, ocols, *width as usize, keys.as_ptr() as usize);
                    let mut key = std::mem::take(&mut self.kbuf);
                    key.clear();
                    for k in keys.iter() {
                        key.push(self.opd(k));
                    }
                    let hit = idx.map.get(&key[..]);
                    self.kbuf = key;
                    let Some(rows) = hit else { return false };
                    let w = outs.len().max(1);
                    for row in rows.chunks(w) {
                        for (i, r) in outs.iter().enumerate() {
                            self.regs[*r as usize] = row[i];
                        }
                        self.c.visits += 1;
                        if self.run_inner(p, pc + 1) {
                            return true;
                        }
                    }
                    return false;
                }
                Ins::ProbeTest { negate, subs, kcols, keys, width } => {
                    let idx = self.probe_index(subs, kcols, &[], *width as usize, keys.as_ptr() as usize);
                    let mut key = std::mem::take(&mut self.kbuf);
                    key.clear();
                    for k in keys.iter() {
                        key.push(self.opd(k));
                    }
                    let any = idx.map.contains_key(&key[..]);
                    self.kbuf = key;
                    if any == *negate {
                        return false;
                    }
                }
                Ins::Let { from, out } => self.regs[*out as usize] = self.opd(from),
                Ins::SubMemo { negate, subs } => {
                    // cut: one shared entry is keyed by the entry (every test site of the definition shares its answer)
                    let key = match &subs[..] {
                        [q] => match &q[..] {
                            [Ins::Shared { idx }] => 0x8000_0000_0000_0000usize | ((*idx as usize) << 1),
                            _ => subs.as_ptr() as usize,
                        },
                        _ => subs.as_ptr() as usize,
                    };
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
                Ins::Capture { text, pattern, outs } => return self.capture(p, pc, text, pattern, outs),
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
                    let q: &'p [Ins] = &self.prog.shared[*idx as usize];
                    if !self.c.prof {
                        return self.run_inner(q, 0);
                    }
                    let i = *idx as usize;
                    if self.c.shared_calls.len() <= i {
                        let n = self.prog.shared.len();
                        self.c.shared_calls.resize(n, 0);
                        self.c.shared_incl.resize(n, 0);
                        self.c.shared_self.resize(n, 0);
                    }
                    let outer = std::mem::take(&mut self.c.prof_child);
                    let s0 = self.c.steps;
                    let flow = self.run_inner(q, 0);
                    let incl = self.c.steps - s0;
                    self.c.shared_calls[i] += 1;
                    self.c.shared_incl[i] += incl;
                    self.c.shared_self[i] += incl - self.c.prof_child;
                    self.c.prof_child = outer + incl;
                    return flow;
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

    /// Ins::Union (navwide): every sub-program's collected tuples, distinct, each continuing the program
    fn union(&mut self, p: &'p [Ins], pc: usize, outs: &[R], subs: &'p [Prog], memo: bool) -> bool {
        let w = outs.len();
        // cut: a memoised union of one shared entry is keyed by the entry (every call site of the definition shares its rows)
        let key = match subs {
            [q] if memo => match &q[..] {
                [Ins::Shared { idx }] => 0x8000_0000_0000_0000usize | ((*idx as usize) << 1),
                _ => subs.as_ptr() as usize,
            },
            _ => subs.as_ptr() as usize,
        };
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

    /// cut: what names a memoised sub-program list: its shared entry when it is one (every call site of the same definition
    /// program with the same args then shares the answer), else the list itself
    fn memo_seed(subs: &[Prog]) -> u64 {
        match subs {
            [q] => match &q[..] {
                [Ins::Shared { idx }] => 0x5bd1_e995_0000_0000u64 | u64::from(*idx),
                _ => subs.as_ptr() as u64,
            },
            _ => subs.as_ptr() as u64,
        }
    }

    /// cut: the uncorrelated rows of a definition's sub-programs, collected once per file (shared by every instruction that
    /// names the same shared entry)
    fn definition_rows(&mut self, subs: &'p [Prog]) -> Rc<Vec<u64>> {
        let key = match subs {
            [q] => match &q[..] {
                [Ins::Shared { idx }] => (*idx as usize) << 1 | 1,
                _ => subs.as_ptr() as usize & !1,
            },
            _ => subs.as_ptr() as usize & !1,
        };
        if let Some(r) = self.def_rows.get(&key) {
            return r.clone();
        }
        let saved = std::mem::take(&mut self.collect);
        for q in subs {
            self.c.subs += 1;
            self.run_inner(q, 0);
        }
        let rows = Rc::new(std::mem::replace(&mut self.collect, saved));
        self.def_rows.insert(key, rows.clone());
        rows
    }

    /// cut: the index of a definition's uncorrelated rows for one probing instruction (`at`): key cells -> distinct tuples
    /// of the wanted cells
    fn probe_index(&mut self, subs: &'p [Prog], kcols: &[u8], ocols: &[u8], width: usize, at: usize) -> Rc<ProbeIndex> {
        if let Some(i) = self.probe_idx.get(&at) {
            return i.clone();
        }
        let rows = self.definition_rows(subs);
        let ti = Instant::now();
        let mut map: FxMap<Box<[u64]>, Vec<u64>> = FxMap::default();
        let mut seen: std::collections::HashSet<Vec<u64>, std::hash::BuildHasherDefault<crate::sema::Fx>> = Default::default();
        let mut key: Vec<u64> = Vec::with_capacity(kcols.len());
        for row in rows.chunks(width.max(1)) {
            key.clear();
            key.extend(kcols.iter().map(|&c| row[c as usize]));
            if ocols.is_empty() {
                if !map.contains_key(&key[..]) {
                    map.insert(key.clone().into(), Vec::new());
                }
                continue;
            }
            let mut full = key.clone();
            full.extend(ocols.iter().map(|&c| row[c as usize]));
            if seen.insert(full) {
                let e = match map.get_mut(&key[..]) {
                    Some(e) => e,
                    None => map.entry(key.clone().into()).or_default(),
                };
                e.extend(ocols.iter().map(|&c| row[c as usize]));
            }
        }
        let idx = Rc::new(ProbeIndex { map });
        self.c.index_builds += 1;
        self.c.index_ns += ti.elapsed().as_nanos() as u64;
        self.probe_idx.insert(at, idx.clone());
        idx
    }

    /// Ins::UnionKeyed (lower): as `union`, memoised per file in `union_memo` under an odd key (a hash of the sub-program
    /// list and the key registers' values; plain union keys are pointers, so even); the stored rows start with the key
    /// values, checked on a hit (a hash collision recomputes)
    fn union_keyed(&mut self, p: &'p [Ins], pc: usize, outs: &[R], subs: &'p [Prog], keys: &[R]) -> bool {
        use std::hash::Hasher;
        let w = outs.len().max(1);
        let nk = keys.len();
        let mut h = crate::sema::Fx::default();
        h.write_u64(Self::memo_seed(subs));
        for k in keys {
            h.write_u64(self.regs[*k as usize]);
        }
        let key = (h.finish() as usize) | 1;
        let hit = match self.union_memo.get(&key) {
            Some(r) if r.len() >= nk && keys.iter().enumerate().all(|(i, k)| r[i] == self.regs[*k as usize]) => Some(r.clone()),
            _ => None,
        };
        self.c.keyed_calls += 1;
        let rows = match hit {
            Some(r) => r,
            None => {
                self.c.keyed_misses += 1;
                let saved = std::mem::take(&mut self.collect);
                for q in subs {
                    self.c.subs += 1;
                    self.run_inner(q, 0);
                }
                let rows = std::mem::replace(&mut self.collect, saved);
                let mut distinct: Vec<u64> = keys.iter().map(|k| self.regs[*k as usize]).collect();
                if rows.len() <= w {
                    // cut: none or one row is distinct as it stands (no set)
                    distinct.extend_from_slice(&rows);
                } else {
                    let mut seen: std::collections::HashSet<&[u64], std::hash::BuildHasherDefault<crate::sema::Fx>> = Default::default();
                    for row in rows.chunks(w) {
                        if seen.insert(row) {
                            distinct.extend_from_slice(row);
                        }
                    }
                }
                let r = Rc::new(distinct);
                self.union_memo.insert(key, r.clone());
                r
            }
        };
        for row in rows[nk..].chunks(w) {
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

    /// Ins::SubKeyed (lower): whether any sub-program holds, memoised per file in `union_memo` under an odd key (hash of
    /// the sub-program list and the key registers' values); the stored entry is the key values then the answer
    fn sub_keyed(&mut self, subs: &'p [Prog], keys: &[R]) -> bool {
        use std::hash::Hasher;
        let nk = keys.len();
        let mut h = crate::sema::Fx::default();
        h.write_u64(Self::memo_seed(subs) ^ 0x9e37_79b9_7f4a_7c15);
        for k in keys {
            h.write_u64(self.regs[*k as usize]);
        }
        let key = (h.finish() as usize) | 1;
        self.c.keyed_calls += 1;
        if let Some(r) = self.union_memo.get(&key) {
            if r.len() == nk + 1 && keys.iter().enumerate().all(|(i, k)| r[i] == self.regs[*k as usize]) {
                return r[nk] != 0;
            }
        }
        self.c.keyed_misses += 1;
        let vals: Vec<u64> = keys.iter().map(|k| self.regs[*k as usize]).collect();
        let any = subs.iter().any(|q| {
            self.c.subs += 1;
            self.run_inner(q, 0)
        });
        let mut entry = vals;
        entry.push(u64::from(any));
        self.union_memo.insert(key, Rc::new(entry));
        any
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
