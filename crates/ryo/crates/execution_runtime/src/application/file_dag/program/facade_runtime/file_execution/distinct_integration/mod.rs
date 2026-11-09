//! Compiled program lifecycle proof with a real owner.
//! Distinct state survives repeated left rows and resets after a failed delivery.
use super::*;
use crate::application::file_dag::execution::CommonOperatorEngine;
use crate::application::file_dag::program::runtime::types::AttemptCheckpoint;
use crate::application::file_dag::program::runtime::types::DependencyCheckpoint;
#[cfg(test)]
use ryo_execution_domain::domain::table_cells::fixtures::{SMALL_SOURCE, SmallTree, small_tree};
use ryo_execution_domain::domain::{
    observation_schedule::types::RuleSchedule,
    operator::types::Emit as RowSink,
    prepared_request::types::{Compiled, ObservationIdentity},
    table_cells::types::{TreeForm, TreeRows},
};
use ryo_execution_store::application::file_dag::service::ScopePartials;
use ryo_language::domain::{files::types::FilePath, symbols::types::Symbols};
use ryo_relations::domain::relations::{
    service::{kinds, node_id},
    types::{CellKind, FileId, Flag, Integer, Offset, Position, Relation, Span, Value as Cell},
};
use ryo_values::domain::{
    files::types::{CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds},
    plan_parts::types::{
        ColumnIndex, Input as PlanInput, JoinColumn, ReachLookup, RepeatedRows, Side, StepId,
        StepScope,
    },
    plans::types::{Emit, Operation, Plan, PlanStep},
    query_tokens::types::{QueryBundle, QueryDocumentSource, QuerySourceId},
    rows::types::{RowCount, Table},
    runs::types::{RuleApplicability, RuleId},
    stored_facts::types::Digest,
};
use std::{collections::BTreeMap, rc::Rc, sync::Arc};

#[derive(Clone, Copy)]
enum Scenario {
    ReachDuplicate,
    ProjectRetry,
    UnionRetry,
}

fn plan(scenario: Scenario) -> Plan {
    let mut steps = vec![PlanStep {
        scope: StepScope::PerFile,
        operation: Operation::Select {
            from: PlanInput::Relation(Relation::Function),
            comparisons: vec![],
            kept: vec![ColumnIndex(0)],
        },
    }];
    let operation = match scenario {
        Scenario::ReachDuplicate => Operation::Join {
            left: PlanInput::Step(StepId(0)),
            right: PlanInput::Relation(Relation::SyntaxChild),
            left_on: vec![ColumnIndex(0)],
            right_on: vec![ColumnIndex(0)],
            output: vec![JoinColumn {
                side: Side::Right,
                column: ColumnIndex(1),
            }],
            filter: vec![],
            view: vec![],
            reach: Some(ReachLookup {
                source: ColumnIndex(0),
                destination: ColumnIndex(1),
                keys: vec![],
                columns: vec![ColumnIndex(0), ColumnIndex(1)],
            }),
        },
        Scenario::ProjectRetry => Operation::Project {
            from: PlanInput::Step(StepId(0)),
            columns: vec![ColumnIndex(0)],
            distinct: RepeatedRows::Dropped,
        },
        Scenario::UnionRetry => Operation::Union {
            left: PlanInput::Step(StepId(0)),
            right: PlanInput::Step(StepId(0)),
        },
    };
    steps.push(PlanStep {
        scope: StepScope::PerFile,
        operation,
    });
    Plan {
        observation_inputs: vec![],
        observations: vec![],
        parameters: vec![],
        steps,
        comparison_origins: vec![],
        frontier: vec![],
        emit: Emit {
            step: StepId(1),
            subject: ColumnIndex(0),
            key: vec![],
            headings: vec![],
        },
    }
}
fn rule() -> RuleSpec {
    RuleSpec {
        id: RuleId("distinct-integration".into()),
        query: QueryBundle {
            root: QueryDocumentSource {
                id: QuerySourceId("distinct-integration".into()),
                text: "".into(),
            },
            modules: Default::default(),
        },
        applicability: RuleApplicability::Always,
        given: vec![],
        observations: vec![],
        spelled: RuleParameters("".into()),
        include: vec![],
        exclude: vec![],
    }
}
fn function_rows(nodes: &[u32]) -> Table {
    let columns = kinds(Relation::Function);
    let mut cells = Vec::new();
    for at in nodes {
        for kind in columns {
            cells.push(match kind {
                CellKind::Node => Cell::Node(node_id(FileId(0), Position(*at))),
                CellKind::Symbol => Cell::Symbol(ryo_relations::domain::terms::types::Symbol(0)),
                CellKind::File => Cell::File(FileId(0)),
                CellKind::Span => Cell::Span(Span {
                    start: Offset(0),
                    end: Offset(0),
                }),
                CellKind::Number => Cell::Number(Integer(0)),
                CellKind::Flag => Cell::Flag(Flag::Lowered),
            });
        }
    }
    Table {
        kinds: columns.to_vec(),
        cells,
        count: RowCount(nodes.len()),
    }
}
fn nodes(rows: &[Cell]) -> Vec<u64> {
    rows.iter()
        .map(|value| match value {
            Cell::Node(node) => node.0,
            other => panic!("expected node, got {other:?}"),
        })
        .collect()
}

struct RefusingSink {
    reject: bool,
    rows: Vec<Vec<Cell>>,
}
impl RowSink for RefusingSink {
    fn row(&mut self, _: &mut OperatorContext<'_>, row: &[Cell]) -> Result<(), OperatingError> {
        if self.reject {
            self.reject = false;
            return Err(OperatingError::WrongBinding);
        }
        self.rows.push(row.to_vec());
        Ok(())
    }
}
fn forward<'a, 'service, 'facts>(
    file: &mut FileRun<'a>,
    read: &dyn Fn(Relation) -> Result<Rows<'facts>, OperatingError>,
    symbols: &mut Symbols,
    sink: &mut dyn RowSink,
) -> Result<InvocationOutcome, OperatingError> {
    let rule = RuleIndex(0);
    let root = file.program.rule_metadata(rule)?.emit_root.clone()?;
    let ScopePartials::File {
        evaluation,
        publication,
    } = &file.partials
    else {
        return Err(OperatingError::WrongOperation);
    };
    let completion = CompletionPolicy::file_inputs(rule, file.file, evaluation, publication);
    let mut context = file.context.borrow(symbols, file.program, rule)?;
    let kernels = super::super::super::kernel_session::KernelSession::file(
        file.services.kernels(),
        file.program,
        Rc::clone(&file.owner),
        read,
        &file.walk,
    );
    let mut sources = SourceInputs {
        execution: &file.execution,
        state: file.execution.file_state(),
        completion: completion.clone(),
        owner: Rc::clone(&file.owner),
        read,
        observations: &mut file.observed,
        file_outputs: Some(&file.roots),
        program: file.program,
        rule,
        project: None,
    };
    let outcome = file.execution.invoke(
        file.execution.file_state(),
        root,
        &mut sources,
        &kernels,
        &completion,
        &mut context,
    )?;
    let InvocationOutcome::Published(input) = &outcome else {
        return Err(OperatingError::NotRun);
    };
    sources.replay(input, &mut context, sink)?;
    Ok(outcome)
}
fn exercise(
    scenario: Scenario,
    derived: Arc<dyn ryo_execution_store::outbound::derived::interface::DerivedPort>,
    operators: Arc<dyn crate::outbound::operators::interface::OperatorsPort>,
) -> Result<Vec<u64>, OperatingError> {
    let SmallTree {
        elements,
        mut symbols,
    } = small_tree();
    let prepared = crate::application::file_dag::program::preparation::service::prepare(
        Compiled {
            schedule: RuleSchedule::Independent,
            observations: vec![vec![]],
            plans: vec![plan(scenario)],
            bindings: vec![vec![]],
        },
        &[rule()],
        &[Digest([0; 16])],
        &[],
        Arc::clone(&derived),
        ObservationIdentity::Independent,
    )?;
    let engine = Arc::new(CommonOperatorEngine::new(operators));
    let services = Arc::new(ProgramServices::new(
        engine,
        derived,
        &[CorpusInput::Available(CorpusEntry {
            executable: ExecutablePermission::Unknown,
            path: FilePath("distinct-integration.py".into()),
            size: FileSize(SMALL_SOURCE.len() as u64),
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
            diagnostic: DiagnosticDelivery::Collected,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::Disabled,
        }],
        services,
        ryo_values::domain::files::types::SourceView(SMALL_SOURCE),
        ReuseEligibility::Eligible,
        &mut symbols,
        None,
        None,
    )?;
    let function = function_rows(match scenario {
        Scenario::ReachDuplicate => &[0, 0],
        _ => &[0, 0, 2],
    });
    let read = |relation| match relation {
        Relation::Function => Ok(Rows::Computed(&function)),
        Relation::SyntaxChild => Ok(Rows::Tree(TreeRows {
            elements: &elements,
            file: FileId(0),
            form: TreeForm::Edges,
        })),
        _ => Err(OperatingError::NotRead),
    };
    match scenario {
        Scenario::ReachDuplicate => {
            let result = file.rule(
                RuleIndex(0),
                RuleDemand {
                    emit: EmitDemand::Enabled,
                    observations: ObservationDemand::NotRequired,
                },
                &read,
                &mut symbols,
            )?;
            let RootOutcome::Complete(table) = result.emit else {
                return Err(OperatingError::NotRun);
            };
            Ok(nodes(&table.cells))
        }
        Scenario::ProjectRetry | Scenario::UnionRetry => {
            let root = file
                .program
                .rule_metadata(RuleIndex(0))?
                .emit_root
                .clone()?;
            let before = file
                .execution
                .retry_checkpoint(file.execution.file_state(), root)?;
            assert_eq!(before.attempt, AttemptCheckpoint::Dormant);
            assert_eq!(before.dependencies, DependencyCheckpoint::Pending);
            let mut fail = RefusingSink {
                reject: true,
                rows: vec![],
            };
            assert!(matches!(
                forward(&mut file, &read, &mut symbols, &mut fail),
                Err(OperatingError::WrongBinding)
            ));
            assert!(
                fail.rows.is_empty(),
                "the first sink refuses before publishing a row"
            );
            let refused = file
                .execution
                .retry_checkpoint(file.execution.file_state(), root)?;
            assert_eq!(refused.attempt, AttemptCheckpoint::Published);
            assert_eq!(refused.dependencies, DependencyCheckpoint::Consumed);
            assert_eq!(
                refused.remaining, before.remaining,
                "delivery refusal retains the already completed publication"
            );
            let mut accept = RefusingSink {
                reject: false,
                rows: vec![],
            };
            assert!(matches!(
                forward(&mut file, &read, &mut symbols, &mut accept),
                Ok(InvocationOutcome::Published(_))
            ));
            let complete = file
                .execution
                .retry_checkpoint(file.execution.file_state(), root)?;
            assert_eq!(complete.attempt, AttemptCheckpoint::Published);
            assert_eq!(complete.dependencies, DependencyCheckpoint::Consumed);
            Ok(accept.rows.iter().flat_map(|row| nodes(row)).collect())
        }
    }
}
use ryo_values::domain::rule_requests::types::{RuleParameters, RuleSpec};

#[cfg(test)]
mod test;

mod producer_lifetime;

mod replay;

mod scratch_owner;

mod execution_boundary;

mod independent_roots;

mod operation_routes;

mod route_controls;

mod ordered_partial;

mod ordered_dfs;

mod closure_abort;

mod measured_work;

mod local_caches;

mod join;

#[cfg(test)]
mod preparation;

#[cfg(test)]
mod readiness;

#[cfg(any(test, feature = "fixtures"))]
mod walk_routes;
