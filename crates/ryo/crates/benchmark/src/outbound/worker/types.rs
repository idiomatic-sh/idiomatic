//! Where a worker's request comes from, as the composition chose.
use std::{ffi::OsString, path::PathBuf};

/// Where the request payload comes from: the command line itself, or a stored request the parent owns.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorkerSource {
    /// The payload text the parent put on the command line.
    Argument(OsString),
    /// A stored request, read when the request is received.
    Stored(PathBuf),
}
