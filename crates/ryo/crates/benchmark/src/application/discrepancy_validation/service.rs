//! Bind both expected outputs, independent evidence and the actual approved reference invocation.
use super::errors::DiscrepancyValidationError as Failure;
use crate::domain::canonical_text::{service::text_digest, types::Document};
use crate::domain::discrepancy::{
    service::{check_evidence, output_digest, reference_digest, require_distinct, validate_pins},
    types::{OptionalDigest, ReferenceKind, ReferenceProof},
};
use crate::domain::invocation_identity::service::normalize;
use crate::domain::process_failure::service::{check_archives, expected, group_digest};
use crate::domain::registered_inputs::types::RefusalText;
use ryo_benchmark_contracts::domain::documents::types::invocation_document::value::InvocationDocument;
use ryo_benchmark_contracts::domain::documents::types::text::value::{Pin, Signal};
use ryo_benchmark_contracts::domain::documents::types::text::value::{ExitStatus, YesNo};
use crate::outbound::{
    inventory::interface::InventoryPort,
    linter::interface::LinterPort,
    suite::{
        interface::SuitePort,
        types::{
            ArtifactPath,
            ArtifactMetadata, CaseArtifacts, EvidenceArtifact, EvidenceRequirement,
        },
    },
};
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::artifacts::types::FileMetadataEntry;
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
    inventory: &dyn InventoryPort,
    reference: &dyn LinterPort,
    rules: &Path,
    cases: &[PreparedCase],
) -> Result<Vec<Option<ReferenceProof>>, Failure> {
    cases
        .iter()
        .map(|case| validate(suite, inventory, reference, rules, case))
        .collect()
}

/// The canonical normalized settings text of one invocation, bound to the case and its profile.
pub(crate) fn settings_text(
    linter: &dyn LinterPort,
    invocation: &Invocation,
    case: &PreparedCase,
    profile: &Tool,
) -> Result<Document, Failure> {
    let normalized = normalize(invocation, case)?;
    Ok(Document(linter.settings_text(&normalized, case, profile)?.0))
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
            worker_sha256: worker_sha256.clone(),
            control_source_sha256: control_source_sha256.clone(),
            control_query_sha256: control_query_sha256.clone(),
            control_profile_sha256: control_profile_sha256.clone(),
            witness_case: witness_case.clone(),
        },
    }
}

/// Bind both expected outputs, independent evidence and the actual approved reference invocation.
pub(crate) fn validate(
    suite: &dyn SuitePort,
    inventory: &dyn InventoryPort,
    reference: &dyn LinterPort,
    rules: &Path,
    case: &PreparedCase,
) -> Result<Option<ReferenceProof>, Failure> {
    let ryo_benchmark_contracts::domain::workloads::types::PreparedReferenceBehavior::Differs(item) = &case.reference_discrepancy else {
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
        &suite.digest(ArtifactPath(&profile_root.join(crate::domain::suite::constants::BENCHMARK_TOML)))?,
    )?;
    let settings = text_digest(&settings_text(reference, &invocation, case, &profile)?);
    let files = crate::domain::suite::service::materialized_sources(&case.request);
    let evidence = item
        .evidence
        .iter()
        .map(|row| EvidenceArtifact {
            path: row.artifact().path.clone(),
            sha256: row.artifact().sha256.clone(),
            requirement: requirement(row),
        })
        .collect::<Vec<_>>();
    check_evidence(item, &profile)?;
    let metadata_files = match &case.request.file_metadata {
        ryo_benchmark_contracts::domain::requests::types::FileMetadataDeclaration::Declared(values) => values
            .keys()
            .map(|file| PathBuf::from(file.0.as_ref()))
            .collect::<Vec<_>>(),
        ryo_benchmark_contracts::domain::requests::types::FileMetadataDeclaration::Undeclared => Vec::new(),
    };
    suite
        .verify_discrepancy_artifacts(CaseArtifacts {
            profile: profile_root,
            root: &case.request.root,
            files: &files,
            file_metadata_files: &metadata_files,
            source_sha256: &identity.source_sha256,
            file_metadata: match &identity.file_metadata_sha256 {
                ryo_benchmark_contracts::domain::workloads::types::MetadataDigestDeclaration::Declared(value) => ArtifactMetadata::Declared(value),
                ryo_benchmark_contracts::domain::workloads::types::MetadataDigestDeclaration::Undeclared => ArtifactMetadata::Undeclared,
            },
            evidence: &evidence,
        })
        .map_err(refused)?;
    let metadata =
        ArtifactDigest::file_metadata(match &case.request.file_metadata {
            ryo_benchmark_contracts::domain::requests::types::FileMetadataDeclaration::Declared(values) => Some(values.iter().map(|(path, executable)| FileMetadataEntry {
                path,
                executable: *executable,
            })),
            ryo_benchmark_contracts::domain::requests::types::FileMetadataDeclaration::Undeclared => None,
        })?;
    let declared_metadata = match &identity.file_metadata_sha256 {
        ryo_benchmark_contracts::domain::workloads::types::MetadataDigestDeclaration::Declared(value) => Some(value.clone()),
        ryo_benchmark_contracts::domain::workloads::types::MetadataDigestDeclaration::Undeclared => None,
    };
    if metadata != declared_metadata
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
            inventory,
            reference,
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
        file_metadata: match &identity.file_metadata_sha256 {
            ryo_benchmark_contracts::domain::workloads::types::MetadataDigestDeclaration::Declared(value) => OptionalDigest::Pinned(value.clone()),
            ryo_benchmark_contracts::domain::workloads::types::MetadataDigestDeclaration::Undeclared => OptionalDigest::Absent,
        },
    }))
}

/// Validate every archived observation and require an exact pair for the actual binary.
pub(crate) fn validate_archives(
    suite: &dyn SuitePort,
    inventory: &dyn InventoryPort,
    linter: &dyn LinterPort,
    profile_root: &Path,
    profile: &Tool,
    case: &PreparedCase,
    process: &ProcessFailure,
    actual: &ArtifactDigest,
) -> Result<(), Failure> {
    let ryo_benchmark_contracts::domain::workloads::types::PreparedReferenceBehavior::Differs(discrepancy) = &case.reference_discrepancy else {
        return Err(Failure::ArchivedBytesDiffer);
    };
    let identity = &discrepancy.identity;
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
            sha256: artifact.sha256.clone(),
            requirement: EvidenceRequirement::Language,
        })
        .collect::<Vec<_>>();
    let bytes = suite
        .read_discrepancy_artifacts(ArtifactPath(profile_root), &artifacts)
        .map_err(refused)?
        .into_iter()
        .map(|(path, bytes)| (path, bytes.0))
        .collect::<std::collections::BTreeMap<_, _>>();
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
        let InvocationDocument::Complete(metadata) = inventory.decode_invocation(record).map_err(refused)? else {
            return Err(Failure::ArchivedBytesDiffer);
        };
        let root = metadata.actual_request_root.0.as_str();
        let stdout = crate::domain::process_output::service::expand(
            &process.stdout,
            Path::new(root),
            &files,
        )
        .map_err(refused)?
        .0;
        let stderr = crate::domain::process_output::service::expand(
            &process.stderr,
            Path::new(root),
            &files,
        )
        .map_err(refused)?
        .0;
        let source = identity.source_sha256.0.as_str();
        let file_metadata = match &identity.file_metadata_sha256 {
            ryo_benchmark_contracts::domain::workloads::types::MetadataDigestDeclaration::Declared(digest) => Pin::Sha256(
                ryo_benchmark_contracts::domain::documents::types::text::value::Sha256Text(
                    digest.0.clone(),
                ),
            ),
            ryo_benchmark_contracts::domain::workloads::types::MetadataDigestDeclaration::Undeclared => Pin::Absent,
        };
        if !metadata.files.iter().map(|file| file.0.as_str()).eq(case
            .request
            .files
            .iter()
            .map(|file| file.0.as_ref()))
            || metadata.executable_sha256.0 != observation.executable_sha256.0
            || metadata.profile_sha256.0 != identity.profile_sha256.0
            || metadata.source_sha256
                != Pin::Sha256(ryo_benchmark_contracts::domain::documents::types::text::value::Sha256Text(
                    source.to_owned(),
                ))
            || metadata.file_metadata_sha256 != file_metadata
            || metadata.settings_sha256.0 != identity.settings_sha256.0
            || metadata.tool.0 != profile.tool.0
            || metadata.version.0 != profile.version.0
            || metadata.exit != ExitStatus(i64::from(process.exit.0))
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
        let text = settings_text(linter, &archived, &archived_case, profile)?;
        if text.0 != metadata.normalized_settings.canonical.0
            || text_digest(&text) != identity.settings_sha256
        {
            return Err(Failure::ArchivedSettingsNotEstablished);
        }
    }
    Ok(())
}
