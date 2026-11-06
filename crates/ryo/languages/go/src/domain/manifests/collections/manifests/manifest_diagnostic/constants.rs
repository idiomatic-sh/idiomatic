//! Stable labels for the diagnostics of manifests the tool refused.

/// Where a diagnostic with no finer place is: the origin of the file.
pub(super) const FILE_ORIGIN: &str = "file-origin";

/// The stage of a diagnostic of text the tool could not read.
pub(super) const PARSE: &str = "parse";

/// The schema of a `go.mod`.
pub(super) const MODULE_SCHEMA: &str = "go-module";

/// The schema of a `go.work`.
pub(super) const WORKSPACE_SCHEMA: &str = "go-workspace";

/// The category of a `go.mod` the tool refused.
pub(super) const INVALID_MODULE: &str = "invalid-go-mod";

/// The category of a `go.work` the tool refused.
pub(super) const INVALID_WORKSPACE: &str = "invalid-go-work";
