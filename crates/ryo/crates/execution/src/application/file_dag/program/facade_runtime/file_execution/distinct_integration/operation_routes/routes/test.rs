//! Every production route publishes the independent expected rows.
use super::super::super::*;
use super::super::fixtures::chain::execute_fixture;
use ryo_values::domain::query_operators::types::OperationKind;
#[test]
fn all26_actual_production_routes_publish_the_independent_expected_rows() {
    for &kind in OperationKind::ALL {
        // Each case uses the real normal scheduler for both an emitted root and
        // an authored downstream row consumer. Rows/Hosted remain request-owned sources.
        for downstream in [false, true] {
            execute_fixture(kind,downstream,Arc::new(ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new()))
                .unwrap_or_else(|error|panic!("compiled {kind:?}: {error:?}"));
        }
    }
}
