//! The texts of the `lint` use case.

/// What the answer of a task says when no worker answered it.
pub(super) const TASK_ANSWERED: &str = "a task every worker took until none was left";

/// How many rows an answer of the rules' own columns writes when its caller does not say.
pub const DEFAULT_LIMIT: u64 = 20;

/// The text "store_staged_descriptors".
pub(super) const STORE_STAGED_DESCRIPTORS: &str = "store_staged_descriptors";

/// The text "store_handed".
pub(super) const STORE_HANDED: &str = "store_handed";

/// The text "cross_rule_memory".
pub(super) const CROSS_RULE_MEMORY: &str = "cross_rule_memory";

/// The text "worker_notes".
pub(super) const WORKER_NOTES: &str = "worker_notes";

/// The text "cross_symbols".
pub(super) const CROSS_SYMBOLS: &str = "cross_symbols";

/// The text "worker_arenas".
pub(super) const WORKER_ARENAS: &str = "worker_arenas";

/// The text "fresh_records".
pub(super) const FRESH_RECORDS: &str = "fresh_records";

/// The text "<request>".
pub(super) const REQUEST: &str = "<request>";

/// The text "after_files".
pub(super) const AFTER_FILES: &str = "after_files";

/// The text "after_store_write".
pub(super) const AFTER_STORE_WRITE: &str = "after_store_write";

/// The text "before_store_write".
pub(super) const BEFORE_STORE_WRITE: &str = "before_store_write";

/// The text "after_store_build".
pub(super) const AFTER_STORE_BUILD: &str = "after_store_build";

/// The text "before_store_build".
pub(super) const BEFORE_STORE_BUILD: &str = "before_store_build";

/// The text "named publication was not admitted by the request owner".
pub(super) const NAMED_PUBLICATION_WAS_NOT_ADMITTED_BY: &str =
    "named publication was not admitted by the request owner";

/// The text "before_cross_cleanup".
pub(super) const BEFORE_CROSS_CLEANUP: &str = "before_cross_cleanup";

/// The text "missing frontier bucket".
pub(super) const MISSING_FRONTIER_BUCKET: &str = "missing frontier bucket";

/// The tracing target of the lifecycle memory diagnostics.
pub(super) const LIFECYCLE_MEMORY_DIAGNOSTIC: &str = "ryo::diagnostic::lifecycle_memory";
