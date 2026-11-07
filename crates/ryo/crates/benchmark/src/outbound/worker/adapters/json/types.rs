//! Encoded response bytes belong to this transport, not the canonical report.
#[derive(Clone, Debug)]
/// Complete serialized response before transport framing is appended.
pub(crate) struct WorkerResponseBytes(pub Vec<u8>);
