//! The words of the `proofs` concept: the problems a proof of a version says.

/// A scenario that neither brings a `tastemaker.toml` in its `given.files` nor is placed by the keys of its own.
pub const UNPLACED_SCENARIO: &str =
    "it has no tastemaker.toml in `given.files` and nothing else places what it proves";

/// A version with no definition file.
pub const NO_DEFINITION_FILE: &str = "the version has no definition file";

/// A definition whose `description` says nothing.
pub const BLANK_DESCRIPTION: &str = "its `description` is blank";

/// A definition whose `why` says nothing.
pub const BLANK_WHY: &str = "its `why` is blank";

/// No proof matches the current version inputs.
pub const UNPROVEN: &str = "missing or stale validation";

/// Reason reported when the scheduler finished without running the job that proves the versions.
pub const UNEXECUTED_PROOFS: &str = "proof job was not executed";
