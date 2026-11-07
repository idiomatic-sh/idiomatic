//! Project text identity does not depend on inherited arenas.
use super::super::*;

#[test]
fn project_text_identity_does_not_depend_on_inherited_arenas() {
    use ryo_values::domain::hosted::types::{
        HostColumn, HostColumnName, HostKind, HostName, HostRelation, HostRow,
    };
    use ryo_values::domain::plan_parts::types::{HostedNumber, LiteralText, Operand, QueryLiteral};
    use ryo_values::domain::row_texts::types::Cell;

    use ryo_values::domain::stored_facts::types::ArenaBytes;
    for use_parameter_rows in [false, true] {
        for arenas in [vec![], vec![ArenaBytes::default()]] {
            for selected in ["F401", "F822"] {
                let engine = engine();
                let mut authored = authored_request(1);
                authored.hosted.push(HostRelation {
                    name: HostName("selected".into()),
                    columns: vec![HostColumn {
                        name: HostColumnName("code".into()),
                        kind: HostKind::Text,
                    }],
                    rows: vec![HostRow(vec![Cell::Text(selected.into())])],
                });
                crate::boundary_fixture::plans(&mut authored)[0].steps = vec![
                    PlanStep {
                        scope: StepScope::CrossFile,
                        operation: Operation::Select {
                            from: Input::Relation(Relation::ModuleIdentity),
                            comparisons: vec![],
                            kept: vec![ColumnIndex(0)],
                        },
                    },
                    PlanStep {
                        scope: StepScope::CrossFile,
                        operation: Operation::Extend {
                            from: Input::Step(StepId(0)),
                            values: vec![Operand::Literal(QueryLiteral::Text(LiteralText(
                                "F401".into(),
                            )))],
                        },
                    },
                    PlanStep {
                        scope: StepScope::CrossFile,
                        operation: Operation::Hosted {
                            relation: HostedNumber(0),
                        },
                    },
                    PlanStep {
                        scope: StepScope::CrossFile,
                        operation: Operation::Join {
                            left: Input::Step(StepId(1)),
                            right: Input::Step(StepId(2)),
                            left_on: vec![ColumnIndex(1)],
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
                if use_parameter_rows {
                    use ryo_values::domain::plan_parts::types::{
                        ParameterId, ParameterName, TableKey,
                    };
                    use ryo_values::domain::query_bindings::types::ParameterType;
                    use ryo_values::domain::query_bindings::types::PlanParameter;
                    use ryo_values::domain::query_bindings::types::{
                        BindingCell, BindingTable, ParameterBinding,
                    };
                    crate::boundary_fixture::plans(&mut authored)[0].parameters = vec![PlanParameter {
                        name: ParameterName("codes".into()),
                        kind: ParameterType::Tables,
                    }];
                    crate::boundary_fixture::bindings(&mut authored)[0] =
                        vec![ParameterBinding::Tables(vec![BindingTable(vec![
                            BindingCell {
                                key: LiteralText("code".into()),
                                value: LiteralText(selected.into()),
                            },
                        ])])];
                    crate::boundary_fixture::plans(&mut authored)[0].steps[2].operation = Operation::Rows {
                        parameter: ParameterId(0),
                        keys: vec![TableKey("code".into())],
                    };
                }
                crate::boundary_fixture::plans(&mut authored)[0].frontier = vec![Input::Relation(Relation::ModuleIdentity)];
                crate::boundary_fixture::plans(&mut authored)[0].emit.step = StepId(3);
                let prepared = engine.prepare(authored).unwrap();
                let corpus = corpus();
                let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus);
                let covered = [Covered {
                    file: FileId(0),
                    rules: vec![RuleIndex(0)],
                    partial: vec![],
                }];
                let phase = request.project_phase().unwrap();
                let scope = phase
                    .project_scope(RuleIndex(0), &corpus, &covered, &[], &ryo_execution::inbound::use_cases::execution::interface::InapplicableFiles::default())
                    .unwrap();
                let stored = CapturedFacts {
                    captures: vec![],
                    reads: AtomicUsize::new(0),
                };
                let project = ProjectFacts::new(Relation::ModuleIdentity);
                let project_symbols = Symbols::default();
                let mut symbols = Symbols::default();
                let ordinary = request
                    .prepare_frontiers(
                        FrontierRequest {
                            demand: FrontierDemand::Diagnostic,
                            scope: &scope,
                            rule: RuleIndex(0),
                            first: Bucket(0),
                            facts: &stored,
                            project: &project,
                            project_symbols: &project_symbols,
                        },
                        &mut symbols,
                    )
                    .unwrap();
                let partials = Partials::default();
                let result = crate::boundary_fixture::project_root(
                    &phase,
                    RuleIndex(0),
                    demand(),
                    ProjectInput {
                        scope: &scope,
                        evaluation: &partials,
                        publication: &partials,
                        ordinary,
                        bound: &[],
                        arenas: &arenas,
                    },
                    &mut symbols,
                )
                .unwrap();
                let RootOutcome::Complete(table) = result.emit else {
                    panic!("complete project join")
                };
                assert_eq!(table.kinds, [CellKind::File]);
                assert_eq!(table.count, RowCount(usize::from(selected == "F401")));
                assert_eq!(
                    table.cells,
                    if selected == "F401" {
                        vec![Value::File(FileId(0))]
                    } else {
                        vec![]
                    }
                );
            }
        }
    }
}
