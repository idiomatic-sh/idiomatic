//! Lexical source provenance, distinct from the runtime value of an expression.
use ryo::languages::{Position, Symbol};
/// The source that supplies a dotted expression's root.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Origin {
    /// A specific import alias declares the root.
    Import(Position),
    /// The language builtin namespace supplies the root.
    Builtin,
    /// A local declaration supplies a value whose qualified identity is dynamic.
    Local,
    /// No known declaration or builtin supplies the root.
    Unresolved,
    /// A wildcard import or deletion prevents a proven origin.
    Ambiguous,
}
/// Provenance and its canonical path, or a registered unknown path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Qualified {
    /// Canonical imported/builtin path.
    pub path: Symbol,
    /// The declaration category that established the path.
    pub provenance: Symbol,
    /// First builtin language version; zero components mean not applicable for other origins.
    pub since: Version,
}
/// Whether surrounding declarations execute after the expression's scope was defined.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Timing {
    /// Source order governs the lookup.
    Immediate,
    /// A surrounding scope may finish before this expression executes.
    Deferred,
}
/// A language version, compared by components rather than lexical text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Version {
    /// Major language version.
    pub major: ryo::languages::Integer,
    /// Minor language version.
    pub minor: ryo::languages::Integer,
}
/// One name in the standard builtin inventory and its first Python version.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Builtin {
    /// The canonical name in Python's builtin namespace.
    pub name: ryo::languages::NameText<'static>,
    /// The first language version that supplies it.
    pub since: Version,
}
/// Whether lexical namespace events preserve the selected origin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Certainty {
    /// No intervening event invalidates this origin.
    Known,
    /// A wildcard or deletion makes the origin unproven.
    Uncertain,
}
