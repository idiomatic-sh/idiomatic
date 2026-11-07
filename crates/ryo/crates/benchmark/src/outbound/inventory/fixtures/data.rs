//! The inputs the contract's rows start from.
use crate::outbound::inventory::types::{
    IdentityKey, ProviderName, ReceiptDigest, Row, RowInstance, RowReason, RowStatus, RunState,
    Summary,
};
use ryo_benchmark_contracts::domain::documents::types::text::value::{ReasonText, RuleName};
use ryo_benchmark_contracts::domain::workloads::types::case_count::value::CaseCount;
use std::collections::BTreeMap;

/// One passing row, the way the application builds it.
pub fn row() -> Row {
    Row {
        rule_id: RuleName("rule".into()),
        status: RowStatus::Fail,
        reason: RowReason::Failed(ReasonText("missing".into())),
        instance: RowInstance::Absent,
        case_results_sha256: ReceiptDigest::Unrecorded,
        case_count: CaseCount(0),
        raw_exact: CaseCount(0),
        documented_discrepancies: CaseCount(0),
        contract_only: CaseCount(0),
    }
}

/// An empty running summary.
pub fn summary() -> Summary {
    Summary {
        provider: ProviderName("provider".into()),
        run_state: RunState::Running,
        identity: BTreeMap::<IdentityKey, _>::new(),
        passed: CaseCount(0),
        failed: CaseCount(1),
        rules: vec![row()],
    }
}

