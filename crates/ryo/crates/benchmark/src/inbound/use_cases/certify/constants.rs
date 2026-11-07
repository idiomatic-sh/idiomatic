//! Stable operation vocabulary and artifact identities.
/// Authored rule directory sealed by certification.
pub(crate) const RULE_DIRECTORY: &str = "rules";
/// Refuse changed content fingerprints of certified artifacts.
pub(crate) const CHANGED_EVIDENCE: &str = "certification inputs changed during cases";

// Independent executable fixture inputs and expected refusal vocabulary.
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture positive source.
pub(crate) const FIXTURE_POSITIVE_SOURCE: &str = "x";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture rule id.
pub(crate) const FIXTURE_RULE_ID: &str = "A";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture fake program.
pub(crate) const FIXTURE_FAKE_PROGRAM: &str = "fixture";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture profile directory.
pub(crate) const FIXTURE_PROFILE_DIRECTORY: &str = "profile";
/// Independent certification fixture rule directory name.
pub(crate) const FIXTURE_RULE_DIRECTORY_NAME: &str = "rules";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture tool file.
#[cfg(not(windows))]
pub(crate) const FIXTURE_TOOL_FILE: &str = "tool";
/// Windows command scripts require their explicit extension for native process launch.
#[cfg(all(windows, any(test, feature = "fixtures")))]
pub(crate) const FIXTURE_TOOL_FILE: &str = "tool.cmd";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture tool script.
#[cfg(not(windows))]
pub(crate) const FIXTURE_TOOL_SCRIPT: &str = "#!/bin/sh\necho fixture 1\n";
/// Fixed Windows fixture with no argument interpolation; verification supplies only `--version`.
/// `std::process::Command` launches `.cmd` files through the platform command processor.
#[cfg(all(windows, any(test, feature = "fixtures")))]
pub(crate) const FIXTURE_TOOL_SCRIPT: &str = "@echo fixture 1\r\n";
/// Independent certification fixture profile file.
pub(crate) const FIXTURE_PROFILE_FILE: &str = ryo_benchmark_contracts::domain::requests::constants::PROFILE_MANIFEST;
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture query file.
pub(crate) const FIXTURE_QUERY_FILE: &str = "A/query.ryoql";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture query content.
pub(crate) const FIXTURE_QUERY_CONTENT: &str = "query";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture cases file.
pub(crate) const FIXTURE_CASES_FILE: &str = "A/cases.toml";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture workload directory.
pub(crate) const FIXTURE_WORKLOAD_DIRECTORY: &str = "workload";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture project content.
pub(crate) const FIXTURE_PROJECT_CONTENT: &str = "pinned project";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture patch file.
pub(crate) const FIXTURE_PATCH_FILE: &str = "patch.diff";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture patch content.
pub(crate) const FIXTURE_PATCH_CONTENT: &str = "pinned patch";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture collector file.
pub(crate) const FIXTURE_COLLECTOR_FILE: &str = "collector";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture case directory.
pub(crate) const FIXTURE_CASE_DIRECTORY: &str = "cases";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture source directory.
pub(crate) const FIXTURE_SOURCE_DIRECTORY: &str = "sources";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture source file.
pub(crate) const FIXTURE_SOURCE_FILE: &str = "input";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture state directory.
pub(crate) const FIXTURE_STATE_DIRECTORY: &str = "cache";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture altered argument.
pub(crate) const FIXTURE_ALTERED_ARGUMENT: &str = "different defaults";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture altered bytes.
pub(crate) const FIXTURE_ALTERED_BYTES: &str = "changed";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture no early execution.
pub(crate) const FIXTURE_NO_EARLY_EXECUTION: &str = "refusal must precede process launch";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture expected command refusal.
pub(crate) const FIXTURE_EXPECTED_COMMAND_REFUSAL: &str =
    "contestant command profile differs from case certification";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture expected instrumentation refusal.
pub(crate) const FIXTURE_EXPECTED_INSTRUMENTATION_REFUSAL: &str =
    "execution profile differs from case certification";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture expected artifact refusal.
pub(crate) const FIXTURE_EXPECTED_ARTIFACT_REFUSAL: &str =
    "certified profile, rules, cases or executable changed";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture case records.
pub(crate) const FIXTURE_CASE_RECORDS: &str = "cases";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture case name.
pub(crate) const FIXTURE_CASE_NAME: &str = "name";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture case files.
pub(crate) const FIXTURE_CASE_FILES: &str = "files";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture case expected.
pub(crate) const FIXTURE_CASE_EXPECTED: &str = "expected";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture finding file.
pub(crate) const FIXTURE_FINDING_FILE: &str = "file";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture finding start.
pub(crate) const FIXTURE_FINDING_START: &str = "start";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture finding end.
pub(crate) const FIXTURE_FINDING_END: &str = "end";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture positive name.
pub(crate) const FIXTURE_POSITIVE_NAME: &str = "positive";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture negative name.
pub(crate) const FIXTURE_NEGATIVE_NAME: &str = "negative";
#[cfg(any(test, feature = "fixtures"))]
/// Independent certification fixture negative source.
pub(crate) const FIXTURE_NEGATIVE_SOURCE: &str = "y";

/// A setup failure invalidates the certification fixture rather than the product result.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const FIXTURE_PRECONDITION: &str = "certification fixture precondition";
/// Certificate identity must bind actual artifact content.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const FIXTURE_DIGEST_REQUIRED: &str =
    "certification must bind an actual artifact digest";
/// Each controlled mutation has an independently specified diagnostic.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const FIXTURE_REFUSAL_REQUIRED: &str = "mutation must produce its specific refusal";
/// Text of the missing verification command in this part.
pub(crate) const MISSING_VERIFICATION_COMMAND: &str =
    crate::domain::suite::constants::MISSING_VERIFICATION_COMMAND;
/// Text of the query ryoql in this part.
pub(crate) const QUERY_RYOQL: &str = crate::domain::suite::constants::QUERY_RYOQL;
