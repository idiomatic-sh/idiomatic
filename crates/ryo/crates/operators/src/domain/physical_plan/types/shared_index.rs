//! The `SharedIndex` value: a probe index the plan shares, as one worker holds it for the open file.

#[expect(
    unreachable_pub,
    reason = "the value-module template places `pub mod value`; its parent `shared_index` is crate-private, so nothing outside the crate reaches it"
)]
pub mod value;

#[cfg(test)]
mod value_test;
