//! The types of the `definitions` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::signatures::types::Refinement;
use ryo::languages::{DeclarationKind, Position};

/// What the element a reader is given declares, by its kind: the readers branch on it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Shape {
    /// A function or a method.
    Function,
    /// A type.
    Type,
    /// The names of a constant or variable declaration.
    Values,
    /// A name an import brings in.
    Import,
    /// The names of a field, a parameter, a result, a receiver or a type parameter, and the type an embedded field names.
    Field,
    /// The names a short variable declaration or a `range` clause declares.
    Variables,
    /// The label a labeled statement declares.
    Label,
    /// Nothing.
    Nothing,
}

/// How the name of a declaration is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NameSpelling {
    /// By the identifier at the position of `named`.
    Identifier,
    /// By the type at the position of `named`: an embedded field is named by its type, without its pointer and its package.
    Embedded,
    /// By the import at the position of `named`'s spec: the name it gives, or the unknown value of the package's own.
    Import,
}

/// What a declaration is: its shared kind and its word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Defined {
    /// The shared kind.
    pub kind: DeclarationKind,
    /// The same in Go's own words.
    pub word: Refinement,
}

/// A name a file declares, as the `defines` relation says it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Declaration {
    /// What it declares: the shared kind and Go's word for it.
    pub defined: Defined,
    /// How its name is written.
    pub written: NameSpelling,
    /// The scope that binds it.
    pub owner: Position,
    /// The element whose span is the name's.
    pub named: Position,
}

/// What a list of fields declares: a kind, a word, and what owns them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Meaning {
    /// The shared kind.
    pub kind: DeclarationKind,
    /// The same in Go's own words.
    pub word: Refinement,
    /// What owns them.
    pub owner: Position,
}

/// A type that annotates, and what it annotates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Annotation {
    /// The type, as written.
    pub annotation: Position,
    /// The name it annotates, or the field that has none.
    pub target: Position,
}

/// A name an assignment binds to a string whose value is known.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Assignment {
    /// The name it binds.
    pub binding: Position,
    /// The string it binds it to.
    pub value: Position,
}
