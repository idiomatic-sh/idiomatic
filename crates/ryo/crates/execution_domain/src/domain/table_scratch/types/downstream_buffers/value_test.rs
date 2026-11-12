//! Lending the downstream buffers keeps their room for the next lend.

use super::value::DownstreamBuffers;

#[test]
fn lending_keeps_the_buffers_capacity() {
    let mut buffers = DownstreamBuffers::default();
    buffers.lend(|_, memo| memo.reserve(8));
    let held = buffers.lend(|_, memo| memo.capacity());
    assert!(held >= 8, "the second lend holds {held} memo slots");
}
