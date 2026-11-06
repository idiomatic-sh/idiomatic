//! Compiled tree and step joins and native provenance keep the same canonical semantic refusal.
use super::super::super::cases::test::{node, tree_rows, wide_tree};
use super::super::super::*;
use super::super::fixtures::{Witness, refusal};
use std::sync::atomic::Ordering;
#[test]
fn compiled_tree_and_step_joins_cannot_bypass_the_same_semantic_refusal() {
    for right in [
        PlanInput::Relation(Relation::SyntaxTree),
        PlanInput::Relation(Relation::SyntaxChild),
        PlanInput::Step(StepId(1)),
    ] {
        for refuse in [false, true] {
            let (elements, mut symbols) = wide_tree(1, true);
            let select = || PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Select {
                    from: PlanInput::Relation(Relation::SyntaxTree),
                    comparisons: vec![],
                    kept: vec![ColumnIndex(0)],
                },
            };
            let mut request = plan(Scenario::ProjectRetry);
            request.steps = vec![select()];
            if matches!(right, PlanInput::Step(_)) {
                request.steps.push(select());
            }
            let target = StepId(request.steps.len() as u32);
            let mut join = operation(
                &[0],
                &[if right == PlanInput::Relation(Relation::SyntaxChild) {
                    1
                } else {
                    0
                }],
                &[(Side::Left, 0)],
                vec![],
                &[],
            );
            let Operation::Join {
                left, right: rhs, ..
            } = &mut join
            else {
                unreachable!()
            };
            *left = PlanInput::Step(StepId(0));
            *rhs = right;
            request.steps.push(PlanStep {
                scope: StepScope::PerFile,
                operation: join,
            });
            request.emit.step = target;
            let (left, _) = provider(tree_rows(&elements, TreeForm::Elements), 0, &mut vec![]);
            let (other, _) = provider(tree_rows(&elements, TreeForm::Edges), 1, &mut vec![]);
            let witness = Arc::new(Witness {
                refuse,
                ..Witness::default()
            });
            let actual = execute(
                request,
                [left, other],
                &mut symbols,
                b"abcd",
                false,
                witness.clone(),
                Faults::default(),
            )
            .unwrap();
            assert!(
                actual.native_prepared,
                "tree/step case must install the real compiled native strategy"
            );
            assert_eq!(actual.work.native_positions, usize::from(!refuse));
            assert_eq!(
                witness.positions.load(Ordering::Relaxed),
                if refuse { 1 } else { 4 },
                "one semantic call per authored left tree position, stopping at the first refusal"
            );
            assert_eq!(witness.columns.load(Ordering::Relaxed), 0);
            if refuse {
                refusal(actual.result);
            } else {
                let expected = if right == PlanInput::Relation(Relation::SyntaxChild) {
                    vec![node(1), node(2), node(3)]
                } else {
                    vec![node(0), node(1), node(2), node(3)]
                };
                assert_eq!(actual.result.unwrap().cells, expected);
            }
        }
    }
}
#[test]
fn native_provenance_preserves_canonical_left_then_right_refusal_even_when_left_is_empty() {
    use ryo_values::domain::plan_parts::types::{
        Comparison, LiteralText, Operand, QueryLiteral, Test,
    };
    for (left_fails, right_fails, error) in [
        (false, true, Some(OperatingError::NotRead)),
        (true, true, Some(OperatingError::NotRun)),
        (true, false, Some(OperatingError::NotRun)),
        (false, false, None),
    ] {
        let (elements, mut symbols) = wide_tree(1, true);
        let mut request = plan(Scenario::ProjectRetry);
        request.steps = vec![PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Select {
                from: PlanInput::Relation(Relation::SyntaxTree),
                kept: vec![ColumnIndex(0)],
                comparisons: vec![Comparison {
                    test: Test::Equal,
                    left: Operand::Column(ColumnIndex(1)),
                    right: Operand::Literal(QueryLiteral::Text(LiteralText("absent-kind".into()))),
                    origin: None,
                }],
            },
        }];
        let mut join = operation(&[0], &[1], &[(Side::Left, 0)], vec![], &[]);
        let Operation::Join { left, right, .. } = &mut join else {
            unreachable!()
        };
        *left = PlanInput::Step(StepId(0));
        *right = PlanInput::Relation(Relation::SyntaxChild);
        request.steps.push(PlanStep {
            scope: StepScope::PerFile,
            operation: join,
        });
        request.emit.step = StepId(1);
        let (left, _) = provider(tree_rows(&elements, TreeForm::Elements), 0, &mut vec![]);
        let (right, _) = provider(tree_rows(&elements, TreeForm::Edges), 1, &mut vec![]);
        let actual = execute(
            request,
            [left, right],
            &mut symbols,
            b"abcd",
            true,
            real_port(),
            Faults {
                left: left_fails.then_some(OperatingError::NotRun),
                right: right_fails.then_some(OperatingError::NotRead),
                ..Faults::default()
            },
        )
        .unwrap();
        assert!(
            actual.native_prepared,
            "the owner must compile actual native provenance"
        );
        if left_fails {
            assert_eq!(
                actual.attempts,
                [SourceAttempt::Refused(0, OperatingError::NotRun)]
            );
            assert_eq!(actual.reads, [1, 0]);
        } else if right_fails {
            let (last, before) = actual
                .attempts
                .split_last()
                .expect("RHS refusal is observed");
            assert_eq!(last, &SourceAttempt::Refused(1, OperatingError::NotRead));
            assert!(!before.is_empty());
            assert!(
                before
                    .iter()
                    .all(|attempt| *attempt == SourceAttempt::Available(0)),
                "every prior borrow is a successful LHS read; no source is retried after the refusal"
            );
            assert_eq!(actual.reads[1], 1);
        } else {
            assert_eq!(actual.attempts.first(), Some(&SourceAttempt::Available(0)));
            assert!(actual.attempts.contains(&SourceAttempt::Available(1)));
            assert!(
                actual
                    .attempts
                    .iter()
                    .all(|attempt| matches!(attempt, SourceAttempt::Available(_)))
            );
        }
        match error {
            Some(error) => assert_eq!(actual.result, Err(error)),
            None => assert!(actual.result.unwrap().cells.is_empty()),
        }
        assert!(actual.prefix.is_empty());
    }
}
