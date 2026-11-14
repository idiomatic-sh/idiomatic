//! The support of the `distinct_integration` cases: one file runs a plan shape and delivers its rows.
use super::super::super::service::*;
use super::super::fixtures::{PlanShape, forward, function_rows, nodes, plan, rule};
use crate::application::file_dag::execution::service::CommonOperatorEngine;
use crate::application::file_dag::program::facade_runtime::service::ProgramServices;
use crate::application::file_dag::program::rule_outcome::types::RootOutcome;
use crate::application::file_dag::program::runtime::service::InvocationOutcome;
use crate::application::file_dag::program::runtime::types::{
    AttemptCheckpoint, DependencyCheckpoint,
};
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operator::types::OperatorContext;
use ryo_execution_domain::domain::rule_intent::types::{
    CaptureIntent, DiagnosticDelivery, EmitDemand, FileRuleIntent, ObservationDemand, RuleDemand,
};
use ryo_execution_domain::domain::table_cells::fixtures::{SMALL_SOURCE, SmallTree, small_tree};
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_execution_domain::domain::{
    observation_schedule::types::RuleSchedule,
    operator::types::Emit as RowSink,
    prepared_request::types::{Compiled, ObservationIdentity},
    table_cells::types::{TreeForm, TreeRows},
};
use ryo_execution_store::application::file_dag::derived::relation_owner::types::ScopePartials;
use ryo_execution_store::application::file_dag::relations::types::ReuseEligibility;
use ryo_language::domain::files::types::FilePath;
use ryo_relations::domain::relations::types::{FileId, Relation, Value as Cell};
use ryo_values::domain::files::types::{
    CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds,
};
use ryo_values::domain::stored_facts::types::Digest;
use std::borrow::Cow;
use std::sync::Arc;

/// Runs the plan shape through one file of the small tree with the crate's derived and operator fixtures.
pub(super) fn run(scenario: PlanShape) -> Result<Delivered, OperatingError> {
    exercise(
        scenario,
        ryo_execution_store::application::file_dag::fixtures::program_derived_port(),
        Arc::new(crate::application::file_dag::fixtures::operators()),
    )
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
/// The root's retry checkpoints around a sink that refuses the first delivery: its attempt and
/// dependencies before anything ran, after the refused delivery (with whether its remaining
/// readers are unchanged) and after the accepted retry; the refused delivery's error and the rows
/// its sink took; and whether the retry published.
#[derive(Debug, PartialEq)]
pub(super) struct Retry {
    pub(super) before: (AttemptCheckpoint, DependencyCheckpoint),
    pub(super) refusal: Option<OperatingError>,
    pub(super) refused_rows: usize,
    pub(super) refused: (AttemptCheckpoint, DependencyCheckpoint, bool),
    pub(super) retried_published: bool,
    pub(super) complete: (AttemptCheckpoint, DependencyCheckpoint),
}

/// The node numbers a run delivered, in order, and its retry checkpoints when the plan retries.
#[derive(Debug, PartialEq)]
pub(super) struct Delivered {
    pub(super) nodes: Cow<'static, [u64]>,
    pub(super) retry: Option<Retry>,
}

pub(super) fn exercise(
    scenario: PlanShape,
    derived: Arc<dyn ryo_execution_store::outbound::derived::interface::DerivedPort>,
    operators: Arc<dyn crate::outbound::operators::interface::OperatorsPort>,
) -> Result<Delivered, OperatingError> {
    let SmallTree {
        elements,
        mut symbols,
    } = small_tree();
    let prepared = crate::application::file_dag::program::preparation::service::prepare(
        Compiled {
            schedule: RuleSchedule::Independent,
            observations: vec![vec![]],
            plans: std::iter::IntoIterator::into_iter(vec![plan(scenario)])
                .map(std::sync::Arc::new)
                .collect(),
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
        PlanShape::ReachDuplicate => &[0, 0],
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
        PlanShape::ReachDuplicate => {
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
            Ok(Delivered {
                nodes: Cow::Owned(nodes(&table.cells)),
                retry: None,
            })
        }
        PlanShape::ProjectRetry | PlanShape::UnionRetry => {
            let root = file
                .program
                .rule_metadata(RuleIndex(0))?
                .emit_root
                .clone()?;
            let before = file
                .execution
                .retry_checkpoint(file.execution.file_state(), root)?;
            let mut fail = RefusingSink {
                reject: true,
                rows: vec![],
            };
            let refusal = forward(&mut file, &read, &mut symbols, &mut fail).err();
            let refused = file
                .execution
                .retry_checkpoint(file.execution.file_state(), root)?;
            let mut accept = RefusingSink {
                reject: false,
                rows: vec![],
            };
            let retried_published = matches!(
                forward(&mut file, &read, &mut symbols, &mut accept)?,
                InvocationOutcome::Published(_)
            );
            let complete = file
                .execution
                .retry_checkpoint(file.execution.file_state(), root)?;
            Ok(Delivered {
                nodes: accept.rows.iter().flat_map(|row| nodes(row)).collect(),
                retry: Some(Retry {
                    before: (before.attempt, before.dependencies),
                    refusal,
                    refused_rows: fail.rows.len(),
                    refused: (
                        refused.attempt,
                        refused.dependencies,
                        refused.remaining == before.remaining,
                    ),
                    retried_published,
                    complete: (complete.attempt, complete.dependencies),
                }),
            })
        }
    }
}
