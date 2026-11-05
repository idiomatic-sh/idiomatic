//! The actual certification/measurement pipeline supplies every publication sample.
use crate::inbound::use_cases::certify::{
    fixtures::{Change, exercise},
    use_case::CertifyUseCaseImpl,
};
use ryo_benchmark_contracts::domain::workloads::types::case_count::value::CaseCount;
#[test]
fn full_inventory_publishes_and_diagnostic_pair_is_refused_by_the_real_public_operation() {
    for change in [Change::InventoryFull, Change::None] {
        assert_eq!(exercise(change, CertifyUseCaseImpl::new), Ok(CaseCount(2)));
    }
}
