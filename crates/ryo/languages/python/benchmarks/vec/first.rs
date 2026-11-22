//! THROWAWAY (ryo-fmx3.1, redo-lowering builder 4): the first half's spec prepared in the process, from the pack's rules as
//! the query language's one parser reads them (the same `PackPrograms` the second half's lowering reads), with no census and
//! no file from an earlier run.
//!
//! Two analyses, the ones `vec/scripts/anchors.py` and `vec/scripts/funnel4.py` made offline from the rule text:
//! - the skip (anchors.py): what each relation's being non-empty requires, as an AND/OR of scans of the tree relations; a
//!   file that has none of the kinds (or fields) of one clause of that requirement cannot give the rule a row;
//! - the prefix plans (funnel4.py): every relation of the query followed as a disjunction of tree patterns over node
//!   variables, each disjunct a plan over the node table entered at a bucket of one of its variables.
//! What the census chose there is chosen without it:
//! - a text-only scan (a test on `text` alone) gives no skip clause (anchors.py: "the kinds the census saw carrying that
//!   text"; a text the census had not counted gave an empty clause and dropped the rule);
//! - a disjunct keeps every bucket it can be entered at (each variable's kinds, fields, or the required words of a name
//!   token), minus those another one's keys contain; the run enters each file at the one of them with the fewest nodes in
//!   that file (funnel4.py chose one by Django's counts). Every alternative covers every node that can match.

use super::funnel::{Bucket, SpecCheck, SpecFact, SpecOp, SpecPlan, SpecRule};
use ryo_engine::domain::plan_lowering::types::{PackPrograms, RuleProgram};
use ryo_values::domain::program::types::{FragmentProgram, ImportDeclaration, LetStep, ParsedModules};
use ryo_values::domain::query_syntax::types::{
    Alias, Argument, ArgumentValue, Call, Columns, Condition, Expression, Joining, Renaming, Select, SelectedValue, SourceReference,
    WrittenOperand, WrittenTest,
};
use ryo_values::domain::query_tokens::types::{QuerySourceId, StepName};
use ryo_values::domain::finish_plans::constants::{OP_CLOSURE, OP_FOLLOW, OP_ORDERED_DFS, OP_ROWS};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::rc::Rc;

/// The tree relations: a scan of one gives a node with its kind and text.
const SYNTAX: [&str; 2] = ["semantic-syntax", "syntax-tree"];
/// The parent-child relations: a scan gives a parent, a child and the child's field.
const CHILD: [&str; 2] = ["semantic-child", "syntax-child"];
/// What the first half needs to know of the language, all given by the provider: the relations written per node as fact
/// columns, the columns whose text is spelled in the source as a name (and how), what a name is, and the kind of the
/// tokens that spell names.
pub(super) struct Language {
    pub(super) facts: Vec<String>,
    pub(super) spelled: Vec<(String, String, Option<char>)>,
    pub(super) is_identifier: fn(&str) -> bool,
    pub(super) word_kind: String,
}

thread_local! {
    /// The language of the pack being prepared (`prepare` installs it).
    static LANGUAGE: std::cell::RefCell<Option<Rc<Language>>> = const { std::cell::RefCell::new(None) };
}

fn language() -> Rc<Language> {
    LANGUAGE.with(|l| l.borrow().clone()).expect("prepare installs the language")
}

/// Operations whose rows are reached from their input (traversals, a literal table), not the input's rows: their output
/// keeps no tree condition of an input. The plan language's own operations (`finish_plans` constants), not a pack's.
const NEW_ROWS: [&str; 4] = [OP_CLOSURE, OP_FOLLOW, OP_ORDERED_DFS, OP_ROWS];
/// A select with more combinations of its sources' disjuncts than this is not followed: its relation is opaque (a
/// superset, so never a lost finding). funnel4.py's own bound on the symbolic product (not a corpus figure); every time it
/// is reached is counted and printed by `prepare`.
const CAP: usize = 512;
/// Registers of a negation's own variables are numbered from here, then renumbered after the positive ones; a plan with
/// this many positive registers is refused loudly (a panic), never folded into a negation's.
const SUB_REG: usize = 1000;

fn ident(w: &str) -> bool {
    (language().is_identifier)(w)
}

/// The word a derived column's literal is spelled as in the source, when the provider declares the column spelled.
fn word_of(rel: &str, col: &str, value: &str) -> Option<String> {
    let lang = language();
    let (_, _, how) = lang.spelled.iter().find(|(r, c, _)| r == rel && c == col)?;
    Some(match how {
        Some(sep) => value.rsplit(*sep).next().unwrap_or(value).to_owned(),
        None => value.to_owned(),
    })
}

type Set = BTreeSet<String>;

// ---------------------------------------------------------------- conditions, from the typed form
#[derive(Clone, Debug)]
enum Opd {
    /// A column: its name and alias.
    Col(String, Option<String>),
    Lit(String),
    /// A column inside a function's arguments: kept only for its alias.
    Applied(Vec<Opd>),
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cmp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

impl Cmp {
    fn name(self) -> &'static str {
        match self {
            Cmp::Eq => "eq",
            Cmp::Ne => "ne",
            Cmp::Lt => "<",
            Cmp::Le => "<=",
            Cmp::Gt => ">",
            Cmp::Ge => ">=",
        }
    }
}

#[derive(Clone, Debug)]
enum Cond {
    Cmp(Cmp, Opd, Opd),
    Matches(Opd, Opd),
    /// Some branch holds; a branch is a conjunction.
    Or(Vec<Vec<Cond>>),
    /// A test this analysis does not read (dropped: a superset), with its operands for their aliases.
    Other(Vec<Opd>),
}

fn opd(o: &WrittenOperand) -> Opd {
    match o {
        WrittenOperand::Column(c) => Opd::Col(c.column.0.to_string(), alias_of(&c.alias)),
        WrittenOperand::Text(t) => Opd::Lit(t.0.to_string()),
        WrittenOperand::Number(n) => Opd::Lit(n.0.to_string()),
        WrittenOperand::Function(f) => Opd::Applied(f.arguments.iter().map(opd).collect()),
        WrittenOperand::Parameter(_) => Opd::Other,
    }
}

fn alias_of(a: &Alias) -> Option<String> {
    match a {
        Alias::Unwritten => None,
        Alias::Written(n) => Some(n.0.to_string()),
    }
}

fn cond(c: &Condition) -> Cond {
    match c {
        Condition::Any(branches) => Cond::Or(branches.iter().map(|b| b.iter().map(cond).collect()).collect()),
        Condition::Test(f) => {
            let (l, r) = (opd(&f.left), opd(&f.right));
            let cmp = match f.test {
                WrittenTest::Equal => Cmp::Eq,
                WrittenTest::NotEqual => Cmp::Ne,
                WrittenTest::LessThan => Cmp::Lt,
                WrittenTest::LessThanOrEqual => Cmp::Le,
                WrittenTest::GreaterThan => Cmp::Gt,
                WrittenTest::GreaterThanOrEqual => Cmp::Ge,
                WrittenTest::Matches => return Cond::Matches(l, r),
                _ => return Cond::Other(vec![l, r]),
            };
            Cond::Cmp(cmp, l, r)
        }
    }
}

fn conds(cs: &[Condition]) -> Vec<Cond> {
    cs.iter().map(cond).collect()
}

/// The aliases a condition's columns are written with (`None`: unaliased).
fn aliases(c: &Cond, out: &mut BTreeSet<Option<String>>) {
    fn of(o: &Opd, out: &mut BTreeSet<Option<String>>) {
        match o {
            Opd::Col(_, a) => {
                out.insert(a.clone());
            }
            Opd::Applied(xs) => xs.iter().for_each(|x| of(x, out)),
            Opd::Lit(_) | Opd::Other => {}
        }
    }
    match c {
        Cond::Cmp(_, l, r) | Cond::Matches(l, r) => {
            of(l, out);
            of(r, out);
        }
        Cond::Or(bs) => bs.iter().flatten().for_each(|x| aliases(x, out)),
        Cond::Other(xs) => xs.iter().for_each(|x| of(x, out)),
    }
}

/// The conjuncts that name only columns of `alias`.
fn split_alias(w: &[Cond], alias: Option<&str>) -> Vec<Cond> {
    w.iter()
        .filter(|c| {
            let mut s = BTreeSet::new();
            aliases(c, &mut s);
            s.iter().all(|a| a.as_deref() == alias)
        })
        .cloned()
        .collect()
}

// ---------------------------------------------------------------- the skip (anchors.py)
#[derive(Debug)]
enum Req {
    True,
    Scan(String, Vec<Cond>),
    And(Vec<R>),
    Or(Vec<R>),
}

/// A requirement node, shared: a step's requirement is read by every step after it (a DAG, never copied).
type R = Rc<Req>;

fn and(xs: Vec<R>) -> R {
    let mut xs: Vec<R> = xs.into_iter().filter(|x| !matches!(**x, Req::True)).collect();
    match xs.len() {
        0 => Rc::new(Req::True),
        1 => xs.remove(0),
        _ => Rc::new(Req::And(xs)),
    }
}

fn or(mut xs: Vec<R>) -> R {
    if xs.is_empty() || xs.iter().any(|x| matches!(**x, Req::True)) {
        return Rc::new(Req::True);
    }
    if xs.len() == 1 { xs.remove(0) } else { Rc::new(Req::Or(xs)) }
}

/// A relation: a pure scan (its conditions can still grow) or a formula.
#[derive(Clone)]
enum SRel {
    Scan(String, Vec<Cond>),
    Formula(R),
}

impl SRel {
    fn req(&self) -> R {
        match self {
            SRel::Scan(b, c) => Rc::new(Req::Scan(b.clone(), c.clone())),
            SRel::Formula(f) => f.clone(),
        }
    }
}

fn truth() -> SRel {
    SRel::Formula(Rc::new(Req::True))
}

/// The values a condition tests column `col` against, when it tests only equality with literals.
fn kset(a: &Cond, col: &str) -> Option<Set> {
    match a {
        Cond::Cmp(Cmp::Eq, Opd::Col(n, _), Opd::Lit(v)) if n == col => Some(BTreeSet::from([v.clone()])),
        Cond::Or(bs) => {
            let mut out = Set::new();
            for b in bs {
                out.extend(kset_all(b, col)?);
            }
            Some(out)
        }
        _ => None,
    }
}

fn kset_all(b: &[Cond], col: &str) -> Option<Set> {
    let mut acc: Option<Set> = None;
    for c in b {
        if let Some(k) = kset(c, col) {
            acc = Some(match acc {
                Some(a) => a.intersection(&k).cloned().collect(),
                None => k,
            });
        }
    }
    acc
}

enum Leaf {
    Kind(Set),
    Field(Set),
    /// A text-only scan, or any other relation: no clause.
    None,
}

fn leaf_info(base: &str, cs: &[Cond]) -> Leaf {
    if SYNTAX.contains(&base) {
        let mut ks: Option<Set> = None;
        for c in cs {
            if let Some(k) = kset(c, "kind") {
                ks = Some(match ks {
                    Some(a) => a.intersection(&k).cloned().collect(),
                    None => k,
                });
            }
        }
        return ks.map_or(Leaf::None, Leaf::Kind);
    }
    if CHILD.contains(&base) {
        let mut fs: Option<Set> = None;
        for c in cs {
            if let Some(k) = kset(c, "field") {
                fs = Some(match fs {
                    Some(a) => a.intersection(&k).cloned().collect(),
                    None => k,
                });
            }
        }
        return fs.map_or(Leaf::None, Leaf::Field);
    }
    Leaf::None
}

/// CNF over `k:<kind>` / `f:<field>` names: a file lacking every name of one clause cannot give the rule a row. A text-only
/// scan gives no clause (no census: which kinds carry a text is not known before the run). Memoised per shared node.
fn clauses(f: &R, memo: &mut HashMap<*const Req, Rc<Vec<Set>>>) -> Rc<Vec<Set>> {
    if let Some(m) = memo.get(&Rc::as_ptr(f)) {
        return m.clone();
    }
    let out = match &**f {
        Req::Scan(b, cs) => match leaf_info(b, cs) {
            Leaf::Kind(ks) => vec![ks.iter().map(|k| format!("k:{k}")).collect()],
            Leaf::Field(fs) => vec![fs.iter().map(|k| format!("f:{k}")).collect()],
            Leaf::None => Vec::new(),
        },
        Req::And(xs) => {
            let mut out: Vec<Set> = Vec::new();
            for x in xs {
                for c in clauses(x, memo).iter() {
                    if !out.contains(c) {
                        out.push(c.clone());
                    }
                }
            }
            out.sort_by(|a, b| a.len().cmp(&b.len()).then_with(|| a.cmp(b)));
            out
        }
        Req::Or(xs) => {
            let mut u = Set::new();
            let mut all = true;
            for x in xs {
                let cs = clauses(x, memo);
                match cs.iter().min_by(|a, b| a.len().cmp(&b.len()).then_with(|| a.cmp(b))) {
                    Some(m) => u.extend(m.iter().cloned()),
                    None => {
                        all = false;
                        break;
                    }
                }
            }
            if all { vec![u] } else { Vec::new() }
        }
        Req::True => Vec::new(),
    };
    let out = Rc::new(out);
    memo.insert(Rc::as_ptr(f), out.clone());
    out
}

struct SkipAnalyzer<'p> {
    frags: &'p ParsedModules,
}

type SEnv = HashMap<String, SRel>;
type Imports = HashMap<String, String>;

fn imports_of(list: &[ImportDeclaration]) -> Imports {
    list.iter().map(|i| (i.alias.0.to_string(), i.module.0.rsplit('/').next().unwrap_or(&i.module.0).to_owned())).collect()
}

fn arg<'a>(args: &'a [Argument], name: &str) -> Option<&'a ArgumentValue> {
    args.iter().find(|a| &*a.name.0 == name).map(|a| &a.value)
}

impl SkipAnalyzer<'_> {
    fn resolve_step(&self, name: &str, env: &SEnv) -> SRel {
        env.get(name).cloned().unwrap_or_else(truth)
    }

    fn rel(&self, v: Option<&ArgumentValue>, env: &SEnv) -> Option<SRel> {
        match v? {
            ArgumentValue::Text(t) => Some(SRel::Scan(t.0.to_string(), Vec::new())),
            ArgumentValue::Step(s) => Some(self.resolve_step(&s.0, env)),
            ArgumentValue::Word(w) => Some(self.resolve_step(&w.0, env)),
            _ => None,
        }
    }

    fn call(&self, c: &Call, env: &SEnv, imports: &Imports) -> SRel {
        let name = &*c.name.0;
        if let Some(module) = imports.get(name) {
            let params: SEnv = c.arguments.iter().filter_map(|a| Some((a.name.0.to_string(), self.rel(Some(&a.value), env)?))).collect();
            return self.fragment(module, params);
        }
        if name == "union" {
            let reqs = ["left", "right"].iter().filter_map(|k| self.rel(arg(&c.arguments, k), env)).map(|r| r.req()).collect();
            return SRel::Formula(or(reqs));
        }
        if name == "antijoin" {
            return SRel::Formula(self.rel(arg(&c.arguments, "left"), env).map_or_else(|| Rc::new(Req::True), |r| r.req()));
        }
        // every step the call reads is required non-empty (a quoted name gives no clause unless it is a tree relation)
        let reqs = c.arguments.iter().filter(|a| matches!(a.value, ArgumentValue::Step(_) | ArgumentValue::Word(_))).filter_map(|a| self.rel(Some(&a.value), env)).map(|r| r.req()).collect();
        SRel::Formula(and(reqs))
    }

    fn source(&self, r: &SourceReference, env: &SEnv) -> SRel {
        match r {
            SourceReference::Relation(n) => SRel::Scan(n.0.to_string(), Vec::new()),
            SourceReference::Step(s) => self.resolve_step(&s.0, env),
        }
    }

    fn select(&self, s: &Select, env: &SEnv) -> SRel {
        let w = conds(&s.filters);
        let mut sources = vec![&s.from];
        if let Joining::Joined(j) = &s.join {
            sources.push(&j.source);
        }
        if sources.len() == 1 {
            let alias = alias_of(&s.from.alias);
            let base = self.source(&s.from.reference, env);
            let mut cs = split_alias(&w, alias.as_deref());
            if alias.is_some() {
                cs.extend(split_alias(&w, None));
            }
            return match base {
                SRel::Scan(b, mut c) => {
                    c.extend(cs);
                    SRel::Scan(b, c)
                }
                f @ SRel::Formula(_) => f,
            };
        }
        let parts = sources
            .iter()
            .map(|src| {
                let alias = alias_of(&src.alias);
                match self.source(&src.reference, env) {
                    SRel::Scan(b, mut c) => {
                        c.extend(split_alias(&w, alias.as_deref()));
                        Rc::new(Req::Scan(b, c))
                    }
                    SRel::Formula(f) => f,
                }
            })
            .collect();
        SRel::Formula(and(parts))
    }

    fn steps(&self, steps: &[LetStep], mut env: SEnv, imports: &Imports) -> SEnv {
        for st in steps {
            let r = match &st.expression {
                Expression::Select(s) => self.select(s, &env),
                Expression::Call(c) => self.call(c, &env, imports),
            };
            env.insert(st.name.0.to_string(), r);
        }
        env
    }

    fn fragment(&self, module: &str, params: SEnv) -> SRel {
        let Some(f) = self.frags.0.get(&QuerySourceId(module.into())) else { return truth() };
        let env = self.steps(&f.steps, params, &imports_of(&f.imports));
        self.resolve_step(&f.result.step.0, &env)
    }
}

// ---------------------------------------------------------------- the prefix plans (funnel4.py)
type V = u32;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Attr {
    Kind,
    Text,
    Field,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Atom {
    Kind(V, ASet),
    NotKind(V, ASet),
    Text(V, ASet),
    Field(V, ASet),
    Re(V, Str),
    Parent(V, V),
    Eq(V, V),
    Words(ASet),
    Derived(Str),
    Dlit(Str, Str),
    Fact(V, Str),
    Fcond(V, Str, &'static str, Rc<[String]>),
    Fkey(V, Str, V),
    Inexact,
}

/// Shared payloads: an atom is copied into every disjunct built from it (the generators shared their tuples).
type ASet = Rc<Set>;
type Str = Rc<str>;

impl Atom {
    /// Names no tree variable: a required-word clause, a derived relation, a literal test on one.
    fn nonvar(&self) -> bool {
        matches!(self, Atom::Words(_) | Atom::Derived(_) | Atom::Dlit(..))
    }
    fn attr(a: Attr, v: V, s: ASet) -> Atom {
        match a {
            Attr::Kind => Atom::Kind(v, s),
            Attr::Text => Atom::Text(v, s),
            Attr::Field => Atom::Field(v, s),
        }
    }
    /// The (attribute, variable, values) of a kind, text or field test.
    fn attr_test(&self) -> Option<(Attr, V, &ASet)> {
        match self {
            Atom::Kind(v, s) => Some((Attr::Kind, *v, s)),
            Atom::Text(v, s) => Some((Attr::Text, *v, s)),
            Atom::Field(v, s) => Some((Attr::Field, *v, s)),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
enum CRef {
    /// A node variable's id.
    V(V),
    A(V, Attr),
    File,
    /// A derived relation's column; with a fact variable when the relation is a fact.
    D(Str, Str, Option<V>),
    /// A literal table's column: its values.
    R(ASet),
}

#[derive(Clone, Debug)]
enum Cols {
    Map(Vec<(Str, Option<CRef>)>),
    /// Every name is a column of the derived relation (with its fact variable when it is a fact).
    Derived(Str, Option<V>),
}

impl Cols {
    fn empty() -> Self {
        Cols::Map(Vec::new())
    }
    fn contains(&self, name: &str) -> bool {
        match self {
            Cols::Map(m) => m.iter().any(|(n, _)| &**n == name),
            Cols::Derived(..) => true,
        }
    }
    fn get(&self, name: &str) -> Option<CRef> {
        match self {
            Cols::Map(m) => m.iter().find(|(n, _)| &**n == name).and_then(|(_, c)| c.clone()),
            Cols::Derived(rel, f) => Some(CRef::D(rel.clone(), name.into(), *f)),
        }
    }
    fn derived(&self) -> bool {
        matches!(self, Cols::Derived(..))
    }
}

fn put(m: &mut Vec<(Str, Option<CRef>)>, name: &str, c: Option<CRef>) {
    match m.iter_mut().find(|(n, _)| &**n == name) {
        Some(slot) => slot.1 = c,
        None => m.push((name.into(), c)),
    }
}

/// A disjunct's atoms as shared segments: a combination of disjuncts shares theirs (no copy until a renaming).
#[derive(Clone, Debug, Default)]
struct Atoms(Vec<Rc<[Atom]>>);

impl Atoms {
    fn of(v: Vec<Atom>) -> Self {
        if v.is_empty() { Atoms(Vec::new()) } else { Atoms(vec![v.into()]) }
    }
    fn iter(&self) -> impl Iterator<Item = &Atom> {
        self.0.iter().flat_map(|s| s.iter())
    }
    fn len(&self) -> usize {
        self.0.iter().map(|s| s.len()).sum()
    }
    fn flat(&self) -> Vec<Atom> {
        self.iter().cloned().collect()
    }
    fn extend_from(&mut self, o: &Atoms) {
        self.0.extend(o.0.iter().cloned());
    }
    fn push_vec(&mut self, v: Vec<Atom>) {
        if !v.is_empty() {
            self.0.push(v.into());
        }
    }
}

#[derive(Clone, Debug)]
struct Disj {
    atoms: Atoms,
    /// Each a NOT over the OR of its disjuncts; shared, as the generators' lists were.
    negs: Vec<Rc<Vec<Disj>>>,
    exact: bool,
    cols: Cols,
}

#[derive(Clone, Debug)]
struct Rel {
    disjs: Vec<Disj>,
    opaque: bool,
}

fn opaque() -> Rel {
    Rel { disjs: vec![Disj { atoms: Atoms::default(), negs: Vec::new(), exact: false, cols: Cols::empty() }], opaque: true }
}

/// A program's steps by name, evaluated when first read (a step the result does not read is never evaluated), and its
/// imports.
struct Env<'p> {
    map: HashMap<String, Entry<'p>>,
    imports: Imports,
}

/// A step's relation (once evaluated) and whether a use has taken it: the first use takes the step's own variables (no other
/// instance of them exists yet); every later use a fresh copy, as the generators' renaming at every use gave.
struct Entry<'p> {
    rel: std::cell::OnceCell<Rc<Rel>>,
    expr: Option<&'p Expression>,
    busy: std::cell::Cell<bool>,
    used: std::cell::Cell<bool>,
}

impl<'p> Entry<'p> {
    fn ready(rel: Rc<Rel>) -> Self {
        Entry { rel: std::cell::OnceCell::from(rel), expr: None, busy: std::cell::Cell::new(false), used: std::cell::Cell::new(false) }
    }
    fn lazy(expr: &'p Expression) -> Self {
        Entry { rel: std::cell::OnceCell::new(), expr: Some(expr), busy: std::cell::Cell::new(false), used: std::cell::Cell::new(false) }
    }
}

/// A fragment call's arguments as a memo key: a relation argument by the identity of the step it names (pinned in the
/// entry, so the identity is not reused), a base relation by its name.
#[derive(Clone, PartialEq, Eq, Hash)]
enum MemoArg {
    Base(String),
    Step(usize),
    Opaque,
}

type MemoKey = (String, Vec<(String, MemoArg)>);
/// A select's sources by alias, their columns borrowed.
type Scope<'a> = [(Option<&'a str>, &'a Cols)];

struct Analyzer<'p> {
    frags: &'p ParsedModules,
    counter: V,
    /// Fragment results by call: those with no step argument for the whole pack, the others for the rule (their pins).
    pure: HashMap<MemoKey, Rc<Rel>>,
    /// Renaming: per variable the disjunct (`stamp`) it was renamed in and its new name.
    slots: Vec<(u32, V)>,
    stamp: u32,
    local: HashMap<MemoKey, (Vec<Rc<Rel>>, Rc<Rel>)>,
}

impl<'p> Analyzer<'p> {
    fn fresh(&mut self) -> V {
        self.counter += 1;
        self.counter
    }

    /// The fresh name of `v` in the disjunct being renamed (`stamp`).
    fn rv(&mut self, v: V) -> V {
        let i = v as usize;
        if i >= self.slots.len() {
            self.slots.resize(i + 1, (0, 0));
        }
        if self.slots[i].0 == self.stamp {
            return self.slots[i].1;
        }
        let x = self.fresh();
        self.slots[i] = (self.stamp, x);
        x
    }

    fn rename(&mut self, d: &Disj) -> Disj {
        let mut atoms = Vec::with_capacity(d.atoms.len());
        for a in d.atoms.iter() {
            atoms.push(match a {
                Atom::Kind(v, s) => Atom::Kind(self.rv(*v), s.clone()),
                Atom::NotKind(v, s) => Atom::NotKind(self.rv(*v), s.clone()),
                Atom::Text(v, s) => Atom::Text(self.rv(*v), s.clone()),
                Atom::Field(v, s) => Atom::Field(self.rv(*v), s.clone()),
                Atom::Re(v, s) => Atom::Re(self.rv(*v), s.clone()),
                Atom::Parent(c, p) => {
                    let c = self.rv(*c);
                    Atom::Parent(c, self.rv(*p))
                }
                Atom::Eq(x, y) => {
                    let x = self.rv(*x);
                    Atom::Eq(x, self.rv(*y))
                }
                Atom::Fact(f, rel) => Atom::Fact(self.rv(*f), rel.clone()),
                Atom::Fcond(f, c, o, vs) => Atom::Fcond(self.rv(*f), c.clone(), o, vs.clone()),
                Atom::Fkey(f, c, v) => {
                    let f = self.rv(*f);
                    Atom::Fkey(f, c.clone(), self.rv(*v))
                }
                other => other.clone(),
            });
        }
        let negs = d.negs.iter().map(|n| Rc::new(n.iter().map(|x| self.rename(x)).collect())).collect();
        let cols = match &d.cols {
            Cols::Derived(rel, f) => Cols::Derived(rel.clone(), f.map(|f| self.rv(f))),
            Cols::Map(cs) => {
                let mut out = Vec::with_capacity(cs.len());
                for (n, c) in cs {
                    let c = match c {
                        Some(CRef::V(v)) => Some(CRef::V(self.rv(*v))),
                        Some(CRef::A(v, a)) => Some(CRef::A(self.rv(*v), *a)),
                        Some(CRef::D(rel, col, Some(f))) => Some(CRef::D(rel.clone(), col.clone(), Some(self.rv(*f)))),
                        other => other.clone(),
                    };
                    out.push((n.clone(), c));
                }
                Cols::Map(out)
            }
        };
        Disj { atoms: Atoms::of(atoms), negs, exact: d.exact, cols }
    }

    fn fresh_rel(&mut self, r: &Rel) -> Rel {
        Rel {
            disjs: r
                .disjs
                .iter()
                .map(|d| {
                    self.stamp += 1;
                    count(2, d.atoms.len() as u64);
                    count(3, 1);
                    self.rename(d)
                })
                .collect(),
            opaque: r.opaque,
        }
    }

    fn base(&mut self, name: &str) -> Rel {
        if SYNTAX.contains(&name) {
            let v = self.fresh();
            let cols = vec![
                ("id".into(), Some(CRef::V(v))),
                ("kind".into(), Some(CRef::A(v, Attr::Kind))),
                ("text".into(), Some(CRef::A(v, Attr::Text))),
                ("file".into(), Some(CRef::File)),
            ];
            return Rel { disjs: vec![Disj { atoms: Atoms::default(), negs: Vec::new(), exact: true, cols: Cols::Map(cols) }], opaque: false };
        }
        if CHILD.contains(&name) {
            let (p, c) = (self.fresh(), self.fresh());
            let cols = vec![
                ("parent".into(), Some(CRef::V(p))),
                ("child".into(), Some(CRef::V(c))),
                ("field".into(), Some(CRef::A(c, Attr::Field))),
                ("file".into(), Some(CRef::File)),
            ];
            return Rel { disjs: vec![Disj { atoms: Atoms::of(vec![Atom::Parent(c, p)]), negs: Vec::new(), exact: true, cols: Cols::Map(cols) }], opaque: false };
        }
        if language().facts.iter().any(|f| f == name) {
            let f = self.fresh();
            let atoms = vec![Atom::Derived(name.into()), Atom::Fact(f, name.into())];
            return Rel { disjs: vec![Disj { atoms: Atoms::of(atoms), negs: Vec::new(), exact: false, cols: Cols::Derived(name.into(), Some(f)) }], opaque: true };
        }
        Rel { disjs: vec![Disj { atoms: Atoms::of(vec![Atom::Derived(name.into())]), negs: Vec::new(), exact: false, cols: Cols::Derived(name.into(), None) }], opaque: true }
    }

    /// The step's relation, evaluated on its first read.
    fn force(&mut self, e: &Entry<'p>, env: &Env<'p>) -> Rc<Rel> {
        if let Some(r) = e.rel.get() {
            return r.clone();
        }
        let Some(expr) = e.expr else { return Rc::new(opaque()) };
        if e.busy.replace(true) {
            return Rc::new(opaque());
        }
        let r = Rc::new(match expr {
            Expression::Select(s) => self.select(s, env),
            Expression::Call(c) => self.call(c, env),
        });
        let _set = e.rel.set(r.clone());
        r
    }

    fn resolve_step(&mut self, name: &str, env: &Env<'p>) -> Rc<Rel> {
        let Some(e) = env.map.get(name) else { return Rc::new(opaque()) };
        let r = self.force(e, env);
        if e.used.replace(true) { Rc::new(self.fresh_rel(&r)) } else { r }
    }

    fn rel(&mut self, v: Option<&ArgumentValue>, env: &Env<'p>) -> Rc<Rel> {
        match v {
            Some(ArgumentValue::Text(t)) => Rc::new(self.base(&t.0)),
            Some(ArgumentValue::Step(s)) => self.resolve_step(&s.0, env),
            Some(ArgumentValue::Word(w)) => self.resolve_step(&w.0, env),
            _ => Rc::new(opaque()),
        }
    }

    fn select(&mut self, s: &Select, env: &Env<'p>) -> Rel {
        count(1, 1);
        let items: Option<Vec<(String, Option<(Option<String>, String)>)>> = match &s.columns {
            Columns::Every => None,
            Columns::Listed(cs) => Some(
                cs.iter()
                    .map(|c| {
                        let r = match &c.column {
                            SelectedValue::Column(c) => Some((alias_of(&c.alias), c.column.0.to_string())),
                            SelectedValue::Applied(_) => None,
                        };
                        let name = match &c.rename {
                            Renaming::Renamed(n) => n.0.to_string(),
                            Renaming::Unchanged => r.as_ref().map_or_else(|| "?".to_owned(), |(_, c)| c.clone()),
                        };
                        (name, r)
                    })
                    .collect(),
            ),
        };
        let mut sources = vec![&s.from];
        let mut ons: Vec<Vec<Cond>> = Vec::new();
        if let Joining::Joined(j) = &s.join {
            sources.push(&j.source);
            ons.push(
                j.on.iter()
                    .map(|k| Cond::Cmp(Cmp::Eq, Opd::Col(k.left.column.0.to_string(), alias_of(&k.left.alias)), Opd::Col(k.right.column.0.to_string(), alias_of(&k.right.alias))))
                    .collect(),
            );
        }
        let mut rels = Vec::new();
        for src in &sources {
            let r = match &src.reference {
                SourceReference::Relation(n) => Rc::new(self.base(&n.0)),
                SourceReference::Step(st) => self.resolve_step(&st.0, env),
            };
            rels.push((alias_of(&src.alias), r));
        }
        let w = conds(&s.filters);
        let mut combos: Vec<Vec<(Option<&str>, &Disj)>> = vec![Vec::new()];
        for (alias, rel) in &rels {
            // more combinations than the cap: not followed (the size is known before the product is made)
            if combos.len() * rel.disjs.len() > CAP {
                count(6, 1);
                return opaque();
            }
            combos = combos.iter().flat_map(|c| rel.disjs.iter().map(move |d| { let mut c = c.clone(); c.push((alias.as_deref(), d)); c })).collect();
        }
        let mut out = Vec::new();
        for combo in combos {
            count(0, 1);
            let (mut atoms, mut negs, mut exact) = (Atoms::default(), Vec::new(), true);
            let mut scope: Vec<(Option<&str>, &Cols)> = Vec::new();
            for (alias, d) in combo {
                atoms.extend_from(&d.atoms);
                negs.extend(d.negs.iter().cloned());
                exact = exact && d.exact;
                match scope.iter_mut().find(|(a, _)| *a == alias) {
                    Some(slot) => slot.1 = &d.cols,
                    None => scope.push((alias, &d.cols)),
                }
            }
            let mut d = Disj { atoms, negs, exact, cols: Cols::empty() };
            for o in &ons {
                apply_where(&mut d, o, &scope);
            }
            apply_where(&mut d, &w, &scope);
            d.cols = project(&scope, items.as_deref());
            out.push(d);
        }
        Rel { disjs: out, opaque: rels.iter().all(|(_, r)| r.opaque) }
    }

    fn antijoin(&mut self, args: &[Argument], env: &Env<'p>) -> Rel {
        let left = self.rel(arg(args, "left"), env);
        let right = self.rel(arg(args, "right"), env);
        let (lk, rk) = (keylist(arg(args, "left-on")), keylist(arg(args, "right-on")));
        let mut out = Vec::new();
        for d in &left.disjs {
            let mut nd = d.clone();
            let mut ok = !right.opaque && right.disjs.iter().all(|r| r.exact) && lk.len() == rk.len() && !lk.is_empty();
            let mut subs = Vec::new();
            if ok {
                for r in &right.disjs {
                    let mut eqs = Vec::new();
                    for (a, b) in lk.iter().zip(&rk) {
                        match (d.cols.get(a), r.cols.get(b)) {
                            (Some(CRef::V(x)), Some(CRef::V(y))) => eqs.push(Atom::Eq(x, y)),
                            (Some(CRef::File), Some(CRef::File)) => {}
                            _ => ok = false,
                        }
                    }
                    let mut atoms = r.atoms.clone();
                    atoms.push_vec(eqs);
                    subs.push(Disj { atoms, negs: r.negs.clone(), exact: true, cols: Cols::empty() });
                }
            }
            if ok {
                nd.negs.push(Rc::new(subs));
            } else {
                nd.exact = false;
            }
            out.push(nd);
        }
        Rel { disjs: out, opaque: left.opaque }
    }

    fn call(&mut self, c: &Call, env: &Env<'p>) -> Rel {
        let name = &*c.name.0;
        let args = &c.arguments;
        if let Some(module) = env.imports.get(name) {
            // the same fragment on the same relations gives the same relation up to its variables' names: a fresh copy
            let mut key: Vec<(String, MemoArg)> = Vec::new();
            let mut pins = Vec::new();
            for a in args {
                let k = match &a.value {
                    ArgumentValue::Text(t) => MemoArg::Base(t.0.to_string()),
                    ArgumentValue::Step(StepName(n, _)) => match env.map.get(&**n) {
                        Some(e) => {
                            let r = self.force(e, env);
                            let k = MemoArg::Step(Rc::as_ptr(&r) as usize);
                            pins.push(r);
                            k
                        }
                        None => MemoArg::Opaque,
                    },
                    ArgumentValue::Word(w) => match env.map.get(&*w.0) {
                        Some(e) => {
                            let r = self.force(e, env);
                            let k = MemoArg::Step(Rc::as_ptr(&r) as usize);
                            pins.push(r);
                            k
                        }
                        None => MemoArg::Opaque,
                    },
                    _ => continue,
                };
                key.push((a.name.0.to_string(), k));
            }
            let key = (module.clone(), key);
            let hit = if pins.is_empty() { self.pure.get(&key).cloned() } else { self.local.get(&key).map(|(_, r)| r.clone()) };
            count(4, 1);
            if let Some(r) = hit {
                count(5, 1);
                return self.fresh_rel(&r);
            }
            let mut params: HashMap<String, Entry<'p>> = HashMap::new();
            for a in args {
                if matches!(a.value, ArgumentValue::Text(_) | ArgumentValue::Step(_) | ArgumentValue::Word(_)) {
                    let r = self.rel(Some(&a.value), env);
                    params.insert(a.name.0.to_string(), Entry::ready(r));
                }
            }
            let r = self.fragment(module, params);
            if pins.is_empty() {
                self.pure.insert(key, r.clone());
            } else {
                self.local.insert(key, (pins, r.clone()));
            }
            return (*r).clone();
        }
        if name == "union" {
            let (l, r) = (self.rel(arg(args, "left"), env), self.rel(arg(args, "right"), env));
            let opaque_both = l.opaque && r.opaque;
            if l.disjs.len() + r.disjs.len() > CAP {
                count(6, 1);
                return opaque();
            }
            let mut ds = l.disjs.clone();
            ds.extend(r.disjs.iter().cloned());
            if ds.len() > CAP {
                return opaque();
            }
            return Rel { disjs: ds, opaque: opaque_both };
        }
        if name == "antijoin" {
            return self.antijoin(args, env);
        }
        if name == OP_ROWS && arg(args, "from").is_some() && arg(args, "columns").is_some() {
            return rows_table(arg(args, "from"), arg(args, "columns")).unwrap_or_else(opaque);
        }
        let src = arg(args, "from").or_else(|| arg(args, "left"));
        if NEW_ROWS.contains(&name) || src.is_none() {
            // the first relation the call is given (a step it reads)
            let inp = args.iter().map(|a| &a.value).find(|v| matches!(v, ArgumentValue::Step(_) | ArgumentValue::Word(_)));
            if let Some(inp) = inp
                && name != OP_ROWS
            {
                let x = self.rel(Some(inp), env);
                return Rel {
                    disjs: x.disjs.iter().map(|d| Disj { atoms: Atoms::of(d.atoms.iter().filter(|a| a.nonvar()).cloned().collect()), negs: Vec::new(), exact: false, cols: Cols::empty() }).collect(),
                    opaque: true,
                };
            }
            return opaque();
        }
        let x = self.rel(src, env);
        let exact = name == "extend" || name == "project";
        Rel {
            disjs: x
                .disjs
                .iter()
                .map(|d| {
                    let mut d = d.clone();
                    d.exact = d.exact && exact;
                    d
                })
                .collect(),
            opaque: x.opaque,
        }
    }

    /// A program's result: its steps bound by name (each evaluated when first read), the result step read.
    fn program(&mut self, steps: &'p [LetStep], mut map: HashMap<String, Entry<'p>>, imports: Imports, result: &str) -> Rc<Rel> {
        for st in steps {
            map.insert(st.name.0.to_string(), Entry::lazy(&st.expression));
        }
        let env = Env { map, imports };
        self.resolve_step(result, &env)
    }

    fn fragment(&mut self, module: &str, params: HashMap<String, Entry<'p>>) -> Rc<Rel> {
        let frags = self.frags;
        let Some(f): Option<&'p FragmentProgram> = frags.0.get(&QuerySourceId(module.into())) else { return Rc::new(opaque()) };
        self.program(&f.steps, params, imports_of(&f.imports), &f.result.step.0)
    }
}

fn keylist(v: Option<&ArgumentValue>) -> Vec<String> {
    fn one(v: &ArgumentValue) -> Option<String> {
        match v {
            ArgumentValue::Word(w) => Some(w.0.to_string()),
            ArgumentValue::Text(t) => Some(t.0.to_string()),
            ArgumentValue::Step(s) => Some(s.0.to_string()),
            _ => None,
        }
    }
    match v {
        Some(ArgumentValue::List(xs)) => xs.iter().filter_map(one).collect(),
        Some(x) => one(x).into_iter().collect(),
        None => Vec::new(),
    }
}

/// `rows(from=[[...], ...], columns=[...])`: a literal table, each column its values.
fn rows_table(from: Option<&ArgumentValue>, columns: Option<&ArgumentValue>) -> Option<Rel> {
    let text = |v: &ArgumentValue| match v {
        ArgumentValue::Text(t) => Some(t.0.to_string()),
        ArgumentValue::Word(w) => Some(w.0.to_string()),
        ArgumentValue::Number(n) => Some(n.0.to_string()),
        _ => None,
    };
    let ArgumentValue::List(names) = columns? else { return None };
    let names: Vec<String> = names.iter().map(text).collect::<Option<_>>()?;
    let ArgumentValue::List(rows) = from? else { return None };
    let rows: Vec<Vec<String>> = rows
        .iter()
        .map(|r| match r {
            ArgumentValue::List(xs) => xs.iter().map(text).collect::<Option<Vec<_>>>(),
            _ => None,
        })
        .collect::<Option<_>>()?;
    let mut cols = Vec::new();
    for (i, n) in names.iter().enumerate() {
        let vals: Set = rows.iter().map(|r| r.get(i).cloned()).collect::<Option<_>>()?;
        put(&mut cols, n, Some(CRef::R(Rc::new(vals))));
    }
    Some(Rel { disjs: vec![Disj { atoms: Atoms::default(), negs: Vec::new(), exact: false, cols: Cols::Map(cols) }], opaque: true })
}

fn project(scope: &Scope<'_>, items: Option<&[(String, Option<(Option<String>, String)>)]>) -> Cols {
    let mut out = Vec::new();
    let Some(items) = items else {
        for (_, c) in scope {
            if let Cols::Map(m) = c {
                for (n, r) in m {
                    put(&mut out, n, r.clone());
                }
            }
        }
        return Cols::Map(out);
    };
    for (name, r) in items {
        let v = match r {
            None => None,
            Some((Some(alias), col)) => scope.iter().find(|(a, _)| *a == Some(alias.as_str())).and_then(|(_, c)| c.get(col)),
            Some((None, col)) => scope.iter().find(|(_, c)| c.contains(col)).and_then(|(_, c)| c.get(col)),
        };
        put(&mut out, name, v);
    }
    Cols::Map(out)
}

fn col_of(o: &Opd, scope: &Scope<'_>) -> Option<CRef> {
    let Opd::Col(name, alias) = o else { return None };
    if let Some(a) = alias {
        return scope.iter().find(|(x, _)| *x == Some(a.as_str())).and_then(|(_, c)| c.get(name));
    }
    // the sources with listed columns first, then the derived ones (every name is one of their columns)
    let listed = scope.iter().find(|(_, c)| !c.derived() && c.contains(name));
    listed.or_else(|| scope.iter().find(|(_, c)| c.derived())).and_then(|(_, c)| c.get(name))
}

fn dlit_atoms(rel: &str, col: &str, f: Option<V>, vals: &Set, op: Cmp) -> Vec<Atom> {
    let mut out = vec![Atom::Dlit(rel.into(), col.into())];
    if let Some(f) = f {
        out.push(Atom::Fcond(f, col.into(), op.name(), vals.iter().cloned().collect()));
    }
    if op != Cmp::Eq {
        return out;
    }
    let ws: Option<Set> = vals.iter().map(|v| word_of(rel, col, v)).collect();
    if let Some(ws) = ws
        && !ws.is_empty()
        && ws.iter().all(|w| ident(w))
    {
        out.push(Atom::Words(Rc::new(ws)));
    }
    out
}

fn text_words(attr: Attr, v: V, vals: Set) -> Vec<Atom> {
    let vals = Rc::new(vals);
    if attr == Attr::Text && !vals.is_empty() && vals.iter().all(|w| ident(w)) {
        let w = Atom::Words(vals.clone());
        return vec![Atom::Text(v, vals), w];
    }
    vec![Atom::attr(attr, v, vals)]
}

fn lits(o: &Opd) -> Option<Set> {
    match o {
        Opd::Lit(v) => Some(BTreeSet::from([v.clone()])),
        _ => None,
    }
}

/// The atoms of one conjunct, or `None` when it cannot be expressed (dropped: inexact).
fn cond_atoms(c: &Cond, scope: &Scope<'_>) -> Option<Vec<Atom>> {
    match c {
        Cond::Cmp(op @ (Cmp::Lt | Cmp::Le | Cmp::Gt | Cmp::Ge), l @ Opd::Col(..), Opd::Lit(v)) => match col_of(l, scope)? {
            CRef::D(_, col, Some(f)) => Some(vec![Atom::Fcond(f, col, op.name(), Rc::from(vec![v.clone()]))]),
            _ => None,
        },
        Cond::Cmp(op @ (Cmp::Eq | Cmp::Ne), l @ Opd::Col(..), r) => {
            let rf = col_of(l, scope);
            if let Opd::Col(..) = r {
                let other = col_of(r, scope);
                let (rf, other) = (rf?, other?);
                return match (op, &rf, &other) {
                    (Cmp::Eq, CRef::V(a), CRef::V(b)) => Some(vec![Atom::Eq(*a, *b)]),
                    (Cmp::Eq, CRef::D(rel, col, f), CRef::R(vals)) | (Cmp::Eq, CRef::R(vals), CRef::D(rel, col, f)) => Some(dlit_atoms(rel, col, *f, vals, Cmp::Eq)),
                    (Cmp::Eq, CRef::D(_, col, fv), CRef::V(v)) | (Cmp::Eq, CRef::V(v), CRef::D(_, col, fv)) => fv.map(|f| vec![Atom::Fkey(f, col.clone(), *v), Atom::Inexact]),
                    (_, CRef::File, CRef::File) => Some(Vec::new()),
                    _ => None,
                };
            }
            let vals = lits(r);
            match (rf?, vals) {
                (CRef::D(rel, col, f), Some(vals)) => Some(dlit_atoms(&rel, &col, f, &vals, *op)),
                (CRef::A(v, attr), Some(vals)) => match op {
                    Cmp::Eq => Some(text_words(attr, v, vals)),
                    _ if attr == Attr::Kind => Some(vec![Atom::NotKind(v, Rc::new(vals))]),
                    _ => None,
                },
                _ => None,
            }
        }
        Cond::Or(branches) => or_atoms(branches, scope),
        Cond::Matches(l @ Opd::Col(..), Opd::Lit(p)) => match col_of(l, scope)? {
            CRef::D(rel, col, Some(f)) => Some(vec![Atom::Dlit(rel, col.clone()), Atom::Fcond(f, col, "re", Rc::from(vec![p.clone()]))]),
            CRef::A(v, Attr::Text) => Some(vec![Atom::Re(v, p.as_str().into())]),
            _ => None,
        },
        _ => None,
    }
}

/// An OR: of equalities on one attribute column or one derived column; else its required words and literal marks only.
fn or_atoms(branches: &[Vec<Cond>], scope: &Scope<'_>) -> Option<Vec<Atom>> {
    let mut parts: Vec<&[Cond]> = Vec::new();
    fn flat<'c>(bs: &'c [Vec<Cond>], parts: &mut Vec<&'c [Cond]>) {
        for b in bs {
            match b.as_slice() {
                [Cond::Or(inner)] => flat(inner, parts),
                _ => parts.push(b),
            }
        }
    }
    flat(branches, &mut parts);
    let mut branch_words: Vec<Option<ASet>> = Vec::new();
    let mut marks: Vec<Atom> = Vec::new();
    for p in &parts {
        let mut ws: Vec<ASet> = Vec::new();
        for q in *p {
            let g = cond_atoms(q, scope).unwrap_or_default();
            for a in g {
                match a {
                    Atom::Words(w) => ws.push(w),
                    m @ Atom::Dlit(..) => marks.push(m),
                    _ => {}
                }
            }
        }
        branch_words.push(ws.into_iter().min_by_key(|w| w.len()));
    }
    let or_words: Vec<Atom> = if branch_words.iter().all(Option::is_some) { vec![Atom::Words(Rc::new(branch_words.iter().flatten().flat_map(|w| w.iter().cloned()).collect()))] } else { Vec::new() };
    let got: Vec<Option<Vec<Atom>>> = parts.iter().map(|p| if p.len() == 1 { cond_atoms(&p[0], scope) } else { None }).collect();
    let dl: Vec<&Vec<Atom>> = got.iter().flatten().filter(|g| matches!(g.first(), Some(Atom::Dlit(..)))).collect();
    if dl.len() == got.len() {
        let heads: BTreeSet<(Str, Str)> = dl.iter().filter_map(|g| if let Some(Atom::Dlit(r, c)) = g.first() { Some((r.clone(), c.clone())) } else { None }).collect();
        if heads.len() == 1 {
            let mut out = vec![dl[0][0].clone()];
            let fcs: Vec<Vec<&Atom>> = dl.iter().map(|g| g.iter().filter(|a| matches!(a, Atom::Fcond(..))).collect()).collect();
            let single_eq = fcs.iter().all(|x| x.len() == 1 && matches!(x[0], Atom::Fcond(_, _, "eq", _)));
            if single_eq {
                let keys: BTreeSet<(V, Str)> = fcs.iter().filter_map(|x| if let Atom::Fcond(f, c, _, _) = x[0] { Some((*f, c.clone())) } else { None }).collect();
                if keys.len() == 1 {
                    let (f, c) = keys.into_iter().next().unwrap_or((0, "".into()));
                    let vals: Set = fcs.iter().flat_map(|x| if let Atom::Fcond(_, _, _, vs) = x[0] { vs.to_vec() } else { Vec::new() }).collect();
                    out.push(Atom::Fcond(f, c, "eq", vals.into_iter().collect()));
                }
            }
            let wl: Vec<Vec<&ASet>> = dl.iter().map(|g| g.iter().filter_map(|a| if let Atom::Words(w) = a { Some(w) } else { None }).collect()).collect();
            if wl.iter().all(|w| !w.is_empty()) {
                out.push(Atom::Words(Rc::new(wl.iter().flat_map(|w| w[0].iter().cloned()).collect())));
            }
            return Some(out);
        }
    }
    let stripped: Vec<Option<Vec<Atom>>> = got.into_iter().map(|g| g.map(|g| g.into_iter().filter(|a| !matches!(a, Atom::Words(_))).collect())).collect();
    if stripped.iter().all(|g| g.as_ref().is_some_and(|g| g.len() == 1 && g[0].attr_test().is_some())) {
        let keys: BTreeSet<(Attr, V)> = stripped.iter().flatten().filter_map(|g| g[0].attr_test().map(|(a, v, _)| (a, v))).collect();
        if keys.len() == 1 {
            let (attr, v) = keys.into_iter().next().unwrap_or((Attr::Kind, 0));
            let vals: Set = stripped.iter().flatten().filter_map(|g| g[0].attr_test().map(|(_, _, s)| s.clone())).flat_map(|s| s.iter().cloned().collect::<Vec<_>>()).collect();
            return Some(text_words(attr, v, vals));
        }
    }
    if !or_words.is_empty() || !marks.is_empty() {
        let mut out = vec![Atom::Inexact];
        out.extend(or_words);
        for m in marks {
            if !out.contains(&m) {
                out.push(m);
            }
        }
        return Some(out);
    }
    None
}

fn apply_where(d: &mut Disj, w: &[Cond], scope: &Scope<'_>) {
    for c in w {
        match cond_atoms(c, scope) {
            None => d.exact = false,
            Some(got) => {
                if got.contains(&Atom::Inexact) {
                    d.exact = false;
                }
                d.atoms.push_vec(got.into_iter().filter(|a| *a != Atom::Inexact).collect());
            }
        }
    }
}

// ---------------------------------------------------------------- a disjunct as rooted plans
#[derive(Default)]
struct Uf(HashMap<V, V>);

impl Uf {
    fn find(&mut self, x: V) -> V {
        let mut x = x;
        loop {
            let p = *self.0.entry(x).or_insert(x);
            if p == x {
                return x;
            }
            let gp = *self.0.entry(p).or_insert(p);
            self.0.insert(x, gp);
            x = gp;
        }
    }
    fn union(&mut self, a: V, b: V) {
        let (a, b) = (self.find(a), self.find(b));
        if a != b {
            self.0.insert(a, b);
        }
    }
}

#[derive(Clone, Default, Debug)]
struct Props {
    kinds: Option<Set>,
    notkinds: Set,
    texts: Option<Set>,
    fields: Option<Set>,
    re: Vec<String>,
}

fn meet(a: &mut Option<Set>, s: &Set) {
    *a = Some(match a.take() {
        Some(x) => x.intersection(s).cloned().collect(),
        None => s.clone(),
    });
}

/// Per variable its tests and the parent edges after unification (parent is functional); `None` when unsatisfiable.
fn collect(atoms: &[Atom], uf: &mut Uf) -> Option<(BTreeMap<V, Props>, Vec<(V, V)>)> {
    loop {
        let mut changed = false;
        let mut par: HashMap<V, V> = HashMap::new();
        for a in atoms {
            if let Atom::Parent(c, p) = a {
                let (c, p) = (uf.find(*c), uf.find(*p));
                match par.get(&c) {
                    Some(q) if *q != p => {
                        uf.union(*q, p);
                        changed = true;
                    }
                    _ => {
                        par.insert(c, p);
                    }
                }
            }
        }
        if !changed {
            break;
        }
    }
    let mut props: BTreeMap<V, Props> = BTreeMap::new();
    for a in atoms {
        let (v, kind) = match a {
            Atom::Kind(v, _) | Atom::NotKind(v, _) | Atom::Text(v, _) | Atom::Field(v, _) | Atom::Re(v, _) => (uf.find(*v), a),
            _ => continue,
        };
        let pr = props.entry(v).or_default();
        match kind {
            Atom::Kind(_, s) => meet(&mut pr.kinds, s),
            Atom::NotKind(_, s) => pr.notkinds.extend(s.iter().cloned()),
            Atom::Text(_, s) => meet(&mut pr.texts, s),
            Atom::Field(_, s) => meet(&mut pr.fields, s),
            Atom::Re(_, r) => pr.re.push(r.to_string()),
            _ => {}
        }
    }
    for pr in props.values_mut() {
        if let Some(k) = &mut pr.kinds {
            k.retain(|x| !pr.notkinds.contains(x));
            if k.is_empty() {
                return None;
            }
        }
        if pr.texts.as_ref().is_some_and(BTreeSet::is_empty) || pr.fields.as_ref().is_some_and(BTreeSet::is_empty) {
            return None;
        }
    }
    let mut parent: Vec<(V, V)> = Vec::new();
    for a in atoms {
        if let Atom::Parent(c, p) = a {
            let (c, p) = (uf.find(*c), uf.find(*p));
            match parent.iter_mut().find(|(x, _)| *x == c) {
                Some(slot) => slot.1 = p,
                None => parent.push((c, p)),
            }
        }
    }
    Some((props, parent))
}

fn check_op(v: usize, pr: &Props) -> Option<SpecOp> {
    let c = SpecCheck {
        v,
        kinds: pr.kinds.as_ref().map(|k| k.iter().cloned().collect()),
        notkinds: (!pr.notkinds.is_empty()).then(|| pr.notkinds.iter().cloned().collect()),
        texts: pr.texts.as_ref().map(|t| t.iter().map(|x| x.as_bytes().to_vec()).collect()),
        fields: pr.fields.as_ref().map(|f| f.iter().cloned().collect()),
        re: (!pr.re.is_empty()).then(|| pr.re.iter().map(|r| format!("(?:{r})")).collect::<Vec<_>>().join("|")),
    };
    (c.kinds.is_some() || c.notkinds.is_some() || c.texts.is_some() || c.fields.is_some() || c.re.is_some()).then_some(SpecOp::Check(c))
}

fn vars_of_atoms(atoms: &[Atom], uf: &mut Uf) -> BTreeSet<V> {
    let mut out = BTreeSet::new();
    for a in atoms {
        match a {
            Atom::Parent(x, y) | Atom::Eq(x, y) => {
                out.insert(uf.find(*x));
                out.insert(uf.find(*y));
            }
            Atom::Kind(v, _) | Atom::NotKind(v, _) | Atom::Text(v, _) | Atom::Field(v, _) | Atom::Re(v, _) => {
                out.insert(uf.find(*v));
            }
            _ => {}
        }
    }
    out
}

type Adj = BTreeMap<V, Vec<(bool, V)>>;

/// `true`: up.
fn adjacency(parent: &[(V, V)]) -> Adj {
    let mut adj: Adj = BTreeMap::new();
    for (c, p) in parent {
        adj.entry(*c).or_default().push((true, *p));
        adj.entry(*p).or_default().push((false, *c));
    }
    adj
}

fn ups_first(adj: &Adj, v: V) -> Vec<(bool, V)> {
    let mut n = adj.get(&v).cloned().unwrap_or_default();
    n.sort_by_key(|(up, _)| !*up);
    n
}

/// A NOT EXISTS sub-plan over the bound registers: its ops and its bound registers; `Err(true)`: unsatisfiable (dropped);
/// `Err(false)`: it reaches outside them (not expressible).
fn neg_plan(x: &Disj, outer_uf: &Uf, reg: &[(V, usize)]) -> Result<(Vec<SpecOp>, Vec<usize>), bool> {
    let xatoms = x.atoms.flat();
    let mut uf = Uf(outer_uf.0.clone());
    for a in &xatoms {
        if let Atom::Eq(p, q) = a {
            uf.union(*p, *q);
        }
    }
    let mut outer: HashMap<V, usize> = HashMap::new();
    for (v, r) in reg {
        let f = uf.find(*v);
        outer.entry(f).or_insert(*r);
    }
    let (props, parent) = collect(&xatoms, &mut uf).ok_or(true)?;
    if !x.negs.is_empty() {
        return Err(false);
    }
    let allv = vars_of_atoms(&xatoms, &mut uf);
    // in register order (structural, whatever the variables are named)
    let mut bound: Vec<V> = allv.iter().copied().filter(|v| outer.contains_key(v)).collect();
    bound.sort_by_key(|v| outer[v]);
    if bound.is_empty() {
        return Err(false);
    }
    let adj = adjacency(&parent);
    let mut ops = Vec::new();
    let mut seen: BTreeSet<V> = bound.iter().copied().collect();
    let mut regs: HashMap<V, usize> = bound.iter().map(|v| (*v, outer[v])).collect();
    for v in &bound {
        if let Some(op) = props.get(v).and_then(|p| check_op(regs[v], p)) {
            ops.push(op);
        }
    }
    let mut used: BTreeSet<(V, V)> = BTreeSet::new();
    let mut stack: Vec<V> = bound.clone();
    while let Some(v) = stack.pop() {
        for (up, w) in ups_first(&adj, v) {
            let edge = if up { (v, w) } else { (w, v) };
            if !used.insert(edge) {
                continue;
            }
            if seen.contains(&w) {
                ops.push(SpecOp::IsParent { child: regs[&edge.0], parent: regs[&edge.1] });
                continue;
            }
            seen.insert(w);
            let r = SUB_REG + regs.len();
            regs.insert(w, r);
            let nw = adj.get(&w).map_or(&[][..], Vec::as_slice);
            let leaf = nw.iter().all(|(_, x2)| seen.contains(x2)) && nw.len() == 1;
            ops.push(if up { SpecOp::Up { from: regs[&v], to: r, leaf } } else { SpecOp::Down { from: regs[&v], to: r, leaf } });
            if let Some(op) = props.get(&w).and_then(|p| check_op(r, p)) {
                ops.push(op);
            }
            stack.push(w);
        }
    }
    if seen.len() != allv.len() {
        return Err(false);
    }
    Ok((ops, bound.iter().map(|v| regs[v]).collect()))
}

/// Sub-plan registers (from `SUB_REG`) become registers after the positive ones.
fn renumber(ops: Vec<SpecOp>, nregs: usize) -> (Vec<SpecOp>, usize) {
    fn walk(ops: Vec<SpecOp>, m: &mut Vec<usize>, nregs: usize) -> Vec<SpecOp> {
        let mut r = |x: usize, m: &mut Vec<usize>| {
            if x < SUB_REG {
                return x;
            }
            match m.iter().position(|y| *y == x) {
                Some(i) => nregs + i,
                None => {
                    m.push(x);
                    nregs + m.len() - 1
                }
            }
        };
        ops.into_iter()
            .map(|o| match o {
                SpecOp::Check(mut c) => {
                    c.v = r(c.v, m);
                    SpecOp::Check(c)
                }
                SpecOp::Up { from, to, leaf } => {
                    let from = r(from, m);
                    SpecOp::Up { from, to: r(to, m), leaf }
                }
                SpecOp::Down { from, to, leaf } => {
                    let from = r(from, m);
                    SpecOp::Down { from, to: r(to, m), leaf }
                }
                SpecOp::IsParent { child, parent } => {
                    let child = r(child, m);
                    SpecOp::IsParent { child, parent: r(parent, m) }
                }
                SpecOp::Not(p) => SpecOp::Not(walk(p, m, nregs)),
                SpecOp::Exists { from, to, plan } => {
                    let from = r(from, m);
                    let to = r(to, m);
                    SpecOp::Exists { from, to, plan: walk(plan, m, nregs) }
                }
                SpecOp::Fact(mut f) => {
                    for k in &mut f.keys {
                        k.1 = r(k.1, m);
                    }
                    SpecOp::Fact(f)
                }
                other => other,
            })
            .collect()
    }
    let mut m = Vec::new();
    let ops = walk(ops, &mut m, nregs);
    (ops, nregs + m.len())
}

/// The planner's inputs for one disjunct, shared by its alternatives.
struct Rooted<'a> {
    d: &'a Disj,
    atoms: Vec<Atom>,
    uf: Uf,
    props: BTreeMap<V, Props>,
    adj: Adj,
}

impl Rooted<'_> {
    /// The plan of the disjunct entered at `root` by `bucket` `keys`: (ops, registers).
    fn plan(&self, root: V, bucket: Bucket, keys: &[String]) -> (Vec<SpecOp>, usize) {
        let adj = &self.adj;
        let mut reg: Vec<(V, usize)> = vec![(root, 0)];
        let regof = |reg: &Vec<(V, usize)>, v: V| reg.iter().find(|(x, _)| *x == v).map(|(_, r)| *r);
        let mut k = 0;
        while k < reg.len() {
            let v = reg[k].0;
            k += 1;
            for (_, w) in adj.get(&v).map_or(&[][..], Vec::as_slice) {
                if regof(&reg, *w).is_none() {
                    let n = reg.len();
                    reg.push((*w, n));
                }
            }
        }
        let r = |v: V| regof(&reg, v).unwrap_or(usize::MAX);
        // negations: plans over the bound variables of the root's component
        let mut neg_plans: Vec<(Vec<SpecOp>, Vec<usize>)> = Vec::new();
        for n in &self.d.negs {
            let mut subs = Vec::new();
            let mut ok = true;
            for x in n.iter() {
                match neg_plan(x, &self.uf, &reg) {
                    Err(false) => {
                        ok = false;
                        break;
                    }
                    Err(true) => {}
                    Ok(sp) => subs.push(sp),
                }
            }
            if ok {
                for sp in subs {
                    if !neg_plans.contains(&sp) {
                        neg_plans.push(sp);
                    }
                }
            }
        }
        // fact tests keyed by tree variables of the component, with their column conditions
        let mut facts: Vec<(V, String, Vec<(String, V)>, Vec<(String, String, Vec<String>)>)> = Vec::new();
        for a in &self.atoms {
            if let Atom::Fact(f, rel) = a
                && !facts.iter().any(|x| x.0 == *f)
            {
                facts.push((*f, rel.to_string(), Vec::new(), Vec::new()));
            }
        }
        let mut uf = Uf(self.uf.0.clone());
        for a in &self.atoms {
            match a {
                Atom::Fkey(f, c, v) => {
                    let v = uf.find(*v);
                    if let Some(x) = facts.iter_mut().find(|x| x.0 == *f) {
                        x.2.push((c.to_string(), v));
                    }
                }
                Atom::Fcond(f, c, o, vs) => {
                    if let Some(x) = facts.iter_mut().find(|x| x.0 == *f) {
                        x.3.push((c.to_string(), (*o).to_owned(), vs.to_vec()));
                    }
                }
                _ => {}
            }
        }
        let mut fact_ops: Vec<SpecOp> = Vec::new();
        for (_, rel, keys, conds) in facts {
            let fkeys: BTreeSet<(String, usize)> = keys.iter().filter_map(|(c, v)| regof(&reg, *v).map(|r| (c.clone(), r))).collect();
            if fkeys.is_empty() {
                continue;
            }
            let mut conds = conds;
            conds.sort_by_key(|c| format!("{c:?}"));
            let op = SpecOp::Fact(SpecFact { rel, keys: fkeys.into_iter().collect(), conds });
            if !fact_ops.contains(&op) {
                fact_ops.push(op);
            }
        }
        let props_of = |v: V| self.props.get(&v);
        let mut rc = props_of(root).cloned().unwrap_or_default();
        match bucket {
            Bucket::Kinds if rc.kinds.as_ref().is_some_and(|k| k.iter().eq(keys.iter())) => rc.kinds = None,
            Bucket::Fields if rc.fields.as_ref().is_some_and(|f| f.iter().eq(keys.iter())) => rc.fields = None,
            Bucket::Words => {
                rc.kinds = None;
                rc.texts = None;
            }
            _ => {}
        }
        // the linear form: a walk from the root, ups before downs, checks right after assignment, negations once bound
        let mut ops: Vec<SpecOp> = Vec::new();
        let mut bound: BTreeSet<usize> = BTreeSet::from([0]);
        if let Some(op) = check_op(0, &rc) {
            ops.push(op);
        }
        let mut pending: Vec<(Vec<SpecOp>, Vec<usize>)> = neg_plans.clone();
        let flush = |ops: &mut Vec<SpecOp>, pending: &mut Vec<(Vec<SpecOp>, Vec<usize>)>, bound: &BTreeSet<usize>| {
            let mut i = 0;
            while i < pending.len() {
                if pending[i].1.iter().all(|b| bound.contains(b)) {
                    let sp = pending.remove(i);
                    ops.push(SpecOp::Not(sp.0));
                } else {
                    i += 1;
                }
            }
        };
        flush(&mut ops, &mut pending, &bound);
        let mut seen: BTreeSet<V> = BTreeSet::from([root]);
        let mut stack: Vec<(V, std::vec::IntoIter<(bool, V)>)> = vec![(root, ups_first(adj, root).into_iter())];
        while let Some((v, it)) = stack.last_mut() {
            let v = *v;
            let Some((up, w)) = it.next() else {
                stack.pop();
                continue;
            };
            if !seen.insert(w) {
                continue;
            }
            let nw = adj.get(&w).map_or(&[][..], Vec::as_slice);
            let leaf = nw.iter().all(|(_, x)| seen.contains(x)) && !pending.iter().any(|sp| sp.1.contains(&r(w)));
            ops.push(if up { SpecOp::Up { from: r(v), to: r(w), leaf } } else { SpecOp::Down { from: r(v), to: r(w), leaf } });
            if let Some(op) = props_of(w).and_then(|p| check_op(r(w), p)) {
                ops.push(op);
            }
            bound.insert(r(w));
            flush(&mut ops, &mut pending, &bound);
            stack.push((w, ups_first(adj, w).into_iter()));
        }
        flush(&mut ops, &mut pending, &bound);
        ops.extend(fact_ops.iter().cloned());
        // the tree form: a child's subtree is one `exists` block, so independent branches never backtrack into each other
        let mut placed = vec![false; neg_plans.len()];
        let mut fplaced = vec![false; fact_ops.len()];
        let tree = self.build(root, &BTreeSet::new(), None, check_op(0, &rc), &reg, &neg_plans, &fact_ops, &mut placed, &mut fplaced).0;
        if placed.iter().all(|x| *x) && fplaced.iter().all(|x| *x) {
            ops = tree;
        }
        assert!(reg.len() < SUB_REG, "a plan with {} registers: more than the negations' numbering allows", reg.len());
        renumber(ops, reg.len())
    }

    #[expect(clippy::too_many_arguments, reason = "throwaway prototype")]
    fn build(
        &self,
        v: V,
        avail: &BTreeSet<usize>,
        came: Option<V>,
        own: Option<SpecOp>,
        reg: &[(V, usize)],
        negs: &[(Vec<SpecOp>, Vec<usize>)],
        facts: &[SpecOp],
        placed: &mut [bool],
        fplaced: &mut [bool],
    ) -> (Vec<SpecOp>, BTreeSet<usize>) {
        let r = |v: V| reg.iter().find(|(x, _)| *x == v).map_or(usize::MAX, |(_, r)| *r);
        let negs_now = |avail: &BTreeSet<usize>, placed: &mut [bool], fplaced: &mut [bool]| {
            let mut out = Vec::new();
            for (i, fo) in facts.iter().enumerate() {
                if let SpecOp::Fact(f) = fo
                    && !fplaced[i]
                    && f.keys.iter().all(|k| avail.contains(&k.1))
                {
                    fplaced[i] = true;
                    out.push(fo.clone());
                }
            }
            for (i, sp) in negs.iter().enumerate() {
                if !placed[i] && sp.1.iter().all(|b| avail.contains(b)) {
                    placed[i] = true;
                    out.push(SpecOp::Not(sp.0.clone()));
                }
            }
            out
        };
        let mut out = Vec::new();
        if let Some(c) = own {
            out.push(c);
        }
        let mut avail = avail.clone();
        avail.insert(r(v));
        out.extend(negs_now(&avail, placed, fplaced));
        let nbrs = ups_first(&self.adj, v);
        for (up, w) in &nbrs {
            if Some(*w) == came || !*up {
                continue;
            }
            let nw = self.adj.get(w).map_or(&[][..], Vec::as_slice);
            let leaf = nw.iter().all(|(_, x)| *x == v) && !negs.iter().any(|sp| sp.1.contains(&r(*w)));
            out.push(SpecOp::Up { from: r(v), to: r(*w), leaf });
            let (sub, a) = self.build(*w, &avail, Some(v), self.props.get(w).and_then(|p| check_op(r(*w), p)), reg, negs, facts, placed, fplaced);
            out.extend(sub);
            avail = a;
        }
        for (up, w) in self.adj.get(&v).map_or(&[][..], Vec::as_slice) {
            if Some(*w) == came || *up {
                continue;
            }
            let (sub, _) = self.build(*w, &avail, Some(v), self.props.get(w).and_then(|p| check_op(r(*w), p)), reg, negs, facts, placed, fplaced);
            out.push(SpecOp::Exists { from: r(v), to: r(*w), plan: sub });
        }
        out.extend(negs_now(&avail, placed, fplaced));
        (out, avail)
    }
}

/// The required-word CNF of a disjunct: its word clauses, a clause implied by a smaller one dropped.
fn cnf_of(d: &Disj) -> Vec<Vec<String>> {
    let mut cs: Vec<Set> = Vec::new();
    for a in d.atoms.iter() {
        if let Atom::Words(w) = a
            && !cs.contains(&**w)
        {
            cs.push((**w).clone());
        }
    }
    cs.sort_by(|a, b| a.len().cmp(&b.len()).then_with(|| a.cmp(b)));
    cs.iter().filter(|c| !cs.iter().any(|o| o.len() < c.len() && o.is_subset(c))).map(|c| c.iter().cloned().collect()).collect()
}

/// The buckets a variable can be entered at: its kinds, its fields, and the required words when it is a name token spelling
/// identifiers. No census: every one is kept, and each file enters at the one with the fewest nodes.
fn buckets(pr: &Props, word_kind: &str) -> Vec<(Bucket, Vec<String>)> {
    let mut out = Vec::new();
    if let Some(k) = &pr.kinds {
        out.push((Bucket::Kinds, k.iter().cloned().collect()));
    }
    if let Some(f) = &pr.fields {
        out.push((Bucket::Fields, f.iter().cloned().collect()));
    }
    if let (Some(t), Some(k)) = (&pr.texts, &pr.kinds)
        && !t.is_empty()
        && t.iter().all(|x| ident(x))
        && k.iter().all(|x| x == word_kind)
    {
        out.push((Bucket::Words, t.iter().cloned().collect()));
    }
    out
}

/// What a disjunct gives: its alternatives (each a plan), unsatisfiable, or no anchor.
enum Planned {
    Plans(Vec<SpecPlan>),
    Unsat,
    NoAnchor,
}

fn plan_disjunct(d: &Disj, word_kind: &str, disjunct: usize) -> Planned {
    let atoms = d.atoms.flat();
    let mut uf = Uf::default();
    for a in &atoms {
        if let Atom::Eq(x, y) = a {
            uf.union(*x, *y);
        }
    }
    let Some((props, parent)) = collect(&atoms, &mut uf) else { return Planned::Unsat };
    let adj = adjacency(&parent);
    let rooted = Rooted { d, atoms, uf, props, adj };
    // every (variable, bucket), minus one whose keys contain another's of the same bucket
    let mut alts: Vec<(V, Bucket, Vec<String>)> = Vec::new();
    // the variables in order of first appearance (structural, whatever they are named)
    let mut order: Vec<V> = Vec::new();
    {
        let mut uf = Uf(rooted.uf.0.clone());
        let mut seen = BTreeSet::new();
        for a in &rooted.atoms {
            let vs: Vec<V> = match a {
                Atom::Parent(x, y) | Atom::Eq(x, y) => vec![*x, *y],
                Atom::Kind(v, _) | Atom::NotKind(v, _) | Atom::Text(v, _) | Atom::Field(v, _) | Atom::Re(v, _) => vec![*v],
                _ => Vec::new(),
            };
            for v in vs {
                let v = uf.find(v);
                if seen.insert(v) {
                    order.push(v);
                }
            }
        }
    }
    for v in &order {
        if let Some(pr) = rooted.props.get(v) {
            for (b, keys) in buckets(pr, word_kind) {
                alts.push((*v, b, keys));
            }
        }
    }
    if alts.is_empty() {
        return Planned::NoAnchor;
    }
    let contains = |a: &[String], b: &[String]| b.iter().all(|x| a.contains(x));
    let mut kept: Vec<(V, Bucket, Vec<String>)> = Vec::new();
    for (i, a) in alts.iter().enumerate() {
        let dominated = alts.iter().enumerate().any(|(j, b)| j != i && b.1 == a.1 && contains(&a.2, &b.2) && (b.2.len() < a.2.len() || j < i));
        if !dominated {
            kept.push(a.clone());
        }
    }
    let words = cnf_of(d);
    let mut plans: Vec<SpecPlan> = Vec::new();
    for (root, bucket, keys) in kept {
        let (ops, regs) = rooted.plan(root, bucket, &keys);
        let p = SpecPlan { disjunct, bucket, keys, ops, regs, words: words.clone() };
        if !plans.iter().any(|q| q.same(&p)) {
            plans.push(p);
        }
    }
    Planned::Plans(plans)
}

/// The first half's spec of one pack, prepared in the process.
pub(super) struct Prepared {
    /// Per rule, in pack order: its name, whether it never matches, and its kinds-and-fields clauses (`k:`/`f:` names).
    pub(super) skips: Vec<(String, bool, Vec<Vec<String>>)>,
    pub(super) rules: Vec<SpecRule>,
}

/// Both analyses over every rule of the parsed pack. `word_kind`: the kind of a name token (the walk's vocabulary).
pub(super) fn prepare(programs: &PackPrograms, lang: Language) -> Prepared {
    let word_kind = lang.word_kind.clone();
    let word_kind = word_kind.as_str();
    LANGUAGE.with(|l| *l.borrow_mut() = Some(Rc::new(lang)));
    let skip = SkipAnalyzer { frags: &programs.fragments };
    let mut an = Analyzer { frags: &programs.fragments, counter: 0, pure: HashMap::new(), local: HashMap::new(), slots: Vec::new(), stamp: 0 };
    let mut skips = Vec::new();
    let mut rules = Vec::new();
    let trace = std::env::var_os("VEC_FIRST_TRACE").is_some();
    for rule in &programs.rules {
        let name = rule.rule.0.to_string();
        let tr0 = super::thread_cpu();
        let RuleProgram::Parsed(p) = &rule.program else {
            skips.push((name.clone(), false, Vec::new()));
            rules.push(SpecRule { name, node: false, plans: Vec::new(), word_cnfs: Vec::new() });
            continue;
        };
        let imports = imports_of(&p.imports);
        // the skip
        let env = skip.steps(&p.steps, SEnv::new(), &imports);
        let req = env.get(&*p.emit.step.0).map_or_else(|| Rc::new(Req::True), SRel::req);
        let cs: Vec<Set> = clauses(&req, &mut HashMap::new()).to_vec();
        let never = cs.iter().any(BTreeSet::is_empty);
        skips.push((name.clone(), never, cs.into_iter().filter(|c| !c.is_empty()).map(|c| c.into_iter().collect()).collect()));
        // the prefix plans
        an.local.clear();
        an.counter = 0;
        let tr1 = super::thread_cpu();
        let rel = an.program(&p.steps, HashMap::new(), imports.clone(), &p.emit.step.0);
        let tr2 = super::thread_cpu();
        if trace {
            eprintln!("rule {name} skip {:.4} follow {:.4}", tr1 - tr0, tr2 - tr1);
        }
        if trace {
            eprintln!("  disjuncts {}", rel.disjs.len());
        }
        let (mut plans, mut cnfs, mut noanchor) = (Vec::<SpecPlan>::new(), Vec::<Vec<Vec<String>>>::new(), 0usize);
        let mut groups: Vec<Vec<SpecPlan>> = Vec::new();
        for d in &rel.disjs {
            let got = plan_disjunct(d, word_kind, groups.len());
            if !matches!(got, Planned::Unsat) {
                let c = cnf_of(d);
                if !cnfs.contains(&c) {
                    cnfs.push(c);
                }
            }
            match got {
                Planned::NoAnchor => noanchor += 1,
                Planned::Unsat => {}
                Planned::Plans(ps) => {
                    // a disjunct whose alternatives equal an earlier one's is that one
                    let same = groups.iter().any(|g| g.len() == ps.len() && ps.iter().all(|b| g.iter().any(|a| a.same(b))));
                    if !same {
                        groups.push(ps);
                    }
                }
            }
        }
        for g in groups {
            plans.extend(g);
        }
        let node = !plans.is_empty() && noanchor == 0;
        if cnfs.iter().any(Vec::is_empty) {
            cnfs = vec![Vec::new()];
        }
        if trace {
            eprintln!("rule {name} cpu {:.4} plans {}", super::thread_cpu() - tr0, plans.len());
        }
        rules.push(SpecRule { name, node, plans: if node { plans } else { Vec::new() }, word_cnfs: cnfs });
    }
    Prepared { skips, rules }
}

/// `vec first <pack> [--request F] [--repeat K]`: the first half's spec prepared from the pack, with its CPU and size, and the
/// spec per rule (`--out F`: one line a rule: name, class, plans, disjuncts with alternatives, skip clauses).
pub(super) fn run_first(args: &[&str]) {
    let request = args.iter().position(|a| *a == "--request").map(|i| args[i + 1]);
    let repeat: usize = args.iter().position(|a| *a == "--repeat").map_or(1, |i| args[i + 1].parse().expect("repeat"));
    let read = super::lower::read_pack(std::path::Path::new(args[0]), request);
    for round in 0..repeat {
        let t0 = super::thread_cpu();
        let p = prepare(&read.programs, super::python_language());
        let t1 = super::thread_cpu();
        let plans: usize = p.rules.iter().map(|r| r.plans.len()).sum();
        println!("counts (combos, selects, atoms renamed, disjuncts renamed, fragment calls, memo hits, products past the CAP) {:?}", counts());
        println!("round {round} parse {:.4} analysis {:.4} s rules {} node {} plans {plans}", read.parse, t1 - t0, p.rules.len(), p.rules.iter().filter(|r| r.node).count());
        if round == 0
            && let Some(i) = args.iter().position(|a| *a == "--out")
        {
            let mut w = String::new();
            for (r, (_, never, cs)) in p.rules.iter().zip(&p.skips) {
                let mut ds: Vec<usize> = r.plans.iter().map(|x| x.disjunct).collect();
                ds.dedup();
                let clauses: Vec<String> = cs.iter().map(|c| c.join(",")).collect();
                w.push_str(&format!("{}\t{}\t{}\t{}\t{}{}\n", r.name, if r.node { "node" } else { "rel" }, r.plans.len(), ds.len(), if *never { "-;" } else { "" }, clauses.join(";")));
            }
            std::fs::write(args[i + 1], w).expect("out");
        }
    }
}

thread_local! {
    static COUNTS: std::cell::RefCell<[u64; 8]> = const { std::cell::RefCell::new([0; 8]) };
}
fn count(i: usize, n: u64) {
    COUNTS.with(|c| c.borrow_mut()[i] += n);
}
pub(super) fn counts() -> [u64; 8] {
    COUNTS.with(|c| *c.borrow())
}
