//! Typed selection-vector loops over provider columns. Compound predicates bind
//! their row functions during plan load and preserve Boolean short-circuit order.
use super::cold_word::{self, Test, Word};
use super::{E, File, NULL, Native, Op, Run, Ty, ViewSpec};

type Row = Box<dyn Fn(&File<'_>, &mut Run, &[Vec<u64>], u32) -> bool>;
enum Action {
    Kind(Box<[bool]>),
    Field(Box<[bool]>, bool),
    Token([bool; 3]),
    Text(Test),
    File(Test),
    Fact(usize, Word),
    Pair(usize, usize, bool),
    Rows(Row),
}

impl Action {
    fn retain(&self, f: &File<'_>, run: &mut Run, cols: &[Vec<u64>], rows: &mut Vec<u32>) {
        match self {
            Self::Kind(allowed) => rows.retain(|r| allowed[usize::from(f.kind[*r as usize])]),
            Self::Field(allowed, other) => rows.retain(|r| {
                allowed
                    .get(usize::from(f.field[*r as usize]))
                    .copied()
                    .unwrap_or(*other)
            }),
            Self::Token(allowed) => rows.retain(|r| allowed[(*r % 3) as usize]),
            Self::Text(test) => rows.retain(|r| {
                let text = f.elem_text(run, *r);
                test(run, u64::from(text))
            }),
            Self::File(test) => {
                if !test(run, u64::from(f.file)) {
                    rows.clear();
                }
            }
            Self::Fact(c, test) => test.retain(run, &cols[*c], rows),
            Self::Pair(a, b, want) => {
                let (lhs, rhs) = (&cols[*a], &cols[*b]);
                rows.retain(|r| {
                    let i = *r as usize;
                    (lhs[i] != NULL && lhs[i] == rhs[i]) == *want
                });
            }
            Self::Rows(test) => rows.retain(|r| test(f, run, cols, *r)),
        }
    }
    fn bind(self) -> Row {
        match self {
            Self::Kind(set) => Box::new(move |f, _, _, r| set[usize::from(f.kind[r as usize])]),
            Self::Field(set, other) => Box::new(move |f, _, _, r| {
                set.get(usize::from(f.field[r as usize]))
                    .copied()
                    .unwrap_or(other)
            }),
            Self::Token(set) => Box::new(move |_, _, _, r| set[(r % 3) as usize]),
            Self::Text(test) => Box::new(move |f, run, _, r| {
                let text = f.elem_text(run, r);
                test(run, u64::from(text))
            }),
            Self::File(test) => Box::new(move |f, run, _, _| test(run, u64::from(f.file))),
            Self::Fact(c, test) => {
                let test = test.bind();
                Box::new(move |_, run, cols, r| test(run, cols[c][r as usize]))
            }
            Self::Pair(a, b, want) => Box::new(move |_, _, cols, r| {
                let x = cols[a][r as usize];
                (x != NULL && x == cols[b][r as usize]) == want
            }),
            Self::Rows(test) => test,
        }
    }
}

fn action(run: &Run, base: Native, e: &E) -> Option<Action> {
    if let Some((c, ty)) = cold_word::column(e)
        && let Some(word) = cold_word::compile(run, e, c, ty)
    {
        return Some(match (base, c) {
            (Native::Tree, 1) => {
                let test = word.bind();
                Action::Kind(
                    run.kind_sym
                        .iter()
                        .map(|s| test(run, u64::from(*s)))
                        .collect(),
                )
            }
            (Native::Child, 3) => {
                let test = word.bind();
                Action::Field(
                    run.field_sym
                        .iter()
                        .map(|s| test(run, u64::from(*s)))
                        .collect(),
                    test(run, u64::from(run.empty)),
                )
            }
            (Native::Tree, 2) => Action::Text(word.bind()),
            (Native::Token, 2) => {
                let test = word.bind();
                Action::Token(run.link_sym.map(|s| test(run, u64::from(s))))
            }
            (Native::Tree | Native::Token, 3) | (Native::Child, 4) | (Native::Location, 1) => {
                Action::File(word.bind())
            }
            (Native::Fact(_), _) => Action::Fact(c, word),
            _ => return None,
        });
    }
    match e {
        E::And(a, b) | E::Or(a, b) => {
            let left = action(run, base, a)?.bind();
            let right = action(run, base, b)?.bind();
            let row: Row = if matches!(e, E::And(..)) {
                Box::new(move |f, s, c, r| left(f, s, c, r) && right(f, s, c, r))
            } else {
                Box::new(move |f, s, c, r| left(f, s, c, r) || right(f, s, c, r))
            };
            Some(Action::Rows(row))
        }
        E::Not(a) => {
            let test = action(run, base, a)?.bind();
            Some(Action::Rows(Box::new(move |f, s, c, r| !test(f, s, c, r))))
        }
        E::Cmp(op, a, b) if matches!(base, Native::Fact(_)) => {
            let (E::Col(0, a, ta), E::Col(0, b, tb)) = (&**a, &**b) else {
                return None;
            };
            if ta == tb && matches!(op, Op::Eq | Op::Ne) {
                return Some(Action::Pair(*a, *b, *op == Op::Eq));
            }
            let accept = cold_word::relation(*op, false)?;
            let (a, b) = (*a, *b);
            let test: Row = match (*ta, *tb) {
                (Ty::Num, Ty::Num) => Box::new(move |_, _, cols, r| {
                    let (x, y) = (cols[a][r as usize], cols[b][r as usize]);
                    x != NULL && y != NULL && accept((x as i64).cmp(&(y as i64)))
                }),
                (Ty::Sym, Ty::Sym | Ty::Num | Ty::Flag | Ty::File)
                | (Ty::Num | Ty::Flag | Ty::File, Ty::Sym) => {
                    let (left, right) = (cold_word::as_bytes(*ta), cold_word::as_bytes(*tb));
                    Box::new(move |_, run, cols, r| {
                        let (x, y) = (cols[a][r as usize], cols[b][r as usize]);
                        x != NULL && y != NULL && accept(left(run, x).cmp(&right(run, y)))
                    })
                }
                _ => Box::new(move |_, _, cols, r| {
                    let (x, y) = (cols[a][r as usize], cols[b][r as usize]);
                    x != NULL && y != NULL && accept(x.cmp(&y))
                }),
            };
            Some(Action::Rows(test))
        }
        _ => None,
    }
}

pub(super) struct Program {
    base: Native,
    actions: Vec<Action>,
}
impl Program {
    pub(super) fn compile(run: &Run, v: &ViewSpec) -> Option<Self> {
        let actions = v
            .filter
            .iter()
            .map(|e| action(run, v.base, e))
            .collect::<Option<_>>()?;
        Some(Self {
            base: v.base,
            actions,
        })
    }
    pub(super) fn retain(&self, f: &File<'_>, run: &mut Run, rows: &mut Vec<u32>) {
        let cols = if let Native::Fact(i) = self.base {
            if super::fill_prof::counting() {
                f.mark_read(i);
            }
            if rows.is_empty() {
                return;
            }
            &f.facts[usize::from(i)]
                .as_ref()
                .expect("entered fact rows")
                .cols[..]
        } else {
            &[]
        };
        for action in &self.actions {
            if rows.is_empty() {
                break;
            }
            action.retain(f, run, cols, rows);
        }
    }
}
