//! Stable operation vocabulary and artifact identities of a corpus measurement.
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
pub(super) const TRIALS: usize = 3;
/// The comparison of the reference's cold and retained-cache reports.
pub(super) const REFERENCE_RETAINED: &str = "reference retained-cache";
/// The comparison of the candidate's cold and retained-cache reports.
pub(super) const CANDIDATE_RETAINED: &str = "candidate retained-cache";
/// The comparison of the reference's edited retained report and its fresh report.
pub(super) const REFERENCE_EDITED: &str = "reference edited retained vs fresh";
/// The comparison of the candidate's edited retained report and its fresh report.
pub(super) const CANDIDATE_EDITED: &str = "candidate edited retained vs fresh";
/// An explicit cohort is measured only within the rules its certificate covers.
pub(super) const COHORT_OUTSIDE_CERTIFICATE: &str =
    "measured cohort names a rule the certificate does not cover";
/// Refusal of a project with no source left to lint after the suffix filter and the Valid profile.
pub(super) const NO_SOURCE: &str = "no source left to lint after discovery and the valid profile";
/// The directory below a run's state that the reference keeps its own state in.
pub(super) const REFERENCE_STATE: &str = "reference";
/// The directory below a run's state that the candidate keeps its own state in: the store measured.
pub(super) const CANDIDATE_STATE: &str = "candidate";
/// The directory the growth scenes are written under, below the run's cache.
pub(super) const SCENES_DIRECTORY: &str = "scenes";
/// Refusal of a pair whose certification does not permit publishing its costs.
pub(super) const NOT_COMPLETE: &str = "only a complete inventory's costs are published";
/// Refusal of a candidate run that collected no CPU or peak.
pub(super) const NO_RESOURCES: &str = "the candidate's run collected no CPU time or peak";
/// Refusal of a growth lint that did not complete.
pub(super) const GROWTH_INCOMPLETE: &str = "a growth lint did not complete";
/// What a project's per-rule judgement reports first.
pub(super) const UNACCEPTED: &str = "unaccepted differences";
/// Where a bench keeps the differences accepted for its projects, relative to the bench.
pub(super) const ACCEPTED_DIRECTORY: &str = "profiles/evidence";
/// The suffix of a project's accepted-differences file, after the project's name.
pub(super) const ACCEPTED_SUFFIX: &str = "-accepted-differences.json";
