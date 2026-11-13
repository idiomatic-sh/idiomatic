//! Both declared Union populations drive the same owner-held invocation.
use super::super::*;
use super::fixtures::program;

#[test]
fn union_consumes_both_populations_in_order_with_one_distinct_scope() {
    for (left, right, expected) in [
        (&[][..], &[2, 1, 2][..], &[2, 1][..]),
        (&[3, 2, 3][..], &[][..], &[3, 2][..]),
        (&[3, 2, 3][..], &[2, 1, 3, 1][..], &[3, 2, 1][..]),
        (&[][..], &[][..], &[][..]),
    ] {
        let engine = engine();
        let prepared = program(&engine, left, right);
        let corpus = corpus();
        let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus, None.into());
        let elements = ryo_language::domain::elements::types::Elements::default();
        let mut facts = TableFacts::new(&[]);
        let mut symbols = Symbols::default();
        let intents = [FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Collected,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::DiagnosticAndObservations,
        }];
        let mut rules = run_file(
            &request,
            FileRun::of(&intents, &elements),
            &mut symbols,
            &mut facts,
        );
        let Ok(RootOutcome::Complete(table)) = root(ran(&mut rules, 0)) else {
            panic!("complete Union output")
        };
        assert_eq!(table.kinds, [CellKind::Number]);
        assert_eq!(table.count, RowCount(expected.len()));
        let cells = table.cells;
        assert_eq!(
            cells,
            expected
                .iter()
                .map(|v| Value::Number(Integer(*v)))
                .collect::<Vec<_>>()
        );
        assert!(
            matches!(rules.end, FileEnd::Finished(_)),
            "release Union file"
        );
        assert_eq!(facts.reads(), 0);
        request.finish();
    }
}

#[test]
fn union_preserves_left_source_refusal_and_demands_right_after_empty_left() {
    for left_refuses in [true, false] {
        let engine = engine();
        let mut input = authored_request(1);
        boundary_fixture::fixtures::plans(&mut input)[0].steps = vec![
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Select {
                    from: Input::Relation(Relation::Function),
                    comparisons: vec![],
                    kept: vec![ColumnIndex(0)],
                },
            },
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Select {
                    from: Input::Relation(Relation::SemanticChild),
                    comparisons: vec![],
                    kept: vec![ColumnIndex(0)],
                },
            },
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Union {
                    left: Input::Step(StepId(0)),
                    right: Input::Step(StepId(1)),
                },
            },
        ];
        boundary_fixture::fixtures::plans(&mut input)[0].emit.step = StepId(2);
        let prepared = engine.prepare(input).expect("source-bound Union");
        let corpus = corpus();
        let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus, None.into());
        let elements = ryo_language::domain::elements::types::Elements::default();
        let empty = Table {
            kinds: kinds(Relation::Function).to_vec(),
            count: RowCount(0),
            cells: vec![],
        };
        // The right side, SemanticChild, is never held, so a read of it is refused; the left, Function, is refused too when
        // `left_refuses`, and else holds no row.
        let mut facts = if left_refuses {
            TableFacts::new(&[])
        } else {
            TableFacts::new(&[(Relation::Function, &empty)])
        };
        let mut symbols = Symbols::default();
        let mut rules = run_file(
            &request,
            FileRun::of(
                &[FileRuleIntent::Run {
                    diagnostic: DiagnosticDelivery::Collected,
                    observations: ObservationDemand::NotRequired,
                    capture: CaptureIntent::DiagnosticAndObservations,
                }],
                &elements,
            ),
            &mut symbols,
            &mut facts,
        );
        assert!(
            matches!(root(ran(&mut rules, 0)), Err(FileStop::Facts(_))),
            "the refused source stops the rule"
        );
        let order = facts.history();
        let left_reads = order
            .iter()
            .filter(|relation| **relation == Relation::Function)
            .count();
        let right_reads = order
            .iter()
            .filter(|relation| **relation == Relation::SemanticChild)
            .count();
        if left_refuses {
            assert_eq!(left_reads, 1, "a failed source is not retried");
            assert_eq!(order.as_slice(), &[Relation::Function]);
        } else {
            // Accepted publications may resolve the pinned borrowed view more than once. The file's presence
            // read is admission's first read, so the right refusal may come before admission's left borrows, and
            // admission meets it without reading the right side again.
            assert!(left_reads > 0);
            assert_eq!(order.first(), Some(&Relation::Function));
            assert!(
                order.iter().all(|relation| matches!(
                    relation,
                    Relation::Function | Relation::SemanticChild
                ))
            );
        }
        assert_eq!(
            right_reads,
            usize::from(!left_refuses),
            "left failure wins; empty success still demands the right side exactly once"
        );
        assert!(matches!(rules.end, FileEnd::Unfinished));
        assert_eq!(
            facts.reads(),
            order.len(),
            "release never demands a failed source again"
        );
        request.finish();
    }
}
