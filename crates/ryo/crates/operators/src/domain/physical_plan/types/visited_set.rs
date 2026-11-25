//! The `VisitedSet` value: the entities one graph walk has met, emptied for the next walk at the cost of what it met.

#[expect(
    unreachable_pub,
    reason = "the value-module template places `pub mod value`; its parent `visited_set` is crate-private, so nothing outside the crate reaches it"
)]
pub mod value;

