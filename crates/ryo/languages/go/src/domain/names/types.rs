//! The types of the `names` concept: plain data, public fields and derives, with no constructor and no inherent impl.

/// Why a name cannot be resolved statically: each has the number it has in `UNKNOWN_REASONS`, within the language's range, 32 to 47.
#[derive(Clone, Copy, Debug, PartialEq, Eq, strum::EnumIter, strum::IntoStaticStr)]
#[strum(serialize_all = "kebab-case")]
#[repr(u8)]
pub(crate) enum Unresolved {
    /// A callee that is no dotted name: `handlers[i]()`, `f()()`, `[]byte(s)`; one index may be of a slice or of a map, so a call of `f[T]()` is as dynamic.
    DynamicCallee = 32,
    /// The name an import without a name of its own binds: the package it imports declares it, and the file does not say.
    DeclaredByPackage = 33,
    /// `import . "p"`: it binds every exported name of the package, and the file does not list them.
    DotImport = 34,
    /// A string literal whose value is no text: an escape makes a byte that is no UTF-8.
    NotText = 35,
    /// A parameter or a receiver the declaration does not name.
    Unnamed = 36,
    /// The key of a composite literal: a field or a variable, which the file does not say apart.
    AmbiguousKey = 37,
}

/// Whether an expression writes a dotted name: a name, or a selection from one, instantiated with several types or not.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Dottedness {
    /// It does.
    Dotted,
    /// It is any other expression.
    Other,
}

/// What a call calls, as far as the file says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Callee {
    /// A dotted name, or the type arguments of a generic one.
    Named,
    /// Any other expression that is called.
    Dynamic,
    /// A type written out, which a call converts to.
    Conversion,
}

/// Whether an identifier is the blank identifier `_`, which declares nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Blankness {
    /// It is `_`.
    Blank,
    /// It is any other name.
    Named,
}

/// Whether a comment starts its line, with nothing but blank space before it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Placement {
    /// It does.
    LineStart,
    /// Code is written before it on its line.
    AfterCode,
}

/// Whether an element starts on the line after the one the element before it ends on, with only blank space between them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Adjacency {
    /// It does.
    NextLine,
    /// Code, a blank line, or nothing but the same line is between them.
    Apart,
}
