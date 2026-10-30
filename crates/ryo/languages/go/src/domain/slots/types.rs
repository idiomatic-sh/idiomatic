//! The types of the `slots` concept: plain data, public fields and derives, with no constructor and no inherent impl.

/// What an identifier is where it stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Slot {
    /// It names something the file declares there.
    Declares,
    /// It uses a name.
    Uses,
    /// It is the name after a dot, or a label, or the name of the package: no use of a name of the file's.
    Elsewhere,
    /// It is a key of a composite literal, which names a field or a variable: the language cannot tell which.
    Ambiguous,
}

/// What an identifier or a selection is as a reference.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Referencing {
    /// It uses a name.
    Used,
    /// It may use a name: the key of a composite literal, a field or a variable.
    Unclear,
    /// It does not.
    Other,
}
