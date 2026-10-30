//! The types of the `declarations` concept: plain data, public fields and derives, with no
//! constructor and no inherent impl. What Rust declares, in its own words.

use crate::domain::refinements::types::Declared;

use ryo::languages;

/// A declaration of a node: what it declares, the name, and where the name is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Declaration {
    /// What it declares.
    pub declared: Declared,
    /// The name.
    pub name: languages::Symbol,
    /// The element the name is: a declaration is known by it when the name is written apart from the declaring node.
    pub named: languages::Position,
    /// Where the name starts.
    pub from: languages::Offset,
    /// Where the name ends.
    pub to: languages::Offset,
}

/// A trait a trait requires, as the bound writes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Supertrait {
    /// The type that writes the bound.
    pub contract: languages::Position,
    /// The name of the trait it requires.
    pub target: languages::Symbol,
}

/// A trait an `impl` implements, and what inherits it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Implementation {
    /// The type that writes the trait.
    pub contract: languages::Position,
    /// The name of the trait.
    pub target: languages::Symbol,
    /// What inherits it: the declaration of the type, when the file declares it once by a single name, otherwise the `impl` itself.
    pub declaration: languages::Position,
}
