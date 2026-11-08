//! The names of the `repository` concept: the labels that begin the bytes the identities of a run are made of.

/// Begins the bytes that say what the catalog of relations is.
pub(crate) const RELATION_SCHEMA_LABEL: &str = "ryo-relation-catalog";

/// Begins the bytes that say which source selection a request asked for.
pub(crate) const SOURCE_SELECTION_LABEL: &str = "ryo-source-selection-primary-role";

/// Begins the bytes that say which compiled plan a rule executes, after every rewrite that shapes what its frontier slots hold.
pub(crate) const COMPILED_PLAN_LABEL: &str = "ryo-compiled-plan";
