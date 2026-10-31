//! Encoded response bytes belong to this transport, not the canonical report.
#[derive(Clone, Debug)]
pub(crate) struct WorkerResponseBytes(pub Vec<u8>);
