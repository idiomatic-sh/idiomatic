//! A cache slot can name an ordered slice in this file's provider table. Multiple
//! disjoint slices keep their lengths and are merged only on the first ordered read.
use super::{File, KindBits, NodeTable, Position, node_table, recycle};
use ryo::languages::FieldCode;
use std::cell::{OnceCell, RefCell};
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::rc::Rc;

#[derive(Clone, Copy)]
enum Bucket {
    Field(u16),
    Kind(u16),
}
impl Bucket {
    fn rows(self, t: &NodeTable) -> &[Position] {
        match self {
            Self::Field(c) => node_table::ids_of_field(t, FieldCode(c)),
            Self::Kind(c) => node_table::ids_of_kind(t, KindBits(c)),
        }
    }
}
#[derive(Clone, Copy)]
pub(super) struct Part {
    bucket: Bucket,
    lo: usize,
    hi: usize,
}
impl Part {
    fn rows(self, t: &NodeTable) -> &[Position] {
        &self.bucket.rows(t)[self.lo..self.hi]
    }
}

pub(super) struct Merge {
    parts: Vec<Part>,
    len: usize,
    form: &'static str,
    ordered: OnceCell<Rc<Vec<u32>>>,
}
impl Merge {
    fn write(&self, table: &NodeTable, out: &mut Vec<u32>) {
        out.reserve(self.len);
        let mut streams: Vec<_> = self.parts.iter().map(|p| p.rows(table).iter()).collect();
        let mut heap: BinaryHeap<_> = streams
            .iter_mut()
            .enumerate()
            .filter_map(|(i, rows)| rows.next().map(|p| Reverse((p.0, i))))
            .collect();
        while let Some(mut top) = heap.peek_mut() {
            let Reverse((id, i)) = *top;
            out.push(id);
            if let Some(next) = streams[i].next() {
                *top = Reverse((next.0, i));
            } else {
                let _ = std::collections::binary_heap::PeekMut::pop(top);
            }
        }
    }
    fn read(&self, table: &NodeTable) -> &[u32] {
        self.ordered.get_or_init(|| {
            let start = super::cold_list::clock();
            let rows = recycle::rows_with(|out| self.write(table, out));
            if super::cold_list::auditing() {
                let seconds = super::cold_list::clock() - start;
                record(self.form, "merge read", self.len, seconds);
            }
            rows
        })
    }
}

#[derive(Clone)]
pub(super) enum Slot {
    Owned(Rc<Vec<u32>>),
    Empty,
    Slice(Part),
    Merge(Rc<Merge>),
}
impl From<Rc<Vec<u32>>> for Slot {
    fn from(rows: Rc<Vec<u32>>) -> Self {
        Self::Owned(rows)
    }
}
impl Slot {
    pub(super) fn len(&self) -> usize {
        match self {
            Self::Owned(rows) => rows.len(),
            Self::Slice(p) => p.hi - p.lo,
            Self::Merge(m) => m.len,
            Self::Empty => 0,
        }
    }
    pub(super) fn owned(&self) -> &Rc<Vec<u32>> {
        match self {
            Self::Owned(r) => r,
            _ => unreachable!("owned auxiliary slot"),
        }
    }
    pub(super) fn resolve<'a>(&self, table: &'a NodeTable) -> Rows<'a> {
        match self {
            Self::Owned(r) => Rows::Owned(r.clone()),
            Self::Slice(p) => Rows::Provider(p.rows(table)),
            Self::Empty => Rows::Provider(&[]),
            Self::Merge(m) => Rows::Merge(table, m.clone()),
        }
    }
    pub(super) fn recycle(self) {
        match self {
            Self::Owned(r) => recycle::recycle_rows(r),
            Self::Merge(m) => {
                if let Ok(m) = Rc::try_unwrap(m)
                    && let Some(r) = m.ordered.into_inner()
                {
                    recycle::recycle_rows(r);
                }
            }
            _ => {}
        }
    }
}

pub(super) enum Rows<'a> {
    Owned(Rc<Vec<u32>>),
    Provider(&'a [Position]),
    Merge(&'a NodeTable, Rc<Merge>),
}
impl From<Rc<Vec<u32>>> for Rows<'_> {
    fn from(r: Rc<Vec<u32>>) -> Self {
        Self::Owned(r)
    }
}
impl Rows<'_> {
    pub(super) fn len(&self) -> usize {
        match self {
            Self::Owned(r) => r.len(),
            Self::Provider(p) => p.len(),
            Self::Merge(_, m) => m.len,
        }
    }
    pub(super) fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub(super) fn iter(&self) -> impl Iterator<Item = &u32> {
        let (rows, positions): (&[u32], &[Position]) = match self {
            Self::Owned(r) => (r, &[]),
            Self::Provider(p) => (&[], p),
            Self::Merge(t, m) => (m.read(t), &[]),
        };
        rows.iter().chain(positions.iter().map(|p| &p.0))
    }
    pub(super) fn extend_into(&self, out: &mut Vec<u32>) {
        match self {
            Self::Owned(r) => out.extend_from_slice(r),
            Self::Provider(p) => out.extend(p.iter().map(|p| p.0)),
            Self::Merge(t, m) => out.extend_from_slice(m.read(t)),
        }
    }
    pub(super) fn to_vec(&self) -> Vec<u32> {
        let mut ids = Vec::with_capacity(self.len());
        self.extend_into(&mut ids);
        ids
    }
    // The oracle must not force a pending merge into the production cache.
    pub(super) fn shadow(&self) -> Vec<u32> {
        match self {
            Self::Merge(t, m) => {
                let mut ids = Vec::with_capacity(m.len);
                m.write(t, &mut ids);
                ids
            }
            _ => self.to_vec(),
        }
    }
}

pub(super) fn select(f: &File<'_>, fields: bool, codes: &[u16]) -> Slot {
    let mut first = None;
    let mut more = Vec::new();
    let mut len = 0;
    let mut append = |bucket, lo, hi| {
        if lo == hi {
            return;
        }
        let p = Part { bucket, lo, hi };
        if first.is_none() {
            first = Some(p);
        } else {
            more.push(p);
        }
        len += hi - lo;
    };
    for &code in codes {
        let bucket = if fields {
            Bucket::Field(code)
        } else {
            Bucket::Kind(code)
        };
        let ids = bucket.rows(f.table);
        let mut lo = 0;
        if fields && !ids.is_empty() {
            // Extra roots are in position order.
            for (root_code, root) in f
                .field
                .first()
                .map(|c| (*c, 0))
                .into_iter()
                .chain(f.extra_roots().iter().copied())
            {
                if root_code == code {
                    let i = ids
                        .binary_search(&Position(root))
                        .expect("root in its field bucket");
                    append(bucket, lo, i);
                    lo = i + 1;
                }
            }
        }
        append(bucket, lo, ids.len());
    }
    let form = if fields { "child-field" } else { "kind" };
    let Some(part) = first else {
        if sw!("VEC_COLD_COUNT") {
            record(form, "empty", 0, 0.0);
        }
        return Slot::Empty;
    };
    if more.is_empty() {
        if sw!("VEC_COLD_COUNT") {
            record(form, "slice", len, 0.0);
        }
        return Slot::Slice(part);
    }
    more.push(part);
    if sw!("VEC_COLD_COUNT") {
        record(form, "multi", len, 0.0);
    }
    Slot::Merge(Rc::new(Merge {
        parts: more,
        len,
        form,
        ordered: OnceCell::new(),
    }))
}

thread_local! {
    static COUNTS: RefCell<std::collections::BTreeMap<(&'static str, &'static str), (u64,u64,f64)>> = const { RefCell::new(std::collections::BTreeMap::new()) };
}
fn record(form: &'static str, what: &'static str, rows: usize, seconds: f64) {
    COUNTS.with_borrow_mut(|m| {
        let v = m.entry((form, what)).or_default();
        v.0 += 1;
        v.1 += rows as u64;
        v.2 += seconds;
    });
}
thread_local! {
    static ROOTS: std::cell::Cell<[u64; 3]> = const { std::cell::Cell::new([0; 3]) };
}
pub(super) fn roots(extra: usize, capacity: usize) {
    ROOTS.with(|state| {
        let [files, roots, bytes] = state.get();
        state.set([
            files + 1,
            roots + extra as u64,
            bytes + (capacity * size_of::<(u16, u32)>()) as u64,
        ]);
    });
}
pub(super) fn report() {
    if sw!("VEC_COLD_COUNT") {
        let [files, roots, bytes] = ROOTS.with(|s| s.get());
        println!("COLD_ROOTS\t{files}\t{roots}\t{bytes}");
    }
    COUNTS.with_borrow(|m| {
        for ((form, what), (calls, rows, seconds)) in m {
            println!("COLD_BUCKET\t{form}\t{what}\t{calls}\t{rows}\t{seconds:.9}");
        }
    });
}
