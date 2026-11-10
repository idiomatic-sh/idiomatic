//! S2: a schema-first operation refuses its schema before any input's physical admission.
use crate::boundary_fixture::{authored_request, engine, source};
use crate::file_intents::answered;
use ryo_execution::inbound::use_cases::execution::interface::{
    OperatingError, PreparedExecutionProgram, RootOutcome,
};
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase as _;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input, RepeatedRows, StepScope};
use ryo_values::domain::plans::types::{Operation, PlanStep};
use ryo_values::domain::rows::errors::RowsError;
use ryo_values::domain::rows::types::RowCount;

/// A projection of `column` of Function.
fn projection(column: u32) -> PreparedExecutionProgram {
    let mut input = authored_request(1);
    crate::boundary_fixture::plans(&mut input)[0].steps = vec![PlanStep {
        scope: StepScope::PerFile,
        operation: Operation::Project {
            from: Input::Relation(Relation::Function),
            columns: vec![ColumnIndex(column)],
            distinct: RepeatedRows::Kept,
        },
    }];
    engine().prepare(input).expect("prepare a projection")
}

/// The answer of `program` when Function's rows say they are two but hold one: a geometry the input's admission refuses.
fn over_malformed_rows(program: &PreparedExecutionProgram) -> RootOutcome {
    let mut malformed = source();
    malformed.count = RowCount(2);
    answered(program, &[(Relation::Function, malformed)], &[], &[])
        .0
        .expect("the rule answers")
}

#[test]
fn a_schema_error_is_refused_before_an_input_physical_refusal() {
    assert!(
        matches!(
            over_malformed_rows(&projection(0)),
            RootOutcome::Failed(OperatingError::ShapeMismatch)
        ),
        "a valid projection meets the malformed input's own refusal"
    );
    assert!(
        matches!(
            over_malformed_rows(&projection(99)),
            RootOutcome::Failed(OperatingError::Table(RowsError::NoSuchColumn { index: 99 }))
        ),
        "the projection's own schema error comes first"
    );
}
