//! The types of the `reading` concept: plain data, public fields and derives, with no constructor and no inherent impl.

/// Whether a file with no tree and no names still has the rows the engine writes of every file, which the rules that ask for it read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EngineRows {
    /// A rule asked for the file, its line count or its input metadata, so its rows are written and the rules run on them.
    Asked,
    /// No rule asked for them.
    NotAsked,
}

/// Which of the rows the engine writes of every file the rules ask for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct EngineAsked {
    /// The row of `filesystem.file`.
    pub file: EngineRows,
    /// The row of `filesystem.line-count`.
    pub lines: EngineRows,
    /// Known executable-permission rows supplied by the input edge.
    pub executable: EngineRows,
    /// Primary discovery or disk-read failure observations.
    pub read_failure: EngineRows,
    /// Any engine relation.
    pub any: EngineRows,
}

/// Why a primary source could not be admitted, independently of diagnostic policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PrimaryFailureKind {
    /// Input-edge discovery or a typed disk-read refusal.
    Disk,
    /// The source was unstable or exceeded a representable size; not a disk-read refusal.
    Other,
}

/// How many rules a run has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RunRuleCount(pub usize);

/// Whether a rule governing the file reads the primary disk-failure observation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReadFailureDemand {
    /// A rule that governs the file reads it, or the metadata of the rules is incomplete.
    Demanded,
    /// No rule that governs the file reads it.
    NotDemanded,
}

/// What the input edge said of a primary source it could not admit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FailureDetail<'a>(pub &'a str);

/// Whether the engine observed the file itself before its source could not be admitted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ObservedFile {
    /// Its metadata was observed, so the row of `filesystem.file` is decidable.
    Observed,
    /// It was not observed.
    Unobserved,
}
