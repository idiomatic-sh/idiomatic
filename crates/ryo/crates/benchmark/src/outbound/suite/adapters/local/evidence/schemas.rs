//! The shape of the evidence envelopes a discrepancy cites, and the check that they establish what the case asserts.
use crate::outbound::suite::adapters::local::constants;
use crate::outbound::suite::errors::SuiteError;
use crate::outbound::suite::types::EvidenceRequirement;
use sha2::{Digest as _, Sha256};

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;

/// Check the asserted source and prior execution against the hash-verified envelope bytes.
pub(in super::super) fn verify_requirement(
    bytes: &[u8],
    requirement: &EvidenceRequirement,
) -> Result<(), SuiteError> {
    use serde_json::Value;
    let fail =
        || SuiteError::Invalid(constants::TOOL_CONTRACT_EVIDENCE_DOES_NOT_ESTABLISH.into());
    let value = match requirement {
        EvidenceRequirement::Language => return Ok(()),
        _ => serde_json::from_slice::<Value>(bytes).map_err(|_| fail())?,
    };
    let field = |name: &str| value.get(name).and_then(Value::as_str).ok_or_else(fail);
    match requirement {
        EvidenceRequirement::Language => Ok(()),
        EvidenceRequirement::PublicToolContract {
            source_revision,
            source_path,
        } => {
            if field(constants::SCHEMA)? != constants::PUBLIC_TOOL_CONTRACT_V1
                || field(constants::SOURCE_REVISION)? != source_revision.0
                || field(constants::SOURCE_PATH)? != source_path.0
            {
                return Err(fail());
            }
            let source = field(constants::SOURCE_TEXT)?;
            let assertion = field(constants::CONTRACT_ASSERTION)?;
            if assertion.is_empty()
                || !source.contains(assertion)
                || field(constants::SOURCE_SHA256)?
                    != format!("{:x}", Sha256::digest(source.as_bytes()))
            {
                return Err(fail());
            }
            Ok(())
        }
        EvidenceRequirement::PriorWorkerRun {
            worker_sha256,
            control_source_sha256,
            control_query_sha256,
            control_profile_sha256,
            witness_case,
        } => {
            if field(constants::SCHEMA)? != constants::PRIOR_WORKER_CONTROL_V1
                || field(constants::WORKER_SHA256)? != worker_sha256.0
                || field(constants::CONTROL_SOURCE_SHA256)? != control_source_sha256.0
                || field(constants::CONTROL_QUERY_SHA256)? != control_query_sha256.0
                || field(constants::CONTROL_PROFILE_SHA256)? != control_profile_sha256.0
                || field(constants::WITNESS_CASE)? != witness_case.0
            {
                return Err(fail());
            }
            let query = field(constants::CONTROL_QUERY)?;
            let profile = field(constants::CONTROL_PROFILE)?;
            if format!("{:x}", Sha256::digest(query.as_bytes())) != control_query_sha256.0
                || format!("{:x}", Sha256::digest(profile.as_bytes())) != control_profile_sha256.0
            {
                return Err(fail());
            }
            let sources = value
                .get(constants::CONTROL_SOURCES)
                .and_then(Value::as_object)
                .ok_or_else(fail)?;
            let mut digest = Sha256::new();
            let mut names = sources.keys().collect::<Vec<_>>();
            names.sort();
            for name in names {
                let source = sources[name].as_str().ok_or_else(fail)?;
                digest.update(name.as_bytes());
                digest.update([0]);
                digest.update(source.as_bytes());
            }
            if format!("{:x}", digest.finalize()) != control_source_sha256.0 {
                return Err(fail());
            }
            let receipt = field(constants::RAW_RECEIPT)?;
            if field(constants::RAW_RECEIPT_SHA256)?
                != format!("{:x}", Sha256::digest(receipt.as_bytes()))
            {
                return Err(fail());
            }
            // The control receipt alone does not name the executable or staged query.
            // Require the captured execution manifest from that run to bind all inputs.
            let execution = field(constants::EXECUTION_MANIFEST)?;
            if field(constants::EXECUTION_MANIFEST_SHA256)?
                != format!("{:x}", Sha256::digest(execution.as_bytes()))
            {
                return Err(fail());
            }
            let run: Value = serde_json::from_str(execution).map_err(|_| fail())?;
            if run[constants::SCHEMA] != constants::PRIOR_WORKER_EXECUTION_V1
                || run[constants::WORKER_SHA256] != worker_sha256.0
                || run[constants::CONTROL_SOURCE_SHA256] != control_source_sha256.0
                || run[constants::CONTROL_QUERY_SHA256] != control_query_sha256.0
                || run[constants::CONTROL_PROFILE_SHA256] != control_profile_sha256.0
                || run[constants::RAW_RECEIPT_SHA256] != field(constants::RAW_RECEIPT_SHA256)?
                || run[constants::WITNESS_CASE] != witness_case.0
                || !matches!(run[constants::STATUS].as_i64(), Some(0 | 1))
                || run[constants::COMMAND][1] != constants::VERIFY_CASES
                || run[constants::COMMAND]
                    .as_array()
                    .is_none_or(|parts| parts.len() != 4)
                || run[constants::COMMAND][3] != run[constants::STAGE_ROOT]
                || run[constants::COMMAND][0]
                    .as_str()
                    .is_none_or(|path| !path.ends_with(worker_sha256))
            {
                return Err(fail());
            }
            let mut witnessed = false;
            for line in receipt.lines() {
                let row: Value = serde_json::from_str(line).map_err(|_| fail())?;
                if row[constants::SIDE] == constants::CANDIDATE
                    && row[constants::NAME] == witness_case.0
                {
                    if witnessed
                        || row[constants::COMPLETE] != true
                        || row[constants::ERROR_2] != Value::Null
                        || row[constants::CASE_PASSED] != true
                        || row[constants::SOURCE_SHA256] != control_source_sha256.0
                        || row[constants::SKIPPED] != serde_json::json!([])
                        || row[constants::DIAGNOSTICS] != row[constants::EXPECTED]
                        || row[constants::DIAGNOSTICS]
                            .as_array()
                            .is_none_or(Vec::is_empty)
                        || row[constants::COVERAGE][constants::SELECTED_RULE_FILE_PAIRS]
                            .as_u64()
                            .is_none_or(|number| {
                                number == 0
                                    || row[constants::COVERAGE]
                                        [constants::EVALUATED_RULE_FILE_PAIRS]
                                        .as_u64()
                                        != Some(number)
                            })
                        || row[constants::COVERAGE][constants::UNKNOWN_OR_UNREAD_RULE_FILE_PAIRS]
                            != 0
                        || row[constants::COVERAGE][constants::SKIPPED_RULE_FILE_PAIRS] != 0
                    {
                        return Err(fail());
                    }
                    witnessed = true;
                }
            }
            if !witnessed {
                return Err(fail());
            }
            Ok(())
        }
    }
}
