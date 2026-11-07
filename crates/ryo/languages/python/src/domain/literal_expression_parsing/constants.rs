//! The names of the `literal_expression_parsing` concept.
/// In-process literal parse memo mode; parser implementation identity is checked separately.
pub(crate) const GRAMMAR: crate::domain::format_syntax::types::GrammarIdentity =
    crate::domain::format_syntax::types::GrammarIdentity("python-string-annotation-carrier");
/// The text `literal-expression parsing requires complete logical string `.
pub(crate) const INCOMPLETE: &str =
    "literal-expression parsing requires complete logical string carriers";
/// The text `(\n`.
pub(crate) const CARRIER_OPEN: &str = "(\n";
/// The text `parsed`.
pub(crate) const PARSED: &str = "parsed";
/// The text `malformed`.
pub(crate) const MALFORMED: &str = "malformed";
/// The text `unavailable`.
pub(crate) const UNAVAILABLE: &str = "unavailable";
/// Most hosts of capacity the embedded-expression set keeps across a reset.
pub(crate) const EMBEDDED_CAPACITY: usize = 256;
