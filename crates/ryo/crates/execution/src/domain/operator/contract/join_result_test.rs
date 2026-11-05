//! Error translation tests the production contract without an execution fixture.
use super::types::join_result;
use crate::domain::operating::errors::OperatingError;
use ryo_operators::inbound::use_cases::operators::interface::{
    OperatorAnswer, PortableOperatorFault,
};
use ryo_relations::domain::relations::types::{Integer, Value};
#[test]
fn original_engine_error_wins_over_every_inconsistent_kernel_terminal() {
    for answer in [
        Ok(OperatorAnswer::Written),
        Ok(OperatorAnswer::SpanBounds([Value::Number(Integer(1)); 2])),
        Err(PortableOperatorFault::InvalidSpan),
    ] {
        assert!(matches!(
            join_result(answer, Some(OperatingError::NotRead)),
            Err(OperatingError::NotRead)
        ));
    }
    assert!(matches!(
        join_result(Err(PortableOperatorFault::Interrupted), None),
        Err(OperatingError::WrongOperation)
    ));
    assert!(matches!(
        join_result(Err(PortableOperatorFault::InvalidSpan), None),
        Err(OperatingError::InvalidSpan)
    ));
}
