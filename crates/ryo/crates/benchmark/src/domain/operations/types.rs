//! The operations a linter bench runs, the bench directory they run on and where each writes.
use std::path::PathBuf;

/// One gated operation of a linter bench.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BenchOperation {
    /// Every default verified against its registered cases (`verify-inventory`).
    Correctness,
    /// Both contestants certified, then measured on the language's projects (`measure-valid`).
    Measure,
    /// Both contestants compared rule by rule on the language's projects (`parity`).
    Parity,
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
