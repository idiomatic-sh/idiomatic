//! Native step joins validate both sources, and tree lookups keep their required support and fallback.
use super::super::super::cases::test::{node, table_of, tree_rows, wide_tree};
use super::super::super::*;
#[test]
fn empty_native_step_join_validates_both_sources_without_sorting_rhs_keys() {
    use ryo_values::domain::plan_parts::types::{
        LiteralText, Operand, PlanComparison, QueryLiteral, Test,
    };
    for (refuse_right, foreign_tree) in [(false, false), (true, false), (false, true)] {
        let (left_elements, mut symbols) = wide_tree(1, true);
        let (right_elements, _) = wide_tree(1, true);
        let mut request = plan(Scenario::ProjectRetry);
        request.steps = vec![
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Select {
                    from: PlanInput::Relation(Relation::SyntaxTree),
                    kept: vec![ColumnIndex(0)],
                    comparisons: vec![PlanComparison {
                        test: Test::Equal,
                        left: Operand::Column(ColumnIndex(1)),
                        right: Operand::Literal(QueryLiteral::Text(LiteralText(
                            "absent-kind".into(),
                        ))),
                        origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
                    }],
                },
            },
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Select {
                    from: PlanInput::Relation(Relation::SyntaxChild),
                    kept: vec![ColumnIndex(1)],
                    comparisons: vec![],
                },
            },
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Join {
                    left: PlanInput::Step(StepId(0)),
                    right: PlanInput::Step(StepId(1)),
                    left_on: vec![ColumnIndex(0)],
                    right_on: vec![ColumnIndex(0)],
                    output: vec![JoinColumn {
                        side: Side::Left,
                        column: ColumnIndex(0),
                    }],
                    filter: vec![],
                    view: vec![],
                    reach: None,
                },
            },
        ];
        request.emit.step = StepId(2);
        let (left, _) = provider(
            tree_rows(&left_elements, TreeForm::Elements),
            0,
            &mut vec![],
        );
        let (right, _) = provider(
            tree_rows(
                if foreign_tree {
                    &right_elements
                } else {
                    &left_elements
                },
                TreeForm::Edges,
            ),
            1,
            &mut vec![],
        );
        let actual = execute(
            request,
            [left, right],
            &mut symbols,
            b"abcd",
            false,
            real_port(),
            Faults {
                right: refuse_right.then_some(OperatingError::NotRead),
                ..Faults::default()
            },
        )
        .unwrap();
        assert!(
            actual.native_prepared,
            "the step-RHS native strategy must be real"
        );
        if refuse_right || foreign_tree {
            assert_eq!(actual.result, Err(OperatingError::NotRead));
        } else {
            let table = actual.result.unwrap();
            assert_eq!(table.count, RowCount(0));
            assert_eq!(table.kinds, [CellKind::Node]);
            assert_eq!(
                actual.work.native_positions, 1,
                "the admitted native empty publication was actually produced"
            );
        }
        assert!(
            actual.derived_builds.is_empty(),
            "including RawKeyOrder: no payload is needed for an empty primary"
        );
    }
}
#[test]
fn tree_lookup_preflight_preserves_required_support_and_optional_equality_fallback() {
    use ryo_derived::domain::contract::types::ViewRequest;
    use ryo_values::domain::plan_parts::types::{Operand, PlanComparison, Test};
    let (elements, mut symbols) = wide_tree(1, true);
    let right = tree_rows(&elements, TreeForm::Elements);
    let mut materialized = Table::default();
    materialize(&right, &mut materialized, &mut Vec::new());
    // The root's actual completed kind is a real provider symbol, not an invented id.
    let root_kind = materialized.cells[1];
    for empty in [true, false] {
        for delivery in [false, true] {
            for required in [0, 1, 2] {
                let left = if empty {
                    table_of(&[CellKind::Symbol], &[])
                } else {
                    table_of(&[CellKind::Symbol], &[&[root_kind]])
                };
                let filter = if required == 1 {
                    vec![PlanComparison {
                        test: Test::Equal,
                        left: Operand::Column(ColumnIndex(1)),
                        right: Operand::Column(ColumnIndex(1)),
                        origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
                    }]
                } else {
                    vec![]
                };
                let view: &[u32] = if required == 2 { &[0, 1, 2, 3, 4] } else { &[] };
                // Elements kind column1 is readable but not a native lookup key.
                let op = operation(&[0], &[1], &[(Side::Left, 0)], filter, view);
                let actual = invoke(
                    &op,
                    [Rows::Computed(&left), right.clone()],
                    &mut symbols,
                    b"abcd",
                    delivery,
                    real_port(),
                    Faults::default(),
                )
                .unwrap();
                assert_eq!(actual.native_prepared, false);
                if required != 0 {
                    assert!(
                        matches!(actual.result, Err(OperatingError::WrongOperation)),
                        "{:?}",
                        actual.result
                    );
                    assert!(actual.prefix.is_empty());
                    assert!(actual.derived_builds.is_empty());
                } else {
                    let result = actual.result.unwrap();
                    if !delivery {
                        assert_eq!(result.kinds, [CellKind::Symbol]);
                    }
                    assert_eq!(result.count.0, usize::from(!empty));
                    assert_eq!(result.cells, if empty { vec![] } else { vec![root_kind] });
                    if empty {
                        assert!(actual.derived_builds.is_empty());
                    } else {
                        assert!(
                            matches!(
                                actual.derived_builds.as_slice(),
                                [
                                    ViewRequest::NativeLookup { .. },
                                    ViewRequest::Equality { .. }
                                ]
                            ),
                            "{:?}",
                            actual.derived_builds
                        );
                    }
                }
            }
            // Supported native node keys must still accept a real filter, including empty LHS.
            let left = if empty {
                table_of(&[CellKind::Node], &[])
            } else {
                table_of(&[CellKind::Node], &[&[node(1)]])
            };
            let op = operation(
                &[0],
                &[0],
                &[(Side::Left, 0)],
                vec![PlanComparison {
                    test: Test::Equal,
                    left: Operand::Column(ColumnIndex(0)),
                    right: Operand::Column(ColumnIndex(0)),
                    origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
                }],
                &[],
            );
            let actual = invoke(
                &op,
                [Rows::Computed(&left), right.clone()],
                &mut symbols,
                b"abcd",
                delivery,
                real_port(),
                Faults::default(),
            )
            .unwrap();
            let result = actual.result.unwrap();
            if !delivery {
                assert_eq!(result.kinds, [CellKind::Node]);
            }
            assert_eq!(result.cells, if empty { vec![] } else { vec![node(1)] });
            assert_eq!(result.count.0, usize::from(!empty));
            if empty {
                assert!(actual.derived_builds.is_empty());
            } else {
                assert!(
                    actual
                        .derived_builds
                        .iter()
                        .any(|request| matches!(request, ViewRequest::NativeLookup { .. }))
                );
            }
        }
    }
}
