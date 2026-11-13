//! Plain data this transport moves between its documents and its adapter.
use std::ffi::OsStr;

/// Complete serialized response before transport framing is appended.
#[derive(Clone, Debug)]
pub(crate) struct WorkerResponseBytes(pub Vec<u8>);

/// The command-line request payload exactly as the parent supplied it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct WorkerPayload<'a>(pub &'a OsStr);

/// One nested document of a request, handed to the document that owns its shape.
#[derive(Clone, Debug)]
pub(crate) struct DocumentWire<T>(pub T);
