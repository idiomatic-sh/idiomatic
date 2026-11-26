//! A native equality probe reads the provider's ordered hits directly. The outer
//! select retains its driver order, residual predicates, touched count and projection.
use super::{Extent, File, Native, Run, Src, ViewSpec, fill_prof, view_cols};
use super::select_kernel::Pushed;

#[derive(Clone, Copy)]
pub enum Ids<'a> {
    Slice(&'a [u32]),
    Range(u32),
    View(&'a super::cold_bucket::Rows<'a>),
}

impl Ids<'_> {
    pub fn len(self) -> usize {
        match self {
            Self::Slice(ids) => ids.len(),
            Self::Range(n) => n as usize,
            Self::View(rows) => rows.len(),
        }
    }

    #[inline]
    pub fn visit(self, f: impl FnMut(u32)) {
        match self {
            Self::Slice(ids) => ids.iter().copied().for_each(f),
            Self::Range(n) => (0..n).for_each(f),
            Self::View(rows) => rows.iter().copied().for_each(f),
        }
    }
}

pub fn supports(f: &File<'_>, v: &ViewSpec, col: usize) -> bool {
    let singleton = matches!(
        (v.base, col),
        (Native::Tree(_) | Native::Location, 0) | (Native::Child(_), 1)
    );
    (singleton || matches!((v.base, col), (Native::Child(_), 0) | (Native::Fact(_), _)))
        && Filter::bind(f, v, singleton).is_some()
}

pub struct Driver<'a> {
    pub rows: Ids<'a>,
    pub source: usize,
    pub column: usize,
}

pub enum Filter<'a> {
    All,
    Code(&'a [u16], u16),
    Codes(&'a [u16], &'a [u16]),
    KindMask(&'a [u16], &'a [bool]),
    Tests,
}

impl<'a> Filter<'a> {
    pub fn bind(f: &'a File<'_>, v: &'a ViewSpec, single: bool) -> Option<Self> {
        if v.tests.is_empty() {
            return Some(Self::All);
        }
        if let Some(entry) = v.cold_bucket.get() {
            let column = if v.base.is_child() {
                &f.field
            } else {
                &f.kind
            };
            return Some(match entry.codes() {
                [code] => Self::Code(column, *code),
                codes => Self::Codes(column, codes),
            });
        }
        if single
            && let Some(super::cold_test::Kernel::Columns(_, program)) = v.cold_test.get().map(|k| k.as_ref())
            && let Some(mask) = program.kind_mask()
        {
            return Some(Self::KindMask(&f.kind, mask));
        }
        // A generic expression can intern text. Preserve evaluation across rows
        // for those filters. A singleton has the original evaluation order.
        if single || matches!(v.base, Native::Fact(_)) && v.tests.iter().all(fact_test) {
            return Some(Self::Tests);
        }
        None
    }

    #[inline]
    pub fn accepts(&self, f: &File<'_>, run: &mut Run, v: &ViewSpec, row: u32) -> bool {
        match self {
            Self::All => true,
            Self::Code(column, code) => column[row as usize] == *code,
            Self::Codes(column, codes) => codes.contains(&column[row as usize]),
            Self::KindMask(kinds, mask) => mask[usize::from(kinds[row as usize])],
            Self::Tests => view_cols::row_holds(f, run, v, row),
        }
    }
}

pub fn fact_test(test: &view_cols::Test) -> bool {
    use view_cols::Test;
    match test {
        Test::Eval(_) => false,
        Test::Not(t) => fact_test(t),
        Test::And(l, r) | Test::Or(l, r) => fact_test(l) && fact_test(r),
        _ => true,
    }
}

/// Each supported native key yields a singleton or an iterator into existing
/// storage. The file's fact index is obtained once for the entire driving batch.
pub fn pairs(
    f: &File<'_>,
    run: &mut Run,
    srcs: &[Src],
    driver: Driver<'_>,
    view: &ViewSpec,
    column: usize,
    emit: impl FnMut(&mut Run, u32, u32),
) -> Option<u64> {
    pairs_where(f, run, srcs, driver, view, column, |_, _| true, emit)
}

/// Apply the same source-local test in either physical join direction.
pub fn filtered_pairs(
    f: &File<'_>,
    run: &mut Run,
    srcs: &[Src],
    driver: Driver<'_>,
    view: &ViewSpec,
    column: usize,
    pushed: Option<&Pushed>,
    mut emit: impl FnMut(&mut Run, u32, u32),
) -> Option<u64> {
    match pushed {
        Some(test) if test.source == driver.source => pairs_where(
            f, run, srcs, driver, view, column,
            |run, row| test.keeps(run, f, srcs, row), emit,
        ),
        Some(test) => pairs(f, run, srcs, driver, view, column, |run, left, right| {
            if test.keeps(run, f, srcs, right) {
                emit(run, left, right);
            }
        }),
        None => pairs(f, run, srcs, driver, view, column, emit),
    }
}

pub fn pairs_where(
    f: &File<'_>,
    run: &mut Run,
    srcs: &[Src],
    driver: Driver<'_>,
    view: &ViewSpec,
    column: usize,
    keep: impl FnMut(&mut Run, u32) -> bool,
    emit: impl FnMut(&mut Run, u32, u32),
) -> Option<u64> {
    let native = (view.base, column);
    let singleton = matches!(
        native,
        (Native::Tree(_) | Native::Location, 0) | (Native::Child(_), 1)
    );
    let filter = Filter::bind(f, view, singleton)?;
    let mut batch = Batch {
        f,
        run,
        srcs,
        driver,
        view,
        filter,
        keep,
        emit,
    };
    let n = f.limit(native.0.extent()) as u64;
    let count = match native {
        (Native::Tree(_) | Native::Location, 0) => batch.drive(|key| (key < n).then_some(key as u32)),
        (Native::Child(_), 1) => {
            batch.drive(|key| (key < n && f.has_parent(key as u32)).then_some(key as u32))
        }
        (Native::Child(_), 0) => batch.drive(|key| f.child_rows(key).filter(move |c| u64::from(*c) < n)),
        (Native::Fact(i), c) => {
            if batch.driver.rows.len() == 0 {
                return Some(0);
            }
            if fill_prof::counting() {
                f.mark_read(i);
            }
            let table = f.fact(usize::from(i));
            match table.map_or(0, |t| t.len()) {
                0 => batch.drive(|_| None),
                1 => {
                    let only = table.expect("one fact row").columns().get(c).at(0);
                    batch.drive(|key| (key == only).then_some(0))
                }
                _ => {
                    let index = f.fact_list(i, c);
                    batch.drive(|key| index.get(&key).unwrap_or_default().iter().copied())
                }
            }
        }
        _ => return None,
    };
    Some(count)
}

pub struct Batch<'a, 'file, F, K> {
    pub f: &'a File<'file>,
    pub run: &'a mut Run,
    pub srcs: &'a [Src],
    pub driver: Driver<'a>,
    pub view: &'a ViewSpec,
    pub filter: Filter<'a>,
    pub emit: F,
    pub keep: K,
}

impl<F: FnMut(&mut Run, u32, u32), K: FnMut(&mut Run, u32) -> bool> Batch<'_, '_, F, K> {
    #[inline]
    pub fn drive<I: IntoIterator<Item = u32>>(&mut self, hits: impl Fn(u64) -> I) -> u64 {
        let mut touched = 0;
        let key_column =
            super::view_key::Key::bind(self.f, &self.srcs[self.driver.source], self.driver.column);
        self.driver.rows.visit(|left| {
            if !(self.keep)(self.run, left) {
                return;
            }
            let key = key_column.read(self.f, self.run, left);
            fill_prof::read(4);
            if sw!("VEC_COLD_COUNT") {
                self.run.cold.probe(self.f, None);
            }
            for right in hits(key) {
                if self.filter.accepts(self.f, self.run, self.view, right) {
                    touched += 1;
                    (self.emit)(self.run, left, right);
                }
            }
        });
        touched
    }
}
