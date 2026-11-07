//! Raw lowered key ordering: the value and its behaviour, owned together.
use crate::domain::allocation::types::ElementCapacity;
use crate::domain::raw_words::types::{
    Keep, OrderCapacities, OrderPlace, RawWord, RowRun, Stride,
};
use ryo_values::domain::rows::types::RowIndex;

/// Raw-word ordering deliberately differs from typed, cluster-preserving PartitionOrder.
#[derive(Clone, Debug)]
pub struct RawOrder {
    pub(crate) stride: Stride,
    pub(crate) keys: Vec<RawWord>,
    pub(crate) order: Vec<RowIndex>,
}

impl RawOrder {
    fn key_of(&self, row: RowIndex) -> Option<&[RawWord]> {
        self.keys
            .get(row.0 * self.stride.0..(row.0 + 1) * self.stride.0)
    }
    /// The places of the rows whose key is the probe.
    #[must_use]
    pub fn hits(&self, probe: &[RawWord]) -> RowRun {
        let probe = Some(probe);
        let from = self.order.partition_point(|row| self.key_of(*row) < probe);
        let to = from
            + self
                .order
                .get(from..)
                .unwrap_or_default()
                .partition_point(|row| self.key_of(*row) == probe);
        RowRun {
            start: OrderPlace(from),
            end: OrderPlace(to),
        }
    }
    /// Marks the first row of each key, in ordering places.
    pub fn firsts(&self, keep: &mut Vec<Keep>) {
        keep.clear();
        keep.resize(self.order.len(), Keep::No);
        let mut last: Option<RowIndex> = None;
        for row in &self.order {
            if last.is_none_or(|before| self.key_of(before) != self.key_of(*row)) {
                if let Some(mark) = keep.get_mut(row.0) {
                    *mark = Keep::Yes;
                }
                last = Some(*row);
            }
        }
    }
    /// The rows in key order, duplicates in their original order.
    #[must_use]
    pub fn rows(&self) -> &[RowIndex] {
        &self.order
    }
    /// The capacity of each buffer.
    #[must_use]
    pub fn capacities(&self) -> OrderCapacities {
        OrderCapacities {
            keys: ElementCapacity(self.keys.capacity()),
            order: ElementCapacity(self.order.capacity()),
        }
    }
}
