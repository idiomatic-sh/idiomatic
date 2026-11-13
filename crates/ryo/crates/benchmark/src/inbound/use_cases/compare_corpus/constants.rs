//! Stable vocabulary of a corpus comparison.
/// Refusal of a request whose selected rules are not exactly the configured rules.
pub(super) const SELECTION_DIFFERS: &str =
    "parity requires the request to select exactly the configured rules";
/// Refusal of a run with differences the accepted differences do not cover.
pub(super) const UNACCEPTED: &str = "unaccepted parity differences";
/// The refusal of a discovered source the project does not track.
pub(super) const UNTRACKED_SOURCE: &str = "the reference discovered a source the project does not track";
/// What joins a project's name and the reference's in the published evidence.
pub(super) const PROJECT_SEPARATOR: &str = "/";
/// The prefix of a failed project's report.
pub(super) const PROJECT_FAILED: &str = "parity failed";
/// The refusal when any project failed.
pub(super) const PROJECTS_FAILED: &str = "parity failed for projects";
