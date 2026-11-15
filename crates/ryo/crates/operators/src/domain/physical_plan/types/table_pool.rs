//! The `TablePool` value: the emptied node tables one worker keeps for its next nodes.

#[expect(
    unreachable_pub,
    reason = "the value-module template places `pub mod value`; its parent `table_pool` is crate-private, so nothing outside the crate reaches it"
)]
pub mod value;

#[cfg(test)]
mod value_test;
