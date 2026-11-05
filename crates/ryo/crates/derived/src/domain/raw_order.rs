//! Raw lowered key ordering; duplicate rows retain their original order.
use super::contract::types::RawOrder;

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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn raw_words_order_duplicates_and_firsts_without_cluster_passthrough() {
        let order = build(&[9, 4, 1, 7, 9, 4, 0, 8], 2, None);
        assert_eq!(order.rows(), &[3, 1, 0, 2]);
        assert_eq!(hits(&order, &[9, 4]), 2..4);
        assert_eq!(hits(&order, &[4, 9]), 2..2);
        let mut keep = Vec::new();
        firsts(&order, &mut keep);
        assert_eq!(keep, [true, true, false, true]);
    }
    #[test]
    fn recycling_changes_keys_and_zero_stride_stays_empty() {
        let old = build(&[u64::MAX, 0, u64::MAX], 1, None);
        assert_eq!(old.rows(), &[1, 0, 2]);
        let capacities = old.capacities();
        let new = build(&[7, 2], 1, Some(old));
        assert_eq!(new.rows(), &[1, 0]);
        assert_eq!(hits(&new, &[u64::MAX]), 2..2);
        assert_eq!(new.capacities(), capacities);
        let empty = build(&[1, 2], 0, Some(new));
        assert!(empty.rows().is_empty());
        assert_eq!(hits(&empty, &[]), 0..0);
    }
}
