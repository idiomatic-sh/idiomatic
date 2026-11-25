//! A scalar predicate with a fixed column type. Choose the comparison and literal at load.
use super::{E, NULL, Op, Run, Ty};
use std::cmp::Ordering;

pub(super) type Test = Box<dyn Fn(&Run, u64) -> bool>;
pub(super) enum Word {
    Eq(u64),
    Ne(u64),
    Test(Test),
}
impl Word {
    pub(super) fn accepts(&self, run: &Run, value: u64) -> bool {
        match self {
            Self::Eq(x) => value == *x,
            Self::Ne(x) => value != *x,
            Self::Test(test) => test(run, value),
        }
    }
    pub(super) fn bind(self) -> Test {
        match self {
            Self::Eq(x) => Box::new(move |_, w| w == x),
            Self::Ne(x) => Box::new(move |_, w| w != x),
            Self::Test(test) => test,
        }
    }
    pub(super) fn scan(&self, run: &Run, column: super::fact_view::Column<'_>, out: &mut Vec<u32>) {
        fn emit(col: super::fact_view::Column<'_>, out: &mut Vec<u32>, test: impl Fn(u64) -> bool) {
            out.extend(
                (0..col.len()).filter_map(|i| test(col.at(i)).then_some(i as u32)),
            );
        }
        match self {
            Self::Eq(x) => emit(column, out, |w| w == *x),
            Self::Ne(x) => emit(column, out, |w| w != *x),
            Self::Test(test) => emit(column, out, |w| test(run, w)),
        }
    }
    pub(super) fn retain(&self, run: &Run, column: super::fact_view::Column<'_>, rows: &mut Vec<u32>) {
        match self {
            Self::Eq(x) => rows.retain(|r| column.at(*r as usize) == *x),
            Self::Ne(x) => rows.retain(|r| column.at(*r as usize) != *x),
            Self::Test(test) => rows.retain(|r| test(run, column.at(*r as usize))),
        }
    }
}

pub(super) fn column(e: &E) -> Option<(usize, Ty)> {
    match e {
        E::Col(0, c, t) => Some((*c, *t)),
        E::Cmp(_, a, b) | E::In(a, b, _) | E::And(a, b) | E::Or(a, b) => {
            column(a).or_else(|| column(b))
        }
        E::TextTest(a, _) | E::Not(a) => column(a),
        E::Call(_, args) => args.iter().find_map(column),
        _ => None,
    }
}
fn is_col(e: &E, c: usize) -> bool {
    matches!(e,E::Col(0,n,_) if *n==c)
}

pub(super) fn relation(op: Op, reverse: bool) -> Option<fn(Ordering) -> bool> {
    Some(match (op, reverse) {
        (Op::Eq, _) => Ordering::is_eq,
        (Op::Ne, _) => Ordering::is_ne,
        (Op::Lt, false) | (Op::Gt, true) => Ordering::is_lt,
        (Op::Le, false) | (Op::Ge, true) => Ordering::is_le,
        (Op::Gt, false) | (Op::Lt, true) => Ordering::is_gt,
        (Op::Ge, false) | (Op::Le, true) => Ordering::is_ge,
        _ => return None,
    })
}

fn compare(run: &Run, ty: Ty, op: Op, literal: &E, reverse: bool) -> Option<Word> {
    let accept = relation(op, reverse)?;
    let (right, value) = match literal {
        E::Text(s) => (Ty::Sym, u64::from(*s)),
        E::Num(n) => (Ty::Num, *n as u64),
        E::Lit(v) => (v.ty(), v.1),
        _ => return None,
    };
    if ty == Ty::Sym && right == Ty::Sym && matches!(op, Op::Eq | Op::Ne) {
        return Some(if op == Op::Eq {
            Word::Eq(value)
        } else {
            Word::Ne(value)
        });
    }
    if value == NULL {
        return Some(Word::Test(Box::new(|_, _| false)));
    }
    let pred: Test = match (ty, right) {
        (Ty::Num, Ty::Num) => {
            Box::new(move |_, w| w != NULL && accept((w as i64).cmp(&(value as i64))))
        }
        (Ty::Sym, Ty::Sym | Ty::Num | Ty::Flag | Ty::File)
        | (Ty::File | Ty::Num | Ty::Flag, Ty::Sym) | (Ty::File, Ty::File) => {
            let rhs = super::text_of(run, super::V::new(right, value)).into_owned();
            match ty {
                Ty::Sym => Box::new(move |r, w| w != NULL && accept(r.text(w as u32).cmp(&rhs))),
                Ty::File => Box::new(move |r, w| {
                    w != NULL && accept(r.paths[w as usize].as_bytes().cmp(&rhs))
                }),
                Ty::Flag => {
                    let yes = accept(b"true".as_slice().cmp(&rhs));
                    let no = accept(b"false".as_slice().cmp(&rhs));
                    Box::new(move |_, w| w != NULL && if w == 1 { yes } else { no })
                }
                Ty::Num => Box::new(move |_, w| {
                    w != NULL && accept((w as i64).to_string().as_bytes().cmp(&rhs))
                }),
                _ => unreachable!(),
            }
        }
        _ => Box::new(move |_, w| w != NULL && accept(w.cmp(&value))),
    };
    Some(Word::Test(pred))
}

/// Bind a column's textual representation once, using the same conversion as text_of.
pub(super) fn as_bytes(ty: Ty) -> for<'a> fn(&'a Run, u64) -> std::borrow::Cow<'a, [u8]> {
    use std::borrow::Cow;
    match ty {
        Ty::Sym => |r, w| Cow::Borrowed(r.text(w as u32)),
        Ty::File => |r, w| Cow::Borrowed(r.paths[w as usize].as_bytes()),
        Ty::Num => |_, w| Cow::Owned((w as i64).to_string().into_bytes()),
        Ty::Flag => |_, w| Cow::Borrowed(if w == 1 { b"true" } else { b"false" }),
        _ => |_, _| Cow::Borrowed(&[]),
    }
}
fn text_predicate(ty: Ty, test: impl Fn(&[u8]) -> bool + 'static) -> Test {
    match ty {
        Ty::Sym => Box::new(move |r, w| test(r.text(w as u32))),
        Ty::File => Box::new(move |r, w| test(r.paths[w as usize].as_bytes())),
        _ => {
            let read = as_bytes(ty);
            Box::new(move |r, w| test(&read(r, w)))
        }
    }
}

/// Compiles only when the whole expression reads this single column.
pub(super) fn compile(run: &Run, e: &E, c: usize, ty: Ty) -> Option<Word> {
    if ty == Ty::Sym
        && let Some(mut ids) = super::cold_list::members(e, c)
    {
        ids.sort_unstable();
        ids.dedup();
        if let [x] = ids.as_slice() {
            return Some(Word::Eq(u64::from(*x)));
        }
        let ids = super::symbol_predicate::SymbolSet::bind(run, ids.into_iter().map(u64::from).collect());
        return Some(Word::Test(Box::new(move |_, w| {
            ids.contains(w)
        })));
    }
    let test: Test = match e {
        E::Cmp(op, a, b) if is_col(a, c) => return compare(run, ty, *op, b, false),
        E::Cmp(op, a, b) if is_col(b, c) => return compare(run, ty, *op, a, true),
        E::And(a, b) | E::Or(a, b) => {
            let l = compile(run, a, c, ty)?.bind();
            let r = compile(run, b, c, ty)?.bind();
            if matches!(e, E::And(..)) {
                Box::new(move |s, w| l(s, w) && r(s, w))
            } else {
                Box::new(move |s, w| l(s, w) || r(s, w))
            }
        }
        E::Not(x) => {
            let t = compile(run, x, c, ty)?.bind();
            Box::new(move |r, w| !t(r, w))
        }
        E::In(x, items, true) if ty == Ty::Sym && is_col(x, c) => {
            let E::List(items) = &**items else {
                return None;
            };
            let words = items
                .iter()
                .map(|e| {
                    if let E::Text(s) = e {
                        Some(u64::from(*s))
                    } else {
                        None
                    }
                })
                .collect::<Option<Vec<_>>>()?;
            let words = super::symbol_predicate::SymbolSet::bind(run, words);
            Box::new(move |_, w| !words.contains(w))
        }
        E::TextTest(x, re) if is_col(x, c) => {
            let re = re.clone();
            if ty == Ty::Sym { Box::new(move |r, w| re.symbol(r, w as u32)) }
            else if ty == Ty::File && matches!(re, super::TextTest::Glob(_) | super::TextTest::Any(_)) {
                Box::new(move |r, w| re.value(r, super::V::new(Ty::File, w)))
            } else { text_predicate(ty, move |bytes| re.bytes(bytes)) }
        }
        E::Call(name, args) => match (name.as_str(), args.as_slice()) {
            ("starts_with" | "ends_with", [x, E::Text(s)]) if is_col(x, c) => {
                let text = run.text(*s).to_owned();
                if name == "starts_with" {
                    text_predicate(ty, move |bytes| bytes.starts_with(&text))
                } else {
                    text_predicate(ty, move |bytes| bytes.ends_with(&text))
                }
            }
            ("numeric_gte", [a, b]) => {
                let (limit, reverse) = match (a, b) {
                    (x, E::Num(n)) if is_col(x, c) => (*n as f64, false),
                    (E::Num(n), x) if is_col(x, c) => (*n as f64, true),
                    _ => return None,
                };
                let accepts = move |x: f64| if reverse { limit >= x } else { x >= limit };
                match ty {
                    Ty::Num => Box::new(move |_, w| accepts(w as i64 as f64)),
                    Ty::Sym => Box::new(move |r, w| {
                        String::from_utf8_lossy(r.text(w as u32))
                            .replace('_', "")
                            .parse::<f64>()
                            .is_ok_and(accepts)
                    }),
                    _ => return None,
                }
            }
            _ => return None,
        },
        _ => return None,
    };
    Some(Word::Test(test))
}
