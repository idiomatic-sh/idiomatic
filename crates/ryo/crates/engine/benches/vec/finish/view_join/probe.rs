//! A native equality probe reads the provider's ordered hits directly. The outer
//! select retains its driver order, residual predicates, touched count and projection.
use super::{File, Native, Run, Src, ViewSpec, fill_prof, get, view_cols};
use ryo::languages::{Position, node_table};

#[derive(Clone, Copy)]
pub(super) enum Ids<'a> {
    Slice(&'a [u32]),
    Range(u32),
    View(&'a super::cold_bucket::Rows<'a>),
}

impl Ids<'_> {
    pub(super) fn len(self) -> usize {
        match self {
            Self::Slice(ids) => ids.len(),
            Self::Range(n) => n as usize,
            Self::View(rows) => rows.len(),
        }
    }

    #[inline]
    fn visit(self, f: impl FnMut(u32)) {
        match self {
            Self::Slice(ids) => ids.iter().copied().for_each(f),
            Self::Range(n) => (0..n).for_each(f),
            Self::View(rows) => rows.iter().copied().for_each(f),
        }
    }
}

pub(super) fn supports(f: &File<'_>, v: &ViewSpec, col: usize) -> bool {
    let singleton = matches!(
        (v.base, col),
        (Native::Tree | Native::Location, 0) | (Native::Child, 1)
    );
    (singleton || matches!((v.base, col), (Native::Child, 0) | (Native::Fact(_), _)))
        && Filter::bind(f, v, singleton).is_some()
}

pub(super) struct Driver<'a> {
    pub(super) rows: Ids<'a>,
    pub(super) source: usize,
    pub(super) column: usize,
}

enum Filter<'a> {
    All,
    Codes(&'a [u16], &'a [u16]),
    Tests,
}

impl<'a> Filter<'a> {
    fn bind(f: &'a File<'_>, v: &'a ViewSpec, single: bool) -> Option<Self> {
        if v.tests.is_empty() {
            return Some(Self::All);
        }
        if let Some(entry) = v.cold_bucket.get() {
            let column = if v.base == Native::Child {
                &f.field
            } else {
                &f.kind
            };
            return Some(Self::Codes(column, entry.codes()));
        }
        // A generic expression can intern text. Preserve evaluation across rows
        // for those filters. A singleton has the original evaluation order.
        if single || matches!(v.base, Native::Fact(_)) && v.tests.iter().all(fact_test) {
            return Some(Self::Tests);
        }
        None
    }

    #[inline]
    fn accepts(&self, f: &File<'_>, run: &mut Run, v: &ViewSpec, row: u32) -> bool {
        match self {
            Self::All => true,
            Self::Codes(column, codes) => codes.contains(&column[row as usize]),
            Self::Tests => view_cols::row_holds(f, run, v, row),
        }
    }
}

fn fact_test(test: &view_cols::Test) -> bool {
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
pub(super) fn pairs(
    f: &File<'_>,
    run: &mut Run,
    srcs: &[Src],
    driver: Driver<'_>,
    view: &ViewSpec,
    column: usize,
    emit: impl FnMut(&mut Run, u32, u32),
) -> Option<u64> {
    let native = (view.base, column);
    let singleton = matches!(
        native,
        (Native::Tree | Native::Location, 0) | (Native::Child, 1)
    );
    let filter = Filter::bind(f, view, singleton)?;
    let mut batch = Batch {
        f,
        run,
        srcs,
        driver,
        view,
        filter,
        emit,
    };
    let n = f.n as u64;
    let count = match native {
        (Native::Tree | Native::Location, 0) => batch.drive(|key| (key < n).then_some(key as u32)),
        (Native::Child, 1) => {
            batch.drive(|key| (key < n && f.has_parent(key as u32)).then_some(key as u32))
        }
        (Native::Child, 0) => batch.drive(|key| {
            let children = if key < n {
                node_table::children_of(f.table, Position(key as u32))
            } else {
                &[]
            };
            children.iter().filter_map(move |child| {
                (child.0 != 0 && u64::from(f.parent[child.0 as usize]) == key).then_some(child.0)
            })
        }),
        (Native::Fact(i), c) => {
            if batch.driver.rows.len() == 0 {
                return Some(0);
            }
            if fill_prof::counting() {
                f.mark_read(i);
            }
            let index = f.fact_list(i, c);
            batch.drive(|key| index.get(&key).unwrap_or_default().iter().copied())
        }
        _ => return None,
    };
    Some(count)
}

struct Batch<'a, 'file, F> {
    f: &'a File<'file>,
    run: &'a mut Run,
    srcs: &'a [Src],
    driver: Driver<'a>,
    view: &'a ViewSpec,
    filter: Filter<'a>,
    emit: F,
}

impl<F: FnMut(&mut Run, u32, u32)> Batch<'_, '_, F> {
    #[inline]
    fn drive<I: IntoIterator<Item = u32>>(&mut self, hits: impl Fn(u64) -> I) -> u64 {
        let mut touched = 0;
        let table_keys = match &self.srcs[self.driver.source] {
            Src::Tab(t, _) => Some(t.cols[self.driver.column].as_slice()),
            _ => None,
        };
        self.driver.rows.visit(|left| {
            let key = match table_keys {
                Some(keys) => keys[left as usize],
                None => get(
                    self.run,
                    self.f,
                    self.srcs,
                    &[left; 2],
                    self.driver.source,
                    self.driver.column,
                ),
            };
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
