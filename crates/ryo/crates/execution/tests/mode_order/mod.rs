//! Declared schema/resource and transport ordering through the real public owner.
use super::*;
use ryo_relations::domain::relations::types::{ColumnsView, FactColumn};
use ryo_values::domain::{
    hosted::types::{HostColumn, HostColumnName, HostKind, HostName, HostRelation, HostRow},
    plan_parts::types::{Direction, HostedNumber},
    records::types::Cell as HostCell,
    rows::errors::TableModelError,
};

fn evaluate<'facts>(
    input: PreparedRequest,
    read: &dyn Fn(Relation) -> Result<SourceRows<'facts>, OperatingError>,
) -> RootOutcome {
    let engine = engine();
    let prepared = engine
        .prepare(input)
        .expect("prepare actual owner policy case");
    let corpus = corpus();
    let request = engine.begin(&prepared, &corpus);
    let elements = ryo_language::domain::elements::types::Elements::default();
    let mut symbols = Symbols::default();
    let mut file = request
        .open_file(
            FileInput {
                language: SourceLanguage::Recognized,
                resident_stability: facts::ResidentStability::Stable,
                file: FileId(0),
                evaluation: &[],
                publication: &[],
                demanded: &[true],
                source: b"abc",
                elements: &elements,
                read,
                semantic: None,
                kinds: None,
            },
            &mut symbols,
        )
        .expect("open actual owner policy file");
    let outcome = file
        .rule(RuleIndex(0), demand(), &mut symbols)
        .expect("obtain typed root outcome")
        .emit;
    file.finish().expect("release actual policy file");
    request.finish();
    outcome
}
fn step(operation: Operation) -> PlanStep {
    PlanStep {
        scope: StepScope::PerFile,
        operation,
    }
}
fn select(relation: Relation) -> Operation {
    Operation::Select {
        from: Input::Relation(relation),
        comparisons: vec![],
        kept: vec![ColumnIndex(0)],
    }
}
fn hosted(values: &[i64]) -> HostRelation {
    HostRelation {
        name: HostName("numbers".into()),
        columns: vec![HostColumn {
            name: HostColumnName("number".into()),
            kind: HostKind::Number,
        }],
        rows: values
            .iter()
            .map(|value| HostRow(vec![HostCell::Number(Integer(*value))]))
            .collect(),
    }
}
fn axis(from: Input, domain: Input, parent: u32) -> Operation {
    Operation::Axis {
        from,
        edges: Input::Relation(Relation::SemanticChild),
        domain,
        direction: Direction::Child,
        id: ColumnIndex(0),
        parent: ColumnIndex(parent),
        child: ColumnIndex(1),
        domain_id: ColumnIndex(99),
        domain_filter: vec![],
        domain_view: vec![],
        output: vec![],
    }
}
fn assert_column(outcome: RootOutcome, index: u32) {
    assert!(
        matches!(outcome, RootOutcome::Failed(OperatingError::Table(TableModelError::NoSuchColumn { index: actual })) if actual == index)
    );
}

#[test]
fn collected_reductions_validate_the_resource_operand_before_the_output_keys() {
    for digest in [false, true] {
        let mut input = authored_request(1);
        let from = Input::Step(StepId(0));
        input.plans[0].steps = vec![
            step(Operation::Hosted {
                relation: HostedNumber(0),
            }),
            step(if digest {
                Operation::GroupDigest {
                    from,
                    keys: vec![ColumnIndex(98)],
                    over: ColumnIndex(99),
                }
            } else {
                Operation::GroupCount {
                    from,
                    keys: vec![ColumnIndex(98)],
                    distinct: Some(ColumnIndex(99)),
                }
            }),
        ];
        input.plans[0].emit.step = StepId(1);
        input.hosted = vec![hosted(&[1])];
        let read = |_| Err::<SourceRows<'_>, _>(OperatingError::NotRead);
        assert_column(evaluate(input, &read), 99);
    }
}

#[test]
fn collected_schema_first_follows_actual_source_admission() {
    let mut input = authored_request(1);
    input.plans[0].steps = vec![step(Operation::Select {
        from: Input::Relation(Relation::Function),
        comparisons: vec![],
        kept: vec![ColumnIndex(99)],
    })];
    let reads = Cell::new(0);
    let read = |_| {
        reads.set(reads.get() + 1);
        Err::<SourceRows<'_>, _>(OperatingError::NotRead)
    };
    assert!(matches!(
        evaluate(input, &read),
        RootOutcome::Failed(OperatingError::NotRead)
    ));
    assert_eq!(reads.get(), 1);
}

#[test]
fn collected_antijoin_empty_left_precedes_unused_right_key_validation() {
    let mut input = authored_request(1);
    input.plans[0].steps = vec![
        step(Operation::Hosted {
            relation: HostedNumber(0),
        }),
        step(Operation::Hosted {
            relation: HostedNumber(1),
        }),
        step(Operation::Antijoin {
            left: Input::Step(StepId(0)),
            right: Input::Step(StepId(1)),
            left_on: vec![ColumnIndex(0)],
            right_on: vec![ColumnIndex(99)],
        }),
    ];
    input.plans[0].emit.step = StepId(2);
    input.hosted = vec![hosted(&[]), hosted(&[1])];
    let read = |_| Err::<SourceRows<'_>, _>(OperatingError::NotRead);
    let RootOutcome::Complete(table) = evaluate(input, &read) else {
        panic!("typed empty antijoin")
    };
    assert_eq!(table.kinds, [CellKind::Number]);
    assert_eq!(table.count, RowCount(0));
    assert!(table.cells.is_empty());
}

#[test]
fn collected_axis_empty_domain_gate_is_specific_to_computed_representation() {
    for computed in [true, false] {
        let mut input = authored_request(1);
        input.plans[0].steps = vec![
            step(select(Relation::Function)),
            step(axis(
                Input::Step(StepId(0)),
                Input::Relation(Relation::Defines),
                98,
            )),
        ];
        input.plans[0].emit.step = StepId(1);
        let roots = source();
        let edges = Table {
            kinds: kinds(Relation::SemanticChild).to_vec(),
            count: RowCount(0),
            cells: vec![],
        };
        let domain = Table {
            kinds: kinds(Relation::Defines).to_vec(),
            count: RowCount(0),
            cells: vec![],
        };
        let columns = kinds(Relation::Defines)
            .iter()
            .map(|_| FactColumn(vec![]))
            .collect::<Vec<_>>();
        let read = |relation| match relation {
            Relation::Function => Ok(SourceRows::Computed(&roots)),
            Relation::SemanticChild => Ok(SourceRows::Computed(&edges)),
            Relation::Defines if computed => Ok(SourceRows::Computed(&domain)),
            Relation::Defines => Ok(SourceRows::Stored(ColumnsView {
                kinds: kinds(Relation::Defines),
                columns: &columns,
            })),
            _ => Err(OperatingError::NotRead),
        };
        let outcome = evaluate(input, &read);
        if computed {
            let RootOutcome::Complete(table) = outcome else {
                panic!("computed empty domain precedes unused edge validation")
            };
            assert_eq!(table.kinds, [CellKind::Node, CellKind::Node]);
            assert_eq!(table.count, RowCount(0));
        } else {
            assert_column(outcome, 98);
        }
    }
}

#[test]
fn held_axis_empty_primary_skips_both_unread_sides_with_a_typed_layout() {
    let mut input = authored_request(1);
    input.plans[0].steps = vec![
        step(select(Relation::Function)),
        step(axis(
            Input::Step(StepId(0)),
            Input::Relation(Relation::Defines),
            98,
        )),
    ];
    input.plans[0].frontier = vec![Input::Step(StepId(0))];
    input.plans[0].emit.step = StepId(1);
    let empty = Table {
        kinds: kinds(Relation::Function).to_vec(),
        count: RowCount(0),
        cells: vec![],
    };
    let side_reads = Cell::new(0);
    let read = |relation| {
        if relation == Relation::Function {
            Ok(SourceRows::Computed(&empty))
        } else {
            side_reads.set(side_reads.get() + 1);
            Err(OperatingError::NotRead)
        }
    };
    let RootOutcome::Complete(table) = evaluate(input, &read) else {
        panic!("held empty roots have a declared empty shape")
    };
    assert_eq!(table.kinds, [CellKind::Node, CellKind::Node]);
    assert_eq!(table.count, RowCount(0));
    assert_eq!(side_reads.get(), 0);
}

#[test]
fn axis_admission_uses_side_first_only_for_the_exact_streamed_held_held_triple() {
    for domain_streamed in [false, true] {
        let mut input = authored_request(1);
        input.plans[0].steps = vec![step(select(Relation::Function))];
        let domain = if domain_streamed {
            input.plans[0].steps.push(step(select(Relation::Defines)));
            Input::Step(StepId(1))
        } else {
            Input::Relation(Relation::Defines)
        };
        input.plans[0]
            .steps
            .push(step(axis(Input::Step(StepId(0)), domain, 0)));
        input.plans[0].emit.step = StepId((input.plans[0].steps.len() - 1) as u32);
        let reads = std::cell::RefCell::new(vec![]);
        let read = |relation| {
            reads.borrow_mut().push(relation);
            Err::<SourceRows<'_>, _>(
                if (domain_streamed && relation == Relation::Function)
                    || (!domain_streamed && relation == Relation::SemanticChild)
                {
                    OperatingError::WrongBinding
                } else {
                    OperatingError::NotRead
                },
            )
        };
        assert!(matches!(
            evaluate(input, &read),
            RootOutcome::Failed(OperatingError::WrongBinding)
        ));
        assert_eq!(
            reads.borrow().as_slice(),
            if domain_streamed {
                &[Relation::Function]
            } else {
                &[Relation::SemanticChild]
            }
        );
    }
}
