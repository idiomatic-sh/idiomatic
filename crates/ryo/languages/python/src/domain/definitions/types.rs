//! The types of the `definitions` concept: plain data, public fields and derives, with no constructor and no inherent impl.

/// What a definition or a parameter is in Python's own words, the `refinement` of its row: `async-method`, `keyword-only`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Refinement(pub &'static str);

/// Where a parameter is in its callable's list, as the `parameter` relation says it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ParameterPlace {
    /// How it takes its argument.
    pub kind: ryo::languages::ParameterKind,
    /// The same in Python's own words.
    pub(crate) word: Refinement,
    /// Its place among the parameters, from zero, the markers `/` and `*` not counted.
    pub ordinal: ryo::languages::Integer,
    /// The function or lambda whose parameter it is.
    pub(crate) callable: ryo::languages::Position,
}
