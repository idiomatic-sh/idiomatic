//! The authored request of the partition controls: the named producer, the project producer and the consumer join, shaped by the mode.
use super::super::super::*;
use ryo_execution::inbound::use_cases::prepare::types::PrepareRequest;
use ryo_relations::domain::relations::types::Integer;
use ryo_values::domain::hosted::types::HostColumn;
use ryo_values::domain::hosted::types::HostColumnName;
use ryo_values::domain::hosted::types::HostKind;
use ryo_values::domain::hosted::types::HostName;
use ryo_values::domain::hosted::types::HostRelation;
use ryo_values::domain::hosted::types::HostRow;
use ryo_values::domain::observations::types::BoundObservationInput;
use ryo_values::domain::observations::types::ObservationBinding;
use ryo_values::domain::observations::types::ObservationBindingName;
use ryo_values::domain::observations::types::ObservationColumn;
use ryo_values::domain::observations::types::ObservationColumnName;
use ryo_values::domain::observations::types::ObservationName;
use ryo_values::domain::observations::types::ObservationProducer;
use ryo_values::domain::observations::types::PlanObservation;
use ryo_values::domain::plan_parts::types::HostedNumber;
use ryo_values::domain::plan_parts::types::ObservationBindingId;
use ryo_values::domain::plan_parts::types::RepeatedRows;
use ryo_values::domain::records::types::ObservationReadinessLayout;
use ryo_values::domain::row_texts::types::Cell as HostCell;
use ryo_values::domain::runs::types::RuleApplicability;
use ryo_values::domain::stored_facts::types::Digest;

#[derive(Clone, Copy, Debug)]
pub(super) enum Mode {
    PartialRight,
    PartialLeft { difference: bool },
    Rekey { file_key: bool },
    Current,
    CurrentRekey { file_key: bool },
}
pub(super) fn authored(project_away_file: bool, mode: Mode) -> PrepareRequest {
    let mut authored = authored_request(3);
    authored.rules[0].applicability = RuleApplicability::AcceptedSyntax;
    let file_column = kinds(Relation::Function)
        .iter()
        .position(|kind| *kind == CellKind::File)
        .unwrap();
    let columns = vec![
        ObservationColumn {
            name: ObservationColumnName("file".into()),
            kind: CellKind::File,
        },
        ObservationColumn {
            name: ObservationColumnName("code".into()),
            kind: CellKind::Number,
        },
    ];
    crate::boundary_fixture::plans(&mut authored)[0].steps[0].operation = Operation::Select {
        from: Input::Relation(Relation::Function),
        comparisons: vec![],
        kept: vec![ColumnIndex(file_column as u32)],
    };
    crate::boundary_fixture::plans(&mut authored)[0]
        .steps
        .push(PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Extend {
                from: Input::Step(StepId(0)),
                values: vec![ryo_values::domain::plan_parts::types::Operand::Literal(
                    ryo_values::domain::plan_parts::types::QueryLiteral::Number(Integer(7)),
                )],
            },
        });
    crate::boundary_fixture::plans(&mut authored)[0].frontier = vec![Input::Step(StepId(1))];
    crate::boundary_fixture::plans(&mut authored)[0].observations = vec![PlanObservation {
        name: ObservationName("files".into()),
        source: Input::Step(StepId(1)),
        columns: columns.clone(),
        scope: StepScope::PerFile,
    }];
    // A real empty project producer makes the declared union project-scoped.
    // It is evaluated and installed through the same public phase protocol.
    crate::boundary_fixture::plans(&mut authored)[1].steps = vec![PlanStep {
        scope: StepScope::CrossFile,
        operation: Operation::Hosted {
            relation: HostedNumber(0),
        },
    }];
    if matches!(mode, Mode::Current | Mode::CurrentRekey { .. }) {
        let column = kinds(Relation::ModuleIdentity)
            .iter()
            .position(|kind| *kind == CellKind::File)
            .unwrap();
        crate::boundary_fixture::plans(&mut authored)[1].steps[0].operation = Operation::Select {
            from: Input::Relation(Relation::ModuleIdentity),
            comparisons: vec![],
            kept: vec![ColumnIndex(column as u32)],
        };
        crate::boundary_fixture::plans(&mut authored)[1]
            .steps
            .push(PlanStep {
                scope: StepScope::CrossFile,
                operation: Operation::Extend {
                    from: Input::Step(StepId(0)),
                    values: vec![ryo_values::domain::plan_parts::types::Operand::Literal(
                        ryo_values::domain::plan_parts::types::QueryLiteral::Number(Integer(7)),
                    )],
                },
            });
    }
    if let Mode::CurrentRekey { file_key } = mode {
        let key = ColumnIndex(if file_key { 0 } else { 1 });
        crate::boundary_fixture::plans(&mut authored)[1]
            .steps
            .push(PlanStep {
                scope: StepScope::CrossFile,
                operation: Operation::Hosted {
                    relation: HostedNumber(2),
                },
            });
        crate::boundary_fixture::plans(&mut authored)[1]
            .steps
            .push(PlanStep {
                scope: StepScope::CrossFile,
                operation: Operation::Join {
                    left: Input::Step(StepId(1)),
                    right: Input::Step(StepId(2)),
                    left_on: vec![key],
                    right_on: vec![key],
                    output: vec![
                        JoinColumn {
                            side: Side::Right,
                            column: ColumnIndex(0),
                        },
                        JoinColumn {
                            side: Side::Right,
                            column: ColumnIndex(1),
                        },
                    ],
                    filter: vec![],
                    view: vec![],
                    reach: None,
                },
            });
    }
    crate::boundary_fixture::plans(&mut authored)[1].observations = vec![PlanObservation {
        name: ObservationName("files".into()),
        source: Input::Step(StepId(
            (crate::boundary_fixture::plans(&mut authored)[1].steps.len() - 1) as u32,
        )),
        columns: columns.clone(),
        scope: StepScope::CrossFile,
    }];
    crate::boundary_fixture::plans(&mut authored)[2].observation_inputs =
        vec![BoundObservationInput {
            name: ObservationBindingName("files".into()),
            columns: columns.clone(),
            scope: StepScope::CrossFile,
        }];
    authored.rules[2].observations = vec![ObservationBinding {
        name: ObservationBindingName("files".into()),
        columns,
        producers: (0..2)
            .map(|rule| ObservationProducer::Output {
                rule: authored.rules[rule].id.clone(),
                output: ObservationName("files".into()),
            })
            .collect(),
    }];
    crate::boundary_fixture::plans(&mut authored)[2].steps = vec![
        PlanStep {
            scope: StepScope::CrossFile,
            operation: Operation::Hosted {
                relation: HostedNumber(1),
            },
        },
        PlanStep {
            scope: StepScope::CrossFile,
            operation: Operation::Antijoin {
                left: Input::Step(StepId(0)),
                right: Input::BoundObservation(ObservationBindingId(0)),
                left_on: vec![ColumnIndex(0)],
                right_on: vec![ColumnIndex(0)],
            },
        },
    ];
    if let Mode::PartialLeft { difference } = mode {
        crate::boundary_fixture::plans(&mut authored)[2].steps[0].operation = Operation::Hosted {
            relation: HostedNumber(0),
        };
        let left = Input::BoundObservation(ObservationBindingId(0));
        let right = Input::Step(StepId(0));
        crate::boundary_fixture::plans(&mut authored)[2].steps[1].operation = if difference {
            Operation::Difference { left, right }
        } else {
            Operation::Antijoin {
                left,
                right,
                left_on: vec![ColumnIndex(0)],
                right_on: vec![ColumnIndex(0)],
            }
        };
    }
    if let Mode::Rekey { file_key } = mode {
        let key = ColumnIndex(if file_key { 0 } else { 1 });
        crate::boundary_fixture::plans(&mut authored)[2].steps[1] = PlanStep {
            scope: StepScope::CrossFile,
            operation: Operation::Hosted {
                relation: HostedNumber(2),
            },
        };
        crate::boundary_fixture::plans(&mut authored)[2]
            .steps
            .push(PlanStep {
                scope: StepScope::CrossFile,
                operation: Operation::Join {
                    left: Input::BoundObservation(ObservationBindingId(0)),
                    right: Input::Step(StepId(1)),
                    left_on: vec![key],
                    right_on: vec![key],
                    output: vec![
                        JoinColumn {
                            side: Side::Right,
                            column: ColumnIndex(0),
                        },
                        JoinColumn {
                            side: Side::Right,
                            column: ColumnIndex(1),
                        },
                    ],
                    filter: vec![],
                    view: vec![],
                    reach: None,
                },
            });
        crate::boundary_fixture::plans(&mut authored)[2]
            .steps
            .push(PlanStep {
                scope: StepScope::CrossFile,
                operation: Operation::Antijoin {
                    left: Input::Step(StepId(0)),
                    right: Input::Step(StepId(2)),
                    left_on: vec![ColumnIndex(0), ColumnIndex(1)],
                    right_on: vec![ColumnIndex(0), ColumnIndex(1)],
                },
            });
    }
    if matches!(mode, Mode::Current | Mode::CurrentRekey { .. }) {
        if let Operation::Antijoin {
            left_on, right_on, ..
        } = &mut crate::boundary_fixture::plans(&mut authored)[2].steps[1].operation
        {
            *left_on = vec![ColumnIndex(0), ColumnIndex(1)];
            *right_on = left_on.clone();
        }
    }
    if project_away_file {
        let previous = crate::boundary_fixture::plans(&mut authored)[2].steps.len() - 1;
        crate::boundary_fixture::plans(&mut authored)[2]
            .steps
            .push(PlanStep {
                scope: StepScope::CrossFile,
                operation: Operation::Project {
                    from: Input::Step(StepId(previous as u32)),
                    columns: vec![ColumnIndex(1)],
                    distinct: RepeatedRows::Kept,
                },
            });
    }
    crate::boundary_fixture::plans(&mut authored)[2].emit.step =
        StepId((crate::boundary_fixture::plans(&mut authored)[2].steps.len() - 1) as u32);
    let host_column = |name: &str, kind| HostColumn {
        name: HostColumnName(name.into()),
        kind,
    };
    authored.hosted = vec![
        HostRelation {
            name: HostName("empty-project".into()),
            columns: vec![
                host_column("file", HostKind::File),
                host_column("code", HostKind::Number),
            ],
            rows: vec![],
        },
        HostRelation {
            name: HostName("candidates".into()),
            columns: vec![
                host_column("file", HostKind::File),
                host_column("value", HostKind::Number),
            ],
            rows: [
                ("second.py", 10),
                ("input.py", 20),
                ("input.py", 20),
                ("second.py", 40),
                ("input.py", 30),
            ]
            .into_iter()
            .map(|(file, value)| {
                HostRow(vec![
                    HostCell::File(file.into()),
                    HostCell::Number(Integer(value)),
                ])
            })
            .collect(),
        },
    ];
    if matches!(mode, Mode::Rekey { .. }) {
        let row = HostRow(vec![
            HostCell::File("input.py".into()),
            HostCell::Number(Integer(7)),
        ]);
        authored.hosted[1].rows = vec![row.clone()];
        authored.hosted.push(HostRelation {
            name: HostName("rekey-targets".into()),
            columns: authored.hosted[1].columns.clone(),
            rows: vec![row],
        });
    }
    if matches!(mode, Mode::Current | Mode::CurrentRekey { .. }) {
        authored.hosted[1].rows = [
            ("second.py", 7),
            ("input.py", 7),
            ("input.py", 8),
            ("second.py", 8),
            ("input.py", 8),
        ]
        .into_iter()
        .map(|(file, code)| {
            HostRow(vec![
                HostCell::File(file.into()),
                HostCell::Number(Integer(code)),
            ])
        })
        .collect();
    }
    if matches!(mode, Mode::CurrentRekey { .. }) {
        let row = HostRow(vec![
            HostCell::File("input.py".into()),
            HostCell::Number(Integer(7)),
        ]);
        authored.hosted[1].rows = vec![row.clone()];
        authored.hosted.push(HostRelation {
            name: HostName("current-rekey-targets".into()),
            columns: authored.hosted[1].columns.clone(),
            rows: vec![row],
        });
    }
    authored.observations =
        ObservationIdentity::Named(ObservationReadinessLayout(Digest([97; 16])));
    authored
}
