//! Raw lowered key ordering; duplicate rows retain their original order.
use super::types::raw_order::value::RawOrder;
use crate::domain::raw_words::types::{KeyStride, RawKeys, RawWords};
use ryo_values::domain::rows::types::RowIndex;

/// Orders the rows of a flat key list by key, reusing the buffers of a recycled ordering.
pub(crate) fn build(
    keys: RawWords<'_>,
    stride: KeyStride,
    recycled: Option<RawOrder>,
) -> RawOrder {
    let keys = keys.0;
    let mut out = recycled.unwrap_or_else(|| RawOrder {
        stride,
        keys: RawKeys(Vec::new()),
        order: Vec::new(),
    });
    out.stride = stride;
    out.keys.0.clear();
    out.keys.0.extend_from_slice(keys);
    out.order.clear();
    out.order
        .extend((0..keys.len().checked_div(stride.0).unwrap_or(0)).map(RowIndex));
    let key_of = |row: RowIndex| keys.get(row.0 * stride.0..(row.0 + 1) * stride.0);
    out.order
        .sort_unstable_by(|left, right| key_of(*left).cmp(&key_of(*right)).then(left.cmp(right)));
    out
}
