//! The `ConsumersDone` value: what one worker's open file is done with before it made a node.

#[expect(
    unreachable_pub,
    reason = "the value-module template places `pub mod value`; its parent `consumers_done` is crate-private, so nothing outside the crate reaches it"
)]
pub mod value;

