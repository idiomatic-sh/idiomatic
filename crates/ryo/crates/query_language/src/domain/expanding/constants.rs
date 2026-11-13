//! Fixed vocabulary and invariant diagnostics owned by this concept.

/// Fragment expands to no step definitions.
pub(super) const EMPTY_FRAGMENT: &str = "fragment expands to no step definitions";

/// The call result has no definition identity.
pub(super) const MISSING_CALL_IDENTITY: &str = "the call result has no definition identity";

/// Fragment result has no linked identity.
pub(super) const MISSING_RESULT_IDENTITY: &str = "fragment result has no linked identity";

/// A fragment step has no definition identity.
pub(super) const MISSING_STEP_IDENTITY: &str = "a fragment step has no definition identity";

/// The returned step is not declared in this fragment.
pub(super) const UNDECLARED_RETURNED_STEP: &str =
    "the returned step is not declared in this fragment";
