//! Digests and archive rules of an explicit process failure, over plain inputs.
use super::constants;
use super::errors::ProcessFailureError;
use crate::domain::canonical_text::{
    service::{digest, render},
    types::{Fragment, Label, Quantity},
};
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::process::types::{
    OutputPart, PathEncoding, ProcessArtifact, ProcessFailure, ReferenceExpectation,
};
use ryo_benchmark_contracts::domain::workloads::types::{
    PreparedCase, PreparedReferenceBehavior, Tool,
};
use std::collections::{BTreeMap, BTreeSet};

fn parts(values: &[OutputPart]) -> Fragment {
    Fragment::List(
        values
            .iter()
            .map(|part| match part {
                OutputPart::Literal(value) => Fragment::Object(BTreeMap::from([
                    (
                        Label::from(constants::KIND),
                        Fragment::from(constants::LITERAL),
                    ),
                    (
                        Label::from(constants::VALUE),
                        Fragment::from(value.0.as_str()),
                    ),
                ])),
                OutputPart::InputPath { file, encoding } => Fragment::Object(BTreeMap::from([
                    (
                        Label::from(constants::KIND),
                        Fragment::from(constants::INPUT_PATH),
                    ),
                    (
                        Label::from(crate::domain::canonical_text::constants::FILE),
                        Fragment::from(&*file.to_string_lossy()),
                    ),
                    (
                        Label::from(constants::ENCODING),
                        Fragment::from(match encoding {
                            PathEncoding::Utf8Bytes => constants::UTF8_BYTES,
                            PathEncoding::JsonStringContent => constants::JSON_STRING_CONTENT,
                        }),
                    ),
                ])),
            })
            .collect(),
    )
}

fn artifact(value: &ProcessArtifact) -> Fragment {
    Fragment::Object(BTreeMap::from([
        (
            Label::from(constants::PATH),
            Fragment::from(&*value.path.to_string_lossy()),
        ),
        (
            Label::from(constants::SHA256),
            Fragment::from(value.sha256.0.as_str()),
        ),
    ]))
}

/// Closed expectation lookup; a process failure is never a diagnostic report.
pub(crate) fn expected(case: &PreparedCase) -> Option<&ProcessFailure> {
    match &case.reference_discrepancy {
        PreparedReferenceBehavior::Differs(item) => match &item.expected {
            ReferenceExpectation::ProcessFailure(process) => Some(process),
            ReferenceExpectation::Diagnostics(_) => None,
        },
        PreparedReferenceBehavior::Agrees => None,
    }
}

/// Hash the exact template contract, without temporary physical roots.
pub(crate) fn output_digest(process: &ProcessFailure) -> ArtifactDigest {
    digest(&Fragment::Object(BTreeMap::from([
        (
            Label::from(constants::KIND),
            Fragment::from(constants::PROCESS_FAILURE),
        ),
        (
            Label::from(constants::EXIT),
            Fragment::Number(Quantity(i64::from(process.exit.0))),
        ),
        (Label::from(constants::SIGNAL), Fragment::Null),
        (Label::from(constants::STDOUT), parts(&process.stdout)),
        (Label::from(constants::STDERR), parts(&process.stderr)),
    ])))
}

/// Bind the chosen exact-binary archive group into both runtime and offline receipts.
pub(crate) fn group_digest(
    process: &ProcessFailure,
    executable: &ArtifactDigest,
) -> Result<ArtifactDigest, ProcessFailureError> {
    let mut groups = BTreeMap::<&str, usize>::new();
    let mut artifacts = BTreeSet::new();
    for observation in &process.observations {
        for raw in [
            &observation.invocation,
            &observation.stdout,
            &observation.stderr,
        ] {
            if !artifacts.insert(&raw.path) {
                return Err(ProcessFailureError::GroupsShareCaptures);
            }
        }
        *groups.entry(&observation.executable_sha256.0).or_default() += 1;
    }
    if groups.values().any(|count| *count < 2)
        || groups
            .get(executable.0.as_str())
            .is_none_or(|count| *count < 2)
    {
        return Err(ProcessFailureError::NoReproducibleGroup);
    }
    let mut rows = process
        .observations
        .iter()
        .filter(|row| row.executable_sha256 == *executable)
        .map(|row| {
            Fragment::Object(BTreeMap::from([
                (
                    Label::from(constants::EXECUTABLE_SHA256),
                    Fragment::from(row.executable_sha256.0.as_str()),
                ),
                (
                    Label::from(constants::INVOCATION),
                    artifact(&row.invocation),
                ),
                (Label::from(constants::STDOUT), artifact(&row.stdout)),
                (Label::from(constants::STDERR), artifact(&row.stderr)),
            ]))
        })
        .collect::<Vec<_>>();
    rows.sort_by_key(|row| render(row).0);
    Ok(digest(&Fragment::List(rows)))
}

/// Every archived observation is of an approved binary, shares no raw artifact, and the actual binary has a pair.
pub(crate) fn check_archives(
    process: &ProcessFailure,
    profile: &Tool,
    actual: &ArtifactDigest,
) -> Result<(), ProcessFailureError> {
    let mut seen = BTreeSet::new();
    let mut groups = BTreeMap::<&str, usize>::new();
    for observation in &process.observations {
        if !profile
            .sha256
            .iter()
            .any(|pin| pin.0 == observation.executable_sha256.0)
        {
            return Err(ProcessFailureError::UnapprovedExactBinary);
        }
        *groups.entry(&observation.executable_sha256.0).or_default() += 1;
        for artifact in [
            &observation.invocation,
            &observation.stdout,
            &observation.stderr,
        ] {
            if !seen.insert(&artifact.path) {
                return Err(ProcessFailureError::ObservationsShareArtifacts);
            }
        }
    }
    if groups.is_empty()
        || groups.values().any(|count| *count < 2)
        || groups.get(actual.0.as_str()).is_none_or(|count| *count < 2)
    {
        return Err(ProcessFailureError::ActualBinaryNeedsTwoCaptures);
    }
    Ok(())
}
