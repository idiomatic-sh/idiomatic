//! The `WalkScratch` value: the work of the graph walks one worker runs.

#[expect(
    unreachable_pub,
    reason = "the value-module template places `pub mod value`; its parent `walk_scratch` is crate-private, so nothing outside the crate reaches it"
)]
pub mod value;

