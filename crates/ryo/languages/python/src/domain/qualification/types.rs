//! Lexical source provenance, distinct from the runtime value of an expression.

/// A language version, compared by components rather than lexical text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Version {
    /// Major language version.
    pub(crate) major: ryo::languages::Integer,
    /// Minor language version.
    pub(crate) minor: ryo::languages::Integer,
}
/// One name in the standard builtin inventory and its first Python version.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Builtin {
    /// The canonical name in Python's builtin namespace.
    pub name: ryo::languages::NameText<'static>,
    /// The first language version that supplies it.
    pub(crate) since: Version,
}
