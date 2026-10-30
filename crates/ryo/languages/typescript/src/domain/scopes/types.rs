//! Whether a syntax node introduces a lexical environment.
/// Whether a node opens a lexical environment of its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Scope {
    /// The node opens one: its names are not seen outside it.
    Opens,
    /// The node does not: its names belong to the scope around it.
    Continues,
}

/// Whether a function of the tree is declared: a function declaration, or the body of a method, which its key names; a function
/// expression or an arrow written as a value declares nothing, and is no row of `function` (its parameters are still `parameter` rows).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Declaredness {
    /// A function declaration.
    Function,
    /// The body of a method, a getter or a setter, with its key when the key is a name and not computed.
    Method(Option<ryo::languages::Position>),
    /// A function expression or an arrow written as a value.
    Expression,
}
