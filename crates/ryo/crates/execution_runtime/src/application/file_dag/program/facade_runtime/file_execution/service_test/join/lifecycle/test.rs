//! Reentrant delivery and retry use one genuine prepared program and file owner.
use super::fixtures::exercise_nested;

#[test]
fn same_owner_nested_join_preserves_shared_index_bag_order_and_delivery_retry() {
    exercise_nested(false).expect("one compiled owner, nested Join and delivery retry");
}
#[test]
fn nested_output_keeps_consumer_memo_scratch_prefix_and_resume() {
    exercise_nested(true).expect("actual nested output scope");
}
