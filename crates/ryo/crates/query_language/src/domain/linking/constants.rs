//! Fixed vocabulary and invariant diagnostics owned by this concept.

/// An import alias is declared more than once.
pub(super) const DUPLICATE_IMPORT_ALIAS: &str = "an import alias is declared more than once";

/// The returned step is not declared in this fragment.
pub(super) const UNDECLARED_RETURNED_STEP: &str =
    "the returned step is not declared in this fragment";

/// A fragment step has no definition identity.
pub(super) const MISSING_STEP_IDENTITY: &str = "a fragment step has no definition identity";

/// The call result has no definition identity.
pub(super) const MISSING_CALL_IDENTITY: &str = "the call result has no definition identity";

/// Fragment expands to no step definitions.
pub(super) const EMPTY_FRAGMENT: &str = "fragment expands to no step definitions";

/// Fragment result has no linked identity.
pub(super) const MISSING_RESULT_IDENTITY: &str = "fragment result has no linked identity";

/// This filter position requires a text or number parameter.
pub(super) const INVALID_FILTER_PARAMETER: &str =
    "this filter position requires a text or number parameter";

/// The root origin was just assigned.
pub(super) const ASSIGNED_ROOT_ORIGIN: &str = "the root origin was just assigned";
