//! The names of the `diagnostics` concept: the code of a diagnostic without a narrower one, and the query names of its phases and impacts.

use super::types::DiagnosticCode;

/// The code of a diagnostic a provider reports without a narrower stable one.
pub const UNSPECIFIED_CODE: DiagnosticCode = DiagnosticCode("unspecified");

/// The query name of the diagnostic phase of a parser.
pub(super) const PARSE_PHASE: &str = "parse";

/// The query name of the diagnostic phase of a validation pass.
pub(super) const VALIDATION_PHASE: &str = "validation";

/// The query name of a diagnostic that leaves the tree complete.
pub(super) const PRESERVED_IMPACT: &str = "preserved";

/// The query name of a diagnostic that leaves the tokens complete to the end of the file and the tree not.
pub(super) const LEXICALLY_COMPLETE_IMPACT: &str = "lexically-complete";

/// The query name of a diagnostic that leaves the tree incomplete.
pub(super) const INCOMPLETE_IMPACT: &str = "incomplete";
