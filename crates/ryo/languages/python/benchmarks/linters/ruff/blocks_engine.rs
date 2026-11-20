//! Spike (ryo-v1np): the blocks engine. See blocks.rs.
use ryo_benchmark::domain::command_line::types::BenchmarkArgument;
use ryo_execution::inbound::use_cases::compile_rules::use_case::{PLAN_TAP, TappedPlans, TappedPlansStop};
use ryo_execution_runtime::outbound::facts::types::ColumnsMut;
use ryo_language::domain::elements::service::syntax_acceptance;
use ryo_language::domain::elements::types::{Elements, SyntaxAcceptance};
use ryo_language::domain::files::types::SourcePath;
use ryo_language::domain::symbols::service::{clear_symbols, find_symbol, text_of};
use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
use ryo_language::inbound::use_cases::parse::types::{FieldRequest, NoteRequest, ParseEngine, ParseRequest};
use ryo_relations::domain::relations::service::{all, kinds, number_of};
use ryo_relations::domain::relations::types::{CellKind, FactColumn, FileId, Relation};
use ryo_relations::domain::terms::types::Symbol;
use ryo_values::domain::plan_parts::types::{Direction, Input, Operand, QueryLiteral, Side, StepScope, Test};
use ryo_values::domain::plans::types::{Operation, Plan};
use ryo_values::domain::query_bindings::types::ParameterBinding;
use rustc_hash::{FxHashMap, FxHashSet};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

type W = u64;
const MAXW: usize = 16;
const SENTINEL: u64 = 1 << 62;

// ---------------------------------------------------------------- lowered programs

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Src {
    Rel(usize),
    Node(usize),
}

#[derive(Clone, Debug)]
enum Lit {
    Glob(usize),
    Regex(usize),
    Num(i64),
    Text(usize),
    Texts(Vec<usize>),
    Nums(Vec<i64>),
}

#[derive(Clone, Debug)]
enum Opd {
    Col(usize),
    Lit(Lit),
    /// A literal of another kind than its column: equal to nothing.
    Never,
}

#[derive(Clone, Debug)]
struct Cmp {
    test: Test,
    l: Opd,
    r: Opd,
    /// Whether text order applies (a symbol column on either side).
    text: bool,
    /// Whether the left operand is a file column (globbed by path).
    file_left: bool,
}

#[derive(Clone, Debug)]
enum N {
    Sel { src: Src, cmps: Vec<Cmp>, kept: Vec<usize> },
    Proj { src: Src, cols: Vec<usize> },
    Ext { src: Src, width: usize, vals: Vec<Lit> },
    Join { l: Src, r: Src, lon: Vec<usize>, ron: Vec<usize>, out: Vec<(bool, usize)>, filt: Vec<Cmp> },
    Union { l: Src, r: Src },
    Group { src: Src, keys: Vec<usize>, distinct: Option<usize> },
    Between { src: Src, start: usize, end: usize, width: usize },
    Closure { edges: Src, keys: Vec<usize>, source: usize, dest: usize },
    Occur { src: Src, span: usize, base_line: Option<usize>, regex: usize, width: usize },
    Anti { l: Src, r: Src, lon: Vec<usize>, ron: Vec<usize>, present: bool },
    Axis { from: Src, edges: usize, dom: Src, dir: Direction, id: usize, efrom: usize, eto: usize, dom_id: usize, dom_filt: Vec<Cmp>, out: Vec<(bool, usize)>, from_width: usize },
}

struct Rule {
    id: String,
    nodes: Vec<N>,
    width: Vec<usize>,
    kinds: Vec<Vec<CellKind>>,
    top: Src,
    top_kinds: Vec<CellKind>,
    core: bool,
    accepted_only: bool,
}

struct Lowered {
    rules: Vec<Rule>,
    blocked: Vec<(String, String)>,
    lits: Vec<String>,
    regexes: Vec<ryo_matching::inbound::use_cases::matching::types::CompiledRegex>,
    globs: Vec<ryo_matching::inbound::use_cases::matching::types::CompiledTextGlobs>,
}

fn rel_no(relation: Relation) -> usize {
    number_of(relation).0
}

fn op_name(op: &Operation) -> String {
    let s = format!("{op:?}");
    s.split(|c: char| !c.is_alphanumeric()).next().unwrap_or("").to_string()
}

struct LowerCx<'a> {
    plan: &'a Plan,
    bindings: &'a [ParameterBinding],
    lits: &'a mut Vec<String>,
    lit_ids: &'a mut FxHashMap<String, usize>,
    regexes: &'a mut Vec<ryo_matching::inbound::use_cases::matching::types::CompiledRegex>,
    globs: &'a mut Vec<ryo_matching::inbound::use_cases::matching::types::CompiledTextGlobs>,
    nodes: Vec<N>,
    width: Vec<usize>,
    kinds: Vec<Vec<CellKind>>,
    map: FxHashMap<usize, Src>,
}

impl LowerCx<'_> {
    fn text(&mut self, text: &str) -> usize {
        if let Some(id) = self.lit_ids.get(text) {
            return *id;
        }
        let id = self.lits.len();
        self.lits.push(text.to_string());
        self.lit_ids.insert(text.to_string(), id);
        id
    }

    fn src_kinds(&self, src: Src) -> Vec<CellKind> {
        match src {
            Src::Rel(r) => kinds(all()[r]).to_vec(),
            Src::Node(n) => self.kinds[n].clone(),
        }
    }

    fn input(&mut self, input: &Input) -> Result<Src, String> {
        match input {
            Input::Relation(r) => {
                let no = rel_no(*r);
                if all()[no] != *r {
                    return Err("relation numbering".into());
                }
                if ryo_relations::domain::relations::service::provenance_of(*r)
                    != ryo_relations::domain::relations::types::Provenance::Trees
                {
                    return Err(format!("host relation {r:?}"));
                }
                if kinds(*r).len() > MAXW {
                    return Err("width".into());
                }
                Ok(Src::Rel(no))
            }
            Input::Step(s) => self.step(usize::from(*s)),
            Input::BoundObservation(_) => Err("BoundObservation".into()),
        }
    }

    fn lit(&mut self, operand: &Operand) -> Result<Lit, String> {
        match operand {
            Operand::Literal(QueryLiteral::Number(n)) => Ok(Lit::Num(n.0)),
            Operand::Literal(QueryLiteral::Text(t)) => Ok(Lit::Text(self.text(&t.0))),
            Operand::Parameter(p) => {
                let b = self.bindings.get(p.0 as usize).ok_or("unbound parameter")?.clone();
                match b {
                    ParameterBinding::Number(n) => Ok(Lit::Num(n.0)),
                    ParameterBinding::Flag(f) => Ok(Lit::Num(i64::from(
                        f == ryo_values::domain::plan_parts::types::FlagState::Raised,
                    ))),
                    ParameterBinding::Text(t) => Ok(Lit::Text(self.text(&t.0))),
                    ParameterBinding::Texts(ts) => Ok(Lit::Texts(ts.iter().map(|t| self.text(&t.0)).collect())),
                    ParameterBinding::Numbers(ns) => Ok(Lit::Nums(ns.iter().map(|n| n.0).collect())),
                    _ => Err("parameter table".into()),
                }
            }
            Operand::Function(_) => Err("Function operand".into()),
            Operand::Column(_) => Err("column as constant".into()),
        }
    }

    fn cmps(&mut self, cmps: &[ryo_values::domain::plan_parts::types::PlanComparison], k: &[CellKind]) -> Result<Vec<Cmp>, String> {
        let mut out = Vec::new();
        for c in cmps {
            match c.test {
                Test::Equal | Test::NotEqual | Test::LessThan | Test::LessThanOrEqual | Test::GreaterThan
                | Test::GreaterThanOrEqual | Test::In | Test::NotIn | Test::StartsWith | Test::EndsWith | Test::Matches | Test::GlobAny | Test::NotGlobAny => {}
                other => return Err(format!("test {other:?}")),
            }
            let mut text = false;
            let mut opd = |o: &Operand, cx: &mut Self| -> Result<Opd, String> {
                Ok(match o {
                    Operand::Column(i) => {
                        let i = i.0 as usize;
                        if k.get(i) == Some(&CellKind::Symbol) {
                            text = true;
                        }
                        Opd::Col(i)
                    }
                    other => {
                        let l = cx.lit(other)?;
                        if matches!(l, Lit::Text(_) | Lit::Texts(_)) {
                            text = true;
                        }
                        Opd::Lit(l)
                    }
                })
            };
            let mut l = opd(&c.left, self)?;
            let mut r = opd(&c.right, self)?;
            if matches!(c.test, Test::GlobAny | Test::NotGlobAny) {
                let patterns: Vec<String> = match &r {
                    Opd::Lit(Lit::Text(t)) => vec![self.lits[*t].clone()],
                    Opd::Lit(Lit::Texts(ts)) => ts.iter().map(|t| self.lits[*t].clone()).collect(),
                    _ => return Err("test GlobAny shape".into()),
                };
                let Opd::Col(_) = &l else { return Err("test GlobAny shape".into()) };
                use ryo_matching::inbound::use_cases::matching::interface::MatchingUseCase as _;
                let m = ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl::new();
                let globs: Vec<ryo_matching::inbound::use_cases::matching::types::GlobPattern<'_>> =
                    patterns.iter().map(|p| ryo_matching::inbound::use_cases::matching::types::GlobPattern(p)).collect();
                self.globs.push(m.compile_text_globs(&globs));
                r = Opd::Lit(Lit::Glob(self.globs.len() - 1));
                out.push(Cmp { test: c.test, file_left: matches!(&l, Opd::Col(x) if k.get(*x) == Some(&CellKind::File)), l, r, text: true });
                continue;
            }
            if c.test == Test::Matches {
                let (Opd::Col(_), Opd::Lit(Lit::Text(t))) = (&l, &r) else { return Err("test Matches shape".into()) };
                use ryo_matching::inbound::use_cases::matching::interface::MatchingUseCase as _;
                let m = ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl::new();
                let re = m
                    .compile_regex(ryo_matching::inbound::use_cases::matching::types::RegexSource(&self.lits[*t]))
                    .map_err(|_| "test Matches invalid")?;
                self.regexes.push(re);
                r = Opd::Lit(Lit::Regex(self.regexes.len() - 1));
                out.push(Cmp { test: c.test, file_left: matches!(&l, Opd::Col(x) if k.get(*x) == Some(&CellKind::File)), l, r, text: true });
                continue;
            }
            // A flag equals the word it is shown as ("true", "false"); a literal of another kind than its column equals nothing.
            let fix = |a: &Opd, b: &Opd, lits: &Vec<String>| -> Option<Opd> {
                let (Opd::Col(col), Opd::Lit(lit)) = (a, b) else { return None };
                match (k.get(*col).copied(), lit) {
                    (Some(CellKind::Flag), Lit::Text(t)) => Some(match lits[*t].as_str() {
                        "true" => Opd::Lit(Lit::Num(1)),
                        "false" => Opd::Lit(Lit::Num(0)),
                        _ => Opd::Never,
                    }),
                    (Some(CellKind::Number | CellKind::Flag), Lit::Text(_) | Lit::Texts(_)) => Some(Opd::Never),
                    (Some(CellKind::Symbol), Lit::Num(_) | Lit::Nums(_)) => Some(Opd::Never),
                    (Some(CellKind::Node | CellKind::Span | CellKind::File), _) => Some(Opd::Never),
                    _ => None,
                }
            };
            if let Some(f) = fix(&l, &r, self.lits) {
                text = text && !matches!(f, Opd::Lit(_));
                r = f;
            } else if let Some(f) = fix(&r, &l, self.lits) {
                text = text && !matches!(f, Opd::Lit(_));
                l = f;
            }
            out.push(Cmp { test: c.test, file_left: false, l, r, text });
        }
        Ok(out)
    }

    fn push(&mut self, n: N, kinds: Vec<CellKind>) -> Src {
        self.width.push(kinds.len());
        self.kinds.push(kinds);
        self.nodes.push(n);
        Src::Node(self.nodes.len() - 1)
    }

    fn step(&mut self, at: usize) -> Result<Src, String> {
        if let Some(s) = self.map.get(&at) {
            return Ok(*s);
        }
        let step = self.plan.steps.get(at).ok_or("no step")?;
        if step.scope != StepScope::PerFile {
            return Err("CrossFile".into());
        }
        let src = match &step.operation {
            Operation::Select { from, comparisons, kept } => {
                let src = self.input(from)?;
                let k = self.src_kinds(src);
                let cmps = self.cmps(comparisons, &k)?;
                let kept: Vec<usize> = kept.iter().map(|c| c.0 as usize).collect();
                let ok = if kept.is_empty() { k.clone() } else { kept.iter().map(|c| k[*c]).collect() };
                self.push(N::Sel { src, cmps, kept }, ok)
            }
            Operation::Project { from, columns, .. } => {
                let src = self.input(from)?;
                let k = self.src_kinds(src);
                let cols: Vec<usize> = columns.iter().map(|c| c.0 as usize).collect();
                let ok = cols.iter().map(|c| k[*c]).collect();
                self.push(N::Proj { src, cols }, ok)
            }
            Operation::Extend { from, values } => {
                let src = self.input(from)?;
                let mut k = self.src_kinds(src);
                let width = k.len();
                let mut vals = Vec::new();
                for v in values {
                    let l = self.lit(v)?;
                    k.push(match l {
                        Lit::Num(_) => CellKind::Number,
                        Lit::Text(_) => CellKind::Symbol,
                        _ => return Err("Extend list".into()),
                    });
                    vals.push(l);
                }
                self.push(N::Ext { src, width, vals }, k)
            }
            Operation::Join { left, right, left_on, right_on, output, filter, view, reach } => {
                if reach.is_some() {
                    return Err("Join reach".into());
                }
                let l = self.input(left)?;
                let r = self.input(right)?;
                let lk = self.src_kinds(l);
                let rk = self.src_kinds(r);
                let placed = |c: usize| view.get(c).map_or(c, |p| p.0 as usize);
                let lon: Vec<usize> = left_on.iter().map(|c| c.0 as usize).collect();
                let ron: Vec<usize> = right_on.iter().map(|c| placed(c.0 as usize)).collect();
                let out: Vec<(bool, usize)> = output
                    .iter()
                    .map(|c| match c.side {
                        Side::Left => (false, c.column.0 as usize),
                        Side::Right => (true, placed(c.column.0 as usize)),
                    })
                    .collect();
                let filt = self.cmps(filter, &rk)?;
                let ok = out.iter().map(|(right, c)| if *right { rk[*c] } else { lk[*c] }).collect();
                self.push(N::Join { l, r, lon, ron, out, filt }, ok)
            }
            Operation::SourceMatches { from, span, base_line, pattern, capture, .. } => {
                if capture.is_some() {
                    return Err("SourceMatches capture".into());
                }
                let src = self.input(from)?;
                let mut k = self.src_kinds(src);
                let width = k.len();
                k.push(CellKind::Span);
                if base_line.is_some() {
                    k.push(CellKind::Number);
                }
                use ryo_matching::inbound::use_cases::matching::interface::MatchingUseCase as _;
                let m = ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl::new();
                let re = m
                    .compile_regex(ryo_matching::inbound::use_cases::matching::types::RegexSource(&pattern.0))
                    .map_err(|_| "SourceMatches invalid")?;
                self.regexes.push(re);
                let regex = self.regexes.len() - 1;
                self.push(N::Occur { src, span: span.0 as usize, base_line: base_line.map(|c| c.0 as usize), regex, width }, k)
            }
            Operation::Closure { edges, keys, source, destination } => {
                let src = self.input(edges)?;
                let k = self.src_kinds(src);
                let keys: Vec<usize> = keys.iter().map(|c| c.0 as usize).collect();
                let mut ok: Vec<CellKind> = keys.iter().map(|c| k[*c]).collect();
                ok.push(k[source.0 as usize]);
                ok.push(k[destination.0 as usize]);
                self.push(N::Closure { edges: src, keys, source: source.0 as usize, dest: destination.0 as usize }, ok)
            }
            Operation::SpanBetween { from, start, end } => {
                let src = self.input(from)?;
                let mut k = self.src_kinds(src);
                let width = k.len();
                k.push(CellKind::Span);
                self.push(N::Between { src, start: start.0 as usize, end: end.0 as usize, width }, k)
            }
            Operation::GroupCount { from, keys, distinct } => {
                let src = self.input(from)?;
                let k = self.src_kinds(src);
                let keys: Vec<usize> = keys.iter().map(|c| c.0 as usize).collect();
                let mut ok: Vec<CellKind> = keys.iter().map(|c| k[*c]).collect();
                ok.push(CellKind::Number);
                self.push(N::Group { src, keys, distinct: distinct.map(|c| c.0 as usize) }, ok)
            }
            Operation::Union { left, right } => {
                let l = self.input(left)?;
                let r = self.input(right)?;
                let k = self.src_kinds(l);
                self.push(N::Union { l, r }, k)
            }
            Operation::Antijoin { left, right, left_on, right_on } => {
                let l = self.input(left)?;
                let r = self.input(right)?;
                let k = self.src_kinds(l);
                let lon = left_on.iter().map(|c| c.0 as usize).collect();
                let ron = right_on.iter().map(|c| c.0 as usize).collect();
                self.push(N::Anti { l, r, lon, ron, present: false }, k)
            }
            Operation::Difference { left, right } | Operation::Intersection { left, right } => {
                let present = matches!(step.operation, Operation::Intersection { .. });
                let l = self.input(left)?;
                let r = self.input(right)?;
                let k = self.src_kinds(l);
                let all_cols: Vec<usize> = (0..k.len()).collect();
                self.push(N::Anti { l, r, lon: all_cols.clone(), ron: all_cols, present }, k)
            }
            Operation::Axis { from, edges, domain, direction, id, parent, child, domain_id, domain_filter, domain_view, output } => {
                if matches!(direction, Direction::Deep | Direction::PreviousSibling | Direction::NextSibling) {
                    return Err(format!("Axis {direction:?}"));
                }
                let f = self.input(from)?;
                let e = match self.input(edges)? {
                    Src::Rel(r) => r,
                    Src::Node(_) => return Err("Axis derived edges".into()),
                };
                let d = self.input(domain)?;
                let fk = self.src_kinds(f);
                let dk = self.src_kinds(d);
                let up = matches!(direction, Direction::Parent | Direction::Ancestor | Direction::NearestAncestor);
                let (p, c) = (parent.0 as usize, child.0 as usize);
                let (efrom, eto) = if up { (c, p) } else { (p, c) };
                let did = domain_view.get(domain_id.0 as usize).map_or(domain_id.0 as usize, |x| x.0 as usize);
                let dom_filt = self.cmps(domain_filter, &dk)?;
                let out: Vec<(bool, usize)> = output
                    .iter()
                    .map(|c| match c.side {
                        Side::Left => (false, c.column.0 as usize),
                        Side::Right => (true, c.column.0 as usize),
                    })
                    .collect();
                let ok = if out.is_empty() {
                    let mut k = fk.clone();
                    k.push(CellKind::Node);
                    k
                } else {
                    out.iter().map(|(r, c)| if *r { dk[*c] } else { fk[*c] }).collect()
                };
                self.push(
                    N::Axis { from: f, edges: e, dom: d, dir: *direction, id: id.0 as usize, efrom, eto, dom_id: did, dom_filt, out, from_width: fk.len() },
                    ok,
                )
            }
            other => return Err(op_name(other)),
        };
        if let Src::Node(n) = src {
            if self.width[n] > MAXW {
                return Err("width".into());
            }
        }
        self.map.insert(at, src);
        Ok(src)
    }
}

fn lower_all(tapped: &TappedPlans, accepted: &FxHashSet<String>) -> Lowered {
    let mut lits = Vec::new();
    let mut lit_ids = FxHashMap::default();
    let mut regexes = Vec::new();
    let mut globs = Vec::new();
    let mut rules = Vec::new();
    let mut blocked = Vec::new();
    for ((id, plan), bindings) in tapped.ids.iter().zip(&tapped.plans).zip(&tapped.bindings) {
        let mut cx = LowerCx {
            plan,
            bindings,
            lits: &mut lits,
            lit_ids: &mut lit_ids,
            regexes: &mut regexes,
            globs: &mut globs,
            nodes: Vec::new(),
            width: Vec::new(),
            kinds: Vec::new(),
            map: FxHashMap::default(),
        };
        let emit = usize::from(plan.emit.step);
        let (top, core) = match cx.step(emit) {
            Ok(s) => (Ok(s), false),
            Err(why) => {
                // The core: the left input of the emitted antijoin (the suppression tail's right side is what failed).
                match &plan.steps[emit].operation {
                    Operation::Antijoin { left, .. } => {
                        cx.map.clear();
                        match cx.input(left) {
                            Ok(s) => (Ok(s), true),
                            Err(core_why) => (Err(core_why), true),
                        }
                    }
                    _ => (Err(why), false),
                }
            }
        };
        match top {
            Ok(top) => {
                let top_kinds = cx.src_kinds(top);
                rules.push(Rule {
                    id: id.clone(),
                    nodes: cx.nodes,
                    width: cx.width,
                    kinds: cx.kinds,
                    top,
                    top_kinds,
                    core,
                    accepted_only: accepted.contains(id),
                });
            }
            Err(why) => blocked.push((id.clone(), why)),
        }
    }
    Lowered { rules, blocked, lits, regexes, globs }
}

// ---------------------------------------------------------------- per-file data

enum Index {
    Dense { base: u64, head: Vec<u32>, next: Vec<u32> },
    Hash { map: FxHashMap<u64, u32>, next: Vec<u32> },
}
const NONE: u32 = u32::MAX;

impl Index {
    fn first(&self, key: u64) -> u32 {
        match self {
            Index::Dense { base, head, .. } => {
                if key >> 32 != *base {
                    return NONE;
                }
                head.get((key & 0xffff_ffff) as usize).copied().unwrap_or(NONE)
            }
            Index::Hash { map, .. } => map.get(&key).copied().unwrap_or(NONE),
        }
    }
    fn next(&self, row: u32) -> u32 {
        match self {
            Index::Dense { next, .. } | Index::Hash { next, .. } => next[row as usize],
        }
    }
    fn build(col: &[u64], kind: CellKind) -> Index {
        let n = col.len();
        let mut next = vec![NONE; n];
        if kind == CellKind::Node && n > 0 {
            let base = col[0] >> 32;
            let mut max = 0u64;
            let mut dense = true;
            for w in col {
                if w >> 32 != base {
                    dense = false;
                    break;
                }
                max = max.max(w & 0xffff_ffff);
            }
            if dense && max < (1 << 22) {
                let mut head = vec![NONE; max as usize + 1];
                for row in (0..n).rev() {
                    let p = (col[row] & 0xffff_ffff) as usize;
                    next[row] = head[p];
                    head[p] = row as u32;
                }
                return Index::Dense { base, head, next };
            }
        }
        let mut map = FxHashMap::with_capacity_and_hasher(n, Default::default());
        for row in (0..n).rev() {
            let e = map.entry(col[row]).or_insert(NONE);
            next[row] = *e;
            *e = row as u32;
        }
        Index::Hash { map, next }
    }
}

#[derive(Default)]
struct Counters {
    anchor_rows: Cell<u64>,
    steps: Cell<u64>,
    lookups: Cell<u64>,
    index_ns: Cell<u64>,
    est_calls: Cell<u64>,
    node_calls: [Cell<u64>; 8],
    /// Anchor rows counted once per file per (relation, column, value) bucket: what a shared dispatch would start.
    shared_anchor_rows: Cell<u64>,
    shared_seen: RefCell<FxHashSet<(usize, usize, u64)>>,
    /// Steps at which the running rule is stopped in this file (a rule the approach runs badly), and whether it was.
    budget_end: Cell<u64>,
    over: Cell<bool>,
}

struct Ctx<'a> {
    facts: &'a [Vec<FactColumn>],
    file: u64,
    symbols: &'a Symbols,
    source: &'a [u8],
    lits: &'a [String],
    regexes: &'a [ryo_matching::inbound::use_cases::matching::types::CompiledRegex],
    globs: &'a [ryo_matching::inbound::use_cases::matching::types::CompiledTextGlobs],
    path: &'a str,
    group_memo: RefCell<FxHashMap<(usize, usize), Rc<Vec<Vec<W>>>>>,
    lit_words: RefCell<Vec<u64>>,
    idx: RefCell<FxHashMap<(usize, usize), Rc<Index>>>,
    idx_fast: RefCell<Vec<Option<Rc<Index>>>>,
    bucket_n: RefCell<FxHashMap<(usize, usize, u64), u64>>,
    est_memo: &'a RefCell<FxHashMap<(usize, usize, u32), u64>>,
    c: &'a Counters,
}

#[derive(Clone, Copy)]
struct Row {
    w: [W; MAXW],
    n: usize,
}
impl Row {
    fn new(n: usize) -> Row {
        Row { w: [0; MAXW], n }
    }
    fn s(&self) -> &[W] {
        &self.w[..self.n]
    }
}

type Bound = [Option<W>; MAXW];
const UNB: Bound = [None; MAXW];

impl Ctx<'_> {
    fn rows(&self, r: usize) -> usize {
        self.facts[r].first().map_or(0, |c| c.0.len())
    }
    fn lit_word(&self, id: usize) -> W {
        let mut words = self.lit_words.borrow_mut();
        if words[id] != u64::MAX {
            return words[id];
        }
        let w = match find_symbol(self.symbols, ByteSlice(self.source), ByteSlice(self.lits[id].as_bytes())) {
            Some(s) => u64::from(s.0),
            None => SENTINEL | id as u64,
        };
        words[id] = w;
        w
    }
    fn text(&self, w: W) -> &[u8] {
        if w & SENTINEL != 0 {
            return self.lits[(w & !SENTINEL) as usize].as_bytes();
        }
        match text_of(self.symbols, ByteSlice(self.source), Symbol(w as u32)) {
            Some(t) => t.0.as_bytes(),
            None => b"",
        }
    }
    fn index(&self, r: usize, col: usize) -> Rc<Index> {
        if let Some(Some(i)) = self.idx_fast.borrow().get(r * MAXW + col) {
            return Rc::clone(i);
        }
        let t = Instant::now();
        let built = Rc::new(Index::build(&self.facts[r][col].0, kinds(all()[r])[col]));
        self.c.index_ns.set(self.c.index_ns.get() + t.elapsed().as_nanos() as u64);
        self.idx_fast.borrow_mut()[r * MAXW + col] = Some(Rc::clone(&built));
        built
    }

    fn lit_val(&self, l: &Lit) -> Option<W> {
        match l {
            Lit::Num(n) => Some(*n as u64),
            Lit::Text(t) => Some(self.lit_word(*t)),
            _ => None,
        }
    }

    fn opd(&self, o: &Opd, row: &[W]) -> W {
        match o {
            Opd::Col(c) => row[*c],
            Opd::Lit(l) => self.lit_val(l).unwrap_or(0),
            Opd::Never => 0,
        }
    }

    fn check(&self, cmps: &[Cmp], row: &[W]) -> bool {
        for c in cmps {
            if !self.test(c, row) {
                return false;
            }
        }
        true
    }

    fn test(&self, c: &Cmp, row: &[W]) -> bool {
        use std::cmp::Ordering as O;
        if matches!(c.l, Opd::Never) || matches!(c.r, Opd::Never) {
            return matches!(c.test, Test::NotEqual | Test::NotIn);
        }
        match c.test {
            Test::In | Test::NotIn => {
                let v = self.opd(&c.l, row);
                let hit = match &c.r {
                    Opd::Lit(Lit::Texts(ts)) => ts.iter().any(|t| self.lit_word(*t) == v),
                    Opd::Lit(Lit::Nums(ns)) => ns.iter().any(|n| *n as u64 == v),
                    other => self.opd(other, row) == v,
                };
                return hit == (c.test == Test::In);
            }
            _ => {}
        }
        if let (Test::GlobAny | Test::NotGlobAny, Opd::Lit(Lit::Glob(g)), Opd::Col(col)) = (c.test, &c.r, &c.l) {
            use ryo_matching::inbound::use_cases::matching::interface::MatchingUseCase as _;
            let a = row[*col];
            // A file column globs the file's path; a name its text.
            let owned;
            let text: &str = if a == self.file && c.file_left {
                self.path
            } else {
                owned = String::from_utf8_lossy(self.text(a)).into_owned();
                &owned
            };
            let m = ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl::new();
            let hit = m.matches_text_globs(&self.globs[*g], ryo_matching::inbound::use_cases::matching::types::SearchText(text))
                == ryo_matching::inbound::use_cases::matching::types::MatchOutcome::Matches;
            return hit == (c.test == Test::GlobAny);
        }
        if let (Test::Matches, Opd::Lit(Lit::Regex(re))) = (c.test, &c.r) {
            use ryo_matching::inbound::use_cases::matching::interface::MatchingUseCase as _;
            let a = self.opd(&c.l, row);
            let Ok(text) = std::str::from_utf8(self.text(a)) else { return false };
            let m = ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl::new();
            return m.matches_regex(&self.regexes[*re], ryo_matching::inbound::use_cases::matching::types::SearchText(text))
                == ryo_matching::inbound::use_cases::matching::types::MatchOutcome::Matches;
        }
        let a = self.opd(&c.l, row);
        let b = self.opd(&c.r, row);
        match c.test {
            Test::Equal => a == b,
            Test::NotEqual => a != b,
            Test::StartsWith => self.text(a).starts_with(self.text(b)),
            Test::EndsWith => self.text(a).ends_with(self.text(b)),
            _ => {
                let ord = if c.text { self.text(a).cmp(self.text(b)) } else { (a as i64).cmp(&(b as i64)) };
                match c.test {
                    Test::LessThan => ord == O::Less,
                    Test::LessThanOrEqual => ord != O::Greater,
                    Test::GreaterThan => ord == O::Greater,
                    Test::GreaterThanOrEqual => ord != O::Less,
                    _ => false,
                }
            }
        }
    }
}

/// Literal equalities of `cmps` pushed into `bound`; false when two disagree (no row can match).
fn push_lits(cx: &Ctx<'_>, cmps: &[Cmp], bound: &mut Bound) -> bool {
    for c in cmps {
        if c.test != Test::Equal {
            continue;
        }
        let (col, lit) = match (&c.l, &c.r) {
            (Opd::Col(col), Opd::Lit(l)) | (Opd::Lit(l), Opd::Col(col)) => (*col, l),
            _ => continue,
        };
        let Some(v) = cx.lit_val(lit) else { continue };
        match bound[col] {
            Some(b) if b != v => return false,
            _ => bound[col] = Some(v),
        }
    }
    true
}

// ---------------------------------------------------------------- the executor

struct Exec<'r, 'c> {
    ri: usize,
    rule: &'r Rule,
    cx: &'r Ctx<'c>,
}

impl Exec<'_, '_> {
    fn width(&self, s: Src) -> usize {
        match s {
            Src::Rel(r) => kinds(all()[r]).len(),
            Src::Node(n) => self.rule.width[n],
        }
    }
    fn kind(&self, s: Src, c: usize) -> CellKind {
        match s {
            Src::Rel(r) => kinds(all()[r])[c],
            Src::Node(n) => self.rule.kinds[n][c],
        }
    }

    /// Rows of a relation matching `bound` and `cmps`; `f` returns false to stop. Returns false when stopped.
    fn rel(&self, r: usize, bound: &Bound, cmps: &[Cmp], f: &mut dyn FnMut(&[W]) -> bool) -> bool {
        let cx = self.cx;
        if cx.c.steps.get() > cx.c.budget_end.get() {
            cx.c.over.set(true);
            return false;
        }
        let k = kinds(all()[r]);
        let w = k.len();
        let mut b = *bound;
        let anchor = (0..w).all(|c| b[c].is_none() || k[c] == CellKind::File);
        if !push_lits(cx, cmps, &mut b) {
            return true;
        }
        // File columns hold the open file: a bound file either matches every row or none.
        for c in 0..w {
            if k[c] == CellKind::File {
                if let Some(v) = b[c] {
                    if v != cx.file {
                        return true;
                    }
                    b[c] = None;
                }
            }
        }
        let cols = &cx.facts[r];
        let n = cx.rows(r);
        if n == 0 {
            return true;
        }
        let mut pick = None;
        for c in 0..w {
            if b[c].is_some() && (pick.is_none() || k[c] == CellKind::Node) {
                if pick.is_none() || k[c] == CellKind::Node {
                    pick = Some(c);
                }
            }
        }
        let mut row = Row::new(w);
        let mut visit = |at: usize, row: &mut Row| -> Option<bool> {
            for c in 0..w {
                let v = cols[c].0[at];
                if let Some(x) = b[c] {
                    if x != v {
                        return None;
                    }
                }
                row.w[c] = v;
            }
            if !cx.check(cmps, row.s()) {
                return None;
            }
            Some(f(row.s()))
        };
        match pick {
            Some(c) => {
                cx.c.lookups.set(cx.c.lookups.get() + 1);
                let idx = cx.index(r, c);
                let mut at = idx.first(b[c].unwrap_or(0));
                let mut steps = 0;
                let mut go = true;
                while at != NONE {
                    steps += 1;
                    if visit(at as usize, &mut row) == Some(false) {
                        go = false;
                        break;
                    }
                    at = idx.next(at);
                }
                cx.c.steps.set(cx.c.steps.get() + steps);
                if anchor {
                    cx.c.anchor_rows.set(cx.c.anchor_rows.get() + steps);
                    if cx.c.shared_seen.borrow_mut().insert((r, c, b[c].unwrap_or(0))) {
                        cx.c.shared_anchor_rows.set(cx.c.shared_anchor_rows.get() + steps);
                    }
                }
                go
            }
            None => {
                cx.c.anchor_rows.set(cx.c.anchor_rows.get() + n as u64);
                if cx.c.shared_seen.borrow_mut().insert((r, usize::MAX, 0)) {
                    cx.c.shared_anchor_rows.set(cx.c.shared_anchor_rows.get() + n as u64);
                }
                cx.c.steps.set(cx.c.steps.get() + n as u64);
                for at in 0..n {
                    if visit(at, &mut row) == Some(false) {
                        return false;
                    }
                }
                true
            }
        }
    }

    fn each(&self, s: Src, bound: &Bound, cmps: &[Cmp], f: &mut dyn FnMut(&[W]) -> bool) -> bool {
        match s {
            Src::Rel(r) => self.rel(r, bound, cmps, f),
            Src::Node(n) => {
                let mut b = *bound;
                if !push_lits(self.cx, cmps, &mut b) {
                    return true;
                }
                if cmps.is_empty() {
                    self.node(n, &b, f)
                } else {
                    self.node(n, &b, &mut |row| if self.cx.check(cmps, row) { f(row) } else { true })
                }
            }
        }
    }

    fn exists(&self, s: Src, bound: &Bound, cmps: &[Cmp]) -> bool {
        let mut found = false;
        self.each(s, bound, cmps, &mut |_| {
            found = true;
            false
        });
        found
    }

    /// A rough per-file size of `s` under `bound` (bound non-file columns count as one row).
    fn est(&self, s: Src, bound: &Bound, cmps: &[Cmp]) -> u64 {
        let cx = self.cx;
        let mut b = *bound;
        if !push_lits(cx, cmps, &mut b) {
            return 0;
        }
        match s {
            Src::Rel(r) => {
                // The file's own sizes: a bound node column is one row, another bound column (or literal) its bucket, else the relation.
                let k = kinds(all()[r]);
                let n = cx.rows(r) as u64;
                if n == 0 {
                    return 0;
                }
                let mut best = n;
                for c in 0..k.len() {
                    if k[c] == CellKind::File {
                        continue;
                    }
                    let Some(v) = b[c] else { continue };
                    if k[c] == CellKind::Node {
                        best = best.min(1);
                        continue;
                    }
                    let known = cx.bucket_n.borrow().get(&(r, c, v)).copied();
                    let m = match known {
                        Some(m) => m,
                        None => {
                            let idx = cx.index(r, c);
                            let mut at = idx.first(v);
                            let mut m = 0;
                            while at != NONE {
                                m += 1;
                                at = idx.next(at);
                            }
                            cx.bucket_n.borrow_mut().insert((r, c, v), m);
                            m
                        }
                    };
                    best = best.min(m);
                }
                best
            }
            Src::Node(n) => {
                // Estimates are kept per file by which columns are bound, not by their values: they only pick a driving side.
                let mut mask = 0u32;
                for c in 0..self.rule.width[n] {
                    if b[c].is_some() {
                        mask |= 1 << c;
                    }
                }
                if let Some(v) = cx.est_memo.borrow().get(&(self.ri, n, mask)) {
                    return *v;
                }
                cx.c.est_calls.set(cx.c.est_calls.get() + 1);
                let v = self.est_node(n, &b);
                cx.est_memo.borrow_mut().insert((self.ri, n, mask), v);
                v
            }
        }
    }

    fn est_node(&self, n: usize, b: &Bound) -> u64 {
        let b = *b;
        match &self.rule.nodes[n] {
                N::Sel { src, cmps, kept } => {
                    let cb = self.map_kept(&b, kept, self.width(*src));
                    self.est(*src, &cb, cmps)
                }
                N::Proj { src, cols } => {
                    let cb = self.map_kept(&b, cols, self.width(*src));
                    self.est(*src, &cb, &[])
                }
                N::Ext { src, width, .. } => {
                    let mut cb = UNB;
                    cb[..*width].copy_from_slice(&b[..*width]);
                    self.est(*src, &cb, &[])
                }
                N::Join { l, r, out, lon, ron, filt } => {
                    let (lb, rb) = self.split(&b, out, *l, *r, lon, ron);
                    self.est(*l, &lb, &[]).min(self.est(*r, &rb, filt))
                }
                N::Union { l, r } => self.est(*l, &b, &[]) + self.est(*r, &b, &[]),
                N::Closure { edges, keys, source, .. } => {
                    let mut cb = UNB;
                    for (i, c) in keys.iter().enumerate() {
                        cb[*c] = b[i];
                    }
                    cb[*source] = b[keys.len()];
                    self.est(*edges, &cb, &[]).saturating_mul(4)
                }
                N::Between { src, width, .. } | N::Occur { src, width, .. } => {
                    let mut cb = UNB;
                    cb[..*width].copy_from_slice(&b[..*width]);
                    self.est(*src, &cb, &[])
                }
                N::Group { src, keys, .. } => {
                    let cb = self.map_kept(&b, keys, self.width(*src));
                    self.est(*src, &cb, &[])
                }
                N::Anti { l, .. } => self.est(*l, &b, &[]),
                N::Axis { from, from_width, out, .. } => {
                    let mut fb = UNB;
                    if out.is_empty() {
                        fb[..*from_width].copy_from_slice(&b[..*from_width]);
                    }
                    self.est(*from, &fb, &[])
                }
        }
    }

    fn map_kept(&self, b: &Bound, kept: &[usize], src_width: usize) -> Bound {
        let mut cb = UNB;
        if kept.is_empty() {
            cb[..src_width].copy_from_slice(&b[..src_width]);
        } else {
            for (i, c) in kept.iter().enumerate() {
                if let Some(v) = b[i] {
                    cb[*c] = Some(v);
                }
            }
        }
        cb
    }

    fn split(&self, b: &Bound, out: &[(bool, usize)], l: Src, r: Src, lon: &[usize], ron: &[usize]) -> (Bound, Bound) {
        let mut lb = UNB;
        let mut rb = UNB;
        for (i, (right, c)) in out.iter().enumerate() {
            if let Some(v) = b[i] {
                if *right {
                    rb[*c] = Some(v);
                } else {
                    lb[*c] = Some(v);
                }
            }
        }
        for (a, z) in lon.iter().zip(ron) {
            match (lb[*a], rb[*z]) {
                (Some(v), None) => rb[*z] = Some(v),
                (None, Some(v)) => lb[*a] = Some(v),
                _ => {}
            }
        }
        (lb, rb)
    }

    fn useful(&self, s: Src, b: &Bound) -> bool {
        (0..self.width(s)).any(|c| b[c].is_some() && self.kind(s, c) != CellKind::File)
    }

    fn node(&self, n: usize, b: &Bound, f: &mut dyn FnMut(&[W]) -> bool) -> bool {
        let rule = self.rule;
        let slot = match &rule.nodes[n] {
            N::Sel { .. } => 0,
            N::Proj { .. } => 1,
            N::Ext { .. } => 2,
            N::Join { .. } => 3,
            N::Union { .. } => 4,
            N::Group { .. } => 5,
            N::Anti { .. } => 6,
            N::Axis { .. } => 7,
            N::Between { .. } | N::Occur { .. } => 2,
            N::Closure { .. } => 5,
        };
        self.cx.c.node_calls[slot].set(self.cx.c.node_calls[slot].get() + 1);
        match &rule.nodes[n] {
            N::Sel { src, cmps, kept } => {
                let cb = self.map_kept(b, kept, self.width(*src));
                if kept.is_empty() {
                    return self.each(*src, &cb, cmps, f);
                }
                let mut row = Row::new(kept.len());
                self.each(*src, &cb, cmps, &mut |r| {
                    for (i, c) in kept.iter().enumerate() {
                        row.w[i] = r[*c];
                    }
                    f(row.s())
                })
            }
            N::Proj { src, cols } => {
                let cb = self.map_kept(b, cols, self.width(*src));
                let mut row = Row::new(cols.len());
                self.each(*src, &cb, &[], &mut |r| {
                    for (i, c) in cols.iter().enumerate() {
                        row.w[i] = r[*c];
                    }
                    f(row.s())
                })
            }
            N::Ext { src, width, vals } => {
                let mut row = Row::new(width + vals.len());
                for (i, v) in vals.iter().enumerate() {
                    let w = self.cx.lit_val(v).unwrap_or(0);
                    if let Some(x) = b[width + i] {
                        if x != w {
                            return true;
                        }
                    }
                    row.w[width + i] = w;
                }
                let mut cb = UNB;
                cb[..*width].copy_from_slice(&b[..*width]);
                self.each(*src, &cb, &[], &mut |r| {
                    row.w[..*width].copy_from_slice(r);
                    f(row.s())
                })
            }
            N::Union { l, r } => self.each(*l, b, &[], f) && self.each(*r, b, &[], f),
            N::Closure { edges, keys, source, dest } => {
                // Every pair joined by a chain of edges, within equal keys: walked from each start the bound columns allow.
                let nk = keys.len();
                let mut sb = UNB;
                for (i, c) in keys.iter().enumerate() {
                    sb[*c] = b[i];
                }
                sb[*source] = b[nk];
                let want_dest = b[nk + 1];
                let mut starts: Vec<Vec<W>> = Vec::new();
                let mut seen_starts: FxHashSet<Vec<W>> = FxHashSet::default();
                self.each(*edges, &sb, &[], &mut |r| {
                    let mut st: Vec<W> = keys.iter().map(|c| r[*c]).collect();
                    st.push(r[*source]);
                    if seen_starts.insert(st.clone()) {
                        starts.push(st);
                    }
                    true
                });
                starts.sort_unstable();
                let mut row = Row::new(nk + 2);
                let mut visited: FxHashSet<W> = FxHashSet::default();
                let mut frontier: Vec<W> = Vec::new();
                for st in &starts {
                    visited.clear();
                    frontier.clear();
                    frontier.push(st[nk]);
                    let mut reached: Vec<W> = Vec::new();
                    while let Some(e) = frontier.pop() {
                        let mut eb = UNB;
                        for (i, c) in keys.iter().enumerate() {
                            eb[*c] = Some(st[i]);
                        }
                        eb[*source] = Some(e);
                        self.each(*edges, &eb, &[], &mut |r| {
                            let d = r[*dest];
                            if visited.insert(d) {
                                reached.push(d);
                                frontier.push(d);
                            }
                            true
                        });
                    }
                    reached.sort_unstable();
                    for d in reached {
                        if want_dest.is_some_and(|w| w != d) {
                            continue;
                        }
                        row.w[..nk + 1].copy_from_slice(st);
                        row.w[nk + 1] = d;
                        if !f(row.s()) {
                            return false;
                        }
                    }
                }
                true
            }
            N::Occur { src, span, base_line, regex, width } => {
                use ryo_matching::inbound::use_cases::matching::interface::MatchingUseCase as _;
                let m = ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl::new();
                let no_span: u64 = {
                    let s = ryo_relations::domain::relations::constants::NO_SPAN;
                    ((s.start.0 as u64) << 32) | s.end.0 as u64
                };
                let mut cb = UNB;
                cb[..*width].copy_from_slice(&b[..*width]);
                let out_w = width + 1 + usize::from(base_line.is_some());
                let want: Vec<(usize, u64)> = (*width..out_w).filter_map(|c| b[c].map(|v| (c, v))).collect();
                let source = self.cx.source;
                let mut row = Row::new(out_w);
                self.each(*src, &cb, &[], &mut |r| {
                    let sp = r[*span];
                    if sp == no_span {
                        return true;
                    }
                    let (start, end) = ((sp >> 32) as usize, (sp & 0xffff_ffff) as usize);
                    if start > end || end > source.len() {
                        return true;
                    }
                    let Ok(text) = std::str::from_utf8(&source[start..end]) else { return true };
                    let mut line: Option<i64> = match base_line {
                        Some(c) => {
                            let n = r[*c] as i64;
                            if n <= 0 {
                                return true;
                            }
                            Some(n)
                        }
                        None => None,
                    };
                    let bytes = text.as_bytes();
                    let mut cursor = 0usize;
                    let mut pending_cr = false;
                    let mut go = true;
                    let _ = m.visit_regex_matches(
                        &self.cx.regexes[*regex],
                        ryo_matching::inbound::use_cases::matching::types::SearchText(text),
                        &mut |found| {
                            let (ws, we) = (found.0.start, found.0.end);
                            if let Some(current) = line.as_mut() {
                                // The line of the occurrence's first byte, counted as the engine counts it (CR, LF, CRLF).
                                for &byte in &bytes[cursor..ws.max(cursor)] {
                                    if pending_cr {
                                        *current += 1;
                                        pending_cr = false;
                                        if byte == b'\n' {
                                            continue;
                                        }
                                    }
                                    if byte == b'\r' {
                                        pending_cr = true;
                                    } else if byte == b'\n' {
                                        *current += 1;
                                    }
                                }
                                cursor = ws.max(cursor);
                                if pending_cr && source.get(start + cursor) != Some(&b'\n') {
                                    *current += 1;
                                    pending_cr = false;
                                }
                            }
                            row.w[..*width].copy_from_slice(r);
                            row.w[*width] = (((start + ws) as u64) << 32) | (start + we) as u64;
                            if let Some(current) = line {
                                row.w[width + 1] = current as u64;
                            }
                            if want.iter().any(|(c, v)| row.w[*c] != *v) {
                                return std::ops::ControlFlow::Continue(());
                            }
                            if f(row.s()) {
                                std::ops::ControlFlow::Continue(())
                            } else {
                                go = false;
                                std::ops::ControlFlow::Break(())
                            }
                        },
                    );
                    go
                })
            }
            N::Between { src, start, end, width } => {
                const NO_SPAN: u64 = {
                    let s = ryo_relations::domain::relations::constants::NO_SPAN;
                    ((s.start.0 as u64) << 32) | s.end.0 as u64
                };
                let mut cb = UNB;
                cb[..*width].copy_from_slice(&b[..*width]);
                let want = b[*width];
                let mut row = Row::new(width + 1);
                let len = self.cx.source.len() as u64;
                self.each(*src, &cb, &[], &mut |r| {
                    let (a, z) = (r[*start], r[*end]);
                    let bad = |s: u64| s != NO_SPAN && ((s >> 32) > (s & 0xffff_ffff) || (s & 0xffff_ffff) > len);
                    if bad(a) || bad(z) {
                        return true;
                    }
                    let v = if a == NO_SPAN || z == NO_SPAN {
                        NO_SPAN
                    } else if (a >> 32) > (z & 0xffff_ffff) {
                        return true;
                    } else {
                        (a & !0xffff_ffff) | (z & 0xffff_ffff)
                    };
                    if want.is_some_and(|w| w != v) {
                        return true;
                    }
                    row.w[..*width].copy_from_slice(r);
                    row.w[*width] = v;
                    f(row.s())
                })
            }
            N::Group { src, keys, distinct } => {
                let nk = keys.len();
                let count_bound = b[nk];
                let keys_bound = (0..nk).all(|i| b[i].is_some());
                let emit_group = |key: &[W], count: u64, f: &mut dyn FnMut(&[W]) -> bool| -> bool {
                    if count == 0 || count_bound.is_some_and(|c| c != count) {
                        return true;
                    }
                    let mut row = Row::new(nk + 1);
                    row.w[..nk].copy_from_slice(key);
                    row.w[nk] = count;
                    f(row.s())
                };
                if keys_bound {
                    // A bound group: count the distinct rows (or values) of the source under the key, per candidate.
                    let cb = self.map_kept(b, keys, self.width(*src));
                    let mut set: FxHashSet<Vec<W>> = FxHashSet::default();
                    self.each(*src, &cb, &[], &mut |r| {
                        match distinct {
                            Some(c) => set.insert(vec![r[*c]]),
                            None => set.insert(r.to_vec()),
                        };
                        true
                    });
                    let key: Vec<W> = (0..nk).map(|i| b[i].unwrap_or(0)).collect();
                    return emit_group(&key, set.len() as u64, f);
                }
                // An unbound group: made once per file and rule, then read.
                let memo = self.cx.group_memo.borrow().get(&(self.ri, n)).cloned();
                let rows = match memo {
                    Some(rows) => rows,
                    None => {
                        let mut groups: FxHashMap<Vec<W>, FxHashSet<Vec<W>>> = FxHashMap::default();
                        self.each(*src, &UNB, &[], &mut |r| {
                            let key: Vec<W> = keys.iter().map(|c| r[*c]).collect();
                            let member = match distinct {
                                Some(c) => vec![r[*c]],
                                None => r.to_vec(),
                            };
                            groups.entry(key).or_default().insert(member);
                            true
                        });
                        let rows: Vec<Vec<W>> = groups
                            .into_iter()
                            .map(|(mut k, m)| {
                                k.push(m.len() as u64);
                                k
                            })
                            .collect();
                        let rows = Rc::new(rows);
                        self.cx.group_memo.borrow_mut().insert((self.ri, n), Rc::clone(&rows));
                        rows
                    }
                };
                for r in rows.iter() {
                    if (0..=nk).any(|i| b[i].is_some_and(|v| v != r[i])) {
                        continue;
                    }
                    if !emit_group(&r[..nk], r[nk], f) {
                        return false;
                    }
                }
                true
            }
            N::Join { l, r, lon, ron, out, filt } => {
                let (lb, rb) = self.split(b, out, *l, *r, lon, ron);
                let lu = self.useful(*l, &lb);
                let ru = self.useful(*r, &rb);
                // A side whose only output columns are its keys is read for existence only (a semijoin): one match is enough.
                let r_exist = out.iter().all(|(right, c)| !*right || ron.contains(c));
                let l_exist = out.iter().all(|(right, c)| *right || lon.contains(c));
                let drive_left = match (lu, ru) {
                    (true, false) => true,
                    (false, true) => false,
                    _ => self.est(*l, &lb, &[]) <= self.est(*r, &rb, filt),
                };
                let mut row = Row::new(out.len());
                if drive_left && r_exist {
                    return self.each(*l, &lb, &[], &mut |lr| {
                        let mut ob = rb;
                        for (a, z) in lon.iter().zip(ron) {
                            ob[*z] = Some(lr[*a]);
                        }
                        if !self.exists(*r, &ob, filt) {
                            return true;
                        }
                        for (i, (right, c)) in out.iter().enumerate() {
                            row.w[i] = if *right {
                                let k = ron.iter().position(|z| z == c).unwrap_or(0);
                                lr[lon[k]]
                            } else {
                                lr[*c]
                            };
                        }
                        f(row.s())
                    });
                }
                if !drive_left && l_exist {
                    return self.each(*r, &rb, filt, &mut |rr| {
                        let mut ob = lb;
                        for (a, z) in lon.iter().zip(ron) {
                            ob[*a] = Some(rr[*z]);
                        }
                        if !self.exists(*l, &ob, &[]) {
                            return true;
                        }
                        for (i, (right, c)) in out.iter().enumerate() {
                            row.w[i] = if *right {
                                rr[*c]
                            } else {
                                let k = lon.iter().position(|a| a == c).unwrap_or(0);
                                rr[ron[k]]
                            };
                        }
                        f(row.s())
                    });
                }
                if drive_left {
                    self.each(*l, &lb, &[], &mut |lr| {
                        let mut ob = rb;
                        for (a, z) in lon.iter().zip(ron) {
                            ob[*z] = Some(lr[*a]);
                        }
                        self.each(*r, &ob, filt, &mut |rr| {
                            for (i, (right, c)) in out.iter().enumerate() {
                                row.w[i] = if *right { rr[*c] } else { lr[*c] };
                            }
                            f(row.s())
                        })
                    })
                } else {
                    self.each(*r, &rb, filt, &mut |rr| {
                        let mut ob = lb;
                        for (a, z) in lon.iter().zip(ron) {
                            ob[*a] = Some(rr[*z]);
                        }
                        self.each(*l, &ob, &[], &mut |lr| {
                            for (i, (right, c)) in out.iter().enumerate() {
                                row.w[i] = if *right { rr[*c] } else { lr[*c] };
                            }
                            f(row.s())
                        })
                    })
                }
            }
            N::Anti { l, r, lon, ron, present } => self.each(*l, b, &[], &mut |lr| {
                let mut ob = UNB;
                for (a, z) in lon.iter().zip(ron) {
                    ob[*z] = Some(lr[*a]);
                }
                if self.exists(*r, &ob, &[]) == *present { f(lr) } else { true }
            }),
            N::Axis { from, edges, dom, dir, id, efrom, eto, dom_id, dom_filt, out, from_width } => {
                let mut fb = UNB;
                let mut related_bound = None;
                let mut right_bound: Vec<(usize, W)> = Vec::new();
                if out.is_empty() {
                    fb[..*from_width].copy_from_slice(&b[..*from_width]);
                    related_bound = b[*from_width];
                } else {
                    for (i, (right, c)) in out.iter().enumerate() {
                        if let Some(v) = b[i] {
                            if *right {
                                right_bound.push((*c, v));
                            } else {
                                fb[*c] = Some(v);
                            }
                        }
                    }
                }
                let cx = self.cx;
                let idx = if cx.rows(*edges) > 0 { Some(cx.index(*edges, *efrom)) } else { None };
                let to_col = &cx.facts[*edges][*eto].0;
                let mut row = Row::new(rule.width[n]);
                let mut frontier: Vec<W> = Vec::new();
                let mut nextf: Vec<W> = Vec::new();
                let mut seen: FxHashSet<W> = FxHashSet::default();
                self.each(*from, &fb, &[], &mut |fr| {
                    let Some(idx) = idx.as_deref() else { return true };
                    let start = fr[*id];
                    // Emits one reached entity; false stops everything.
                    let mut emit = |entity: W| -> (bool, bool) {
                        // (admitted, keep going)
                        if let Some(rbv) = related_bound {
                            if out.is_empty() && rbv != entity {
                                // Still must know admission for NearestAncestor's stop.
                                let mut db = UNB;
                                db[*dom_id] = Some(entity);
                                return (self.exists(*dom, &db, dom_filt), true);
                            }
                        }
                        let mut db = UNB;
                        db[*dom_id] = Some(entity);
                        if out.is_empty() {
                            if !self.exists(*dom, &db, dom_filt) {
                                return (false, true);
                            }
                            row.w[..*from_width].copy_from_slice(fr);
                            row.w[*from_width] = entity;
                            return (true, f(row.s()));
                        }
                        for (c, v) in &right_bound {
                            db[*c] = Some(*v);
                        }
                        let mut admitted = false;
                        let go = self.each(*dom, &db, dom_filt, &mut |dr| {
                            admitted = true;
                            for (i, (right, c)) in out.iter().enumerate() {
                                row.w[i] = if *right { dr[*c] } else { fr[*c] };
                            }
                            f(row.s())
                        });
                        (admitted, go)
                    };
                    let follow = |from: W, into: &mut Vec<W>| {
                        let mut at = idx.first(from);
                        cx.c.lookups.set(cx.c.lookups.get() + 1);
                        while at != NONE {
                            cx.c.steps.set(cx.c.steps.get() + 1);
                            into.push(to_col[at as usize]);
                            at = idx.next(at);
                        }
                    };
                    frontier.clear();
                    follow(start, &mut frontier);
                    match dir {
                        Direction::Parent | Direction::Child => {
                            for e in frontier.clone() {
                                if !emit(e).1 {
                                    return false;
                                }
                            }
                            true
                        }
                        _ => {
                            let all_reached = *dir != Direction::NearestAncestor;
                            seen.clear();
                            while !frontier.is_empty() {
                                nextf.clear();
                                for e in std::mem::take(&mut frontier) {
                                    if !seen.insert(e) {
                                        continue;
                                    }
                                    if !all_reached {
                                        let mut db = UNB;
                                        db[*dom_id] = Some(e);
                                        if self.exists(*dom, &db, dom_filt) {
                                            return emit(e).1;
                                        }
                                    } else if !emit(e).1 {
                                        return false;
                                    }
                                    follow(e, &mut nextf);
                                }
                                std::mem::swap(&mut frontier, &mut nextf);
                            }
                            true
                        }
                    }
                })
            }
        }
    }
}

// ---------------------------------------------------------------- the driver

struct Finding {
    rule: u32,
    file: u32,
    start: u32,
    end: u32,
}

#[derive(Default, Clone)]
struct Phase {
    read: Duration,
    parse: Duration,
    fill: Duration,
    buckets: Duration,
    rules: Duration,
    index_ns: u64,
    anchor_rows: u64,
    steps: u64,
    lookups: u64,
    files: u64,
    est_calls: u64,
    node_calls: [u64; 8],
    shared_anchor_rows: u64,
    rule_runs: u64,
    rule_runs_started: u64,
    rejected: u64,
    rows: u64,
}

fn tap_plans(request: &str) -> Result<TappedPlans, String> {
    PLAN_TAP.set(std::sync::Mutex::new(None)).map_err(|_| "tap armed twice")?;
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if info.payload().downcast_ref::<TappedPlansStop>().is_none() {
            previous(info);
        }
    }));
    let args: Vec<BenchmarkArgument> = ["run-ryo", "--request-file", request, "python-target=3.10"]
        .iter()
        .map(|a| BenchmarkArgument((*a).to_string()))
        .collect();
    let mut sink = std::io::sink();
    let mut sink2 = std::io::sink();
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        super::composition::run(args, &mut sink, &mut sink2)
    }));
    let taken = PLAN_TAP.get().and_then(|t| t.lock().ok()?.take());
    match (taken, outcome) {
        (Some(t), _) => Ok(t),
        (None, Ok(Err(e))) => Err(format!("front end refused: {e}")),
        (None, _) => Err("front end ran without compiling".into()),
    }
}

pub(crate) fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let request_path = args.get(1).ok_or("usage: blocks <request.json> <findings.jsonl> [stats.json]")?.clone();
    let out_path = args.get(2).cloned();
    let stats_path = args.get(3).cloned();
    let mode = std::env::var("BLOCKS_MODE").unwrap_or_default();
    let threads: usize = std::env::var("BLOCKS_THREADS").ok().and_then(|t| t.parse().ok()).unwrap_or(4);
    let per_rule = std::env::var("BLOCKS_PER_RULE").is_ok();
    let budget: u64 = std::env::var("BLOCKS_BUDGET").ok().and_then(|t| t.parse().ok()).unwrap_or(u64::MAX / 4);
    let t0 = Instant::now();
    let cpu0 = cpu_time::ProcessTime::now();
    let req: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&request_path).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let root = std::path::PathBuf::from(req["root"].as_str().ok_or("root")?);
    let files: Vec<String> = req["files"].as_array().ok_or("files")?.iter().filter_map(|f| f.as_str().map(String::from)).collect();
    let accepted: FxHashSet<String> = req["rule_applicability"]
        .as_object()
        .map(|o| o.iter().filter(|(_, v)| v.as_str() == Some("accepted-syntax")).map(|(k, _)| k.clone()).collect())
        .unwrap_or_default();

    // Front end: the existing compile, per rule.
    let tapped = tap_plans(&request_path)?;
    let front_cpu = cpu0.elapsed();
    let front_wall = t0.elapsed();
    let t1 = Instant::now();
    let mut lowered = lower_all(&tapped, &accepted);
    let only: Option<FxHashSet<String>> = std::env::var("BLOCKS_ONLY").ok().map(|s| s.split(',').map(String::from).collect());
    if let Some(only) = &only {
        lowered.rules.retain(|r| only.contains(&r.id));
    }
    if mode == "floor" || mode == "parse" || mode == "fillsplit" {
        lowered.rules.clear();
    }
    let lower_wall = t1.elapsed();
    // Relations read: by the loaded rules, or (floor) by every per-file step of the whole pack.
    let mut wanted_set: FxHashSet<usize> = FxHashSet::default();
    if (mode == "floor" || mode == "fillsplit") {
        for plan in &tapped.plans {
            for step in &plan.steps {
                if step.scope != StepScope::PerFile {
                    continue;
                }
                let s = format!("{:?}", step.operation);
                for r in all() {
                    if s.contains(&format!("Relation({r:?})")) {
                        wanted_set.insert(rel_no(*r));
                    }
                }
            }
        }
    } else {
        for rule in &lowered.rules {
            for n in &rule.nodes {
                let srcs: Vec<Src> = match n {
                    N::Sel { src, .. } | N::Proj { src, .. } | N::Ext { src, .. } => vec![*src],
                    N::Join { l, r, .. } | N::Union { l, r } | N::Anti { l, r, .. } => vec![*l, *r],
                    N::Group { src, .. } | N::Between { src, .. } | N::Occur { src, .. } => vec![*src],
                    N::Closure { edges, .. } => vec![*edges],
                    N::Axis { from, edges, dom, .. } => vec![*from, Src::Rel(*edges), *dom],
                };
                for s in srcs {
                    if let Src::Rel(r) = s {
                        wanted_set.insert(r);
                    }
                }
            }
            if let Src::Rel(r) = rule.top {
                wanted_set.insert(r);
            }
        }
    }
    let mut wanted_nos: Vec<usize> = wanted_set.into_iter().collect();
    wanted_nos.sort_unstable();
    let wanted: Vec<Relation> = wanted_nos.iter().map(|n| all()[*n]).collect();
    // Floor buckets: an index for every (relation, column) a pack literal equality reads on a relation directly.
    let mut bucket_cols: Vec<(usize, usize)> = Vec::new();
    if (mode == "floor" || mode == "fillsplit") {
        for plan in &tapped.plans {
            for step in &plan.steps {
                if let Operation::Select { from: Input::Relation(r), comparisons, .. } = &step.operation {
                    for c in comparisons {
                        if let (Test::Equal, Operand::Column(col), Operand::Literal(_)) = (c.test, &c.left, &c.right) {
                            let key = (rel_no(*r), col.0 as usize);
                            if !bucket_cols.contains(&key) {
                                bucket_cols.push(key);
                            }
                        }
                    }
                }
            }
        }
    }

    let language = ryo_python::linked_with_settings(ryo_python::settings::PythonParserSettings::new(
        ryo_python::settings::PythonTargetVersion::Py310,
    ));
    let engine: Arc<dyn ParseEngine> = language.parse_engine();
    let next = AtomicUsize::new(0);
    let nrel = all().len();
    let nrules = lowered.rules.len();
    let results: Vec<(Vec<Finding>, Phase, Vec<(u64, u64, u64)>, Vec<Duration>, Vec<u64>, Vec<u64>)> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..threads)
            .map(|_| {
                let engine = Arc::clone(&engine);
                let lowered = &lowered;
                let files = &files;
                let root = &root;
                let wanted = &wanted;
                let wanted_nos = &wanted_nos;
                let next = &next;
                let mode = &mode;
                let bucket_cols = &bucket_cols;
                scope.spawn(move || {
                    let mut findings = Vec::new();
                    let mut ph = Phase::default();
                    let mut rule_stats = vec![(0u64, 0u64, 0u64); nrules];
                    let mut elements = Elements::default();
                    let mut symbols = Symbols::default();
                    let mut facts: Vec<Vec<FactColumn>> =
                        all().iter().map(|r| (0..kinds(*r).len()).map(|_| FactColumn(Vec::new())).collect()).collect();
                    let mut seen_rows: FxHashSet<Vec<W>> = FxHashSet::default();
                    let mut fill_split = vec![Duration::ZERO; wanted_nos.len() + 1];
                    let est_memo: RefCell<FxHashMap<(usize, usize, u32), u64>> = RefCell::new(FxHashMap::default());
                    let mut rel_rows = vec![0u64; wanted_nos.len()];
                    let mut over_budget = vec![0u64; nrules];
                    loop {
                        let at = next.fetch_add(1, Ordering::Relaxed);
                        if at >= files.len() {
                            break;
                        }
                        let t = Instant::now();
                        let Ok(bytes) = std::fs::read(root.join(&files[at])) else { continue };
                        ph.read += t.elapsed();
                        let t = Instant::now();
                        clear_symbols(&mut symbols);
                        let request = ParseRequest {
                            path: SourcePath(&files[at]),
                            file: FileId(at as u32),
                            bytes: ByteSlice(&bytes),
                            notes: NoteRequest::Unwanted,
                            fields: FieldRequest::Wanted,
                        };
                        if mode == "parse" {
                            if engine.baseline_file(&request).is_err() {
                                continue;
                            }
                            ph.parse += t.elapsed();
                            ph.files += 1;
                            continue;
                        }
                        if engine.build_file(request, &mut elements, &mut symbols, wanted).is_err() {
                            continue;
                        }
                        ph.parse += t.elapsed();
                        let t = Instant::now();
                        for no in wanted_nos.iter() {
                            for c in &mut facts[*no] {
                                c.0.clear();
                            }
                        }
                        {
                            let mut cols: ColumnsMut<'_> = facts
                                .iter_mut()
                                .enumerate()
                                .filter(|(no, _)| wanted_nos.binary_search(no).is_ok())
                                .map(|(no, c)| (all()[no], c.as_mut_slice()))
                                .collect();
                            engine.walk_file(&mut elements, &mut symbols, ByteSlice(&bytes), FileId(at as u32), wanted, &mut cols);
                        }
                        ph.fill += t.elapsed();
                        if mode == "fillsplit" {
                            // Each relation's reader alone (the file model and analyses it needs included), then none at all.
                            for (i, no) in wanted_nos.iter().enumerate() {
                                for c in &mut facts[*no] {
                                    c.0.clear();
                                }
                                let one = [all()[*no]];
                                let t = Instant::now();
                                let mut cols: ColumnsMut<'_> = facts
                                    .iter_mut()
                                    .enumerate()
                                    .filter(|(n, _)| n == no)
                                    .map(|(n, c)| (all()[n], c.as_mut_slice()))
                                    .collect();
                                engine.walk_file(&mut elements, &mut symbols, ByteSlice(&bytes), FileId(at as u32), &one, &mut cols);
                                fill_split[i] += t.elapsed();
                            }
                            let t = Instant::now();
                            let mut cols = ColumnsMut::default();
                            engine.walk_file(&mut elements, &mut symbols, ByteSlice(&bytes), FileId(at as u32), &[], &mut cols);
                            fill_split[wanted_nos.len()] += t.elapsed();
                        }
                        ph.files += 1;
                        let rejected = syntax_acceptance(&elements) == SyntaxAcceptance::Rejected;
                        if rejected {
                            ph.rejected += 1;
                        }
                        let counters = Counters::default();
                        est_memo.borrow_mut().clear();
                        let cx = Ctx {
                            facts: &facts,
                            file: at as u64,
                            symbols: &symbols,
                            source: &bytes,
                            lits: &lowered.lits,
                            regexes: &lowered.regexes,
                            globs: &lowered.globs,
                            path: &files[at],
                            group_memo: RefCell::new(FxHashMap::default()),
                            lit_words: RefCell::new(vec![u64::MAX; lowered.lits.len()]),
                            idx: RefCell::new(FxHashMap::default()),
                            idx_fast: RefCell::new(vec![None; all().len() * MAXW]),
                            bucket_n: RefCell::new(FxHashMap::default()),
                            est_memo: &est_memo,
                            c: &counters,
                        };
                        if !bucket_cols.is_empty() {
                            let t = Instant::now();
                            for (r, c) in bucket_cols.iter() {
                                if cx.rows(*r) > 0 {
                                    drop(cx.index(*r, *c));
                                }
                            }
                            ph.buckets += t.elapsed();
                        }
                        let t = Instant::now();
                        for (ri, rule) in lowered.rules.iter().enumerate() {
                            if rule.accepted_only && rejected {
                                continue;
                            }
                            let tr = if per_rule { Some(Instant::now()) } else { None };
                            let a0 = counters.anchor_rows.get();
                            let s0 = counters.steps.get();
                            counters.budget_end.set(counters.steps.get() + budget);
                            counters.over.set(false);
                            let ex = Exec { ri, rule, cx: &cx };
                            let span_col = rule.top_kinds.iter().position(|k| *k == CellKind::Span);
                            seen_rows.clear();
                            ex.each(rule.top, &UNB, &[], &mut |row| {
                                if seen_rows.insert(row.to_vec()) {
                                    if let Some(c) = span_col {
                                        findings.push(Finding {
                                            rule: ri as u32,
                                            file: at as u32,
                                            start: (row[c] >> 32) as u32,
                                            end: row[c] as u32,
                                        });
                                    }
                                }
                                true
                            });
                            if counters.over.get() {
                                over_budget[ri] += 1;
                            }
                            ph.rule_runs += 1;
                            if counters.anchor_rows.get() > a0 {
                                ph.rule_runs_started += 1;
                            }
                            let st = &mut rule_stats[ri];
                            st.0 += counters.anchor_rows.get() - a0;
                            st.1 += counters.steps.get() - s0;
                            if let Some(tr) = tr {
                                st.2 += tr.elapsed().as_nanos() as u64;
                            }
                        }
                        ph.rules += t.elapsed();
                        ph.index_ns += counters.index_ns.get();
                        ph.anchor_rows += counters.anchor_rows.get();
                        ph.steps += counters.steps.get();
                        ph.lookups += counters.lookups.get();
                        ph.est_calls += counters.est_calls.get();
                        ph.shared_anchor_rows += counters.shared_anchor_rows.get();
                        for i in 0..8 {
                            ph.node_calls[i] += counters.node_calls[i].get();
                        }
                        for (i, no) in wanted_nos.iter().enumerate() {
                            let n = facts[*no].first().map_or(0, |c| c.0.len()) as u64;
                            ph.rows += n;
                            rel_rows[i] += n;
                        }
                    }
                    (findings, ph, rule_stats, fill_split, rel_rows, over_budget)
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap_or_else(|_| (Vec::new(), Phase::default(), Vec::new(), Vec::new(), Vec::new(), Vec::new()))).collect()
    });
    let mut all_findings: Vec<(String, String, u32, u32)> = Vec::new();
    let mut tot = Phase::default();
    let mut rule_tot = vec![(0u64, 0u64, 0u64); nrules];
    let mut split_tot = vec![0f64; wanted_nos.len() + 1];
    let mut rows_tot = vec![0u64; wanted_nos.len()];
    let mut over_tot = vec![0u64; nrules];
    for (_, _, _, _, _, ob) in &results {
        for (i, n) in ob.iter().enumerate() {
            over_tot[i] += n;
        }
    }
    for (_, _, _, _, rr, _) in &results {
        for (i, n) in rr.iter().enumerate() {
            rows_tot[i] += n;
        }
    }
    for (_, _, _, sp, _, _) in &results {
        for (i, d) in sp.iter().enumerate() {
            split_tot[i] += d.as_secs_f64();
        }
    }
    for (fs, ph, rs, _, _, _) in &results {
        for f in fs {
            all_findings.push((lowered.rules[f.rule as usize].id.clone(), files[f.file as usize].clone(), f.start, f.end));
        }
        tot.read += ph.read;
        tot.parse += ph.parse;
        tot.fill += ph.fill;
        tot.buckets += ph.buckets;
        tot.rules += ph.rules;
        tot.index_ns += ph.index_ns;
        tot.anchor_rows += ph.anchor_rows;
        tot.steps += ph.steps;
        tot.lookups += ph.lookups;
        tot.est_calls += ph.est_calls;
        tot.shared_anchor_rows += ph.shared_anchor_rows;
        tot.rule_runs += ph.rule_runs;
        tot.rule_runs_started += ph.rule_runs_started;
        for i in 0..8 {
            tot.node_calls[i] += ph.node_calls[i];
        }
        tot.files += ph.files;
        tot.rejected += ph.rejected;
        tot.rows += ph.rows;
        for (i, r) in rs.iter().enumerate() {
            rule_tot[i].0 += r.0;
            rule_tot[i].1 += r.1;
            rule_tot[i].2 += r.2;
        }
    }
    all_findings.sort();
    if let Some(p) = &out_path {
        let mut s = String::new();
        for (r, f, a, b) in &all_findings {
            s.push_str(&format!("{{\"rule\":\"{r}\",\"file\":\"{f}\",\"start\":{a},\"end\":{b}}}\n"));
        }
        std::fs::write(p, s).map_err(|e| e.to_string())?;
    }
    if let Some(p) = &stats_path {
        let per: Vec<serde_json::Value> = lowered
            .rules
            .iter()
            .zip(&rule_tot)
            .zip(&over_tot)
            .map(|((r, t), o)| serde_json::json!({"rule": r.id, "core": r.core, "nodes": r.nodes.len(), "anchor_rows": t.0, "steps": t.1, "ns": t.2, "files_over_budget": o}))
            .collect();
        let mut blocked_by: FxHashMap<String, u32> = FxHashMap::default();
        for (_, why) in &lowered.blocked {
            *blocked_by.entry(why.clone()).or_default() += 1;
        }
        let stats = serde_json::json!({
            "mode": mode, "threads": threads, "files": tot.files, "rejected_files": tot.rejected,
            "front_end_cpu_s": front_cpu.as_secs_f64(), "front_end_wall_s": front_wall.as_secs_f64(),
            "lower_s": lower_wall.as_secs_f64(),
            "wanted_relations": wanted.iter().map(|r| format!("{r:?}")).collect::<Vec<_>>(),
            "fill_alone_s": wanted.iter().map(|r| format!("{r:?}")).chain(std::iter::once("(none)".to_string())).zip(&split_tot).filter(|(_, v)| **v > 0.0).map(|(k, v)| serde_json::json!([k, v])).collect::<Vec<_>>(),
            "fact_rows": tot.rows,
            "rows_by_relation": wanted.iter().zip(&rows_tot).map(|(r, n)| serde_json::json!([format!("{r:?}"), n])).collect::<Vec<_>>(),
            "thread_s": {"read": tot.read.as_secs_f64(), "parse": tot.parse.as_secs_f64(), "fill": tot.fill.as_secs_f64(),
                         "buckets": tot.buckets.as_secs_f64(), "rules_incl_index": tot.rules.as_secs_f64(), "index": tot.index_ns as f64 / 1e9},
            "anchor_rows": tot.anchor_rows, "est_calls": tot.est_calls,
            "shared_anchor_rows": tot.shared_anchor_rows, "rule_file_runs": tot.rule_runs, "rule_file_runs_with_candidates": tot.rule_runs_started,
            "node_calls": {"sel": tot.node_calls[0], "proj": tot.node_calls[1], "ext": tot.node_calls[2], "join": tot.node_calls[3],
                           "union": tot.node_calls[4], "group": tot.node_calls[5], "anti": tot.node_calls[6], "axis": tot.node_calls[7]}, "steps": tot.steps, "lookups": tot.lookups,
            "rules_loaded": lowered.rules.len(), "rules_core_only": lowered.rules.iter().filter(|r| r.core).count(),
            "blocked": lowered.blocked.len(), "blocked_by": blocked_by,
            "blocked_rules": lowered.blocked.iter().map(|(r, w)| serde_json::json!([r, w])).collect::<Vec<_>>(),
            "findings": all_findings.len(),
            "total_cpu_s": cpu0.elapsed().as_secs_f64(), "total_wall_s": t0.elapsed().as_secs_f64(),
            "per_rule": per,
        });
        std::fs::write(p, serde_json::to_string_pretty(&stats).unwrap_or_default()).map_err(|e| e.to_string())?;
    }
    Ok(())
}
