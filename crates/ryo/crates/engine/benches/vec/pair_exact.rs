//! A row certificate identifies every projected value with a column of one node
//! or its parent. A domain represents the set of those rows (the emit is a set).
//! A guard is a necessary condition of an excluded relation: if that condition
//! fails, the antijoin returns its complete left input regardless of its keys.
use super::*;

#[derive(Clone, Copy, Eq, PartialEq)]
struct Value<'a> {
    depth: u8,
    field: &'a str,
}
impl<'a> Value<'a> {
    fn shifted(self, depth: u8) -> Self {
        Self {
            depth: if self.field == "file" {
                0
            } else {
                self.depth + depth
            },
            ..self
        }
    }
}
#[derive(Clone)]
struct Row<'a> {
    nodes: Id,
    guard: Id,
    cols: Vec<(&'a str, Value<'a>)>,
    // Presence is exact, but a subsequent projection must discard derived values.
    derived: bool,
}
impl<'a> Row<'a> {
    fn column(&self, name: &str) -> Option<Value<'a>> {
        self.cols.iter().find(|(n, _)| *n == name).map(|(_, v)| *v)
    }
}
#[derive(Clone, Debug)]
pub(crate) struct Certificate {
    pub nodes: Id,
    pub guard: Id,
    pub facts: Vec<usize>,
    pub columns: Box<[LocationColumn]>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum LocationColumn {
    File,
    Span,
    Node,
}

impl LocationColumn {
    fn named(name: &str) -> Option<Self> {
        Some(match name {
            "file" => Self::File,
            "span" => Self::Span,
            "id" => Self::Node,
            _ => return None,
        })
    }
}

fn base<'a>(b: &mut Builder<'_>, name: &str) -> Option<Row<'a>> {
    if !b.native(name) { return None; }
    let (nodes, fields): (_, &[&str]) = match name {
        "syntax-tree" | "semantic-syntax" => (ANY, &["id", "file", "kind", "text", "span"]),
        "source-location" => (
            ANY,
            &[
                "id",
                "file",
                "span",
                "start-line",
                "end-line",
                "start-byte",
                "end-byte",
                "start-column",
                "end-column",
            ],
        ),
        "syntax-child" | "semantic-child" => (
            b.domain(Domain::HasParent),
            &["parent", "child", "field", "file", "ordinal"],
        ),
        _ => return None,
    };
    let cols = fields
        .iter()
        .map(|&name| {
            let (depth, field) = match name {
                "parent" => (1, "id"),
                "child" => (0, "id"),
                _ => (0, name),
            };
            (name, Value { depth, field })
        })
        .collect();
    Some(Row {
        nodes,
        guard: EMPTY,
        derived: false,
        cols,
    })
}

// A provider fact may contain many rows for one id. Filter its rows before
// projecting the node key, keeping conjuncts on the same provider row.
fn fact_projection<'a>(
    b: &mut Builder<'_>,
    s: &'a SelectStep,
    plan: &FinishPlan,
) -> Option<Row<'a>> {
    let [src] = s.sources.as_slice() else {
        return None;
    };
    let FinishStep::Base(name) = &plan.steps[src.step.0] else {
        return None;
    };
    let slot = b
        .out
        .facts
        .iter()
        .position(|f| f.name == &*name.0 && f.exact)?;
    let spec = &b.out.facts[slot];
    let key = spec.key?;
    let kinds = spec.kinds.clone();
    let names = spec.columns.clone();
    let column = |c: &ColumnRead| {
        if c.alias != SourceAlias::Unwritten && c.alias != src.alias {
            return None;
        }
        names.iter().position(|n| *n == &*c.column.0)
    };
    let SelectedColumns::Items(items) = &s.columns else {
        return None;
    };
    let mut cols = Vec::new();
    let mut derived = false;
    for item in items.iter() {
        let PlanExpression::Column(c) = &item.value else {
            return None;
        };
        let at = column(c)?;
        let field = if at == key {
            "id"
        } else if kinds[at] == CellKind::File {
            "file"
        } else {
            derived = true;
            continue;
        };
        cols.push((&*item.name.0, Value { depth: 0, field }));
    }
    let d = match &s.condition {
        RowCondition::Every => b.domain(Domain::Fact(slot, None)),
        RowCondition::Tested(e) => {
            let mut conjuncts = Conjuncts::new();
            terms(e, &mut conjuncts);
            let correlated = b.same_row(slot, &conjuncts, &|c| {
                Some(Column {
                    var: 0,
                    attr: Attribute::Cell(slot, column(c)?),
                })
            });
            if let Some((_, d)) = correlated {
                d
            } else if let Some((c, cmp, value)) = numeric_comparison(e) {
                b.number(Attribute::Cell(slot, column(c)?), cmp, value)?
            } else if let PlanExpression::Call(f) = e.as_ref() {
                let [
                    PlanExpression::Column(c),
                    PlanExpression::Literal(PlanConstant::Text(p)),
                ] = f.arguments.as_slice()
                else {
                    return None;
                };
                if &*f.function.0 != "matches" {
                    return None;
                }
                b.matching(Attribute::Cell(slot, column(c)?), &p.0, CellKind::Symbol)?
            } else {
                let mut values = Literals::new();
                let c = column(choices(e, &mut values)?)?;
                if kinds[c] != CellKind::Symbol {
                    return None;
                }
                let mut ids: Vec<_> = values.into_iter().map(|v| b.symbol(v)).collect();
                ids.sort_unstable();
                ids.dedup();
                b.domain(Domain::Fact(slot, Some((c, ids))))
            }
        }
    };
    Some(Row {
        nodes: d,
        guard: EMPTY,
        derived,
        cols,
    })
}

pub(super) struct Grouped {
    pub key: usize,
    pub depth: u8,
    pub rows: Id,
    pub guard: Id,
    pub facts: Vec<usize>,
}

// In a keyed group each distinct child id counts once, even when a join or
// union produced duplicate rows. File and one fixed ancestor identify its group.
fn group<'a>(
    b: &mut Builder<'_>,
    step: usize,
    g: &'a ryo_values::domain::finish_plans::arguments::groups::types::GroupCountArguments,
    row: &Row<'a>,
) -> Option<Row<'a>> {
    use ryo_values::domain::finish_plans::arguments::groups::types::CountedRows;
    let CountedRows::Distinct(distinct) = &g.distinct else {
        return None;
    };
    if row.column(&distinct.0)?
        != (Value {
            depth: 0,
            field: "id",
        })
    {
        return None;
    }
    let mut cols = Vec::new();
    let mut parent = None;
    for (index, k) in g.keys.iter().enumerate() {
        if k == &g.count {
            return None;
        }
        let value = row.column(&k.0)?;
        if value.field == "id" {
            let (_, depth) = parent.get_or_insert((index, value.depth));
            if *depth != value.depth {
                return None;
            }
        } else if value.field != "file" {
            return None;
        }
        cols.push((
            &*k.0,
            Value {
                depth: 0,
                field: value.field,
            },
        ));
    }
    let (key, depth) = parent?;
    let nodes = above(b, row.nodes, depth);
    let mut facts = Vec::new();
    required_facts(&b.out.domains, row.nodes, &mut facts);
    let _ = b.counts.insert(
        step,
        Grouped {
            key,
            depth,
            rows: row.nodes,
            guard: row.guard,
            facts,
        },
    );
    Some(Row {
        nodes,
        guard: row.guard,
        cols,
        derived: true,
    })
}

fn required_facts(domains: &[Domain], id: Id, slots: &mut Vec<usize>) {
    let mut pending = vec![id];
    let mut seen = Vec::new();
    while let Some(id) = pending.pop() {
        if seen.contains(&id) {
            continue;
        }
        seen.push(id);
        match domains[id] {
            Domain::Fact(slot, _)
            | Domain::SameRow(slot, _)
            | Domain::OtherValues(slot, _, _)
            | Domain::Flag(slot, ..)
            | Domain::Number(slot, ..)
            | Domain::Unmatched(slot, _, _)
            | Domain::Matching(slot, _, _) => {
                if !slots.contains(&slot) {
                    slots.push(slot);
                }
            }
            Domain::Member(a, _, b, _) => {
                for slot in [a, b] {
                    if !slots.contains(&slot) {
                        slots.push(slot);
                    }
                }
            }
            Domain::Both(a, b) | Domain::Either(a, b) | Domain::Except(a, b) => {
                pending.extend([a, b])
            }
            Domain::CountUnder(a, ..)
            | Domain::Parents(a)
            | Domain::Children(a)
            | Domain::Ancestors(a)
            | Domain::Descendants(a) => pending.push(a),
            _ => {}
        }
    }
}

fn predicate<'a>(
    b: &mut Builder<'_>,
    e: &PlanExpression,
    get: &impl Fn(&ColumnRead) -> Option<Value<'a>>,
    domain: Id,
) -> Option<Id> {
    use PlanExpression::*;
    let native = |v: Value<'_>| match v.field {
        "kind" => Some(Attribute::Kind),
        "field" => Some(Attribute::Field),
        "text" => Some(Attribute::Text),
        _ => None,
    };
    let (value, test) = match e {
        Compare(c) if matches!(c.comparator, Comparator::Equal | Comparator::NotEqual) => {
            if let (Column(a), Column(d)) = (&*c.left, &*c.right) {
                if get(a)? != get(d)? {
                    return None;
                }
                return Some(if c.comparator == Comparator::Equal {
                    ANY
                } else {
                    EMPTY
                });
            }
            let (col, lit) = match (&*c.left, &*c.right) {
                (Column(c), Literal(PlanConstant::Text(t)))
                | (Literal(PlanConstant::Text(t)), Column(c)) => (c, t),
                _ => return None,
            };
            let v = get(col)?;
            let d = b.literal(native(v)?, &lit.0);
            if d == ANY {
                return None;
            }
            (
                v,
                if c.comparator == Comparator::Equal {
                    d
                } else {
                    b.domain(Domain::Except(ANY, d))
                },
            )
        }
        In(p) | NotIn(p) => {
            let (Column(col), List(values)) = (&*p.left, &*p.right) else {
                return None;
            };
            if values.len() > CAP {
                return None;
            }
            let v = get(col)?;
            let attr = native(v)?;
            let mut d = EMPTY;
            for value in values {
                let Literal(PlanConstant::Text(text)) = value else {
                    return None;
                };
                let one = b.literal(attr, &text.0);
                d = b.either(d, one);
                if d == ANY {
                    return None;
                }
            }
            (
                v,
                if matches!(e, NotIn(_)) {
                    b.domain(Domain::Except(ANY, d))
                } else {
                    d
                },
            )
        }
        Call(c) if &*c.function.0 == "matches" => {
            let [Column(col), Literal(PlanConstant::Text(pattern))] = c.arguments.as_slice() else {
                return None;
            };
            let v = get(col)?;
            if v.field != "text" || v.depth != 0 {
                return None;
            }
            let k = b.required_kind(domain)?;
            let re = b.regex(&pattern.0)?;
            let d = if b.language.text_kinds.contains(&k) {
                b.domain(Domain::TextMatching(k, re))
            } else if b.out.regexes[re].compiled()?.is_match(b"") {
                ANY
            } else {
                EMPTY
            };
            (v, d)
        }
        Or(p) | And(p) => {
            let a = predicate(b, &p.left, get, domain)?;
            let c = predicate(b, &p.right, get, domain)?;
            return Some(if matches!(e, Or(_)) {
                b.either(a, c)
            } else {
                b.both(a, c)
            });
        }
        _ => return None,
    };
    Some(below(b, test, value.depth))
}

// Keep the witness at the deepest row. A named ancestor is functional in
// that row, however many other rows share that ancestor.
fn below(b: &mut Builder<'_>, mut set: Id, depth: u8) -> Id {
    for _ in 0..depth {
        set = b.domain(Domain::Children(set));
    }
    set
}

fn above(b: &mut Builder<'_>, mut set: Id, depth: u8) -> Id {
    for _ in 0..depth {
        set = b.domain(Domain::Parents(set));
    }
    set
}

fn select<'a>(
    b: &mut Builder<'_>,
    s: &'a SelectStep,
    rows: &[Option<Row<'a>>],
    plan: &FinishPlan,
) -> Option<Row<'a>> {
    if !(1..=2).contains(&s.sources.len()) || s.count != CountMark::Unmarked {
        return None;
    }
    if matches!(&s.columns, SelectedColumns::Items(items) if items.iter().any(|v| !matches!(v.value, PlanExpression::Column(_))))
    {
        return None;
    }
    let lookup = |c: &ColumnRead| {
        let i = if c.alias == SourceAlias::Unwritten && s.sources.len() == 1 {
            0
        } else {
            s.sources
                .iter()
                .position(|src| src.alias == c.alias && c.alias != SourceAlias::Unwritten)?
        };
        Some((i, rows[s.sources[i].step.0].as_ref()?.column(&c.column.0)?))
    };
    let mut nodes = rows[s.sources[0].step.0].as_ref()?.nodes;
    let mut guard = rows[s.sources[0].step.0].as_ref()?.guard;
    let mut filters = Conjuncts::new();
    if let RowCondition::Tested(e) = &s.condition {
        terms(e, &mut filters)
    }
    let mut shifts = [0, 0];
    if s.sources.len() == 2 {
        let right = rows[s.sources[1].step.0].as_ref()?;
        // Equality of the two source identities binds a unique row. The same
        // holds when one identity is the other row's parent.
        let (a, c) = filters.iter().find_map(|e| {
            let PlanExpression::Compare(eq) = e else {
                return None;
            };
            if eq.comparator != Comparator::Equal {
                return None;
            }
            let (PlanExpression::Column(x), PlanExpression::Column(y)) = (&*eq.left, &*eq.right)
            else {
                return None;
            };
            let ((i, a), (j, c)) = (lookup(x)?, lookup(y)?);
            if i == j || a.field != "id" || c.field != "id" || a.depth.min(c.depth) != 0 {
                return None;
            }
            Some(if i == 0 {
                (a.depth, c.depth)
            } else {
                (c.depth, a.depth)
            })
        })?;
        shifts = [c, a];
        let left = below(b, nodes, c);
        let right_set = below(b, right.nodes, a);
        nodes = b.both(left, right_set);
        guard = b.or(guard, right.guard);
    }
    let get = |c: &ColumnRead| {
        let (i, v) = lookup(c)?;
        Some(v.shifted(shifts[i]))
    };
    // Bucket restrictions first, so a text test can use the declared text model.
    filters.sort_by_key(|e| matches!(e, PlanExpression::Call(_)));
    for e in filters {
        let d = if let Some((col, cmp, literal)) = numeric_comparison(e) {
            let i = if col.alias == SourceAlias::Unwritten && s.sources.len() == 1 {
                0
            } else {
                s.sources
                    .iter()
                    .position(|src| src.alias == col.alias && col.alias != SourceAlias::Unwritten)?
            };
            let step = s.sources[i].step.0;
            let FinishStep::GroupCount(g) = &plan.steps[step] else {
                return None;
            };
            if col.column != g.count {
                return None;
            }
            let proof = b.counts.get(&step)?;
            let d = b.domain(Domain::CountUnder(proof.rows, proof.depth, cmp, literal));
            below(b, d, shifts[i])
        } else {
            predicate(b, e, &get, nodes)?
        };
        nodes = b.both(nodes, d);
    }
    let cols = match &s.columns {
        SelectedColumns::Items(items) => items
            .iter()
            .map(|i| {
                let PlanExpression::Column(c) = &i.value else {
                    return None;
                };
                Some((&*i.name.0, get(c)?))
            })
            .collect::<Option<Vec<_>>>()?,
        SelectedColumns::All if s.sources.len() == 1 => {
            rows[s.sources[0].step.0].as_ref()?.cols.clone()
        }
        _ => return None,
    };
    let derived =
        matches!(s.columns, SelectedColumns::All) && rows[s.sources[0].step.0].as_ref()?.derived;
    let mut row = Row {
        nodes,
        guard,
        cols,
        derived,
    };
    while row.cols.iter().any(|(_, v)| v.depth > 0)
        && !row.derived
        && row
            .cols
            .iter()
            .all(|(_, v)| v.depth > 0 || v.field == "file")
    {
        row.nodes = b.domain(Domain::Parents(row.nodes));
        for (_, v) in &mut row.cols {
            v.depth = v.depth.saturating_sub(1)
        }
    }
    row.cols
        .iter()
        .all(|(_, v)| usize::from(v.depth) <= CAP)
        .then_some(row)
}

/// The final slot of a two-slot template, and the literal immediately before it.
fn captured_suffix(pattern: &str) -> Option<(&str, &[u8])> {
    let (first, rest) = pattern.strip_prefix('{')?.split_once('}')?;
    let (delimiter, last) = rest.strip_suffix('}')?.split_once('{')?;
    let name = last.trim_start_matches('*');
    if first.contains('{')
        || delimiter.is_empty()
        || delimiter.contains('}')
        || name.is_empty()
        || name.contains(['{', '}'])
        || name == first.trim_start_matches('*')
    {
        return None;
    }
    Some((name, delimiter.as_bytes()))
}

// Propagate only necessary file-set predicates. A missing or partial provider
// answer makes the guard possible. An operator without a transfer rule does too.
fn guards(b: &mut Builder<'_>, plan: &FinishPlan) -> Vec<Id> {
    use FinishStep::*;
    let mut out = Vec::with_capacity(plan.steps.len());
    let mut bases = Map::default();
    let mut suffixes = Map::default();
    for (i, s) in plan.steps.iter().enumerate() {
        let value = match s {
            Base(n) if b.language.file_sets.iter().any(|r| r == &*n.0) => {
                let row = b.base(&n.0);
                let need = row.need;
                bases.insert(i, row);
                need
            }
            Select(s) => {
                let mut need = s
                    .sources
                    .iter()
                    .fold(ANY, |a, src| b.and(a, out[src.step.0]));
                if let [src] = s.sources.as_slice()
                    && let Some(base) = bases.get(&src.step.0)
                    && let RowCondition::Tested(e) = &s.condition
                {
                    let test = b.file_test(e, &|c| {
                        (c.alias == SourceAlias::Unwritten || c.alias == src.alias)
                            .then(|| base.column(&c.column.0))
                            .flatten()
                    });
                    need = b.and(need, test);
                }
                if let [src] = s.sources.as_slice()
                    && let Some(&(name, slot, col, delimiter)) = suffixes.get(&src.step.0)
                    && let RowCondition::Tested(term) = &s.condition
                    && let PlanExpression::Call(f) = term.as_ref()
                    && &*f.function.0 == "matches"
                    && let [
                        PlanExpression::Column(c),
                        PlanExpression::Literal(PlanConstant::Text(p)),
                    ] = f.arguments.as_slice()
                    && (c.alias == SourceAlias::Unwritten || c.alias == src.alias)
                    && &*c.column.0 == name
                    && let Some(re) = b.regex(&p.0)
                {
                    let d = b.domain(Domain::CapturedSuffix(slot, col, Vec::from(delimiter), re));
                    let n = b.need(Need::Some(d));
                    need = b.and(need, n);
                }
                need
            }
            Antijoin(j) => out[j.left.0],
            AsOf(j) => out[j.left.0],
            Union(u) => b.or(out[u.left.0], out[u.right.0]),
            GroupCount(g) if !g.keys.is_empty() => out[g.from.0],
            GroupMin(g) | GroupMax(g) => out[g.from.0],
            SourceMatches(x) => out[x.from.0],
            Extend(x) => out[x.from.0],
            Project(x) => out[x.from.0],
            SpanBounds(x) => out[x.from.0],
            Capture(x) => {
                // In a pattern ending in a literal followed by a slot, that
                // slot is a suffix of the original cell. Test every such split;
                // the earlier slots and the separator can only narrow the result.
                if let Some(base) = bases.get(&x.from.0)
                    && let Some(Column {
                        attr: Attribute::Cell(slot, col),
                        ..
                    }) = base.column(&x.path.0)
                    && b.out.facts[slot].kinds[col] == CellKind::Symbol
                    && let Some((name, delimiter)) = captured_suffix(&x.pattern.0)
                    && x.captures.iter().any(|c| &*c.0 == name)
                {
                    suffixes.insert(i, (name, slot, col, delimiter));
                }
                out[x.from.0]
            }
            Replace(x) => out[x.from.0],
            SpanBetween(x) => out[x.from.0],
            FitsRendered(x) => out[x.from.0],
            RenderSyntax(x) => out[x.from.0],
            OrderedDfs(x) => out[x.from.0],
            SubtreeClass(x) => out[x.from.0],
            GroupOrdinal(x) => out[x.from.0],
            Rows(r) if r.rows.is_empty() => EMPTY,
            _ => ANY,
        };
        // An invalid predicate can never prove that the suppressing side is empty.
        let invalid = b.take_validation();
        out.push(b.or(value, invalid));
    }
    out
}

// Bound the exact proof frontier before following demand backwards. Any
// operator outside the proof language breaks that path. A base remains
// possible here because its provider may support an exact key projection.
fn possible_rows(plan: &FinishPlan) -> Vec<bool> {
    use FinishStep::*;
    let mut possible = Vec::with_capacity(plan.steps.len());
    for step in &plan.steps {
        let can = match step {
            Base(_) => true,
            Select(q) => {
                (1..=2).contains(&q.sources.len())
                    && q.sources.iter().all(|v| possible[v.step.0])
                    && match &q.columns {
                        SelectedColumns::All => true,
                        SelectedColumns::Items(items) => items
                            .iter()
                            .all(|item| matches!(item.value, PlanExpression::Column(_))),
                    }
            }
            Union(x) => possible[x.left.0] && possible[x.right.0],
            Antijoin(x) => possible[x.left.0],
            Project(x) => possible[x.from.0],
            SpanBounds(x) => possible[x.from.0],
            Extend(x) => possible[x.from.0],
            GroupCount(x) => possible[x.from.0],
            GroupOrdinal(x) => possible[x.from.0],
            GroupMin(x) | GroupMax(x) => possible[x.from.0],
            _ => false,
        };
        possible.push(can);
    }
    possible
}

pub(super) fn prepare<'a>(
    b: &mut Builder<'_>,
    plan: &'a FinishPlan,
    live: &[bool],
    targets: &[usize],
) -> Vec<(usize, Exclusion<'a>)> {
    // Only a retained proof needs its matcher validated. Carry this obligation
    // even when the row's node expression folds away the matching instruction.
    b.proof_validation = Some(EMPTY);
    let guards = guards(b, plan);
    // Follow the right side for exact keyed differences. File guards already
    // describe suppression inputs whose rows need richer relational operators.
    let positive = live;
    let possible = possible_rows(plan);
    let mut live = vec![false; plan.steps.len()];
    for rule in &plan.rules {
        if let RuleEmit::Written(e) = &rule.emit
            && &*e.subject.0 == "file"
            && targets.contains(&e.step.0)
        {
            live[e.step.0] = true;
        }
    }
    for (i, step) in plan.steps.iter().enumerate() {
        if !positive[i] {
            continue;
        }
        match step {
            FinishStep::GroupCount(g) if !g.keys.is_empty() => live[i] = true,
            FinishStep::Antijoin(j) if guards[j.right.0] == ANY || positive[j.right.0] => {
                live[j.right.0] = true
            }
            _ => {}
        }
    }
    for (i, s) in plan.steps.iter().enumerate().rev() {
        live[i] &= possible[i];
        if !live[i] {
            continue;
        }
        match s {
            FinishStep::Antijoin(j) => {
                live[j.left.0] = true;
                if guards[j.right.0] == ANY || positive[j.right.0] {
                    live[j.right.0] = true;
                }
            }
            FinishStep::Project(_)
            | FinishStep::Select(_)
            | FinishStep::Union(_)
            | FinishStep::Extend(_)
            | FinishStep::SpanBounds(_)
            | FinishStep::GroupCount(_)
            | FinishStep::GroupOrdinal(_)
            | FinishStep::GroupMin(_)
            | FinishStep::GroupMax(_) => {
                for input in inputs(s) {
                    live[input.0] = true;
                }
            }
            _ => {}
        }
    }
    let mut rows: Vec<Option<Row<'_>>> = Vec::with_capacity(plan.steps.len());
    for (i, s) in plan.steps.iter().enumerate() {
        let capped = b.out.capped;
        let mut row = live[i]
            .then(|| -> Option<Row<'_>> {
                use FinishStep::*;
                match s {
                    Base(n) => base(b, &n.0),
                    Select(s) => select(b, s, &rows, plan).or_else(|| fact_projection(b, s, plan)),
                    Antijoin(j) => {
                        let mut row = rows[j.left.0].clone()?;
                        if let Some(right) = &rows[j.right.0] {
                            let keys = j.left_on.iter().zip(&j.right_on);
                            if j.left_on.len() == j.right_on.len()
                                && keys.clone().any(|(l, r)| {
                                    row.column(&l.0)
                                        == Some(Value {
                                            depth: 0,
                                            field: "id",
                                        })
                                        && right.column(&r.0) == row.column(&l.0)
                                })
                                && keys.into_iter().all(|(l, r)| {
                                    row.column(&l.0).is_some()
                                        && row.column(&l.0) == right.column(&r.0)
                                })
                            {
                                row.nodes = b.domain(Domain::Except(row.nodes, right.nodes));
                                row.guard = b.or(row.guard, right.guard);
                                return Some(row);
                            }
                        }
                        row.guard = b.or(row.guard, guards[j.right.0]);
                        (row.guard != ANY).then_some(row)
                    }
                    Extend(x) => {
                        let mut row = rows[x.from.0].clone()?;
                        if row.derived || x.values.iter().any(|v| row.column(&v.name.0).is_some()) {
                            return None;
                        }
                        row.derived = !x.values.is_empty();
                        Some(row)
                    }
                    SpanBounds(x) => {
                        let mut row = rows[x.from.0].clone()?;
                        if row.derived
                            || row.column(&x.span.0)?.field != "span"
                            || row.column(&x.start_output.0).is_some()
                            || row.column(&x.end_output.0).is_some()
                        {
                            return None;
                        }
                        row.derived = true;
                        Some(row)
                    }
                    GroupCount(g) => group(b, i, g, rows[g.from.0].as_ref()?),
                    GroupOrdinal(g) => {
                        let mut row = rows[g.from.0].clone()?;
                        row.column(&g.order.0)?;
                        for key in &g.keys {
                            row.column(&key.0)?;
                        }
                        row.cols.retain(|(n, _)| *n != &*g.output.0);
                        row.derived = true;
                        Some(row)
                    }
                    GroupMin(g) | GroupMax(g) => {
                        let row = rows[g.from.0].clone()?;
                        row.column(&g.order.0)?;
                        let mut keyed = false;
                        for key in &g.keys {
                            let value = row.column(&key.0)?;
                            keyed |= value
                                == Value {
                                    depth: 0,
                                    field: "id",
                                };
                        }
                        keyed.then_some(row)
                    }
                    Project(p) => {
                        let mut row = rows[p.from.0].clone()?;
                        row.derived = false;
                        row.cols = p.columns.iter().map(|name| {
                            Some((name.0.as_ref(), row.column(&name.0)?))
                        }).collect::<Option<_>>()?;
                        Some(row)
                    }
                    Union(u) => {
                        let mut l = rows[u.left.0].clone()?;
                        let r = rows[u.right.0].as_ref()?;
                        if l.derived || r.derived || l.cols != r.cols {
                            return None;
                        }
                        l.nodes = b.either(l.nodes, r.nodes);
                        l.guard = b.or(l.guard, r.guard);
                        Some(l)
                    }
                    _ => None,
                }
            })
            .flatten()
            .filter(|_| b.out.capped == capped);
        let invalid = b.take_validation();
        if let Some(row) = &mut row {
            row.guard = b.or(row.guard, invalid);
        }
        if row.is_none() && matches!(s, FinishStep::GroupCount(_)) {
            let _ = b.counts.remove(&i);
        }
        rows.push(row);
    }
    b.proof_validation = None;
    let base = b.out.roots.len();
    let mut certificates = plan
        .rules
        .iter()
        .filter_map(|r| {
            let RuleEmit::Written(e) = &r.emit else {
                return None;
            };
            let root = targets.iter().position(|&step| step == e.step.0)?;
            let row = rows[e.step.0].as_ref()?;
            if row.nodes == ANY
                || &*e.subject.0 != "file"
                || row.column("span")
                    != Some(Value {
                        depth: 0,
                        field: "span",
                    })
                || row.column("file")
                    != Some(Value {
                        depth: 0,
                        field: "file",
                    })
                || row.derived
                || !e.key.iter().all(|c| row.column(&c.0).is_some())
            {
                return None;
            }
            Some((
                base + root,
                Certificate {
                    nodes: row.nodes,
                    guard: row.guard,
                    facts: {
                        let mut slots = Vec::new();
                        required_facts(&b.out.domains, row.nodes, &mut slots);
                        slots
                    },
                    columns: row.cols.iter().map(|(name, value)| {
                        (value.depth == 0 && *name == value.field)
                            .then(|| LocationColumn::named(name))
                            .flatten()
                    }).collect::<Option<_>>()?,
                },
            ))
        })
        .collect::<Vec<_>>();
    certificates.sort_unstable_by_key(|(root, _)| *root);
    certificates.dedup_by_key(|(root, _)| *root);
    b.out.exact.extend(certificates);

    plan.steps
        .iter()
        .enumerate()
        .filter_map(|(i, s)| {
            if !positive[i] {
                return None;
            }
            let FinishStep::Antijoin(j) = s else {
                return None;
            };
            let right = rows[j.right.0].as_ref()?;
            if j.left_on.len() != j.right_on.len() {
                return None;
            }
            let mut slots = Vec::new();
            required_facts(&b.out.domains, right.nodes, &mut slots);
            let mut id = None;
            let mut depth = None;
            let mut keys = Vec::new();
            for (left, r) in j.left_on.iter().zip(&j.right_on) {
                let v = right.column(&r.0)?;
                let attr = match v.field {
                    "id" => {
                        id = Some(&*left.0);
                        Attribute::Id
                    }
                    "file" => Attribute::FilePath,
                    "span" => Attribute::Span,
                    _ => return None,
                };
                if attr != Attribute::FilePath && *depth.get_or_insert(v.depth) != v.depth {
                    return None;
                }
                keys.push((&*left.0, attr));
            }
            let capped = b.out.capped;
            let domain = above(b, right.nodes, depth?);
            if b.out.capped != capped {
                return None;
            }
            Some((
                i,
                Exclusion {
                    id: id?,
                    keys,
                    domain,
                    guard: right.guard,
                    slots,
                },
            ))
        })
        .collect()
}

pub(super) struct Exclusion<'a> {
    id: &'a str,
    keys: Vec<(&'a str, Attribute)>,
    domain: Id,
    guard: Id,
    slots: Vec<usize>,
}

impl Exclusion<'_> {
    pub(super) fn apply(&self, b: &mut Builder<'_>, row: &mut Summary<'_>) {
        let Some(node) = row.column(self.id) else {
            return;
        };
        if !self.keys.iter().all(|(n, a)| {
            row.column(n).is_some_and(|c| {
                c.attr == *a && (c.var == node.var || c.attr == Attribute::FilePath)
            })
        }) {
            return;
        }
        let left = row.domains[node.var];
        let mut d = b.domain(Domain::Except(left, self.domain));
        if d != ANY && (self.guard != EMPTY || !self.slots.is_empty()) {
            d = b.domain(Domain::ProvenExcept(
                left,
                d,
                self.guard,
                self.slots.clone(),
            ));
        }
        if d != ANY {
            row.domains[node.var] = d;
            b.require(row, d);
        }
    }
}
