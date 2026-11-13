//! The public inventory summary, the per-rule result rows and the offline audit answer.
use crate::outbound::inventory::adapters::local::constants;
use crate::outbound::inventory::adapters::local::types::{AuditJson, RowJson, SummaryJson};
use crate::outbound::inventory::types::{
    ReceiptDigest, Row, RowInstance, RowReason, RowStatus, RunState, Summary,
};
use ryo_benchmark_contracts::domain::documents::types::text::value::CanonicalJson;
use ryo_benchmark_contracts::domain::measurements::types::CapturedBytes;
use ryo_benchmark_contracts::domain::receipts::constants::{
    BENCHMARK_INVENTORY_V1, COMPLETE, FAIL, FAILED, PASS, RUNNING,
};
use ryo_benchmark_contracts::domain::receipts::types::ReceiptAudit;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use serde::Serialize;
use std::collections::BTreeMap;

/// The one-line JSON answer of an offline receipt audit, newline included.
fn audit(audit: &ReceiptAudit) -> Result<CapturedBytes, Error> {
    use ryo_benchmark_contracts::domain::receipts::types::CaseKey;
    let case = |key: &CaseKey| serde_json::json!({"rule": key.rule.0, "name": key.name.0});
    let mut result = serde_json::json!({
        "schema": "benchmark-case-receipt-audit/v1",
        "case_count": audit.case_count.0,
        "validated_case_ids": audit.verified.iter().map(case).collect::<Vec<_>>(),
        "raw_exact": audit.raw_exact.0,
        "documented_discrepancies": audit.documented_discrepancies.0,
        "contract_only": audit.contract_only.0,
        "errors": audit.errors.iter().map(|error| error.0.as_str()).collect::<Vec<_>>(),
    });
    if !audit.metadata_verified.is_empty() {
        result["file_metadata_verified_cases"] = serde_json::json!(
            audit
                .metadata_verified
                .iter()
                .map(|verified| serde_json::json!({"rule": verified.case.rule.0, "name": verified.case.name.0, "sha256": verified.digest.0}))
                .collect::<Vec<_>>()
        );
    }
    if !audit.tool_contract_differences.is_empty() {
        result["tool_contract_difference_case_ids"] = serde_json::json!(
            audit
                .tool_contract_differences
                .iter()
                .map(case)
                .collect::<Vec<_>>()
        );
    }
    let mut bytes = serde_json::to_vec(&result).map_err(|error| Error::Json(Box::new(error)))?;
    bytes.push(b'\n');
    Ok(CapturedBytes(bytes))
}

/// One result row as the public summary and the per-rule result file write it.
#[derive(Serialize)]
struct RowSchema {
    rule_id: String,
    status: &'static str,
    reason: Option<String>,
    instance: Option<serde_json::Value>,
    case_results_sha256: Option<String>,
    case_count: usize,
    raw_exact: usize,
    documented_discrepancies: usize,
    contract_only: usize,
}

/// The atomic public inventory snapshot.
#[derive(Serialize)]
struct SummarySchema {
    schema: &'static str,
    provider: String,
    run_state: &'static str,
    identity: BTreeMap<String, serde_json::Value>,
    counts: BTreeMap<&'static str, usize>,
    rules: Vec<RowSchema>,
}

/// The JSON value a canonical JSON text spells.
fn value(text: &CanonicalJson) -> Result<serde_json::Value, Error> {
    serde_json::from_str(&text.0).map_err(|error| Error::Json(Box::new(error)))
}

/// The wire shape of a row.
fn row_schema(row: &Row) -> Result<RowSchema, Error> {
    Ok(RowSchema {
        rule_id: row.rule_id.0.clone(),
        status: match row.status {
            RowStatus::Pass => PASS,
            RowStatus::Fail => FAIL,
        },
        reason: match &row.reason {
            RowReason::Failed(reason) => Some(reason.0.clone()),
            RowReason::Passed => None,
        },
        instance: match &row.instance {
            RowInstance::Carried(text) => Some(value(text)?),
            RowInstance::Absent => None,
        },
        case_results_sha256: match &row.case_results_sha256 {
            ReceiptDigest::Recorded(digest) => Some(digest.0.clone()),
            ReceiptDigest::Unrecorded => None,
        },
        case_count: row.case_count.0,
        raw_exact: row.raw_exact.0,
        documented_discrepancies: row.documented_discrepancies.0,
        contract_only: row.contract_only.0,
    })
}

/// Encode one result row, pretty printed.
fn row(row: &Row) -> Result<CapturedBytes, Error> {
    serde_json::to_vec_pretty(&row_schema(row)?)
        .map(CapturedBytes)
        .map_err(|error| Error::Json(Box::new(error)))
}

/// Encode a whole summary, pretty printed, with its final newline.
fn summary(summary: &Summary) -> Result<CapturedBytes, Error> {
    let wire = SummarySchema {
        schema: BENCHMARK_INVENTORY_V1,
        provider: summary.provider.0.clone(),
        run_state: match summary.run_state {
            RunState::Running => RUNNING,
            RunState::Complete => COMPLETE,
            RunState::Failed => FAILED,
        },
        identity: summary
            .identity
            .iter()
            .map(|(key, text)| Ok((key.0.clone(), value(text)?)))
            .collect::<Result<_, Error>>()?,
        counts: BTreeMap::from([(PASS, summary.passed.0), (FAIL, summary.failed.0)]),
        rules: summary
            .rules
            .iter()
            .map(row_schema)
            .collect::<Result<_, _>>()?,
    };
    let mut bytes =
        serde_json::to_vec_pretty(&wire).map_err(|error| Error::Json(Box::new(error)))?;
    bytes.extend_from_slice(constants::NEWLINE);
    Ok(CapturedBytes(bytes))
}

impl TryFrom<&ReceiptAudit> for AuditJson {
    type Error = Error;

    fn try_from(value: &ReceiptAudit) -> Result<Self, Error> {
        audit(value).map(Self)
    }
}

impl TryFrom<&Row> for RowJson {
    type Error = Error;

    fn try_from(value: &Row) -> Result<Self, Error> {
        row(value).map(Self)
    }
}

impl TryFrom<&Summary> for SummaryJson {
    type Error = Error;

    fn try_from(value: &Summary) -> Result<Self, Error> {
        summary(value).map(Self)
    }
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
