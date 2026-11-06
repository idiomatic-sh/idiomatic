//! Bind both expected outputs, independent evidence and the actual approved reference invocation.
use super::constants;
use super::errors::DiscrepancyValidationError as Failure;
use crate::domain::canonical_text::{service::text_digest, types::Document};
use crate::domain::discrepancy::{
    service::{check_evidence, output_digest, reference_digest, require_distinct, validate_pins},
    types::{OptionalDigest, ReferenceKind, ReferenceProof},
};
use crate::domain::invocation_identity::service::{effective_observations, normalize};
use crate::domain::process_failure::service::{check_archives, expected, group_digest};
use crate::domain::registered_inputs::types::RefusalText;
use crate::outbound::inventory::types::{InvocationDocument, Pin, Signal};
use crate::outbound::inventory::types::{observation_document::ExitStatus, text::YesNo};
use crate::outbound::{
    linter::interface::LinterPort,
    suite::{
        interface::SuitePort,
        types::{
            ArtifactPath,
            discrepancy::{CaseArtifacts, EvidenceArtifact, EvidenceRequirement},
        },
    },
};
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::artifacts::types::{FileExecutable, FileMetadataEntry};
use ryo_benchmark_contracts::domain::measurements::types::Invocation;
use ryo_benchmark_contracts::domain::process::types::ProcessFailure;
use ryo_benchmark_contracts::domain::workloads::types::{
    PreparedCase, Tool, discrepancy_evidence::value::DiscrepancyEvidence,
};
use std::path::{Path, PathBuf};

fn refused(error: impl std::fmt::Display) -> Failure {
    Failure::Refused(RefusalText(error.to_string()))
}

/// Refuse stale evidence across the selected inventory before either contestant executes.
pub(crate) fn validate_all(
    suite: &dyn SuitePort,
    reference: &dyn LinterPort,
    rules: &Path,
    cases: &[PreparedCase],
) -> Result<Vec<Option<ReferenceProof>>, Failure> {
    cases
        .iter()
        .map(|case| validate(suite, reference, rules, case))
        .collect()
}

/// The canonical normalized settings text of one invocation, bound to the case and its profile.
pub(super) fn settings_text(
    invocation: &Invocation,
    case: &PreparedCase,
    profile: &Tool,
) -> Result<Document, Failure> {
    let normalized = normalize(invocation, case)?;
    let observations = effective_observations(case, profile);
    Ok(Document(
        crate::outbound::worker::schemas::invocation_settings_text(
            &normalized,
            case,
            profile,
            observations.as_ref(),
        )?,
    ))
}

fn requirement(row: &DiscrepancyEvidence) -> EvidenceRequirement {
    match row {
        DiscrepancyEvidence::Language { .. } => EvidenceRequirement::Language,
        DiscrepancyEvidence::PublicToolContract {
            source_revision,
            source_path,
            ..
        } => EvidenceRequirement::PublicToolContract {
            source_revision: source_revision.clone(),
            source_path: source_path.clone(),
        },
        DiscrepancyEvidence::PriorWorkerRun {
            worker_sha256,
            control_source_sha256,
            control_query_sha256,
            control_profile_sha256,
            witness_case,
            ..
        } => EvidenceRequirement::PriorWorkerRun {
            worker_sha256: worker_sha256.0.clone(),
            control_source_sha256: control_source_sha256.0.clone(),
            control_query_sha256: control_query_sha256.0.clone(),
            control_profile_sha256: control_profile_sha256.0.clone(),
            witness_case: witness_case.clone(),
        },
    }
}

/// Bind both expected outputs, independent evidence and the actual approved reference invocation.
pub(crate) fn validate(
    suite: &dyn SuitePort,
    reference: &dyn LinterPort,
    rules: &Path,
    case: &PreparedCase,
) -> Result<Option<ReferenceProof>, Failure> {
    let Some(item) = &case.reference_discrepancy else {
        return Ok(None);
    };
    require_distinct(case, item)?;
    let profile_root = rules.parent().ok_or(Failure::MissingProfileRoot)?;
    let profile = suite.profile(ArtifactPath(profile_root))?;
    let identity = &item.identity;
    let invocation = reference.prepare(&case.request)?;
    suite.verify_tool(ArtifactPath(&invocation.executable), &profile)?;
    let actual_executable = suite.digest(ArtifactPath(&invocation.executable))?;
    validate_pins(
        identity,
        &profile,
        &actual_executable,
        &suite.digest(ArtifactPath(&profile_root.join(constants::BENCHMARK_TOML)))?,
    )?;
    let settings = text_digest(&settings_text(&invocation, case, &profile)?);
    let files = crate::domain::suite::service::materialized_sources(&case.request);
    let evidence = item
        .evidence
        .iter()
        .map(|row| EvidenceArtifact {
            path: row.artifact().0.clone(),
            sha256: row.artifact().1.0.clone(),
            requirement: requirement(row),
        })
        .collect::<Vec<_>>();
    check_evidence(item, &profile)?;
    let metadata_files = case
        .request
        .file_metadata
        .as_ref()
        .map(|values| {
            values
                .keys()
                .map(|file| PathBuf::from(file.0.as_ref()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    suite
        .verify_discrepancy_artifacts(CaseArtifacts {
            profile: profile_root,
            root: &case.request.root,
            files: &files,
            file_metadata_files: &metadata_files,
            source_sha256: &identity.source_sha256.0,
            file_metadata_sha256: identity
                .file_metadata_sha256
                .as_ref()
                .map(|value| value.0.as_str()),
            evidence: &evidence,
        })
        .map_err(refused)?;
    let metadata =
        ArtifactDigest::file_metadata(case.request.file_metadata.as_ref().map(|values| {
            values.iter().map(|(path, executable)| FileMetadataEntry {
                path,
                executable: if *executable {
                    FileExecutable::Set
                } else {
                    FileExecutable::Unset
                },
            })
        }))?;
    if metadata != identity.file_metadata_sha256
        || settings != identity.settings_sha256
        || output_digest(&case.expected) != identity.intended_output_sha256
        || reference_digest(&item.expected) != identity.reference_output_sha256
    {
        return Err(Failure::StaleIdentity);
    }
    let process = expected(case);
    if let Some(process) = process {
        validate_archives(
            suite,
            profile_root,
            &profile,
            case,
            process,
            &actual_executable,
        )?;
    }
    Ok(Some(ReferenceProof {
        basis: item.basis,
        process_group: match process {
            Some(process) => OptionalDigest::Pinned(group_digest(process, &actual_executable)?),
            None => OptionalDigest::Absent,
        },
        kind: if process.is_some() {
            ReferenceKind::ProcessFailure
        } else {
            ReferenceKind::Diagnostics
        },
        settings,
        executable: actual_executable,
        profile: identity.profile_sha256.clone(),
        source: identity.source_sha256.clone(),
        reference_output: identity.reference_output_sha256.clone(),
        intended_output: identity.intended_output_sha256.clone(),
        file_metadata: identity
            .file_metadata_sha256
            .clone()
            .map_or(OptionalDigest::Absent, OptionalDigest::Pinned),
    }))
}

/// Validate every archived observation and require an exact pair for the actual binary.
pub(crate) fn validate_archives(
    suite: &dyn SuitePort,
    profile_root: &Path,
    profile: &Tool,
    case: &PreparedCase,
    process: &ProcessFailure,
    actual: &ArtifactDigest,
) -> Result<(), Failure> {
    let identity = &case
        .reference_discrepancy
        .as_ref()
        .ok_or(Failure::ArchivedBytesDiffer)?
        .identity;
    check_archives(process, profile, actual)?;
    let artifacts = process
        .observations
        .iter()
        .flat_map(|observation| {
            [
                &observation.invocation,
                &observation.stdout,
                &observation.stderr,
            ]
        })
        .map(|artifact| EvidenceArtifact {
            path: artifact.path.clone(),
            sha256: artifact.sha256.0.clone(),
            requirement: EvidenceRequirement::Language,
        })
        .collect::<Vec<_>>();
    let bytes = suite
        .read_discrepancy_artifacts(ArtifactPath(profile_root), &artifacts)
        .map_err(refused)?;
    let files = case
        .request
        .files
        .iter()
        .map(|file| PathBuf::from(file.0.as_ref()))
        .collect::<Vec<_>>();
    for observation in &process.observations {
        let record = bytes
            .get(&observation.invocation.path)
            .ok_or(Failure::ArchivedBytesDiffer)?;
        let InvocationDocument::Complete(metadata) = suite.decode_invocation(record)? else {
            return Err(Failure::ArchivedBytesDiffer);
        };
        let root = metadata.actual_request_root.0.as_str();
        let stdout = crate::domain::suite::process::service::expand(
            &process.stdout,
            Path::new(root),
            &files,
        )
        .map_err(refused)?;
        let stderr = crate::domain::suite::process::service::expand(
            &process.stderr,
            Path::new(root),
            &files,
        )
        .map_err(refused)?;
        let source = identity.source_sha256.0.as_str();
        let file_metadata = identity
            .file_metadata_sha256
            .as_ref()
            .map_or(Pin::Absent, |digest| {
                Pin::Sha256(crate::outbound::inventory::types::text::Sha256Text(
                    digest.0.clone(),
                ))
            });
        if !metadata.files.iter().map(|file| file.0.as_str()).eq(case
            .request
            .files
            .iter()
            .map(|file| file.0.as_ref()))
            || metadata.executable_sha256.0 != observation.executable_sha256.0
            || metadata.profile_sha256.0 != identity.profile_sha256.0
            || metadata.source_sha256
                != Pin::Sha256(crate::outbound::inventory::types::text::Sha256Text(
                    source.to_owned(),
                ))
            || metadata.file_metadata_sha256 != file_metadata
            || metadata.settings_sha256.0 != identity.settings_sha256.0
            || metadata.tool.0 != profile.tool.0
            || metadata.version.0 != profile.version.0
            || metadata.exit != ExitStatus(i64::from(process.exit))
            || metadata.signal != Signal::Absent
            || metadata.observer_no_deadline != YesNo::Yes
            || bytes.get(&observation.stdout.path) != Some(&stdout)
            || bytes.get(&observation.stderr.path) != Some(&stderr)
        {
            return Err(Failure::ArchivedBytesDiffer);
        }
        let mut archived_case = case.clone();
        archived_case.request.root = root.into();
        archived_case.request.state = metadata.state.0.as_str().into();
        let archived = Invocation {
            executable: PathBuf::new(),
            arguments: metadata
                .arguments
                .iter()
                .map(|argument| argument.0.as_str().into())
                .collect(),
            environment: metadata
                .environment
                .iter()
                .map(|(name, value)| (name.0.as_str().into(), value.0.as_str().into()))
                .collect(),
            directory: metadata.directory.0.as_str().into(),
        };
        let text = settings_text(&archived, &archived_case, profile)?;
        if text.0 != metadata.normalized_settings.canonical.0
            || text_digest(&text) != identity.settings_sha256
        {
            return Err(Failure::ArchivedSettingsNotEstablished);
        }
    }
    Ok(())
}
