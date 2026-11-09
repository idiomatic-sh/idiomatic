//! S9: an operation that acquires its resources at its first row acquires none for an input with no row, so a resource it could
//! not acquire refuses only an input with a row.
use crate::boundary_fixture::{authored_request, engine};
use crate::file_intents::answered;
use ryo_execution::inbound::use_cases::execution::interface::{OperatingError, RootOutcome};
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase as _;
use ryo_values::domain::hosted::types::{HostColumn, HostColumnName, HostKind, HostName, HostRelation, HostRow};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, HostedNumber, Input, ParameterId, ParameterName, StepId, StepScope,
};
use ryo_values::domain::plans::types::{Operation, PlanStep};
use ryo_values::domain::query_bindings::types::{ParameterType, PlanParameter};
use ryo_values::domain::row_texts::types::Cell;
use ryo_values::domain::rows::types::RowCount;

/// The rank of the hosted names `names` by a sets parameter nothing binds.
fn unbound_rank(names: &[&str]) -> RootOutcome {
    let mut input = authored_request(1);
    input.hosted.push(HostRelation {
        name: HostName("names".into()),
        columns: vec![HostColumn {
            name: HostColumnName("name".into()),
            kind: HostKind::Text,
        }],
        rows: names
            .iter()
            .map(|name| HostRow(vec![Cell::Text((*name).into())]))
            .collect(),
    });
    let plan = &mut crate::boundary_fixture::plans(&mut input)[0];
    plan.parameters.push(PlanParameter {
        name: ParameterName("sets".into()),
        kind: ParameterType::TextSets,
    });
    plan.steps = vec![
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Hosted {
                relation: HostedNumber(0),
            },
        },
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Rank {
                from: Input::Step(StepId(0)),
                column: ColumnIndex(0),
                sets: ParameterId(0),
            },
        },
    ];
    plan.emit.step = StepId(1);
    let program = engine().prepare(input).expect("prepare a rank of hosted names");
    answered(&program, &[], &[], &[]).0.expect("the rule answers")
}

#[test]
fn a_first_row_resource_is_acquired_only_when_a_row_reaches_it() {
    let outcome = unbound_rank(&[]);
    let RootOutcome::Complete(table) = outcome else {
        panic!("no row reaches the rank, so its sets are never read")
    };
    assert_eq!(table.count, RowCount(0));
    assert!(
        matches!(unbound_rank(&["cli"]), RootOutcome::Failed(OperatingError::Unbound)),
        "a row reaches the rank, whose sets nothing binds"
    );
}
