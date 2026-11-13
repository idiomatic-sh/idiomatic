//! S2: a schema-first operation refuses its schema before any input's physical admission.
use crate::boundary_fixture::fixtures::{authored_request, engine, source};
use crate::file_intents::fixtures::answered;
use ryo_execution::inbound::use_cases::execution::interface::{
    OperatingError, PreparedExecutionProgram, RootOutcome,
};
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase as _;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input, RepeatedRows, StepScope};
use ryo_values::domain::plans::types::{Operation, PlanStep};
use ryo_values::domain::rows::errors::RowsError;

/// A projection of `column` of Function.
fn projection(column: u32) -> PreparedExecutionProgram {
    let mut input = authored_request(1);
    crate::boundary_fixture::fixtures::plans(&mut input)[0].steps = vec![PlanStep {
        scope: StepScope::PerFile,
        operation: Operation::Project {
            from: Input::Relation(Relation::Function),
            columns: vec![ColumnIndex(column)],
            distinct: RepeatedRows::Kept,
        },
    }];
    engine().prepare(input).expect("prepare a projection")
}

/// The answer of `program` when Function's last column holds one row fewer than the others: the one malformed geometry a
/// worker's columns can hold (a row count that disagrees with its cells exists only for a computed table, which the per-file
/// route never reads).
fn over_malformed_rows(program: &PreparedExecutionProgram) -> RootOutcome {
    let mut malformed = source();
    let _ragged = malformed.cells.pop();
    answered(program, &[(Relation::Function, malformed)], &[], &[])
        .0
        .expect("the rule answers")
}

#[test]
fn a_schema_error_is_refused_before_an_input_physical_refusal() {
    assert!(
        matches!(
            over_malformed_rows(&projection(99)),
            RootOutcome::Failed(OperatingError::Table(RowsError::NoSuchColumn { index: 99 }))
        ),
        "the projection's own schema error comes first"
    );
}
