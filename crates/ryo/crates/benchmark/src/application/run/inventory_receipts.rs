//! Validate native case observations; expectation comparison stays in the existing verifier.
use ryo_benchmark_contracts::domain::artifacts::types::{FileExecutable, FileMetadataEntry};
use std::collections::HashSet;

use serde_json::Value;

fn normalized(value: &Value) -> Result<Vec<String>, String> {
    let source = value.as_array().ok_or(crate::application::run::constants::DIAGNOSTICS_MUST_BE_AN_ARRAY)?;
    let mut rows = Vec::with_capacity(source.len());
    for value in source {
        let rule = value[crate::application::run::constants::RULE].as_str().ok_or(crate::application::run::constants::DIAGNOSTIC_RULE_IS_MISSING)?;
        let file = value[crate::application::run::constants::FILE].as_str().ok_or(crate::application::run::constants::DIAGNOSTIC_FILE_IS_MISSING)?;
        let start = value[crate::application::run::constants::START]
            .as_u64()
            .ok_or(crate::application::run::constants::DIAGNOSTIC_START_IS_INVALID)?;
        let end = value[crate::application::run::constants::END].as_u64().ok_or(crate::application::run::constants::DIAGNOSTIC_END_IS_INVALID)?;
        if rule.is_empty() || file.is_empty() || start > end {
            return Err(crate::application::run::constants::DIAGNOSTIC_SPAN_OR_IDENTITY_IS_INVALID.into());
        }
        rows.push(value.to_string());
    }
    rows.sort();
    Ok(rows)
}
/// Decode only declared metadata and recompute its pure framed identity.
pub(super) fn metadata_digest(case: &Value) -> Result<Value, String> {
    let files = case[crate::application::run::constants::FILES]
        .as_array()
        .ok_or(crate::application::run::constants::REGISTERED_FILES_ARE_MISSING)?;
    let required = !case[crate::application::run::constants::FILE_OBSERVATION].is_null();
    if required && case[crate::application::run::constants::FILE_OBSERVATION] != crate::application::run::constants::POSIX_EXECUTABLE {
        return Err(crate::application::run::constants::UNSUPPORTED_REGISTERED_FILE_OBSERVATION.into());
    }
    let metadata = if case[crate::application::run::constants::FILE_METADATA].is_null() {
        None
    } else {
        let values = case[crate::application::run::constants::FILE_METADATA]
            .as_object()
            .ok_or(crate::application::run::constants::FILE_METADATA_MUST_BE_AN_OBJECT)?;
        let mut parsed = std::collections::BTreeMap::new();
        for (path, value) in values {
            ryo_benchmark_contracts::domain::artifacts::types::relative_artifact_path::value::RelativeArtifactPath::try_from(std::path::Path::new(std::path::Path::new(path)))
                .map_err(|error| error.to_string())?;
            if !files.iter().any(|file| file.as_str() == Some(path)) {
                return Err(crate::application::run::constants::METADATA_NAMES_AN_UNDECLARED_SOURCE.into());
            }
            parsed.insert(
                ryo::linter::RelativePath(path.clone().into_boxed_str()),
                value
                    .as_bool()
                    .ok_or(crate::application::run::constants::EXECUTABLE_OBSERVATION_MUST_BE_BOOLEAN)?,
            );
        }
        Some(parsed)
    };
    if required
        && metadata
            .as_ref()
            .is_none_or(|values| values.len() != files.len())
    {
        return Err(crate::application::run::constants::REQUIRED_INPUT_OBSERVATIONS_ARE_MISSING.into());
    }
    Ok(serde_json::json!(
        ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest::file_metadata((metadata.as_ref()).map(|values| values.iter().map(|(path, executable)| FileMetadataEntry { path, executable: if *executable { FileExecutable::Set } else { FileExecutable::Unset } })))
            .map_err(|error| error.to_string())?
            .map(|value| value.0)
    ))
}

/// Recompute the project input identity from the registered explicit roots and entry inventory.
fn project_inputs_digest(case: &Value) -> Result<Value, String> {
    let roots = if case[crate::application::run::constants::PROJECT_SEARCH_ROOTS].is_null() {
        None
    } else {
        Some(
            case[crate::application::run::constants::PROJECT_SEARCH_ROOTS]
                .as_array()
                .ok_or(crate::application::run::constants::PROJECT_SEARCH_ROOTS_MUST_BE_AN)?
                .iter()
                .map(|value| {
                    let root = value.as_str().ok_or(crate::application::run::constants::PROJECT_SEARCH_ROOT_IS_NOT_TEXT)?;
                    if !root.is_empty() {
                        ryo_benchmark_contracts::domain::artifacts::types::relative_artifact_path::value::RelativeArtifactPath::try_from(std::path::Path::new(root))
                            .map_err(|error| error.to_string())?;
                    }
                    Ok(ryo::linter::RelativePath(root.to_owned().into_boxed_str()))
                })
                .collect::<Result<Vec<_>, String>>()?,
        )
    };
    let inventory = if case[crate::application::run::constants::GIVEN_PROJECT_INVENTORY].is_null() {
        ryo::linter::GivenProjectInventory::default()
    } else {
        let wire: crate::outbound::context::adapters::local::schemas::ProjectInventorySchema =
            serde_json::from_value(case[crate::application::run::constants::GIVEN_PROJECT_INVENTORY].clone())
                .map_err(|error| error.to_string())?;
        wire.into()
    };
    for entry in &inventory.entries {
        ryo_benchmark_contracts::domain::artifacts::types::relative_artifact_path::value::RelativeArtifactPath::try_from(std::path::Path::new(entry.path.0.as_ref()))
            .map_err(|error| error.to_string())?;
    }
    Ok(serde_json::json!(
        crate::domain::suite::project_inputs::digest(roots.as_deref(), &inventory)
            .map_err(|error| error.to_string())?
            .map(|digest| digest.0)
    ))
}

/// Require exact registered case identities and two actual complete, passing observations.
pub(super) fn validate(
    cases: &[Value],
    observations: &[Value],
) -> Result<(usize, usize, usize), String> {
    validate_with_artifacts(cases, observations, &std::collections::BTreeMap::new())
}
/// The same validation with exact raw process bytes loaded by the artifact boundary.
pub(super) fn validate_with_artifacts(
    cases: &[Value],
    observations: &[Value],
    artifacts: &std::collections::BTreeMap<std::path::PathBuf, Vec<u8>>,
) -> Result<(usize, usize, usize), String> {
    if cases.is_empty() || observations.len() != cases.len() * 2 {
        return Err(crate::application::run::constants::MISSING_OR_EXTRA_NATIVE_CASE_RECEIPTS.into());
    }
    let mut registered = HashSet::with_capacity(cases.len());
    for case in cases {
        let rule = case[crate::application::run::constants::RULE]
            .as_str()
            .filter(|value| !value.is_empty())
            .ok_or(crate::application::run::constants::REGISTERED_RULE_IDENTITY_IS_MISSING)?;
        let name = case[crate::application::run::constants::NAME]
            .as_str()
            .filter(|value| !value.is_empty())
            .ok_or(crate::application::run::constants::REGISTERED_CASE_IDENTITY_IS_MISSING)?;
        if !registered.insert((rule, name)) {
            return Err(crate::application::run::constants::DUPLICATE_REGISTERED_CASE_IDENTITY.into());
        }
    }
    let mut observed = HashSet::with_capacity(observations.len());
    for row in observations {
        let rule = row[crate::application::run::constants::RULE]
            .as_str()
            .ok_or(crate::application::run::constants::NATIVE_RULE_IDENTITY_IS_MISSING)?;
        let name = row[crate::application::run::constants::NAME]
            .as_str()
            .ok_or(crate::application::run::constants::NATIVE_CASE_IDENTITY_IS_MISSING)?;
        let side = row[crate::application::run::constants::SIDE]
            .as_str()
            .ok_or(crate::application::run::constants::NATIVE_SIDE_IDENTITY_IS_MISSING)?;
        if !registered.contains(&(rule, name)) || !matches!(side, crate::application::run::constants::REFERENCE | crate::application::run::constants::CANDIDATE) {
            return Err(crate::application::run::constants::UNREGISTERED_NATIVE_CASE_OBSERVATION.into());
        }
        if !observed.insert((rule, name, side)) {
            return Err(crate::application::run::constants::DUPLICATE_NATIVE_CASE_OBSERVATION.into());
        }
    }
    let mut raw_exact = 0;
    let mut documented = 0;
    let mut contract_only = 0;
    for case in cases {
        let metadata = metadata_digest(case)?;
        let project_inputs = project_inputs_digest(case)?;
        if case[crate::application::run::constants::FILE_METADATA_SHA256] != metadata {
            return Err(crate::application::run::constants::REGISTERED_FILE_METADATA_DIGEST_DIFFERS.into());
        }
        if case[crate::application::run::constants::PROJECT_INPUTS_SHA256] != project_inputs {
            return Err(crate::application::run::constants::REGISTERED_PROJECT_INPUTS_DIGEST_DIFFERS.into());
        }
        let declared = if case[crate::application::run::constants::EXPECTATION_CONTRACT].is_null() {
            ryo_benchmark_contracts::domain::reports::types::ComparisonContract::Diagnostics
        } else {
            super::inventory_execution::contract(&case[crate::application::run::constants::EXPECTATION_CONTRACT])?
        };
        let expected = normalized(&case[crate::application::run::constants::EXPECTED])?;
        let certificate = &case[crate::application::run::constants::REFERENCE_DISCREPANCY];
        let basis = if certificate.is_null() {
            crate::application::run::constants::NONE
        } else {
            certificate[crate::application::run::constants::BASIS].as_str().unwrap_or(crate::application::run::constants::LANGUAGE_DEFECT)
        };
        if !matches!(
            basis,
            crate::application::run::constants::NONE | crate::application::run::constants::LANGUAGE_DEFECT | crate::application::run::constants::TOOL_CONTRACT_DIFFERENCE
        ) {
            return Err(crate::application::run::constants::UNKNOWN_DISCREPANCY_AUTHORITY.into());
        }
        let process = !certificate[crate::application::run::constants::PROCESS].is_null();
        if process
            && declared
                != ryo_benchmark_contracts::domain::reports::types::ComparisonContract::Diagnostics
        {
            return Err(crate::application::run::constants::PROCESS_EXPECTATION_CANNOT_SATISFY_COMPLETE.into());
        }
        let reference_expected = if certificate.is_null() {
            expected.clone()
        } else {
            if normalized(&certificate[crate::application::run::constants::INTENDED_EXPECTED])? != expected {
                return Err(crate::application::run::constants::DISCREPANCY_INTENTION_DIFFERS_FROM_REGISTERE.into());
            }
            if process {
                Vec::new()
            } else {
                normalized(&certificate[crate::application::run::constants::REFERENCE_EXPECTED])?
            }
        };
        let mut sides = Vec::new();
        for side in [crate::application::run::constants::REFERENCE, crate::application::run::constants::CANDIDATE] {
            let found = observations
                .iter()
                .filter(|row| {
                    row[crate::application::run::constants::RULE] == case[crate::application::run::constants::RULE]
                        && row[crate::application::run::constants::NAME] == case[crate::application::run::constants::NAME]
                        && row[crate::application::run::constants::SIDE] == side
                })
                .collect::<Vec<_>>();
            if found.len() != 1 {
                return Err(crate::application::run::constants::MISSING_OR_DUPLICATE_NATIVE_SIDE_RECEIPT.into());
            }
            let row = found[0];
            if row[crate::application::run::constants::RULE_CONTEXT] != case[crate::application::run::constants::RULE_CONTEXT] {
                return Err(crate::application::run::constants::NATIVE_CONFIGURED_RULE_CONTEXT_DIFFERS_FROM.into());
            }
            if row[crate::application::run::constants::RULE_OBSERVATIONS] != case[crate::application::run::constants::RULE_OBSERVATIONS] {
                return Err(crate::application::run::constants::NATIVE_OBSERVATION_DECLARATIONS_DIFFER_FROM.into());
            }
            if row[crate::application::run::constants::SELECTED_RULES] != case[crate::application::run::constants::SELECTED_RULES] {
                return Err(crate::application::run::constants::NATIVE_SELECTED_RULE_COHORT_DIFFERS_FROM.into());
            }
            if row[crate::application::run::constants::OBSERVATION_SCHEMA] != crate::application::run::constants::BENCHMARK_CASE_V2
                || super::inventory_execution::contract(&row[crate::application::run::constants::EXPECTATION_CONTRACT])? != declared
            {
                return Err(crate::application::run::constants::NATIVE_EXPECTATION_CONTRACT_DIFFERS_FROM_REG.into());
            }
            if row[crate::application::run::constants::COMPLETE] != true
                || !row[crate::application::run::constants::ERROR].is_null()
                || row[crate::application::run::constants::CASE_PASSED] != true
                || row[crate::application::run::constants::SOURCE_SHA256] != case[crate::application::run::constants::SOURCE_SHA256]
                || row[crate::application::run::constants::FILE_METADATA_SHA256] != metadata
                || row[crate::application::run::constants::PROJECT_INPUTS_SHA256] != project_inputs
                || row[crate::application::run::constants::FILES] != case[crate::application::run::constants::FILES]
                || normalized(&row[crate::application::run::constants::EXPECTED])? != expected
                || if process {
                    !row[crate::application::run::constants::REFERENCE_EXPECTED].is_null()
                } else {
                    normalized(&row[crate::application::run::constants::REFERENCE_EXPECTED])? != reference_expected
                }
            {
                return Err(crate::application::run::constants::FAILED_INCOMPLETE_OR_STALE_NATIVE_OBSERVATIO.into());
            }
            if row[crate::application::run::constants::DOCUMENTED_REFERENCE_DISCREPANCY]
                .as_bool()
                .unwrap_or(false)
                != (basis == crate::application::run::constants::LANGUAGE_DEFECT)
                || row[crate::application::run::constants::DOCUMENTED_TOOL_CONTRACT_DIFFERENCE]
                    .as_bool()
                    .unwrap_or(false)
                    != (basis == crate::application::run::constants::TOOL_CONTRACT_DIFFERENCE)
            {
                return Err(crate::application::run::constants::NATIVE_DISCREPANCY_AUTHORITY_DIFFERS_FROM_RE.into());
            }
            if process && side == crate::application::run::constants::REFERENCE {
                super::process_receipts::validate(case, row, artifacts)?;
            } else {
                if !row[crate::application::run::constants::PROCESS_OBSERVATION].is_null() {
                    return Err(crate::application::run::constants::DIAGNOSTIC_REPORT_UNEXPECTEDLY_CONTAINS_A_PR.into());
                }
                let actual = normalized(&row[crate::application::run::constants::DIAGNOSTICS])?;
                let required = if side == crate::application::run::constants::REFERENCE {
                    &reference_expected
                } else {
                    &expected
                };
                if actual.as_slice() != required.as_slice() || !row[crate::application::run::constants::SKIPPED].is_array() {
                    return Err(crate::application::run::constants::NATIVE_DIAGNOSTICS_DIFFER_FROM_REGISTERED_EX.into());
                }
            }
            if side == crate::application::run::constants::CANDIDATE {
                let coverage = &row[crate::application::run::constants::COVERAGE];
                let selected = coverage[crate::application::run::constants::SELECTED_RULE_FILE_PAIRS].as_u64().unwrap_or(0);
                let evaluated = coverage[crate::application::run::constants::EVALUATED_RULE_FILE_PAIRS]
                    .as_u64()
                    .unwrap_or(u64::MAX);
                let skipped = coverage[crate::application::run::constants::SKIPPED_RULE_FILE_PAIRS]
                    .as_u64()
                    .unwrap_or(u64::MAX);
                if row[crate::application::run::constants::OBSERVATION_SCHEMA] != crate::application::run::constants::BENCHMARK_CASE_V2
                    || row[crate::application::run::constants::EXPECTATION_CONTRACT] != crate::application::run::constants::DIAGNOSTICS
                    || row[crate::application::run::constants::EXECUTION_METADATA][crate::application::run::constants::AVAILABILITY] != crate::application::run::constants::COMPLETE
                    || selected == 0
                    || evaluated.checked_add(skipped) != Some(selected)
                    || coverage[crate::application::run::constants::UNKNOWN_OR_UNREAD_RULE_FILE_PAIRS] != 0
                {
                    return Err(crate::application::run::constants::CANDIDATE_EXECUTION_IS_NOT_COMPLETELY_ACCOUN.into());
                }
            }
            if case[crate::application::run::constants::PROJECT_CONTEXT_MODE] == crate::application::run::constants::ISOLATED {
                if !row[crate::application::run::constants::CONTEXT_MANIFEST].is_null() || !row[crate::application::run::constants::PROJECT_CONTEXT].is_null() {
                    return Err(crate::application::run::constants::UNEXPECTED_PROJECT_CONTEXT_RECEIPT.into());
                }
            } else {
                for key in [crate::application::run::constants::MODE, crate::application::run::constants::LOGICAL_ROOT, crate::application::run::constants::MANIFEST_HASH] {
                    if row[crate::application::run::constants::CONTEXT_MANIFEST][key] != case[crate::application::run::constants::CONTEXT_IDENTITY][key] {
                        return Err(crate::application::run::constants::STALE_PROJECT_CONTEXT_RECEIPT.into());
                    }
                }
            }
            sides.push(row);
        }
        if !certificate.is_null() {
            let proof = &sides[0][crate::application::run::constants::DISCREPANCY_VALIDATION];
            if proof[crate::application::run::constants::FILE_METADATA_SHA256] != metadata
                || certificate[crate::application::run::constants::IDENTITY][crate::application::run::constants::FILE_METADATA_SHA256_2] != metadata
            {
                return Err(crate::application::run::constants::DISCREPANCY_FILE_METADATA_IDENTITY_DIFFERS.into());
            }
            if proof.is_null() || proof != &sides[1][crate::application::run::constants::DISCREPANCY_VALIDATION] {
                return Err(crate::application::run::constants::MISSING_INDEPENDENT_DISCREPANCY_VALIDATION.into());
            }
            if proof[crate::application::run::constants::DISCREPANCY_BASIS]
                .as_str()
                .unwrap_or(crate::application::run::constants::LANGUAGE_DEFECT)
                != basis
            {
                return Err(crate::application::run::constants::DISCREPANCY_AUTHORITY_WAS_NOT_VERIFIED.into());
            }
            for (actual, expected) in [
                (crate::application::run::constants::SETTINGS_SHA256, crate::application::run::constants::SETTINGS_SHA256_2),
                (crate::application::run::constants::PROFILE_SHA256, crate::application::run::constants::PROFILE_SHA256_2),
                (crate::application::run::constants::SOURCE_SHA256, crate::application::run::constants::SOURCE_SHA256_2),
                (crate::application::run::constants::REFERENCE_OUTPUT_SHA256, crate::application::run::constants::REFERENCE_OUTPUT_SHA256_2),
                (crate::application::run::constants::INTENDED_OUTPUT_SHA256, crate::application::run::constants::INTENDED_OUTPUT_SHA256_2),
            ] {
                if !proof[actual]
                    .as_str()
                    .is_some_and(|value| !value.is_empty())
                    || !certificate[crate::application::run::constants::IDENTITY][expected]
                        .as_str()
                        .is_some_and(|value| !value.is_empty())
                    || proof[actual] != certificate[crate::application::run::constants::IDENTITY][expected]
                {
                    return Err(crate::application::run::constants::STALE_DISCREPANCY_EVIDENCE_IDENTITY.into());
                }
            }
            if !proof[crate::application::run::constants::ACTUAL_EXECUTABLE_SHA256]
                .as_str()
                .is_some_and(|value| !value.is_empty())
            {
                return Err(crate::application::run::constants::DISCREPANCY_TOOL_IDENTITY_IS_MISSING.into());
            }
            if !certificate[crate::application::run::constants::APPROVED_EXECUTABLE_SHA256]
                .as_array()
                .is_some_and(|pins| pins.contains(&proof[crate::application::run::constants::ACTUAL_EXECUTABLE_SHA256]))
            {
                return Err(crate::application::run::constants::DISCREPANCY_TOOL_IS_NOT_INDEPENDENTLY_PINNED.into());
            }
        }
        super::inventory_execution::compare(sides[0], sides[1], declared)?;
        let exact = !process
            && normalized(&sides[0][crate::application::run::constants::DIAGNOSTICS])? == normalized(&sides[1][crate::application::run::constants::DIAGNOSTICS])?
            && sides[0][crate::application::run::constants::SKIPPED] == sides[1][crate::application::run::constants::SKIPPED];
        raw_exact += usize::from(exact);
        documented += usize::from(!exact && !certificate.is_null());
        contract_only += usize::from(!exact && certificate.is_null());
    }
    Ok((raw_exact, documented, contract_only))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn forged_pass_flags_cannot_hide_wrong_or_malformed_diagnostics() {
        let case = json!({
            crate::application::run::constants::RULE: crate::application::run::constants::A001, crate::application::run::constants::NAME: crate::application::run::constants::CASE, crate::application::run::constants::SOURCE_SHA256: crate::application::run::constants::SOURCE_PIN,
            crate::application::run::constants::FILES: [crate::application::run::constants::INPUT_TXT], crate::application::run::constants::EXPECTED: [],
            crate::application::run::constants::PROJECT_CONTEXT_MODE: crate::application::run::constants::ISOLATED, crate::application::run::constants::REFERENCE_DISCREPANCY: null
        });
        let base = json!({
            crate::application::run::constants::RULE: crate::application::run::constants::A001, crate::application::run::constants::NAME: crate::application::run::constants::CASE, crate::application::run::constants::COMPLETE: true, crate::application::run::constants::ERROR: null,
            crate::application::run::constants::CASE_PASSED: true, crate::application::run::constants::SOURCE_SHA256: crate::application::run::constants::SOURCE_PIN, crate::application::run::constants::FILES: [crate::application::run::constants::INPUT_TXT],
            crate::application::run::constants::EXPECTED: [], crate::application::run::constants::REFERENCE_EXPECTED: [], crate::application::run::constants::DIAGNOSTICS: [], crate::application::run::constants::SKIPPED: [],
            crate::application::run::constants::CONTEXT_MANIFEST: null, crate::application::run::constants::PROJECT_CONTEXT: null,
            crate::application::run::constants::OBSERVATION_SCHEMA: crate::application::run::constants::BENCHMARK_CASE_V2, crate::application::run::constants::EXPECTATION_CONTRACT: crate::application::run::constants::DIAGNOSTICS,
            crate::application::run::constants::EXECUTION_METADATA: {crate::application::run::constants::AVAILABILITY: crate::application::run::constants::COMPLETE},
            crate::application::run::constants::COVERAGE: {crate::application::run::constants::SELECTED_RULE_FILE_PAIRS: 1, crate::application::run::constants::EVALUATED_RULE_FILE_PAIRS: 1,
                         crate::application::run::constants::SKIPPED_RULE_FILE_PAIRS: 0, crate::application::run::constants::UNKNOWN_OR_UNREAD_RULE_FILE_PAIRS: 0}
        });
        let mut reference = base.clone();
        reference[crate::application::run::constants::SIDE] = json!(crate::application::run::constants::REFERENCE);
        let mut candidate = base;
        candidate[crate::application::run::constants::SIDE] = json!(crate::application::run::constants::CANDIDATE);
        assert_eq!(
            validate(&[case.clone()], &[reference.clone(), candidate.clone()]),
            Ok((1, 0, 0))
        );

        candidate[crate::application::run::constants::DIAGNOSTICS] =
            json!([{crate::application::run::constants::RULE: crate::application::run::constants::A001, crate::application::run::constants::FILE: crate::application::run::constants::INPUT_TXT, crate::application::run::constants::START: 0, crate::application::run::constants::END: 1}]);
        assert!(
            validate(&[case.clone()], &[reference.clone(), candidate.clone()])
                .unwrap_err()
                .contains(crate::application::run::constants::NATIVE_DIAGNOSTICS_DIFFER)
        );

        candidate[crate::application::run::constants::DIAGNOSTICS] = json!({crate::application::run::constants::NOT: crate::application::run::constants::AN_ARRAY});
        assert!(
            validate(&[case], &[reference, candidate])
                .unwrap_err()
                .contains(crate::application::run::constants::DIAGNOSTICS_MUST_BE_AN_ARRAY)
        );
    }

    #[test]
    fn registered_and_observed_case_identities_must_be_bijective() {
        let case = json!({
            crate::application::run::constants::RULE: crate::application::run::constants::A001, crate::application::run::constants::NAME: crate::application::run::constants::FIRST, crate::application::run::constants::SOURCE_SHA256: crate::application::run::constants::SOURCE_PIN,
            crate::application::run::constants::FILES: [crate::application::run::constants::INPUT_TXT], crate::application::run::constants::EXPECTED: [],
            crate::application::run::constants::PROJECT_CONTEXT_MODE: crate::application::run::constants::ISOLATED, crate::application::run::constants::REFERENCE_DISCREPANCY: null
        });
        let base = json!({
            crate::application::run::constants::RULE: crate::application::run::constants::A001, crate::application::run::constants::NAME: crate::application::run::constants::FIRST, crate::application::run::constants::COMPLETE: true, crate::application::run::constants::ERROR: null,
            crate::application::run::constants::CASE_PASSED: true, crate::application::run::constants::SOURCE_SHA256: crate::application::run::constants::SOURCE_PIN, crate::application::run::constants::FILES: [crate::application::run::constants::INPUT_TXT],
            crate::application::run::constants::EXPECTED: [], crate::application::run::constants::REFERENCE_EXPECTED: [], crate::application::run::constants::DIAGNOSTICS: [], crate::application::run::constants::SKIPPED: [],
            crate::application::run::constants::CONTEXT_MANIFEST: null, crate::application::run::constants::PROJECT_CONTEXT: null,
            crate::application::run::constants::OBSERVATION_SCHEMA: crate::application::run::constants::BENCHMARK_CASE_V2, crate::application::run::constants::EXPECTATION_CONTRACT: crate::application::run::constants::DIAGNOSTICS,
            crate::application::run::constants::EXECUTION_METADATA: {crate::application::run::constants::AVAILABILITY: crate::application::run::constants::COMPLETE},
            crate::application::run::constants::COVERAGE: {crate::application::run::constants::SELECTED_RULE_FILE_PAIRS: 1, crate::application::run::constants::EVALUATED_RULE_FILE_PAIRS: 1,
                         crate::application::run::constants::SKIPPED_RULE_FILE_PAIRS: 0, crate::application::run::constants::UNKNOWN_OR_UNREAD_RULE_FILE_PAIRS: 0}
        });
        let mut reference = base.clone();
        reference[crate::application::run::constants::SIDE] = json!(crate::application::run::constants::REFERENCE);
        let mut candidate = base;
        candidate[crate::application::run::constants::SIDE] = json!(crate::application::run::constants::CANDIDATE);
        let mut unrelated_reference = reference.clone();
        unrelated_reference[crate::application::run::constants::NAME] = json!(crate::application::run::constants::UNRELATED);
        let mut unrelated_candidate = candidate.clone();
        unrelated_candidate[crate::application::run::constants::NAME] = json!(crate::application::run::constants::UNRELATED);
        let rows = [
            reference.clone(),
            candidate.clone(),
            unrelated_reference,
            unrelated_candidate,
        ];

        assert!(
            validate(&[case.clone(), case.clone()], &rows)
                .unwrap_err()
                .contains(crate::application::run::constants::DUPLICATE_REGISTERED_CASE_IDENTITY)
        );
        let mut second = case;
        second[crate::application::run::constants::NAME] = json!(crate::application::run::constants::SECOND);
        assert!(
            validate(
                &[
                    second.clone(),
                    json!({
                        crate::application::run::constants::RULE: crate::application::run::constants::A001, crate::application::run::constants::NAME: crate::application::run::constants::FIRST, crate::application::run::constants::SOURCE_SHA256: crate::application::run::constants::SOURCE_PIN,
                        crate::application::run::constants::FILES: [crate::application::run::constants::INPUT_TXT], crate::application::run::constants::EXPECTED: [],
                        crate::application::run::constants::PROJECT_CONTEXT_MODE: crate::application::run::constants::ISOLATED, crate::application::run::constants::REFERENCE_DISCREPANCY: null
                    })
                ],
                &rows
            )
            .unwrap_err()
            .contains(crate::application::run::constants::UNREGISTERED_NATIVE_CASE_OBSERVATION)
        );

        let duplicate_rows = [reference.clone(), candidate.clone(), reference, candidate];
        assert!(
            validate(
                &[
                    second,
                    json!({
                        crate::application::run::constants::RULE: crate::application::run::constants::A001, crate::application::run::constants::NAME: crate::application::run::constants::FIRST, crate::application::run::constants::SOURCE_SHA256: crate::application::run::constants::SOURCE_PIN,
                        crate::application::run::constants::FILES: [crate::application::run::constants::INPUT_TXT], crate::application::run::constants::EXPECTED: [],
                        crate::application::run::constants::PROJECT_CONTEXT_MODE: crate::application::run::constants::ISOLATED, crate::application::run::constants::REFERENCE_DISCREPANCY: null
                    })
                ],
                &duplicate_rows
            )
            .unwrap_err()
            .contains(crate::application::run::constants::DUPLICATE_NATIVE_CASE_OBSERVATION)
        );
    }

    #[test]
    fn independently_bound_reference_difference_can_pass() {
        let finding = json!({crate::application::run::constants::RULE: crate::application::run::constants::A001, crate::application::run::constants::FILE: crate::application::run::constants::INPUT_TXT, crate::application::run::constants::START: 0, crate::application::run::constants::END: 1});
        let case = json!({
            crate::application::run::constants::RULE: crate::application::run::constants::A001, crate::application::run::constants::NAME: crate::application::run::constants::CASE, crate::application::run::constants::SOURCE_SHA256: crate::application::run::constants::SOURCE_PIN,
            crate::application::run::constants::FILES: [crate::application::run::constants::INPUT_TXT], crate::application::run::constants::EXPECTED: [], crate::application::run::constants::PROJECT_CONTEXT_MODE: crate::application::run::constants::ISOLATED,
            crate::application::run::constants::REFERENCE_DISCREPANCY: {
                crate::application::run::constants::INTENDED_EXPECTED: [], crate::application::run::constants::REFERENCE_EXPECTED: [finding],
                crate::application::run::constants::APPROVED_EXECUTABLE_SHA256: [crate::application::run::constants::TOOL_PIN],
                crate::application::run::constants::IDENTITY: {
                    crate::application::run::constants::SETTINGS_SHA256_2: crate::application::run::constants::SETTINGS_PIN, crate::application::run::constants::PROFILE_SHA256_2: crate::application::run::constants::PROFILE_PIN,
                    crate::application::run::constants::SOURCE_SHA256_2: crate::application::run::constants::SOURCE_PIN, crate::application::run::constants::REFERENCE_OUTPUT_SHA256_2: crate::application::run::constants::REFERENCE_PIN,
                    crate::application::run::constants::INTENDED_OUTPUT_SHA256_2: crate::application::run::constants::INTENDED_PIN
                }
            }
        });
        let proof = json!({
            crate::application::run::constants::ACTUAL_EXECUTABLE_SHA256: crate::application::run::constants::TOOL_PIN, crate::application::run::constants::SETTINGS_SHA256: crate::application::run::constants::SETTINGS_PIN,
            crate::application::run::constants::PROFILE_SHA256: crate::application::run::constants::PROFILE_PIN, crate::application::run::constants::SOURCE_SHA256: crate::application::run::constants::SOURCE_PIN,
            crate::application::run::constants::REFERENCE_OUTPUT_SHA256: crate::application::run::constants::REFERENCE_PIN, crate::application::run::constants::INTENDED_OUTPUT_SHA256: crate::application::run::constants::INTENDED_PIN
        });
        let base = json!({
            crate::application::run::constants::RULE: crate::application::run::constants::A001, crate::application::run::constants::NAME: crate::application::run::constants::CASE, crate::application::run::constants::COMPLETE: true, crate::application::run::constants::ERROR: null,
            crate::application::run::constants::CASE_PASSED: true, crate::application::run::constants::SOURCE_SHA256: crate::application::run::constants::SOURCE_PIN, crate::application::run::constants::FILES: [crate::application::run::constants::INPUT_TXT],
            crate::application::run::constants::EXPECTED: [], crate::application::run::constants::REFERENCE_EXPECTED: [finding], crate::application::run::constants::DIAGNOSTICS: [], crate::application::run::constants::SKIPPED: [],
            crate::application::run::constants::DISCREPANCY_VALIDATION: proof,
            crate::application::run::constants::DOCUMENTED_REFERENCE_DISCREPANCY: true, crate::application::run::constants::DOCUMENTED_TOOL_CONTRACT_DIFFERENCE: false,
            crate::application::run::constants::CONTEXT_MANIFEST: null, crate::application::run::constants::PROJECT_CONTEXT: null,
            crate::application::run::constants::OBSERVATION_SCHEMA: crate::application::run::constants::BENCHMARK_CASE_V2, crate::application::run::constants::EXPECTATION_CONTRACT: crate::application::run::constants::DIAGNOSTICS,
            crate::application::run::constants::EXECUTION_METADATA: {crate::application::run::constants::AVAILABILITY: crate::application::run::constants::COMPLETE},
            crate::application::run::constants::COVERAGE: {crate::application::run::constants::SELECTED_RULE_FILE_PAIRS: 1, crate::application::run::constants::EVALUATED_RULE_FILE_PAIRS: 1,
                         crate::application::run::constants::SKIPPED_RULE_FILE_PAIRS: 0, crate::application::run::constants::UNKNOWN_OR_UNREAD_RULE_FILE_PAIRS: 0}
        });
        let mut reference = base.clone();
        reference[crate::application::run::constants::SIDE] = json!(crate::application::run::constants::REFERENCE);
        reference[crate::application::run::constants::DIAGNOSTICS] = json!([finding]);
        let mut candidate = base;
        candidate[crate::application::run::constants::SIDE] = json!(crate::application::run::constants::CANDIDATE);
        assert_eq!(
            validate(&[case.clone()], &[reference.clone(), candidate.clone()]),
            Ok((0, 1, 0))
        );

        let mut unbound = case;
        unbound[crate::application::run::constants::REFERENCE_DISCREPANCY][crate::application::run::constants::IDENTITY][crate::application::run::constants::SETTINGS_SHA256_2] = Value::Null;
        reference[crate::application::run::constants::DISCREPANCY_VALIDATION][crate::application::run::constants::SETTINGS_SHA256] = Value::Null;
        candidate[crate::application::run::constants::DISCREPANCY_VALIDATION][crate::application::run::constants::SETTINGS_SHA256] = Value::Null;
        assert!(
            validate(&[unbound], &[reference, candidate])
                .unwrap_err()
                .contains(crate::application::run::constants::STALE_DISCREPANCY_EVIDENCE_IDENTITY)
        );
    }
}

#[cfg(test)]
mod input_metadata_test;
