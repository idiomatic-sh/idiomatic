//! The types of the `bindings` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo::languages;

/// What a parameter binds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Binding {
    /// The element that stands for the parameter: its name, or the parameter when it has none.
    pub id: languages::Position,
    /// The name; none for a parameter that binds no name.
    pub name: languages::Symbol,
}

/// A type written for something: the element annotated and the type as written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Annotated {
    /// What is annotated: a bound name, a pattern, a parameter or a function.
    pub owner: languages::Position,
    /// The type written for it.
    pub annotation: languages::Position,
}

/// Whether a kind of node is a pattern, or a part of one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Patternness {
    /// It is.
    Pattern,
    /// It is not.
    NotPattern,
}

/// Whether a name pattern is nested in the pattern of a `let`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LetPattern {
    /// It is: it binds one of the names a destructuring `let` binds.
    In,
    /// It is not.
    Outside,
}

/// Whether a name starts as a constant, a unit struct or a variant is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Constness {
    /// It does.
    Constant,
    /// It does not.
    NotConstant,
}

/// Whether a name is `None`, the one name of the prelude that a pattern writes with no path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Preludeness {
    /// It is.
    Prelude,
    /// It is not.
    NotPrelude,
}

/// Whether an element is a name in a destructuring `let` that may be a constant and so bind nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PossibleConstant {
    /// It may be one.
    Possible,
    /// It cannot be one.
    Impossible,
}

/// How many arguments a parameter takes: one, or, for the C-variadic `...` of a foreign function, the ones left over.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Spread {
    /// One argument.
    One,
    /// The arguments left over.
    Rest,
}
