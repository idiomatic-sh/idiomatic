//! The names the `postcard` adapter's wire shapes carry: postcard writes none of them, and serde asks for each.

/// The name of a note's shape.
pub(super) const NOTE: &str = "Note";
/// The name of an import's shape.
pub(super) const IMPORT: &str = "ImportNote";
/// The packages a file imports.
pub(super) const IMPORTS: &str = "imports";
/// The path an import names.
pub(super) const PATH: &str = "path";
/// Where an import starts.
pub(super) const START: &str = "start";
/// Where an import ends.
pub(super) const END: &str = "end";
/// An import spec as the file writes it, its whitespace left out.
pub(super) const TEXT: &str = "text";
/// How many fields a note has.
pub(super) const NOTE_FIELDS: usize = 1;
/// How many fields an import has.
pub(super) const IMPORT_FIELDS: usize = 4;
