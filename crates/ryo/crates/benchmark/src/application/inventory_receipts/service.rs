//! Validate native case observations; expectation comparison stays in the existing verifier.
use super::errors::InventoryReceiptsError;
use super::types::{ReceiptCounts, Tally};
use crate::application::run::{inventory_execution, process_receipts};
use crate::domain::registered_inputs::{
    service as inputs,
    types::{ObservationDeclaration, RegisteredInputs},
};
use crate::outbound::inventory::types::case_document::{
    CaseContextIdentity, CaseDocument, EntryState, ExpectationContract,
    FileMetadata, FileObservation, InventoryCompleteness, ProjectInventory, ProjectScope,
};
use crate::outbound::suite::types::{
    finding::Finding,
    observation_document::{
        ContextMode, ContextRun, FileMetadataDigest, ObservationDocument, ProjectContextRecord,
        ProjectInputsDigest, ReferenceExpectation, SourceIdentity, ToolContractRecord,
    },
    text::{PathText, Sha256Text, Side, YesNo},
};
use ryo::linter::{
    GivenProjectInventory, ProjectEntryObservation, ProjectEntryState,
    ProjectInventoryCompleteness, RelativePath,
};
use ryo_benchmark_contracts::domain::reports::types::{ComparisonContract, ExecutionMetadata};
use ryo_benchmark_contracts::domain::workloads::types::DiscrepancyBasis;
use std::collections::HashSet;

type Failure = InventoryReceiptsError;

fn relative(path: &PathText) -> RelativePath {
    RelativePath(path.0.clone().into_boxed_str())
}

/// What a registered case declares about its files and project scope.
pub(crate) fn registered_inputs(case: &CaseDocument) -> RegisteredInputs {
    RegisteredInputs {
        files: case.files.iter().map(relative).collect(),
        observation: match case.file_observation {
            FileObservation::PosixExecutable => ObservationDeclaration::Required,
            FileObservation::Undeclared => ObservationDeclaration::Absent,
        },
        metadata: match &case.file_metadata {
            FileMetadata::Declared(values) => Some(
                values
                    .iter()
                    .map(|(path, executable)| (relative(path), *executable))
                    .collect(),
            ),
            FileMetadata::Undeclared => None,
        },
        search_roots: match &case.project_search_roots {
            ProjectScope::Roots(roots) => Some(roots.iter().map(relative).collect()),
            ProjectScope::Undeclared => None,
        },
        inventory: match &case.given_project_inventory {
            ProjectInventory::Declared(inventory) => GivenProjectInventory {
                entries: inventory
                    .entries
                    .iter()
                    .map(|entry| ProjectEntryObservation {
                        path: relative(&entry.path),
                        state: match entry.state {
                            EntryState::File => ProjectEntryState::File,
                            EntryState::Directory => ProjectEntryState::Directory,
                            EntryState::Other => ProjectEntryState::Other,
                            EntryState::Missing => ProjectEntryState::Missing,
                            EntryState::Unknown => ProjectEntryState::Unknown,
                        },
                    })
                    .collect(),
                completeness: match inventory.completeness {
                    InventoryCompleteness::Partial => ProjectInventoryCompleteness::Partial,
                    InventoryCompleteness::CompleteWithinDeclaredScope => {
                        ProjectInventoryCompleteness::CompleteWithinDeclaredScope
                    }
                },
            },
            ProjectInventory::Undeclared => GivenProjectInventory::default(),
        },
    }
}

/// The diagnostics a document derived for the reference, when it did.
fn derived(expected: &ReferenceExpectation) -> Option<&Vec<Finding>> {
    match expected {
        ReferenceExpectation::Derived(rows) => Some(rows),
        ReferenceExpectation::Underivable => None,
    }
}

/// The sorted diagnostics of one document, each with a rule, a file and an ordered span.
fn normalized(rows: Option<&Vec<Finding>>) -> Result<Vec<Finding>, Failure> {
    let mut rows = rows.ok_or(Failure::DiagnosticsMissing)?.clone();
    if rows
        .iter()
        .any(|row| row.rule.0.is_empty() || row.file.0.is_empty() || row.start > row.end)
    {
        return Err(Failure::DiagnosticSpanOrIdentityInvalid);
    }
    rows.sort();
    Ok(rows)
}

fn digest_text(
    digest: Option<
        ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest,
    >,
) -> Option<Sha256Text> {
    digest.map(|value| Sha256Text(value.0))
}

/// Require exact registered case identities and two actual complete, passing observations.
pub(crate) fn validate(
    cases: &[CaseDocument],
    observations: &[ObservationDocument],
) -> Result<ReceiptCounts, Failure> {
    validate_with_artifacts(cases, observations, &std::collections::BTreeMap::new())
}

/// The same validation with exact raw process bytes loaded by the artifact boundary.
pub(crate) fn validate_with_artifacts(
    cases: &[CaseDocument],
    observations: &[ObservationDocument],
    artifacts: &std::collections::BTreeMap<std::path::PathBuf, Vec<u8>>,
) -> Result<ReceiptCounts, Failure> {
    if cases.is_empty() || observations.len() != cases.len() * 2 {
        return Err(Failure::NativeCaseReceiptCount);
    }
    let mut registered = HashSet::with_capacity(cases.len());
    for case in cases {
        let (rule, name) = (&case.rule, &case.name);
        if rule.0.is_empty() {
            return Err(Failure::RegisteredRuleMissing);
        }
        if name.0.is_empty() {
            return Err(Failure::RegisteredCaseMissing);
        }
        if !registered.insert((rule, name)) {
            return Err(Failure::DuplicateRegisteredCase);
        }
    }
    let mut observed = HashSet::with_capacity(observations.len());
    for row in observations {
        let (rule, name, side) = (&row.rule, &row.name, row.side);
        if !registered.contains(&(rule, name)) {
            return Err(Failure::UnregisteredObservation);
        }
        if !observed.insert((rule, name, side)) {
            return Err(Failure::DuplicateObservation);
        }
    }
    let mut raw_exact = 0;
    let mut documented = 0;
    let mut contract_only = 0;
    for case in cases {
        let declared_inputs = registered_inputs(case);
        let metadata = digest_text(inputs::metadata_digest(&declared_inputs)?);
        let project_inputs = digest_text(inputs::project_inputs_digest(&declared_inputs)?);
        let metadata_digest = FileMetadataDigest::from(metadata.clone());
        let inputs_digest = ProjectInputsDigest::from(project_inputs);
        if case.file_metadata_sha256 != metadata {
            return Err(Failure::FileMetadataDigestDiffers);
        }
        if case.project_inputs_sha256 != inputs_digest {
            return Err(Failure::ProjectInputsDigestDiffers);
        }
        let declared = match case.expectation_contract {
            ExpectationContract::Declared(contract) => contract,
            ExpectationContract::Unwritten => ComparisonContract::Diagnostics,
        };
        let expected = normalized(Some(&case.expected))?;
        let certificate = case.reference_discrepancy.as_ref();
        let basis = certificate.map(|certificate| certificate.basis);
        let process = certificate.is_some_and(|certificate| certificate.process.is_some());
        if process && declared != ComparisonContract::Diagnostics {
            return Err(Failure::ProcessExpectationNotComplete);
        }
        let reference_expected = match certificate {
            None => expected.clone(),
            Some(certificate) => {
                if normalized(Some(&certificate.intended_expected))? != expected {
                    return Err(Failure::DiscrepancyIntentionDiffers);
                }
                if process {
                    Vec::new()
                } else {
                    normalized(derived(&certificate.reference_expected))?
                }
            }
        };
        let mut sides = Vec::new();
        for side in [Side::Reference, Side::Candidate] {
            let row = observations
                .iter()
                .find(|row| row.rule == case.rule && row.name == case.name && row.side == side)
                .ok_or(Failure::SideReceiptCount)?;
            if row.rule_context != case.rule_context {
                return Err(Failure::RuleContextDiffers);
            }
            if row.rule_observations != case.rule_observations {
                return Err(Failure::ObservationDeclarationsDiffer);
            }
            if row.selected_rules != case.selected_rules {
                return Err(Failure::SelectedCohortDiffers);
            }
            if row.observation_schema.0 != "benchmark-case/v2"
                || row.expectation_contract != declared
            {
                return Err(Failure::ExpectationContractDiffers);
            }
            if row.complete != YesNo::Yes
                || row.error.is_some()
                || row.case_passed != YesNo::Yes
                || row.source_sha256 != SourceIdentity::Sealed(case.source_sha256.clone())
                || row.file_metadata_sha256 != metadata_digest
                || row.project_inputs_sha256 != inputs_digest
                || row.files != case.files
                || normalized(Some(&row.expected))? != expected
                || if process {
                    matches!(row.reference_expected, ReferenceExpectation::Derived(_))
                } else {
                    normalized(derived(&row.reference_expected))? != reference_expected
                }
            {
                return Err(Failure::ObservationStale);
            }
            if (row.documented_reference_discrepancy == YesNo::Yes)
                != (basis == Some(DiscrepancyBasis::LanguageDefect))
                || (row.documented_tool_contract_difference == ToolContractRecord::Recorded(YesNo::Yes))
                    != (basis == Some(DiscrepancyBasis::ToolContract))
            {
                return Err(Failure::DiscrepancyAuthorityDiffers);
            }
            if process && side == Side::Reference {
                process_receipts::validate(case, row, artifacts)?;
            } else {
                if row.process_observation.is_some() {
                    return Err(Failure::UnexpectedProcessObservation);
                }
                let actual = normalized(row.diagnostics.as_ref())?;
                let required = if side == Side::Reference {
                    &reference_expected
                } else {
                    &expected
                };
                if actual != *required || row.skipped.is_none() {
                    return Err(Failure::DiagnosticsDiffer);
                }
            }
            if side == Side::Candidate {
                let accounted = row.coverage.is_some_and(|coverage| {
                    coverage.selected != 0
                        && coverage.evaluated.checked_add(coverage.skipped)
                            == Some(coverage.selected)
                        && coverage.unknown_or_unread == 0
                });
                if row.expectation_contract != ComparisonContract::Diagnostics
                    || row.execution_metadata != Some(ExecutionMetadata::Complete)
                    || !accounted
                {
                    return Err(Failure::CandidateNotAccounted);
                }
            }
            if case.project_context_mode == ContextMode::Isolated {
                if row.context_manifest != ContextRun::Isolated
                    || row.project_context != ProjectContextRecord::NotCarried
                {
                    return Err(Failure::UnexpectedProjectContext);
                }
            } else {
                let manifest = match &row.context_manifest {
                    ContextRun::Listed(value) => {
                        Some((value.mode, &value.logical_root, &value.manifest_hash))
                    }
                    ContextRun::Isolated => None,
                };
                let identity = match &case.context_identity {
                    CaseContextIdentity::Listed(value) => {
                        Some((value.mode, &value.logical_root, &value.manifest_hash))
                    }
                    CaseContextIdentity::Isolated => None,
                };
                if manifest != identity {
                    return Err(Failure::StaleProjectContext);
                }
            }
            sides.push(row);
        }
        if let Some(certificate) = certificate {
            let proof = sides[0].discrepancy_validation.as_ref();
            if proof.and_then(|proof| proof.file_metadata_sha256.as_ref()) != metadata.as_ref()
                || certificate.identity.file_metadata != metadata
            {
                return Err(Failure::DiscrepancyMetadataDiffers);
            }
            let Some(proof) =
                proof.filter(|proof| sides[1].discrepancy_validation.as_ref() == Some(*proof))
            else {
                return Err(Failure::DiscrepancyValidationMissing);
            };
            if proof.discrepancy_basis != certificate.basis {
                return Err(Failure::DiscrepancyAuthorityUnverified);
            }
            let identity = &certificate.identity;
            for (actual, expected) in [
                (&proof.settings_sha256, &identity.settings),
                (&proof.profile_sha256, &identity.profile),
                (&proof.source_sha256, &identity.source),
                (&proof.reference_output_sha256, &identity.reference_output),
                (&proof.intended_output_sha256, &identity.intended_output),
            ] {
                if actual.0.is_empty() || actual != expected {
                    return Err(Failure::StaleDiscrepancyEvidence);
                }
            }
            let tool = &proof.actual_executable_sha256;
            if tool.0.is_empty() {
                return Err(Failure::DiscrepancyToolMissing);
            }
            if !certificate.approved_executable_sha256.contains(tool) {
                return Err(Failure::DiscrepancyToolUnpinned);
            }
        }
        inventory_execution::compare(sides[0], sides[1], declared)?;
        let exact = !process
            && normalized(sides[0].diagnostics.as_ref())?
                == normalized(sides[1].diagnostics.as_ref())?
            && sides[0].skipped == sides[1].skipped;
        raw_exact += usize::from(exact);
        documented += usize::from(!exact && certificate.is_some());
        contract_only += usize::from(!exact && certificate.is_none());
    }
    Ok(ReceiptCounts {
        raw_exact: Tally(raw_exact),
        documented: Tally(documented),
        contract_only: Tally(contract_only),
    })
}
