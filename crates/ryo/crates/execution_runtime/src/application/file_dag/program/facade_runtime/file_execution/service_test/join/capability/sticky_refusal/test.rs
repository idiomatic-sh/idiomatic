//! A sink failure or a fused semantic refusal stays sticky while the kernel continues and reports success.
use super::super::super::cases::fixtures::table_of;
use super::super::super::*;
use super::super::fixtures::REFUSAL;
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
/// Where a join of `7` with `(7, 1)` fails while its kernel keeps calling and reports success.
#[derive(Clone, Copy, Debug)]
enum Failing {
    /// The sink refuses the first row the join commits.
    Sink,
    /// The fused project after the join refuses its row in the operator semantics.
    FusedProject,
    /// Nothing refuses: the sink takes the rows the join commits.
    Nothing,
}

/// A refused run: its error, the rows delivered before it, and the instructions it ran in.
#[derive(Debug, PartialEq)]
struct Refused {
    error: OperatingError,
    delivered: usize,
    instructions: usize,
}

type Input = Failing;
/// The rows a run published.
type Value = usize;
type Failure = Refused;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_join_whose_sink_takes_its_rows_publishes_them"),
        word: Word::Succeeds,
        input: || Failing::Nothing,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName(
            "sink_failure_wins_after_a_kernel_commits_multiple_rows_and_reports_success",
        ),
        word: Word::Refuses,
        input: || Failing::Sink,
        expect: Expect::Error(Refused {
            error: OperatingError::NotRun,
            delivered: 0,
            instructions: 1,
        }),
    },
    Case {
        // The failure occurs in the real fused Join→Project continuation, and a failed semantic
        // publication delivers no prefix.
        name: CaseName(
            "fused_semantic_refusal_stays_sticky_when_the_kernel_continues_and_reports_success",
        ),
        word: Word::Refuses,
        input: || Failing::FusedProject,
        expect: Expect::Error(Refused {
            error: OperatingError::Table(
                ryo_values::domain::rows::errors::RowsError::NoSuchColumn { index: REFUSAL },
            ),
            delivered: 0,
            instructions: 2,
        }),
    },
];

/// Runs the join with the failing part.
fn subject(failing: Input) -> Result<Value, Failure> {
    let left_table = table_of(&[CellKind::Number], &[&[Cell::Number(Integer(7))]]);
    let right_table = table_of(
        &[CellKind::Number; 2],
        &[&[Cell::Number(Integer(7)), Cell::Number(Integer(1))]],
    );
    let refused = |error| Refused {
        error,
        delivered: 0,
        instructions: 0,
    };
    let actual = match failing {
        Failing::Sink | Failing::Nothing => invoke(
            &operation(&[0], &[0], &[(Side::Right, 1)], vec![], &[]),
            [Rows::Computed(&left_table), Rows::Computed(&right_table)],
            &mut Symbols::default(),
            b"",
            true,
            Arc::new(KeepsCalling {
                refuse_project: false,
            }),
            Faults {
                sink_after: matches!(failing, Failing::Sink).then_some(0),
                ..Faults::default()
            },
        )
        .map_err(refused)?,
        Failing::FusedProject => {
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
            let mut request = plan(PlanShape::ProjectRetry);
            request.emit.step = StepId((steps.len() - 1) as u32);
            request.steps = steps;
            execute(
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
            .map_err(refused)?
        }
    };
    let delivered = actual.prefix.len();
    let instructions = actual.instructions;
    actual
        .result
        .map(|table| table.count.0)
        .map_err(|error| Refused {
            error,
            delivered,
            instructions,
        })
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
