//! The types of the `fact_notes` concept: plain data, public fields and derives, with no constructor and no inherent impl. The notes of a
//! file are these types; the port `notes` writes them as bytes and reads them back.

use ryo::languages::Offset;

/// What the dotted path of an import names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Form {
    /// A module: `import a.b` names the module `a.b`.
    Module,
    /// A module, or a name its last part's module defines: `from a import b` writes `a.b`, the module `a.b` or the name `b` of `a`.
    Name,
    /// Every name of a module: `from a import *` writes `a`.
    Everything,
}

/// How many packages up a relative import starts: the number of its leading dots; none for an absolute import.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Level(pub u32);

/// The dotted path of an import without its leading dots: `a.b` for `from ..a import b`; empty for `from . import *`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct DottedPath(pub Box<str>);

/// An import as the file writes it from where it starts to where it ends, its whitespace left out: `workers`, `a.b as c`.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Spelling(pub Box<str>);

/// One name an import brings in, as the `static_import` relation reads it: one for each alias of an `import` or a `from ... import`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ImportNote {
    /// What the path names.
    pub form: Form,
    /// How many packages up it starts.
    pub level: Level,
    /// The path.
    pub path: DottedPath,
    /// Where the alias starts, in bytes.
    pub start: Offset,
    /// Where it ends, in bytes.
    pub end: Offset,
    /// The alias as the file writes it from `start` to `end`, its whitespace left out.
    pub text: Spelling,
}

/// The notes of a file: the imports it writes, in source order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Note {
    /// The imports.
    pub imports: Vec<ImportNote>,
}
