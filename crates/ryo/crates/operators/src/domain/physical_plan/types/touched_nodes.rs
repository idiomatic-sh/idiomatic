//! The `TouchedNodes` value: the nodes one worker's open file touched, each once.

#[expect(
    unreachable_pub,
    reason = "the value-module template places `pub mod value`; its parent `touched_nodes` is crate-private, so nothing outside the crate reaches it"
)]
pub mod value;

#[cfg(test)]
mod value_test;
