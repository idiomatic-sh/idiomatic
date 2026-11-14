//! The graph inputs, hosted plan and request runner the ordered traversal proofs share.
use super::super::super::service::*;
use super::super::fixtures::*;
use crate::application::file_dag::execution::service::CommonOperatorEngine;
use crate::application::file_dag::program::facade_runtime::service::ProgramServices;
use crate::application::file_dag::program::rule_outcome::types::RootOutcome;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operating::fixtures::ExecutionObservation;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::CaptureIntent;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::DiagnosticDelivery;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::EmitDemand;
use ryo_execution_domain::domain::rule_intent::types::FileRuleIntent;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::ObservationDemand;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::RuleDemand;
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_execution_domain::domain::{
    observation_schedule::types::RuleSchedule,
    prepared_request::types::{Compiled, ObservationIdentity},
};
use ryo_execution_store::application::file_dag::accounting::ledger::types::ByteCount;
use ryo_execution_store::application::file_dag::derived::relation_owner::types::ScopePartials;
use ryo_execution_store::application::file_dag::relations::types::ReuseEligibility;
use ryo_language::domain::{files::types::FilePath, symbols::types::Symbols};
use ryo_relations::domain::relations::types::{CellKind, FileId, Integer, Relation, Value as Cell};
use ryo_values::domain::hosted::types::{
    HostColumn, HostColumnName, HostKind, HostName, HostRelation, HostRow,
};
use ryo_values::domain::plan_parts::types::HostedNumber;
use ryo_values::domain::row_texts::types::Cell as HostCell;
use ryo_values::domain::rows::service::push_row;
use ryo_values::domain::{
    files::types::{CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds},
    plan_parts::types::{ColumnIndex, Input as PlanInput, StepId, StepScope},
    plans::types::{Operation, Plan, PlanStep},
    rows::types::Table,
    stored_facts::types::Digest,
};
use std::borrow::Cow;
use std::{rc::Rc, sync::Arc};
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
/// The rows a traversal answered, sorted, as the cells they hold.
#[derive(Debug, PartialEq)]
pub(crate) enum Listing {
    /// Every row's cells, row after row, for an answer short enough to state whole.
    Listed(Cow<'static, [Cell]>),
    /// The first and the last row, for an answer of more than `LISTED` rows.
    Ends {
        first: Cow<'static, [Cell]>,
        last: Cow<'static, [Cell]>,
    },
}
/// The most rows an answer states whole.
const LISTED: usize = 16;
/// An answered table as the rows state it: its column kinds, row count, cell count and rows.
#[derive(Debug, PartialEq)]
pub(crate) struct Answered {
    pub(crate) kinds: Cow<'static, [CellKind]>,
    pub(crate) count: usize,
    pub(crate) cells: usize,
    pub(crate) rows: Listing,
}
/// The traversal work a run recorded.
#[derive(Debug, PartialEq)]
pub(crate) struct Work {
    pub(crate) examined: usize,
    pub(crate) expanded: usize,
    pub(crate) frames: usize,
    pub(crate) visited: usize,
}
/// What one traversal observed: its answer, its work, and the working bytes left after the file finished.
#[derive(Debug, PartialEq)]
pub(crate) struct Observed {
    pub(crate) answered: Answered,
    pub(crate) work: Work,
    pub(crate) retained: ByteCount,
}
/// A number cell, for the rows a case states.
pub(crate) const fn n(value: i64) -> Cell {
    Cell::Number(Integer(value))
}
/// The table as the rows state it.
pub(crate) fn answered(out: &Table) -> Answered {
    let width = out.kinds.len().max(1);
    let mut rows: Vec<Cow<'static, [Cell]>> = out
        .cells
        .chunks_exact(width)
        .map(|row| Cow::Owned(row.to_vec()))
        .collect();
    rows.sort_unstable();
    Answered {
        kinds: Cow::Owned(out.kinds.clone()),
        count: out.count.0,
        cells: out.cells.len(),
        rows: if rows.len() > LISTED {
            Listing::Ends {
                first: rows.first().cloned().unwrap_or_default(),
                last: rows.last().cloned().unwrap_or_default(),
            }
        } else {
            Listing::Listed(Cow::Owned(rows.concat()))
        },
    }
}
/// The work counters of a traversal's observation.
pub(crate) fn work(observed: &ExecutionObservation) -> Work {
    Work {
        examined: observed.dfs_examined,
        expanded: observed.dfs_expanded,
        frames: observed.dfs_frames_len,
        visited: observed.dfs_visited_len,
    }
}
/// A request's answer and the working bytes the graph owner still held after the file finished.
#[derive(Debug)]
pub(crate) struct Ran {
    pub(crate) answer: Result<Table, OperatingError>,
    pub(crate) retained: ByteCount,
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
) -> Result<Ran, OperatingError> {
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
    let mut plan = plan(PlanShape::ProjectRetry);
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
) -> Result<Ran, OperatingError> {
    execute_bound_request(
        plan,
        vec![],
        hosts,
        providers,
        symbols,
        source,
        delivery,
        observed,
    )
}
/// The same request with the plan's parameter bindings.
#[expect(
    clippy::too_many_arguments,
    reason = "execute_request's plan, inputs, source, delivery and observation, plus the plan's bindings"
)]
pub(crate) fn execute_bound_request(
    plan: Plan,
    bindings: Vec<ryo_values::domain::query_bindings::types::ParameterBinding>,
    hosts: &[HostRelation],
    providers: &[(Relation, Table)],
    symbols: &mut Symbols,
    source: &[u8],
    delivery: bool,
    observed: &mut ExecutionObservation,
) -> Result<Ran, OperatingError> {
    let target = plan.emit.step.0 as usize;
    let derived = ryo_execution_store::application::file_dag::fixtures::program_derived_port();
    let prepared = crate::application::file_dag::program::preparation::service::prepare(
        Compiled {
            schedule: RuleSchedule::Independent,
            observations: vec![vec![]],
            plans: std::iter::IntoIterator::into_iter(vec![plan])
                .map(std::sync::Arc::new)
                .collect(),
            bindings: vec![bindings],
        },
        &[rule()],
        &[Digest([0; 16])],
        hosts,
        Arc::clone(&derived),
        ObservationIdentity::Independent,
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
        ReuseEligibility::Eligible,
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
    Ok(Ran {
        answer: result,
        retained: owner.derived().working_bytes(),
    })
}
pub(crate) fn hosted_plan(mut op: Operation, roots: u32, edges: u32) -> Plan {
    match &mut op {
        Operation::OrderedDfs { from, edges, .. } => {
            *from = PlanInput::Step(StepId(0));
            *edges = PlanInput::Step(StepId(1));
        }
        _ => panic!("DFS fixture operation"),
    }
    let mut request = plan(PlanShape::ProjectRetry);
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
