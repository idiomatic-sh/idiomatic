//! The names of the `elements` concept: the text its code shows.

/// What a worker's scratch is called when it is shown.
pub(super) const SCRATCH: &str = "Scratch";

/// What a read of the element table says when it is given a position the table never gave.
pub(super) const FOREIGN_POSITION: &str = "a position this table gave";

/// The code of a diagnostic a provider reports without a narrower stable one.
pub(super) const UNSPECIFIED_CODE: &str = "unspecified";

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
