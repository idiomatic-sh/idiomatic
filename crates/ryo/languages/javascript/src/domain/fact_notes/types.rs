//! The types of the `fact_notes` concept: plain data, public fields and derives, with no constructor and no inherent impl. The notes of a
//! file are these types; the port `notes` writes them as bytes and reads them back.

use ryo::languages::Offset;

/// A module specifier as the code writes it, decoded: `./a`, `../b/index.js`, `@scope/pkg`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Specifier(pub Box<str>);

/// An import as the file writes it from where it starts to where it ends, its whitespace left out: `import{a}from'./a'`.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Spelling(pub Box<str>);

/// One module a file imports: an `import` or `export ... from` declaration, an `import(...)` of a string, or a `require(...)` of one.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ImportNote {
    /// The module it names.
    pub specifier: Specifier,
    /// Where the declaration or the expression starts, in bytes.
    pub start: Offset,
    /// Where it ends.
    pub end: Offset,
    /// The import as the file writes it from `start` to `end`, its whitespace left out.
    pub text: Spelling,
}

/// The notes of a file: the modules it imports, in source order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Note {
    /// The imports.
    pub imports: Vec<ImportNote>,
}
