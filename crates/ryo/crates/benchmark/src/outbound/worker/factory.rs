//! Constructs the worker transport from process resources opened by the composition.
use super::adapters::json::adapter::WorkerJsonAdapter;
use super::types::WorkerSource;
use std::io::Write;
/// Bind the request source and the response stream to the JSON transport.
#[must_use]
pub fn json(
    source: WorkerSource,
    output: Box<dyn Write + Send>,
) -> impl super::interface::WorkerPort {
    WorkerJsonAdapter::new(source, output)
}
