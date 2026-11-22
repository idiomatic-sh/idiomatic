//! The same shared first conditions with library kernels over a batch of files held as Arrow arrays. No `arrow-ord` in the local
//! registry: equality is `BooleanBuffer::collect_bool` (the loop `arrow-ord`'s `eq` runs for primitives), selection is
//! `arrow_select::filter::filter` and `arrow_select::take::take`.

use super::{Anchor, Columns, Key, Lit, counting_sort};
use arrow_array::{Array, BooleanArray, StringArray, UInt16Array, UInt32Array};
use arrow_buffer::BooleanBuffer;
use std::hint::black_box;

#[derive(Default)]
pub(super) struct State {
    kind: Vec<u16>,
    field: Vec<u16>,
    text: Vec<u8>,
    text_offsets: Vec<i32>,
    kind_offsets: Vec<u32>,
    by_kind: Vec<u32>,
    field_offsets: Vec<u32>,
    by_field: Vec<u32>,
}

/// Counters of one batch: entries visited, survivors, kernel calls.
#[derive(Default, Clone, Copy)]
pub(super) struct Counts {
    pub(super) visited: u64,
    pub(super) survivors: u64,
    pub(super) calls: u64,
}

impl State {
    pub(super) fn add_file(&mut self, cols: &Columns, source: &[u8]) {
        if self.text_offsets.is_empty() {
            self.text_offsets.push(0);
        }
        self.kind.extend_from_slice(&cols.kind);
        self.field.extend_from_slice(&cols.field);
        for i in 0..cols.kind.len() {
            let (a, b) = (cols.start[i] as usize, cols.end[i] as usize);
            // A long text cannot equal a literal; only texts of up to 32 bytes are kept (regex tests see the same).
            if b - a <= 32 {
                self.text.extend_from_slice(&source[a..b]);
            }
            self.text_offsets.push(i32::try_from(self.text.len()).expect("i32"));
        }
    }

    pub(super) fn clear(&mut self) {
        self.kind.clear();
        self.field.clear();
        self.text.clear();
        self.text_offsets.clear();
    }

    /// The batch's columns as Arrow arrays (built once a batch).
    pub(super) fn arrays(&self) -> (UInt16Array, UInt16Array, StringArray) {
        let kinds = UInt16Array::from(self.kind.clone());
        let fields = UInt16Array::from(self.field.clone());
        let texts = StringArray::new(
            arrow_buffer::OffsetBuffer::new(arrow_buffer::ScalarBuffer::from(self.text_offsets.clone())),
            arrow_buffer::Buffer::from(self.text.clone()),
            None,
        );
        (kinds, fields, texts)
    }

    /// Scan form: per condition, an equality kernel over the whole batch column per key, `or`, the literal's kernel and
    /// `and`, then `filter` of the positions.
    pub(super) fn scan(&self, conditions: &[Anchor], arrays: &(UInt16Array, UInt16Array, StringArray)) -> Counts {
        let (kinds, fields, texts) = arrays;
        let positions = UInt32Array::from_iter_values(0..kinds.len() as u32);
        let mut counts = Counts::default();
        for c in conditions {
            let mut mask: Option<BooleanBuffer> = None;
            for key in &c.keys {
                let (column, code) = match key {
                    Key::Kind(k) => (kinds.values(), *k),
                    Key::Field(f) => (fields.values(), *f),
                };
                let m = BooleanBuffer::collect_bool(column.len(), |i| column[i] == code);
                counts.calls += 1;
                counts.visited += column.len() as u64;
                mask = Some(match mask {
                    None => m,
                    Some(prev) => &prev | &m,
                });
            }
            let Some(mut mask) = mask else { continue };
            if let Some(lm) = literal_mask(&c.lit, texts, None) {
                counts.calls += 1;
                mask = &mask & &lm;
            }
            let kept = arrow_select::filter::filter(&positions, &BooleanArray::new(mask, None)).expect("filter");
            counts.calls += 1;
            counts.survivors += kept.len() as u64;
            black_box(kept);
        }
        counts
    }

    /// Bucket form: counting sort of the batch by kind and by field once; per condition, its buckets' positions, then (with
    /// a literal) `take` of the texts, the literal's kernel and `filter` of the positions.
    pub(super) fn bucketed(&mut self, conditions: &[Anchor], arrays: &(UInt16Array, UInt16Array, StringArray), kinds_width: usize, fields_width: usize) -> Counts {
        counting_sort(&self.kind, kinds_width, &mut self.kind_offsets, &mut self.by_kind);
        counting_sort(&self.field, fields_width, &mut self.field_offsets, &mut self.by_field);
        let texts = &arrays.2;
        let mut counts = Counts::default();
        let mut ids: Vec<u32> = Vec::new();
        for c in conditions {
            ids.clear();
            for key in &c.keys {
                let (offsets, sorted, code) = match key {
                    Key::Kind(k) => (&self.kind_offsets, &self.by_kind, usize::from(*k)),
                    Key::Field(f) => (&self.field_offsets, &self.by_field, usize::from(*f)),
                };
                ids.extend_from_slice(&sorted[offsets[code] as usize..offsets[code + 1] as usize]);
            }
            counts.visited += ids.len() as u64;
            if matches!(c.lit, Lit::None) {
                counts.survivors += ids.len() as u64;
                black_box(&ids);
                continue;
            }
            let indices = UInt32Array::from(ids.clone());
            let taken = arrow_select::take::take(texts, &indices, None).expect("take");
            counts.calls += 1;
            let taken = taken.as_any().downcast_ref::<StringArray>().expect("strings");
            if let Some(lm) = literal_mask(&c.lit, taken, None) {
                counts.calls += 1;
                let kept = arrow_select::filter::filter(&indices, &BooleanArray::new(lm, None)).expect("filter");
                counts.calls += 1;
                counts.survivors += kept.len() as u64;
                black_box(kept);
            }
        }
        counts
    }
}

fn literal_mask(lit: &Lit, texts: &StringArray, _: Option<()>) -> Option<BooleanBuffer> {
    match lit {
        Lit::None => None,
        Lit::Eq(set) => {
            let mut out: Option<BooleanBuffer> = None;
            for s in set {
                let m = BooleanBuffer::collect_bool(texts.len(), |i| texts.value(i).as_bytes() == s.as_slice());
                out = Some(match out {
                    None => m,
                    Some(prev) => &prev | &m,
                });
            }
            out
        }
        Lit::Re(re) => Some(BooleanBuffer::collect_bool(texts.len(), |i| re.is_match(texts.value(i).as_bytes()))),
    }
}
