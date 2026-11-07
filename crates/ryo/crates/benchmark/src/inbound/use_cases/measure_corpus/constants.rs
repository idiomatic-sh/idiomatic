//! Stable operation vocabulary and artifact identities of a corpus measurement.
/// Directory below the cache that holds the preflight state of each project.
pub(super) const PREFLIGHT_DIRECTORY: &str = "preflight";
/// Name prefix of the fresh directory one trial of a project runs in.
pub(super) const TRIAL_PREFIX: &str = "trial-";
/// Directory below one trial that holds the retained cache state.
pub(super) const RETAINED_DIRECTORY: &str = "retained";
/// Directory below one trial that holds the fresh state of the edited sources.
pub(super) const CLEAN_EDITED_DIRECTORY: &str = "clean-edited";
/// The observation condition of an empty tool cache.
pub(super) const EMPTY_CACHE_CONDITION: &str = "empty-tool-cache";
/// The observation condition of a retained tool cache.
pub(super) const RETAINED_CACHE_CONDITION: &str = "retained-tool-cache";
/// The observation condition of a retained tool cache after the sources were edited.
pub(super) const EDITED_CACHE_CONDITION: &str = "edited-tool-cache";
/// The name the candidate's samples carry.
pub(super) const CANDIDATE_NAME: &str = "ryo";
/// Separator in human-readable excluded-path observations.
pub(super) const PATH_SEPARATOR: &str = ", ";
/// Separator between explicit per-rule finding counts.
pub(super) const FINDING_SEPARATOR: &str = ", ";
/// Increment for one diagnostic occurrence.
pub(super) const ONE_FINDING: usize = 1;
/// Stable observation label for reference-first execution.
pub(super) const REFERENCE_FIRST: &str = "ReferenceFirst";
/// Stable observation label for candidate-first execution.
pub(super) const CANDIDATE_FIRST: &str = "CandidateFirst";
/// How many ordered trials one project is measured in, alternating which contestant runs first.
pub(super) const TRIALS: usize = 6;
/// The comparison of the reference's cold and retained-cache reports.
pub(super) const REFERENCE_RETAINED: &str = "reference retained-cache";
/// The comparison of the candidate's cold and retained-cache reports.
pub(super) const CANDIDATE_RETAINED: &str = "candidate retained-cache";
/// The comparison of the reference's edited retained report and its fresh report.
pub(super) const REFERENCE_EDITED: &str = "reference edited retained vs fresh";
/// The comparison of the candidate's edited retained report and its fresh report.
pub(super) const CANDIDATE_EDITED: &str = "candidate edited retained vs fresh";
