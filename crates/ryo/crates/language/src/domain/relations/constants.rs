//! The names of the `relations` concept: what a kind of column is called when a listing shows it.

use super::types::{CellKind, FieldId, FileId, Flag, Flags, Offset, Span};
use crate::domain::terms::types::Symbol;

/// The text of a kind.
#[must_use]
pub fn kind_name(kind: CellKind) -> &'static str {
    match kind {
        CellKind::Node => "node",
        CellKind::Symbol => "name",
        CellKind::File => "file",
        CellKind::Span => "span",
        CellKind::Number => "number",
        CellKind::Flag => "flag",
    }
}

/// The name of the `package-dependency` relation, whose kinds and origins the terms name their words by.
pub const PACKAGE_DEPENDENCY: &str = crate::domain::terms::constants::DEPENDENCY_OWNER.0;

/// The name of the column of a relation that says what kind of thing a row is, when it has one.
pub const KIND: &str = "kind";

/// A column of every relation that has it, named the same in each: The node a row is about, where it occurs.
pub const OCCURRENCE: &str = "occurrence";

/// A column of every relation that has it, named the same in each: What a row points at.
pub const TARGET: &str = "target";

/// A column of every relation that has it, named the same in each: Where a row stands among its siblings, from zero.
pub const ORDINAL: &str = "ordinal";

/// A column of every relation that has it, named the same in each: The name a row binds.
pub const BINDING: &str = "binding";

/// A column of every relation that has it, named the same in each: The package a row names.
pub const PACKAGE: &str = "package";

/// A column of every relation that has it, named the same in each: What a row points from.
pub const SOURCE: &str = "source";

/// A column of every relation that has it, named the same in each: The declaration a row is about.
pub const DECLARATION: &str = "declaration";

/// A column of every relation that has it, named the same in each: The node that holds a row's node.
pub const PARENT: &str = "parent";

/// Why every relation of code is read in part in a file with syntax errors: the tree is a recovery of what was written.
pub const SYNTAX_ERRORS: &str = "the file has syntax errors";

/// Why every relation of code is read in part in a file no language could parse.
pub const UNPARSED: &str = "the file could not be parsed";

/// How a raised flag is written.
pub const RAISED: &str = "true";

/// How a lowered flag is written.
pub const LOWERED: &str = "false";

impl std::fmt::Display for Flag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Raised => RAISED,
            Self::Lowered => LOWERED,
        })
    }
}

/// Where the places of diagnostics start: the low half of the id of a report the parser made is this and its number, above every place of
/// an element, so a report never has the id of a node.
pub const DIAGNOSTIC_BASE: u32 = 0x8000_0000;

/// Flag of an element that is a token, not a node.
pub const TOKEN: Flags = Flags(1);

/// What a token that carries no text holds in its `text` column.
pub const NO_SYMBOL: Symbol = Symbol(u32::MAX);

/// What a column of files holds where a row names no file of the repository: `package-dependency`'s `target` for a package the
/// repository does not hold. It is shown as empty text and joins no file.
pub const NO_FILE: FileId = FileId(u32::MAX);

/// A column of every relation of a data document's values, named the same in each (`entry`, `sequence-element`): the value decoded
/// as a yes or no when it is a boolean (`true` is raised); lowered for a value of every other kind, which `kind` tells apart.
pub const BOOLEAN: &str = "boolean";

/// A column of every relation of a data document's values, named the same in each (`entry`, `sequence-element`): the value decoded
/// as a whole number when it is an integer (`0x1F` is 31, `1_000` is 1000); zero for a value of every other kind, which `kind` tells
/// apart, and for an integer the engine's numbers cannot hold, whose `text` keeps it.
pub const INTEGER: &str = "integer";

/// A column of every relation of a data document's values, named the same in each (`entry`, `sequence-element`): the value decoded
/// as a number when it is a float, written in the shortest decimal form that reads back as the same number (`1e3` is `1000.0`,
/// `6.626e-34` stays, `+inf` is `inf`, `nan` is `NaN`); empty for a value of every other kind, which `kind` tells apart. A rule
/// orders it as a number with `float_text_gte`.
pub const FLOAT: &str = "float";

/// A column of every relation of a data document's values, named the same in each (`entry`, `sequence-element`): the value decoded
/// when it is a date, a time or both, written as RFC 3339 writes it (`1979-05-27 07:32:00Z` is `1979-05-27T07:32:00Z`), a local
/// date, time or date-time without the parts it leaves out; empty for a value of every other kind, which `kind` tells apart.
pub const DATETIME: &str = "datetime";

/// A column of every relation of a data document's values, named the same in each (`entry`, `sequence-element`): the base an
/// integer is written in, 16 for `0xFF`, 8 for `0o17`, 2 for `0b101` and 10 for a decimal; zero for a value of every other kind.
pub const RADIX: &str = "radix";

/// A column of `package-dependency`: the branch of a git repository the dependency follows, as the manifest writes it (Cargo's,
/// Poetry's and uv's `branch`); empty when it names none.
pub const BRANCH: &str = "branch";

/// A column of `package-dependency`: the tag of a git repository the dependency is taken at, as the manifest writes it (Cargo's,
/// Poetry's and uv's `tag`); empty when it names none.
pub const TAG: &str = "tag";

/// A column of `package-dependency`: a revision of a git repository the dependency is taken at, named without saying whether it
/// is a branch, a tag or a commit, as the manifest writes it (Cargo's, Poetry's and uv's `rev`, Pipenv's `ref`, the `@` of a PEP
/// 508 git URL, the `#` of a package.json git spec); empty when it names none.
pub const REVISION: &str = "revision";

/// A column of `package-dependency`: the versions of the language's interpreter the dependency applies under, as the manifest
/// writes them on the dependency itself, as a requirement on the interpreter; empty when it writes none. A platform expression is the
/// `condition`.
pub const INTERPRETER: &str = "interpreter";

/// A column of `package-dependency`: the shared table the manifest takes the requirement from instead of writing it, as it spells
/// that (pnpm's and bun's `catalog:` and `catalog:<name>`, Cargo's `workspace = true`); empty when it writes the requirement
/// itself. `requirement` holds what that table writes.
pub const CATALOG: &str = "catalog";

/// A column of `package-dependency`: what the manifest puts in the place of the package depended on, as it writes it (a go.mod
/// `replace` target, `example.com/fork v1.2.0` or `../util`; the directory or repository of Cargo's `[patch]` and `[replace]`; an
/// `overrides` spec of package.json; an `override-dependencies` requirement of uv); empty when nothing replaces it.
pub const REPLACEMENT: &str = "replacement";

/// The column that says where a dependency's package comes from, one of the words of `DependencyOrigin`.
pub const ORIGIN: &str = "origin";

/// The column that says whether a string literal is an import specifier: the text an import, an export from or another statement or
/// call that loads code writes to name the module, package or file it loads.
pub const IMPORT: &str = "import";

/// The role of an edge that has none.
pub const NO_FIELD: FieldId = FieldId(0);

/// An absent location. This invalid interval can never denote source bytes.
pub const NO_SPAN: Span = Span {
    start: Offset(u32::MAX),
    end: Offset(0),
};

/// Marks a node implied by the parser, without a source location.
pub const IMPLIED: Flags = Flags(2);

/// The displayed value of a location explicitly absent from the parser tree.
pub const UNLOCATED: &str = "unlocated";
