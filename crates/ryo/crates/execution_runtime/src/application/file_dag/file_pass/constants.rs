//! The texts the file pass answers an incomplete named output with, and the target its node counters are emitted to.

pub(crate) const SOURCE_FACTS_INCOMPLETE: &str = "physical source facts are incomplete";
pub(crate) const PRODUCER_COVERAGE_INCOMPLETE: &str =
    "producer publication has incomplete physical coverage";
/// Why a bound observation in a file is withheld when a producer bound to it has no rows in the file.
pub(crate) const BOUND_PRODUCER_UNAVAILABLE: &str = "declared bound producer is unavailable";
/// The tracing target of the node counters: a subscriber that enables it at info switches the counting on for a run, and is
/// sent one event per node made and one for the run when the run's program is dropped.
pub(crate) const NODE_COUNTERS_TARGET: &str = "ryo::node_counters";
/// The name of the event of one node's counts.
pub(crate) const NODE_COUNTERS_NODE: &str = "node";
/// The name of the event of the run's counts, every node's summed.
pub(crate) const NODE_COUNTERS_RUN: &str = "run";
