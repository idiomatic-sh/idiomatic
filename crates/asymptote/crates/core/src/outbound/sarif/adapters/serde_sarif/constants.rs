//! The names SARIF uses.

/// The fingerprint GitHub code scanning fills, the one a tool's own identity is read from.
pub(super) const LINE_HASH: &str = "primaryLocationLineHash";
/// The base every printed result's path is relative to.
pub(super) const PRODUCT_ROOT: &str = "PRODUCTROOT";
/// The schema a printed log names.
pub(super) const SCHEMA: &str = "https://json.schemastore.org/sarif-2.1.0.json";
/// Where the tool that prints it is documented.
pub(super) const INFORMATION: &str = "https://github.com/idiomatic-sh/idiomatic";
/// A file URI's scheme.
pub(super) const FILE_SCHEME: &str = "file://";
/// Why no output is no log.
pub(super) const NOTHING: &str = crate::domain::judgement::constants::NOTHING;
/// The status of a suppression that suppresses.
pub(super) const ACCEPTED: &str = "accepted";
/// The region's fields, as a refusal names them.
pub(super) const START_LINE: &str = "startLine";
/// The region's start column.
pub(super) const START_COLUMN: &str = "startColumn";
/// The region's end line.
pub(super) const END_LINE: &str = "endLine";
/// The region's end column.
pub(super) const END_COLUMN: &str = "endColumn";
/// The one SARIF version a log is read in.
pub(super) const VERSION: &str = "2.1.0";
