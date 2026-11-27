//! Demand and CPU of ordered base-view row lists. View projections do not change row IDs.
use super::{E, File, Native, Op, Run, ViewSpec};
use super::{Ty, V};
use std::rc::Rc;

// Stable typed atoms, remapped to base columns by Exec::view. Flatten/sort associative
// conjunctions and alternatives, intern literal membership as a set, retain seed and regex identity.
/// The entire expression is a positive literal membership on this column. The entry
/// planner may extract only one side of an AND, which is insufficient for identity.
pub(crate) fn members(e: &E, col: usize) -> Option<Vec<u32>> {
    match e {
        E::Or(a, b) => {
            let mut values = members(a, col)?;
            values.extend(members(b, col)?);
            Some(values)
        }
        E::Cmp(Op::Eq, ..) | E::In(_, _, false) => super::kinds_required(e, 0, col),
        _ => None,
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum FilterKey {
    Member(usize, Vec<u32>),
    Col(usize, usize, Ty),
    Lit(V),
    Text(u32),
    Num(i64),
    And(Vec<Self>),
    Or(Vec<Self>),
    Cmp(Op, Box<Self>, Box<Self>),
    Not(Box<Self>),
    In(bool, Box<Self>, Vec<Self>),
    InExpr(bool, Box<Self>, Box<Self>),
    Call(String, Vec<Self>),
    Re(Box<Self>, String),
    LowerEqual(Box<Self>, Rc<[u8]>),
    Glob(Box<Self>, u32),
    List(Vec<Self>),
}

pub(crate) fn normalized(e: &E) -> FilterKey {
    use FilterKey as K;
    if let E::Cmp(Op::Eq, a, b) = e {
        if let (E::Col(0, c, super::Ty::Sym), E::Text(t))
        | (E::Text(t), E::Col(0, c, super::Ty::Sym)) = (&**a, &**b)
        {
            return K::Member(*c, vec![*t]);
        }
    }
    if matches!(e, E::Or(..) | E::In(..)) {
        fn column(e: &E) -> Option<usize> {
            match e {
                E::Col(0, c, super::Ty::Sym) => Some(*c),
                E::Cmp(_, a, b) | E::Or(a, b) | E::In(a, b, _) => column(a).or_else(|| column(b)),
                _ => None,
            }
        }
        if let Some(c) = column(e)
            && let Some(mut vs) = members(e, c)
        {
            vs.sort_unstable();
            vs.dedup();
            return K::Member(c, vs);
        }
    }
    fn terms(e: &E, and: bool, out: &mut Vec<FilterKey>) {
        match e {
            E::And(a, b) if and => {
                terms(a, and, out);
                terms(b, and, out);
            }
            E::Or(a, b) if !and => {
                terms(a, and, out);
                terms(b, and, out);
            }
            _ => out.push(normalized(e)),
        }
    }
    match e {
        E::And(..) | E::Or(..) => {
            let and = matches!(e, E::And(..));
            let mut xs = Vec::new();
            terms(e, and, &mut xs);
            xs.sort_unstable();
            xs.dedup();
            if and { K::And(xs) } else { K::Or(xs) }
        }
        E::Cmp(op, a, b) => {
            let (mut a, mut b) = (normalized(a), normalized(b));
            if matches!(op, Op::Eq | Op::Ne) && a > b {
                std::mem::swap(&mut a, &mut b);
            }
            K::Cmp(*op, Box::new(a), Box::new(b))
        }
        E::Not(a) => K::Not(Box::new(normalized(a))),
        E::In(a, b, neg) => {
            let mut items = if let E::List(xs) = &**b {
                xs.iter().map(normalized).collect::<Vec<_>>()
            } else {
                return K::InExpr(*neg, Box::new(normalized(a)), Box::new(normalized(b)));
            };
            items.sort_unstable();
            items.dedup();
            K::In(*neg, Box::new(normalized(a)), items)
        }
        E::Call(name, xs) => K::Call(name.clone(), xs.iter().map(normalized).collect()),
        E::TextTest(a, super::TextTest::Any(tests)) => K::Or(tests.pattern.iter().map(|test| normalized(&E::TextTest(a.clone(), test.clone()))).collect()),
        E::TextTest(a, super::TextTest::Regex(re)) => K::Re(Box::new(normalized(a)), re.pattern.as_str().into()),
        E::TextTest(a, super::TextTest::Glob(glob)) => K::Glob(Box::new(normalized(a)), glob.pattern),
        E::TextTest(a, super::TextTest::LowerEqual(lit)) => K::LowerEqual(Box::new(normalized(a)), lit.pattern.clone()),
        E::List(xs) => K::List(xs.iter().map(normalized).collect()),
        E::Col(s, c, ty) => K::Col(*s, *c, *ty),
        E::Lit(v) => K::Lit(*v),
        E::Text(t) => K::Text(*t),
        E::Num(n) => K::Num(*n),
    }
}

pub(crate) fn conjuncts(es: &[E]) -> Vec<FilterKey> {
    fn append(e: &E, out: &mut Vec<FilterKey>) {
        if let E::And(a, b) = e {
            append(a, out);
            append(b, out);
        } else {
            out.push(normalized(e));
        }
    }
    let mut xs = Vec::new();
    for e in es {
        append(e, &mut xs);
    }
    xs.sort_unstable();
    xs.dedup();
    xs
}

/// Whether the file's existing, ordered provider buckets already contain the result.
pub(crate) struct Handoff {
    pub(crate) fields: bool,
    pub(crate) codes: Vec<u16>,
    pub(crate) extent: super::Extent,
}
impl Handoff {
    pub(crate) fn compile(run: &Run, v: &ViewSpec) -> Option<Self> {
        let (col, fields, vocab) = match v.base {
            Native::Tree(_) => (1, false, &run.kind_sym),
            Native::Child(_) if !run.file_field_codes => (3, true, &run.field_sym),
            _ => return None,
        };
        if v.filter.is_empty() {
            return None;
        }
        let requirements = v
            .filter
            .iter()
            .map(|e| members(e, col))
            .collect::<Option<Vec<_>>>()?;
        if fields && requirements.iter().all(|xs| xs.contains(&run.empty)) {
            return None;
        }
        let codes = vocab
            .iter()
            .enumerate()
            .filter(|(_, s)| requirements.iter().all(|xs| xs.contains(s)))
            .filter_map(|(i, _)| u16::try_from(i).ok())
            .collect();
        Some(Self { fields, codes, extent: v.base.extent() })
    }
    pub(crate) fn codes(&self) -> &[u16] {
        &self.codes
    }
    pub(crate) fn select(&self, f: &File<'_>) -> super::cold_bucket::Slot {
        super::cold_bucket::select(f, self.fields, &self.codes, self.extent)
    }

}
