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
/// The directory the growth scenes are written under, below the run's cache.
pub(super) const SCENES_DIRECTORY: &str = "scenes";
/// Refusal of a pair whose certification does not permit publishing its costs.
pub(super) const NOT_COMPLETE: &str = "only a complete inventory's costs are published";
