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
}

fn base<'a>(b: &mut Builder<'_>, name: &str) -> Option<Row<'a>> {
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
// projecting the node key. Two tests on different fact columns could have
// different row witnesses, so this transfer accepts just one indexed test.
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
    let rel = catalog::relation_named(NameRef(&name.0))?;
    let slot = b.out.facts.iter().position(|f| f.relation == rel)?;
    let spec = &b.out.facts[slot];
    let key = spec.key?;
    let kinds = spec.kinds;
    let names = catalog::columns(rel);
    let column = |c: &ColumnRead| {
        if c.alias != SourceAlias::Unwritten && c.alias != src.alias {
            return None;
        }
        names
            .iter()
            .position(|n| catalog::column_name(*n).0 == &*c.column.0)
    };
    let d = match &s.condition {
        RowCondition::Every => b.domain(Domain::Fact(slot, None)),
        RowCondition::Tested(e) => {
            if let PlanExpression::Call(f) = e {
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
                let mut values = Vec::new();
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
    let SelectedColumns::Items(items) = &s.columns else {
        return None;
    };
    let mut cols = Vec::new();
    let mut derived = false;
    for item in items {
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
    Some(Row {
        nodes: d,
        guard: EMPTY,
        derived,
        cols,
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
            | Domain::OtherValues(slot, _, _)
            | Domain::Flag(slot, ..)
            | Domain::Matching(slot, _, _) => {
                if !slots.contains(&slot) {
                    slots.push(slot);
                }
            }
            Domain::Both(a, b) | Domain::Either(a, b) | Domain::Except(a, b) => {
                pending.extend([a, b])
            }
            Domain::Parents(a)
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
            } else if b.out.regexes[re].is_match(b"") {
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

fn select<'a>(b: &mut Builder<'_>, s: &'a SelectStep, rows: &[Option<Row<'a>>]) -> Option<Row<'a>> {
    if !(1..=2).contains(&s.sources.len()) || s.count != CountMark::Unmarked {
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
    let mut filters = Vec::new();
    if let RowCondition::Tested(e) = &s.condition {
        terms(e, &mut filters)
    }
    let mut shifts = [0, 0];
    let mut nodes = rows[s.sources[0].step.0].as_ref()?.nodes;
    let mut guard = rows[s.sources[0].step.0].as_ref()?.guard;
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
        let d = predicate(b, e, &get, nodes)?;
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
            Base(n) if b.file_sets && b.language.file_sets.iter().any(|r| r == &*n.0) => {
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
                    && let RowCondition::Tested(PlanExpression::Call(f)) = &s.condition
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
        out.push(value);
    }
    out
}

pub(super) fn prepare<'a>(
    b: &mut Builder<'_>,
    plan: &'a FinishPlan,
    live: &[bool],
) -> Vec<Option<Exclusion<'a>>> {
    let guards = guards(b, plan);
    // Follow the right side for exact keyed differences. File guards already
    // describe suppression inputs whose rows need richer relational operators.
    let mut live = live.to_vec();
    for (i, s) in plan.steps.iter().enumerate().rev() {
        if !live[i] {
            continue;
        }
        match s {
            FinishStep::Antijoin(j) => {
                live[j.left.0] = true;
                if guards[j.right.0] == ANY {
                    live[j.right.0] = true;
                }
            }
            FinishStep::Project(_)
            | FinishStep::Select(_)
            | FinishStep::Union(_)
            | FinishStep::Extend(_)
            | FinishStep::SpanBounds(_)
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
        let row = live[i]
            .then(|| -> Option<Row<'_>> {
                use FinishStep::*;
                match s {
                    Base(n) => base(b, &n.0),
                    Select(s) => select(b, s, &rows).or_else(|| fact_projection(b, s, plan)),
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
                        for name in &p.columns {
                            row.column(&name.0)?;
                        }
                        row.cols
                            .retain(|(n, _)| p.columns.iter().any(|c| &*c.0 == *n));
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
        rows.push(row);
    }
    if std::env::var_os("VEC_PAIR_TRACE").is_some() {
        for (i, row) in rows.iter().enumerate() {
            if row.is_some() {
                println!("PROVEN\t{i}");
            }
        }
    }
    if std::env::var_os("VEC_EXACT_DIAG").is_some() {
        for r in &plan.rules {
            if let RuleEmit::Written(e) = &r.emit
                && let FinishStep::Antijoin(j) = &plan.steps[e.step.0]
            {
                println!(
                    "PROOF\t{}\tleft {}\tright {}\troot {}",
                    r.rule.0,
                    rows[j.left.0].is_some(),
                    guards[j.right.0],
                    rows[e.step.0].is_some()
                );
            }
        }
    }
    b.out.exact = plan
        .rules
        .iter()
        .enumerate()
        .filter_map(|(ri, r)| {
            let RuleEmit::Written(e) = &r.emit else {
                return None;
            };
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
                || !row.cols.iter().all(|(n, v)| {
                    v.depth == 0 && *n == v.field && matches!(*n, "file" | "span" | "id")
                })
                || !e.key.iter().all(|c| row.column(&c.0).is_some())
            {
                return None;
            }
            Some((
                ri,
                Certificate {
                    nodes: row.nodes,
                    guard: row.guard,
                    facts: {
                        let mut slots = Vec::new();
                        required_facts(&b.out.domains, row.nodes, &mut slots);
                        slots
                    },
                },
            ))
        })
        .collect();

    plan.steps
        .iter()
        .map(|s| {
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
            Some(Exclusion {
                id: id?,
                keys,
                domain,
                guard: right.guard,
                slots,
            })
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
