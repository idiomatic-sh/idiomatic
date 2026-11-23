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
    let cols = items
        .iter()
        .map(|i| {
            let PlanExpression::Column(c) = &i.value else {
                return None;
            };
            let at = column(c)?;
            let field = if at == key {
                "id"
            } else if kinds[at] == CellKind::File {
                "file"
            } else {
                return None;
            };
            Some((&*i.name.0, Value { depth: 0, field }))
        })
        .collect::<Option<Vec<_>>>()?;
    Some(Row {
        nodes: d,
        guard: EMPTY,
        cols,
    })
}

fn required_facts(domains: &[Domain], id: Id, slots: &mut Vec<usize>) {
    match domains[id] {
        Domain::Fact(slot, _) | Domain::Matching(slot, _, _) => {
            if !slots.contains(&slot) {
                slots.push(slot);
            }
        }
        Domain::Both(a, b) | Domain::Either(a, b) | Domain::Except(a, b) => {
            required_facts(domains, a, slots);
            required_facts(domains, b, slots);
        }
        Domain::Parents(a) | Domain::Children(a) | Domain::Ancestors(a) => {
            required_facts(domains, a, slots)
        }
        _ => {}
    }
}

fn predicate<'a>(
    b: &mut Builder<'_>,
    e: &PlanExpression,
    get: &impl Fn(&ColumnRead) -> Option<Value<'a>>,
    domain: Id,
) -> Option<Id> {
    use PlanExpression::*;
    let (value, test) = match e {
        Compare(c) if c.comparator == Comparator::Equal => {
            if let (Column(a), Column(d)) = (&*c.left, &*c.right) {
                return (get(a)? == get(d)?).then_some(ANY);
            }
            let (Column(col), Literal(PlanConstant::Text(lit))) = (&*c.left, &*c.right) else {
                return None;
            };
            let v = get(col)?;
            let attr = match v.field {
                "kind" => Attribute::Kind,
                "field" => Attribute::Field,
                "text" => Attribute::Text,
                _ => return None,
            };
            let d = b.literal(attr, &lit.0);
            if d == ANY {
                return None;
            }
            (v, d)
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
    match value.depth {
        0 => Some(test),
        1 => Some(b.domain(Domain::Children(test))),
        _ => None,
    }
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
        let under = |b: &mut Builder<'_>, d, depth| {
            if depth == 0 || d == ANY {
                d
            } else {
                b.domain(Domain::Children(d))
            }
        };
        let left = under(b, nodes, c);
        let right_set = under(b, right.nodes, a);
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
    let mut row = Row { nodes, guard, cols };
    if row.cols.iter().any(|(_, v)| v.depth > 0)
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
    row.cols.iter().all(|(_, v)| v.depth <= 1).then_some(row)
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
            FinishStep::Project(_) | FinishStep::Select(_) | FinishStep::Union(_) => {
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
                    Project(p) => {
                        let mut row = rows[p.from.0].clone()?;
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
                        if l.cols != r.cols {
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
            if right.guard != EMPTY || j.left_on.len() != j.right_on.len() {
                return None;
            }
            let mut slots = Vec::new();
            required_facts(&b.out.domains, right.nodes, &mut slots);
            if !slots.is_empty() {
                return None;
            }
            let mut id = None;
            let mut keys = Vec::new();
            for (left, r) in j.left_on.iter().zip(&j.right_on) {
                let v = right.column(&r.0)?;
                if v.depth != 0 {
                    return None;
                }
                let attr = match v.field {
                    "id" => {
                        id = Some(&*left.0);
                        Attribute::Id
                    }
                    "file" => Attribute::FilePath,
                    "span" => Attribute::Span,
                    _ => return None,
                };
                keys.push((&*left.0, attr));
            }
            Some(Exclusion {
                id: id?,
                keys,
                domain: right.nodes,
            })
        })
        .collect()
}

pub(super) struct Exclusion<'a> {
    id: &'a str,
    keys: Vec<(&'a str, Attribute)>,
    domain: Id,
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
        let d = b.domain(Domain::Except(row.domains[node.var], self.domain));
        if d != ANY {
            row.domains[node.var] = d;
            b.require(row, d);
        }
    }
}
