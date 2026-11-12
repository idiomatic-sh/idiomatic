//! Lending the downstream buffers keeps their room for the next lend.

use super::value::DownstreamBuffers;

#[test]
fn lending_keeps_the_buffers_capacity() {
    let mut buffers = DownstreamBuffers::default();
    buffers.lend().1.reserve(8);
    let (_, memo) = buffers.lend();
    assert!(
        memo.capacity() >= 8,
        "the second lend holds {} memo slots",
        memo.capacity()
    );
}
