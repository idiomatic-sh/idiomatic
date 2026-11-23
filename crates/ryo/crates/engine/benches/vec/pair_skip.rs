//! Capped file prerequisites of the same lowered step as its node witnesses.
use super::{first::Language, pair_summary::inputs};
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
