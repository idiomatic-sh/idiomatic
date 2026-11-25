//! Capped file prerequisites of the same lowered step as its node witnesses.
use super::{first::Language, pair_checks::AtomAnswers, pair_summary::inputs};
use rustc_hash::{FxHashMap, FxHashSet};
use ryo_values::domain::finish_plans::{expressions::types::*, types::*};
use std::{collections::BTreeSet, rc::Rc};

type Clause = Rc<BTreeSet<String>>;

/// Passthrough steps share the already ordered prerequisite list. A union or
/// an additional restriction builds a new list only when it changes the set.
#[derive(Clone, Default)]
struct Clauses(Option<Rc<Vec<Clause>>>);
impl std::ops::Deref for Clauses {
    type Target = [Clause];
    fn deref(&self) -> &[Clause] {
        self.0.as_deref().map_or(&[], Vec::as_slice)
    }
}
impl Clauses {
    fn singleton(c: Clause) -> Self {
        Self(Some(Rc::new(vec![c])))
    }
    fn include(&mut self, r: &Self) {
        let Some(right) = &r.0 else { return };
        let Some(left) = &mut self.0 else {
            self.0 = r.0.clone();
            return;
        };
        if Rc::ptr_eq(left, right)
            || (right.len() == 1 && left.iter().any(|c| Rc::ptr_eq(c, &right[0])))
        {
            return;
        }
        let output = Rc::make_mut(left);
        output.extend(right.iter().cloned());
        ordered(output);
    }
    fn push(&mut self, c: Clause) {
        let list = self.0.get_or_insert_with(|| Rc::new(Vec::new()));
        let list = Rc::make_mut(list);
        list.push(c);
        ordered(list);
    }
}
const CAP: usize = 32;

#[derive(Clone, Default)]
pub(super) struct Skip<'a> {
    scan: Option<&'a str>,
    shape: Clauses,
    words: Clauses,
}

fn ordered(v: &mut Vec<Clause>) {
    v.sort_unstable_by(|a, b| a.len().cmp(&b.len()).then_with(|| a.cmp(b)));
    v.dedup();
    v.truncate(CAP);
}
fn either(a: &Clauses, b: &Clauses) -> Clauses {
    match (a.first(), b.first()) {
        (Some(l), Some(r)) if Rc::ptr_eq(l, r) || l == r => {
            if a.len() == 1 {
                a.clone()
            } else if b.len() == 1 {
                b.clone()
            } else {
                Clauses::singleton(l.clone())
            }
        }
        (Some(l), Some(r)) => Clauses::singleton(Rc::new(l.union(r).cloned().collect())),
        _ => Clauses::default(),
    }
}

fn values<'a>(
    e: &'a PlanExpression,
    n: &str,
    alias: &SourceAlias,
    single: bool,
) -> Option<BTreeSet<&'a str>> {
    use PlanExpression::*;
    let matches = |c: &ColumnRead| {
        &*c.column.0 == n && (&c.alias == alias || (single && c.alias == SourceAlias::Unwritten))
    };
    match e {
        Compare(c) if c.comparator == Comparator::Equal => {
            let (Column(col), Literal(PlanConstant::Text(t))) = (&*c.left, &*c.right) else {
                return None;
            };
            matches(col).then(|| BTreeSet::from([&*t.0]))
        }
        In(p) => {
            let (Column(c), List(es)) = (&*p.left, &*p.right) else {
                return None;
            };
            if !matches(c) {
                return None;
            }
            es.iter()
                .map(|e| {
                    if let Literal(PlanConstant::Text(t)) = e {
                        Some(&*t.0)
                    } else {
                        None
                    }
                })
                .collect()
        }
        Or(p) => {
            let mut a = values(&p.left, n, alias, single)?;
            a.extend(values(&p.right, n, alias, single)?);
            Some(a)
        }
        And(p) => match (
            values(&p.left, n, alias, single),
            values(&p.right, n, alias, single),
        ) {
            (Some(l), Some(r)) => Some(l.intersection(&r).copied().collect()),
            (l, r) => l.or(r),
        },
        _ => None,
    }
}

impl<'a> Skip<'a> {
    fn filter(
        &mut self,
        base: &str,
        term: &PlanExpression,
        alias: &SourceAlias,
        single: bool,
        lang: &Language,
    ) {
        let tree = super::first::SYNTAX.contains(&base);
        let column = if tree {
            Some(("kind", "k:"))
        } else if super::first::CHILD.contains(&base) {
            Some(("field", "f:"))
        } else {
            None
        };
        if let Some((name, prefix)) = column
            && let Some(v) = values(term, name, alias, single)
        {
            let mut set: BTreeSet<_> = v.iter().map(|x| format!("{prefix}{x}")).collect();
            if let Some(earlier) = self.shape.first() {
                set.retain(|x| earlier.contains(x))
            }
            self.shape = Clauses::singleton(Rc::new(set));
        }
        let mut add = |name: &str, sep: Option<char>| {
            if let Some(v) = values(term, name, alias, single) {
                let words: BTreeSet<_> = v
                    .into_iter()
                    .map(|t| match sep {
                        Some(c) => t.rsplit(c).next().unwrap_or(t),
                        None => t,
                    })
                    .collect();
                if !words.is_empty() && words.iter().all(|w| (lang.is_identifier)(w)) {
                    self.words
                        .push(Rc::new(words.into_iter().map(str::to_owned).collect()));
                }
            }
        };
        if tree {
            add("text", None)
        } else {
            for (r, c, sep) in &lang.spelled {
                if r == base {
                    add(c, *sep);
                }
            }
        }
    }
    fn take(&mut self, r: &Self) {
        self.shape.include(&r.shape);
        self.words.include(&r.words);
    }
    pub(super) fn with_terms(
        s: &'a FinishStep,
        previous: &[Self],
        lang: &Language,
        terms: &[&PlanExpression],
    ) -> Self {
        use FinishStep::*;
        let mut out = Self::default();
        match s {
            Base(n) => out.scan = Some(&*n.0),
            Union(u) => {
                let (l, r) = (&previous[u.left.0], &previous[u.right.0]);
                out.shape = either(&l.shape, &r.shape);
                out.words = either(&l.words, &r.words);
            }
            Antijoin(j) => out.take(&previous[j.left.0]),
            OrderedDfs(x) => out.take(&previous[x.from.0]),
            SubtreeClass(x) => out.take(&previous[x.from.0]),
            GroupCount(g) if g.keys.is_empty() => {}
            Unlisted(_) => {}
            Select(q) => {
                let single = q.sources.len() == 1;
                for src in &q.sources {
                    let mut scan = previous[src.step.0].clone();
                    if let Some(base) = scan.scan {
                        for term in terms {
                            scan.filter(base, term, &src.alias, single, lang);
                        }
                    }
                    if single {
                        // A projection can reuse a base-column name for a new
                        // value. Later filters then apply to the projected value.
                        out.scan = scan.scan.filter(|base| match &q.columns {
                            SelectedColumns::All => true,
                            SelectedColumns::Items(items) => items.iter().all(|item| {
                                let name = &*item.name.0;
                                let tested = (super::first::SYNTAX.contains(base) && matches!(name, "kind" | "text"))
                                    || (super::first::CHILD.contains(base) && name == "field")
                                    || lang.spelled.iter().any(|(r, c, _)| r == *base && c == name);
                                !tested || matches!(&item.value, PlanExpression::Column(c) if c.column == item.name)
                            }),
                        });
                    }
                    out.take(&scan);
                }
            }
            _ => {
                for p in inputs(s) {
                    out.take(&previous[p.0]);
                }
            }
        }
        out
    }

    pub(super) fn root(&self) -> (bool, Vec<Vec<String>>, Vec<Vec<String>>) {
        let lists = |v: &[Clause]| {
            v.iter()
                .filter(|c| !c.is_empty())
                .map(|c| c.iter().cloned().collect())
                .collect()
        };
        (
            self.shape.iter().any(|c| c.is_empty()),
            lists(&self.shape),
            lists(&self.words),
        )
    }
}

/// Which of the provider's file-level projections a prerequisite reads.
#[derive(Clone, Copy, Default, Eq, Hash, PartialEq)]
pub(super) struct Demand {
    pub kinds: bool,
    pub fields: bool,
    pub words: bool,
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
enum Shape {
    Kind(u32),
    Field(u32),
}

#[derive(Eq, Hash, PartialEq)]
struct BoundClause<T> {
    id: usize,
    terms: Rc<[T]>,
}

/// The legacy file skip interned its clauses for a whole pack. The reader
/// can add more prepared programs while those same clause IDs remain live.
#[derive(Default)]
pub(super) struct PrerequisiteClauses {
    shape: FxHashMap<Rc<[Shape]>, usize>,
    words: FxHashMap<Rc<[u32]>, usize>,
}

impl PrerequisiteClauses {
    fn intern<T: Eq + std::hash::Hash>(
        seen: &mut FxHashMap<Rc<[T]>, usize>,
        terms: Vec<T>,
        next: usize,
    ) -> BoundClause<T> {
        if let Some((stored, &id)) = seen.get_key_value(terms.as_slice()) {
            return BoundClause {
                id,
                terms: stored.clone(),
            };
        }
        let terms: Rc<[T]> = terms.into();
        seen.insert(terms.clone(), next);
        BoundClause { id: next, terms }
    }

    fn shape(&mut self, terms: Vec<Shape>) -> BoundClause<Shape> {
        let next = self.shape.len() + self.words.len();
        Self::intern(&mut self.shape, terms, next)
    }

    fn words(&mut self, terms: Vec<u32>) -> BoundClause<u32> {
        let next = self.shape.len() + self.words.len();
        Self::intern(&mut self.words, terms, next)
    }
}

/// Prepared once with the reader's symbol dictionary, shared by its bindings.
#[derive(Eq, Hash, PartialEq)]
pub(super) struct BoundSummary {
    never: bool,
    shape: Vec<BoundClause<Shape>>,
    words: Vec<BoundClause<u32>>,
    demand: Demand,
}

impl BoundSummary {
    pub(super) fn bind(
        (never, shape, words): &(bool, Vec<Vec<String>>, Vec<Vec<String>>),
        clauses: &mut PrerequisiteClauses,
        mut intern: impl FnMut(&[u8]) -> u32,
    ) -> Self {
        let mut demand = Demand {
            words: !words.is_empty(),
            ..Demand::default()
        };
        let shape = shape
            .iter()
            .map(|clause| {
                let terms = clause
                    .iter()
                    .map(|term| {
                        let (kind, value) = term.split_once(':').expect("shape term");
                        let symbol = intern(value.as_bytes());
                        match kind {
                            "k" => {
                                demand.kinds = true;
                                Shape::Kind(symbol)
                            }
                            "f" => {
                                demand.fields = true;
                                Shape::Field(symbol)
                            }
                            _ => unreachable!("undeclared shape term"),
                        }
                    })
                    .collect();
                clauses.shape(terms)
            })
            .collect();
        let words = words
            .iter()
            .map(|clause| {
                clauses.words(clause.iter().map(|word| intern(word.as_bytes())).collect())
            })
            .collect();
        Self {
            never: *never,
            shape,
            words,
            demand,
        }
    }

    pub(super) fn demand(&self) -> Demand {
        if self.never {
            Demand::default()
        } else {
            self.demand
        }
    }
}

/// Missing output slots are already handled, or have no live consumer.
pub(super) struct Projection<'a> {
    pub kinds: Option<&'a mut FxHashSet<u32>>,
    pub fields: Option<&'a mut FxHashSet<u32>>,
    pub words: Option<&'a mut FxHashSet<u32>>,
}

impl Projection<'_> {
    /// A supplied column covers the numbered tree. The provider can append
    /// additional positions to that same projection before it becomes complete.
    pub(super) fn populate(
        mut self,
        tree: &ryo::languages::NodeTable,
        run: &mut super::finish::Run,
        numbered: Demand,
        words: impl FnOnce(&mut FxHashSet<u32>),
        expand: impl FnOnce(&mut super::finish::Run, Projection<'_>),
    ) -> Demand {
        use ryo::languages::node_table;
        if let Some(set) = self.kinds.as_deref_mut().filter(|_| numbered.kinds) {
            for (group, kinds) in node_table::kinds_present(tree).0.iter().enumerate() {
                let mut word = kinds.0;
                while word != 0 {
                    let code = group * 64 + word.trailing_zeros() as usize;
                    set.insert(run.kind_sym[code]);
                    word &= word - 1;
                }
            }
        } else {
            self.kinds = None;
        }
        if let Some(set) = self.fields.as_deref_mut().filter(|_| numbered.fields) {
            project_fields(tree, &run.field_sym, set);
        } else {
            self.fields = None;
        }
        if let Some(set) = self.words.as_deref_mut().filter(|_| numbered.words) {
            set.insert(run.empty);
            words(set);
        } else {
            self.words = None;
        }
        let complete = Demand {
            kinds: self.kinds.is_some(),
            fields: self.fields.is_some(),
            words: self.words.is_some(),
        };
        if complete.kinds || complete.fields || complete.words {
            expand(run, self);
        }
        complete
    }
}

/// File version membership. A provider explicitly marks complete projections.
/// The scheduler resets this when the numbered projection changes.
#[derive(Default)]
pub(super) struct FileSummary {
    asked: Demand,
    complete: Demand,
    answers: AtomAnswers,
    clauses: AtomAnswers,
    kinds: FxHashSet<u32>,
    fields: FxHashSet<u32>,
    words: FxHashSet<u32>,
}

impl FileSummary {
    pub(super) fn reset(&mut self) {
        self.asked = Demand::default();
        self.complete = Demand::default();
        self.answers.clear();
        self.clauses.clear();
        self.kinds.clear();
        self.fields.clear();
        self.words.clear();
    }

    /// `id` is the reader's identity for an equivalent bound prerequisite.
    /// The surrounding pair decision still owns coverage and applicability.
    pub(super) fn excludes(
        &mut self,
        id: usize,
        bound: &BoundSummary,
        mut project: impl FnMut(Projection<'_>) -> Demand,
    ) -> bool {
        if let Some(answer) = self.answers.get(id) {
            return answer;
        }
        let excluded = bound.never || self.missing(bound, &mut project);
        self.answers.set(id, excluded);
        excluded
    }

    fn missing(
        &mut self,
        bound: &BoundSummary,
        project: &mut impl FnMut(Projection<'_>) -> Demand,
    ) -> bool {
        let kinds = bound.demand.kinds && !self.asked.kinds;
        let fields = bound.demand.fields && !self.asked.fields;
        if kinds || fields {
            let complete = project(Projection {
                kinds: kinds.then_some(&mut self.kinds),
                fields: fields.then_some(&mut self.fields),
                words: None,
            });
            self.asked.kinds |= kinds;
            self.asked.fields |= fields;
            self.complete.kinds |= kinds && complete.kinds;
            self.complete.fields |= fields && complete.fields;
        }
        if bound.shape.iter().any(|clause| {
            clause_missing(&mut self.clauses, clause, |term| match *term {
                Shape::Kind(id) => self.complete.kinds && !self.kinds.contains(&id),
                Shape::Field(id) => self.complete.fields && !self.fields.contains(&id),
            })
        }) {
            return true;
        }
        if bound.demand.words && !self.asked.words {
            self.complete.words = project(Projection {
                kinds: None,
                fields: None,
                words: Some(&mut self.words),
            })
            .words;
            self.asked.words = true;
        }
        self.complete.words
            && bound.words.iter().any(|clause| {
                clause_missing(&mut self.clauses, clause, |id| !self.words.contains(id))
            })
    }
}

/// A file's clause is tested once. The memo holds both negative and positive
/// decisions, like the native file skip's held mask and stamped word memo.
fn clause_missing<T>(
    answers: &mut AtomAnswers,
    clause: &BoundClause<T>,
    absent: impl FnMut(&T) -> bool,
) -> bool {
    if let Some(value) = answers.get(clause.id) {
        return value;
    }
    let missing = clause.terms.iter().all(absent);
    answers.set(clause.id, missing);
    missing
}

/// The numbered visit already grouped positions by field, including field zero.
fn project_fields(tree: &ryo::languages::NodeTable, names: &[u32], into: &mut FxHashSet<u32>) {
    use ryo::languages::{FieldCode, node_table};
    for (code, &name) in names.iter().enumerate() {
        let Ok(code) = u16::try_from(code) else { break };
        if !node_table::ids_of_field(tree, FieldCode(code)).is_empty() {
            into.insert(name);
        }
    }
}

