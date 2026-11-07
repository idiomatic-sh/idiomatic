//! Raw lowered key ordering: the value and its behaviour, owned together.
use crate::domain::allocation::types::ElementCapacity;
use crate::domain::raw_words::types::{
    KeepFlags, KeyStride, OrderCapacities, OrderPlace, RawKeys, RawWords, RowRun,
};
use ryo_values::domain::rows::types::RowIndex;

/// Raw-word ordering deliberately differs from typed, cluster-preserving PartitionOrder.
#[derive(Clone, Debug)]
pub struct RawOrder {
    pub(crate) stride: KeyStride,
    pub(crate) keys: RawKeys,
    pub(crate) order: Vec<RowIndex>,
}

impl RawOrder {
    /// The places of the rows whose key is the probe.
    #[must_use]
    pub fn hits(&self, probe: RawWords<'_>) -> RowRun {
        let key_of = |row: RowIndex| {
            self.keys
                .0
                .get(row.0 * self.stride.0..(row.0 + 1) * self.stride.0)
        };
        let probe = Some(probe.0);
        let from = self.order.partition_point(|row| key_of(*row) < probe);
        let to = from
            + self
                .order
                .get(from..)
                .unwrap_or_default()
                .partition_point(|row| key_of(*row) == probe);
        RowRun {
            start: OrderPlace(from),
            end: OrderPlace(to),
        }
    }
    /// Marks the first row of each key, in ordering places.
    pub fn firsts(&self, keep: &mut KeepFlags) {
        let key_of = |row: RowIndex| {
            self.keys
                .0
                .get(row.0 * self.stride.0..(row.0 + 1) * self.stride.0)
        };
        keep.0.clear();
        keep.0.resize(self.order.len(), false);
        let mut last: Option<RowIndex> = None;
        for row in &self.order {
            if last.is_none_or(|before| key_of(before) != key_of(*row)) {
                if let Some(mark) = keep.0.get_mut(row.0) {
                    *mark = true;
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
            keys: ElementCapacity(self.keys.0.capacity()),
            order: ElementCapacity(self.order.capacity()),
        }
    }
}
