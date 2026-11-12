//! The names the `postcard` adapter's wire shapes carry: postcard writes none of them, and serde asks for each.

/// The name of a note's shape.
pub(super) const NOTE: &str = "Note";
/// The name of a declaration's shape.
pub(super) const DECLARED: &str = "DeclaredNote";
/// The name of an inline block's shape.
pub(super) const STEP: &str = "StepNote";
/// The name of a written path's shape.
pub(super) const WRITTEN: &str = "WrittenNote";
/// Whether the file parses without errors.
pub(super) const CLEAN: &str = "clean";
/// The modules a file declares.
pub(super) const DECLARED_FIELD: &str = "declared";
/// The paths a file writes.
pub(super) const WRITTEN_FIELD: &str = "written";
/// The inline blocks around a declaration.
pub(super) const AROUND: &str = "around";
/// A module's name.
pub(super) const NAME: &str = "name";
/// Whether a module is written inline.
pub(super) const INLINE: &str = "inline";
/// The value of a `#[path]`.
pub(super) const PATH: &str = "path";
/// Whether a declaration is under `#[cfg(test)]`.
pub(super) const TEST_ONLY: &str = "test_only";
/// Whether a declaration is `pub`.
pub(super) const EXPORTED: &str = "exported";
/// The segments of a written path.
pub(super) const SEGMENTS: &str = "segments";
/// The inline modules around a written path.
pub(super) const ENCLOSING: &str = "enclosing";
/// Where a written path starts.
pub(super) const START: &str = "start";
/// Where a written path ends.
pub(super) const END: &str = "end";
/// A written path as the file writes it, its whitespace left out.
pub(super) const TEXT: &str = "text";
/// The name a `use` leaf binds.
pub(super) const BOUND: &str = "bound";
/// Where a `use` leaf binds its name.
pub(super) const SCOPE: &str = "scope";
/// How many fields a note has.
pub(super) const NOTE_FIELDS: usize = 3;
/// How many fields a declaration has.
pub(super) const DECLARED_FIELDS: usize = 6;
/// How many fields an inline block has.
pub(super) const STEP_FIELDS: usize = 2;
/// How many fields a written path has.
pub(super) const WRITTEN_FIELDS: usize = 7;
