//! Plain data of the execution port.

/// How a port implementation observes the numeric exit of a completed child.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExitObservationMode {
    /// Numeric exits are directly observed with no synthetic timeout status.
    Direct,
    /// Wrapped, bounded and unknown observers cannot certify process failures.
    Mediated,
}

/// The exit of a completed child, decoded from the operating system status.
pub(crate) mod process_exit;

/// The compile times of the rules a request names, by rule; none when its engine compiles inside each run.
pub type CompileTimes =
    Option<std::collections::BTreeMap<ryo::linter::RuleId, std::time::Duration>>;

/// How the edge runs a worker's stored request in this process: the request at a path, its response written to a stream.
pub type RequestRun = Box<
    dyn Fn(
            &std::path::Path,
            Box<dyn std::io::Write + Send>,
        ) -> Result<(), ryo_benchmark_contracts::domain::reports::errors::ReportsError>
        + Send
        + Sync,
>;

/// How the edge compiles the rules of a worker's stored request ahead of its first run.
pub type RequestCompile = Box<
    dyn Fn(
            &std::path::Path,
        )
            -> Result<CompileTimes, ryo_benchmark_contracts::domain::reports::errors::ReportsError>
        + Send
        + Sync,
>;

/// The worker the edge composes over its engine, run inside this process in place of a child.
pub struct InProcessWorker {
    /// Runs one stored request and writes its response.
    pub run: RequestRun,
    /// Compiles the rules of one stored request.
    pub compile: RequestCompile,
}
impl std::fmt::Debug for InProcessWorker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}
