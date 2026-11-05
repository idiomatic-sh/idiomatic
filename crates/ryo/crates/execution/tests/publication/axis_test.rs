//! Real native Axis failures must not leak partially evaluated held results.
use super::*;
use ryo_language::domain::{
    completion::service as completion,
    elements::{service as elements, types::Elements},
    symbols::types::ByteSlice,
};
use ryo_relations::{domain::relations::types::KindBits, domain::terms::types::NameText};
use ryo_values::domain::plan_parts::types::{Comparison, Direction, Operand, QueryLiteral, Test};

fn native_tree(bytes: &[u8]) -> (Elements, Symbols, Position, [Position; 2]) {
    let mut elements = Elements::default();
    let mut symbols = Symbols::default();
    let (root, first, second);
    {
        let mut builder = elements::table_builder(&mut elements, &mut symbols, ByteSlice(bytes));
        root = elements::builder_position(&mut builder);
        elements::enter_node(&mut builder, KindBits(0), Offset(0));
        first = elements::builder_position(&mut builder);
        elements::token(&mut builder, KindBits(1), Offset(0), Offset(1));
        second = elements::builder_position(&mut builder);
        elements::token(
            &mut builder,
            KindBits(1),
            Offset(2),
            Offset(u32::try_from(bytes.len() - 1).unwrap()),
        );
        elements::leave_node(&mut builder, Offset(u32::try_from(bytes.len()).unwrap()));
    }
    let mut completion =
        completion::completion(&mut elements, &mut symbols, ByteSlice(bytes), FileId(0));
    completion::name_kind(&mut completion, KindBits(0), NameText("root"));
    completion::name_kind(&mut completion, KindBits(1), NameText("token"));
    (elements, symbols, root, [first, second])
}
fn filter() -> Comparison {
    Comparison {
        test: Test::Matches,
        left: Operand::Literal(QueryLiteral::Text(LiteralText("x".into()))),
        right: Operand::Column(ColumnIndex(2)),
        origin: None,
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
        input.plans[0].steps = vec![PlanStep {
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
        input.plans[0].emit.step = StepId(0);
        let prepared = engine.prepare(input).expect("prepare real native Axis");
        let corpus = corpus();
        let request = engine.begin(&prepared, &corpus);
        let (elements, mut symbols, root, children) = native_tree(bytes);
        let mut roots = source();
        roots.cells[0] = Value::Node(node_id(FileId(0), root));
        let read = |relation| {
            assert_eq!(relation, Relation::Function);
            Ok(SourceRows::Computed(&roots))
        };
        let mut file = request
            .open_file(
                FileInput {
                    language: SourceLanguage::Recognized,
                    resident_stability: facts::ResidentStability::Stable,
                    file: FileId(0),
                    evaluation: &[],
                    publication: &[],
                    intents: &[FileRuleIntent::Run {
                        roots: RuleDemand {
                            emit: EmitDemand::Enabled,
                            observations: ObservationDemand::NotRequired,
                        },
                        capture: CaptureIntent::Disabled,
                    }],
                    source: bytes,
                    elements: &elements,
                    read: &read,
                    semantic: None,
                    kinds: None,
                },
                &mut symbols,
            )
            .expect("open native Axis file");
        let mut exposed = Vec::new();
        let result = file.emit_rows(RuleIndex(0), &mut symbols, &mut |row| {
            exposed.push(row.to_vec());
            Ok(())
        });
        if fails {
            assert!(matches!(result, Err(OperatingError::InvalidRegex { .. })));
            assert!(
                exposed.is_empty(),
                "a later Axis predicate failure discards the whole held batch"
            );
        } else {
            result.expect("valid dynamic patterns commit");
            let expected = children.map(|child| {
                let mut row = roots.cells.clone();
                row.push(Value::Node(node_id(FileId(0), child)));
                row
            });
            assert_eq!(exposed, expected);
        }
        file.finish().expect("release native Axis invocation");
        request.finish();
    }
}

#[test]
fn ordinary_incremental_select_preserves_prefix_before_late_predicate_failure() {
    let bytes = b"x
'['
";
    let engine = engine();
    let mut input = authored_request(1);
    input.plans[0].steps = vec![PlanStep {
        scope: StepScope::PerFile,
        operation: Operation::Select {
            from: Input::Relation(Relation::SyntaxTree),
            comparisons: vec![
                Comparison {
                    test: Test::Equal,
                    left: Operand::Column(ColumnIndex(1)),
                    right: Operand::Literal(QueryLiteral::Text(LiteralText("token".into()))),
                    origin: None,
                },
                filter(),
            ],
            kept: vec![],
        },
    }];
    input.plans[0].emit.step = StepId(0);
    let prepared = engine
        .prepare(input)
        .expect("prepare actual incremental Select");
    let corpus = corpus();
    let request = engine.begin(&prepared, &corpus);
    let (elements, mut symbols, _root, children) = native_tree(bytes);
    let read = |_| Err::<SourceRows<'_>, _>(OperatingError::NotRead);
    let mut file = request
        .open_file(
            FileInput {
                language: SourceLanguage::Recognized,
                resident_stability: facts::ResidentStability::Stable,
                file: FileId(0),
                evaluation: &[],
                publication: &[],
                intents: &[FileRuleIntent::Run {
                    roots: RuleDemand {
                        emit: EmitDemand::Enabled,
                        observations: ObservationDemand::NotRequired,
                    },
                    capture: CaptureIntent::Disabled,
                }],
                source: bytes,
                elements: &elements,
                read: &read,
                semantic: None,
                kinds: None,
            },
            &mut symbols,
        )
        .unwrap();
    let mut exposed = Vec::new();
    let result = file.emit_rows(RuleIndex(0), &mut symbols, &mut |row| {
        exposed.push(row.to_vec());
        Ok(())
    });
    assert!(matches!(result, Err(OperatingError::InvalidRegex { .. })));
    assert!(
        exposed
            .iter()
            .any(|row| row[0] == Value::Node(node_id(FileId(0), children[0]))),
        "incremental delivery retains the first valid child before the later refusal"
    );
    assert!(
        !exposed
            .iter()
            .any(|row| row[0] == Value::Node(node_id(FileId(0), children[1])))
    );
    file.finish().expect("release incremental refusal");
    request.finish();
}
