//! The types of the `bindings` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo::languages::{DeclarationKind, Offset, Position};

/// One name a declaration of the file binds, by where its binding identifier starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BoundName {
    /// Where the binding identifier starts.
    pub start: Offset,
    /// The declaration that binds the name: the id of its `defines` row.
    pub at: Position,
    /// Where the name it declares again was first declared, when the semantic pass says it declares a name again.
    pub(crate) redeclares: Option<Offset>,
}

/// Whether a file's names are worked out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum IndexState {
    /// Not yet, for this file.
    #[default]
    Pending,
    /// They are.
    Indexed,
}

/// The names a file's declarations bind, worked out on a reader's first lookup.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct BindingIndex {
    /// Whether the names are worked out for the file.
    pub state: IndexState,
    /// The names, by where the name each declares was first declared, then by their own start: the declarations of one name are
    /// together, its first declaration first.
    pub names: Vec<BoundName>,
}

/// One name a declaration binds: the `defines` row it gives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Definition {
    /// The declaration: a parameter, or the node that declares the name.
    pub id: Position,
    /// The binding identifier that names it.
    pub binding: Position,
    /// The scope the name binds in.
    pub owner: Position,
    /// What kind of declaration it is.
    pub kind: DeclarationKind,
}
