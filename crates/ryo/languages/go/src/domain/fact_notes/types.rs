//! The types of the `fact_notes` concept: plain data, public fields and derives, with no constructor and no inherent impl. The notes of a
//! file are these types; the port `notes` writes them as bytes and reads them back.

use ryo::languages::Offset;

/// An import path as the code writes it, decoded: `fmt`, `example.com/app/internal/store`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ImportPath(pub Box<str>);

/// An import spec as the file writes it from where it starts to where it ends, its whitespace left out: `store"example.com/app/store"`.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Spelling(pub Box<str>);

/// One package a file imports: one spec of an `import` declaration.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ImportNote {
    /// The path it names.
    pub path: ImportPath,
    /// Where the spec starts, in bytes.
    pub start: Offset,
    /// Where it ends.
    pub end: Offset,
    /// The spec as the file writes it from `start` to `end`, its whitespace left out.
    pub text: Spelling,
}

/// The notes of a file: the packages it imports, in source order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Note {
    /// The imports.
    pub imports: Vec<ImportNote>,
}
