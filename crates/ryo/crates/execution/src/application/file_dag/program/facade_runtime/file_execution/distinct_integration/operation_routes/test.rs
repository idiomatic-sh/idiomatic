//! All inventory operations publish independent expected rows through the common compiled owner.
use super::super::*;
use crate::application::file_dag::acceptance::operator_contract_test::fixtures;
use ryo_language::domain::symbols::{
    service::{query_intern_derived, text_of},
    types::ByteSlice,
};
use ryo_relations::domain::terms::types::NameText;
use ryo_values::domain::{
    hosted::types::{HostColumn, HostColumnName, HostKind, HostName, HostRelation, HostRow},
    plan_parts::types::{HostedNumber, ParameterName},
    plans::{
        service::{each_input, inputs_of, named},
        types::{OperationKind, ParameterType, PlanParameter},
    },
    records::types::Cell as HostedCell,
};

/// Data-only fixture assembly: scalar snapshots and canonical provider rows.
/// Every operation, schema, publication and dispatch below belongs to production.
#[derive(Default)]
struct Sources {
    steps: Vec<PlanStep>,
    hosts: Vec<HostRelation>,
    providers: Vec<(Relation, Table)>,
}
impl Sources {
    fn push(&mut self, operation: Operation) -> PlanInput {
        let at = self.steps.len();
        self.steps.push(PlanStep {
            scope: StepScope::PerFile,
            operation,
        });
        PlanInput::Step(StepId(at as u32))
    }
    fn snapshot(&mut self, table: &Table, symbols: &Symbols) -> HostedNumber {
        let columns = table
            .kinds
            .iter()
            .enumerate()
            .map(|(at, kind)| HostColumn {
                name: HostColumnName(format!("value-{at}").into()),
                kind: match kind {
                    CellKind::Number => HostKind::Number,
                    CellKind::Symbol => HostKind::Text,
                    other => panic!("scalar fixture snapshot cannot invent {other:?}"),
                },
            })
            .collect();
        let rows = table
            .cells
            .chunks_exact(table.kinds.len())
            .map(|row| {
                HostRow(
                    row.iter()
                        .map(|cell| match cell {
                            Cell::Number(number) => HostedCell::Number(*number),
                            Cell::Symbol(symbol) => HostedCell::Text(
                                text_of(symbols, ByteSlice(fixtures::SOURCE), *symbol)
                                    .expect("fixture symbol text")
                                    .0
                                    .into(),
                            ),
                            other => panic!("scalar fixture cell {other:?}"),
                        })
                        .collect(),
                )
            })
            .collect();
        let number = HostedNumber(self.hosts.len() as u32);
        self.hosts.push(HostRelation {
            name: HostName(format!("route-input-{}", number.0).into()),
            columns,
            rows,
        });
        number
    }
    fn free_tree_schema(&self) -> Relation {
        if self
            .providers
            .iter()
            .all(|(relation, _)| *relation != Relation::SemanticSyntax)
        {
            Relation::SemanticSyntax
        } else {
            assert!(
                self.providers
                    .iter()
                    .all(|(relation, _)| *relation != Relation::SourceLocation)
            );
            Relation::SourceLocation
        }
    }
    fn provider(&mut self, relation: Relation, rows: Vec<Vec<Cell>>) -> PlanInput {
        let count = rows.len();
        self.providers.push((
            relation,
            Table {
                kinds: kinds(relation).to_vec(),
                cells: rows.into_iter().flatten().collect(),
                count: RowCount(count),
            },
        ));
        PlanInput::Relation(relation)
    }
    fn base(relation: Relation, at: usize) -> Vec<Cell> {
        kinds(relation)
            .iter()
            .copied()
            .map(|kind| match kind {
                CellKind::Node => {
                    Cell::Node(ryo_relations::domain::relations::types::NodeId(at as u64))
                }
                CellKind::Symbol => Cell::Symbol(ryo_relations::domain::terms::types::Symbol(0)),
                CellKind::File => Cell::File(FileId(0)),
                CellKind::Span => Cell::Span(Span {
                    start: Offset(0),
                    end: Offset(0),
                }),
                CellKind::Number => Cell::Number(Integer(0)),
                CellKind::Flag => Cell::Flag(Flag::Lowered),
            })
            .collect()
    }
    fn projection(&mut self, from: PlanInput, columns: Vec<ColumnIndex>) -> PlanInput {
        self.push(Operation::Select {
            from,
            comparisons: vec![],
            kept: columns,
        })
    }
    fn input(&mut self, table: &Table, symbols: &Symbols) -> PlanInput {
        if table
            .kinds
            .iter()
            .all(|kind| matches!(kind, CellKind::Number | CellKind::Symbol))
        {
            let relation = self.snapshot(table, symbols);
            return self.push(Operation::Hosted { relation });
        }
        if table.kinds == [CellKind::Node; 2] {
            let relation = Relation::SemanticChild;
            let rows = table
                .cells
                .chunks_exact(2)
                .enumerate()
                .map(|(at, row)| {
                    let mut cells = Self::base(relation, at);
                    cells[..2].copy_from_slice(row);
                    cells
                })
                .collect();
            let from = self.provider(relation, rows);
            return self.projection(from, vec![ColumnIndex(0), ColumnIndex(1)]);
        }
        if table.kinds == [CellKind::Span; 2] {
            let mut sides = Vec::new();
            for side in 0..2 {
                let relation = self.free_tree_schema();
                let schema = kinds(relation);
                let span = schema
                    .iter()
                    .position(|kind| *kind == CellKind::Span)
                    .unwrap();
                let rows = table
                    .cells
                    .chunks_exact(2)
                    .enumerate()
                    .map(|(at, row)| {
                        let mut cells = Self::base(relation, at);
                        cells[span] = row[side];
                        cells
                    })
                    .collect();
                let from = self.provider(relation, rows);
                sides.push(self.projection(from, vec![ColumnIndex(0), ColumnIndex(span as u32)]));
            }
            return self.push(Operation::Join {
                left: sides[0],
                right: sides[1],
                left_on: vec![ColumnIndex(0)],
                right_on: vec![ColumnIndex(0)],
                output: vec![
                    JoinColumn {
                        side: Side::Left,
                        column: ColumnIndex(1),
                    },
                    JoinColumn {
                        side: Side::Right,
                        column: ColumnIndex(1),
                    },
                ],
                filter: vec![],
                view: vec![],
                reach: None,
            });
        }
        let relation = self.free_tree_schema();
        let schema = kinds(relation);
        let projected = table
            .kinds
            .iter()
            .map(|kind| {
                ColumnIndex(
                    schema
                        .iter()
                        .position(|column| column == kind)
                        .expect("canonical fixture schema") as u32,
                )
            })
            .collect::<Vec<_>>();
        let rows = table
            .cells
            .chunks_exact(table.kinds.len())
            .enumerate()
            .map(|(at, row)| {
                let mut cells = Self::base(relation, at);
                for (column, value) in projected.iter().zip(row) {
                    cells[column.0 as usize] = *value;
                }
                cells
            })
            .collect();
        let from = self.provider(relation, rows);
        self.projection(from, projected)
    }
}
fn execute_fixture(
    kind: OperationKind,
    downstream: bool,
    operators: Arc<dyn crate::outbound::operators::interface::OperatorsPort>,
) -> Result<(Table, Symbols), OperatingError> {
    let mut case = fixtures::fixture(kind);
    if ryo_language::domain::symbols::service::name_count(&case.symbols).0 == 0 {
        query_intern_derived(
            &mut (&mut case.symbols).into(),
            ByteSlice(fixtures::SOURCE),
            NameText(""),
        );
    }
    let mut sources = Sources::default();
    let mut operation = case.operation.clone();
    if kind == OperationKind::Hosted {
        let relation = sources.snapshot(&case.inputs[0], &case.symbols);
        operation = Operation::Hosted { relation };
    } else {
        let declarations = named(&inputs_of(&operation)).to_vec();
        assert_eq!(declarations.len(), case.inputs.len());
        let installed = case
            .inputs
            .iter()
            .map(|table| sources.input(table, &case.symbols))
            .collect::<Vec<_>>();
        each_input(&mut operation, &mut |input| {
            let at = declarations
                .iter()
                .position(|declared| declared == input)
                .expect("declared fixture input");
            *input = installed[at];
        });
    }
    let target = sources.steps.len();
    sources.push(operation);
    if downstream {
        sources.push(Operation::Extend {
            from: PlanInput::Step(StepId(target as u32)),
            values: vec![],
        });
    }
    let parameters = if case.bindings.is_empty() {
        vec![]
    } else {
        vec![PlanParameter {
            name: ParameterName("fixture".into()),
            kind: if kind == OperationKind::Rows {
                ParameterType::Tables
            } else {
                ParameterType::TextSets
            },
        }]
    };
    let plan = Plan {
        parameters,
        observation_inputs: vec![],
        observations: vec![],
        comparison_origins: vec![],
        frontier: vec![],
        emit: Emit {
            step: StepId((sources.steps.len() - 1) as u32),
            subject: ColumnIndex(0),
            key: vec![],
            headings: vec![],
        },
        steps: sources.steps,
    };
    let derived = crate::application::file_dag::testing::program_derived_port();
    let prepared = crate::application::file_dag::program::preparation::service::prepare(
        Compiled {
            schedule: RuleSchedule::Independent,
            observations: vec![vec![]],
            plans: vec![plan],
            bindings: vec![case.bindings],
        },
        &[rule()],
        &[Digest([0; 16])],
        &sources.hosts,
        Arc::clone(&derived),
        ObservationIdentity::Independent,
    )?;
    let services = Arc::new(ProgramServices::new(
        Arc::new(CommonOperatorEngine::new(operators)),
        derived,
        &[CorpusInput::Available(CorpusEntry {
            executable: ExecutablePermission::Unknown,
            path: FilePath("operation-route.py".into()),
            size: FileSize(fixtures::SOURCE.len() as u64),
            modified: Nanoseconds(0),
        })],
    ));
    let semantic = fixtures::KnownSyntax;
    let mut file = open_file(
        &prepared,
        FileId(0),
        ScopePartials::File {
            evaluation: &[],
            publication: &[],
        },
        &[FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Collected,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::Disabled,
        }],
        services,
        ryo_values::domain::files::types::SourceView(fixtures::SOURCE),
        crate::application::file_dag::program::facade_runtime::types::ReuseEligibility::Eligible,
        &mut case.symbols,
        Some(&semantic),
        None)?;
    let read = |relation| {
        sources
            .providers
            .iter()
            .find(|(source, _)| *source == relation)
            .map(|(_, table)| Rows::Computed(table))
            .ok_or(OperatingError::NotRead)
    };
    let outcome = file.rule(
        RuleIndex(0),
        RuleDemand {
            emit: EmitDemand::Enabled,
            observations: ObservationDemand::NotRequired,
        },
        &read,
        &mut case.symbols,
    )?;
    let result = match outcome.emit {
        RootOutcome::Complete(table) => Ok(table),
        RootOutcome::Unknown(error) | RootOutcome::Failed(error) => Err(error),
        _ => Err(OperatingError::NotRun),
    };
    file.finish()?;
    let actual = result?;
    assert!(actual.count.0 > 0, "nonempty actual {kind:?} owner route");
    let rows = actual
        .cells
        .chunks_exact(actual.kinds.len())
        .map(<[_]>::to_vec)
        .collect::<Vec<_>>();
    fixtures::assert_expected(&case.expected, &rows, &case.symbols);
    Ok((actual, case.symbols))
}
#[test]
fn all26_actual_production_routes_publish_the_independent_expected_rows() {
    for &kind in OperationKind::ALL {
        // Each case uses the real normal scheduler for both an emitted root and
        // an authored downstream row consumer. Rows/Hosted remain request-owned sources.
        for downstream in [false, true] {
            execute_fixture(kind,downstream,Arc::new(ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new()))
                .unwrap_or_else(|error|panic!("compiled {kind:?}: {error:?}"));
        }
    }
}
#[derive(Debug)]
struct RefusingSemantics;
impl ryo_operators::inbound::use_cases::operators::interface::OperatorsUseCase
    for RefusingSemantics
{
    fn operators(
        &self,
        _: ryo_operators::inbound::use_cases::operators::interface::OperatorCall<'_>,
    ) -> Result<
        ryo_operators::inbound::use_cases::operators::interface::OperatorAnswer,
        ryo_operators::inbound::use_cases::operators::interface::PortableOperatorFault,
    > {
        Err(ryo_operators::inbound::use_cases::operators::interface::PortableOperatorFault::WrongOperation)
    }
}
#[test]
fn production_route_cannot_bypass_an_injected_semantic_refusal() {
    assert!(matches!(
        execute_fixture(OperationKind::Project, false, Arc::new(RefusingSemantics)),
        Err(OperatingError::WrongOperation)
    ));
}
