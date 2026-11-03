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
    /// Any engine relation.
    pub any: EngineRows,
}
