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
    Near { start: R, out: R, subs: Box<[Prog]> },
    Sub { negate: bool, subs: Box<[Prog]> },
    Emit { cols: Box<[Opd]> },
    Yield,
    SpanBetween { a: Opd, b: Opd, out: R },
    SpanBounds { s: Opd, start: R, end: R },
    SourceMatches { span: Opd, base: Option<Opd>, regex: usize, group: Option<usize>, outs: Box<[R]>, capture_text: bool, capture_line: bool },
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
}

pub(crate) struct FactRel {
    /// the model relation that serves it (index into semrows::RELATIONS); None: the provider's reader
    pub model: Option<usize>,
    pub name: String,
    pub relation: Option<ryo_relations::domain::relations::types::Relation>,
    pub kinds: Vec<ryo_relations::domain::relations::types::CellKind>,
    /// a made table (group count): its input's programs, the number of key columns, and whether it counts distinct values
    pub made: Option<(Vec<Prog>, Vec<usize>, Option<usize>, usize)>,
}

pub(crate) struct NavProgram {
    pub rules: Vec<NavRule>,
    pub facts: Vec<FactRel>,
    pub regexes: Vec<Regex>,
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
    regex_of: FxMap<(String, bool), usize>,
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
        self.facts.push(FactRel { model, name: name.to_owned(), relation: Some(relation), kinds: kinds(relation).to_vec(), made: None });
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
                    f => panic!("fn {f}"),
                }
            }
            o => panic!("ins {o}"),
        }
    }
}

impl NavProgram {
    pub(crate) fn load(rules: &[&Value], sym: &mut Symbols) -> Self {
        let mut facts = Vec::new();
        let mut regexes = Vec::new();
        let mut out = Vec::new();
        let mut max_regs = 0;
        for r in rules {
            let nav = &r["nav"];
            let kinds = nav["kinds"].as_object().unwrap();
            let mut loader = Loader { sym, regs: FxMap::default(), kinds, facts: &mut facts, regexes: &mut regexes, regex_of: FxMap::default() };
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
                loader.facts.push(FactRel {
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
        Self { rules: out, facts, regexes, max_regs }
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
    pub visits: u64,
    pub rule_steps: Vec<u64>,
    pub rule_visits: Vec<u64>,
    pub rule_starts: Vec<u64>,
}

/// What a file's facts from the provider are, decoded into the view's ids and the run's text codes.
#[derive(Default)]
pub(crate) struct Facts {
    pub cols: Vec<Vec<Vec<u64>>>,
    pub loaded: Vec<bool>,
    pub provider_done: bool,
    pub idx: FxMap<(u16, u8), Rc<FIdx>>,
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
        });
        if facts.loaded[rel as usize] {
            return;
        }
        let t = Instant::now();
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
            let mut seen: std::collections::HashSet<Vec<u64>, std::hash::BuildHasherDefault<crate::sema::Fx>> = Default::default();
            let mut groups: FxMap<Vec<u64>, u64> = FxMap::default();
            let mut order: Vec<Vec<u64>> = Vec::new();
            for row in rows.chunks((*width).max(1)) {
                let key: Vec<u64> = keys.iter().map(|&k| row[k]).collect();
                let identity: Vec<u64> = match distinct {
                    Some(d) => key.iter().copied().chain(std::iter::once(row[*d])).collect(),
                    None => row.to_vec(),
                };
                if !seen.insert(identity) {
                    continue;
                }
                let e = groups.entry(key.clone()).or_insert_with(|| {
                    order.push(key);
                    0
                });
                *e += 1;
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
