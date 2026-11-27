//! redo-fills c4 (THROWAWAY, bead ryo-fmx3.1): a view's tests compiled once a plan node (when the view is made) and read
//! as whole columns: a literal test is a compare of interned ids over the base column's slice; a test the compiler does not
//! take stays the generic `eval` of the row. The answers are `eval`'s: the same fast-path meanings (`Col = 'text'` is an
//! id compare, `!=` holds on the absent cell, `in` never holds on it, `is null`, a column against a column of its type).

use super::{E, File, NULL, Native, Op, Run, Src, ViewSpec, eval, truthy};

/// One compiled test on the view's base columns.
pub(crate) enum Test {
    /// `col = id` (a symbol column against a text).
    Eq(usize, u64),
    /// `col != id`.
    Ne(usize, u64),
    /// `col in [ids]` (`true`: `not in`).
    In(usize, std::rc::Rc<super::symbol_predicate::SymbolSet>, bool),
    /// `col is null` (`true`: `is not null`).
    /// `a = b` (`true`) or `a != b` (`false`) on two columns of one type.
    Cols(usize, usize, bool),
    Not(Box<Test>),
    And(Box<Test>, Box<Test>),
    Or(Box<Test>, Box<Test>),
    /// Anything else: the generic evaluation of the conjunct.
    Eval(E),
}

/// A view filter's conjuncts compiled.
pub(crate) fn compile(run: &Run, filter: &[E]) -> Vec<Test> {
    filter.iter().map(|e| one(run, e)).collect()
}

pub(crate) fn one(run: &Run, e: &E) -> Test {
    match e {
        E::Cmp(o @ (Op::Eq | Op::Ne), a, b) => match (&**a, &**b) {
            (E::Col(0, c, super::Ty::Sym), E::Text(t)) | (E::Text(t), E::Col(0, c, super::Ty::Sym)) => {
                if *o == Op::Eq { Test::Eq(*c, u64::from(*t)) } else { Test::Ne(*c, u64::from(*t)) }
            }
            (E::Col(0, c1, t1), E::Col(0, c2, t2)) if t1 == t2 => Test::Cols(*c1, *c2, *o == Op::Eq),
            _ => Test::Eval(e.clone()),
        },
        E::In(a, b, neg) => match (&**a, &**b) {
            (E::Col(0, c, super::Ty::Sym), E::List(items)) if items.iter().all(|i| matches!(i, E::Text(_))) => {
                Test::In(*c, super::symbol_predicate::SymbolSet::bind(run, items.iter().filter_map(|i| if let E::Text(t) = i { Some(u64::from(*t)) } else { None }).collect()), *neg)
            }
            _ => Test::Eval(e.clone()),
        },
        E::Not(a) => Test::Not(Box::new(one(run, a))),
        E::And(a, b) => Test::And(Box::new(one(run, a)), Box::new(one(run, b))),
        E::Or(a, b) => Test::Or(Box::new(one(run, a)), Box::new(one(run, b))),
        _ => Test::Eval(e.clone()),
    }
}

/// A base cell: a fill's column read in place; any other native relation through `native_get`.
#[inline]
pub(crate) fn cell(f: &File<'_>, run: &mut Run, base: Native, row: u32, col: usize) -> u64 {
    match base {
        Native::Fact(i) => {
            f.fact(usize::from(i)).map_or(NULL, |t| t.cell(col, row as usize))
        }
        Native::Tree(_) if col == 1 => u64::from(run.kind_sym[usize::from(f.kind[row as usize])]),
        Native::Tree(_) | Native::Location if col == 0 => u64::from(row),
        Native::Child(_) if col == 0 => u64::from(f.parent[row as usize]),
        Native::Child(_) if col == 1 => u64::from(row),
        Native::Child(_) if col == 3 => u64::from(run.field_sym.get(usize::from(f.field[row as usize])).copied().unwrap_or(run.empty)),
        Native::Tree(_) if col == 3 => u64::from(f.file),
        Native::Tree(_) if col == 2 => u64::from(f.elem_text(run, row)),
        Native::Child(_) if col == 4 => u64::from(f.file),
        _ => f.native_get(run, base, row, col),
    }
}

pub(crate) fn holds(f: &File<'_>, run: &mut Run, base: Native, t: &Test, row: u32) -> bool {
    match t {
        Test::Eq(c, v) => cell(f, run, base, row, *c) == *v,
        Test::Ne(c, v) => cell(f, run, base, row, *c) != *v,
        Test::In(c, vs, neg) => {
            let x = cell(f, run, base, row, *c);
            (x != NULL && vs.contains(x)) != *neg
        }
        Test::Cols(a, b, want) => {
            let (x, y) = (cell(f, run, base, row, *a), cell(f, run, base, row, *b));
            (x == y && x != NULL) == *want
        }
        Test::Not(a) => !holds(f, run, base, a, row),
        Test::And(a, b) => holds(f, run, base, a, row) && holds(f, run, base, b, row),
        Test::Or(a, b) => holds(f, run, base, a, row) || holds(f, run, base, b, row),
        Test::Eval(e) => truthy(eval(run, f, &[Src::Base(base)], &[row], e)),
    }
}

/// Whether the view's tests hold on one base row.
#[inline]
pub(crate) fn row_holds(f: &File<'_>, run: &mut Run, v: &ViewSpec, row: u32) -> bool {
    v.tests.iter().all(|t| holds(f, run, v.base, t, row))
}

/// Keeps the rows on which the view's tests hold: each test over the rows still kept, a fill's column read as a slice.
pub(crate) fn retain(f: &File<'_>, run: &mut Run, v: &ViewSpec, rows: &mut Vec<u32>) {
    if !f.vrows.take_filtering_entry() || !super::cold_test::entry(f, run, v, rows) { retain_original(f, run, v, rows); }
}

pub(crate) fn retain_original(f: &File<'_>, run: &mut Run, v: &ViewSpec, rows: &mut Vec<u32>) {
    for t in &v.tests {
        if rows.is_empty() {
            return;
        }
        if let Native::Fact(i) = v.base
            && let Some(tab) = f.fact(usize::from(i))
        {
            match t {
                Test::Eq(c, x) => {
                    let col = tab.columns().get(*c);
                    rows.retain(|r| col.at(*r as usize) == *x);
                    continue;
                }
                Test::Ne(c, x) => {
                    let col = tab.columns().get(*c);
                    rows.retain(|r| col.at(*r as usize) != *x);
                    continue;
                }
                Test::In(c, vs, neg) => {
                    let col = tab.columns().get(*c);
                    rows.retain(|r| {
                        let x = col.at(*r as usize);
                        (x != NULL && vs.contains(x)) != *neg
                    });
                    continue;
                }
                _ => {}
            }
        }
        rows.retain(|r| holds(f, run, v.base, t, *r));
    }
}

/// The view's column `col` (a view column) at `rows`, appended to `out`: a fill's column gathered from its slice.
pub(crate) fn gather(f: &File<'_>, run: &mut Run, base: Native, bc: usize, rows: &[u32], out: &mut Vec<u64>) {
    gather_ids(f, run, base, bc, rows.iter().copied(), out);
}

pub(crate) fn gather_ids(f: &File<'_>, run: &mut Run, base: Native, bc: usize, rows: impl Iterator<Item=u32>, out: &mut Vec<u64>) {
    match base {
        Native::Fact(i) => {
            match f.fact(usize::from(i)) {
                Some(t) => {
                    let col = t.columns().get(bc);
                    out.extend(rows.map(|r| col.at(r as usize)));
                }
                None => out.extend(rows.map(|_| NULL)),
            }
        }
        Native::Tree(_) if bc == 1 => out.extend(rows.map(|r| u64::from(run.kind_sym[usize::from(f.kind[r as usize])]))),
        Native::Tree(_) | Native::Location | Native::Child(_) if matches!((base, bc), (Native::Tree(_) | Native::Location, 0) | (Native::Tree(_), 2 | 3) | (Native::Child(_), 0 | 1 | 3 | 4)) => {
            out.extend(rows.map(|r| cell(f, run, base, r, bc)));
        }
        _ => out.extend(rows.map(|r| f.native_get(run, base, r, bc))),
    }
}

pub(crate) fn mark(f: &File<'_>, base: Native) {
    if let Native::Fact(i) = base {
        f.mark_read(i);
    }
}
