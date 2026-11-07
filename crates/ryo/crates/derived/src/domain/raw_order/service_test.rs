//! The `raw_order` service's cases.

use super::service::*;

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
