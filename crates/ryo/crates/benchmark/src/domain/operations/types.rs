//! The operations a linter bench runs, the bench directory they run on and where each writes.
use std::path::PathBuf;

/// One gated operation of a linter bench.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BenchOperation {
    /// Every default verified against its cases, then the language's projects linted by both contestants, compared and
    /// measured, and the rules grown; the document publishes the deterministic measures alone (`bench`), the ones a
    /// pull request is judged by.
    Bench,
    /// The same run as `Bench`; the document publishes its timing measures alone (`bench-timing`), which a shared runner
    /// moves from run to run, so no pull request is judged by them.
    Timing,
    /// Profile the registered cold workloads.
    Profile,
}

/// The collector verb that names an operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OperationVerb<'a>(pub &'a str);

/// A linter bench directory: `languages/<lang>/benchmarks/linters/<linter>`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BenchDirectory(pub PathBuf);

/// Where one operation of one bench writes: `<target>/benchmark/<lang>/<linter>/<verb>`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperationLayout {
    /// The operation's own output directory, made fresh for each run.
    pub root: PathBuf,
}
