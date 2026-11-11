//! A sink failure or a fused semantic refusal stays sticky while the kernel continues and reports success.
use super::super::super::super::super::service::*;
use super::super::super::super::super::*;
use super::super::super::cases::fixtures::table_of;
use super::super::super::*;
use super::super::fixtures::refusal;
use super::fixtures::KeepsCalling;
use crate::application::file_dag::program::facade_runtime::file_execution::service_test::fixtures::*;
use crate::cases::{Case, CaseName, Expect, Word, run};
#[cfg(test)]
use fixtures::execution::*;
#[cfg(test)]
use fixtures::provider::*;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{CellKind, Integer, Value as Cell};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input as PlanInput, RepeatedRows, Side, StepId, StepScope},
    plans::types::{Operation, PlanStep},
};
use std::sync::Arc;
fn sink_failure_wins_after_a_kernel_commits_multiple_rows_and_reports_success() {
    let left = table_of(&[CellKind::Number], &[&[Cell::Number(Integer(7))]]);
    let right = table_of(
        &[CellKind::Number, CellKind::Number],
        &[&[Cell::Number(Integer(7)), Cell::Number(Integer(1))]],
    );
    let actual = invoke(
        &operation(&[0], &[0], &[(Side::Right, 1)], vec![], &[]),
        [Rows::Computed(&left), Rows::Computed(&right)],
        &mut Symbols::default(),
        b"",
        true,
        Arc::new(KeepsCalling {
            refuse_project: false,
        }),
        Faults {
            sink_after: Some(0),
            ..Faults::default()
        },
    )
    .unwrap();
    assert!(matches!(actual.result, Err(OperatingError::NotRun)));
    assert!(actual.prefix.is_empty());
}
fn fused_semantic_refusal_stays_sticky_when_the_kernel_continues_and_reports_success() {
    let left_table = table_of(&[CellKind::Number], &[&[Cell::Number(Integer(7))]]);
    let right_table = table_of(
        &[CellKind::Number; 2],
        &[&[Cell::Number(Integer(7)), Cell::Number(Integer(1))]],
    );
    let mut steps = vec![];
    let (left, l) = provider(Rows::Computed(&left_table), 0, &mut steps);
    let (right, r) = provider(Rows::Computed(&right_table), 1, &mut steps);
    let join = steps.len() as u32;
    let mut op = operation(&[0], &[0], &[(Side::Right, 1)], vec![], &[]);
    let Operation::Join {
        left: from,
        right: other,
        ..
    } = &mut op
    else {
        panic!("the fixture builds a Join")
    };
    *from = l;
    *other = r;
    steps.push(PlanStep {
        scope: StepScope::PerFile,
        operation: op,
    });
    steps.push(PlanStep {
        scope: StepScope::PerFile,
        operation: Operation::Project {
            from: PlanInput::Step(StepId(join)),
            columns: vec![ColumnIndex(0)],
            distinct: RepeatedRows::Kept,
        },
    });
    let mut request = plan(Scenario::ProjectRetry);
    request.emit.step = StepId((steps.len() - 1) as u32);
    request.steps = steps;
    let actual = execute(
        request,
        [left, right],
        &mut Symbols::default(),
        b"",
        true,
        Arc::new(KeepsCalling {
            refuse_project: true,
        }),
        Faults::default(),
    )
    .unwrap();
    assert_eq!(
        actual.instructions, 2,
        "the failure must occur in the real fused Join→Project continuation"
    );
    refusal(actual.result);
    assert!(
        actual.prefix.is_empty(),
        "failed semantic publication delivers no prefix"
    );
}

/// The case each row runs: one of the proofs below.
#[derive(Clone, Copy, Debug)]
enum Proof {
    SinkFailureWinsAfterAKernelCommitsMultipleRowsAndReportsSuccess,
    FusedSemanticRefusalStaysStickyWhenTheKernelContinuesAndReportsSuccess,
}

type Input = Proof;
type Value = ();
type Failure = String;

/// One row per proof: its name, and the scenario that runs it.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "sink_failure_wins_after_a_kernel_commits_multiple_rows_and_reports_success",
        ),
        word: Word::Succeeds,
        input: || Proof::SinkFailureWinsAfterAKernelCommitsMultipleRowsAndReportsSuccess,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(
            "fused_semantic_refusal_stays_sticky_when_the_kernel_continues_and_reports_success",
        ),
        word: Word::Succeeds,
        input: || Proof::FusedSemanticRefusalStaysStickyWhenTheKernelContinuesAndReportsSuccess,
        expect: Expect::Value(()),
    },
];

/// Runs one scenario; its assertions are the case's expectation.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Proof::SinkFailureWinsAfterAKernelCommitsMultipleRowsAndReportsSuccess => {
            sink_failure_wins_after_a_kernel_commits_multiple_rows_and_reports_success();
            Ok(())
        }
        Proof::FusedSemanticRefusalStaysStickyWhenTheKernelContinuesAndReportsSuccess => {
            fused_semantic_refusal_stays_sticky_when_the_kernel_continues_and_reports_success();
            Ok(())
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
