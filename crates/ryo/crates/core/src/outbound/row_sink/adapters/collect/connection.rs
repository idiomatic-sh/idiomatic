//! The place the collect adapter keeps every row of a run, shared with the edge that asked for them.

use crate::outbound::row_sink::types::Collected;

/// Where the collect adapter keeps every row, shared with the edge that takes them after the run: the edge makes it, hands a clone
/// to the factory, and takes what it holds once the run ends.
#[derive(Clone, Debug, Default)]
pub struct CollectorConnection(pub std::sync::Arc<std::sync::Mutex<Collected>>);
