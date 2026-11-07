//! A sink failure or a fused semantic refusal stays sticky while the kernel continues and reports success.
use super::super::super::cases::test::table_of;
use super::super::super::*;
use super::super::fixtures::{REFUSAL, refusal};
use ryo_operators::domain::equality_join::types::JoinInput;
use ryo_operators::domain::contract::types::OperatorAnswer;
use ryo_operators::domain::contract::types::OperatorCall;
use ryo_operators::inbound::use_cases::operators::interface::OperatorsUseCase;
use ryo_operators::domain::contract::types::PortableOperatorFault;
#[derive(Debug)]
struct KeepsCalling {
    refuse_project: bool,
}
impl OperatorsUseCase for KeepsCalling {
    fn operators(&self, call: OperatorCall<'_>) -> Result<OperatorAnswer, PortableOperatorFault> {
        match call {
            OperatorCall::Project { .. } if self.refuse_project => {
                Err(PortableOperatorFault::NoSuchColumn { index: REFUSAL })
            }
            OperatorCall::EqualityJoin {
                input: JoinInput::Values { driver, .. },
            } => {
                if self.refuse_project {
                    assert!(driver.emit(&[Cell::Number(Integer(1))]).is_err());
                    assert!(driver.emit(&[Cell::Number(Integer(2))]).is_err());
                    assert!(driver.next(&[Cell::Number(Integer(7))]).is_err());
                } else {
                    // External delivery begins only after this semantic result
                    // commits; it cannot interrupt this driver retroactively.
                    assert!(driver.emit(&[Cell::Number(Integer(1))]).is_ok());
                    assert!(driver.emit(&[Cell::Number(Integer(2))]).is_ok());
                }
                Ok(OperatorAnswer::Written)
            }
            call => {
                ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new()
                    .operators(call)
            }
        }
    }
}
#[test]
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
#[test]
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
        unreachable!()
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
