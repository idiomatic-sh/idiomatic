//! Validate native case observations; expectation comparison stays in the existing verifier.
use super::constants;
use super::errors::InventoryReceiptsError;
use super::types::{DiagnosticRow, ReceiptCounts, Tally};
use crate::application::run::{inventory_execution, process_receipts};
use crate::domain::registered_inputs::service as inputs;
use std::collections::HashSet;

use serde_json::Value;

fn normalized(value: &Value) -> Result<Vec<DiagnosticRow>, InventoryReceiptsError> {
    let source = value
        .as_array()
        .ok_or(InventoryReceiptsError::DiagnosticsNotArray)?;
    let mut rows = Vec::with_capacity(source.len());
    for value in source {
        let rule = value[constants::RULE]
            .as_str()
            .ok_or(InventoryReceiptsError::DiagnosticRuleMissing)?;
        let file = value[constants::FILE]
            .as_str()
            .ok_or(InventoryReceiptsError::DiagnosticFileMissing)?;
        let start = value[constants::START]
            .as_u64()
            .ok_or(InventoryReceiptsError::DiagnosticStartInvalid)?;
        let end = value[constants::END]
            .as_u64()
            .ok_or(InventoryReceiptsError::DiagnosticEndInvalid)?;
        if rule.is_empty() || file.is_empty() || start > end {
            return Err(InventoryReceiptsError::DiagnosticSpanOrIdentityInvalid);
        }
        rows.push(DiagnosticRow(value.to_string()));
    }
    rows.sort();
    Ok(rows)
}
/// Require exact registered case identities and two actual complete, passing observations.
pub(crate) fn validate(
    cases: &[Value],
    observations: &[Value],
) -> Result<ReceiptCounts, InventoryReceiptsError> {
    validate_with_artifacts(cases, observations, &std::collections::BTreeMap::new())
}
/// The same validation with exact raw process bytes loaded by the artifact boundary.
pub(crate) fn validate_with_artifacts(
    cases: &[Value],
    observations: &[Value],
    artifacts: &std::collections::BTreeMap<std::path::PathBuf, Vec<u8>>,
) -> Result<ReceiptCounts, InventoryReceiptsError> {
    if cases.is_empty() || observations.len() != cases.len() * 2 {
        return Err(InventoryReceiptsError::NativeCaseReceiptCount);
    }
    let mut registered = HashSet::with_capacity(cases.len());
    for case in cases {
        let rule = case[constants::RULE]
            .as_str()
            .filter(|value| !value.is_empty())
            .ok_or(InventoryReceiptsError::RegisteredRuleMissing)?;
        let name = case[constants::NAME]
            .as_str()
            .filter(|value| !value.is_empty())
            .ok_or(InventoryReceiptsError::RegisteredCaseMissing)?;
        if !registered.insert((rule, name)) {
            return Err(InventoryReceiptsError::DuplicateRegisteredCase);
        }
    }
    let mut observed = HashSet::with_capacity(observations.len());
    for row in observations {
        let rule = row[constants::RULE]
            .as_str()
            .ok_or(InventoryReceiptsError::NativeRuleMissing)?;
        let name = row[constants::NAME]
            .as_str()
            .ok_or(InventoryReceiptsError::NativeCaseMissing)?;
        let side = row[constants::SIDE]
            .as_str()
            .ok_or(InventoryReceiptsError::NativeSideMissing)?;
        if !registered.contains(&(rule, name))
            || !matches!(side, constants::REFERENCE | constants::CANDIDATE)
        {
            return Err(InventoryReceiptsError::UnregisteredObservation);
        }
        if !observed.insert((rule, name, side)) {
            return Err(InventoryReceiptsError::DuplicateObservation);
        }
    }
    let mut raw_exact = 0;
    let mut documented = 0;
    let mut contract_only = 0;
    for case in cases {
        let metadata = inputs::metadata_digest(case)?;
        let project_inputs = inputs::project_inputs_digest(case)?;
        if case[constants::FILE_METADATA_SHA256] != metadata {
            return Err(InventoryReceiptsError::FileMetadataDigestDiffers);
        }
        if case[constants::PROJECT_INPUTS_SHA256] != project_inputs {
            return Err(InventoryReceiptsError::ProjectInputsDigestDiffers);
        }
        let declared = if case[constants::EXPECTATION_CONTRACT].is_null() {
            ryo_benchmark_contracts::domain::reports::types::ComparisonContract::Diagnostics
        } else {
            inventory_execution::contract(&case[constants::EXPECTATION_CONTRACT])?
        };
        let expected = normalized(&case[constants::EXPECTED])?;
        let certificate = &case[constants::REFERENCE_DISCREPANCY];
        let basis = if certificate.is_null() {
            constants::NONE
        } else {
            certificate[constants::BASIS]
                .as_str()
                .unwrap_or(constants::LANGUAGE_DEFECT)
        };
        if !matches!(
            basis,
            constants::NONE | constants::LANGUAGE_DEFECT | constants::TOOL_CONTRACT_DIFFERENCE
        ) {
            return Err(InventoryReceiptsError::UnknownDiscrepancyAuthority);
        }
        let process = !certificate[constants::PROCESS].is_null();
        if process
            && declared
                != ryo_benchmark_contracts::domain::reports::types::ComparisonContract::Diagnostics
        {
            return Err(InventoryReceiptsError::ProcessExpectationNotComplete);
        }
        let reference_expected = if certificate.is_null() {
            expected.clone()
        } else {
            if normalized(&certificate[constants::INTENDED_EXPECTED])? != expected {
                return Err(InventoryReceiptsError::DiscrepancyIntentionDiffers);
            }
            if process {
                Vec::new()
            } else {
                normalized(&certificate[constants::REFERENCE_EXPECTED])?
            }
        };
        let mut sides = Vec::new();
        for side in [constants::REFERENCE, constants::CANDIDATE] {
            let found = observations
                .iter()
                .filter(|row| {
                    row[constants::RULE] == case[constants::RULE]
                        && row[constants::NAME] == case[constants::NAME]
                        && row[constants::SIDE] == side
                })
                .collect::<Vec<_>>();
            if found.len() != 1 {
                return Err(InventoryReceiptsError::SideReceiptCount);
            }
            let row = found[0];
            if row[constants::RULE_CONTEXT] != case[constants::RULE_CONTEXT] {
                return Err(InventoryReceiptsError::RuleContextDiffers);
            }
            if row[constants::RULE_OBSERVATIONS] != case[constants::RULE_OBSERVATIONS] {
                return Err(InventoryReceiptsError::ObservationDeclarationsDiffer);
            }
            if row[constants::SELECTED_RULES] != case[constants::SELECTED_RULES] {
                return Err(InventoryReceiptsError::SelectedCohortDiffers);
            }
            if row[constants::OBSERVATION_SCHEMA] != constants::BENCHMARK_CASE_V2
                || inventory_execution::contract(&row[constants::EXPECTATION_CONTRACT])? != declared
            {
                return Err(InventoryReceiptsError::ExpectationContractDiffers);
            }
            if row[constants::COMPLETE] != true
                || !row[constants::ERROR].is_null()
                || row[constants::CASE_PASSED] != true
                || row[constants::SOURCE_SHA256] != case[constants::SOURCE_SHA256]
                || row[constants::FILE_METADATA_SHA256] != metadata
                || row[constants::PROJECT_INPUTS_SHA256] != project_inputs
                || row[constants::FILES] != case[constants::FILES]
                || normalized(&row[constants::EXPECTED])? != expected
                || if process {
                    !row[constants::REFERENCE_EXPECTED].is_null()
                } else {
                    normalized(&row[constants::REFERENCE_EXPECTED])? != reference_expected
                }
            {
                return Err(InventoryReceiptsError::ObservationStale);
            }
            if row[constants::DOCUMENTED_REFERENCE_DISCREPANCY]
                .as_bool()
                .unwrap_or(false)
                != (basis == constants::LANGUAGE_DEFECT)
                || row[constants::DOCUMENTED_TOOL_CONTRACT_DIFFERENCE]
                    .as_bool()
                    .unwrap_or(false)
                    != (basis == constants::TOOL_CONTRACT_DIFFERENCE)
            {
                return Err(InventoryReceiptsError::DiscrepancyAuthorityDiffers);
            }
            if process && side == constants::REFERENCE {
                process_receipts::validate(case, row, artifacts)?;
            } else {
                if !row[constants::PROCESS_OBSERVATION].is_null() {
                    return Err(InventoryReceiptsError::UnexpectedProcessObservation);
                }
                let actual = normalized(&row[constants::DIAGNOSTICS])?;
                let required = if side == constants::REFERENCE {
                    &reference_expected
                } else {
                    &expected
                };
                if actual.as_slice() != required.as_slice() || !row[constants::SKIPPED].is_array() {
                    return Err(InventoryReceiptsError::DiagnosticsDiffer);
                }
            }
            if side == constants::CANDIDATE {
                let coverage = &row[constants::COVERAGE];
                let selected = coverage[constants::SELECTED_RULE_FILE_PAIRS]
                    .as_u64()
                    .unwrap_or(0);
                let evaluated = coverage[constants::EVALUATED_RULE_FILE_PAIRS]
                    .as_u64()
                    .unwrap_or(u64::MAX);
                let skipped = coverage[constants::SKIPPED_RULE_FILE_PAIRS]
                    .as_u64()
                    .unwrap_or(u64::MAX);
                if row[constants::OBSERVATION_SCHEMA] != constants::BENCHMARK_CASE_V2
                    || row[constants::EXPECTATION_CONTRACT] != constants::DIAGNOSTICS
                    || row[constants::EXECUTION_METADATA][constants::AVAILABILITY]
                        != constants::COMPLETE
                    || selected == 0
                    || evaluated.checked_add(skipped) != Some(selected)
                    || coverage[constants::UNKNOWN_OR_UNREAD_RULE_FILE_PAIRS] != 0
                {
                    return Err(InventoryReceiptsError::CandidateNotAccounted);
                }
            }
            if case[constants::PROJECT_CONTEXT_MODE] == constants::ISOLATED {
                if !row[constants::CONTEXT_MANIFEST].is_null()
                    || !row[constants::PROJECT_CONTEXT].is_null()
                {
                    return Err(InventoryReceiptsError::UnexpectedProjectContext);
                }
            } else {
                for key in [
                    constants::MODE,
                    constants::LOGICAL_ROOT,
                    constants::MANIFEST_HASH,
                ] {
                    if row[constants::CONTEXT_MANIFEST][key]
                        != case[constants::CONTEXT_IDENTITY][key]
                    {
                        return Err(InventoryReceiptsError::StaleProjectContext);
                    }
                }
            }
            sides.push(row);
        }
        if !certificate.is_null() {
            let proof = &sides[0][constants::DISCREPANCY_VALIDATION];
            if proof[constants::FILE_METADATA_SHA256] != metadata
                || certificate[constants::IDENTITY][constants::FILE_METADATA_SHA256_2] != metadata
            {
                return Err(InventoryReceiptsError::DiscrepancyMetadataDiffers);
            }
            if proof.is_null() || proof != &sides[1][constants::DISCREPANCY_VALIDATION] {
                return Err(InventoryReceiptsError::DiscrepancyValidationMissing);
            }
            if proof[constants::DISCREPANCY_BASIS]
                .as_str()
                .unwrap_or(constants::LANGUAGE_DEFECT)
                != basis
            {
                return Err(InventoryReceiptsError::DiscrepancyAuthorityUnverified);
            }
            for (actual, expected) in [
                (constants::SETTINGS_SHA256, constants::SETTINGS_SHA256_2),
                (constants::PROFILE_SHA256, constants::PROFILE_SHA256_2),
                (constants::SOURCE_SHA256, constants::SOURCE_SHA256_2),
                (
                    constants::REFERENCE_OUTPUT_SHA256,
                    constants::REFERENCE_OUTPUT_SHA256_2,
                ),
                (
                    constants::INTENDED_OUTPUT_SHA256,
                    constants::INTENDED_OUTPUT_SHA256_2,
                ),
            ] {
                if !proof[actual]
                    .as_str()
                    .is_some_and(|value| !value.is_empty())
                    || !certificate[constants::IDENTITY][expected]
                        .as_str()
                        .is_some_and(|value| !value.is_empty())
                    || proof[actual] != certificate[constants::IDENTITY][expected]
                {
                    return Err(InventoryReceiptsError::StaleDiscrepancyEvidence);
                }
            }
            if !proof[constants::ACTUAL_EXECUTABLE_SHA256]
                .as_str()
                .is_some_and(|value| !value.is_empty())
            {
                return Err(InventoryReceiptsError::DiscrepancyToolMissing);
            }
            if !certificate[constants::APPROVED_EXECUTABLE_SHA256]
                .as_array()
                .is_some_and(|pins| pins.contains(&proof[constants::ACTUAL_EXECUTABLE_SHA256]))
            {
                return Err(InventoryReceiptsError::DiscrepancyToolUnpinned);
            }
        }
        inventory_execution::compare(sides[0], sides[1], declared)?;
        let exact = !process
            && normalized(&sides[0][constants::DIAGNOSTICS])?
                == normalized(&sides[1][constants::DIAGNOSTICS])?
            && sides[0][constants::SKIPPED] == sides[1][constants::SKIPPED];
        raw_exact += usize::from(exact);
        documented += usize::from(!exact && !certificate.is_null());
        contract_only += usize::from(!exact && certificate.is_null());
    }
    Ok(ReceiptCounts {
        raw_exact: Tally(raw_exact),
        documented: Tally(documented),
        contract_only: Tally(contract_only),
    })
}
