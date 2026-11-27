//! The types of the `lexical_facts` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::qualification::types::Version;
use ryo::languages::Symbol;

/// The text of a dotted or qualified name being written.
#[derive(Clone, Debug, Default)]
pub(crate) struct QualifiedPath(pub String);

/// Where a name or attribute expression that is read comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NameProvenance {
    /// An import: the path is the imported name and the attributes read from it.
    Import,
    /// A builtin of the language, there since the version: the path starts at the builtins module.
    Builtin(Version),
    /// A definition of the file that is not an import: no path.
    Local,
    /// Nothing the file defines and no builtin: no path.
    Unresolved,
    /// A wildcard import or a deletion leaves the origin unproven: no path.
    Ambiguous,
}

/// The five words that say where a name comes from, as the file's words have them.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ProvenanceWords {
    /// An import.
    pub(crate) import: Symbol,
    /// A builtin.
    pub(crate) builtin: Symbol,
    /// A definition of the file that is not an import.
    pub(crate) local: Symbol,
    /// Nothing the file defines and no builtin.
    pub(crate) unresolved: Symbol,
    /// An origin left unproven.
    pub(crate) ambiguous: Symbol,
}
