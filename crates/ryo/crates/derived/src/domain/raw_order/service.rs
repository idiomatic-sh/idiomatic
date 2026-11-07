//! Raw lowered key ordering; duplicate rows retain their original order.
use crate::domain::contract::types::RawOrder;

pub(crate) fn build(keys: &[u64], stride: usize, recycled: Option<RawOrder>) -> RawOrder {
    let mut out = recycled.unwrap_or_else(|| RawOrder {
        stride,
        keys: Vec::new(),
        order: Vec::new(),
    });
    out.stride = stride;
    out.keys.clear();
    out.keys.extend_from_slice(keys);
    out.order.clear();
    out.order
        .extend(0..keys.len().checked_div(stride).unwrap_or(0));
    let key_of = |row: usize| keys.get(row * stride..(row + 1) * stride);
    out.order
        .sort_unstable_by(|left, right| key_of(*left).cmp(&key_of(*right)).then(left.cmp(right)));
    out
}
pub(crate) fn hits(order: &RawOrder, probe: &[u64]) -> std::ops::Range<usize> {
    let key_of = |row: usize| order.keys.get(row * order.stride..(row + 1) * order.stride);
    let probe = Some(probe);
    let from = order.order.partition_point(|row| key_of(*row) < probe);
    let to = from
        + order
            .order
            .get(from..)
            .unwrap_or_default()
            .partition_point(|row| key_of(*row) == probe);
    from..to
}
pub(crate) fn firsts(order: &RawOrder, keep: &mut Vec<bool>) {
    keep.clear();
    keep.resize(order.order.len(), false);
    let key_of = |row: usize| order.keys.get(row * order.stride..(row + 1) * order.stride);
    let mut last = None;
    for row in &order.order {
        if last.is_none_or(|before| key_of(before) != key_of(*row)) {
            if let Some(mark) = keep.get_mut(*row) {
                *mark = true;
            }
            last = Some(*row);
        }
    }
}
