//! A cache slot can name an ordered slice in this file's provider table. Multiple
//! disjoint slices keep their lengths and are merged only on the first ordered read.
use super::{File, KindBits, NodeTable, Position, node_table, recycle};
use crate::languages::FieldCode;
use std::cell::{OnceCell};
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::rc::Rc;

#[derive(Clone, Copy)]
pub(crate) enum Bucket {
    Field(u16),
    Kind(u16),
}
impl Bucket {
    pub fn rows(self, t: &NodeTable) -> &[Position] {
        match self {
            Self::Field(c) => node_table::ids_of_field(t, FieldCode(c)),
            Self::Kind(c) => node_table::ids_of_kind(t, KindBits(c)),
        }
    }
}
#[derive(Clone, Copy)]
pub(crate) struct Part {
    pub(crate) bucket: Bucket,
    pub(crate) lo: usize,
    pub(crate) hi: usize,
}
impl Part {
    pub fn rows(self, t: &NodeTable) -> &[Position] {
        &self.bucket.rows(t)[self.lo..self.hi]
    }
}

pub(crate) struct Merge {
    pub(crate) parts: Vec<Part>,
    pub(crate) len: usize,
    pub(crate) form: &'static str,
    pub(crate) ordered: OnceCell<Rc<Vec<u32>>>,
}
impl Merge {
    pub(crate) fn write(&self, table: &NodeTable, out: &mut Vec<u32>) {
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
    pub(crate) fn read(&self, table: &NodeTable) -> &[u32] {
        self.ordered.get_or_init(|| {
            recycle::rows_with(|out| self.write(table, out))
        })
    }
}

#[derive(Clone)]
pub(crate) enum Slot {
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
    pub(crate) fn len(&self) -> usize {
        match self {
            Self::Owned(rows) => rows.len(),
            Self::Slice(p) => p.hi - p.lo,
            Self::Merge(m) => m.len,
            Self::Empty => 0,
        }
    }
    pub(crate) fn owned(&self) -> &Rc<Vec<u32>> {
        match self {
            Self::Owned(r) => r,
            _ => unreachable!("owned auxiliary slot"),
        }
    }
    pub(crate) fn resolve<'a>(&self, table: &'a NodeTable) -> Rows<'a> {
        match self {
            Self::Owned(r) => Rows::Owned(r.clone()),
            Self::Slice(p) => Rows::Provider(p.rows(table)),
            Self::Empty => Rows::Provider(&[]),
            Self::Merge(m) => Rows::Merge(table, m.clone()),
        }
    }
    pub(crate) fn recycle(self) {
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

pub(crate) enum Rows<'a> {
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
    pub(crate) fn len(&self) -> usize {
        match self {
            Self::Owned(r) => r.len(),
            Self::Provider(p) => p.len(),
            Self::Merge(_, m) => m.len,
        }
    }
    pub(crate) fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub(crate) fn iter(&self) -> impl Iterator<Item = &u32> {
        let (rows, positions): (&[u32], &[Position]) = match self {
            Self::Owned(r) => (r, &[]),
            Self::Provider(p) => (&[], p),
            Self::Merge(t, m) => (m.read(t), &[]),
        };
        rows.iter().chain(positions.iter().map(|p| &p.0))
    }
    pub(crate) fn extend_into(&self, out: &mut Vec<u32>) {
        match self {
            Self::Owned(r) => out.extend_from_slice(r),
            Self::Provider(p) => out.extend(p.iter().map(|p| p.0)),
            Self::Merge(t, m) => out.extend_from_slice(m.read(t)),
        }
    }
    pub(crate) fn to_vec(&self) -> Vec<u32> {
        let mut ids = Vec::with_capacity(self.len());
        self.extend_into(&mut ids);
        ids
    }
    // The oracle must not force a pending merge into the production cache.
    pub(crate) fn shadow(&self) -> Vec<u32> {
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

pub(crate) fn select(f: &File<'_>, fields: bool, codes: &[u16], extent: super::Extent) -> Slot {
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
        let ids = f.cut(bucket.rows(f.table), extent);
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
                if root_code == code && (root as usize) < f.limit(extent) {
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

        return Slot::Empty;
    };
    if more.is_empty() {

        return Slot::Slice(part);
    }
    more.push(part);

    Slot::Merge(Rc::new(Merge {
        parts: more,
        len,
        form,
        ordered: OnceCell::new(),
    }))
}

