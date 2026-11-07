//! The types of the `scopes` concept: plain data, public fields and derives, with no constructor and no inherent impl.

/// Whether a kind opens a scope of its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Scopeness {
    /// A function, a class, a lambda or a comprehension.
    Scope,
    /// Anything else.
    Inner,
}

/// What a name expression does with its name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Context {
    /// It reads it.
    Load,
    /// It binds it.
    Store,
    /// It deletes it.
    Del,
}

/// The place of a node among the nodes of one part of the grammar under the same element, from zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct CategoryIndex(pub usize);

/// Whether a child of a comprehension clause is evaluated in the scope around the comprehension: the first clause's iterable is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Evaluation {
    /// In the scope around the comprehension.
    Around,
    /// In the comprehension's own scope.
    Within,
}

/// Whether a name expression is the target of an assignment expression, `(n := ...)`, which binds outside any comprehension.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Targeting {
    /// The target of an assignment expression.
    AssignmentExpression,
    /// Any other name expression.
    Other,
}
