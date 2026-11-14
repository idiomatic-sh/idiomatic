//! Collected-mode preflight and explicit empty gates.
use crate::mode_order::fixtures::*;
use crate::*;
use ryo_values::domain::{
    hosted::types::{HostColumn, HostColumnName, HostKind, HostRow},
    plan_parts::types::HostedNumber,
    row_texts::types::Cell as HostCell,
};
#[test]
fn collected_reductions_validate_the_resource_operand_before_the_output_keys() {
    for digest in [false, true] {
        let mut input = authored_request(1);
        let from = Input::Step(StepId(0));
        boundary_fixture::fixtures::plans(&mut input)[0].steps = vec![
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
        boundary_fixture::fixtures::plans(&mut input)[0].emit.step = StepId(1);
        input.hosted = vec![hosted(&[1])];
        let mut read = TableFacts::new(&[]);
        assert_eq!(failed_column(evaluate(input, &mut read)), Some(99));
    }
}

#[test]
fn collected_schema_first_follows_actual_source_admission() {
    let mut input = authored_request(1);
    boundary_fixture::fixtures::plans(&mut input)[0].steps = vec![step(Operation::Select {
        from: Input::Relation(Relation::Function),
        comparisons: vec![],
        kept: vec![ColumnIndex(99)],
    })];
    let mut read = TableFacts::new(&[]);
    assert!(matches!(
        evaluate(input, &mut read),
        Err(FileStop::Facts(_))
    ));
    assert_eq!(read.reads(), 1);
}

#[test]
fn collected_antijoin_empty_left_precedes_unused_right_key_validation() {
    let mut input = authored_request(1);
    boundary_fixture::fixtures::plans(&mut input)[0].steps = vec![
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
    boundary_fixture::fixtures::plans(&mut input)[0].emit.step = StepId(2);
    input.hosted = vec![hosted(&[]), hosted(&[1])];
    let mut read = TableFacts::new(&[]);
    let Ok(RootOutcome::Complete(table)) = evaluate(input, &mut read) else {
        panic!("typed empty antijoin")
    };
    assert_eq!(table.kinds, [CellKind::Number]);
    assert_eq!(table.count, RowCount(0));
    assert!(table.cells.is_empty());
}

#[test]
fn collected_axis_empty_domain_gate_is_specific_to_computed_representation() {
    // A fact relation reaches the run as the worker's stored columns: the computed form of a fact relation has no route through
    // `run_file`, so only the stored domain is run.
    {
        let mut input = authored_request(1);
        boundary_fixture::fixtures::plans(&mut input)[0].steps = vec![
            step(select(Relation::Function)),
            step(axis(
                Input::Step(StepId(0)),
                Input::Relation(Relation::Defines),
                98,
            )),
        ];
        boundary_fixture::fixtures::plans(&mut input)[0].emit.step = StepId(1);
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
        let mut read = TableFacts::new(&[
            (Relation::Function, &roots),
            (Relation::SemanticChild, &edges),
            (Relation::Defines, &domain),
        ]);
        let outcome = evaluate(input, &mut read);
        assert_eq!(failed_column(outcome), Some(98));
    }
}

#[test]
fn empty_axis_and_antijoin_carry_the_final_fused_projection_layout() {
    use ryo_values::domain::plan_parts::types::RepeatedRows;
    for populated in [false, true] {
        let mut input = authored_request(1);
        let mut walk = axis(
            Input::Step(StepId(0)),
            Input::Relation(Relation::Defines),
            0,
        );
        if let Operation::Axis { domain_id, .. } = &mut walk {
            *domain_id = ColumnIndex(0);
        }
        boundary_fixture::fixtures::plans(&mut input)[0].steps = vec![
            step(select(Relation::Function)),
            step(walk),
            step(Operation::Project {
                from: Input::Step(StepId(1)),
                columns: vec![ColumnIndex(1)],
                distinct: RepeatedRows::Kept,
            }),
        ];
        boundary_fixture::fixtures::plans(&mut input)[0].emit.step = StepId(2);
        let mut roots = source();
        if !populated {
            roots.count = RowCount(0);
            roots.cells.clear();
        }
        let edges = Table {
            kinds: kinds(Relation::SemanticChild).to_vec(),
            count: RowCount(1),
            cells: vec![
                Value::Node(node_id(FileId(0), Position(2))),
                Value::Node(node_id(FileId(0), Position(3))),
                Value::Number(Integer(0)),
                Value::Symbol(ryo_relations::domain::terms::types::Symbol(0)),
                Value::File(FileId(0)),
            ],
        };
        let mut domain = source();
        domain.kinds = kinds(Relation::Defines).to_vec();
        domain.cells = domain
            .kinds
            .iter()
            .map(|kind| match kind {
                CellKind::Node => Value::Node(node_id(FileId(0), Position(3))),
                CellKind::File => Value::File(FileId(0)),
                CellKind::Number => Value::Number(Integer(0)),
                CellKind::Flag => Value::Flag(Flag::Lowered),
                CellKind::Span => Value::Span(Span {
                    start: Offset(0),
                    end: Offset(0),
                }),
                CellKind::Symbol => Value::Symbol(ryo_relations::domain::terms::types::Symbol(0)),
            })
            .collect();
        let mut read = TableFacts::new(&[
            (Relation::Function, &roots),
            (Relation::SemanticChild, &edges),
            (Relation::Defines, &domain),
        ]);
        let Ok(RootOutcome::Complete(table)) = evaluate(input, &mut read) else {
            panic!("complete projected axis")
        };
        assert_eq!(table.kinds, [CellKind::Node]);
        assert_eq!(table.count, RowCount(usize::from(populated)));
        assert_eq!(
            table.cells,
            if populated {
                vec![Value::Node(node_id(FileId(0), Position(3)))]
            } else {
                vec![]
            }
        );

        let mut input = authored_request(1);
        let mut left = hosted(&[]);
        left.columns.push(HostColumn {
            name: HostColumnName("payload".into()),
            kind: HostKind::Number,
        });
        if populated {
            left.rows.push(HostRow(vec![
                HostCell::Number(Integer(2)),
                HostCell::Number(Integer(9)),
            ]));
        }
        input.hosted = vec![left, hosted(&[1])];
        boundary_fixture::fixtures::plans(&mut input)[0].steps = vec![
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
                right_on: vec![ColumnIndex(0)],
            }),
            step(Operation::Project {
                from: Input::Step(StepId(2)),
                columns: vec![ColumnIndex(1)],
                distinct: RepeatedRows::Kept,
            }),
        ];
        boundary_fixture::fixtures::plans(&mut input)[0].emit.step = StepId(3);
        let mut read = TableFacts::new(&[]);
        let Ok(RootOutcome::Complete(table)) = evaluate(input, &mut read) else {
            panic!("complete projected antijoin")
        };
        assert_eq!(table.kinds, [CellKind::Number]);
        assert_eq!(table.count, RowCount(usize::from(populated)));
        assert_eq!(
            table.cells,
            if populated {
                vec![Value::Number(Integer(9))]
            } else {
                vec![]
            }
        );
    }
}

#[test]
fn an_empty_gate_schema_error_is_not_erased_by_a_valid_zero_column_continuation() {
    use ryo_values::domain::plan_parts::types::RepeatedRows;
    let mut input = authored_request(1);
    let mut walk = axis(
        Input::Step(StepId(0)),
        Input::Relation(Relation::Defines),
        0,
    );
    if let Operation::Axis { id, domain_id, .. } = &mut walk {
        *id = ColumnIndex(99);
        *domain_id = ColumnIndex(0);
    }
    boundary_fixture::fixtures::plans(&mut input)[0].steps = vec![
        step(select(Relation::Function)),
        step(walk),
        step(Operation::Project {
            from: Input::Step(StepId(1)),
            columns: vec![],
            distinct: RepeatedRows::Kept,
        }),
    ];
    boundary_fixture::fixtures::plans(&mut input)[0].emit.step = StepId(2);
    let roots = Table {
        kinds: kinds(Relation::Function).to_vec(),
        count: RowCount(0),
        cells: vec![],
    };
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
    let mut read = TableFacts::new(&[
        (Relation::Function, &roots),
        (Relation::SemanticChild, &edges),
        (Relation::Defines, &domain),
    ]);
    assert_eq!(failed_column(evaluate(input, &mut read)), Some(99));
}
