//! The graph inputs, hosted plan and request runner the ordered traversal proofs share.
use super::super::*;
use crate::domain::operating::types::ExecutionObservation;
use ryo_values::domain::hosted::types::{
    HostColumn, HostColumnName, HostKind, HostName, HostRelation, HostRow,
};
use ryo_values::domain::plan_parts::types::HostedNumber;
use ryo_values::domain::row_texts::types::Cell as HostCell;
use ryo_values::domain::rows::service::push_row;
pub(crate) const FROM: PlanInput = PlanInput::Relation(Relation::Function);
pub(crate) fn column(at: u32) -> ColumnIndex {
    ColumnIndex(at)
}
pub(crate) fn numbers(width: usize, rows: &[&[i64]]) -> Table {
    let mut table = Table {
        kinds: vec![CellKind::Number; width],
        ..Table::default()
    };
    for row in rows {
        push_row(
            &mut table,
            &row.iter()
                .map(|n| Cell::Number(Integer(*n)))
                .collect::<Vec<_>>(),
        );
    }
    table
}
pub(crate) fn operation(root_keys: &[u32], edge_keys: &[u32]) -> Operation {
    Operation::OrderedDfs {
        from: FROM,
        edges: FROM,
        root: column(0),
        source: column(0),
        destination: column(1),
        order: column(2),
        edge: column(3),
        root_keys: root_keys.iter().copied().map(column).collect(),
        edge_keys: edge_keys.iter().copied().map(column).collect(),
    }
}
pub(crate) fn expected(out: &Table, width: usize, rows: &[&[i64]]) {
    let mut actual: Vec<_> = out
        .cells
        .chunks_exact(width)
        .map(<[Cell]>::to_vec)
        .collect();
    let mut wanted: Vec<Vec<_>> = rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|number| Cell::Number(Integer(*number)))
                .collect()
        })
        .collect();
    actual.sort_unstable();
    wanted.sort_unstable();
    assert_eq!(out.kinds, vec![CellKind::Number; width]);
    assert_eq!(out.count.0, wanted.len());
    assert_eq!(out.cells.len(), out.count.0 * width);
    assert_eq!(actual, wanted);
}
pub(crate) fn host(table: &Table, role: &str) -> HostRelation {
    HostRelation {
        name: HostName(format!("dfs-input-{role}").into()),
        columns: table
            .kinds
            .iter()
            .enumerate()
            .map(|(at, kind)| HostColumn {
                name: HostColumnName(format!("value-{at}").into()),
                kind: match kind {
                    CellKind::Number => HostKind::Number,
                    CellKind::Symbol => HostKind::Text,
                    other => panic!("unsupported scalar fixture kind {other:?}"),
                },
            })
            .collect(),
        rows: table
            .cells
            .chunks_exact(table.kinds.len())
            .map(|row| {
                HostRow(
                    row.iter()
                        .map(|cell| match cell {
                            Cell::Number(value) => HostCell::Number(*value),
                            other => panic!("unsupported scalar fixture cell {other:?}"),
                        })
                        .collect(),
                )
            })
            .collect(),
    }
}
pub(crate) fn compute(
    operation: &Operation,
    roots: &Table,
    edges: &Table,
    observed: &mut ExecutionObservation,
) -> Result<Table, OperatingError> {
    let mut operation = operation.clone();
    let Operation::OrderedDfs {
        from,
        edges: input_edges,
        ..
    } = &mut operation
    else {
        return Err(OperatingError::WrongOperation);
    };
    *from = PlanInput::Step(StepId(0));
    *input_edges = PlanInput::Step(StepId(1));
    let mut plan = plan(Scenario::ProjectRetry);
    plan.steps = [
        Operation::Hosted {
            relation: HostedNumber(0),
        },
        Operation::Hosted {
            relation: HostedNumber(1),
        },
        operation,
    ]
    .into_iter()
    .map(|operation| PlanStep {
        scope: StepScope::PerFile,
        operation,
    })
    .collect();
    plan.emit.step = StepId(2);
    execute_request(
        plan,
        &[host(roots, "roots"), host(edges, "edges")],
        &[],
        &mut Symbols::default(),
        b"",
        false,
        observed,
    )
}
pub(crate) fn execute_request(
    plan: Plan,
    hosts: &[HostRelation],
    providers: &[(Relation, Table)],
    symbols: &mut Symbols,
    source: &[u8],
    delivery: bool,
    observed: &mut ExecutionObservation,
) -> Result<Table, OperatingError> {
    let target = plan.emit.step.0 as usize;
    let derived = crate::application::file_dag::fixtures::program_derived_port();
    let prepared = crate::application::file_dag::program::preparation::service::prepare(
        Compiled {
            schedule: RuleSchedule::Independent,
            observations: vec![vec![]],
            plans: vec![plan],
            bindings: vec![vec![]],
        },
        &[rule()],
        &[Digest([0; 16])],
        hosts,
        Arc::clone(&derived),
        ObservationIdentity::Independent, crate::domain::executable_program::types::RoutePolicy::default(),
    )?;
    let services = Arc::new(ProgramServices::new(
        Arc::new(CommonOperatorEngine::new(Arc::new(
            crate::application::file_dag::fixtures::operators(),
        ))),
        derived,
        &[CorpusInput::Available(CorpusEntry {
            executable: ExecutablePermission::Unknown,
            path: FilePath("ordered.py".into()),
            size: FileSize(0),
            modified: Nanoseconds(0),
        })],
    ));
    let mut file = open_file(
        &prepared,
        FileId(0),
        ScopePartials::File {
            evaluation: &[],
            publication: &[],
        },
        &[FileRuleIntent::Run {
            diagnostic: if delivery {
                DiagnosticDelivery::Forwarded
            } else {
                DiagnosticDelivery::Collected
            },
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::Disabled,
        }],
        services,
        ryo_values::domain::files::types::SourceView(source),
        crate::application::file_dag::relations::types::ReuseEligibility::Eligible,
        symbols,
        None,
        None,
    )?;
    let read = |relation| {
        providers
            .iter()
            .find(|(kind, _)| *kind == relation)
            .map(|(_, table)| Rows::Computed(table))
            .ok_or(OperatingError::NotRead)
    };
    let result = if delivery {
        let mut table = Table::default();
        file.emit_rows(RuleIndex(0), &read, symbols, &mut |row| {
            table.cells.extend_from_slice(row);
            table.count.0 += 1;
            Ok(())
        })?;
        Ok(table)
    } else {
        let outcome = file.rule(
            RuleIndex(0),
            RuleDemand {
                emit: EmitDemand::Enabled,
                observations: ObservationDemand::NotRequired,
            },
            &read,
            symbols,
        )?;
        match outcome.emit {
            RootOutcome::Complete(table) => Ok(table),
            RootOutcome::Unknown(error) | RootOutcome::Failed(error) => Err(error),
            _ => Err(OperatingError::NotRun),
        }
    };
    *observed = file.route_observation(RuleIndex(0), StepId(target as u32))?;
    let owner = Rc::clone(&file.owner);
    file.finish()?;
    assert_eq!(
        owner.derived().working_bytes(),
        0,
        "actual graph owner releases file resources"
    );
    result
}
pub(crate) fn subject(
    input: fn() -> Result<usize, OperatingError>,
) -> Result<usize, OperatingError> {
    input()
}
pub(crate) fn hosted_plan(mut op: Operation, roots: u32, edges: u32) -> Plan {
    match &mut op {
        Operation::OrderedDfs { from, edges, .. } => {
            *from = PlanInput::Step(StepId(0));
            *edges = PlanInput::Step(StepId(1));
        }
        _ => panic!("DFS fixture operation"),
    }
    let mut request = plan(Scenario::ProjectRetry);
    request.steps = [
        Operation::Hosted {
            relation: HostedNumber(roots),
        },
        Operation::Hosted {
            relation: HostedNumber(edges),
        },
        op,
    ]
    .into_iter()
    .map(|operation| PlanStep {
        scope: StepScope::PerFile,
        operation,
    })
    .collect();
    request.emit.step = StepId(2);
    request
}
