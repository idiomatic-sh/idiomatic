//! The plans, rules, row helpers and the forwarding harness that file execution's case groups share.
use super::super::service::*;
use crate::application::file_dag::program::admission::service::Inputs;
#[cfg(test)]
use crate::application::file_dag::program::facade_runtime::source_admission::service::SourceInputs;
use crate::application::file_dag::program::runtime::service::InvocationOutcome;
#[cfg(test)]
use ryo_execution_domain::domain::completion::types::completion_policy::value::CompletionPolicy;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operator::types::Emit as RowSink;
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_execution_store::application::file_dag::service::ScopePartials;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::{
    service::{kinds, node_id},
    types::{CellKind, FileId, Flag, Integer, Offset, Position, Relation, Span, Value as Cell},
};
use ryo_values::domain::{
    plan_parts::types::{
        ColumnIndex, Input as PlanInput, JoinColumn, ReachLookup, RepeatedRows, Side, StepId,
        StepScope,
    },
    plans::types::{Emit, Operation, Plan, PlanStep},
    query_tokens::types::{QueryBundle, QueryDocumentSource, QuerySourceId},
    rows::types::{RowCount, Table},
    runs::types::{RuleApplicability, RuleId},
};
use std::rc::Rc;

/// The shape of the plan a file runs: a reach over duplicated left rows, or a project or union
/// whose first delivery a sink refuses before the retry.
#[derive(Clone, Copy, Debug)]
pub(super) enum PlanShape {
    ReachDuplicate,
    ProjectRetry,
    UnionRetry,
}

pub(super) fn plan(scenario: PlanShape) -> Plan {
    let mut steps = vec![PlanStep {
        scope: StepScope::PerFile,
        operation: Operation::Select {
            from: PlanInput::Relation(Relation::Function),
            comparisons: vec![],
            kept: vec![ColumnIndex(0)],
        },
    }];
    let operation = match scenario {
        PlanShape::ReachDuplicate => Operation::Join {
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
        PlanShape::ProjectRetry => Operation::Project {
            from: PlanInput::Step(StepId(0)),
            columns: vec![ColumnIndex(0)],
            distinct: RepeatedRows::Dropped,
        },
        PlanShape::UnionRetry => Operation::Union {
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
pub(super) fn rule() -> RuleSpec {
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
pub(super) fn function_rows(nodes: &[u32]) -> Table {
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
pub(super) fn nodes(rows: &[Cell]) -> Vec<u64> {
    rows.iter()
        .map(|value| match value {
            Cell::Node(node) => node.0,
            other => panic!("expected node, got {other:?}"),
        })
        .collect()
}

pub(super) fn forward<'facts>(
    file: &mut FileRun<'_>,
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
    let kernels = super::super::super::super::kernel_session::service::KernelSession::file(
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
use ryo_values::domain::rule_requests::types::{RuleParameters, RuleSpec};

/// What a root answered, in the terms a case's expected value states: a table's cells and rows,
/// or the outcome's kind and its error or phase.
#[derive(Debug, PartialEq)]
pub(super) enum Root {
    Complete {
        cells: std::borrow::Cow<'static, [Cell]>,
        rows: usize,
    },
    Partial {
        cells: std::borrow::Cow<'static, [Cell]>,
        rows: usize,
    },
    Skipped,
    Unknown(OperatingError),
    /// Unknown because the semantics are incomplete, for this reason.
    SemanticIncomplete(std::borrow::Cow<'static, str>),
    Failed(OperatingError),
    NoQuery,
    Deferred(StepScope),
}

impl Root {
    /// A complete table of these cells in `rows` rows.
    pub(super) const fn complete(cells: &'static [Cell], rows: usize) -> Self {
        Self::Complete {
            cells: std::borrow::Cow::Borrowed(cells),
            rows,
        }
    }
}

impl From<crate::application::file_dag::program::rule_outcome::types::RootOutcome> for Root {
    fn from(
        outcome: crate::application::file_dag::program::rule_outcome::types::RootOutcome,
    ) -> Self {
        use crate::application::file_dag::program::rule_outcome::types::RootOutcome;
        match outcome {
            RootOutcome::Complete(table) => Self::Complete {
                cells: std::borrow::Cow::Owned(table.cells),
                rows: table.count.0,
            },
            RootOutcome::Partial { table, .. } => Self::Partial {
                cells: std::borrow::Cow::Owned(table.cells),
                rows: table.count.0,
            },
            RootOutcome::Skipped(_) => Self::Skipped,
            RootOutcome::Unknown(OperatingError::SemanticIncomplete { reason }) => {
                Self::SemanticIncomplete(std::borrow::Cow::Owned(reason))
            }
            RootOutcome::Unknown(error) => Self::Unknown(error),
            RootOutcome::Failed(error) => Self::Failed(error),
            RootOutcome::NoQuery => Self::NoQuery,
            RootOutcome::Deferred { phase } => Self::Deferred(phase),
        }
    }
}
