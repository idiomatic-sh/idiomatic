//! Constructs the worker transport from process resources opened by the composition.
use super::adapters::json::adapter::WorkerJsonAdapter;
use std::{ffi::OsString, io::Write};
/// Bind one request argument and the response stream to the JSON transport.
#[must_use]
pub fn json(payload: OsString, output: Box<dyn Write + Send>) -> impl super::interface::WorkerPort {
    WorkerJsonAdapter::new(payload, output)
}
