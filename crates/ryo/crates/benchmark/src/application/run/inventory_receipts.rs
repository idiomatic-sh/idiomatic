//! Validate native case observations; expectation comparison stays in the existing verifier.
use crate::application::run::constants;
use ryo_benchmark_contracts::domain::artifacts::types::{FileExecutable, FileMetadataEntry};
use std::collections::HashSet;

use serde_json::Value;

fn normalized(value: &Value) -> Result<Vec<String>, String> {
    let source = value
        .as_array()
        .ok_or(constants::DIAGNOSTICS_MUST_BE_AN_ARRAY)?;
    let mut rows = Vec::with_capacity(source.len());
    for value in source {
        let rule = value[constants::RULE]
            .as_str()
            .ok_or(constants::DIAGNOSTIC_RULE_IS_MISSING)?;
        let file = value[constants::FILE]
            .as_str()
            .ok_or(constants::DIAGNOSTIC_FILE_IS_MISSING)?;
        let start = value[constants::START]
            .as_u64()
            .ok_or(constants::DIAGNOSTIC_START_IS_INVALID)?;
        let end = value[constants::END]
            .as_u64()
            .ok_or(constants::DIAGNOSTIC_END_IS_INVALID)?;
        if rule.is_empty() || file.is_empty() || start > end {
            return Err(constants::DIAGNOSTIC_SPAN_OR_IDENTITY_IS_INVALID.into());
        }
        rows.push(value.to_string());
    }
    rows.sort();
    Ok(rows)
}
/// Decode only declared metadata and recompute its pure framed identity.
pub(super) fn metadata_digest(case: &Value) -> Result<Value, String> {
    let files = case[constants::FILES]
        .as_array()
        .ok_or(constants::REGISTERED_FILES_ARE_MISSING)?;
    let required = !case[constants::FILE_OBSERVATION].is_null();
    if required && case[constants::FILE_OBSERVATION] != constants::POSIX_EXECUTABLE {
        return Err(constants::UNSUPPORTED_REGISTERED_FILE_OBSERVATION.into());
    }
    let metadata = if case[constants::FILE_METADATA].is_null() {
        None
    } else {
        let values = case[constants::FILE_METADATA]
            .as_object()
            .ok_or(constants::FILE_METADATA_MUST_BE_AN_OBJECT)?;
        let mut parsed = std::collections::BTreeMap::new();
        for (path, value) in values {
            ryo_benchmark_contracts::domain::artifacts::types::relative_artifact_path::value::RelativeArtifactPath::try_from(std::path::Path::new(std::path::Path::new(path)))
                .map_err(|error| error.to_string())?;
            if !files.iter().any(|file| file.as_str() == Some(path)) {
                return Err(constants::METADATA_NAMES_AN_UNDECLARED_SOURCE.into());
            }
            parsed.insert(
                ryo::linter::RelativePath(path.clone().into_boxed_str()),
                value
                    .as_bool()
                    .ok_or(constants::EXECUTABLE_OBSERVATION_MUST_BE_BOOLEAN)?,
            );
        }
        Some(parsed)
    };
    if required
        && metadata
            .as_ref()
            .is_none_or(|values| values.len() != files.len())
    {
        return Err(constants::REQUIRED_INPUT_OBSERVATIONS_ARE_MISSING.into());
    }
    Ok(serde_json::json!(
        ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest::file_metadata((metadata.as_ref()).map(|values| values.iter().map(|(path, executable)| FileMetadataEntry { path, executable: if *executable { FileExecutable::Set } else { FileExecutable::Unset } })))
            .map_err(|error| error.to_string())?
            .map(|value| value.0)
    ))
}

/// Recompute the project input identity from the registered explicit roots and entry inventory.
fn project_inputs_digest(case: &Value) -> Result<Value, String> {
    let roots = if case[constants::PROJECT_SEARCH_ROOTS].is_null() {
        None
    } else {
        Some(
            case[constants::PROJECT_SEARCH_ROOTS]
                .as_array()
                .ok_or(constants::PROJECT_SEARCH_ROOTS_MUST_BE_AN)?
                .iter()
                .map(|value| {
                    let root = value.as_str().ok_or(constants::PROJECT_SEARCH_ROOT_IS_NOT_TEXT)?;
                    if !root.is_empty() {
                        ryo_benchmark_contracts::domain::artifacts::types::relative_artifact_path::value::RelativeArtifactPath::try_from(std::path::Path::new(root))
                            .map_err(|error| error.to_string())?;
                    }
                    Ok(ryo::linter::RelativePath(root.to_owned().into_boxed_str()))
                })
                .collect::<Result<Vec<_>, String>>()?,
        )
    };
    let inventory = if case[constants::GIVEN_PROJECT_INVENTORY].is_null() {
        ryo::linter::GivenProjectInventory::default()
    } else {
        let wire: crate::outbound::context::adapters::local::schemas::ProjectInventorySchema =
            serde_json::from_value(case[constants::GIVEN_PROJECT_INVENTORY].clone())
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
        return Err(constants::MISSING_OR_EXTRA_NATIVE_CASE_RECEIPTS.into());
    }
    let mut registered = HashSet::with_capacity(cases.len());
    for case in cases {
        let rule = case[constants::RULE]
            .as_str()
            .filter(|value| !value.is_empty())
            .ok_or(constants::REGISTERED_RULE_IDENTITY_IS_MISSING)?;
        let name = case[constants::NAME]
            .as_str()
            .filter(|value| !value.is_empty())
            .ok_or(constants::REGISTERED_CASE_IDENTITY_IS_MISSING)?;
        if !registered.insert((rule, name)) {
            return Err(constants::DUPLICATE_REGISTERED_CASE_IDENTITY.into());
        }
    }
    let mut observed = HashSet::with_capacity(observations.len());
    for row in observations {
        let rule = row[constants::RULE]
            .as_str()
            .ok_or(constants::NATIVE_RULE_IDENTITY_IS_MISSING)?;
        let name = row[constants::NAME]
            .as_str()
            .ok_or(constants::NATIVE_CASE_IDENTITY_IS_MISSING)?;
        let side = row[constants::SIDE]
            .as_str()
            .ok_or(constants::NATIVE_SIDE_IDENTITY_IS_MISSING)?;
        if !registered.contains(&(rule, name))
            || !matches!(side, constants::REFERENCE | constants::CANDIDATE)
        {
            return Err(constants::UNREGISTERED_NATIVE_CASE_OBSERVATION.into());
        }
        if !observed.insert((rule, name, side)) {
            return Err(constants::DUPLICATE_NATIVE_CASE_OBSERVATION.into());
        }
    }
    let mut raw_exact = 0;
    let mut documented = 0;
    let mut contract_only = 0;
    for case in cases {
        let metadata = metadata_digest(case)?;
        let project_inputs = project_inputs_digest(case)?;
        if case[constants::FILE_METADATA_SHA256] != metadata {
            return Err(constants::REGISTERED_FILE_METADATA_DIGEST_DIFFERS.into());
        }
        if case[constants::PROJECT_INPUTS_SHA256] != project_inputs {
            return Err(constants::REGISTERED_PROJECT_INPUTS_DIGEST_DIFFERS.into());
        }
        let declared = if case[constants::EXPECTATION_CONTRACT].is_null() {
            ryo_benchmark_contracts::domain::reports::types::ComparisonContract::Diagnostics
        } else {
            super::inventory_execution::contract(&case[constants::EXPECTATION_CONTRACT])?
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
            return Err(constants::UNKNOWN_DISCREPANCY_AUTHORITY.into());
        }
        let process = !certificate[constants::PROCESS].is_null();
        if process
            && declared
                != ryo_benchmark_contracts::domain::reports::types::ComparisonContract::Diagnostics
        {
            return Err(constants::PROCESS_EXPECTATION_CANNOT_SATISFY_COMPLETE.into());
        }
        let reference_expected = if certificate.is_null() {
            expected.clone()
        } else {
            if normalized(&certificate[constants::INTENDED_EXPECTED])? != expected {
                return Err(constants::DISCREPANCY_INTENTION_DIFFERS_FROM_REGISTERE.into());
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
                return Err(constants::MISSING_OR_DUPLICATE_NATIVE_SIDE_RECEIPT.into());
            }
            let row = found[0];
            if row[constants::RULE_CONTEXT] != case[constants::RULE_CONTEXT] {
                return Err(constants::NATIVE_CONFIGURED_RULE_CONTEXT_DIFFERS_FROM.into());
            }
            if row[constants::RULE_OBSERVATIONS] != case[constants::RULE_OBSERVATIONS] {
                return Err(constants::NATIVE_OBSERVATION_DECLARATIONS_DIFFER_FROM.into());
            }
            if row[constants::SELECTED_RULES] != case[constants::SELECTED_RULES] {
                return Err(constants::NATIVE_SELECTED_RULE_COHORT_DIFFERS_FROM.into());
            }
            if row[constants::OBSERVATION_SCHEMA] != constants::BENCHMARK_CASE_V2
                || super::inventory_execution::contract(&row[constants::EXPECTATION_CONTRACT])?
                    != declared
            {
                return Err(constants::NATIVE_EXPECTATION_CONTRACT_DIFFERS_FROM_REG.into());
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
                return Err(constants::FAILED_INCOMPLETE_OR_STALE_NATIVE_OBSERVATIO.into());
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
                return Err(constants::NATIVE_DISCREPANCY_AUTHORITY_DIFFERS_FROM_RE.into());
            }
            if process && side == constants::REFERENCE {
                super::process_receipts::validate(case, row, artifacts)?;
            } else {
                if !row[constants::PROCESS_OBSERVATION].is_null() {
                    return Err(constants::DIAGNOSTIC_REPORT_UNEXPECTEDLY_CONTAINS_A_PR.into());
                }
                let actual = normalized(&row[constants::DIAGNOSTICS])?;
                let required = if side == constants::REFERENCE {
                    &reference_expected
                } else {
                    &expected
                };
                if actual.as_slice() != required.as_slice() || !row[constants::SKIPPED].is_array() {
                    return Err(constants::NATIVE_DIAGNOSTICS_DIFFER_FROM_REGISTERED_EX.into());
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
                    return Err(constants::CANDIDATE_EXECUTION_IS_NOT_COMPLETELY_ACCOUN.into());
                }
            }
            if case[constants::PROJECT_CONTEXT_MODE] == constants::ISOLATED {
                if !row[constants::CONTEXT_MANIFEST].is_null()
                    || !row[constants::PROJECT_CONTEXT].is_null()
                {
                    return Err(constants::UNEXPECTED_PROJECT_CONTEXT_RECEIPT.into());
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
                        return Err(constants::STALE_PROJECT_CONTEXT_RECEIPT.into());
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
                return Err(constants::DISCREPANCY_FILE_METADATA_IDENTITY_DIFFERS.into());
            }
            if proof.is_null() || proof != &sides[1][constants::DISCREPANCY_VALIDATION] {
                return Err(constants::MISSING_INDEPENDENT_DISCREPANCY_VALIDATION.into());
            }
            if proof[constants::DISCREPANCY_BASIS]
                .as_str()
                .unwrap_or(constants::LANGUAGE_DEFECT)
                != basis
            {
                return Err(constants::DISCREPANCY_AUTHORITY_WAS_NOT_VERIFIED.into());
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
                    return Err(constants::STALE_DISCREPANCY_EVIDENCE_IDENTITY.into());
                }
            }
            if !proof[constants::ACTUAL_EXECUTABLE_SHA256]
                .as_str()
                .is_some_and(|value| !value.is_empty())
            {
                return Err(constants::DISCREPANCY_TOOL_IDENTITY_IS_MISSING.into());
            }
            if !certificate[constants::APPROVED_EXECUTABLE_SHA256]
                .as_array()
                .is_some_and(|pins| pins.contains(&proof[constants::ACTUAL_EXECUTABLE_SHA256]))
            {
                return Err(constants::DISCREPANCY_TOOL_IS_NOT_INDEPENDENTLY_PINNED.into());
            }
        }
        super::inventory_execution::compare(sides[0], sides[1], declared)?;
        let exact = !process
            && normalized(&sides[0][constants::DIAGNOSTICS])?
                == normalized(&sides[1][constants::DIAGNOSTICS])?
            && sides[0][constants::SKIPPED] == sides[1][constants::SKIPPED];
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
            constants::RULE: constants::A001, constants::NAME: constants::CASE, constants::SOURCE_SHA256: constants::SOURCE_PIN,
            constants::FILES: [constants::INPUT_TXT], constants::EXPECTED: [],
            constants::PROJECT_CONTEXT_MODE: constants::ISOLATED, constants::REFERENCE_DISCREPANCY: null
        });
        let base = json!({
            constants::RULE: constants::A001, constants::NAME: constants::CASE, constants::COMPLETE: true, constants::ERROR: null,
            constants::CASE_PASSED: true, constants::SOURCE_SHA256: constants::SOURCE_PIN, constants::FILES: [constants::INPUT_TXT],
            constants::EXPECTED: [], constants::REFERENCE_EXPECTED: [], constants::DIAGNOSTICS: [], constants::SKIPPED: [],
            constants::CONTEXT_MANIFEST: null, constants::PROJECT_CONTEXT: null,
            constants::OBSERVATION_SCHEMA: constants::BENCHMARK_CASE_V2, constants::EXPECTATION_CONTRACT: constants::DIAGNOSTICS,
            constants::EXECUTION_METADATA: {constants::AVAILABILITY: constants::COMPLETE},
            constants::COVERAGE: {constants::SELECTED_RULE_FILE_PAIRS: 1, constants::EVALUATED_RULE_FILE_PAIRS: 1,
                         constants::SKIPPED_RULE_FILE_PAIRS: 0, constants::UNKNOWN_OR_UNREAD_RULE_FILE_PAIRS: 0}
        });
        let mut reference = base.clone();
        reference[constants::SIDE] = json!(constants::REFERENCE);
        let mut candidate = base;
        candidate[constants::SIDE] = json!(constants::CANDIDATE);
        assert_eq!(
            validate(&[case.clone()], &[reference.clone(), candidate.clone()]),
            Ok((1, 0, 0))
        );

        candidate[constants::DIAGNOSTICS] = json!([{constants::RULE: constants::A001, constants::FILE: constants::INPUT_TXT, constants::START: 0, constants::END: 1}]);
        assert!(
            validate(&[case.clone()], &[reference.clone(), candidate.clone()])
                .unwrap_err()
                .contains(constants::NATIVE_DIAGNOSTICS_DIFFER)
        );

        candidate[constants::DIAGNOSTICS] = json!({constants::NOT: constants::AN_ARRAY});
        assert!(
            validate(&[case], &[reference, candidate])
                .unwrap_err()
                .contains(constants::DIAGNOSTICS_MUST_BE_AN_ARRAY)
        );
    }

    #[test]
    fn registered_and_observed_case_identities_must_be_bijective() {
        let case = json!({
            constants::RULE: constants::A001, constants::NAME: constants::FIRST, constants::SOURCE_SHA256: constants::SOURCE_PIN,
            constants::FILES: [constants::INPUT_TXT], constants::EXPECTED: [],
            constants::PROJECT_CONTEXT_MODE: constants::ISOLATED, constants::REFERENCE_DISCREPANCY: null
        });
        let base = json!({
            constants::RULE: constants::A001, constants::NAME: constants::FIRST, constants::COMPLETE: true, constants::ERROR: null,
            constants::CASE_PASSED: true, constants::SOURCE_SHA256: constants::SOURCE_PIN, constants::FILES: [constants::INPUT_TXT],
            constants::EXPECTED: [], constants::REFERENCE_EXPECTED: [], constants::DIAGNOSTICS: [], constants::SKIPPED: [],
            constants::CONTEXT_MANIFEST: null, constants::PROJECT_CONTEXT: null,
            constants::OBSERVATION_SCHEMA: constants::BENCHMARK_CASE_V2, constants::EXPECTATION_CONTRACT: constants::DIAGNOSTICS,
            constants::EXECUTION_METADATA: {constants::AVAILABILITY: constants::COMPLETE},
            constants::COVERAGE: {constants::SELECTED_RULE_FILE_PAIRS: 1, constants::EVALUATED_RULE_FILE_PAIRS: 1,
                         constants::SKIPPED_RULE_FILE_PAIRS: 0, constants::UNKNOWN_OR_UNREAD_RULE_FILE_PAIRS: 0}
        });
        let mut reference = base.clone();
        reference[constants::SIDE] = json!(constants::REFERENCE);
        let mut candidate = base;
        candidate[constants::SIDE] = json!(constants::CANDIDATE);
        let mut unrelated_reference = reference.clone();
        unrelated_reference[constants::NAME] = json!(constants::UNRELATED);
        let mut unrelated_candidate = candidate.clone();
        unrelated_candidate[constants::NAME] = json!(constants::UNRELATED);
        let rows = [
            reference.clone(),
            candidate.clone(),
            unrelated_reference,
            unrelated_candidate,
        ];

        assert!(
            validate(&[case.clone(), case.clone()], &rows)
                .unwrap_err()
                .contains(constants::DUPLICATE_REGISTERED_CASE_IDENTITY)
        );
        let mut second = case;
        second[constants::NAME] = json!(constants::SECOND);
        assert!(
            validate(
                &[
                    second.clone(),
                    json!({
                        constants::RULE: constants::A001, constants::NAME: constants::FIRST, constants::SOURCE_SHA256: constants::SOURCE_PIN,
                        constants::FILES: [constants::INPUT_TXT], constants::EXPECTED: [],
                        constants::PROJECT_CONTEXT_MODE: constants::ISOLATED, constants::REFERENCE_DISCREPANCY: null
                    })
                ],
                &rows
            )
            .unwrap_err()
            .contains(constants::UNREGISTERED_NATIVE_CASE_OBSERVATION)
        );

        let duplicate_rows = [reference.clone(), candidate.clone(), reference, candidate];
        assert!(
            validate(
                &[
                    second,
                    json!({
                        constants::RULE: constants::A001, constants::NAME: constants::FIRST, constants::SOURCE_SHA256: constants::SOURCE_PIN,
                        constants::FILES: [constants::INPUT_TXT], constants::EXPECTED: [],
                        constants::PROJECT_CONTEXT_MODE: constants::ISOLATED, constants::REFERENCE_DISCREPANCY: null
                    })
                ],
                &duplicate_rows
            )
            .unwrap_err()
            .contains(constants::DUPLICATE_NATIVE_CASE_OBSERVATION)
        );
    }

    #[test]
    fn independently_bound_reference_difference_can_pass() {
        let finding = json!({constants::RULE: constants::A001, constants::FILE: constants::INPUT_TXT, constants::START: 0, constants::END: 1});
        let case = json!({
            constants::RULE: constants::A001, constants::NAME: constants::CASE, constants::SOURCE_SHA256: constants::SOURCE_PIN,
            constants::FILES: [constants::INPUT_TXT], constants::EXPECTED: [], constants::PROJECT_CONTEXT_MODE: constants::ISOLATED,
            constants::REFERENCE_DISCREPANCY: {
                constants::INTENDED_EXPECTED: [], constants::REFERENCE_EXPECTED: [finding],
                constants::APPROVED_EXECUTABLE_SHA256: [constants::TOOL_PIN],
                constants::IDENTITY: {
                    constants::SETTINGS_SHA256_2: constants::SETTINGS_PIN, constants::PROFILE_SHA256_2: constants::PROFILE_PIN,
                    constants::SOURCE_SHA256_2: constants::SOURCE_PIN, constants::REFERENCE_OUTPUT_SHA256_2: constants::REFERENCE_PIN,
                    constants::INTENDED_OUTPUT_SHA256_2: constants::INTENDED_PIN
                }
            }
        });
        let proof = json!({
            constants::ACTUAL_EXECUTABLE_SHA256: constants::TOOL_PIN, constants::SETTINGS_SHA256: constants::SETTINGS_PIN,
            constants::PROFILE_SHA256: constants::PROFILE_PIN, constants::SOURCE_SHA256: constants::SOURCE_PIN,
            constants::REFERENCE_OUTPUT_SHA256: constants::REFERENCE_PIN, constants::INTENDED_OUTPUT_SHA256: constants::INTENDED_PIN
        });
        let base = json!({
            constants::RULE: constants::A001, constants::NAME: constants::CASE, constants::COMPLETE: true, constants::ERROR: null,
            constants::CASE_PASSED: true, constants::SOURCE_SHA256: constants::SOURCE_PIN, constants::FILES: [constants::INPUT_TXT],
            constants::EXPECTED: [], constants::REFERENCE_EXPECTED: [finding], constants::DIAGNOSTICS: [], constants::SKIPPED: [],
            constants::DISCREPANCY_VALIDATION: proof,
            constants::DOCUMENTED_REFERENCE_DISCREPANCY: true, constants::DOCUMENTED_TOOL_CONTRACT_DIFFERENCE: false,
            constants::CONTEXT_MANIFEST: null, constants::PROJECT_CONTEXT: null,
            constants::OBSERVATION_SCHEMA: constants::BENCHMARK_CASE_V2, constants::EXPECTATION_CONTRACT: constants::DIAGNOSTICS,
            constants::EXECUTION_METADATA: {constants::AVAILABILITY: constants::COMPLETE},
            constants::COVERAGE: {constants::SELECTED_RULE_FILE_PAIRS: 1, constants::EVALUATED_RULE_FILE_PAIRS: 1,
                         constants::SKIPPED_RULE_FILE_PAIRS: 0, constants::UNKNOWN_OR_UNREAD_RULE_FILE_PAIRS: 0}
        });
        let mut reference = base.clone();
        reference[constants::SIDE] = json!(constants::REFERENCE);
        reference[constants::DIAGNOSTICS] = json!([finding]);
        let mut candidate = base;
        candidate[constants::SIDE] = json!(constants::CANDIDATE);
        assert_eq!(
            validate(&[case.clone()], &[reference.clone(), candidate.clone()]),
            Ok((0, 1, 0))
        );

        let mut unbound = case;
        unbound[constants::REFERENCE_DISCREPANCY][constants::IDENTITY]
            [constants::SETTINGS_SHA256_2] = Value::Null;
        reference[constants::DISCREPANCY_VALIDATION][constants::SETTINGS_SHA256] = Value::Null;
        candidate[constants::DISCREPANCY_VALIDATION][constants::SETTINGS_SHA256] = Value::Null;
        assert!(
            validate(&[unbound], &[reference, candidate])
                .unwrap_err()
                .contains(constants::STALE_DISCREPANCY_EVIDENCE_IDENTITY)
        );
    }
}

#[cfg(test)]
mod input_metadata_test;
