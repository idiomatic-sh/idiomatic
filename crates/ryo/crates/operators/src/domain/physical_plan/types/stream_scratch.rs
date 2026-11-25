//! The `StreamScratch` value: the work of the chains of streamed stages one worker runs.

#[expect(
    unreachable_pub,
    reason = "the value-module template places `pub mod value`; its parent `stream_scratch` is crate-private, so nothing outside the crate reaches it"
)]
pub mod value;

