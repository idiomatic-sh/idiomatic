//! Names owned by the worker identity check.
/// The refusal when the request names a parser configuration other than the executable's.
pub(crate) const WORKER_PARSER_CONFIGURATION_DIFFER: &str =
    "worker parser configuration differs from its profile identity";
/// The refusal when the request carries no selected-rule applicability.
pub(crate) const WORKER_REQUIRES_EXPLICIT_SELECTED: &str =
    "worker requires explicit selected rule applicability";
/// The refusal when a rule's applicability is not a pinned phase policy.
pub(crate) const WORKER_RULE_APPLICABILITY_DIFFERS: &str =
    "worker rule applicability differs from the pinned phase policy";
/// The applicability of a rule that runs on every source.
pub(crate) const ALWAYS: &str = "always";
/// The applicability of a rule that runs only on sources the parser accepts.
pub(crate) const ACCEPTED_SYNTAX: &str = "accepted-syntax";
