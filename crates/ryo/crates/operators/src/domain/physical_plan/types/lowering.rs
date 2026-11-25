//! The `Lowering` value: what one node of the physical plan runs.

#[expect(
    unreachable_pub,
    reason = "the value-module template places `pub mod value`; its parent `lowering` is crate-private, so nothing outside the crate reaches it"
)]
pub mod value;

