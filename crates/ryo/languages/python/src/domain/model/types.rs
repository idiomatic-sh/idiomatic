//! The types of the `model` concept: plain data, public fields and derives, with no constructor and no inherent impl. What the
//! language works out of a file once, before its readers run.

use ryo::languages::{Position, Symbol};

/// Where a scope says a name it assigns is bound instead of in itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BindingRedirect {
    /// `global`: in the module.
    Global,
    /// `nonlocal`: in the nearest enclosing function.
    Nonlocal,
}

/// One name a scope declares `global` or `nonlocal`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ScopeDeclaration {
    /// The scope the declaration is in.
    pub scope: Position,
    /// The name, as the file's names hold it.
    pub name: Symbol,
    /// Where the scope binds it.
    pub redirect: BindingRedirect,
}

/// What the language works out of a file once: the `global` and `nonlocal` names of every scope, in the order of the file, so no
/// reader searches a function again for them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Declarations {
    /// Every declared name.
    pub declared: Vec<ScopeDeclaration>,
}
