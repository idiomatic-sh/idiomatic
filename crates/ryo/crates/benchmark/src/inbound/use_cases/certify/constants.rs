//! Stable operation vocabulary and artifact identities.
/// Stable filename used only to compare prepared invocation identities.
pub(super) const PROFILE_PROBE: &str = "__certificate_probe__";
/// Separate state directory for preparation identity probes.
pub(super) const IDENTITY_STATE: &str = "identity";
/// Authored rule directory sealed by certification.
pub(super) const RULE_DIRECTORY: &str = "rules";
/// Refuse changed content fingerprints of certified artifacts.
pub(super) const CHANGED_EVIDENCE: &str = "certification inputs changed during cases";

// Independent executable fixture inputs and expected refusal vocabulary.
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture positive source.
pub(super) const FIXTURE_POSITIVE_SOURCE: &str = "x";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture rule id.
pub(super) const FIXTURE_RULE_ID: &str = "A";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture fake program.
pub(super) const FIXTURE_FAKE_PROGRAM: &str = "fixture";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture profile directory.
pub(super) const FIXTURE_PROFILE_DIRECTORY: &str = "profile";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture rule directory name.
pub(super) const FIXTURE_RULE_DIRECTORY_NAME: &str = "rules";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture tool file.
#[cfg(not(windows))]
pub(super) const FIXTURE_TOOL_FILE: &str = "tool";
/// Windows command scripts require their explicit extension for native process launch.
#[cfg(all(windows, any(test, feature = "fixtures")))]
pub(super) const FIXTURE_TOOL_FILE: &str = "tool.cmd";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture tool script.
#[cfg(not(windows))]
pub(super) const FIXTURE_TOOL_SCRIPT: &str = "#!/bin/sh\necho fixture 1\n";
/// Fixed Windows fixture with no argument interpolation; verification supplies only `--version`.
/// `std::process::Command` launches `.cmd` files through the platform command processor.
#[cfg(all(windows, any(test, feature = "fixtures")))]
pub(super) const FIXTURE_TOOL_SCRIPT: &str = "@echo fixture 1\r\n";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture profile file.
pub(super) const FIXTURE_PROFILE_FILE: &str = "benchmark.toml";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture query file.
pub(super) const FIXTURE_QUERY_FILE: &str = "A/query.ryoql";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture query content.
pub(super) const FIXTURE_QUERY_CONTENT: &str = "query";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture cases file.
pub(super) const FIXTURE_CASES_FILE: &str = "A/cases.toml";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture workload directory.
pub(super) const FIXTURE_WORKLOAD_DIRECTORY: &str = "workload";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture project content.
pub(super) const FIXTURE_PROJECT_CONTENT: &str = "pinned project";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture patch file.
pub(super) const FIXTURE_PATCH_FILE: &str = "patch.diff";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture patch content.
pub(super) const FIXTURE_PATCH_CONTENT: &str = "pinned patch";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture collector file.
pub(super) const FIXTURE_COLLECTOR_FILE: &str = "collector";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture case directory.
pub(super) const FIXTURE_CASE_DIRECTORY: &str = "cases";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture source directory.
pub(super) const FIXTURE_SOURCE_DIRECTORY: &str = "sources";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture source file.
pub(super) const FIXTURE_SOURCE_FILE: &str = "input";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture state directory.
pub(super) const FIXTURE_STATE_DIRECTORY: &str = "cache";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture altered argument.
pub(super) const FIXTURE_ALTERED_ARGUMENT: &str = "different defaults";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture altered bytes.
pub(super) const FIXTURE_ALTERED_BYTES: &str = "changed";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture no early execution.
pub(super) const FIXTURE_NO_EARLY_EXECUTION: &str = "refusal must precede process launch";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture expected command refusal.
pub(super) const FIXTURE_EXPECTED_COMMAND_REFUSAL: &str =
    "contestant command profile differs from case certification";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture expected instrumentation refusal.
pub(super) const FIXTURE_EXPECTED_INSTRUMENTATION_REFUSAL: &str =
    "execution profile differs from case certification";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture expected artifact refusal.
pub(super) const FIXTURE_EXPECTED_ARTIFACT_REFUSAL: &str =
    "certified profile, rules, cases or executable changed";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture case records.
pub(super) const FIXTURE_CASE_RECORDS: &str = "cases";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture case name.
pub(super) const FIXTURE_CASE_NAME: &str = "name";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture case files.
pub(super) const FIXTURE_CASE_FILES: &str = "files";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture case expected.
pub(super) const FIXTURE_CASE_EXPECTED: &str = "expected";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture finding file.
pub(super) const FIXTURE_FINDING_FILE: &str = "file";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture finding start.
pub(super) const FIXTURE_FINDING_START: &str = "start";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture finding end.
pub(super) const FIXTURE_FINDING_END: &str = "end";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture positive name.
pub(super) const FIXTURE_POSITIVE_NAME: &str = "positive";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture negative name.
pub(super) const FIXTURE_NEGATIVE_NAME: &str = "negative";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture negative source.
pub(super) const FIXTURE_NEGATIVE_SOURCE: &str = "y";
