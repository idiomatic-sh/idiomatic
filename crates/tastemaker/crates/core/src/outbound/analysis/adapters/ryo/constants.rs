//! The names the ryo adapter declares to ryo.

/// The host relation of placed files.
pub(super) const PLACED: &str = crate::domain::applications::constants::PLACED;
/// Its columns, in order.
pub(super) const PLACED_COLUMNS: [&str; 6] = [
    "path",
    "entry",
    crate::domain::packages::constants::PACKAGE_KEY,
    crate::domain::packages::constants::TEMPLATE_WORD,
    "version",
    crate::domain::packages::constants::SPEC_WORD,
];
/// What the catalog says `tastemaker.placed` is.
pub(super) const PLACED_SUMMARY: &str =
    "A file a template entry placed, and the entry; Tastemaker's own relation";

/// Why a blank query is not one ryo runs.
pub(super) const BLANK_QUERY: &str = "the query is blank";

/// How ryo begins the refusal of an operation a query calls that it has none of.
pub(super) const UNKNOWN_OPERATION: &str = "no operation named";
/// Where a query's author finds the relations there are.
pub(super) const RELATIONS_LISTED: &str = "`tm list relations` names every relation";
/// Where a query's author finds the operators there are.
pub(super) const OPERATORS_LISTED: &str = "`tm list operators` names every operator";

/// How the catalog names the kind of a column that holds a file.
pub(super) const FILE_KIND: &str = "file";
/// How the catalog names the kind of a column that holds text.
pub(super) const TEXT_KIND: &str = "text";

/// How the engine writes a span as text, between its start and its end: `12..40`.
pub(super) const SPAN_TEXT: &str = crate::domain::applications::constants::SPAN_SEPARATOR;

/// Ryo's own directory within the store.
pub(super) const RYO_STORE: &str = "ryo";
