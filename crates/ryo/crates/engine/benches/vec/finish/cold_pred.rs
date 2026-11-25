//! Typed selection-vector loops over provider columns. Compound predicates bind
//! their row functions during plan load and preserve Boolean short-circuit order.
use super::fact_view::Columns;
use super::cold_word::{self, Test, Word};
use super::{E, File, NULL, Native, Op, Run, Ty, ViewSpec};

type Row = Box<dyn Fn(&File<'_>, &mut Run, Columns<'_>, u32) -> bool>;
enum Action {
    Kind(Box<[bool]>),
    Field(Box<[bool]>, bool),
    Token([bool; 3]),
    Text(std::rc::Rc<super::cold_text::Predicate>),
    File(Test),
    Fact(usize, Word),
    Pair(usize, usize, bool),
    Rows(Row),
}

impl Action {
    fn retain(&self, f: &File<'_>, run: &mut Run, cols: Columns<'_>, rows: &mut Vec<u32>) {
        match self {
            Self::Kind(allowed) => rows.retain(|r| allowed[usize::from(f.kind[*r as usize])]),
            Self::Field(allowed, other) => rows.retain(|r| {
                allowed
                    .get(usize::from(f.field[*r as usize]))
                    .copied()
                    .unwrap_or(*other)
            }),
            Self::Token(allowed) => rows.retain(|r| allowed[(*r % 3) as usize]),
            Self::Text(test) => rows.retain(|r| test.accepts(f, run, *r)),
            Self::File(test) => {
                if !test(run, u64::from(f.file)) {
                    rows.clear();
                }
            }
            Self::Fact(c, test) => test.retain(run, cols.get(*c), rows),
            Self::Pair(a, b, want) => {
                let (lhs, rhs) = (cols.get(*a), cols.get(*b));
                rows.retain(|r| {
                    let i = *r as usize;
                    (lhs.at(i) != NULL && lhs.at(i) == rhs.at(i)) == *want
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
            Self::Text(test) => Box::new(move |f, run, _, r| test.accepts(f, run, r)),
            Self::File(test) => Box::new(move |f, run, _, _| test(run, u64::from(f.file))),
            Self::Fact(c, test) => {
                let test = test.bind();
                Box::new(move |_, run, cols, r| test(run, cols.get(c).at(r as usize)))
            }
            Self::Pair(a, b, want) => Box::new(move |_, _, cols, r| {
                let x = cols.get(a).at(r as usize);
                (x != NULL && x == cols.get(b).at(r as usize)) == want
            }),
            Self::Rows(test) => test,
        }
    }
}

fn action(run: &mut Run, base: Native, e: &E) -> Option<Action> {
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
            (Native::Child, 3) if run.file_field_codes => {
                let test = word.bind();
                Action::Rows(Box::new(move |f, run, _, r| {
                    let field = f.native_get(run, Native::Child, r, 3);
                    test(run, field)
                }))
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
            (Native::Tree, 2) => {
                Action::Text(super::cold_text::Predicate::compile(run, e, word.bind()))
            }
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
                    let (x, y) = (cols.get(a).at(r as usize), cols.get(b).at(r as usize));
                    x != NULL && y != NULL && accept((x as i64).cmp(&(y as i64)))
                }),
                (Ty::Sym, Ty::Sym | Ty::Num | Ty::Flag | Ty::File)
                | (Ty::Num | Ty::Flag | Ty::File, Ty::Sym) => {
                    let (left, right) = (cold_word::as_bytes(*ta), cold_word::as_bytes(*tb));
                    Box::new(move |_, run, cols, r| {
                        let (x, y) = (cols.get(a).at(r as usize), cols.get(b).at(r as usize));
                        x != NULL && y != NULL && accept(left(run, x).cmp(&right(run, y)))
                    })
                }
                _ => Box::new(move |_, _, cols, r| {
                    let (x, y) = (cols.get(a).at(r as usize), cols.get(b).at(r as usize));
                    x != NULL && y != NULL && accept(x.cmp(&y))
                }),
            };
            Some(Action::Rows(test))
        }
        _ => None,
    }
}

// When every conjunct reads the same column, fetch each cell only once.
fn single_column(run: &Run, tests: &[E]) -> Option<(usize, Word)> {
    let (col, ty) = cold_word::column(tests.first()?)?;
    let mut words = tests
        .iter()
        .map(|e| cold_word::compile(run, e, col, ty))
        .collect::<Option<Vec<_>>>()?;
    let word = if words.len() == 1 {
        words.pop()?
    } else {
        let bound: Vec<_> = words.into_iter().map(Word::bind).collect();
        Word::Test(Box::new(move |run, value| {
            bound.iter().all(|test| test(run, value))
        }))
    };
    Some((col, word))
}

struct Literal {
    column: usize,
    words: Vec<u32>,
    captured: Option<usize>,
    complete: bool,
}

enum Entry {
    Generic,
    Text {
        slots: Vec<usize>,
        captured: usize,
    },
    Fact {
        family: u16,
        literal: Option<Literal>,
        scalar: Option<(usize, Word)>,
    },
}

pub(super) struct Program {
    base: Native,
    actions: Vec<Action>,
    entry: Entry,
}
impl Program {
    pub(super) fn compile(run: &mut Run, v: &ViewSpec) -> Option<Self> {
        let mut actions: Vec<_> = v
            .filter
            .iter()
            .map(|e| action(run, v.base, e))
            .collect::<Option<_>>()?;
        // A probe has one node position. Bind all kind-only conjuncts to a
        // single lookup in the same vocabulary as the selection-vector loop.
        if let Some((Action::Kind(first), rest)) = actions.split_first_mut()
            && !rest.is_empty()
            && rest.iter().all(|action| matches!(action, Action::Kind(_)))
        {
            for action in rest {
                let Action::Kind(mask) = action else { unreachable!() };
                for (keep, &bit) in first.iter_mut().zip(mask.iter()) {
                    *keep &= bit;
                }
            }
            actions.truncate(1);
        }
        let entry = match v.base {
            Native::Tree => v
                .filter
                .iter()
                .enumerate()
                .find_map(|(captured, e)| {
                    let mut words = super::cold_list::members(e, 2)?;
                    words.sort_unstable();
                    words.dedup();
                    Some(Entry::Text {
                        slots: super::cold_text::slots(run, &words),
                        captured,
                    })
                })
                .unwrap_or(Entry::Generic),
            Native::Fact(family) => {
                let scalar = single_column(run, &v.filter);
                let literal = super::leaf_scan::fact_entry(run, family, &v.filter).map(
                    |(col, _, mut words)| {
                        words.sort_unstable();
                        words.dedup();
                        let captured = v.filter.iter().position(|e| {
                            super::cold_list::members(e, col)
                                .is_some_and(|xs| words.iter().all(|w| xs.contains(w)))
                        });
                        let complete = if let Some((c, word)) = &scalar
                            && *c == col
                        {
                            words.retain(|w| word.accepts(run, u64::from(*w)));
                            true
                        } else {
                            false
                        };
                        Literal {
                            column: col,
                            words,
                            captured,
                            complete,
                        }
                    },
                );
                Entry::Fact {
                    family,
                    literal,
                    scalar,
                }
            }
            _ => Entry::Generic,
        };
        Some(Self {
            base: v.base,
            actions,
            entry,
        })
    }

    pub(super) fn kind_mask(&self) -> Option<&[bool]> {
        match self.actions.as_slice() {
            [Action::Kind(bits)] => Some(bits),
            _ => None,
        }
    }

    pub(super) fn rows(
        &self,
        f: &File<'_>,
        run: &mut Run,
    ) -> Option<(std::rc::Rc<Vec<u32>>, usize, u64)> {
        if matches!(self.entry, Entry::Generic) {
            return None;
        }
        let mut entered = 0;
        let mut attempts = 0;
        let rows = super::recycle::rows_with(|out| {
            let (cols, skip): (Columns<'_>, _) = match &self.entry {
                Entry::Text { slots, captured } => {
                    let start = phase_clock();
                    let indexed = super::cold_text::candidates(f, run, slots, out);
                    phase("text literal entry including postings", start);
                    entered = indexed + out.len();
                    (Columns::Owned(&[]), Some(*captured))
                }
                Entry::Fact {
                    family,
                    literal,
                    scalar,
                } => {
                    if super::fill_prof::counting() {
                        f.mark_read(*family);
                    }
                    let Some(table) = f.fact(usize::from(*family)) else {
                        return;
                    };
                    let cols = table.columns();
                    if let Some(key) = literal
                        && table.ty(key.column) == Ty::Sym
                    {
                        if key.words.is_empty() || table.len() == 0 {
                            return;
                        }
                        if sw!("VEC_COLD_COUNT") {
                            let fresh = !f.fact_ix.borrow().contains_key(&(*family, key.column));
                            fact_counts(|n| {
                                n[0] += 1;
                                if fresh {
                                    n[1] += table.len() as u64;
                                }
                            });
                        }
                        let start = phase_clock();
                        let ix = f.fact_list(*family, key.column);
                        let mut streams = 0;
                        for w in &key.words {
                            if let Some(rows) = ix.get(&u64::from(*w))
                                && !rows.is_empty()
                            {
                                streams += 1;
                                out.extend_from_slice(rows);
                            }
                        }
                        if streams > 1 {
                            out.sort_unstable();
                        }
                        phase("fact literal entry", start);
                        entered = out.len();
                        if sw!("VEC_COLD_COUNT") {
                            fact_counts(|n| n[2] += entered as u64);
                        }
                        if key.complete {
                            return;
                        }
                        (cols, key.captured)
                    } else {
                        entered = table.len();
                        if let Some((c, word)) = scalar {
                            let start = phase_clock();
                            word.scan(run, cols.get(*c), out);
                            phase("fact column scan", start);
                            if sw!("VEC_COLD_COUNT") {
                                fact_counts(|n| {
                                    n[3] += 1;
                                    n[4] += table.len() as u64;
                                });
                            }
                            attempts = table.len() as u64;
                            return;
                        }
                        if let Some(Action::Fact(c, test)) = self.actions.first() {
                            let start = phase_clock();
                            test.scan(run, cols.get(*c), out);
                            phase("fact column scan", start);
                            if sw!("VEC_COLD_COUNT") {
                                fact_counts(|n| {
                                    n[3] += 1;
                                    n[4] += table.len() as u64;
                                });
                            }
                            attempts = table.len() as u64;
                            (cols, Some(0))
                        } else {
                            out.extend(0..table.len() as u32);
                            (cols, None)
                        }
                    }
                }
                Entry::Generic => unreachable!(),
            };
            for (i, action) in self.actions.iter().enumerate() {
                if out.is_empty() {
                    break;
                }
                if Some(i) != skip {
                    if sw!("VEC_COLD_COUNT") {
                        attempts += out.len() as u64;
                    }
                    action.retain(f, run, cols, out);
                }
            }
        });
        Some((rows, entered, attempts))
    }
    pub(super) fn retain(&self, f: &File<'_>, run: &mut Run, rows: &mut Vec<u32>) {
        let cols = if let Native::Fact(i) = self.base {
            if super::fill_prof::counting() {
                f.mark_read(i);
            }
            if rows.is_empty() {
                return;
            }
            f.fact(usize::from(i)).expect("entered fact rows").columns()
        } else {
            Columns::Owned(&[])
        };
        for action in &self.actions {
            if rows.is_empty() {
                break;
            }
            action.retain(f, run, cols, rows);
        }
    }
}

thread_local! { static FACTS: std::cell::Cell<[u64;5]> = const { std::cell::Cell::new([0;5]) }; }
fn fact_counts(add: impl FnOnce(&mut [u64; 5])) {
    FACTS.with(|state| {
        let mut n = state.get();
        add(&mut n);
        state.set(n);
    });
}
#[inline]
pub(super) fn phase_clock() -> f64 {
    if sw!("VEC_COLD_PHASE") {
        super::cold_list::clock()
    } else {
        0.0
    }
}
#[inline]
pub(super) fn phase(label: &'static str, start: f64) {
    if sw!("VEC_COLD_PHASE") {
        let elapsed = super::cold_list::clock() - start;
        PHASES.with_borrow_mut(|all| {
            let n = all.entry(label).or_default();
            n.0 += 1;
            n.1 += elapsed;
        });
    }
}
thread_local! {
    static PHASES: std::cell::RefCell<std::collections::BTreeMap<&'static str,(u64,f64)>> = const { std::cell::RefCell::new(std::collections::BTreeMap::new()) };
}
pub(super) fn report() {
    PHASES.with_borrow(|all| {
        for (label, (calls, seconds)) in all {
            println!("COLD_SOURCE_PHASE\t{label}\t{calls}\t{seconds:.9}");
        }
    });
    if sw!("VEC_COLD_COUNT") {
        let [keyed, indexed, ids, scans, cells] = FACTS.with(|state| state.get());
        println!("COLD_FACT\t{keyed}\t{indexed}\t{ids}\t{scans}\t{cells}");
    }
}
