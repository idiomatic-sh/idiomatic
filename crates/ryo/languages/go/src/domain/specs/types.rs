//! The types of the `specs` concept: plain data, public fields and derives, with no constructor and no inherent impl.

/// What a `GenDecl` declares, by the keyword it starts with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Declares {
    /// `const`.
    Constants,
    /// `var`.
    Variables,
    /// `type`.
    Types,
    /// `import`.
    Imports,
}

/// What a type declaration declares, by the type it gives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Underlying {
    /// A struct type.
    Structure,
    /// An interface type.
    Interface,
    /// Another name for a type: `type A = B`.
    Alias,
    /// Any other type.
    Other,
}
