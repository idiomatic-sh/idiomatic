//! Convert one verification outcome into a durable boundary-owned observation.
use super::types::ProcessEvidence;
use crate::domain::canonical_text::types::Label;
use crate::domain::process_output::types::ReceiptKeyParts;
use crate::domain::discrepancy::types::{
    Contestant, OptionalDigest, ReferenceKind, ReferenceProof,
};
use crate::outbound::context::interface::ContextPort;
use ryo_benchmark_contracts::domain::documents::types::invocation_document::value::InvocationDocument;
use ryo_benchmark_contracts::domain::documents::types::text::value::Pin;
use crate::outbound::receipts::interface::ReceiptsPort;
use crate::outbound::receipts::types::{CapturedProcess, CaseReceipts, ReceiptPath};
use crate::outbound::suite::interface::SuitePort;
use crate::outbound::suite::types::{ArtifactPath, ArtifactSet};
use ryo_benchmark_contracts::domain::documents::types::finding::value::{ExpectedFindings, Finding, FindingOffset, SkippedPair};
use ryo_benchmark_contracts::domain::documents::types::observation_document::value::{ContextMode, ContextRun, IntentMatch, ListedContext, ObservationDocument, ProcessArtifactsRecord, ProcessRecord, ProjectContextRecord, RecordedMetadata, RecordedReason, ReferenceProcess, ReportedFindings, ReportedSkips, RequestRoot, ValidationRecord, RuleCohort, RuleContextRecord, RuleObservationsRecord, SourceIdentity, ToolContractRecord};
use ryo_benchmark_contracts::domain::documents::types::text::value::{CanonicalJson, CaseName, ContestantSide, ExitStatus, FileMetadataDigest, PathText, ProjectInputsDigest, ReasonText, RuleName, SchemaName, Sha256Text, YesNo};
use ryo_benchmark_contracts::domain::documents::types::reference_discrepancy::value::{DiscrepancyValidation, ExpectationKind};
use ryo_benchmark_contracts::domain::requests::types::{
    ContextSeal, CohortSelection, InputDeclaration, RuleContextPreparation, SearchRoots,
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::{CoverageAccounting, ProjectContext, RunReport};
use ryo_benchmark_contracts::domain::reports::types::{BenchmarkFinding, ComparisonContract};
use ryo_benchmark_contracts::domain::workloads::types::DiscrepancyBasis;
use ryo_benchmark_contracts::domain::workloads::types::PreparedCase;
use std::path::Path;

fn findings(rows: &[BenchmarkFinding]) -> Vec<Finding> {
    rows.iter()
        .map(|row| Finding {
            rule: RuleName(row.rule.as_ref().to_owned()),
            file: PathText(row.file.0.as_ref().to_owned()),
            start: FindingOffset(u64::try_from(row.start.0).unwrap_or(u64::MAX)),
            end: FindingOffset(u64::try_from(row.end.0).unwrap_or(u64::MAX)),
        })
        .collect()
}

fn pinned(digest: &OptionalDigest) -> Pin {
    match digest {
        OptionalDigest::Absent => Pin::Absent,
        OptionalDigest::Pinned(digest) => Pin::Sha256(Sha256Text(digest.0.clone())),
    }
}

/// The file metadata identity a proof pins, as a document declares it.
fn declared_metadata(digest: &OptionalDigest) -> FileMetadataDigest {
    match digest {
        OptionalDigest::Absent => FileMetadataDigest::Undeclared,
        OptionalDigest::Pinned(digest) => FileMetadataDigest::Declared(Sha256Text(digest.0.clone())),
    }
}

/// The validation the discrepancy check produced.
fn validation_document(proof: &ReferenceProof) -> DiscrepancyValidation {
    DiscrepancyValidation {
        discrepancy_basis: proof.basis,
        process_observation_group_sha256: pinned(&proof.process_group),
        reference_expectation_kind: match proof.kind {
            ReferenceKind::ProcessFailure => ExpectationKind::ProcessFailure,
            ReferenceKind::Diagnostics => ExpectationKind::Diagnostics,
        },
        settings_sha256: Sha256Text(proof.settings.0.clone()),
        actual_executable_sha256: Sha256Text(proof.executable.0.clone()),
        profile_sha256: Sha256Text(proof.profile.0.clone()),
        source_sha256: Sha256Text(proof.source.0.clone()),
        reference_output_sha256: Sha256Text(proof.reference_output.0.clone()),
        intended_output_sha256: Sha256Text(proof.intended_output.0.clone()),
        file_metadata_sha256: declared_metadata(&proof.file_metadata),
    }
}

/// The reference process a captured invocation records.
fn process_observation(case: &PreparedCase, invocation: &InvocationDocument) -> ProcessRecord {
    let (exit, signal, root, observer, executable, settings, metadata_error) = match invocation {
        InvocationDocument::Complete(complete) => (
            complete.exit,
            complete.signal,
            &complete.actual_request_root,
            complete.observer_no_deadline,
            Pin::Sha256(complete.executable_sha256.clone()),
            Pin::Sha256(complete.settings_sha256.clone()),
            RecordedReason::Absent,
        ),
        InvocationDocument::Failed(failed) => (
            failed.exit,
            failed.signal,
            &failed.actual_request_root,
            failed.observer_no_deadline,
            Pin::Absent,
            Pin::Absent,
            RecordedReason::Stated(failed.metadata_error.clone()),
        ),
    };
    ProcessRecord {
        kind: ExpectationKind::ProcessFailure,
        exit,
        signal,
        actual_request_root: root.clone(),
        observer_no_deadline: observer,
        executable_sha256: executable,
        settings_sha256: settings,
        metadata_error,
        template_sha256: crate::domain::process_failure::service::expected(case).map_or(
            Pin::Absent,
            |process| {
                Pin::Sha256(Sha256Text(
                    crate::domain::process_failure::service::output_digest(process).0,
                ))
            },
        ),
        artifacts: ProcessArtifactsRecord::Unattached,
    }
}

/// Bind the source and file metadata identities the record establishes into the raw invocation.
fn bound(
    invocation: &InvocationDocument,
    source: Pin,
    metadata: &FileMetadataDigest,
) -> InvocationDocument {
    let mut invocation = invocation.clone();
    let (source_pin, metadata_pin) = match &mut invocation {
        InvocationDocument::Complete(complete) => {
            (&mut complete.source_sha256, &mut complete.file_metadata_sha256)
        }
        InvocationDocument::Failed(failed) => {
            (&mut failed.source_sha256, &mut failed.file_metadata_sha256)
        }
    };
    *source_pin = source;
    if let FileMetadataDigest::Declared(digest) = metadata {
        *metadata_pin = Pin::Sha256(digest.clone());
    }
    invocation
}

/// Append one complete side record while preserving the unprojected report and source identity.
pub(crate) fn write(
    suite: &dyn SuitePort,
    receipts: &dyn ReceiptsPort,
    context: &dyn ContextPort,
    case: &PreparedCase,
    contestant: Contestant,
    report: Option<&RunReport>,
    error: Option<&ReasonText>,
    passed: YesNo,
    validation: Option<&ReferenceProof>,
    process: ProcessEvidence<'_>,
) -> Result<(), Error> {
    let CaseReceipts::Recorded(path) = receipts.case_receipts() else {
        return Ok(());
    };
    let side = match contestant {
        Contestant::Reference => ContestantSide::Reference,
        Contestant::Candidate => ContestantSide::Candidate,
    };
    let files = report
        .map(|report| {
            report
                .files
                .iter()
                .map(|file| PathText(file.0.as_ref().to_owned()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| {
            case.request
                .files
                .iter()
                .map(|file| PathText(file.0.as_ref().to_owned()))
                .collect()
        });
    let skipped = report.map_or(ReportedSkips::Unreported, |report| {
        ReportedSkips::Reported(
            report
                .skipped
                .iter()
                .map(|skip| SkippedPair {
                    rule: RuleName(skip.rule.as_ref().to_owned()),
                    path: PathText(skip.path.0.as_ref().to_owned()),
                })
                .collect::<Vec<_>>(),
        )
    });
    let source_identity = suite.case_source_digest(
        ArtifactPath(&case.request.root),
        ArtifactSet(&crate::domain::suite::service::materialized_sources(
            &case.request,
        )),
    );
    let metadata_identity = crate::domain::input_metadata::service::digest(&case.request.file_metadata);
    let project_identity = crate::domain::project_inputs::service::digest(
        match &case.request.project_search_roots {
            SearchRoots::Declared(roots) => Some(roots.as_slice()),
            SearchRoots::Undeclared => None,
        },
        &case.request.given_project_inventory,
    );
    let (metadata_sha256, metadata_error) = match metadata_identity {
        Ok(identity) => (identity.map(|value| value.0), None),
        Err(failure) => (None, Some(failure.to_string())),
    };
    let project_inputs_sha256 = project_identity.map(|value| value.0);
    let (source_sha256, source_failure) = match source_identity {
        Ok(identity) => (Some(identity.0), None),
        Err(failure) => (None, Some(failure.to_string())),
    };
    let source_error = source_failure.or_else(|| {
        metadata_error.map(|reason| format!("file metadata identity unavailable: {reason}"))
    });
    let source_text = source_sha256.clone();
    let recorded_error = match (error, source_error.as_deref()) {
        (Some(error), Some(source)) => Some(format!(
            "{}; source identity unavailable: {source}",
            error.0
        )),
        (Some(error), None) => Some(error.0.clone()),
        (None, Some(source)) => Some(format!("source identity unavailable: {source}")),
        (None, None) => None,
    };
    let context_manifest = match &case.request.context_manifest {
        ContextSeal::Sealed(manifest) => Some(ListedContext {
            mode: ContextMode::ListedDisk,
            logical_root: PathText(manifest.logical_root.to_string_lossy().into_owned()),
            actual_root: PathText(case.request.root.to_string_lossy().into_owned()),
            manifest_hash: Sha256Text(context.fingerprint(manifest).0),
        }),
        ContextSeal::Unsealed => None,
    };
    let intended_matches = report.map(|actual| {
        YesNo::from(
            crate::domain::discrepancy::service::matches_intended(case, actual)
                == crate::domain::discrepancy::types::IntentAgreement::Matches,
        )
    });
    let basis = match &case.reference_discrepancy {
        ryo_benchmark_contracts::domain::workloads::types::PreparedReferenceBehavior::Differs(item) => Some(item.basis),
        ryo_benchmark_contracts::domain::workloads::types::PreparedReferenceBehavior::Agrees => None,
    };
    let mut record = ObservationDocument {
        rule: RuleName(case.rule.as_ref().to_owned()),
        name: CaseName(case.name.to_string()),
        side,
        complete: YesNo::Yes,
        error: recorded_error.map_or(RecordedReason::Absent, |reason| {
            RecordedReason::Stated(ReasonText(reason))
        }),
        case_passed: YesNo::from(passed == YesNo::Yes && source_error.is_none()),
        files,
        source_sha256: source_sha256.map_or(SourceIdentity::Unavailable, |digest| {
            SourceIdentity::Sealed(Sha256Text(digest))
        }),
        file_metadata_sha256: FileMetadataDigest::from(metadata_sha256.map(Sha256Text)),
        project_inputs_sha256: ProjectInputsDigest::from(project_inputs_sha256.map(Sha256Text)),
        expected: findings(&case.expected.diagnostics),
        reference_expected: crate::domain::discrepancy::service::reference_expected(case)
            .map_or(ExpectedFindings::Underivable, |report| {
                ExpectedFindings::Derived(findings(&report.diagnostics))
            }),
        diagnostics: report.map_or(ReportedFindings::Unreported, |report| {
            ReportedFindings::Reported(findings(&report.diagnostics))
        }),
        skipped,
        observation_schema: SchemaName(crate::domain::suite::constants::BENCHMARK_CASE_V2.to_owned()),
        expectation_contract: ComparisonContract::Diagnostics,
        execution_metadata: report.map_or(RecordedMetadata::Unrecorded, |report| {
            RecordedMetadata::Recorded(report.execution_metadata.clone())
        }),
        coverage: report.map_or(CoverageAccounting::Unaccounted, |report| report.coverage),
        context_manifest: context_manifest.map_or(ContextRun::Isolated, ContextRun::Listed),
        project_context: match report.map(|report| &report.project_context) {
            Some(ProjectContext::Observed(receipt)) => context
                .receipt_text(receipt)
                .map_or(ProjectContextRecord::NotCarried, |text| {
                    ProjectContextRecord::Carried(CanonicalJson(text))
                }),
            _ => ProjectContextRecord::NotCarried,
        },
        rule_context: match &case.request.rule_context {
            RuleContextPreparation::Prepared(prepared) => RuleContextRecord::Declared(
                CanonicalJson(crate::outbound::worker::schemas::prepared_text(prepared)),
            ),
            RuleContextPreparation::Unconfigured => RuleContextRecord::Undeclared,
        },
        rule_observations: match &case.request.rule_observations {
            InputDeclaration::Declared(declarations) => RuleObservationsRecord::Declared(
                CanonicalJson(
                    crate::outbound::worker::schemas::observation_declarations_text(declarations),
                ),
            ),
            InputDeclaration::Undeclared => RuleObservationsRecord::Undeclared,
        },
        selected_rules: match &case.request.selected_rules {
            CohortSelection::Explicit(selected) => RuleCohort::Declared(
                selected.iter().map(|rule| RuleName(rule.to_string())).collect(),
            ),
            CohortSelection::Frozen => RuleCohort::Undeclared,
        },
        actual_request_root: RequestRoot::Written(PathText(
            case.request.root.to_string_lossy().into_owned(),
        )),
        documented_reference_discrepancy: YesNo::from(
            basis == Some(DiscrepancyBasis::LanguageDefect),
        ),
        documented_tool_contract_difference: ToolContractRecord::Recorded(YesNo::from(
            basis == Some(DiscrepancyBasis::ToolContract),
        )),
        discrepancy_validation: validation.map_or(ValidationRecord::Unattested, |proof| {
            ValidationRecord::Attested(validation_document(proof))
        }),
        raw_parity_matches_intended: intended_matches
            .map_or(IntentMatch::Unreported, IntentMatch::Matched),
        diagnostics_match_intended: intended_matches
            .map_or(IntentMatch::Unreported, IntentMatch::Matched),
        process_observation: ReferenceProcess::NotObserved,
    };
    let kept = match process {
        ProcessEvidence::Absent => None,
        ProcessEvidence::Kept(observed) => Some((
            bound(
                observed.invocation,
                source_text.clone().map_or(Pin::Absent, |digest| Pin::Sha256(Sha256Text(digest))),
                &record.file_metadata_sha256,
            ),
            observed.output,
        )),
    };
    if let Some((invocation, _)) = &kept {
        record.process_observation =
            ReferenceProcess::Observed(process_observation(case, invocation));
        record.raw_parity_matches_intended = IntentMatch::Matched(YesNo::No);
    }
    let key = crate::domain::process_output::service::observation_key(
        &ReceiptKeyParts {
            rule: Label::from(case.rule.as_ref()),
            name: Label::from(case.name.to_string().as_str()),
            side: Label::from(contestant.to_string().as_str()),
        },
    );
    let process = kept.as_ref().map(|(invocation, output)| {
        CapturedProcess {
            key: &key,
            invocation,
            stdout: &output.stdout,
            stderr: &output.stderr,
        }
    });
    receipts
        .record_case(ReceiptPath(Path::new(&path)), record, process)
        .map_err(|error| Error::Invalid(error.to_string()))?;
    if let Some(error) = source_error {
        return Err(Error::Invalid(format!(
            "source identity unavailable after raw capture: {error}"
        )));
    }
    Ok(())
}
