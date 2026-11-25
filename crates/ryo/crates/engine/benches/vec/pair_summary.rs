//! Necessary witnesses of a row, shared by step identity in the lowered plan.
//! A node domain overapproximates one column. Joining two domains intersects them
//! only when the query equates their node ids. File conditions can have independent witnesses.
use super::{Vocab, first::Language};
use rustc_hash::FxHashMap as Map;
use rustc_hash::FxHashSet;
use ryo::languages::{CellKind, Relation};
use ryo_relations::domain::{labels::types::NameRef, relations::service as catalog};
use ryo_values::domain::finish_plans::{
    arguments::types::StepNumber, expressions::types::*, service::extend_inputs, types::*,
};
use ryo_values::domain::plan_parts::types::Direction;
use std::borrow::Cow;

#[path = "pair_compact.rs"]
mod compact;
#[path = "pair_cost.rs"]
pub(super) mod cost;
#[path = "pair_exact.rs"]
mod exact;
pub(super) use exact::LocationColumn;
#[path = "pair_groups.rs"]
mod groups;
#[path = "pair_patterns.rs"]
mod patterns;
#[path = "pair_paths.rs"]
pub(super) mod paths;
#[path = "pair_precheck.rs"]
mod precheck;
#[path = "pair_project.rs"]
mod project;
#[path = "pair_regexes.rs"]
pub(super) mod regexes;

pub(super) fn inputs(s: &FinishStep) -> smallvec::SmallVec<[StepNumber; 4]> {
    let mut result = smallvec::SmallVec::new();
    extend_inputs(s, &mut result);
    result
}

type Literals<'a> = smallvec::SmallVec<[&'a str; 4]>;
pub(super) type Conjuncts<'a> = smallvec::SmallVec<[&'a PlanExpression; 4]>;

pub(super) type Id = usize;
const ANY: Id = 0;
const EMPTY: Id = 1;
const CAP: usize = 32;

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub(super) enum Domain {
    Any,
    Empty,
    HasParent,
    CountUnder(Id, u8, Comparator, i64),
    Kind(u16),
    Field(u16),
    Text(String),
    TextMatching(u16, usize),
    SpanMatching(u16, usize),
    Fact(usize, Option<(usize, Vec<Id>)>),
    SameRow(usize, Vec<CellTest>),
    OtherValues(usize, usize, Vec<Id>),
    Flag(usize, usize, bool),
    Number(usize, usize, Comparator, i64),
    RelatedIds(usize, usize, Option<(usize, Vec<Id>)>),
    Matching(usize, usize, usize),
    Unmatched(usize, usize, usize),
    Member(usize, usize, usize, usize),
    CapturedSuffix(usize, usize, Vec<u8>, usize),
    When(Id, Id), // file prerequisite, admitted nodes
    Except(Id, Id),
    ProvenExcept(Id, Id, Id, Vec<usize>), // left, difference, possible guard, required tables
    Both(Id, Id),
    Either(Id, Id),
    Parents(Id),
    Children(Id),
    Ancestors(Id),
    Descendants(Id),
}
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub(super) enum Need {
    Terms(bool, Box<[Id]>),
    Any,
    PathMatches(usize, bool),
    PathGlob(usize),
    CapturedPath(patterns::CapturePath),
    SyntaxMatches(usize, bool),
    InvalidPattern(usize), // proof prerequisite, resolved during plan compaction
    SourceContains(usize),
    Empty,
    Some(Id),
    Both(Id, Id),
    Either(Id, Id),
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub(super) enum CellTest {
    Symbols(usize, Vec<Id>),
    Numeric(usize, Comparator, i64),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Source {
    Catalog(Relation),
    Walk(&'static str),
}

pub(super) struct Fact {
    pub family: cost::Family,
    pub source: Source,
    pub name: &'static str,
    pub key: Option<usize>,
    pub columns: std::rc::Rc<[&'static str]>,
    pub kinds: std::rc::Rc<[CellKind]>,
    pub exact: bool,
}

pub(super) struct Program {
    pub scratch: std::cell::RefCell<super::pair_checks::Scratch>,
    pub meter: Option<cost::Meter>,
    pub paths: Vec<Option<paths::Edge>>,
    pub file_only: Vec<bool>,
    pub domains: Vec<Domain>,
    pub needs: Vec<Need>,
    pub exact: Vec<(usize, exact::Certificate)>,
    pub roots: Vec<(String, Id)>,
    pub skips: Vec<(bool, Vec<Vec<String>>, Vec<Vec<String>>)>,
    pub regexes: Vec<regexes::Pattern>,
    pub path_globs: Vec<String>,
    pub text_kinds: Vec<u16>,
    pub symbols: Map<String, Id>,
    pub max_text: usize,
    pub texts: FxHashSet<Vec<u8>>,
    pub facts: Vec<Fact>,
    pub capped: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Attribute {
    Id,
    FilePath,
    Span,
    Kind,
    Field,
    Text,
    Cell(usize, usize),
    ProjectChoices(usize, u64),
    SymbolOrigins(usize),
    Count(usize),
}
#[derive(Clone, Copy)]
struct Column {
    var: usize,
    attr: Attribute,
}
#[derive(Clone, Default)]
struct Summary<'a> {
    need: Id,
    native_row: Option<usize>,
    domains: smallvec::SmallVec<[Id; 2]>,
    columns: Cow<'a, [(&'a str, Column)]>,
    edges: smallvec::SmallVec<[(usize, usize, u8); 1]>, // descendant, ancestor, fixed number of parent edges
}
impl<'a> Summary<'a> {
    fn writing(scratch: &mut Vec<(&'a str, Column)>) -> Self {
        Self {
            columns: Cow::Owned(std::mem::take(scratch)),
            ..Self::default()
        }
    }
    // Stored columns are immutable throughout the bottom-up pass. Passthrough
    // steps borrow that slice. A new projection writes into reusable storage.
    fn freeze(&mut self, arena: &'a bumpalo::Bump, scratch: &mut Vec<(&'a str, Column)>) {
        if let Cow::Owned(buf) = &mut self.columns {
            let stored = arena.alloc_slice_copy(buf);
            if buf.capacity() > scratch.capacity() {
                buf.clear();
                std::mem::swap(buf, scratch);
            }
            self.columns = Cow::Borrowed(stored);
        }
    }
    fn retain_columns(&mut self, keep: impl Fn(&(&str, Column)) -> bool) {
        if let Cow::Borrowed(cols) = &self.columns
            && cols.iter().all(&keep)
        {
            return;
        }
        self.columns.to_mut().retain(keep);
    }
    fn column(&self, name: &str) -> Option<Column> {
        self.columns
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, c)| *c)
    }
    fn var(&mut self, domain: Id) -> usize {
        let i = self.domains.len();
        self.domains.push(domain);
        i
    }
    fn add(&mut self, name: &'a str, var: usize, attr: Attribute) {
        self.columns.to_mut().push((name, Column { var, attr }));
    }
    fn compact(&mut self) {
        if self.domains.len() <= 1 {
            if self.columns.is_empty() {
                self.domains.clear();
            }
            self.edges.clear();
            return;
        }
        let mut vars = smallvec::SmallVec::new();
        let mut map = smallvec::SmallVec::<[Option<usize>; 8]>::from_elem(None, self.domains.len());
        for (_, c) in self.columns.to_mut() {
            c.var = *map[c.var].get_or_insert_with(|| {
                vars.push(self.domains[c.var]);
                vars.len() - 1
            });
        }
        self.domains = vars;
        // A projection may discard the intermediate node while retaining both
        // ends of a path. Parent is functional. Following one known path per
        // edge suffices even with several equivalent names for a parent.
        let old = std::mem::take(&mut self.edges);
        for &(child, mut parent, mut depth) in &old {
            let Some(from) = map[child] else { continue };
            loop {
                if let Some(to) = map[parent] {
                    if from != to {
                        self.edges.push((from, to, depth));
                    }
                    break;
                }
                let Some(&(_, next, extra)) = old.iter().find(|(c, _, _)| *c == parent) else {
                    break;
                };
                depth += extra;
                if usize::from(depth) > CAP {
                    break;
                }
                parent = next;
            }
        }
        self.edges.sort_unstable();
        self.edges.dedup();
    }
}

struct Builder<'a> {
    vocab: &'a Vocab,
    language: &'a Language,
    reader_rows: &'a [String],
    current_file: Option<(&'static str, &'static str)>,
    syntax_symbols: bool,
    positive: bool,
    regex_slots: Map<String, Id>,
    glob_slots: Map<Vec<String>, Id>,
    symbol_origins: Vec<Vec<(usize, usize)>>,
    proof_validation: Option<Id>,
    early_omit: u16,
    signatures: Vec<u64>,
    out: Program,
    ds: Map<Domain, Id>,
    ns: Map<Need, Id>,
    sizes: Vec<usize>,
    visited: Vec<usize>,
    generation: usize,
    file_guards: Vec<Option<Id>>,
    exists: Vec<Option<Id>>,
    counts: Map<usize, exact::Grouped>,
}

impl Builder<'_> {
    fn dependencies(d: &Domain) -> &[Id] {
        match d {
            Domain::CountUnder(x, ..)
            | Domain::Parents(x)
            | Domain::Children(x)
            | Domain::Ancestors(x)
            | Domain::Descendants(x)
            | Domain::When(_, x)
            | Domain::ProvenExcept(_, x, _, _) => std::slice::from_ref(x),
            _ => &[],
        }
    }
    fn distinct_size(&mut self, root: &Domain) -> usize {
        // Only visits to this domain contribute to its distinct-instruction cap.
        self.generation += 1;
        let stamp = self.generation;
        let mut used = 0;
        let mut pending = smallvec::SmallVec::<[Id; 32]>::new();
        let mut current = root;
        loop {
            match current {
                Domain::Both(a, b) | Domain::Either(a, b) | Domain::Except(a, b) => {
                    pending.extend([*a, *b])
                }
                _ => pending.extend_from_slice(Self::dependencies(current)),
            }
            let Some(next) =
                std::iter::from_fn(|| pending.pop()).find(|&v| self.visited[v] != stamp)
            else {
                break;
            };
            self.visited[next] = stamp;
            used += 1;
            if used == CAP {
                break;
            }
            current = &self.out.domains[next];
        }
        1 + used
    }
    fn domain(&mut self, d: Domain) -> Id {
        if self.early_omit != 0 && cost::omit_positive(&d, &self.out.facts, self.early_omit) {
            return ANY;
        }
        if let Some(&i) = self.ds.get(&d) {
            if i == ANY && !matches!(d, Domain::Any) {
                self.out.capped += 1;
            }
            return i;
        }
        self.insert_domain(d)
    }
    fn insert_domain(&mut self, d: Domain) -> Id {
        let size = match d {
            Domain::Except(a, b) | Domain::Both(a, b) | Domain::Either(a, b) => {
                1 + self.sizes[a] + self.sizes[b]
            }
            Domain::CountUnder(x, ..)
            | Domain::Parents(x)
            | Domain::Children(x)
            | Domain::Ancestors(x)
            | Domain::Descendants(x) => 1 + self.sizes[x],
            Domain::ProvenExcept(_, difference, _, _) => 1 + self.sizes[difference],
            Domain::When(_, x) => 1 + self.sizes[x],
            _ => 1,
        };
        let size = if size > CAP {
            self.distinct_size(&d)
        } else {
            size
        };
        if size > CAP {
            self.out.capped += 1;
            self.ds.insert(d, ANY);
            return ANY;
        }
        let i = self.out.domains.len();
        self.ds.insert(d.clone(), i);
        self.out.domains.push(d);
        self.sizes.push(size);
        self.visited.push(0);
        i
    }
    fn need(&mut self, n: Need) -> Id {
        if let Need::Some(node) = n {
            if self.exists.len() <= node {
                self.exists.resize(node + 1, None);
            }
            return *self.exists[node].get_or_insert_with(|| {
                let id = self.out.needs.len();
                self.out.needs.push(n);
                self.signatures.push(1 << (id % 64));
                id
            });
        }
        if let Some(&i) = self.ns.get(&n) {
            return i;
        }
        self.insert_need(n)
    }
    fn insert_need(&mut self, n: Need) -> Id {
        let i = self.out.needs.len();
        let mut signature = 1 << (i % 64);
        if let Need::Both(a, b) = n {
            signature |= self.signatures[a] | self.signatures[b];
        }
        self.signatures.push(signature);
        self.ns.insert(n.clone(), i);
        self.out.needs.push(n);
        i
    }
    // Keep repeated constraints once before charging the formula cap. Only
    // positive, monotone implications are used here. No alternatives expand.
    fn subset(&self, small: Id, large: Id) -> bool {
        let mut budget = CAP;
        self.implies(small, large, &mut budget)
    }
    fn implies(&self, small: Id, large: Id, budget: &mut usize) -> bool {
        if small == large || small == EMPTY || large == ANY {
            return true;
        }
        if *budget == 0 {
            return false;
        }
        *budget -= 1;
        if let Domain::Both(a, b) = self.out.domains[large] {
            return self.implies(small, a, budget) && self.implies(small, b, budget);
        }
        match (&self.out.domains[small], &self.out.domains[large]) {
            (Domain::Both(a, b), _) => {
                self.implies(*a, large, budget) || self.implies(*b, large, budget)
            }
            (Domain::Except(a, _), _) => self.implies(*a, large, budget),
            (Domain::TextMatching(k, _), Domain::Kind(t)) => k == t,
            (Domain::Children(a), Domain::Children(b))
            | (Domain::Parents(a), Domain::Parents(b)) => self.implies(*a, *b, budget),
            _ => false,
        }
    }
    fn both(&mut self, a: Id, b: Id) -> Id {
        if a == EMPTY || b == EMPTY {
            return EMPTY;
        }
        if a == ANY || a == b {
            return b;
        }
        if b == ANY {
            return a;
        }
        let key = Domain::Both(a.min(b), a.max(b));
        if let Some(&id) = self.ds.get(&key) {
            if id == ANY {
                self.out.capped += 1;
            }
            return if id == ANY { a } else { id };
        }
        if self.subset(a, b) {
            self.ds.insert(key, a);
            return a;
        }
        if self.subset(b, a) {
            self.ds.insert(key, b);
            return b;
        }
        let combined = self.insert_domain(key);
        // The additional conjunct is unrepresentable at the cap. The
        // previous witness is still necessary; exact issuance sees the cap.
        if combined == ANY { a } else { combined }
    }
    fn either(&mut self, a: Id, b: Id) -> Id {
        if a == ANY || b == ANY {
            return ANY;
        }
        if a == EMPTY || a == b {
            return b;
        }
        if b == EMPTY {
            return a;
        }
        let key = Domain::Either(a.min(b), a.max(b));
        if let Some(&id) = self.ds.get(&key) {
            if id == ANY {
                self.out.capped += 1;
            }
            return id;
        }
        if self.subset(a, b) {
            self.ds.insert(key, b);
            return b;
        }
        if self.subset(b, a) {
            self.ds.insert(key, a);
            return a;
        }
        self.insert_domain(key)
    }
    // A conjunction entails every conjunct reachable solely through AND.
    // The signature only disproves membership. Elimination requires an actual
    // instruction id found by the bounded search, independent of file data.
    fn entails(&self, root: Id, target: Id, remaining: &mut usize) -> bool {
        if root == target {
            return true;
        }
        if root < target || self.signatures[root] & (1 << (target % 64)) == 0 || *remaining == 0 {
            return false;
        }
        let Need::Both(a, b) = self.out.needs[root] else {
            return false;
        };
        *remaining -= 1;
        self.entails(a, target, remaining) || self.entails(b, target, remaining)
    }
    fn and(&mut self, a: Id, b: Id) -> Id {
        if a == EMPTY || b == EMPTY {
            return EMPTY;
        }
        if a == ANY || a == b {
            return b;
        }
        if b == ANY {
            return a;
        }
        if matches!(self.out.needs[a],Need::Both(x,y) if x==b||y==b) {
            return a;
        }
        if matches!(self.out.needs[b],Need::Both(x,y) if x==a||y==a) {
            return b;
        }
        let (lo, hi) = (a.min(b), a.max(b));
        let both = Need::Both(lo, hi);
        if let Some(&id) = self.ns.get(&both) {
            return id;
        }
        let mut remaining = CAP;
        if self.entails(hi, lo, &mut remaining) {
            self.ns.insert(both, hi);
            return hi;
        }
        self.insert_need(both)
    }
    fn or(&mut self, a: Id, b: Id) -> Id {
        if a == ANY || b == ANY {
            return ANY;
        }
        if a == EMPTY || a == b {
            return b;
        }
        if b == EMPTY {
            return a;
        }
        self.need(Need::Either(a.min(b), a.max(b)))
    }
    fn require(&mut self, r: &mut Summary<'_>, d: Id) {
        if matches!(self.out.domains[d], Domain::RelatedIds(_, _, None)) {
            // The base already requires a row. Read this endpoint only when
            // a join or a predicate constrains its node set.
            return;
        }
        let n = if d <= EMPTY {
            d
        } else {
            self.need(Need::Some(d))
        };
        r.need = self.and(r.need, n);
    }
    fn native(&self, name: &str) -> bool {
        !self.reader_rows.iter().any(|external| external == name)
    }
    fn base<'a>(&mut self, name: &str) -> Summary<'a> {
        let mut r = Summary::default();
        if (self.language.host_len)(name) == Some(0) {
            r.need = EMPTY;
            return r;
        }
        use Attribute::*;
        if let Some((relation, column)) = self.current_file && relation == name {
            let file = r.var(ANY);
            r.add(column, file, FilePath);
            return r;
        }
        match name {
            "syntax-tree" | "semantic-syntax" if self.native(name) => {
                r.var(ANY);
                r.add("id", 0, Id);
                r.add("span", 0, Span);
                r.add("file", 0, FilePath);
                r.add("kind", 0, Kind);
                r.add("text", 0, Text);
            }
            "syntax-child" | "semantic-child" if self.native(name) => {
                r.var(ANY);
                r.var(ANY);
                r.add("child", 0, Id);
                r.add("file", 0, FilePath);
                r.add("field", 0, Field);
                r.add("parent", 1, Id);
                r.edges.push((0, 1, 1));
            }
            "source-location" if self.native(name) => {
                r.var(ANY);
                r.add("id", 0, Id);
                r.add("span", 0, Span);
                r.add("file", 0, FilePath);
            }
            _ => {
                let slot =
                    if let Some(slot) = self.out.facts.iter().position(|spec| spec.name == name) {
                        slot
                    } else {
                        let (name, source, columns, kinds, exact): (_, _, Vec<_>, Vec<_>, _) =
                            if let Some(view) = self
                                .language
                                .walk_facts
                                .iter()
                                .find(|f| f.name == name)
                            {
                                let (columns, kinds) = view.columns.iter().copied().unzip();
                                (
                                    view.name,
                                    Source::Walk(view.name),
                                    columns,
                                    kinds,
                                    view.exact,
                                )
                            } else {
                                if !self.language.facts.iter().any(|n| n == name)
                                    && !self.language.file_sets.iter().any(|n| n == name)
                                {
                                    return r;
                                }
                                let Some(rel) = catalog::relation_named(NameRef(name)) else {
                                    return r;
                                };
                                let columns = catalog::columns(rel)
                                    .into_iter()
                                    .map(|c| catalog::column_name(c).0)
                                    .collect();
                                (
                                    catalog::name(rel).0,
                                    Source::Catalog(rel),
                                    columns,
                                    catalog::kinds(rel).to_vec(),
                                    true,
                                )
                            };
                        let key = kinds.iter().position(|k| *k == CellKind::Node);
                        self.out.facts.push(Fact {
                            family: (self.language.fact_family)(name),
                            name,
                            source,
                            key,
                            columns: columns.into(),
                            kinds: kinds.into(),
                            exact,
                        });
                        self.out.facts.len() - 1
                    };
                let key = self.out.facts[slot].key;
                let d = self.domain(Domain::Fact(slot, None));
                r.native_row = Some(slot);
                r.var(d);
                self.require(&mut r, d);
                for i in 0..self.out.facts[slot].columns.len() {
                    let col = self.out.facts[slot].columns[i];
                    let (var, attr) = if Some(i) == key {
                        (0, Id)
                    } else if self.out.facts[slot].kinds[i] == CellKind::Node {
                        let ids = self.domain(Domain::RelatedIds(slot, i, None));
                        (r.var(ids), Id)
                    } else {
                        (0, Cell(slot, i))
                    };
                    r.add(col, var, attr);
                }
            }
        }
        r
    }

    fn symbol(&mut self, text: &str) -> Id {
        if let Some(&n) = self.out.symbols.get(text) {
            return n;
        }
        let n = self.out.symbols.len();
        self.out.symbols.insert(text.to_owned(), n);
        n
    }

    fn literal(&mut self, attr: Attribute, text: &str) -> Id {
        let d = match attr {
            Attribute::Kind => self.vocab.kinds.get(text).map(|k| Domain::Kind(*k)),
            Attribute::Field => self.vocab.fields.get(text).map(|k| Domain::Field(*k)),
            // Empty text can also be the provider's value for a node without text.
            Attribute::Text if !text.is_empty() => Some(Domain::Text(text.to_owned())),
            Attribute::Cell(slot, c) if self.out.facts[slot].kinds[c] == CellKind::Flag => {
                Some(Domain::Flag(
                    slot,
                    c,
                    match text {
                        "true" => true,
                        "false" => false,
                        _ => return ANY,
                    },
                ))
            }
            Attribute::Cell(slot, c) if self.out.facts[slot].kinds[c] == CellKind::Symbol => {
                Some(Domain::Fact(slot, Some((c, vec![self.symbol(text)]))))
            }
            _ => return ANY,
        };
        // An unrecognised vocabulary name may denote the provider's absent value.
        d.map_or(ANY, |x| self.domain(x))
    }

    fn matching(&mut self, attr: Attribute, pattern: &str, kind: CellKind) -> Option<Id> {
        if let Attribute::ProjectChoices(slot, cols) = attr {
            let mut domain = EMPTY;
            for col in project::columns(cols) {
                let next = self.matching(Attribute::Cell(slot, col), pattern, kind)?;
                domain = self.either(domain, next);
            }
            return Some(domain);
        }
        let Attribute::Cell(slot, col) = attr else {
            return None;
        };
        if self.out.facts[slot].kinds[col] != kind {
            return None;
        }
        let regex = self.regex(pattern)?;
        Some(self.domain(Domain::Matching(slot, col, regex)))
    }

    fn take_validation(&mut self) -> Id {
        self.proof_validation
            .as_mut()
            .map_or(EMPTY, |v| std::mem::replace(v, EMPTY))
    }
    fn regex(&mut self, pattern: &str) -> Option<usize> {
        let slot = if let Some(&slot) = self.regex_slots.get(pattern) {
            slot
        } else {
            let slot = self.out.regexes.len();
            self.out.regexes.push(regexes::Pattern::new(pattern));
            self.regex_slots.insert(pattern.to_owned(), slot);
            slot
        };
        if let Some(guard) = self.proof_validation {
            let invalid = self.need(Need::InvalidPattern(slot));
            self.proof_validation = Some(self.or(guard, invalid));
            return Some(slot);
        }
        (self.positive || self.out.regexes[slot].compiled().is_some()).then_some(slot)
    }

    fn required_kind(&self, domain: Id) -> Option<u16> {
        match self.out.domains[domain] {
            Domain::Kind(k) => Some(k),
            Domain::Both(a, b) => self.required_kind(a).or_else(|| self.required_kind(b)),
            _ => None,
        }
    }
    fn number(&mut self, attr: Attribute, cmp: Comparator, value: i64) -> Option<Id> {
        if let Attribute::Count(step) = attr {
            let g = self.counts.get(&step)?;
            let (nodes, hops, guard, facts) = (g.rows, g.depth, g.guard, g.facts.clone());
            let d = self.domain(Domain::CountUnder(nodes, hops, cmp, value));
            return Some(if d == ANY || (guard == EMPTY && facts.is_empty()) {
                d
            } else {
                self.domain(Domain::ProvenExcept(ANY, d, guard, facts))
            });
        }
        let Attribute::Cell(slot, col) = attr else {
            return None;
        };
        (self.out.facts[slot].kinds[col] == CellKind::Number)
            .then(|| self.domain(Domain::Number(slot, col, cmp, value)))
    }
    // Intersect cells in the input row before projecting the node key. One
    // node can have several rows, each satisfying a different conjunct.
    fn same_row(
        &mut self,
        slot: usize,
        terms: &[&PlanExpression],
        lookup: &impl Fn(&ColumnRead) -> Option<Column>,
    ) -> Option<(usize, Id)> {
        if !(2..=CAP).contains(&terms.len()) {
            return None;
        }
        let mut tests = Vec::new();
        let mut anchor = None;
        let mut size = terms.len();
        for term in terms {
            let test = if let Some((c, cmp, literal)) = numeric_comparison(term) {
                let col = lookup(c)?;
                let Attribute::Cell(source, index) = col.attr else {
                    return None;
                };
                if source != slot
                    || self.out.facts[slot].kinds[index] != CellKind::Number
                    || *anchor.get_or_insert(col.var) != col.var
                {
                    return None;
                }
                CellTest::Numeric(index, cmp, literal)
            } else {
                let mut values = Literals::new();
                let col = lookup(choices(term, &mut values)?)?;
                let Attribute::Cell(source, index) = col.attr else {
                    return None;
                };
                if source != slot
                    || self.out.facts[slot].kinds[index] != CellKind::Symbol
                    || *anchor.get_or_insert(col.var) != col.var
                {
                    return None;
                }
                size += values.len();
                if size > CAP {
                    return None;
                }
                let mut values: Vec<_> = values.into_iter().map(|v| self.symbol(v)).collect();
                values.sort_unstable();
                values.dedup();
                CellTest::Symbols(index, values)
            };
            tests.push(test);
        }
        Some((anchor?, self.domain(Domain::SameRow(slot, tests))))
    }

    fn filter(
        &mut self,
        e: &PlanExpression,
        lookup: &impl Fn(&ColumnRead) -> Option<Column>,
    ) -> Option<(usize, Id)> {
        use PlanExpression::*;
        if self.syntax_symbols && matches!(e, Call(_) | Not(_) | Or(_)) && let Some(test) = self.glob_domain(e, lookup) {
            return Some(test);
        }
        if let Some((column, cmp, value)) = numeric_comparison(e) {
            let c = lookup(column)?;
            return self.number(c.attr, cmp, value).map(|d| (c.var, d));
        }
        // A written membership list is one local test. Keep its literals together,
        // including when the parser represents the list as an OR of equalities.
        if matches!(e, Or(_) | In(_)) {
            let mut values = Literals::new();
            if let Some(read) = choices(e, &mut values)
                && let Some(c) = lookup(read)
                && let Attribute::Cell(slot, col) = c.attr
                && self.out.facts[slot].kinds[col] == CellKind::Symbol
            {
                let mut ids: Vec<_> = values.into_iter().map(|v| self.symbol(v)).collect();
                ids.sort_unstable();
                ids.dedup();
                return Some((c.var, self.domain(Domain::Fact(slot, Some((col, ids))))));
            }
        }
        // Complements are safe only for total, single-valued native columns.
        // Complementing a projected provider key would also remove nodes with
        // another fact row whose column does satisfy the predicate.
        let mut values = Literals::new();
        let comparison = match e {
            Compare(c) if c.comparator == Comparator::NotEqual => match (&*c.left, &*c.right) {
                (Column(col), Literal(PlanConstant::Text(v)))
                | (Literal(PlanConstant::Text(v)), Column(col)) => {
                    values.push(&*v.0);
                    Some((col, true))
                }
                _ => None,
            },
            NotIn(p) => match (&*p.left, &*p.right) {
                (Column(col), List(items)) => {
                    for item in items {
                        let Literal(PlanConstant::Text(v)) = item else {
                            return None;
                        };
                        values.push(&*v.0);
                    }
                    Some((col, true))
                }
                _ => None,
            },
            Not(inner) => choices(inner, &mut values).map(|c| (c, true)),
            In(_) => choices(e, &mut values).map(|c| (c, false)),
            _ => None,
        };
        if let Some((name, negated)) = comparison {
            let col = lookup(name)?;
            if negated
                && let Attribute::Cell(slot, c) = col.attr
                && self.out.facts[slot].kinds[c] == CellKind::Symbol
            {
                let mut ids: Vec<_> = values.iter().map(|v| self.symbol(v)).collect();
                ids.sort_unstable();
                ids.dedup();
                return Some((col.var, self.domain(Domain::OtherValues(slot, c, ids))));
            }
            if !matches!(
                col.attr,
                Attribute::Kind | Attribute::Field | Attribute::Text
            ) {
                return None;
            }
            let mut set = EMPTY;
            for v in values {
                let d = self.literal(col.attr, v);
                set = self.either(set, d);
                if set == ANY {
                    return None;
                }
            }
            let d = if negated {
                self.domain(Domain::Except(ANY, set))
            } else {
                set
            };
            return Some((col.var, d));
        }
        match e {
            Compare(c) if c.comparator == Comparator::Equal => {
                let (col, text) = match (&*c.left, &*c.right) {
                    (Column(c), Literal(PlanConstant::Text(t)))
                    | (Literal(PlanConstant::Text(t)), Column(c)) => (lookup(c)?, &t.0),
                    _ => return None,
                };
                Some((col.var, self.literal(col.attr, text)))
            }
            Call(f) if matches!(&*f.function.0, "matches" | "starts_with" | "ends_with") => {
                let [Column(c), Literal(PlanConstant::Text(p))] = f.arguments.as_slice() else {
                    return None;
                };
                let col = lookup(c)?;
                let anchored;
                let pattern = match &*f.function.0 {
                    "starts_with" => {
                        anchored = format!(r"\A{}", regex::escape(&p.0));
                        anchored.as_str()
                    }
                    "ends_with" => {
                        anchored = format!(r"{}\z", regex::escape(&p.0));
                        anchored.as_str()
                    }
                    _ => &p.0,
                };
                self.matching(col.attr, pattern, CellKind::Symbol)
                    .map(|d| (col.var, d))
            }
            Or(p) | And(p) => {
                let l = self.filter(&p.left, lookup);
                let r = self.filter(&p.right, lookup);
                match (l, r) {
                    (Some((a, x)), Some((b, y))) if a == b => Some((
                        a,
                        if matches!(e, Or(_)) {
                            self.either(x, y)
                        } else {
                            self.both(x, y)
                        },
                    )),
                    (a, None) | (None, a) if matches!(e, And(_)) => a,
                    _ => None,
                }
            }
            _ => None,
        }
    }
    /// Branches testing different node variables still impose a file condition:
    /// every conjunct needs a witness, and at least one disjunct needs its witnesses.
    fn file_test(
        &mut self,
        e: &PlanExpression,
        lookup: &impl Fn(&ColumnRead) -> Option<Column>,
    ) -> Id {
        use PlanExpression::*;
        if let PlanExpression::Call(f) = e
            && &*f.function.0 == "glob_any"
            && matches!(f.arguments.as_slice(), [PlanExpression::Column(_), PlanExpression::List(items)] if items.is_empty())
        {
            return EMPTY;
        }
        if self.syntax_symbols && let Some((col, re, polarity)) = self.glob_test(e, lookup) {
            match col.attr {
                Attribute::Text => return self.need(Need::SyntaxMatches(re, polarity)),
                Attribute::FilePath => return self.need(Need::PathMatches(re, polarity)),
                Attribute::Cell(slot, col) if self.out.facts[slot].kinds[col] == CellKind::Symbol && polarity => {
                    let d = self.domain(Domain::Matching(slot, col, re));
                    return self.need(Need::Some(d));
                }
                Attribute::SymbolOrigins(i) if polarity => {
                    let mut any = EMPTY;
                    for (slot, column) in self.symbol_origins[i].clone() {
                        let d = self.domain(Domain::Matching(slot, column, re));
                        let exists = self.need(Need::Some(d));
                        any = self.or(any, exists);
                    }
                    return any;
                }
                _ => {}
            }
        }
        if let Call(f) = e
            && matches!(&*f.function.0, "matches" | "glob_any")
            && let [Column(c), Literal(PlanConstant::Text(pattern))] = f.arguments.as_slice()
            && lookup(c).is_some_and(|col| col.attr == Attribute::FilePath)
        {
            let glob;
            let pattern = if &*f.function.0 == "glob_any" {
                let Some(compiled) = super::glob_pattern(&pattern.0) else {
                    let slot = self.out.path_globs.iter().position(|p| p == &*pattern.0)
                        .unwrap_or_else(|| {
                            self.out.path_globs.push(pattern.0.to_string());
                            self.out.path_globs.len() - 1
                        });
                    return self.need(Need::PathGlob(slot));
                };
                glob = compiled;
                &glob
            } else {
                &*pattern.0
            };
            return self
                .regex(pattern)
                .map_or(ANY, |re| self.need(Need::PathMatches(re, true)));
        }
        match e {
            And(p) | Or(p) => {
                let l = self.file_test(&p.left, lookup);
                let r = self.file_test(&p.right, lookup);
                if matches!(e, And(_)) {
                    self.and(l, r)
                } else {
                    self.or(l, r)
                }
            }
            _ => match self.filter(e, lookup) {
                Some((_, d)) if d > EMPTY => self.need(Need::Some(d)),
                Some((_, d)) => d,
                None => ANY,
            },
        }
    }
    // A branch-specific file guard pays for itself before materializing its
    // nodes. Node witnesses are already retained in their input domains.
    fn file_guard(&mut self, need: Id) -> Id {
        if let Some(&Some(guard)) = self.file_guards.get(need) {
            return guard;
        }
        let guard = match self.out.needs[need] {
            Need::Both(a, b) | Need::Either(a, b) => {
                let left = self.file_guard(a);
                let right = self.file_guard(b);
                if matches!(self.out.needs[need], Need::Both(..)) {
                    self.and(left, right)
                } else {
                    self.or(left, right)
                }
            }
            Need::Some(d) => match self.out.domains[d] {
                Domain::Fact(slot, _)
                | Domain::Flag(slot, ..)
                | Domain::Matching(slot, ..)
                | Domain::CapturedSuffix(slot, ..)
                    if self.out.facts[slot].key.is_none() =>
                {
                    need
                }
                _ => ANY,
            },
            _ => need,
        };
        if self.file_guards.len() <= need {
            self.file_guards.resize(need + 1, None);
        }
        self.file_guards[need] = Some(guard);
        guard
    }
    fn in_branch(&mut self, need: Id, nodes: Id) -> Id {
        let need = self.file_guard(need);
        if need == ANY || nodes == EMPTY || matches!(self.out.needs[need],Need::Some(d) if d==nodes)
        {
            return nodes;
        }
        if need == EMPTY {
            return EMPTY;
        }
        let guarded = self.domain(Domain::When(need, nodes));
        if guarded == ANY { nodes } else { guarded }
    }
    fn select<'a>(
        &mut self,
        s: &'a SelectStep,
        rows: &[Summary<'a>],
        conjuncts: &[&PlanExpression],
        scratch: &mut Vec<(&'a str, Column)>,
    ) -> Summary<'a> {
        // Without an input column, no row predicate or output expression
        // can carry a tracked node. Keep the independent file prerequisites.
        if s.sources
            .iter()
            .all(|src| rows[src.step.0].domains.is_empty())
        {
            let need = s
                .sources
                .iter()
                .fold(ANY, |n, src| self.and(n, rows[src.step.0].need));
            return Summary {
                need,
                ..Summary::default()
            };
        }
        if let [src] = s.sources.as_slice()
            && !matches!(s.condition, RowCondition::Tested(_))
        {
            let input = &rows[src.step.0];
            if matches!(s.columns, SelectedColumns::All) {
                let mut out = input.clone();
                for &d in &input.domains {
                    self.require(&mut out, d);
                }
                out.compact();
                return out;
            }
            let mut out = Summary {
                need: input.need,
                domains: input.domains.clone(),
                edges: input.edges.clone(),
                ..Summary::writing(scratch)
            };
            for &d in &input.domains {
                self.require(&mut out, d);
            }
            if let SelectedColumns::Items(items) = &s.columns {
                for item in items.iter() {
                    if let PlanExpression::Column(c) = &item.value
                        && (c.alias == SourceAlias::Unwritten || c.alias == src.alias)
                        && let Some(col) = input.column(&c.column.0)
                    {
                        out.add(&item.name.0, col.var, col.attr);
                    }
                }
            }
            out.compact();
            return out;
        }
        let mut out = Summary::writing(scratch);
        let mut offsets = smallvec::SmallVec::<[usize; 4]>::new();
        for source in &s.sources {
            let r = &rows[source.step.0];
            let offset = out.domains.len();
            offsets.push(offset);
            out.domains.extend_from_slice(&r.domains);
            out.need = self.and(out.need, r.need);
            out.edges.extend(
                r.edges
                    .iter()
                    .map(|(c, p, depth)| (c + offset, p + offset, *depth)),
            );
        }
        let lookup = |c: &ColumnRead| {
            if let [src] = s.sources.as_slice() {
                return (c.alias == SourceAlias::Unwritten || c.alias == src.alias)
                    .then(|| rows[src.step.0].column(&c.column.0))
                    .flatten();
            }
            if c.alias == SourceAlias::Unwritten && s.sources.len() != 1 {
                return None;
            }
            if let [left, right] = s.sources.as_slice()
                && left.alias != right.alias
            {
                if c.alias == left.alias {
                    return rows[left.step.0].column(&c.column.0);
                }
                if c.alias == right.alias {
                    let mut col = rows[right.step.0].column(&c.column.0)?;
                    col.var += offsets[1];
                    return Some(col);
                }
                return None;
            }
            let mut cols = s
                .sources
                .iter()
                .zip(&offsets)
                .filter(|(src, _)| c.alias == SourceAlias::Unwritten || c.alias == src.alias)
                .filter_map(|(src, offset)| {
                    let mut col = rows[src.step.0].column(&c.column.0)?;
                    col.var += offset;
                    Some(col)
                });
            let col = cols.next()?;
            cols.next().is_none().then_some(col)
        };
        let mut map: smallvec::SmallVec<[usize; 8]> = (0..out.domains.len()).collect();
        let mut changed = smallvec::SmallVec::<[bool; 8]>::from_elem(false, map.len());
        // Resolve each equality's columns once. Apply projected membership after
        // all equal node variables have been unified.
        let mut project_pairs = smallvec::SmallVec::<[(Column, Column); 1]>::new();
        for e in conjuncts {
            if let PlanExpression::Compare(c) = e
                && c.comparator == Comparator::Equal
                && let (PlanExpression::Column(a), PlanExpression::Column(b)) =
                    (&*c.left, &*c.right)
                && let (Some(a), Some(b)) = (lookup(a), lookup(b))
            {
                if a.attr != Attribute::Id || b.attr != Attribute::Id {
                    if matches!((a.attr, b.attr), (Attribute::Cell(..), Attribute::Cell(..))) {
                        project_pairs.push((a, b));
                    }
                    continue;
                }
                let (a, b) = (map[a.var], map[b.var]);
                if a != b {
                    changed[a] |= changed[b] || out.domains[a] != out.domains[b];
                    out.domains[a] = self.both(out.domains[a], out.domains[b]);
                    for v in &mut map {
                        if *v == b {
                            *v = a
                        }
                    }
                }
            }
        }
        for (a, b) in project_pairs {
            if let Some((var, d)) = self.project_membership(a, b) {
                let v = map[var];
                changed[v] = true;
                out.domains[v] = self.both(out.domains[v], d);
                self.require(&mut out, d);
            }
        }
        let correlated = match s.sources.as_slice() {
            [src] => rows[src.step.0]
                .native_row
                .and_then(|slot| self.same_row(slot, conjuncts, &lookup)),
            _ => None,
        };
        if let Some((var, d)) = correlated {
            let v = map[var];
            changed[v] = true;
            out.domains[v] = self.both(out.domains[v], d);
            self.require(&mut out, d);
        } else {
            for e in conjuncts {
                if let Some((var, d)) = self.filter(e, &lookup) {
                    let v = map[var];
                    changed[v] |= d != ANY && d != out.domains[v];
                    out.domains[v] = self.both(out.domains[v], d);
                    self.require(&mut out, d);
                } else if matches!(e, PlanExpression::Or(_) | PlanExpression::Call(_) | PlanExpression::Not(_)) {
                    let n = self.file_test(e, &lookup);
                    out.need = self.and(out.need, n);
                }
            }
        }
        // Restrict text regexes to a known entry bucket. The kind constraint
        // may come from an earlier projection or another term in this select.
        for e in conjuncts {
            if let PlanExpression::Call(f) = e
                && &*f.function.0 == "matches"
                && let [
                    PlanExpression::Column(c),
                    PlanExpression::Literal(PlanConstant::Text(p)),
                ] = f.arguments.as_slice()
                && let Some(col) = lookup(c)
                && col.attr == Attribute::Text
            {
                let v = map[col.var];
                if let Some(kind) = self.required_kind(out.domains[v])
                    && let Some(re) = self.regex(&p.0)
                {
                    let d = if self.language.text_kinds.contains(&kind) {
                        self.domain(Domain::TextMatching(kind, re))
                    } else if self.out.regexes[re].is_match(b"") {
                        ANY
                    } else {
                        EMPTY
                    };
                    changed[v] |= d != ANY;
                    out.domains[v] = self.both(out.domains[v], d);
                    self.require(&mut out, d);
                }
            }
        }
        for (c, p, _) in &mut out.edges {
            *c = map[*c];
            *p = map[*p];
        }
        for &(c, p, depth) in &out.edges {
            let (child, parent) = (out.domains[c], out.domains[p]);
            if changed[p] && parent > EMPTY {
                let d = self.at_distance(parent, depth, Domain::Children);
                out.domains[c] = self.both(child, d);
            }
            if changed[c] && child > EMPTY {
                let d = self.at_distance(child, depth, Domain::Parents);
                out.domains[p] = self.both(parent, d);
            }
        }
        for v in 0..out.domains.len() {
            let d = out.domains[v];
            self.require(&mut out, d);
        }
        match &s.columns {
            SelectedColumns::Items(items) => {
                for item in items.iter() {
                    if let PlanExpression::Column(c) = &item.value
                        && let Some(col) = lookup(c)
                    {
                        out.add(&item.name.0, map[col.var], col.attr);
                    }
                }
            }
            SelectedColumns::All if s.sources.len() == 1 => {
                for (n, c) in rows[s.sources[0].step.0].columns.iter() {
                    out.add(n, map[c.var], c.attr);
                }
            }
            _ => {}
        }
        out.compact();
        out
    }

    fn at_distance(&mut self, mut nodes: Id, depth: u8, step: fn(Id) -> Domain) -> Id {
        for _ in 0..depth {
            nodes = self.domain(step(nodes));
            if nodes == ANY {
                break;
            }
        }
        nodes
    }

    fn ordered<'a>(
        &mut self,
        left: &Summary<'a>,
        right: &Summary<'a>,
        outputs: &'a [ryo_values::domain::finish_plans::arguments::joins::types::OutputColumn],
    ) -> Summary<'a> {
        use ryo_values::domain::plan_parts::types::Side;
        let mut out = Summary {
            need: self.and(left.need, right.need),
            ..Summary::default()
        };
        let offset = left.domains.len();
        out.domains.extend_from_slice(&left.domains);
        out.domains.extend_from_slice(&right.domains);
        out.edges.extend_from_slice(&left.edges);
        out.edges.extend(
            right
                .edges
                .iter()
                .map(|(c, p, depth)| (c + offset, p + offset, *depth)),
        );
        for o in outputs {
            let (input, base) = match o.side {
                Side::Left => (left, 0),
                Side::Right => (right, offset),
            };
            if let Some(col) = input.column(&o.column.0) {
                out.add(&o.name.0, col.var + base, col.attr);
            }
        }
        out.compact();
        out
    }
    fn step<'a>(
        &mut self,
        s: &'a FinishStep,
        rows: &[Summary<'a>],
        steps: &[FinishStep],
        conjuncts: &[&PlanExpression],
        scratch: &mut Vec<(&'a str, Column)>,
    ) -> Summary<'a> {
        use FinishStep::*;
        let mut out = Summary::default();
        match s {
            Base(n) => return self.base(&n.0),
            Select(s) => {
                out = self.select(s, rows, conjuncts, scratch);
                if let [source] = s.sources.as_slice() {
                    out.native_row = rows[source.step.0].native_row;
                }
                // A base fact can have several node columns. Filter its rows
                // before projecting the chosen endpoint, keeping the same row.
                if let [source] = s.sources.as_slice()
                    && let Some(origin) = out.native_row
                    && let RowCondition::Tested(e) = &s.condition
                {
                    let mut values = Literals::new();
                    if let Some(read) = choices(e, &mut values)
                        && (read.alias == SourceAlias::Unwritten || read.alias == source.alias)
                        && let Some(Column {
                            attr: Attribute::Cell(slot, col),
                            ..
                        }) = rows[source.step.0].column(&read.column.0)
                        && origin == slot
                        && self.out.facts[slot].kinds[col] == CellKind::Symbol
                    {
                        for i in 0..out.domains.len() {
                            if let Domain::RelatedIds(other, endpoint, None) =
                                self.out.domains[out.domains[i]]
                                && slot == other
                            {
                                let mut ids: Vec<_> =
                                    values.iter().map(|v| self.symbol(v)).collect();
                                ids.sort_unstable();
                                ids.dedup();
                                let filtered = self.domain(Domain::RelatedIds(
                                    slot,
                                    endpoint,
                                    Some((col, ids)),
                                ));
                                out.domains[i] = filtered;
                                self.require(&mut out, filtered);
                            }
                        }
                    }
                }
            }
            Union(u) => {
                out = Summary::writing(scratch);
                let (l, r) = (&rows[u.left.0], &rows[u.right.0]);
                out.need = self.or(l.need, r.need);
                // Only columns whose origin agrees in both arms keep that origin.
                if l.domains.len() == 1 && r.domains.len() == 1 {
                    for (name, a) in l.columns.iter() {
                        if let Some(b) = r.column(name)
                            && let Some(attr) = self.common_origin(a.attr, b.attr)
                        {
                            out.add(name, 0, attr);
                        }
                    }
                    if !out.columns.is_empty() {
                        let left = self.in_branch(l.need, l.domains[0]);
                        let right = self.in_branch(r.need, r.domains[0]);
                        out.var(self.either(left, right));
                    }
                    return out;
                }
                let mut vars = Map::default();
                for (n, a) in l.columns.iter() {
                    if let Some(b) = r.column(n)
                        && let Some(origin) = self.common_origin(a.attr, b.attr)
                    {
                        let var = *vars.entry((a.var, b.var)).or_insert_with(|| {
                            let left = self.in_branch(l.need, l.domains[a.var]);
                            let right = self.in_branch(r.need, r.domains[b.var]);
                            out.var(self.either(left, right))
                        });
                        out.add(n, var, origin);
                    }
                }
                // Parent is functional. One common lineage per projected
                // child is sufficient; never combine alternative witnesses.
                for (&(lc, rc), &child) in &vars {
                    if let Some(&(_, lp, depth)) = l.edges.iter().find(|(c, _, _)| *c == lc)
                        && let Some(&(_, rp, _)) =
                            r.edges.iter().find(|(c, _, d)| *c == rc && *d == depth)
                        && let Some(&parent) = vars.get(&(lp, rp))
                        && parent != child
                    {
                        out.edges.push((child, parent, depth));
                    }
                }
                out.edges.sort_unstable();
                out.edges.dedup();
            }
            Antijoin(j) => {
                out = rows[j.left.0].clone();
                if let Some(need) = self.rejected_filter(j, steps, &out) {
                    out.need = self.and(out.need, need);
                }
            }
            GroupMin(g) | GroupMax(g) => return rows[g.from.0].clone(),
            Project(p) => {
                out = rows[p.from.0].clone();
                out.retain_columns(|(n, _)| p.columns.iter().any(|c| *n == &*c.0));
                out.compact();
            }
            Extend(x) => {
                out = rows[x.from.0].clone();
                out.retain_columns(|(n, _)| !x.values.iter().any(|c| *n == &*c.name.0));
            }
            GroupOrdinal(g) => {
                out = rows[g.from.0].clone();
                out.retain_columns(|(n, _)| *n != &*g.output.0);
            }
            FitsRendered(x) => return rows[x.from.0].clone(),
            Replace(x) => {
                out = rows[x.from.0].clone();
                out.retain_columns(|(n, _)| *n != &*x.output.0);
            }
            SpanBetween(x) => {
                out = rows[x.from.0].clone();
                out.retain_columns(|(n, _)| *n != &*x.output.0);
            }
            SpanBounds(x) => {
                out = rows[x.from.0].clone();
                out.retain_columns(|(n, _)| *n != &*x.start_output.0 && *n != &*x.end_output.0);
            }
            Capture(x) => {
                out = rows[x.from.0].clone();
                if self.syntax_symbols
                    && let Some(col) = out.column(&x.path.0)
                    && col.attr == Attribute::FilePath
                    && let Some(pattern) = patterns::CapturePath::from_pattern(&x.pattern.0)
                {
                    let path = self.need(Need::CapturedPath(pattern));
                    out.need = self.and(path, out.need);
                }
                out.retain_columns(|(n, _)| !x.captures.iter().any(|col| *n == &*col.0));
            }
            AsOf(x) => return self.ordered(&rows[x.left.0], &rows[x.right.0], &x.output),
            OrderedMatch(x) => return self.ordered(&rows[x.left.0], &rows[x.right.0], &x.output),
            GroupCount(g) => {
                if !g.keys.is_empty() {
                    out = rows[g.from.0].clone();
                    out.retain_columns(|(n, _)| {
                        g.keys.iter().any(|c| *n == &*c.0) && *n != &*g.count.0
                    });
                    out.compact();
                }
            }
            Rows(r) => {
                if r.rows.is_empty() {
                    out.need = EMPTY;
                }
            }
            Axis(a) => {
                let a = &a.arguments;
                let (from, dom) = (&rows[a.from.0], &rows[a.domain.0]);
                out.need = self.and(from.need, dom.need);
                let node = |r: &Summary<'_>| {
                    r.column("id")
                        .filter(|c| c.attr == Attribute::Id)
                        .map_or(ANY, |c| r.domains[c.var])
                };
                let (mut id, mut related) = (node(from), node(dom));
                if matches!(&steps[a.relation.0],Base(n) if self.native(&n.0) && super::first::CHILD.contains(&&*n.0))
                {
                    let (origin, target) = (id, related);
                    let (down, up): (fn(Id) -> Domain, fn(Id) -> Domain) = match a.direction {
                        Direction::Parent => {
                            out.edges.push((0, 1, 1));
                            (Domain::Children, Domain::Parents)
                        }
                        Direction::Child => {
                            out.edges.push((1, 0, 1));
                            (Domain::Parents, Domain::Children)
                        }
                        Direction::Ancestor | Direction::NearestAncestor => {
                            (Domain::Descendants, Domain::Ancestors)
                        }
                        Direction::Descendant => (Domain::Ancestors, Domain::Descendants),
                        _ => {
                            out.var(id);
                            out.var(related);
                            out.add("id", 0, Attribute::Id);
                            out.add("related", 1, Attribute::Id);
                            return out;
                        }
                    };
                    if target > EMPTY {
                        let allowed = self.domain(down(target));
                        id = self.both(origin, allowed);
                        self.require(&mut out, id);
                    }
                    if origin > EMPTY {
                        let allowed = self.domain(up(origin));
                        related = self.both(target, allowed);
                        self.require(&mut out, related);
                    }
                }
                if id == EMPTY {
                    id = ANY
                }
                out.var(id);
                out.var(related);
                out.add("id", 0, Attribute::Id);
                out.add("related", 1, Attribute::Id);
            }
            // These operations can keep an input row without a corresponding row of their other input.
            SourceMatches(x) => {
                let from = &rows[x.from.0];
                use ryo_values::domain::finish_plans::arguments::texts::types::OptionalColumn;
                out = from.clone();
                out.retain_columns(|(name, _)| {
                    *name != &*x.output.0
                        && ![
                            &x.line_output,
                            &x.capture_output,
                            &x.capture_text_output,
                            &x.capture_line_output,
                        ]
                        .iter()
                        .any(|opt| matches!(opt,OptionalColumn::Written(c) if &*c.0==*name))
                        && !x.capture_texts.iter().any(|c| &*c.output.0 == *name)
                });
                // SourceMatches always reads a slice of this file's raw bytes.
                // Without a boundary assertion, a match on that slice is also
                // a match somewhere in the full file. Test the shared regex once.
                let pattern = &*x.pattern.0;
                if !["^", "$", r"\A", r"\z", r"\b", r"\B", r"\<", r"\>"]
                    .iter()
                    .any(|op| pattern.contains(op))
                    && let Some(re) = self.regex(pattern)
                {
                    let need = self.need(Need::SourceContains(re));
                    out.need = self.and(out.need, need);
                }
                if let Some(col) = from.column(&x.span.0)
                    && col.attr == Attribute::Span
                    && let Some(kind) = self.required_kind(from.domains[col.var])
                    && let Some(regex) = self.regex(&x.pattern.0)
                {
                    // Every capture lies inside a match on this particular input
                    // span. A required named group may narrow the result further.
                    let raw = self.domain(Domain::SpanMatching(kind, regex));
                    let d = self.both(raw, from.domains[col.var]);
                    self.require(&mut out, d);
                }
                if let Some(col) = from.column(&x.span.0)
                    && let Some(d) = self.matching(col.attr, &x.pattern.0, CellKind::Span)
                {
                    let d = self.both(from.domains[col.var], d);
                    self.require(&mut out, d);
                }
            }
            SubtreeClass(x) => out.need = rows[x.from.0].need,
            OrderedDfs(x) => out.need = rows[x.from.0].need,
            Unlisted(x) if &*x.operator.0 == "order" => {
                // Ordering can append ranks; each output still needs an input row.
                use ryo_values::domain::finish_plans::arguments::literals::types::ArgumentValue;
                for a in &x.arguments {
                    if &*a.name.0 == "from"
                        && let ArgumentValue::Step(from) = a.value
                    {
                        out.need = rows[from.0].need;
                    }
                }
            }
            Unlisted(_) => {}
            _ => {
                for i in inputs(s) {
                    out.need = self.and(out.need, rows[i.0].need);
                }
            }
        }
        out
    }
}

fn choices<'a>(e: &'a PlanExpression, values: &mut Literals<'a>) -> Option<&'a ColumnRead> {
    use PlanExpression::*;
    match e {
        Compare(c) if c.comparator == Comparator::Equal => {
            let (col, t) = match (&*c.left, &*c.right) {
                (Column(col), Literal(PlanConstant::Text(t)))
                | (Literal(PlanConstant::Text(t)), Column(col)) => (col, t),
                _ => return None,
            };
            values.push(&t.0);
            Some(col)
        }
        Or(p) => {
            let l = choices(&p.left, values)?;
            let r = choices(&p.right, values)?;
            (l == r).then_some(l)
        }
        In(p) => {
            let (Column(col), List(es)) = (&*p.left, &*p.right) else {
                return None;
            };
            for v in es {
                let Literal(PlanConstant::Text(t)) = v else {
                    return None;
                };
                values.push(&t.0);
            }
            Some(col)
        }
        _ => None,
    }
}

fn numeric_comparison(e: &PlanExpression) -> Option<(&ColumnRead, Comparator, i64)> {
    use PlanExpression::*;
    let Compare(c) = e else { return None };
    let whole = |e: &PlanExpression| match e {
        Number(PlanNumber::Whole(n)) | Literal(PlanConstant::Number(PlanNumber::Whole(n))) => {
            Some(n.0)
        }
        _ => None,
    };
    match (&*c.left, &*c.right) {
        (Column(col), n) => Some((col, c.comparator, whole(n)?)),
        (n, Column(col)) => {
            use Comparator::*;
            let cmp = match c.comparator {
                Less => Greater,
                LessOrEqual => GreaterOrEqual,
                Greater => Less,
                GreaterOrEqual => LessOrEqual,
                other => other,
            };
            Some((col, cmp, whole(n)?))
        }
        _ => None,
    }
}

pub(super) fn conjuncts(step: &FinishStep) -> Conjuncts<'_> {
    let mut result = Conjuncts::new();
    if let FinishStep::Select(q) = step
        && let RowCondition::Tested(e) = &q.condition
    {
        terms(e, &mut result);
    }
    result
}

fn terms<'a>(e: &'a PlanExpression, out: &mut Conjuncts<'a>) {
    if let PlanExpression::And(p) = e {
        terms(&p.left, out);
        terms(&p.right, out)
    } else {
        out.push(e)
    }
}

/// Prepared texts share one set of file and node tests without renumbering their executor steps.
pub(super) fn prepare_plans(
    plans: &[&FinishPlan],
    targets: Option<&[Vec<usize>]>,
    vocab: &Vocab,
    language: &Language,
    reader_rows: &[String],
    current_file: Option<(&'static str, &'static str)>,
    syntax_symbols: bool,
) -> Program {
    let steps = plans.iter().map(|p| p.steps.len()).sum::<usize>();
    let clock = cfg!(feature = "pair-diagnostics") && std::env::var_os("VEC_PAY_PREP").is_some();
    let mut t = super::thread_cpu();
    let mut phase = |name: &str| {
        if clock {
            let now = super::thread_cpu();
            println!("PAYPREP {name} {:.6}", now - t);
            t = now;
        }
    };
    let mut b = Builder {
        vocab,
        language,
        reader_rows,
        current_file,
        syntax_symbols,
        positive: false,
        regex_slots: Map::default(),
        glob_slots: Map::default(),
        symbol_origins: Vec::new(),
        proof_validation: None,
        early_omit: 0,
        signatures: Vec::with_capacity(steps),
        out: Program {
            scratch: Default::default(),
            meter: None,
            paths: Vec::new(),
            file_only: Vec::new(),
            regexes: Vec::new(),
            path_globs: Vec::new(),
            text_kinds: language.text_kinds.clone(),
            symbols: Map::default(),
            max_text: 0,
            texts: Default::default(),
            domains: vec![],
            needs: vec![],
            facts: vec![],
            exact: vec![],
            roots: vec![],
            skips: vec![],
            capped: 0,
        },
        ds: Map::with_capacity_and_hasher(steps / 2, Default::default()),
        ns: Map::with_capacity_and_hasher(steps, Default::default()),
        sizes: vec![],
        visited: vec![],
        generation: 0,
        file_guards: vec![],
        exists: vec![],
        counts: Map::default(),
    };
    b.domain(Domain::Any);
    b.domain(Domain::Empty);
    b.need(Need::Any);
    b.need(Need::Empty);
    let omit = cost::omitted();
    for (i, plan) in plans.iter().enumerate() {
        prepare_roots(&mut b, plan, targets.map(|ts| ts[i].as_slice()), omit, &mut phase);
    }
    // The separate raw-word index has no proof that every provider value has
    // a raw spelling (for example, a normalized or resolved identifier).
    // Certify the whole first half only when that independent skip is vacuous.
    b.out.exact.retain(|(ri, _)| b.out.skips[*ri].2.is_empty());
    precheck::schedule(&mut b);
    phase("roots");
    cost::arrange(&mut b.out, omit);
    #[cfg(feature = "pair-diagnostics")]
    if std::env::var_os("VEC_PAIR_METER").is_some() {
        b.out.meter = Some(cost::Meter::new(b.out.facts.len()));
    }
    (b.out.paths, b.out.file_only) = paths::compile(&b.out);
    phase("arrange");
    if clock || b.out.meter.is_some() {
        println!(
            "PAYREGEX prepare {} {}",
            b.out.regexes.iter().filter(|r| r.initialized()).count(),
            b.out.regexes.len()
        );
    }
    for d in &b.out.domains {
        if let Domain::Text(t) = d {
            b.out.max_text = b.out.max_text.max(t.len());
            b.out.texts.insert(t.as_bytes().to_vec());
        }
    }
    b.out
}

fn prepare_roots(b: &mut Builder<'_>, plan: &FinishPlan, targets: Option<&[usize]>, omit: u16, phase: &mut impl FnMut(&str)) {
    b.counts.clear();
    b.positive = false;
    b.early_omit = 0;
    let language = b.language;
    let mut live = vec![false; plan.steps.len()];
    if let Some(steps) = targets {
        for &step in steps { live[step] = true; }
    } else {
        for r in &plan.rules {
            if let RuleEmit::Written(e) = &r.emit {
                live[e.step.0] = true;
            }
        }
    }
    for (i, s) in plan.steps.iter().enumerate().rev() {
        if live[i] {
            use FinishStep::*;
            match s {
                Antijoin(j) => live[j.left.0] = true,
                SubtreeClass(x) => live[x.from.0] = true,
                OrderedDfs(x) => live[x.from.0] = true,
                GroupCount(g) if g.keys.is_empty() => {}
                Unlisted(x) if &*x.operator.0 != "order" => {}
                _ => {
                    for p in inputs(s) {
                        live[p.0] = true;
                    }
                }
            }
        }
    }
    // Assign slots once, also for facts used only on an excluded side.
    for s in &plan.steps {
        if let FinishStep::Base(name) = s
            && (language.facts.iter().any(|r| r == &*name.0)
                || language.file_sets.iter().any(|r| r == &*name.0)
                || language.walk_facts.iter().any(|f| f.name == &*name.0))
        {
            b.base(&name.0);
        }
    }
    phase("live");
    let mut exclusions = exact::prepare(b, plan, &live, targets).into_iter().peekable();
    b.positive = true;
    // Proofs are complete before this point. Widen only positive witnesses.
    b.early_omit = omit;
    phase("proof");
    let columns = bumpalo::Bump::new();
    let mut scratch = Vec::new();
    let mut rows = Vec::with_capacity(plan.steps.len());
    let mut skips = Vec::with_capacity(plan.steps.len());
    for (i, s) in plan.steps.iter().enumerate() {
        let (mut row, skip) = if live[i] {
            let conjuncts = conjuncts(s);
            (
                b.step(s, &rows, &plan.steps, &conjuncts, &mut scratch),
                super::pair_skip::Skip::with_terms(s, &skips, language, &conjuncts),
            )
        } else {
            (Summary::default(), super::pair_skip::Skip::default())
        };
        if let Some((_, exclusion)) = exclusions.next_if(|(step, _)| *step == i) {
            exclusion.apply(b, &mut row);
        }
        if let FinishStep::GroupCount(g) = s
            && let Some(proof) = b.counts.get(&i)
            && let Some(c) = row.column(&g.keys[proof.key].0)
            && c.attr == Attribute::Id
        {
            row.add(&g.count.0, c.var, Attribute::Count(i));
        }
        row.freeze(&columns, &mut scratch);
        rows.push(row);
        skips.push(skip);
    }
    phase("rows-and-skips");
    #[cfg(feature = "pair-diagnostics")]
    if std::env::var_os("VEC_PAIR_TRACE").is_some() {
        for (step, row) in rows.iter().enumerate() {
            println!("WITNESS\t{step}\t{}\t{:?}", row.need, row.domains);
        }
    }
    let mut push = |name, step: Option<usize>| {
        let (need, skip) = step.map_or_else(|| (ANY, Default::default()), |i| {
            (rows[i].need, skips[i].root())
        });
        let early = b.file_guard(need);
        let root = if early == ANY || early == need {
            need
        } else {
            b.need(Need::Both(early, need))
        };
        b.out.roots.push((name, root));
        b.out.skips.push(skip);
    };
    if let Some(steps) = targets {
        for &step in steps { push(String::new(), Some(step)); }
    } else {
        for rule in &plan.rules {
            let step = match &rule.emit {
                RuleEmit::Written(e) => Some(e.step.0),
                RuleEmit::Unwritten => None,
            };
            push(rule.rule.0.to_string(), step);
        }
    }
}
