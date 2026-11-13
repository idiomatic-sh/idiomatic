//! Real native Axis failures must not leak partially evaluated held results.
use crate::*;
use ryo_language::domain::{
    builders::fixtures as builders, completion::fixtures as completion, elements::types::Elements,
    symbols::types::ByteSlice,
};
use ryo_relations::{domain::relations::types::KindBits, domain::terms::types::NameText};
use ryo_values::domain::plan_parts::types::LiteralText;
use ryo_values::domain::plan_parts::types::{
    Direction, Operand, PlanComparison, QueryLiteral, Test,
};

fn native_tree(bytes: &[u8]) -> (Elements, Symbols, Position, [Position; 2]) {
    let mut elements = Elements::default();
    let mut symbols = Symbols::default();
    let (root, first, second);
    {
        let mut builder = builders::table_builder(&mut elements, &mut symbols, ByteSlice(bytes));
        root = builders::builder_position(&mut builder);
        builders::enter_node(&mut builder, KindBits(0), Offset(0));
        first = builders::builder_position(&mut builder);
        builders::token(&mut builder, KindBits(1), Offset(0), Offset(1));
        second = builders::builder_position(&mut builder);
        builders::token(
            &mut builder,
            KindBits(1),
            Offset(2),
            Offset(u32::try_from(bytes.len() - 1).unwrap()),
        );
        builders::leave_node(&mut builder, Offset(u32::try_from(bytes.len()).unwrap()));
    }
    let mut completion =
        completion::completion(&mut elements, &mut symbols, ByteSlice(bytes), FileId(0));
    completion::name_kind(&mut completion, KindBits(0), NameText("root"));
    completion::name_kind(&mut completion, KindBits(1), NameText("token"));
    (elements, symbols, root, [first, second])
}
fn filter() -> PlanComparison {
    PlanComparison {
        test: Test::Matches,
        left: Operand::Literal(QueryLiteral::Text(LiteralText("x".into()))),
        right: Operand::Column(ColumnIndex(2)),
        origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
    }
}

#[test]
fn held_axis_buffers_late_native_predicate_failure_and_commits_success_in_order() {
    for (bytes, fails) in [
        (
            b"x
'['
"
            .as_slice(),
            true,
        ),
        (
            b"x
x
"
            .as_slice(),
            false,
        ),
    ] {
        let engine = engine();
        let mut input = authored_request(1);
        boundary_fixture::fixtures::plans(&mut input)[0].steps = vec![PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Axis {
                from: Input::Relation(Relation::Function),
                edges: Input::Relation(Relation::SyntaxChild),
                domain: Input::Relation(Relation::SyntaxTree),
                direction: Direction::Child,
                id: ColumnIndex(0),
                parent: ColumnIndex(0),
                child: ColumnIndex(1),
                domain_id: ColumnIndex(0),
                domain_filter: vec![filter()],
                domain_view: vec![],
                output: vec![],
            },
        }];
        boundary_fixture::fixtures::plans(&mut input)[0].emit.step = StepId(0);
        let prepared = engine.prepare(input).expect("prepare real native Axis");
        let corpus = corpus();
        let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus, None.into());
        let (elements, mut symbols, root, children) = native_tree(bytes);
        let mut roots = source();
        roots.cells[0] = Value::Node(node_id(FileId(0), root));
        let facts = TableFacts::new(&[(Relation::Function, &roots)]);
        let mut rules = run_file(
            &request,
            FileRun {
                source: bytes,
                ..FileRun::of(
                    &[FileRuleIntent::Run {
                        diagnostic: DiagnosticDelivery::Collected,
                        observations: ObservationDemand::NotRequired,
                        capture: CaptureIntent::DiagnosticAndObservations,
                    }],
                    &elements,
                )
            },
            &mut symbols,
            &facts,
        );
        let result = boundary_fixture::fixtures::root(ran(&mut rules, 0));
        if fails {
            assert!(
                matches!(
                    result,
                    Ok(RootOutcome::Failed(OperatingError::InvalidRegex { .. }))
                ),
                "a later Axis predicate failure discards the whole held batch"
            );
        } else {
            let Ok(RootOutcome::Complete(table)) = result else {
                panic!("valid dynamic patterns commit")
            };
            let exposed = table
                .cells
                .chunks(table.kinds.len())
                .map(<[Value]>::to_vec)
                .collect::<Vec<_>>();
            let expected = children.map(|child| {
                let mut row = roots.cells.clone();
                row.push(Value::Node(node_id(FileId(0), child)));
                row
            });
            assert_eq!(exposed, expected);
        }
        request.finish();
    }
}

#[test]
fn ordinary_select_does_not_publish_a_prefix_before_late_predicate_failure() {
    let bytes = b"x
'['
";
    let engine = engine();
    let mut input = authored_request(1);
    boundary_fixture::fixtures::plans(&mut input)[0].steps = vec![PlanStep {
        scope: StepScope::PerFile,
        operation: Operation::Select {
            from: Input::Relation(Relation::SyntaxTree),
            comparisons: vec![
                PlanComparison {
                    test: Test::Equal,
                    left: Operand::Column(ColumnIndex(1)),
                    right: Operand::Literal(QueryLiteral::Text(LiteralText("token".into()))),
                    origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
                },
                filter(),
            ],
            kept: vec![],
        },
    }];
    boundary_fixture::fixtures::plans(&mut input)[0].emit.step = StepId(0);
    let prepared = engine
        .prepare(input)
        .expect("prepare actual incremental Select");
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus, None.into());
    let (elements, mut symbols, _root, _children) = native_tree(bytes);
    let facts = TableFacts::new(&[]);
    let mut rules = run_file(
        &request,
        FileRun {
            source: bytes,
            ..FileRun::of(
                &[FileRuleIntent::Run {
                    diagnostic: DiagnosticDelivery::Collected,
                    observations: ObservationDemand::NotRequired,
                    capture: CaptureIntent::DiagnosticAndObservations,
                }],
                &elements,
            )
        },
        &mut symbols,
        &facts,
    );
    assert!(
        matches!(
            root(ran(&mut rules, 0)),
            Ok(RootOutcome::Failed(OperatingError::InvalidRegex { .. }))
        ),
        "semantic failure publishes no partial result"
    );
    request.finish();
}
