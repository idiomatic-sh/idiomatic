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
    pub word: Refinement,
    /// Its place among the parameters, from zero, the markers `/` and `*` not counted.
    pub ordinal: ryo::languages::Integer,
    /// The function or lambda whose parameter it is.
    pub callable: ryo::languages::Position,
}

/// What a definition is: its shared kind and its word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Defined {
    /// The shared kind.
    pub kind: ryo::languages::DeclarationKind,
    /// The same in Python's own words.
    pub word: Refinement,
}

/// A name a file defines, as the `defines` relation says it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Definition {
    /// What it defines: the shared kind and Python's word for it.
    pub defined: Defined,
    /// The name.
    pub name: ryo::languages::Symbol,
    /// The scope that binds it.
    pub owner: ryo::languages::Position,
    /// The element whose span is the name's.
    pub named: ryo::languages::Position,
}
